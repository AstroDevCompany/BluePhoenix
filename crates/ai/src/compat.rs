//! Per-family OpenRouter request and response shapes.
//!
//! OpenRouter is OpenAI-compatible at the wire, but providers still reject or
//! reshape fields. Gemma via Google AI Studio is the usual failure: JSON mode
//! and (on Gemma 3) system roles come back as "Provider returned error".

use serde_json::{json, Value};

use crate::provider::{ChatMessage, CompletionRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFamily {
    Gemma,
    Gemini,
    Qwen,
    Claude,
    Gpt,
    DeepSeek,
    Other,
}

pub fn normalize_id(model: &str) -> String {
    model.trim().to_ascii_lowercase()
}

pub fn family(model: &str) -> ModelFamily {
    let m = normalize_id(model);
    if m.contains("gemma") {
        ModelFamily::Gemma
    } else if m.contains("gemini") {
        ModelFamily::Gemini
    } else if m.contains("qwen") || m.contains("qwq") {
        ModelFamily::Qwen
    } else if m.contains("claude") || m.contains("anthropic/") {
        ModelFamily::Claude
    } else if m.contains("deepseek") {
        ModelFamily::DeepSeek
    } else if m.contains("gpt") || m.contains("openai/") || is_o_series(&m) {
        ModelFamily::Gpt
    } else {
        ModelFamily::Other
    }
}

fn is_o_series(m: &str) -> bool {
    ["o1", "o3", "o4"].iter().any(|tag| {
        m.contains(&format!("/{tag}"))
            || m.contains(&format!("{tag}-"))
            || m.contains(&format!("{tag}:"))
            || m.ends_with(tag)
    })
}

pub fn uses_max_completion_tokens(model: &str) -> bool {
    let m = normalize_id(model);
    family(&m) == ModelFamily::Gpt && (m.contains("gpt-5") || is_o_series(&m))
}

pub fn uses_developer_role(model: &str) -> bool {
    uses_max_completion_tokens(model)
}

pub fn folds_system_into_user(model: &str) -> bool {
    family(model) == ModelFamily::Gemma
}

pub fn allows_temperature(model: &str) -> bool {
    if uses_max_completion_tokens(model) {
        return false;
    }
    let m = normalize_id(model);
    !(m.contains("deepseek-r1") || m.contains("deepseek-reasoner"))
}

/// `response_format: json_object` is widely rejected (Gemma, many Qwen/DeepSeek
/// endpoints, o-series). Prompts already ask for JSON; omit it unless the
/// family is known to accept it.
pub fn supports_json_object(model: &str) -> bool {
    match family(model) {
        ModelFamily::Claude => true,
        ModelFamily::Gpt => !uses_max_completion_tokens(model),
        ModelFamily::Gemma
        | ModelFamily::Gemini
        | ModelFamily::Qwen
        | ModelFamily::DeepSeek
        | ModelFamily::Other => false,
    }
}

pub fn normalize_messages(model: &str, messages: &[ChatMessage]) -> Vec<ChatMessage> {
    let mut out: Vec<ChatMessage> = if folds_system_into_user(model) {
        fold_system(messages)
    } else {
        messages.to_vec()
    };
    if uses_developer_role(model) {
        for message in &mut out {
            if message.role == "system" {
                message.role = "developer".into();
            }
        }
    }
    out
}

fn fold_system(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    let mut system = String::new();
    let mut rest = Vec::new();
    for message in messages {
        if message.role == "system" {
            if !system.is_empty() {
                system.push_str("\n\n");
            }
            system.push_str(&message.content);
        } else {
            rest.push(message.clone());
        }
    }
    if system.is_empty() {
        return rest;
    }
    if let Some(first_user) = rest.iter_mut().find(|m| m.role == "user") {
        first_user.content = format!("{system}\n\n{}", first_user.content);
    } else {
        rest.insert(
            0,
            ChatMessage {
                role: "user".into(),
                content: system,
            },
        );
    }
    rest
}

pub fn chat_completions_body(request: &CompletionRequest) -> Value {
    let messages = normalize_messages(&request.model, &request.messages);
    let mut body = json!({
        "model": request.model,
        "messages": messages.iter().map(|m| json!({
            "role": m.role,
            "content": m.content,
        })).collect::<Vec<_>>(),
    });
    if uses_max_completion_tokens(&request.model) {
        body["max_completion_tokens"] = json!(request.max_tokens);
    } else {
        body["max_tokens"] = json!(request.max_tokens);
        if allows_temperature(&request.model) {
            body["temperature"] = json!(request.temperature);
        }
    }
    if request.json_mode && supports_json_object(&request.model) {
        body["response_format"] = json!({ "type": "json_object" });
    }
    body
}

#[derive(Debug, Default)]
pub struct SplitText {
    pub visible: String,
    pub hidden: String,
}

/// Visible answer from a chat `message` or stream `delta` object.
pub fn message_text(node: &Value) -> String {
    let split = split_message_text(node);
    let visible = strip_thinking_wrappers(&split.visible);
    if !visible.trim().is_empty() {
        visible
    } else {
        strip_thinking_wrappers(&split.hidden)
    }
}

pub fn split_message_text(node: &Value) -> SplitText {
    let mut visible = String::new();
    let mut hidden = String::new();
    if let Some(content) = node.get("content") {
        collect_content(content, &mut visible, &mut hidden);
    }
    if let Some(text) = scalar_text(node.get("output_text")) {
        push_join(&mut visible, &text);
    }
    if let Some(text) = scalar_text(node.get("text")) {
        push_join(&mut visible, &text);
    }
    collect_reasoning(node, &mut hidden);
    SplitText { visible, hidden }
}

pub fn choice_text(choice: &Value) -> String {
    if let Some(message) = choice.get("message") {
        let text = message_text(message);
        if !text.trim().is_empty() {
            return text;
        }
    }
    if let Some(delta) = choice.get("delta") {
        let text = message_text(delta);
        if !text.trim().is_empty() {
            return text;
        }
    }
    if let Some(text) = scalar_text(choice.get("text")) {
        return strip_thinking_wrappers(&text);
    }
    String::new()
}

pub fn completion_text(value: &Value) -> String {
    if let Some(choice) = value.pointer("/choices/0") {
        let text = choice_text(choice);
        if !text.trim().is_empty() {
            return text;
        }
    }
    if let Some(text) = scalar_text(value.get("output_text")) {
        return strip_thinking_wrappers(&text);
    }
    // Native Gemini generateContent, if it ever leaks through.
    if let Some(parts) = value
        .pointer("/candidates/0/content/parts")
        .and_then(|v| v.as_array())
    {
        let mut visible = String::new();
        let mut hidden = String::new();
        for part in parts {
            collect_part(part, &mut visible, &mut hidden);
        }
        let visible = strip_thinking_wrappers(&visible);
        if !visible.trim().is_empty() {
            return visible;
        }
        return strip_thinking_wrappers(&hidden);
    }
    String::new()
}

fn collect_content(value: &Value, visible: &mut String, hidden: &mut String) {
    match value {
        Value::String(s) => push_join(visible, s),
        Value::Array(parts) => {
            for part in parts {
                collect_part(part, visible, hidden);
            }
        }
        Value::Object(_) => collect_part(value, visible, hidden),
        _ => {}
    }
}

fn collect_part(part: &Value, visible: &mut String, hidden: &mut String) {
    if let Some(s) = part.as_str() {
        push_join(visible, s);
        return;
    }
    let ty = part.get("type").and_then(|v| v.as_str()).unwrap_or("text");
    let text = scalar_text(part.get("text"))
        .or_else(|| scalar_text(part.get("content")))
        .or_else(|| scalar_text(part.get("output_text")))
        .or_else(|| scalar_text(part.get("thinking")))
        .unwrap_or_default();
    if text.is_empty() {
        return;
    }
    if matches!(ty, "thinking" | "reasoning" | "thought" | "reasoning_text") {
        push_join(hidden, &text);
    } else {
        push_join(visible, &text);
    }
}

fn collect_reasoning(node: &Value, hidden: &mut String) {
    for key in ["reasoning_content", "reasoning"] {
        if let Some(value) = node.get(key) {
            let text = match value {
                Value::String(s) => s.clone(),
                Value::Array(_) | Value::Object(_) => {
                    let mut visible = String::new();
                    let mut nested = String::new();
                    collect_content(value, &mut visible, &mut nested);
                    if !visible.is_empty() {
                        visible
                    } else {
                        nested
                    }
                }
                _ => String::new(),
            };
            push_join(hidden, &text);
        }
    }
    if let Some(details) = node.get("reasoning_details").and_then(|v| v.as_array()) {
        for detail in details {
            let text = scalar_text(detail.get("text"))
                .or_else(|| scalar_text(detail.get("summary")))
                .or_else(|| scalar_text(detail.get("content")))
                .or_else(|| detail.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            push_join(hidden, &text);
        }
    }
}

fn scalar_text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

fn push_join(buf: &mut String, piece: &str) {
    if piece.is_empty() {
        return;
    }
    buf.push_str(piece);
}

pub fn strip_thinking_wrappers(text: &str) -> String {
    tidy_visible(&strip_thinking_blocks(text))
}

fn strip_thinking_blocks(text: &str) -> String {
    let mut out = text.to_string();
    loop {
        let next = strip_channel_thought(&strip_xml_blocks(
            &strip_xml_blocks(&out, "think"),
            "thinking",
        ));
        if next == out {
            return next;
        }
        out = next;
    }
}

fn strip_xml_blocks(input: &str, tag: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some((start, open_end)) = find_open_tag(rest, tag) {
        out.push_str(&rest[..start]);
        let close = format!("</{tag}>");
        if let Some(rel) = find_ci(&rest[open_end..], &close) {
            rest = &rest[open_end + rel + close.len()..];
        } else {
            return out;
        }
    }
    out.push_str(rest);
    out
}

fn find_open_tag(hay: &str, tag: &str) -> Option<(usize, usize)> {
    let needle = format!("<{tag}");
    let lower_hay = hay.to_ascii_lowercase();
    let lower_needle = needle.to_ascii_lowercase();
    let mut from = 0;
    while from < lower_hay.len() {
        let Some(rel) = lower_hay[from..].find(&lower_needle) else {
            return None;
        };
        let start = from + rel;
        let after = start + needle.len();
        let next = hay[after..].chars().next();
        if next.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
            from = after;
            continue;
        }
        return match hay[after..].find('>') {
            Some(gt) => Some((start, after + gt + 1)),
            None => Some((start, hay.len())),
        };
    }
    None
}

fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    hay.to_ascii_lowercase().find(&needle.to_ascii_lowercase())
}

