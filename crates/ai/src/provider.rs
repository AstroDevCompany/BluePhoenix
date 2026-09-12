use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{AiFailureKind, AiResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub json_mode: bool,
}

impl CompletionRequest {
    pub fn chat(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: 0.4,
            max_tokens: 1800,
            json_mode: false,
        }
    }

    pub fn json(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            json_mode: true,
            temperature: 0.1,
            max_tokens: 2200,
            ..Self::chat(model, messages)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResponse {
    pub text: String,
    pub model: String,
    pub finish_reason: Option<String>,
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> AiResult<CompletionResponse>;
}

pub fn classify_http(status: u16, body: &str) -> AiFailureKind {
    match status {
        401 | 403 => AiFailureKind::Auth,
        404 => AiFailureKind::Unavailable,
        408 => AiFailureKind::Timeout,
        429 => AiFailureKind::RateLimit,
        400 | 422 => {
            let lower = body.to_lowercase();
            if lower.contains("api key") || lower.contains("unauthorized") {
                AiFailureKind::Auth
            } else {
                AiFailureKind::Validation
            }
        }
        500..=599 => AiFailureKind::Unavailable,
        _ => AiFailureKind::Other,
    }
}

pub fn redact(value: &str) -> String {
    if value.len() <= 8 {
        return "••••".into();
    }
    format!("{}…{}", &value[..4], &value[value.len() - 2..])
}
