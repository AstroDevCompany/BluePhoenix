use std::ffi::CString;
use std::num::NonZeroU32;
use std::pin::pin;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::LlamaToken;
use llama_cpp_2::{send_logs_to_tracing, LogOptions};

use super::grammar::JSON_GBNF;
use super::{compiled_gpu_backend, set_state, EngineState, Inner, Job, LocalModelSpec};
use crate::compat::strip_thinking_wrappers;
use crate::error::{AiError, AiFailureKind, AiResult};
use crate::provider::{ChatMessage, CompletionRequest, CompletionResponse};

pub fn run(rx: Receiver<Job>, inner: Arc<Inner>) {
    send_logs_to_tracing(LogOptions::default());
    let backend = match LlamaBackend::init() {
        Ok(backend) => backend,
        Err(err) => {
            set_state(
                &inner,
                EngineState::Error {
                    message: err.to_string(),
                },
            );
            while let Ok(job) = rx.recv() {
                match job {
                    Job::Complete { result_tx, .. } => {
                        let _ = result_tx.send(Err(AiError::provider(
                            AiFailureKind::Unavailable,
                            format!("llama.cpp failed to start: {err}"),
                            None,
                        )));
                    }
                    Job::Unload => set_state(&inner, EngineState::Unloaded),
                }
            }
            return;
        }
    };

    loop {
        match recv_job(&rx, &inner, true) {
            Recv::Disconnected => return,
            Recv::Timeout => set_state(&inner, EngineState::Unloaded),
            Recv::Job(Job::Unload) => set_state(&inner, EngineState::Unloaded),
            Recv::Job(Job::Complete {
                spec,
                request,
                delta_tx,
                result_tx,
            }) => {
                run_session(&backend, &rx, &inner, spec, request, delta_tx, result_tx);
            }
        }
    }
}

enum Recv {
    Job(Job),
    Timeout,
    Disconnected,
}

fn recv_job(rx: &Receiver<Job>, inner: &Inner, block_forever: bool) -> Recv {
    let minutes = inner
        .idle_minutes
        .load(std::sync::atomic::Ordering::Relaxed);
    if block_forever || minutes == 0 {
        return match rx.recv() {
            Ok(job) => Recv::Job(job),
            Err(_) => Recv::Disconnected,
        };
    }
    match rx.recv_timeout(Duration::from_secs(u64::from(minutes) * 60)) {
        Ok(job) => Recv::Job(job),
        Err(RecvTimeoutError::Timeout) => Recv::Timeout,
        Err(RecvTimeoutError::Disconnected) => Recv::Disconnected,
    }
}

fn run_session(
    backend: &LlamaBackend,
    rx: &Receiver<Job>,
    inner: &Arc<Inner>,
    mut spec: LocalModelSpec,
    mut request: CompletionRequest,
    mut delta_tx: tokio::sync::mpsc::UnboundedSender<String>,
    mut result_tx: tokio::sync::oneshot::Sender<AiResult<CompletionResponse>>,
) {
    loop {
        set_state(
            inner,
            EngineState::Loading {
                model_id: spec.id.clone(),
            },
        );
        match load_and_serve(backend, rx, inner, &spec, request, delta_tx, result_tx) {
            SessionEnd::Done => return,
            SessionEnd::Reload {
                spec: next_spec,
                request: next_request,
                delta_tx: next_delta,
                result_tx: next_result,
            } => {
                spec = next_spec;
                request = next_request;
                delta_tx = next_delta;
                result_tx = next_result;
            }
        }
    }
}

enum SessionEnd {
    Done,
    Reload {
        spec: LocalModelSpec,
        request: CompletionRequest,
        delta_tx: tokio::sync::mpsc::UnboundedSender<String>,
        result_tx: tokio::sync::oneshot::Sender<AiResult<CompletionResponse>>,
    },
}

