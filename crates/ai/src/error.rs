use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiFailureKind {
    Unavailable,
    RateLimit,
    Timeout,
    Network,
    Malformed,
    Auth,
    Validation,
    Other,
}

impl AiFailureKind {
    pub fn should_fallback(self) -> bool {
        matches!(
            self,
            Self::Unavailable | Self::RateLimit | Self::Timeout | Self::Network | Self::Malformed | Self::Other
        )
    }
}

#[derive(Debug, Error, Clone)]
pub enum AiError {
    #[error("AI is disabled")]
    Disabled,
    #[error("OpenRouter API key is not configured")]
    MissingKey,
    #[error("No models are configured")]
    NoModels,
    #[error("{kind:?}: {message}")]
    Provider {
        kind: AiFailureKind,
        message: String,
        model: Option<String>,
    },
    #[error("All configured models failed")]
    AllFailed { attempts: Vec<AttemptRecord> },
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AttemptRecord {
    pub model: String,
    pub kind: AiFailureKind,
    pub message: String,
}

impl AiError {
    pub fn provider(kind: AiFailureKind, message: impl Into<String>, model: Option<String>) -> Self {
        Self::Provider {
            kind,
            message: message.into(),
            model,
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Disabled => "AI features are turned off in Settings.".into(),
            Self::MissingKey => "Add an OpenRouter API key in Settings → AI.".into(),
            Self::NoModels => "Configure at least one OpenRouter model identifier.".into(),
            Self::Provider { kind, message, model } => {
                let model = model.as_deref().unwrap_or("model");
                match kind {
                    AiFailureKind::Auth => "The OpenRouter API key was rejected. Check Settings → AI.".into(),
                    AiFailureKind::Validation => format!("The request was rejected: {message}"),
                    _ => format!("{model}: {message}"),
                }
            }
            Self::AllFailed { attempts } => {
                let last = attempts.last().map(|a| a.message.as_str()).unwrap_or("unknown error");
                format!("All fallback models failed. Last error: {last}")
            }
            Self::Message(m) => m.clone(),
        }
    }

    pub fn kind(&self) -> Option<AiFailureKind> {
        match self {
            Self::Provider { kind, .. } => Some(*kind),
            Self::AllFailed { attempts } => attempts.last().map(|a| a.kind),
            _ => None,
        }
    }
}

pub type AiResult<T> = Result<T, AiError>;
