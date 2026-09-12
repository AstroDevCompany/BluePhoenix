use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

pub fn parse_id(value: &str) -> Result<Uuid, uuid::Error> {
    Uuid::parse_str(value)
}

pub const SOFTWARE_CATEGORY_ID: &str = "00000000-0000-7000-8000-000000000001";
pub const UNIVERSITY_CATEGORY_ID: &str = "00000000-0000-7000-8000-000000000002";
pub const FITNESS_CATEGORY_ID: &str = "00000000-0000-7000-8000-000000000003";
pub const PHOTOGRAPHY_CATEGORY_ID: &str = "00000000-0000-7000-8000-000000000004";
pub const PERSONAL_CATEGORY_ID: &str = "00000000-0000-7000-8000-000000000005";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryKind {
    Software,
    University,
    Generic,
}

impl CategoryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Software => "software",
            Self::University => "university",
            Self::Generic => "generic",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "software" => Some(Self::Software),
            "university" => Some(Self::University),
            "generic" => Some(Self::Generic),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Active,
    Paused,
    Completed,
    Archived,
}

impl ProjectStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "paused" => Some(Self::Paused),
            "completed" => Some(Self::Completed),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Open,
    InProgress,
    Completed,
    Cancelled,
}

impl TodoStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl TodoPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopicStatus {
    NotStarted,
    InProgress,
    Completed,
}

impl TopicStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExamStatus {
    Scheduled,
    Attempted,
    Passed,
    Failed,
    Withdrawn,
    NoShow,
}

impl ExamStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Attempted => "attempted",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Withdrawn => "withdrawn",
            Self::NoShow => "no_show",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "scheduled" => Some(Self::Scheduled),
            "attempted" => Some(Self::Attempted),
            "passed" => Some(Self::Passed),
            "failed" => Some(Self::Failed),
            "withdrawn" => Some(Self::Withdrawn),
            "no_show" => Some(Self::NoShow),
            _ => None,
        }
    }

    pub fn counts_as_failed(self) -> bool {
        matches!(self, Self::Failed)
    }

    pub fn has_grade(self) -> bool {
        matches!(self, Self::Attempted | Self::Passed | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagKind {
    Language,
    Framework,
    Custom,
}

impl TagKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Language => "language",
            Self::Framework => "framework",
            Self::Custom => "custom",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "language" => Some(Self::Language),
            "framework" => Some(Self::Framework),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Synced,
    Syncing,
    Offline,
    ChangesPending,
    SyncError,
}

impl SyncStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Synced => "synced",
            Self::Syncing => "syncing",
            Self::Offline => "offline",
            Self::ChangesPending => "changes_pending",
            Self::SyncError => "sync_error",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Synced => "Synced",
            Self::Syncing => "Syncing…",
            Self::Offline => "Offline",
            Self::ChangesPending => "Changes pending",
            Self::SyncError => "Sync error",
        }
    }
}
