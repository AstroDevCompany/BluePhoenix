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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CommandProposal {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub reasoning: String,
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
        name: json_string(obj, &["name", "title"]),
        description: json_string(obj, &["description", "summary", "about"]),
        github_url: json_string(obj, &["githubUrl", "github_url"]),
        website_url: json_string(obj, &["websiteUrl", "website_url"]),
        languages: json_string_list(obj, &["languages"]),
        frameworks: json_string_list(obj, &["frameworks"]),
    })
}

pub fn parse_command_scan(text: &str) -> Option<Vec<CommandProposal>> {
    let value = extract_json_object(text)?;
    let raw = if let Ok(list) = serde_json::from_value::<Vec<CommandProposal>>(value.clone()) {
        list
    } else {
        value
            .get("commands")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())?
    };
    Some(sanitize_command_proposals(raw))
}

fn sanitize_command_proposals(list: Vec<CommandProposal>) -> Vec<CommandProposal> {
    list.into_iter()
        .filter_map(|mut item| {
            item.name = item.name.trim().to_string();
            item.command = item.command.split_whitespace().collect::<Vec<_>>().join(" ");
            item.description = item.description.trim().to_string();
            item.reasoning = item.reasoning.trim().to_string();
            if let Some(dir) = item.working_directory.take() {
                let cleaned = dir.trim().replace('\\', "/");
                if !cleaned.is_empty()
                    && cleaned != "."
                    && !cleaned.contains("..")
                    && !cleaned.starts_with('/')
                {
                    item.working_directory = Some(cleaned);
                }
            }
            if item.name.is_empty() || item.command.is_empty() || has_disallowed_shell(&item.command)
            {
                return None;
            }
            if item.source.trim().eq_ignore_ascii_case("found") {
                item.source = "found".into();
            } else {
                item.source = "hypothesized".into();
            }
            Some(item)
        })
        .collect()
}

fn has_disallowed_shell(command: &str) -> bool {
    let compact = command.trim();
    if compact.contains("&&")
        || compact.contains("||")
        || compact.contains(';')
        || compact.contains('|')
        || compact.contains('>')
        || compact.contains('<')
        || compact.contains('`')
        || compact.contains('$')
    {
        return true;
    }
    compact
        .split_whitespace()
        .next()
        .is_some_and(|token| token.contains('='))
}

pub fn requested_inspect_files(text: &str) -> Vec<String> {
    let Some(value) = extract_json_object(text) else {
        return Vec::new();
    };
    let Some(obj) = value.as_object() else {
        return Vec::new();
    };
    json_string_list(obj, &["needFiles", "need_files", "readFiles"])
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
                .filter_map(|v| {
                    v.as_str()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(ToOwned::to_owned)
                })
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
        let search =
            parse_search("Sure.\n{\"projectIds\":[\"a\"],\"explanation\":\"match\"}\n").unwrap();
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
        let titled = parse_software_folder_draft(
            "{\"title\":\"BluePhoenix\",\"summary\":\"A local command center.\",\"needFiles\":[\"src/lib.rs\"]}",
        )
        .unwrap();
        assert_eq!(titled.name, "BluePhoenix");
        assert_eq!(titled.description, "A local command center.");
        assert_eq!(
            requested_inspect_files("{\"needFiles\":[\"src/lib.rs\",\"../etc/passwd\"]}"),
            vec!["src/lib.rs", "../etc/passwd"]
        );
        let commands = parse_command_scan(
            "Sure.\n{\"commands\":[{\"name\":\"Dev\",\"command\":\"pnpm run dev\",\"source\":\"found\",\"reasoning\":\"package.json scripts.dev\"},{\"name\":\"Bad\",\"command\":\"rm -rf / && echo\"}]}\n",
        )
        .unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command, "pnpm run dev");
        assert_eq!(commands[0].source, "found");
        assert!(parse_command_scan("{\"commands\":[{\"name\":\"X\",\"command\":\"FOO=1 pnpm test\"}]}")
            .unwrap()
            .is_empty());
    }
}