fn load_and_serve(
    backend: &LlamaBackend,
    rx: &Receiver<Job>,
    inner: &Arc<Inner>,
    spec: &LocalModelSpec,
    first_request: CompletionRequest,
    first_delta: tokio::sync::mpsc::UnboundedSender<String>,
    first_result: tokio::sync::oneshot::Sender<AiResult<CompletionResponse>>,
) -> SessionEnd {
    let try_gpu = spec.gpu_layers > 0 && compiled_gpu_backend() != "none";
    let attempts: &[bool] = if try_gpu { &[true, false] } else { &[false] };
    let mut last_error = None;
    let mut first = Some((first_request, first_delta, first_result));
    for &use_gpu in attempts {
        let model = match load_model(backend, spec, use_gpu) {
            Ok(model) => model,
            Err(err) => {
                last_error = Some(err);
                continue;
            }
        };
        let ctx_params = context_params(&model, spec);
        let mut ctx = match model.new_context(backend, ctx_params) {
            Ok(ctx) => ctx,
            Err(err) => {
                last_error = Some(format!("Could not create llama.cpp context: {err}"));
                continue;
            }
        };
        let (first_request, first_delta, first_result) = first.take().expect("first job");
        let mut last_tokens: Vec<LlamaToken> = Vec::new();
        let mut pending = Some((first_request, first_delta, first_result));
        while let Some((request, delta_tx, result_tx)) = pending.take() {
            set_state(
                inner,
                EngineState::Busy {
                    model_id: spec.id.clone(),
                },
            );
            let outcome = generate(
                &model,
                &mut ctx,
                &mut last_tokens,
                spec,
                request,
                &delta_tx,
                &result_tx,
            );
            set_state(
                inner,
                EngineState::Ready {
                    model_id: spec.id.clone(),
                },
            );
            let _ = result_tx.send(outcome);
            match recv_job(rx, inner, false) {
                Recv::Disconnected => return SessionEnd::Done,
                Recv::Timeout | Recv::Job(Job::Unload) => {
                    set_state(inner, EngineState::Unloaded);
                    return SessionEnd::Done;
                }
                Recv::Job(Job::Complete {
                    spec: next,
                    request,
                    delta_tx,
                    result_tx,
                }) => {
                    if needs_reload(spec, &next) {
                        return SessionEnd::Reload {
                            spec: next,
                            request,
                            delta_tx,
                            result_tx,
                        };
                    }
                    pending = Some((request, delta_tx, result_tx));
                }
            }
        }
        return SessionEnd::Done;
    }
    let message = last_error.unwrap_or_else(|| "Could not load GGUF model".into());
    set_state(
        inner,
        EngineState::Error {
            message: message.clone(),
        },
    );
    if let Some((_, _, first_result)) = first.take() {
        let _ = first_result.send(Err(AiError::provider(
            AiFailureKind::Unavailable,
            message,
            Some(spec.name.clone()),
        )));
    }
    SessionEnd::Done
}

fn load_model(
    backend: &LlamaBackend,
    spec: &LocalModelSpec,
    use_gpu: bool,
) -> Result<LlamaModel, String> {
    if use_gpu {
        return load_model_gpu(backend, spec);
    }
    let params = LlamaModelParams::default()
        .with_n_gpu_layers(0)
        .with_use_mlock(false)
        .with_use_mmap(true);
    LlamaModel::load_from_file(backend, &spec.path, &params)
        .map_err(|err| format!("Could not load GGUF model: {err}"))
}

fn load_model_gpu(backend: &LlamaBackend, spec: &LocalModelSpec) -> Result<LlamaModel, String> {
    let path = CString::new(spec.path.as_str()).map_err(|_| "Invalid model path".to_string())?;
    let n_ctx = NonZeroU32::new(spec.ctx_len.max(1)).unwrap_or(NonZeroU32::MIN);
    let threads = spec.threads.max(1) as i32;
    let mut ctx_params = LlamaContextParams::default()
        .with_n_ctx(Some(n_ctx))
        .with_n_threads(threads)
        .with_n_threads_batch(threads);
    let mut params = pin!(LlamaModelParams::default());
    let mut margins = vec![0usize; llama_cpp_2::max_devices()];
    params
        .as_mut()
        .fit_params(
            &path,
            &mut ctx_params,
            &mut margins,
            spec.ctx_len.max(1),
            llama_cpp_sys_2::GGML_LOG_LEVEL_INFO,
        )
        .map_err(|err| format!("Could not fit model to GPU memory: {err}"))?;
    LlamaModel::load_from_file(backend, &spec.path, &*params)
        .map_err(|err| format!("Could not load GGUF model: {err}"))
}

