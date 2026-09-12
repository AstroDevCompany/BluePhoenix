use serde::{Deserialize, Serialize};

use crate::capabilities::CapabilitySet;
use crate::ids::{CategoryKind, ProjectStatus};

/// Capability-filtered project snapshot for command palette, detail pages,
/// and future AI adapters. UI must not be scraped for this data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectContext {
    pub project_id: String,
    pub category_id: String,
    pub kind: CategoryKind,
    pub capabilities: CapabilitySet,
    pub name: String,
    pub description: String,
    pub status: ProjectStatus,
    pub tracked_seconds: i64,
    pub xp: i64,
    pub open_todos: i64,
    pub github_url: Option<String>,
    pub website_url: Option<String>,
    pub current_version: Option<String>,
    pub final_grade: Option<f64>,
    pub cfu: Option<f64>,
}

/// Capability-filtered slices for AI. Never a full database dump.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AiProjectBundle {
    pub snapshot: Option<ProjectContext>,
    pub todos: Vec<AiTodoSlice>,
    pub activity: Vec<AiActivitySlice>,
    pub links: Vec<AiLinkSlice>,
    pub versions: Vec<AiVersionSlice>,
    pub topics: Vec<AiTopicSlice>,
    pub exams: Vec<AiExamSlice>,
    pub tags: Vec<String>,
    pub time_total_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiTodoSlice {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiActivitySlice {
    pub event_type: String,
    pub created_at: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiLinkSlice {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiVersionSlice {
    pub version: String,
    pub title: Option<String>,
    pub changelog: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiTopicSlice {
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiExamSlice {
    pub date: Option<String>,
    pub status: String,
    pub grade: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchHit {
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub subtitle: String,
    pub category_id: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TodoFilter {
    pub project_id: Option<String>,
    pub category_id: Option<String>,
    pub status: Option<String>,
    pub kind_id: Option<String>,
    pub query: Option<String>,
}

pub fn todo_matches(
    title: &str,
    description: &str,
    status: &str,
    kind_id: &str,
    project_id: &str,
    category_id: &str,
    filter: &TodoFilter,
) -> bool {
    if let Some(ref id) = filter.project_id {
        if project_id != id {
            return false;
        }
    }
    if let Some(ref id) = filter.category_id {
        if category_id != id {
            return false;
        }
    }
    if let Some(ref s) = filter.status {
        if status != s {
            return false;
        }
    }
    if let Some(ref k) = filter.kind_id {
        if kind_id != k {
            return false;
        }
    }
    if let Some(ref q) = filter.query {
        let q = q.to_lowercase();
        if q.is_empty() {
            return true;
        }
        return title.to_lowercase().contains(&q) || description.to_lowercase().contains(&q);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_todos_by_status_and_query() {
        let filter = TodoFilter {
            project_id: None,
            category_id: None,
            status: Some("open".into()),
            kind_id: None,
            query: Some("cloud".into()),
        };
        assert!(todo_matches(
            "Cloud sync",
            "",
            "open",
            "features",
            "p1",
            "c1",
            &filter
        ));
        assert!(!todo_matches(
            "Cloud sync",
            "",
            "completed",
            "features",
            "p1",
            "c1",
            &filter
        ));
    }
}
