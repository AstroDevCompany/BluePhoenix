use crate::ai_store;
use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::native;
use crate::secrets;
use crate::state::AppState;
use bluephoenix_ai::local::{
    compiled_gpu_backend, default_thread_count, inspect_gguf, LocalEngine, LocalLlamaProvider,
    LocalModelSpec,
};
use bluephoenix_ai::prompts;
use bluephoenix_ai::settings::{AiProviderKind, AiSettings};
use bluephoenix_ai::{
    complete_with_fallback, parse_agent_prompt, parse_changelog, parse_command_scan, parse_commit,
    parse_prioritize, parse_search, parse_software_folder_draft, parse_todos,
    requested_inspect_files, require_ready, stream_with_fallback, AiProvider, ChatMessage,
    CommandProposal, CompletionRequest, OpenRouterProvider, OPENROUTER_SECRET_KIND,
};
use bluephoenix_domain::commands_safety::is_dangerous;
use bluephoenix_domain::ids::CategoryKind;
use serde::Deserialize;
use serde_json::json;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};

fn map_ai(err: bluephoenix_ai::AiError) -> AppError {
    AppError::msg(err.user_message())
}

fn engine_status(engine: &LocalEngine) -> LocalEngineStatusDto {
    let state = engine.state();
    LocalEngineStatusDto {
        state: state.label().into(),
        model_id: state.model_id().map(|s| s.to_string()),
        error: state.error().map(|s| s.to_string()),
    }
}

fn load_ready(state: &AppState) -> AppResult<(Box<dyn AiProvider>, AiSettings, Vec<String>)> {
    let settings = state.db.with(|c| Ok(ai_store::load_ai_settings(c)))?;
    match settings.provider {
        AiProviderKind::Local => {
            require_ready(&settings, secrets::has_openrouter_key()).map_err(map_ai)?;
            let id = settings
                .local_model_id
                .clone()
                .ok_or_else(|| map_ai(bluephoenix_ai::AiError::NoLocalModel))?;
            let model = state
                .db
                .with(|c| ai_store::get_local_model(c, &id))?
                .ok_or_else(|| AppError::msg("Selected local model is missing"))?;
            let spec = LocalModelSpec {
                id: model.id,
                name: model.name.clone(),
                path: model.path,
                ctx_len: settings.local_ctx_len,
                gpu_layers: if settings.local_gpu_offload && compiled_gpu_backend() != "none" {
                    1
                } else {
                    0
                },
                threads: default_thread_count(),
            };
            let display = spec.name.clone();
            let provider = LocalLlamaProvider::new(state.local_llm.clone(), spec);
            Ok((Box::new(provider), settings, vec![display]))
        }
        AiProviderKind::OpenRouter => {
            let key = secrets::openrouter_key()
                .ok_or_else(|| map_ai(bluephoenix_ai::AiError::MissingKey))?;
            require_ready(&settings, true).map_err(map_ai)?;
            let models = settings.active_models();
            Ok((Box::new(OpenRouterProvider::new(key)), settings, models))
        }
    }
}

async fn complete_text(
    state: &AppState,
    messages: Vec<ChatMessage>,
    json_mode: bool,
) -> AppResult<bluephoenix_ai::FallbackOutcome> {
    let (provider, _settings, models) = load_ready(state)?;
    let request = if json_mode {
        CompletionRequest::json(models.first().cloned().unwrap_or_default(), messages)
    } else {
        CompletionRequest::chat(models.first().cloned().unwrap_or_default(), messages)
    };
    complete_with_fallback(&*provider, &models, request)
        .await
        .map_err(map_ai)
}

fn persist_openrouter_envelope(state: &AppState, plaintext: &str) -> AppResult<()> {
    if !secrets::wrap_key_available() {
        return Ok(());
    }
    let (ciphertext, nonce, wrap_params) = secrets::encrypt_secret(plaintext)?;
    state.db.with(|c| {
        db::store_secret_envelope(c, OPENROUTER_SECRET_KIND, &ciphertext, &nonce, wrap_params)
    })?;
    Ok(())
}