fn context_params(model: &LlamaModel, spec: &LocalModelSpec) -> LlamaContextParams {
    let train = model.n_ctx_train();
    let n_ctx = if train == 0 {
        spec.ctx_len.max(1)
    } else {
        spec.ctx_len.min(train).max(1)
    };
    let n_ctx = NonZeroU32::new(n_ctx).unwrap_or(NonZeroU32::MIN);
    let threads = spec.threads.max(1) as i32;
    LlamaContextParams::default()
        .with_n_ctx(Some(n_ctx))
        .with_n_threads(threads)
        .with_n_threads_batch(threads)
}

fn needs_reload(current: &LocalModelSpec, next: &LocalModelSpec) -> bool {
    current.path != next.path
        || current.ctx_len != next.ctx_len
        || current.gpu_layers != next.gpu_layers
        || current.threads != next.threads
}

fn generate(
    model: &LlamaModel,
    ctx: &mut LlamaContext<'_>,
    last_tokens: &mut Vec<LlamaToken>,
    spec: &LocalModelSpec,
    request: CompletionRequest,
    delta_tx: &tokio::sync::mpsc::UnboundedSender<String>,
    result_tx: &tokio::sync::oneshot::Sender<AiResult<CompletionResponse>>,
) -> AiResult<CompletionResponse> {
    let prompt = render_prompt(model, &request.messages)?;
    let tokens = model.str_to_token(&prompt, AddBos::Always).map_err(|err| {
        AiError::provider(
            AiFailureKind::Validation,
            format!("Could not tokenize prompt: {err}"),
            Some(spec.name.clone()),
        )
    })?;
    if tokens.is_empty() {
        return Err(AiError::provider(
            AiFailureKind::Validation,
            "Prompt produced no tokens",
            Some(spec.name.clone()),
        ));
    }
    let n_ctx = ctx.n_ctx() as usize;
    if tokens.len() >= n_ctx {
        return Err(AiError::provider(
            AiFailureKind::Validation,
            "Prompt is too long for the context window",
            Some(spec.name.clone()),
        ));
    }

    let prefix = common_prefix(last_tokens, &tokens);
    if prefix == 0 {
        ctx.clear_kv_cache();
    } else {
        let _ = ctx.clear_kv_cache_seq(Some(0), Some(prefix as u32), None);
    }

    let decode_from = if prefix == tokens.len() && prefix > 0 {
        prefix - 1
    } else {
        prefix
    };
    let mut batch = decode_tokens(ctx, &tokens, decode_from)?;
    *last_tokens = tokens.clone();

    let mut sampler = build_sampler(model, &request);
    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut assembled = String::new();
    let mut n_cur = tokens.len() as i32;
    let max_new = request.max_tokens.max(1);
    let mut produced = 0u32;

    while produced < max_new && (n_cur as usize) < n_ctx {
        if result_tx.is_closed() || delta_tx.is_closed() {
            break;
        }
        let token = sampler.sample(ctx, batch.n_tokens() - 1);
        sampler.accept(token);
        if model.is_eog_token(token) {
            break;
        }
        let piece = model
            .token_to_piece(token, &mut decoder, false, None)
            .unwrap_or_default();
        if !piece.is_empty() {
            assembled.push_str(&piece);
            let _ = delta_tx.send(piece);
        }
        batch.clear();
        batch.add(token, n_cur, &[0], true).map_err(|err| {
            AiError::provider(
                AiFailureKind::Unavailable,
                format!("llama.cpp batch failed: {err}"),
                Some(spec.name.clone()),
            )
        })?;
        ctx.decode(&mut batch).map_err(|err| {
            AiError::provider(
                AiFailureKind::Unavailable,
                format!("llama.cpp decode failed: {err}"),
                Some(spec.name.clone()),
            )
        })?;
        n_cur += 1;
        produced += 1;
    }

    let text = strip_thinking_wrappers(&assembled);
    if text.trim().is_empty() {
        return Err(AiError::provider(
            AiFailureKind::Malformed,
            "Local model returned no text",
            Some(spec.name.clone()),
        ));
    }
    Ok(CompletionResponse {
        text,
        model: spec.name.clone(),
        finish_reason: Some("stop".into()),
    })
}