fn strip_channel_thought(text: &str) -> String {
    const START: &str = "<|channel>thought";
    let Some(start) = find_ci(text, START) else {
        return text.to_string();
    };
    let after = start + START.len();
    let close = find_ci(&text[after..], "<channel|>")
        .map(|i| (after + i, "<channel|>".len()))
        .or_else(|| {
            find_ci(&text[after..], "<|channel|>").map(|i| (after + i, "<|channel|>".len()))
        });
    match close {
        Some((at, len)) => format!("{}{}", &text[..start], &text[at + len..]),
        None => text[..start].to_string(),
    }
}

fn tidy_visible(text: &str) -> String {
    let mut s = text.to_string();
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }
    s.trim().to_string()
}

/// Incremental filter so streamed tokens inside `<think>` never reach the UI.
#[derive(Debug, Default)]
pub struct ThinkStream {
    raw: String,
    emitted: usize,
}

impl ThinkStream {
    pub fn push(&mut self, piece: &str) -> String {
        if piece.is_empty() {
            return String::new();
        }
        self.raw.push_str(piece);
        let visible = strip_thinking_blocks(&self.raw);
        self.emit_from(hold_incomplete_wrappers(&visible))
    }

    /// Emit any suffix held back as a possible think-tag prefix.
    pub fn flush(&mut self) -> String {
        let visible = strip_thinking_blocks(&self.raw);
        self.emit_from(&visible)
    }

