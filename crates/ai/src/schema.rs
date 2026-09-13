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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareFolderDraft {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub github_url: String,
    #[serde(default)]
    pub website_url: String,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub frameworks: Vec<String>,
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

pub fn parse_software_folder_draft(text: &str) -> Option<SoftwareFolderDraft> {
    let value = extract_json_object(text)?;
    let obj = value.as_object()?;
    Some(SoftwareFolderDraft {
        name: json_string(obj, &["name"]),
        description: json_string(obj, &["description"]),
        github_url: json_string(obj, &["githubUrl", "github_url"]),
        website_url: json_string(obj, &["websiteUrl", "website_url"]),
        languages: json_string_list(obj, &["languages"]),
        frameworks: json_string_list(obj, &["frameworks"]),
    })
}

fn json_string(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> String {
    for key in keys {
        if let Some(value) = obj.get(*key) {
            if let Some(s) = value.as_str() {
                return s.trim().to_string();
            }
        }
    }
    String::new()
}

fn json_string_list(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Vec<String> {
    for key in keys {
        if let Some(Value::Array(items)) = obj.get(*key) {
            return items
                .iter()
                .filter_map(|v| v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(ToOwned::to_owned))
                .collect();
        }
    }
    Vec::new()
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
        let draft = parse_software_folder_draft(
            "Sure.\n{\"name\":\"App\",\"description\":null,\"githubUrl\":\"https://github.com/org/app\",\"websiteUrl\":\"\",\"languages\":[\"TypeScript\"],\"frameworks\":[]}\n",
        )
        .unwrap();
        assert_eq!(draft.name, "App");
        assert!(draft.description.is_empty());
        assert_eq!(draft.github_url, "https://github.com/org/app");
        assert_eq!(draft.languages, vec!["TypeScript"]);
    }
}