pub fn try_unwrap_after_auth(state: &AppState, email: &str, password: &str) -> AppResult<()> {
    secrets::on_login(email, password)?;
    let _ = state.db.with(|c| ai_store::set_account_email(c, email));
    if secrets::has_openrouter_key() {
        return Ok(());
    }
    if let Some((_, ct, nonce, params)) = state
        .db
        .with(|c| Ok(ai_store::load_openrouter_envelope(c)))?
    {
        if let Ok(plain) = secrets::decrypt_secret(&ct, &nonce, &params, password, email) {
            secrets::keyring_set(secrets::OPENROUTER_KEY, &plain)?;
        }
    }
    Ok(())
}

pub fn try_unwrap_with_stored_key(state: &AppState) -> AppResult<()> {
    if secrets::has_openrouter_key() || !secrets::wrap_key_available() {
        return Ok(());
    }
    if let Some((_, ct, nonce, _)) = state
        .db
        .with(|c| Ok(ai_store::load_openrouter_envelope(c)))?
    {
        if let Ok(plain) = secrets::decrypt_with_stored_key(&ct, &nonce) {
            secrets::keyring_set(secrets::OPENROUTER_KEY, &plain)?;
        }
    }
    Ok(())
}

#[tauri::command]
fn read_ai_status(state: &AppState) -> AppResult<AiStatusDto> {
    let settings = state.db.with(|c| Ok(ai_store::load_ai_settings(c)))?;
    let key = secrets::openrouter_key();
    let cloud = state
        .db
        .with(|c| Ok(ai_store::load_openrouter_envelope(c).is_some()))?;
    let local_models = state.db.with(ai_store::list_local_models)?;
    Ok(AiStatusDto {
        enabled: settings.enabled,
        has_key: key.is_some(),
        masked_key: key.as_deref().map(secrets::mask_key),
        models: settings.models,
        commit_follow_style: settings.commit_follow_style,
        setup_dismissed: settings.setup_dismissed,
        signed_in: secrets::wrap_key_available() || secrets::keyring_get("access_token").is_some(),
        cloud_secret: cloud,
        provider: settings.provider.as_str().into(),
        local_model_id: settings.local_model_id,
        local_models,
        local_engine: engine_status(&state.local_llm),
        local_ctx_len: settings.local_ctx_len,
        local_gpu_offload: settings.local_gpu_offload,
        local_idle_unload_minutes: settings.local_idle_unload_minutes,
    })
}

#[tauri::command]
pub fn ai_status(state: State<AppState>) -> AppResult<AiStatusDto> {
    read_ai_status(&state)
}

#[tauri::command]
pub fn ai_save_settings(state: State<AppState>, settings: AiSettings) -> AppResult<AiSettings> {
    let before = state.db.with(|c| Ok(ai_store::load_ai_settings(c)))?;
    let saved = state
        .db
        .with(|c| ai_store::save_ai_settings(c, &settings))?;
    state
        .local_llm
        .set_idle_unload(saved.local_idle_unload_minutes);
    let reload = before.provider != saved.provider
        || before.local_model_id != saved.local_model_id
        || before.local_ctx_len != saved.local_ctx_len
        || before.local_gpu_offload != saved.local_gpu_offload;
    if reload {
        state.local_llm.unload();
    }
    Ok(saved)
}

#[tauri::command]
pub fn ai_set_key(state: State<AppState>, key: String) -> AppResult<AiStatusDto> {
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err(AppError::msg("API key is required"));
    }
    if key.contains(char::is_whitespace) {
        return Err(AppError::msg(
            "That does not look like an OpenRouter API key",
        ));
    }
    secrets::keyring_set(secrets::OPENROUTER_KEY, &key)?;
    persist_openrouter_envelope(&state, &key)?;
    read_ai_status(&state)
}

#[tauri::command]
pub fn ai_clear_key(state: State<AppState>) -> AppResult<AiStatusDto> {
    secrets::keyring_delete(secrets::OPENROUTER_KEY);
    read_ai_status(&state)
}

#[tauri::command]
pub fn ai_reveal_key() -> AppResult<RevealedKeyDto> {
    let key = secrets::openrouter_key().ok_or_else(|| AppError::msg("No API key is stored"))?;
    Ok(RevealedKeyDto { key })
}

