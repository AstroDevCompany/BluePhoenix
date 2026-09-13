mod gguf;
mod grammar;
mod provider;
mod worker;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use crate::error::{AiError, AiFailureKind, AiResult};
use crate::provider::{CompletionRequest, CompletionResponse};

pub use gguf::{inspect_gguf, GgufInfo};
pub use provider::LocalLlamaProvider;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalModelSpec {
    pub id: String,
    pub name: String,
    pub path: String,
    pub ctx_len: u32,
    pub gpu_layers: u32,
    pub threads: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineState {
    Unloaded,
    Loading { model_id: String },
    Ready { model_id: String },
    Busy { model_id: String },
    Error { message: String },
}

impl EngineState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Unloaded => "unloaded",
            Self::Loading { .. } => "loading",
            Self::Ready { .. } => "ready",
            Self::Busy { .. } => "busy",
            Self::Error { .. } => "error",
        }
    }

    pub fn model_id(&self) -> Option<&str> {
        match self {
            Self::Loading { model_id } | Self::Ready { model_id } | Self::Busy { model_id } => {
                Some(model_id)
            }
            Self::Unloaded | Self::Error { .. } => None,
        }
    }

    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Error { message } => Some(message),
            _ => None,
        }
    }
}

pub(crate) enum Job {
    Complete {
        spec: LocalModelSpec,
        request: CompletionRequest,
        delta_tx: tokio::sync::mpsc::UnboundedSender<String>,
        result_tx: tokio::sync::oneshot::Sender<AiResult<CompletionResponse>>,
    },
    Unload,
}

pub(crate) struct Inner {
    tx: Mutex<Option<Sender<Job>>>,
    state: Mutex<EngineState>,
    idle_minutes: AtomicU32,
}

#[derive(Clone)]
pub struct LocalEngine {
    inner: Arc<Inner>,
}

impl Default for LocalEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalEngine {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                tx: Mutex::new(None),
                state: Mutex::new(EngineState::Unloaded),
                idle_minutes: AtomicU32::new(0),
            }),
        }
    }

    pub fn state(&self) -> EngineState {
        lock_state(&self.inner).clone()
    }

    pub fn set_idle_unload(&self, minutes: u32) {
        self.inner.idle_minutes.store(minutes, Ordering::Relaxed);
    }

    pub fn unload(&self) {
        if self.ensure_thread().is_err() {
            set_state(&self.inner, EngineState::Unloaded);
            return;
        }
        if let Some(tx) = lock_tx(&self.inner).clone() {
            let _ = tx.send(Job::Unload);
        }
    }

    pub async fn submit(
        &self,
        spec: LocalModelSpec,
        request: CompletionRequest,
        on_delta: &mut (dyn FnMut(String) + Send),
    ) -> AiResult<CompletionResponse> {
        self.ensure_thread()?;
        let (delta_tx, mut delta_rx) = tokio::sync::mpsc::unbounded_channel();
        let (result_tx, mut result_rx) = tokio::sync::oneshot::channel();
        {
            let tx = lock_tx(&self.inner)
                .clone()
                .ok_or_else(|| AiError::Message("Local model worker is not running".into()))?;
            tx.send(Job::Complete {
                spec,
                request,
                delta_tx,
                result_tx,
            })
            .map_err(|_| {
                AiError::provider(
                    AiFailureKind::Unavailable,
                    "Local model worker stopped",
                    None,
                )
            })?;
        }
        loop {
            tokio::select! {
                Some(delta) = delta_rx.recv() => {
                    on_delta(delta);
                }
                result = &mut result_rx => {
                    while let Ok(delta) = delta_rx.try_recv() {
                        on_delta(delta);
                    }
                    return result.unwrap_or_else(|_| {
                        Err(AiError::provider(
                            AiFailureKind::Unavailable,
                            "Local model worker stopped",
                            None,
                        ))
                    });
                }
            }
        }
    }

    fn ensure_thread(&self) -> AiResult<()> {
        let mut slot = lock_tx(&self.inner);
        if slot.is_some() {
            return Ok(());
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let inner = Arc::clone(&self.inner);
        std::thread::Builder::new()
            .name("bluephoenix-llama".into())
            .spawn(move || worker::run(rx, inner))
            .map_err(|err| {
                AiError::Message(format!("Could not start local model thread: {err}"))
            })?;
        *slot = Some(tx);
        Ok(())
    }
}

pub fn default_thread_count() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).max(1) as u32)
        .unwrap_or(4)
}

fn lock_tx(inner: &Inner) -> std::sync::MutexGuard<'_, Option<Sender<Job>>> {
    inner.tx.lock().unwrap_or_else(|err| err.into_inner())
}

fn lock_state(inner: &Inner) -> std::sync::MutexGuard<'_, EngineState> {
    inner.state.lock().unwrap_or_else(|err| err.into_inner())
}

pub(crate) fn set_state(inner: &Inner, state: EngineState) {
    *lock_state(inner) = state;
}
