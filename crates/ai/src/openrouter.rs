use crate::compat::{
    self, chat_completions_body, completion_text, paid_variant, provider_error_text,
    split_message_text,
};
use crate::error::{AiError, AiFailureKind, AiResult};
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
        chat_completions_body(request)
    }

    async fn stream_once(
        &self,
        request: CompletionRequest,
        on_delta: &mut impl FnMut(&str),
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
            return Err(AiError::provider(
                kind,
                sanitize_provider_body(&text),
                Some(model),
            ));
        }
        let mut stream = res.bytes_stream();
        let mut buffer = String::new();
        let mut assembled = String::new();
        let mut reasoning = String::new();
        let mut used_model = model.clone();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|_| {
                AiError::provider(
                    AiFailureKind::Network,
                    "Stream interrupted",
                    Some(model.clone()),
                )
            })?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(idx) = buffer.find('\n') {
                let line = buffer[..idx].trim().to_string();
                buffer = buffer[idx + 1..].to_string();
                if line.is_empty() {
                    continue;
                }
                if let Some(err) = sse_error_message(&line) {
                    return Err(AiError::provider(
                        classify_http(502, &err),
                        provider_error_text(&err),
                        Some(model.clone()),
                    ));
                }
                if let Some((delta, maybe_model, done)) = parse_sse_delta(&line) {
                    if let Some(m) = maybe_model {
                        used_model = m;
                    }
                    if let Some(hidden) = sse_reasoning_delta(&line) {
                        reasoning.push_str(&hidden);
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
        if assembled.trim().is_empty() {
            assembled = compat::strip_thinking_wrappers(&reasoning);
        } else {
            assembled = compat::strip_thinking_wrappers(&assembled);
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
        let original = request.model.clone();
        match self.complete_once(&request).await {
            Err(err) if should_retry_paid(&original, &err) => {
                let mut paid = request;
                paid.model = paid_variant(&original).unwrap_or(original);
                self.complete_once(&paid).await
            }
            other => other,
        }
    }

    async fn stream(
        &self,
        request: CompletionRequest,
        on_delta: &mut (dyn FnMut(String) + Send),
    ) -> AiResult<CompletionResponse> {
        let original = request.model.clone();
        match self
            .stream_once(request.clone(), &mut |delta| on_delta(delta.to_string()))
            .await
        {
            Err(err) if should_retry_paid(&original, &err) => {
                let mut paid = request;
                paid.model = paid_variant(&original).unwrap_or(original);
                self.stream_once(paid, &mut |delta| on_delta(delta.to_string()))
                    .await
            }
            other => other,
        }
    }
}

impl OpenRouterProvider {
    async fn complete_once(&self, request: &CompletionRequest) -> AiResult<CompletionResponse> {
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
            .json(&Self::body(request))
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

fn should_retry_paid(model: &str, err: &AiError) -> bool {
    if paid_variant(model).is_none() {
        return false;
    }
    match err {
        AiError::Provider { kind, message, .. } => {
            *kind == AiFailureKind::RateLimit || crate::compat::looks_like_rate_limit(message)
        }
        _ => false,
    }
}

fn sse_error_message(line: &str) -> Option<String> {
    let data = line.strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return None;
    }
    let value: Value = serde_json::from_str(data).ok()?;
    if value.get("error").is_some() {
        Some(data.to_string())
    } else {
        None
    }
}

fn sse_reasoning_delta(line: &str) -> Option<String> {
    let data = line.strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return None;
    }
    let value: Value = serde_json::from_str(data).ok()?;
    let hidden = split_message_text(value.pointer("/choices/0/delta")?).hidden;
    if hidden.is_empty() {
        None
    } else {
        Some(hidden)
    }
}

pub fn parse_sse_delta(line: &str) -> Option<(String, Option<String>, bool)> {
    let data = line.strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return Some((String::new(), None, true));
    }
    let value: Value = serde_json::from_str(data).ok()?;
    if value.get("error").is_some() {
        return None;
    }
    let delta = value
        .pointer("/choices/0/delta")
        .map(|node| split_message_text(node).visible)
        .unwrap_or_default();
    let model = value
        .get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let done = value
        .pointer("/choices/0/finish_reason")
        .and_then(|v| v.as_str())
        .is_some();
    Some((delta, model, done))
}

pub fn parse_completion(body: &str, model: &str) -> AiResult<CompletionResponse> {
    let value: Value = serde_json::from_str(body).map_err(|_| {
        AiError::provider(
            AiFailureKind::Malformed,
            "Provider returned invalid JSON",
            Some(model.into()),
        )
    })?;
    if value.get("error").is_some() && value.pointer("/choices/0").is_none() {
        return Err(AiError::provider(
            classify_http(502, body),
            provider_error_text(body),
            Some(model.into()),
        ));
    }
    let text = completion_text(&value);
    if text.trim().is_empty() {
        return Err(AiError::provider(
            AiFailureKind::Malformed,
            "Provider response had no text",
            Some(model.into()),
        ));
    }
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
    provider_error_text(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chat_completion() {
        let body =
            r#"{"model":"x","choices":[{"message":{"content":"ok"},"finish_reason":"stop"}]}"#;
        let parsed = parse_completion(body, "fallback").unwrap();
        assert_eq!(parsed.text, "ok");
        assert_eq!(parsed.model, "x");
    }

    #[test]
    fn parses_sse_and_done() {
        let (delta, model, done) =
            parse_sse_delta(r#"data: {"model":"m","choices":[{"delta":{"content":"Hi"}}]}"#)
                .unwrap();
        assert_eq!(delta, "Hi");
        assert_eq!(model.as_deref(), Some("m"));
        assert!(!done);
        let done = parse_sse_delta("data: [DONE]").unwrap();
        assert!(done.2);
    }

    #[test]
    fn parses_array_content_and_reasoning_fallback() {
        let claude = r#"{"model":"anthropic/claude-sonnet-4.5","choices":[{"message":{"content":[{"type":"text","text":"{\"ok\":true}"}]},"finish_reason":"stop"}]}"#;
        assert_eq!(
            parse_completion(claude, "fallback").unwrap().text,
            "{\"ok\":true}"
        );

        let qwen = r#"{"model":"qwen/qwen3-32b","choices":[{"message":{"content":"","reasoning_content":"{\"ok\":true}"},"finish_reason":"stop"}]}"#;
        assert_eq!(
            parse_completion(qwen, "fallback").unwrap().text,
            "{\"ok\":true}"
        );

        let gemini = r#"{"model":"google/gemini-2.5-flash","choices":[{"message":{"content":[{"type":"text","text":"hi"}]},"finish_reason":"stop"}]}"#;
        assert_eq!(parse_completion(gemini, "fallback").unwrap().text, "hi");

        let gpt = r#"{"model":"openai/gpt-5","choices":[{"message":{"content":[{"type":"output_text","text":"done"}]},"finish_reason":"stop"}]}"#;
        assert_eq!(parse_completion(gpt, "fallback").unwrap().text, "done");

        let deepseek = r#"{"model":"deepseek/deepseek-r1","choices":[{"message":{"content":"4","reasoning_content":"count"},"finish_reason":"stop"}]}"#;
        assert_eq!(parse_completion(deepseek, "fallback").unwrap().text, "4");

        let gemma = r#"{"model":"google/gemma-4-31b-it:free","choices":[{"message":{"content":"<|channel>thought\nplan\n<channel|>{\"ok\":true}"},"finish_reason":"stop"}]}"#;
        assert_eq!(
            parse_completion(gemma, "fallback").unwrap().text,
            "{\"ok\":true}"
        );
    }

    #[test]
    fn parses_sse_array_delta() {
        let (delta, _, _) = parse_sse_delta(
            r#"data: {"choices":[{"delta":{"content":[{"type":"text","text":"Hi"}]}}]}"#,
        )
        .unwrap();
        assert_eq!(delta, "Hi");
    }

    #[test]
    fn retries_paid_slug_on_free_upstream_cap() {
        let err = AiError::provider(
            AiFailureKind::RateLimit,
            "google/gemma-4-31b-it:free is temporarily rate-limited upstream",
            Some("google/gemma-4-31b-it:free".into()),
        );
        assert!(should_retry_paid("google/gemma-4-31b-it:free", &err));
        assert!(!should_retry_paid("google/gemma-4-31b-it", &err));
    }
}