#[tauri::command]
pub async fn ai_test_connection(state: State<'_, AppState>) -> AppResult<serde_json::Value> {
    let outcome = complete_text(
        &state,
        vec![
            ChatMessage {
                role: "system".into(),
                content: "Reply with the single word OK.".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: "ping".into(),
            },
        ],
        false,
    )
    .await?;
    Ok(json!({
        "ok": true,
        "model": outcome.response.model,
        "fallbackUsed": outcome.fallback_used,
        "text": outcome.response.text.trim(),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModelImportInput {
    pub path: String,
    pub copy: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModelRemoveInput {
    pub id: String,
    pub delete_file: bool,
}

fn is_gguf_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("gguf"))
        .unwrap_or(false)
}

fn unique_copy_dest(dir: &Path, file_name: &str) -> PathBuf {
    let dest = dir.join(file_name);
    if !dest.exists() {
        return dest;
    }
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model");
    for i in 2..10_000 {
        let candidate = dir.join(format!("{stem}-{i}.gguf"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!(
        "{stem}-{}.gguf",
        chrono::Utc::now().timestamp_millis()
    ))
}

fn copy_with_progress(app: &AppHandle, src: &Path, dest: &Path) -> AppResult<u64> {
    let total = std::fs::metadata(src)?.len();
    let mut reader = std::fs::File::open(src)?;
    let mut writer = std::fs::File::create(dest)?;
    let mut buf = vec![0u8; 1024 * 1024];
    let mut copied = 0u64;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n])?;
        copied += n as u64;
        let _ = app.emit(
            "local-model-import-progress",
            json!({ "bytes": copied, "total": total }),
        );
    }
    writer.sync_all()?;
    Ok(copied)
}

#[tauri::command]
pub async fn local_model_import(
    app: AppHandle,
    state: State<'_, AppState>,
    input: LocalModelImportInput,
) -> AppResult<AiStatusDto> {
    let src = PathBuf::from(input.path.trim());
    if !src.is_file() {
        return Err(AppError::msg("Choose a GGUF file to import"));
    }
    if !is_gguf_path(&src) {
        return Err(AppError::msg("Only .gguf model files can be imported"));
    }
    let info = inspect_gguf(&src).map_err(map_ai)?;
    let stored_path = if input.copy {
        let dir = native::models_dir()?;
        let file_name = src
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| AppError::msg("Invalid model file name"))?;
        let dest = unique_copy_dest(&dir, file_name);
        let dest_clone = dest.clone();
        let src_clone = src.clone();
        let app_clone = app.clone();
        tokio::task::spawn_blocking(move || {
            copy_with_progress(&app_clone, &src_clone, &dest_clone)
        })
        .await
        .map_err(|err| AppError::msg(format!("Copy failed: {err}")))??;
        dest
    } else {
        src.clone()
    };
    let path_str = stored_path.to_string_lossy().into_owned();
    if state
        .db
        .with(|c| ai_store::find_local_model_by_path(c, &path_str))?
        .is_some()
    {
        return Err(AppError::msg("This model is already in the list"));
    }
    let inserted = state.db.with(|c| {
        ai_store::insert_local_model(
            c,
            &info.name,
            &path_str,
            input.copy,
            Some(info.size_bytes as i64),
            info.arch.as_deref(),
            info.n_ctx_train.map(|n| n as i64),
        )
    })?;
    let mut settings = state.db.with(|c| Ok(ai_store::load_ai_settings(c)))?;
    if settings.local_model_id.is_none() {
        settings.local_model_id = Some(inserted.id);
        state
            .db
            .with(|c| ai_store::save_ai_settings(c, &settings))?;
    }
    read_ai_status(&state)
}

#[tauri::command]
pub fn local_model_remove(
    state: State<AppState>,
    input: LocalModelRemoveInput,
) -> AppResult<AiStatusDto> {
    let removed = state
        .db
        .with(|c| ai_store::delete_local_model(c, &input.id))?
        .ok_or_else(|| AppError::msg("Model not found"))?;
    let mut settings = state.db.with(|c| Ok(ai_store::load_ai_settings(c)))?;
    if settings.local_model_id.as_deref() == Some(removed.id.as_str()) {
        settings.local_model_id = None;
        state
            .db
            .with(|c| ai_store::save_ai_settings(c, &settings))?;
        state.local_llm.unload();
    }
    if input.delete_file && removed.managed {
        let _ = std::fs::remove_file(&removed.path);
    }
    read_ai_status(&state)
}

#[tauri::command]
pub fn local_model_unload(state: State<AppState>) -> AppResult<AiStatusDto> {
    state.local_llm.unload();
    read_ai_status(&state)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatInput {
    pub conversation_id: Option<String>,
    pub project_id: Option<String>,
    pub message: String,
}

#[tauri::command]
pub async fn ai_chat_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    input: ChatInput,
) -> AppResult<AiMessageDto> {
    let message = input.message.trim().to_string();
    if message.is_empty() {
        return Err(AppError::msg("Message is required"));
    }
    let project_id = input.project_id.clone();
    let conversation_id = state.db.with(|c| {
        ai_store::ensure_conversation(c, input.conversation_id.as_deref(), project_id.as_deref())
    })?;
    state
        .db
        .with(|c| ai_store::insert_message(c, &conversation_id, "user", &message, None, false))?;
    let history = state
        .db
        .with(|c| ai_store::list_messages(c, &conversation_id))?;
    let bundle = if let Some(pid) = &project_id {
        state.db.with(|c| ai_store::ai_project_bundle(c, pid)).ok()
    } else {
        None
    };
    let kind = bundle
        .as_ref()
        .and_then(|b| b.snapshot.as_ref())
        .map(|s| s.kind);
    let mut system = prompts::chat_system(kind);
    if let Some(bundle) = &bundle {
        system.push_str("\n\nProject context:\n");
        system.push_str(&prompts::bundle_to_prompt(bundle));
        if let Some(pid) = &project_id {
            let chunks = state
                .db
                .with(|c| ai_store::retrieve_chunks(c, pid, &message, 6))
                .unwrap_or_default();
            let empty = chunks.is_empty();
            let rendered = chunks
                .iter()
                .map(|c| {
                    format!(
                        "[{} p.{} {}] {}",
                        c.document_name,
                        c.page.map(|p| p.to_string()).unwrap_or_else(|| "?".into()),
                        c.heading.clone().unwrap_or_default(),
                        c.text
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            system.push_str("\n");
            system.push_str(&prompts::document_qa_suffix(&rendered, empty));
        }
    }
    let mut messages = vec![ChatMessage {
        role: "system".into(),
        content: system,
    }];
    for m in history
        .iter()
        .rev()
        .take(16)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        if m.role == "system" {
            continue;
        }
        messages.push(ChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        });
    }
    let (provider, _settings, models) = load_ready(&state)?;
    let cid = conversation_id.clone();
    let app_for_delta = app.clone();
    let request = CompletionRequest::chat(models.first().cloned().unwrap_or_default(), messages);
    let outcome = stream_with_fallback(&*provider, &models, request, move |delta| {
        let _ = app_for_delta.emit(
            "ai-chat-delta",
            json!({ "conversationId": cid, "delta": delta }),
        );
    })
    .await
    .map_err(map_ai)?;
    let assistant_id = state.db.with(|c| {
        ai_store::insert_message(
            c,
            &conversation_id,
            "assistant",
            &outcome.response.text,
            Some(&outcome.response.model),
            outcome.fallback_used,
        )
    })?;
    let _ = app.emit(
        "ai-chat-done",
        json!({
            "conversationId": conversation_id,
            "model": outcome.response.model,
            "fallbackUsed": outcome.fallback_used,
        }),
    );
    let messages = state
        .db
        .with(|c| ai_store::list_messages(c, &conversation_id))?;
    messages
        .into_iter()
        .find(|m| m.id == assistant_id)
        .ok_or_else(|| AppError::msg("Message missing after save"))
}

#[tauri::command]
pub fn ai_list_conversations(
    state: State<AppState>,
    project_id: Option<String>,
) -> AppResult<Vec<AiConversationDto>> {
    state
        .db
        .with(|c| ai_store::list_conversations(c, project_id.as_deref()))
}

#[tauri::command]
pub fn ai_list_messages(
    state: State<AppState>,
    conversation_id: String,
) -> AppResult<Vec<AiMessageDto>> {
    state
        .db
        .with(|c| ai_store::list_messages(c, &conversation_id))
}

#[tauri::command]
pub fn ai_new_conversation(
    state: State<AppState>,
    project_id: Option<String>,
) -> AppResult<String> {
    state
        .db
        .with(|c| ai_store::new_conversation(c, project_id.as_deref()))
}

#[tauri::command]
pub fn ai_clear_conversation(state: State<AppState>, conversation_id: String) -> AppResult<()> {
    state
        .db
        .with(|c| ai_store::clear_conversation(c, &conversation_id))
}

#[tauri::command]
pub fn ai_project_bundle(
    state: State<AppState>,
    project_id: String,
) -> AppResult<bluephoenix_domain::context::AiProjectBundle> {
    state
        .db
        .with(|c| ai_store::ai_project_bundle(c, &project_id))
}

#[tauri::command]
pub async fn ai_interpret_search(
    state: State<'_, AppState>,
    query: String,
) -> AppResult<serde_json::Value> {
    let local = state.db.with(|c| db::search(c, &query))?;
    let settings = state.db.with(|c| Ok(ai_store::load_ai_settings(c)))?;
    if require_ready(&settings, secrets::has_openrouter_key()).is_err() {
        return Ok(json!({ "hits": local, "explanation": null, "ai": false }));
    }
    let index = state.db.with(|c| ai_store::compact_project_index(c))?;
    let outcome = complete_text(
        &state,
        vec![
            ChatMessage {
                role: "system".into(),
                content: "Return JSON only.".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: prompts::search_prompt(
                    &serde_json::to_string(&index).unwrap_or_default(),
                    &query,
                ),
            },
        ],
        true,
    )
    .await?;
    let parsed = parse_search(&outcome.response.text);
    Ok(json!({
        "hits": local,
        "ai": true,
        "interpretation": parsed,
        "fallbackUsed": outcome.fallback_used,
        "model": outcome.response.model,
    }))
}

fn require_software(state: &AppState, project_id: &str) -> AppResult<CategoryKind> {
    let kind = state.db.with(|c| db::project_kind(c, project_id))?;
    if kind != CategoryKind::Software {
        return Err(AppError::msg(
            "This action is available on software projects",
        ));
    }
    Ok(kind)
}

#[tauri::command]
pub async fn ai_prioritize_todos(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<serde_json::Value> {
    require_software(&state, &project_id)?;
    let bundle = state
        .db
        .with(|c| ai_store::ai_project_bundle(c, &project_id))?;
    let json_todos = serde_json::to_string(&bundle.todos).unwrap_or_else(|_| "[]".into());
    let outcome = complete_text(
        &state,
        vec![
            ChatMessage {
                role: "system".into(),
                content: "Return JSON only.".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: prompts::prioritize_prompt(&json_todos),
            },
        ],
        true,
    )
    .await?;
    let parsed = parse_prioritize(&outcome.response.text)
        .ok_or_else(|| AppError::msg("Model returned invalid JSON"))?;
    Ok(
        json!({ "result": parsed, "fallbackUsed": outcome.fallback_used, "model": outcome.response.model }),
    )
}

#[tauri::command]
pub fn ai_apply_todo_priorities(state: State<AppState>, order: Vec<String>) -> AppResult<()> {
    state.db.with(|c| {
        for (i, id) in order.iter().enumerate() {
            let priority = match i {
                0 | 1 => "urgent",
                2 | 3 => "high",
                4 | 5 => "medium",
                _ => "low",
            };
            ai_store::update_todo_priority(c, id, priority)?;
        }
        Ok(())
    })
}

#[tauri::command]
pub async fn ai_generate_prompt(
    state: State<'_, AppState>,
    project_id: String,
    extra: Option<String>,
) -> AppResult<serde_json::Value> {
    require_software(&state, &project_id)?;
    let bundle = state
        .db
        .with(|c| ai_store::ai_project_bundle(c, &project_id))?;
    let outcome = complete_text(
        &state,
        vec![
            ChatMessage {
                role: "system".into(),
                content: "Return JSON only.".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: prompts::agent_prompt_builder(
                    &prompts::bundle_to_prompt(&bundle),
                    extra.as_deref().unwrap_or(""),
                ),
            },
        ],
        true,
    )
    .await?;
    let parsed = parse_agent_prompt(&outcome.response.text)
        .ok_or_else(|| AppError::msg("Model returned invalid JSON"))?;
    Ok(json!({ "result": parsed, "fallbackUsed": outcome.fallback_used }))
}

#[tauri::command]
pub async fn ai_summarize_changelog(
    state: State<'_, AppState>,
    project_id: String,
    version_id: Option<String>,
) -> AppResult<String> {
    require_software(&state, &project_id)?;
    let versions = state.db.with(|c| db::list_versions(c, &project_id))?;
    let body = if let Some(id) = version_id {
        versions
            .iter()
            .find(|v| v.id == id)
            .map(|v| v.changelog.clone())
            .unwrap_or_default()
    } else {
        versions
            .first()
            .map(|v| v.changelog.clone())
            .unwrap_or_default()
    };
    if body.trim().is_empty() {
        return Err(AppError::msg("No changelog text to summarize"));
    }
    let outcome = complete_text(
        &state,
        vec![ChatMessage {
            role: "user".into(),
            content: prompts::changelog_summarize_prompt(&body),
        }],
        false,
    )
    .await?;
    Ok(outcome.response.text)
}

#[tauri::command]
pub async fn ai_generate_changelog(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<serde_json::Value> {
    require_software(&state, &project_id)?;
    let (git_path, folder, versions) = state.db.with(|c| {
        let git = db::load_settings(c).git_path;
        let (local, _) = db::project_binding(c, &state.db.device_id, &project_id);
        Ok((git, local, db::list_versions(c, &project_id)?))
    })?;
    let path = folder.ok_or(AppError::FolderMissing)?;
    let last = versions.first().map(|v| v.version.clone());
    let log = crate::git::log_since(&git_path, std::path::Path::new(&path), last.as_deref());
    let version_text = versions
        .iter()
        .take(6)
        .map(|v| format!("{} {}", v.version, v.changelog))
        .collect::<Vec<_>>()
        .join("\n");
    let outcome = complete_text(
        &state,
        vec![
            ChatMessage {
                role: "system".into(),
                content: "Return JSON only.".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: prompts::changelog_generate_prompt(&log, &version_text),
            },
        ],
        true,
    )
    .await?;
    let parsed = parse_changelog(&outcome.response.text)
        .ok_or_else(|| AppError::msg("Model returned invalid JSON"))?;
    Ok(json!({ "result": parsed, "gitLog": log, "fallbackUsed": outcome.fallback_used }))
}

#[tauri::command]
pub async fn ai_todos_from_changelog(
    state: State<'_, AppState>,
    project_id: String,
    changelog: String,
) -> AppResult<serde_json::Value> {
    require_software(&state, &project_id)?;
    let outcome = complete_text(
        &state,
        vec![
            ChatMessage {
                role: "system".into(),
                content: "Return JSON only.".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: prompts::todos_from_changelog_prompt(&changelog),
            },
        ],
        true,
    )
    .await?;
    let parsed = parse_todos(&outcome.response.text)
        .ok_or_else(|| AppError::msg("Model returned invalid JSON"))?;
    Ok(json!({ "todos": parsed, "fallbackUsed": outcome.fallback_used }))
}

#[tauri::command]
pub async fn ai_suggest_commit(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<serde_json::Value> {
    require_software(&state, &project_id)?;
    let (git_path, folder, follow) = state.db.with(|c| {
        let settings = db::load_settings(c);
        let ai = ai_store::load_ai_settings(c);
        let (local, _) = db::project_binding(c, &state.db.device_id, &project_id);
        Ok((settings.git_path, local, ai.commit_follow_style))
    })?;
    let path = folder.ok_or(AppError::FolderMissing)?;
    let ctx = crate::git::diff_context(&git_path, std::path::Path::new(&path));
    if !ctx.dirty && ctx.diff.trim().is_empty() {
        return Err(AppError::msg("Working tree is clean — nothing to commit"));
    }
    let log = ctx.subjects.join("\n");
    let outcome = complete_text(
        &state,
        vec![
            ChatMessage {
                role: "system".into(),
                content: "Return JSON only. Never commit.".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: prompts::commit_prompt(&ctx.diff, &log, follow),
            },
        ],
        true,
    )
    .await?;
    let parsed = parse_commit(&outcome.response.text)
        .ok_or_else(|| AppError::msg("Model returned invalid JSON"))?;
    Ok(json!({
        "result": parsed,
        "styleFollowed": follow && ctx.has_history,
        "hasHistory": ctx.has_history,
        "truncated": ctx.truncated,
        "fallbackUsed": outcome.fallback_used,
    }))
}

#[tauri::command]
pub async fn ai_inspect_software_folder(
    state: State<'_, AppState>,
    local_path: String,
) -> AppResult<serde_json::Value> {
    let path = std::path::PathBuf::from(local_path.trim());
    if local_path.trim().is_empty() {
        return Err(AppError::FolderMissing);
    }
    let (git_path, tags) = state
        .db
        .with(|c| Ok((db::load_settings(c).git_path, db::list_tags(c)?)))?;
    let mut facts = crate::project_facts::collect(&path, &git_path)?;
    let languages: Vec<String> = tags
        .iter()
        .filter(|t| t.kind == "language")
        .map(|t| t.name.clone())
        .collect();
    let frameworks: Vec<String> = tags
        .iter()
        .filter(|t| t.kind == "framework")
        .map(|t| t.name.clone())
        .collect();
    let inspect_messages = |evidence: &str| {
        vec![
            ChatMessage {
                role: "system".into(),
                content: "You inspect a local software folder. Read the tree and excerpts, then infer a human project title and a grounded 1-2 sentence description. Return JSON only. Never write or modify files. Never invent URLs.".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: prompts::inspect_software_folder_prompt(
                    evidence,
                    &languages.join(", "),
                    &frameworks.join(", "),
                ),
            },
        ]
    };
    let mut outcome = complete_text(&state, inspect_messages(&facts.evidence), true).await?;
    let mut parsed = parse_software_folder_draft(&outcome.response.text)
        .ok_or_else(|| AppError::msg("Model returned invalid JSON"))?;
    let needed = requested_inspect_files(&outcome.response.text);
    if (parsed.name.trim().is_empty() || parsed.description.trim().is_empty()) && !needed.is_empty()
    {
        let extra = crate::project_facts::read_extra_files(&path, &needed);
        if !extra.trim().is_empty() {
            facts.evidence.push_str(&extra);
            outcome = complete_text(&state, inspect_messages(&facts.evidence), true).await?;
            if let Some(again) = parse_software_folder_draft(&outcome.response.text) {
                parsed = again;
            }
        }
    }
    let merged = crate::project_facts::merge_draft(
        &facts,
        crate::project_facts::SoftwareFolderDraft {
            name: parsed.name,
            description: parsed.description,
            github_url: parsed.github_url,
            website_url: parsed.website_url,
            languages: parsed.languages,
            frameworks: parsed.frameworks,
        },
    );
    Ok(json!({
        "result": {
            "name": merged.name,
            "description": merged.description,
            "githubUrl": merged.github_url,
            "websiteUrl": merged.website_url,
            "languageIds": map_tag_ids(&tags, &merged.languages, "language"),
            "frameworkIds": map_tag_ids(&tags, &merged.frameworks, "framework"),
        },
        "fallbackUsed": outcome.fallback_used,
    }))
}

#[tauri::command]
pub async fn ai_scan_commands(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<serde_json::Value> {
    require_software(&state, &project_id)?;
    let (git_path, folder, existing) = state.db.with(|c| {
        let settings = db::load_settings(c);
        let (local, _) = db::project_binding(c, &state.db.device_id, &project_id);
        let commands = db::list_commands(c, &project_id)?;
        Ok((settings.git_path, local, commands))
    })?;
    let path = folder.ok_or(AppError::FolderMissing)?;
    let root = std::path::PathBuf::from(&path);
    let mut facts = crate::project_facts::collect(&root, &git_path)?;
    let found = crate::command_scan::extract_found_commands(&root)?;
    let existing_lines: Vec<String> = existing.iter().map(|c| c.command.clone()).collect();
    let found_json = serde_json::to_string(&found).unwrap_or_else(|_| "[]".into());
    let existing_json = serde_json::to_string(&existing_lines).unwrap_or_else(|_| "[]".into());
    let scan_messages = |evidence: &str| {
        vec![
            ChatMessage {
                role: "system".into(),
                content: "You scan a local software folder for runnable commands. Return JSON only. Never write or modify files. Never run commands.".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: prompts::scan_commands_prompt(evidence, &found_json, &existing_json),
            },
        ]
    };
    let mut outcome = complete_text(&state, scan_messages(&facts.evidence), true).await?;
    let mut parsed = parse_command_scan(&outcome.response.text).unwrap_or_default();
    let needed = requested_inspect_files(&outcome.response.text);
    if parsed.is_empty() && !needed.is_empty() {
        let extra = crate::project_facts::read_extra_files(&root, &needed);
        if !extra.trim().is_empty() {
            facts.evidence.push_str(&extra);
            outcome = complete_text(&state, scan_messages(&facts.evidence), true).await?;
            if let Some(again) = parse_command_scan(&outcome.response.text) {
                parsed = again;
            }
        }
    }
    let merged = merge_scan_commands(found, parsed, &existing);
    if merged.is_empty() {
        return Err(AppError::msg("No commands found in project configs"));
    }
    Ok(json!({
        "commands": merged,
        "fallbackUsed": outcome.fallback_used,
    }))
}

fn merge_scan_commands(
    found: Vec<crate::command_scan::FoundCommand>,
    model: Vec<CommandProposal>,
    existing: &[CommandDto],
) -> Vec<serde_json::Value> {
    use crate::command_scan::normalize_command;
    use std::collections::{HashMap, HashSet};

    let existing_keys: HashSet<String> = existing
        .iter()
        .map(|c| normalize_command(&c.command))
        .collect();
    let mut order: Vec<String> = Vec::new();
    let mut by_key: HashMap<String, CommandProposal> = HashMap::new();

    let mut consider = |item: CommandProposal| {
        let cmd_key = normalize_command(&item.command);
        if cmd_key.is_empty() || existing_keys.contains(&cmd_key) {
            return;
        }
        let key = format!(
            "{}|{}",
            cmd_key,
            item.working_directory.as_deref().unwrap_or("")
        );
        if let Some(current) = by_key.get_mut(&key) {
            if current.description.is_empty() && !item.description.is_empty() {
                current.description = item.description;
            }
            if current.reasoning.is_empty() && !item.reasoning.is_empty() {
                current.reasoning = item.reasoning;
            }
            return;
        }
        order.push(key.clone());
        by_key.insert(key, item);
    };

    for item in found {
        consider(CommandProposal {
            name: item.name,
            command: item.command,
            description: item.description,
            working_directory: item.working_directory,
            source: "found".into(),
            reasoning: item.reasoning,
        });
    }
    for item in model {
        consider(item);
    }

    order
        .into_iter()
        .take(20)
        .filter_map(|key| by_key.remove(&key))
        .map(|item| {
            json!({
                "name": item.name,
                "command": item.command,
                "description": item.description,
                "workingDirectory": item.working_directory,
                "source": item.source,
                "reasoning": item.reasoning,
                "dangerous": is_dangerous(&item.command),
            })
        })
        .collect()
}

fn map_tag_ids(tags: &[TagDto], names: &[String], kind: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for name in names {
        let Some(id) = tags
            .iter()
            .find(|t| t.kind == kind && t.name.eq_ignore_ascii_case(name.trim()))
            .map(|t| t.id.clone())
        else {
            continue;
        };
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

#[tauri::command]
pub fn git_log(state: State<AppState>, project_id: String) -> AppResult<Vec<String>> {
    state.db.with(|c| {
        let git = db::load_settings(c).git_path;
        let (local, _) = db::project_binding(c, &state.db.device_id, &project_id);
        let path = local.ok_or(AppError::FolderMissing)?;
        Ok(crate::git::recent_subjects(
            &git,
            std::path::Path::new(&path),
            20,
        ))
    })
}

#[tauri::command]
pub fn git_diff(
    state: State<AppState>,
    project_id: String,
) -> AppResult<crate::git::GitDiffContext> {
    state.db.with(|c| {
        let git = db::load_settings(c).git_path;
        let (local, _) = db::project_binding(c, &state.db.device_id, &project_id);
        let path = local.ok_or(AppError::FolderMissing)?;
        Ok(crate::git::diff_context(&git, std::path::Path::new(&path)))
    })
}

#[tauri::command]
pub fn list_document_records(
    state: State<AppState>,
    project_id: String,
) -> AppResult<Vec<DocumentRecordDto>> {
    state
        .db
        .with(|c| ai_store::list_document_records(c, &project_id))
}

#[cfg(test)]
mod tests {
    use bluephoenix_ai::{require_ready, AiSettings};

    #[test]
    fn chat_off_when_disabled() {
        let mut s = AiSettings::default();
        s.enabled = false;
        s.models[0] = "x".into();
        assert!(require_ready(&s, true).is_err());
    }
}