fn decode_tokens(
    ctx: &mut LlamaContext<'_>,
    tokens: &[LlamaToken],
    start: usize,
) -> AiResult<LlamaBatch<'static>> {
    let start = start.min(tokens.len().saturating_sub(1));
    let cap = 512.min(tokens.len().saturating_sub(start).max(1));
    let mut batch = LlamaBatch::new(cap, 1);
    let mut i = start;
    while i < tokens.len() {
        batch.clear();
        let end = (i + cap).min(tokens.len());
        for (offset, token) in tokens[i..end].iter().enumerate() {
            let pos = (i + offset) as i32;
            let is_last = i + offset + 1 == tokens.len();
            batch.add(*token, pos, &[0], is_last).map_err(|err| {
                AiError::provider(
                    AiFailureKind::Unavailable,
                    format!("llama.cpp batch failed: {err}"),
                    None,
                )
            })?;
        }
        ctx.decode(&mut batch).map_err(|err| {
            AiError::provider(
                AiFailureKind::Unavailable,
                format!("llama.cpp decode failed: {err}"),
                None,
            )
        })?;
        i = end;
    }
    Ok(batch)
}

fn build_sampler(model: &LlamaModel, request: &CompletionRequest) -> LlamaSampler {
    let mut chain = Vec::new();
    if request.json_mode {
        if let Ok(grammar) = LlamaSampler::grammar(model, JSON_GBNF, "root") {
            chain.push(grammar);
        }
    }
    chain.push(LlamaSampler::top_k(40));
    chain.push(LlamaSampler::top_p(0.95, 1));
    if request.temperature > 0.0 {
        chain.push(LlamaSampler::temp(request.temperature));
    }
    if request.json_mode {
        chain.push(LlamaSampler::greedy());
    } else {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(1234);
        chain.push(LlamaSampler::dist(seed));
    }
    LlamaSampler::chain_simple(chain)
}

fn render_prompt(model: &LlamaModel, messages: &[ChatMessage]) -> AiResult<String> {
    let try_apply = |tmpl: &LlamaChatTemplate, fold: bool| -> Option<String> {
        let converted = convert_messages(messages, fold).ok()?;
        model.apply_chat_template(tmpl, &converted, true).ok()
    };
    if let Ok(tmpl) = model.chat_template(None) {
        if let Some(text) = try_apply(&tmpl, false).or_else(|| try_apply(&tmpl, true)) {
            return Ok(text);
        }
    }
    if let Ok(tmpl) = LlamaChatTemplate::new("chatml") {
        if let Some(text) = try_apply(&tmpl, false).or_else(|| try_apply(&tmpl, true)) {
            return Ok(text);
        }
    }
    Ok(flatten_messages(messages))
}

fn convert_messages(
    messages: &[ChatMessage],
    fold_system: bool,
) -> AiResult<Vec<LlamaChatMessage>> {
    if !fold_system {
        return messages
            .iter()
            .map(|m| {
                LlamaChatMessage::new(m.role.clone(), m.content.clone()).map_err(|err| {
                    AiError::provider(
                        AiFailureKind::Validation,
                        format!("Invalid chat message: {err}"),
                        None,
                    )
                })
            })
            .collect();
    }
    let mut system = String::new();
    let mut out = Vec::new();
    for message in messages {
        if message.role == "system" {
            if !system.is_empty() {
                system.push_str("\n\n");
            }
            system.push_str(&message.content);
            continue;
        }
        let mut content = message.content.clone();
        if !system.is_empty() && message.role == "user" {
            content = format!("{system}\n\n{content}");
            system.clear();
        }
        out.push(
            LlamaChatMessage::new(message.role.clone(), content).map_err(|err| {
                AiError::provider(
                    AiFailureKind::Validation,
                    format!("Invalid chat message: {err}"),
                    None,
                )
            })?,
        );
    }
    if !system.is_empty() && out.is_empty() {
        out.push(LlamaChatMessage::new("user".into(), system).map_err(|err| {
            AiError::provider(
                AiFailureKind::Validation,
                format!("Invalid chat message: {err}"),
                None,
            )
        })?);
    }
    Ok(out)
}

fn flatten_messages(messages: &[ChatMessage]) -> String {
    let mut out = String::new();
    for message in messages {
        out.push_str(&message.role);
        out.push_str(": ");
        out.push_str(&message.content);
        out.push('\n');
    }
    out.push_str("assistant:");
    out
}

fn common_prefix(a: &[LlamaToken], b: &[LlamaToken]) -> usize {
    a.iter().zip(b.iter()).take_while(|(l, r)| l == r).count()
}
