use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn extract_json_object(text: &str) -> Option<Value> {
    let trimmed = text.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if value.is_object() || value.is_array() {
            return Some(value);
        }
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    serde_json::from_str(&trimmed[start..=end]).ok()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchInterpretation {
    pub project_ids: Vec<String>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TodoProposal {
    pub title: String,
    pub description: String,
    pub priority: String,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TodoOrderItem {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrioritizeResult {
    pub order: Vec<TodoOrderItem>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogDraft {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommitSuggestion {
    pub message: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentPrompt {
    pub prompt: String,
    pub known: Vec<String>,
    pub assumptions: Vec<String>,
}

pub fn parse_search(text: &str) -> Option<SearchInterpretation> {
    let value = extract_json_object(text)?;
    serde_json::from_value(value).ok()
}

pub fn parse_prioritize(text: &str) -> Option<PrioritizeResult> {
    serde_json::from_value(extract_json_object(text)?).ok()
}

pub fn parse_todos(text: &str) -> Option<Vec<TodoProposal>> {
    let value = extract_json_object(text)?;
    if let Ok(list) = serde_json::from_value::<Vec<TodoProposal>>(value.clone()) {
        return Some(list);
    }
    value
        .get("todos")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
}

pub fn parse_changelog(text: &str) -> Option<ChangelogDraft> {
    serde_json::from_value(extract_json_object(text)?).ok()
}

pub fn parse_commit(text: &str) -> Option<CommitSuggestion> {
    serde_json::from_value(extract_json_object(text)?).ok()
}

pub fn parse_agent_prompt(text: &str) -> Option<AgentPrompt> {
    serde_json::from_value(extract_json_object(text)?).ok()
}

pub fn search_schema_hint() -> Value {
    json!({
        "type": "object",
        "required": ["projectIds", "explanation"],
        "properties": {
            "projectIds": { "type": "array", "items": { "type": "string" } },
            "explanation": { "type": "string" }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wrapped_json() {
        let search = parse_search("Sure.\n{\"projectIds\":[\"a\"],\"explanation\":\"match\"}\n").unwrap();
        assert_eq!(search.project_ids, vec!["a"]);
        let todos = parse_todos("{\"todos\":[{\"title\":\"T\",\"description\":\"d\",\"priority\":\"high\",\"reasoning\":\"r\"}]}").unwrap();
        assert_eq!(todos[0].title, "T");
        assert!(parse_commit("not json").is_none());
    }
}
