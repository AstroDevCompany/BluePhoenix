use crate::error::{AiError, AiFailureKind, AiResult, AttemptRecord};
use crate::fallback::FallbackOutcome;
use crate::provider::{classify_http, AiProvider, CompletionRequest, CompletionResponse};
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::{json, Value};

pub const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

pub struct OpenRouterProvider {
    pub api_key: String,
    pub http: reqwest::Client,
    pub base_url: String,
}

impl OpenRouterProvider {
    pub fn new(api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(45))
            .build()
            .expect("http client");
        Self {
            api_key,
            http,
            base_url: OPENROUTER_URL.into(),
        }
    }

    fn body(request: &CompletionRequest) -> Value {
        let mut body = json!({
            "model": request.model,
            "messages": request.messages.iter().map(|m| json!({"role": m.role, "content": m.content})).collect::<Vec<_>>(),
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
        });
        if request.json_mode {
            body["response_format"] = json!({ "type": "json_object" });
        }
        body
    }

    pub async fn stream(
        &self,
        request: CompletionRequest,
        mut on_delta: impl FnMut(&str) + Send,
    ) -> AiResult<CompletionResponse> {
        if self.api_key.trim().is_empty() {
            return Err(AiError::MissingKey);
        }
        let model = request.model.clone();
        let mut body = Self::body(&request);
        body["stream"] = json!(true);
        let res = self
            .http
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://bluephoenix.app")
            .header("X-Title", "BluePhoenix")
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                let kind = if err.is_timeout() {
                    AiFailureKind::Timeout
                } else {
                    AiFailureKind::Network
                };
                AiError::provider(kind, "OpenRouter is unreachable", Some(model.clone()))
            })?;
        let status = res.status().as_u16();
        if !(200..300).contains(&status) {
            let text = res.text().await.unwrap_or_default();
            let kind = classify_http(status, &text);
            return Err(AiError::provider(kind, sanitize_provider_body(&text), Some(model)));
        }
        let mut stream = res.bytes_stream();
        let mut buffer = String::new();
        let mut assembled = String::new();
        let mut used_model = model.clone();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|_| {
                AiError::provider(AiFailureKind::Network, "Stream interrupted", Some(model.clone()))
            })?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(idx) = buffer.find('\n') {
                let line = buffer[..idx].trim().to_string();
                buffer = buffer[idx + 1..].to_string();
                if line.is_empty() {
                    continue;
                }
                if let Some((delta, maybe_model, done)) = parse_sse_delta(&line) {
                    if let Some(m) = maybe_model {
                        used_model = m;
                    }
                    if !delta.is_empty() {
                        assembled.push_str(&delta);
                        on_delta(&delta);
                    }
                    if done {
                        break;
                    }
                }
            }
        }
        if assembled.is_empty() {
            return Err(AiError::provider(
                AiFailureKind::Malformed,
                "Provider stream had no text",
                Some(model),
            ));
        }
        Ok(CompletionResponse {
            text: assembled,
            model: used_model,
            finish_reason: Some("stop".into()),
        })
    }
}

#[async_trait]
impl AiProvider for OpenRouterProvider {
    async fn complete(&self, request: CompletionRequest) -> AiResult<CompletionResponse> {
        if self.api_key.trim().is_empty() {
            return Err(AiError::MissingKey);
        }
        let model = request.model.clone();
        let res = self
            .http
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://bluephoenix.app")
            .header("X-Title", "BluePhoenix")
            .json(&Self::body(&request))
            .send()
            .await
            .map_err(|err| {
                let kind = if err.is_timeout() {
                    AiFailureKind::Timeout
                } else {
                    AiFailureKind::Network
                };
                AiError::provider(kind, "OpenRouter is unreachable", Some(model.clone()))
            })?;
        let status = res.status().as_u16();
        let text = res.text().await.unwrap_or_default();
        if !(200..300).contains(&status) {
            let kind = classify_http(status, &text);
            let snippet = sanitize_provider_body(&text);
            return Err(AiError::provider(kind, snippet, Some(model)));
        }
        parse_completion(&text, &model)
    }
}