    pub fn finish(&self) -> String {
        strip_thinking_wrappers(&self.raw)
    }

    fn emit_from(&mut self, visible: &str) -> String {
        if visible.len() <= self.emitted {
            return String::new();
        }
        if !visible.is_char_boundary(self.emitted) {
            self.emitted = visible.len();
            return String::new();
        }
        let delta = visible[self.emitted..].to_string();
        self.emitted = visible.len();
        delta
    }
}

fn hold_incomplete_wrappers(s: &str) -> &str {
    match trailing_incomplete_wrapper(s) {
        Some(i) => &s[..i],
        None => s,
    }
}

fn trailing_incomplete_wrapper(s: &str) -> Option<usize> {
    let i = s.rfind('<')?;
    let lower = s.to_ascii_lowercase();
    if !lower.is_char_boundary(i) {
        return None;
    }
    let tail = &lower[i..];
    if is_incomplete_wrapper_prefix(tail) {
        Some(i)
    } else {
        None
    }
}

fn is_incomplete_wrapper_prefix(tail: &str) -> bool {
    const CHANNEL: &str = "<|channel>thought";
    if CHANNEL.starts_with(tail) && tail != CHANNEL {
        return true;
    }
    let Some(inner) = tail.strip_prefix('<') else {
        return false;
    };
    let inner = inner.strip_prefix('/').unwrap_or(inner);
    if inner.contains('>') {
        return false;
    }
    if inner.is_empty() {
        return true;
    }
    for tag in ["thinking", "think"] {
        if tag.starts_with(inner) {
            return true;
        }
        if inner.starts_with(tag) {
            let rest = &inner[tag.len()..];
            if rest.is_empty() {
                return true;
            }
            let c = rest.chars().next().unwrap();
            return !c.is_ascii_alphanumeric() && c != '_';
        }
    }
    false
}

