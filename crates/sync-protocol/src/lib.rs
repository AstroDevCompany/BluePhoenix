use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeOp {
    Upsert,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetadataChange {
    pub table: String,
    pub row_id: String,
    pub op: ChangeOp,
    pub payload: serde_json::Value,
    pub revision: i64,
    pub updated_at: DateTime<Utc>,
    pub base_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PushBatch {
    pub device_id: String,
    pub changes: Vec<MetadataChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PullResponse {
    pub cursor: String,
    pub changes: Vec<MetadataChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictReason {
    StaleRevision,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConflictRecord {
    pub table: String,
    pub row_id: String,
    pub reason: ConflictReason,
    pub server_row: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PushResult {
    pub accepted: Vec<String>,
    pub conflicts: Vec<ConflictRecord>,
}

/// Opaque encrypted secret. The server stores ciphertext only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EncryptedSecretEnvelope {
    pub id: String,
    pub kind: String,
    pub ciphertext: String,
    pub nonce: String,
    pub wrap_params: serde_json::Value,
    pub revision: i64,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecretPushBatch {
    pub device_id: String,
    pub secrets: Vec<EncryptedSecretEnvelope>,
    pub base_revisions: Vec<(String, i64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvalidateKind {
    Metadata,
    Secrets,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvalidateMessage {
    pub kind: InvalidateKind,
    pub user_id: String,
}

pub const OPENROUTER_SECRET_KIND: &str = "openrouter_api_key";
pub const METADATA_TABLES: &[&str] = &[
    "categories",
    "projects",
    "software_profiles",
    "university_profiles",
    "tags",
    "project_tags",
    "time_entries",
    "todos",
    "todo_kinds",
    "commands",
    "project_links",
    "project_files",
    "project_versions",
    "changelog_entries",
    "course_topics",
    "course_lessons",
    "exam_attempts",
    "activity_events",
    "achievement_unlocks",
];