pub async fn stream_with_fallback(
    provider: &OpenRouterProvider,
    models: &[String],
    mut request: CompletionRequest,
    mut on_delta: impl FnMut(&str) + Send,
) -> AiResult<FallbackOutcome> {
    if models.is_empty() {
        return Err(AiError::NoModels);
    }
    let mut attempts = Vec::new();
    for (index, model) in models.iter().take(4).enumerate() {
        request.model = model.clone();
        let mut got_token = false;
        let result = provider
            .stream(request.clone(), |delta| {
                if !delta.is_empty() {
                    got_token = true;
                }
                on_delta(delta);
            })
            .await;
        match result {
            Ok(response) => {
                return Ok(FallbackOutcome {
                    fallback_used: index > 0,
                    attempted: {
                        let mut used: Vec<String> = attempts.iter().map(|a: &AttemptRecord| a.model.clone()).collect();
                        used.push(model.clone());
                        used
                    },
                    response,
                });
            }
            Err(err) => {
                let kind = err.kind().unwrap_or(AiFailureKind::Other);
                attempts.push(AttemptRecord {
                    model: model.clone(),
                    kind,
                    message: err.user_message(),
                });
                if got_token || !kind.should_fallback() {
                    return Err(err);
                }
            }
        }
    }
    Err(AiError::AllFailed { attempts })
}

pub fn parse_sse_delta(line: &str) -> Option<(String, Option<String>, bool)> {
    let data = line.strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return Some((String::new(), None, true));
    }
    let value: Value = serde_json::from_str(data).ok()?;
    let delta = value
        .pointer("/choices/0/delta/content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let model = value.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());
    let done = value
        .pointer("/choices/0/finish_reason")
        .and_then(|v| v.as_str())
        .is_some();
    Some((delta, model, done))
}

pub fn parse_completion(body: &str, model: &str) -> AiResult<CompletionResponse> {
    let value: Value = serde_json::from_str(body).map_err(|_| {
        AiError::provider(AiFailureKind::Malformed, "Provider returned invalid JSON", Some(model.into()))
    })?;
    let text = value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            value
                .pointer("/choices/0/message/content")
                .and_then(|v| v.as_array())
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join("")
                })
        })
        .ok_or_else(|| {
            AiError::provider(AiFailureKind::Malformed, "Provider response had no text", Some(model.into()))
        })?;
    let used_model = value
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or(model)
        .to_string();
    Ok(CompletionResponse {
        text,
        model: used_model,
        finish_reason: value
            .pointer("/choices/0/finish_reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}

fn sanitize_provider_body(body: &str) -> String {
    let lower = body.to_lowercase();
    if lower.contains("sk-or-") || lower.contains("authorization") {
        return "OpenRouter rejected the request".into();
    }
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| {
            v.pointer("/error/message")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| {
            let trimmed = body.trim();
            if trimmed.len() > 180 {
                format!("{}…", &trimmed[..180])
            } else if trimmed.is_empty() {
                "OpenRouter request failed".into()
            } else {
                trimmed.to_string()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chat_completion() {
        let body = r#"{"model":"x","choices":[{"message":{"content":"ok"},"finish_reason":"stop"}]}"#;
        let parsed = parse_completion(body, "fallback").unwrap();
        assert_eq!(parsed.text, "ok");
        assert_eq!(parsed.model, "x");
    }

    #[test]
    fn parses_sse_and_done() {
        let (delta, model, done) = parse_sse_delta(
            r#"data: {"model":"m","choices":[{"delta":{"content":"Hi"}}]}"#,
        )
        .unwrap();
        assert_eq!(delta, "Hi");
        assert_eq!(model.as_deref(), Some("m"));
        assert!(!done);
        let done = parse_sse_delta("data: [DONE]").unwrap();
        assert!(done.2);
    }
}
