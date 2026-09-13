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

    async fn stream(
        &self,
        request: CompletionRequest,
        on_delta: &mut (dyn FnMut(String) + Send),
    ) -> AiResult<CompletionResponse>;
}

pub fn classify_http(status: u16, body: &str) -> AiFailureKind {
    if matches!(status, 401 | 403) {
        return AiFailureKind::Auth;
    }
    if status == 429 || crate::compat::looks_like_rate_limit(body) {
        return AiFailureKind::RateLimit;
    }
    match status {
        404 => AiFailureKind::Unavailable,
        408 => AiFailureKind::Timeout,
        400 | 422 => {
            let lower = body.to_lowercase();
            if lower.contains("api key") || lower.contains("unauthorized") {
                AiFailureKind::Auth
            } else if crate::compat::looks_like_provider_param_error(body) {
                // Google/OpenRouter wrap unsupported JSON mode as 400 "Provider
                // returned error". Treat as unavailable so the next fallback slot runs.
                AiFailureKind::Unavailable
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_mode_provider_error_is_unavailable() {
        let body = r#"{"error":{"message":"Provider returned error","code":400,"metadata":{"raw":"JSON mode is not enabled"}}}"#;
        assert_eq!(classify_http(400, body), AiFailureKind::Unavailable);
    }

    #[test]
    fn generic_400_stays_validation() {
        assert_eq!(
            classify_http(
                400,
                r#"{"error":{"message":"max_tokens must be positive"}}"#
            ),
            AiFailureKind::Validation
        );
    }

    #[test]
    fn upstream_free_cap_is_rate_limit() {
        let body = r#"{"error":{"message":"google/gemma-4-31b-it:free is temporarily rate-limited upstream. Please retry shortly, or add your own key to accumulate your rate limits: https://openrouter.ai/settings/integrations"}}"#;
        assert_eq!(classify_http(429, body), AiFailureKind::RateLimit);
        assert_eq!(classify_http(502, body), AiFailureKind::RateLimit);
    }
}