pub fn provider_error_text(body: &str) -> String {
    let lower = body.to_lowercase();
    if lower.contains("sk-or-") || lower.contains("authorization") {
        return "OpenRouter rejected the request".into();
    }
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        if let Some(raw) = value
            .pointer("/error/metadata/raw")
            .and_then(|v| v.as_str())
        {
            if let Ok(inner) = serde_json::from_str::<Value>(raw) {
                if let Some(msg) = json_error_message(&inner) {
                    return msg;
                }
            }
            let raw = raw.trim();
            if !raw.is_empty() && raw.len() <= 400 && !raw.to_lowercase().contains("sk-or-") {
                return raw.to_string();
            }
        }
        if let Some(msg) = json_error_message(&value) {
            return msg;
        }
    }
    let trimmed = body.trim();
    if trimmed.len() > 180 {
        format!("{}…", &trimmed[..180])
    } else if trimmed.is_empty() {
        "OpenRouter request failed".into()
    } else {
        trimmed.to_string()
    }
}

fn json_error_message(value: &Value) -> Option<String> {
    value
        .pointer("/error/message")
        .and_then(|m| m.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn looks_like_rate_limit(body: &str) -> bool {
    let lower = format!("{} {}", body, provider_error_text(body)).to_lowercase();
    lower.contains("rate-limited") || lower.contains("rate limited")
}

/// OpenRouter `:free` slugs use a shared upstream pool and ignore paid quota.
pub fn paid_variant(model: &str) -> Option<String> {
    let trimmed = model.trim();
    let (base, variant) = trimmed.rsplit_once(':')?;
    if variant.eq_ignore_ascii_case("free") && !base.is_empty() {
        Some(base.to_string())
    } else {
        None
    }
}

pub fn looks_like_provider_param_error(body: &str) -> bool {
    let lower = format!("{} {}", body, provider_error_text(body)).to_lowercase();
    lower.contains("provider returned error")
        || lower.contains("json mode")
        || lower.contains("response_format")
        || lower.contains("developer instruction")
        || lower.contains("not enabled for models/")
        || lower.contains("unsupported parameter")
        || lower.contains("unknown parameter")
        || (lower.contains("max_tokens") && lower.contains("unsupported"))
        || (lower.contains("temperature")
            && (lower.contains("unsupported") || lower.contains("does not support")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(model: &str, json_mode: bool) -> CompletionRequest {
        CompletionRequest {
            model: model.into(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: "Return JSON.".into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: "hi".into(),
                },
            ],
            temperature: 0.1,
            max_tokens: 2200,
            json_mode,
        }
    }

    #[test]
    fn classifies_families() {
        assert_eq!(family("google/gemma-4-31b-it:free"), ModelFamily::Gemma);
        assert_eq!(family("google/gemini-2.5-flash"), ModelFamily::Gemini);
        assert_eq!(family("qwen/qwen3-32b"), ModelFamily::Qwen);
        assert_eq!(family("qwen/qwen3.5-plus"), ModelFamily::Qwen);
        assert_eq!(family("anthropic/claude-sonnet-4.5"), ModelFamily::Claude);
        assert_eq!(family("openai/gpt-4o"), ModelFamily::Gpt);
        assert_eq!(family("openai/gpt-5"), ModelFamily::Gpt);
        assert_eq!(family("openai/o3-mini"), ModelFamily::Gpt);
        assert_eq!(family("deepseek/deepseek-r1"), ModelFamily::DeepSeek);
        assert_eq!(family("deepseek/deepseek-chat"), ModelFamily::DeepSeek);
        assert_eq!(
            paid_variant("google/gemma-4-31b-it:free").as_deref(),
            Some("google/gemma-4-31b-it")
        );
        assert!(paid_variant("openai/gpt-4o").is_none());
        assert!(paid_variant("openai/gpt-4o:nitro").is_none());
    }

    #[test]
    fn gemma_omits_json_mode_and_folds_system() {
        let body = chat_completions_body(&req("google/gemma-4-31b-it:free", true));
        assert!(body.get("response_format").is_none());
        assert_eq!(body["max_tokens"], 2200);
        assert!((body["temperature"].as_f64().unwrap() - 0.1).abs() < 1e-6);
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
        assert!(messages[0]["content"]
            .as_str()
            .unwrap()
            .starts_with("Return JSON."));
    }

    #[test]
    fn qwen_and_deepseek_and_gemini_omit_json_object() {
        for model in [
            "qwen/qwen3-32b",
            "qwen/qwen3.5-plus",
            "deepseek/deepseek-chat",
            "google/gemini-2.5-flash",
        ] {
            let body = chat_completions_body(&req(model, true));
            assert!(body.get("response_format").is_none(), "{model}");
            assert_eq!(
                body["messages"].as_array().unwrap()[0]["role"],
                "system",
                "{model}"
            );
        }
    }

    #[test]
    fn gpt4_and_claude_keep_json_object() {
        for model in ["openai/gpt-4o", "anthropic/claude-sonnet-4.5"] {
            let body = chat_completions_body(&req(model, true));
            assert_eq!(body["response_format"]["type"], "json_object", "{model}");
            assert_eq!(body["max_tokens"], 2200, "{model}");
        }
    }

    #[test]
    fn gpt5_uses_completion_tokens_and_developer_role() {
        let body = chat_completions_body(&req("openai/gpt-5", true));
        assert_eq!(body["max_completion_tokens"], 2200);
        assert!(body.get("max_tokens").is_none());
        assert!(body.get("temperature").is_none());
        assert!(body.get("response_format").is_none());
        assert_eq!(body["messages"][0]["role"], "developer");
    }

    #[test]
    fn deepseek_reasoner_omits_temperature() {
        let body = chat_completions_body(&req("deepseek/deepseek-r1", false));
        assert!(body.get("temperature").is_none());
        assert_eq!(body["max_tokens"], 2200);
    }

    #[test]
    fn extracts_openai_string_and_array_content() {
        let string = json!({"content": "ok"});
        assert_eq!(message_text(&string), "ok");
        let parts = json!({"content": [
            {"type": "output_text", "text": "hel"},
            {"type": "text", "text": "lo"}
        ]});
        assert_eq!(message_text(&parts), "hello");
    }

    #[test]
    fn prefers_content_over_reasoning() {
        let qwen = json!({
            "content": "{\"ok\":true}",
            "reasoning_content": "thinking"
        });
        assert_eq!(message_text(&qwen), "{\"ok\":true}");
        let empty_content = json!({
            "content": "",
            "reasoning": "the answer is 4"
        });
        assert_eq!(message_text(&empty_content), "the answer is 4");
        let claude = json!({"content": [
            {"type": "thinking", "thinking": "hmm"},
            {"type": "text", "text": "done"}
        ]});
        assert_eq!(message_text(&claude), "done");
    }

    #[test]
    fn extracts_gemini_candidates_and_gemma_channels() {
        let gemini = json!({
            "candidates": [{"content": {"parts": [{"text": "hi"}]}}]
        });
        assert_eq!(completion_text(&gemini), "hi");
        let tagged = "<|channel>thought\nplan\n<channel|>{\"a\":1}";
        assert_eq!(strip_thinking_wrappers(tagged), "{\"a\":1}");
        assert_eq!(
            strip_thinking_wrappers("<think>plan</think>\n{\"a\":1}"),
            "{\"a\":1}"
        );
        assert_eq!(
            strip_thinking_wrappers("<think>one</think>hi<think>two</think>there"),
            "hithere"
        );
        assert_eq!(
            strip_thinking_wrappers("<THINK>secret</think>\nVisible"),
            "Visible"
        );
        assert_eq!(
            strip_thinking_wrappers("<thinking>nope</thinking>yes"),
            "yes"
        );
        assert_eq!(strip_thinking_wrappers("<think>unclosed"), "");
        assert_eq!(
            strip_thinking_wrappers("keep\n<think>hide"),
            "keep"
        );
        assert_eq!(
            message_text(&json!({"content": "<think>secret</think>"})),
            ""
        );
        let mut stream = ThinkStream::default();
        assert_eq!(stream.push("<th"), "");
        assert_eq!(stream.push("ink>hid"), "");
        assert_eq!(stream.push("den</think>Hey"), "Hey");
        assert_eq!(stream.finish(), "Hey");
        let mut split_open = ThinkStream::default();
        assert_eq!(split_open.push("Hi <"), "Hi ");
        assert_eq!(split_open.push("th"), "");
        assert_eq!(split_open.push("ink>no</think>Go"), "Go");
        assert_eq!(split_open.finish(), "Hi Go");
        let mut lt = ThinkStream::default();
        assert_eq!(lt.push("1 <"), "1 ");
        assert_eq!(lt.push(" 2"), "< 2");
        assert_eq!(lt.finish(), "1 < 2");
    }

    #[test]
    fn unwraps_nested_google_provider_error() {
        let body = r#"{
            "error": {
                "message": "Provider returned error",
                "code": 400,
                "metadata": {
                    "raw": "{\n \"error\": {\n \"message\": \"JSON mode is not enabled for models/gemma-4-31b-it\"\n }\n}\n",
                    "provider_name": "Google AI Studio"
                }
            }
        }"#;
        assert_eq!(
            provider_error_text(body),
            "JSON mode is not enabled for models/gemma-4-31b-it"
        );
        assert!(looks_like_provider_param_error(body));
    }
}
