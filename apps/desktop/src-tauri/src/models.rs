use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDto {
    pub id: String,
    pub slug: String,
    pub kind: String,
    pub name: String,
    pub icon: String,
    pub accent: String,
    pub enabled: bool,
    pub sort_order: i64,
    pub is_system: bool,
    pub capabilities: serde_json::Value,
    pub terminology: serde_json::Value,
    pub nav: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    pub id: String,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCardDto {
    pub id: String,
    pub category_id: String,
    pub kind: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub favorite: bool,
    pub archived: bool,
    pub icon: Option<String>,
    pub accent: Option<String>,
    pub total_tracked_seconds: i64,
    pub xp: i64,
    pub level: i64,
    pub level_ratio: f64,
    pub github_url: Option<String>,
    pub website_url: Option<String>,
    pub current_version: Option<String>,
    pub cfu: Option<f64>,
    pub final_grade: Option<f64>,
    pub average_grade: Option<f64>,
    pub honors: bool,
    pub open_todos: i64,
    pub pinned_command: Option<CommandDto>,
    pub tags: Vec<TagDto>,
    pub achievements: Vec<UnlockDto>,
    pub local_path: Option<String>,
    pub primary_file_path: Option<String>,
    pub attendance: Option<AttendanceDto>,
    pub last_activity: Option<String>,
    pub git: Option<GitStatusDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub command: String,
    pub description: String,
    pub working_directory: Option<String>,
    pub pinned: bool,
    pub favorite: bool,
    pub sort_order: i64,
    pub dangerous: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockDto {
    pub id: String,
    pub achievement_id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub rarity: String,
    pub earned_at: String,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceDto {
    pub lessons: i64,
    pub attended: i64,
    pub missed: i64,
    pub percent: f64,
    pub seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatusDto {
    pub available: bool,
    pub is_repo: bool,
    pub branch: Option<String>,
    pub ahead: i64,
    pub behind: i64,
    pub dirty: bool,
    pub last_commit: Option<String>,
    pub last_commit_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectInput {
    pub category_id: String,
    pub name: String,
    pub description: Option<String>,
    pub local_path: Option<String>,
    pub primary_file_path: Option<String>,
    pub github_url: Option<String>,
    pub website_url: Option<String>,
    pub language_ids: Option<Vec<String>>,
    pub framework_ids: Option<Vec<String>>,
    pub university: Option<String>,
    pub professor: Option<String>,
    pub academic_year: Option<String>,
    pub semester: Option<String>,
    pub cfu: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub url: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub pinned: bool,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoDto {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub category_id: String,
    pub kind_id: Option<String>,
    pub kind_name: Option<String>,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub status: String,
    pub due_date: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntryDto {
    pub id: String,
    pub project_id: String,
    pub topic_id: Option<String>,
    pub topic_title: Option<String>,
    pub device_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub seconds: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningTimerDto {
    pub entry: TimeEntryDto,
    pub project_name: String,
    pub device_id: String,
    pub is_local_device: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsDto {
    pub theme: String,
    pub accent: String,
    pub reduced_motion: bool,
    pub default_workspace: Option<String>,
    pub automatic_updates: bool,
    pub update_channel: String,
    #[serde(default)]
    pub launch_at_startup: bool,
    pub git_path: String,
    pub vscode_path: String,
    pub terminal: String,
    pub shell: String,
    pub include_failed_grades: bool,
    pub onboarding_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapDto {
    pub device_id: String,
    pub settings: AppSettingsDto,
    pub categories: Vec<CategoryDto>,
    pub tags: Vec<TagDto>,
    pub sync: SyncStatusDto,
    pub onboarding_complete: bool,
    pub user: Option<UserDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusDto {
    pub status: String,
    pub label: String,
    pub pending: i64,
    pub last_error: Option<String>,
    pub last_pulled_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntryDto {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFileDto {
    pub id: String,
    pub project_id: String,
    pub display_name: String,
    pub relative_path: Option<String>,
    pub mime: Option<String>,
    pub size_bytes: Option<i64>,
    pub file_kind: Option<String>,
    pub absolute_path: Option<String>,
    #[serde(default)]
    pub index_status: Option<String>,
    #[serde(default)]
    pub index_stage: Option<String>,
    #[serde(default)]
    pub index_limitation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDto {
    pub id: String,
    pub project_id: String,
    pub version: String,
    pub title: Option<String>,
    pub released_at: Option<String>,
    pub changelog: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub sort_order: i64,
    pub status: String,
    pub estimated_seconds: Option<i64>,
    pub tracked_seconds: i64,
    pub completion_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonDto {
    pub id: String,
    pub project_id: String,
    pub topic_id: Option<String>,
    pub date: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub duration_seconds: i64,
    pub attended: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamDto {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub date: Option<String>,
    pub grade: Option<f64>,
    pub status: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDto {
    pub id: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub category_id: Option<String>,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpaDashboardDto {
    pub simple_average: Option<f64>,
    pub weighted_average: Option<f64>,
    pub used_weighted: bool,
    pub included: Vec<String>,
    pub excluded_missing_final: Vec<String>,
    pub excluded_failed: Vec<String>,
    pub total_cfu: f64,
    pub completed_cfu: f64,
    pub passed_courses: i64,
    pub failed_attempts: i64,
    pub total_attempts: i64,
    pub study_seconds: i64,
    pub attendance_percent: f64,
    pub max_grade: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckDto {
    pub local: String,
    pub remote: Option<String>,
    pub newer: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutputDto {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHitDto {
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub subtitle: String,
    pub category_id: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalProgressDto {
    pub xp: i64,
    pub level: i64,
    pub ratio: f64,
    pub tracked_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStatusDto {
    pub enabled: bool,
    pub has_key: bool,
    pub masked_key: Option<String>,
    pub models: Vec<String>,
    pub commit_follow_style: bool,
    pub setup_dismissed: bool,
    pub signed_in: bool,
    pub cloud_secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevealedKeyDto {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConversationDto {
    pub id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiMessageDto {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub fallback_used: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRecordDto {
    pub id: String,
    pub file_id: String,
    pub display_name: String,
    pub status: String,
    pub stage: String,
    pub limitation: Option<String>,
    pub parser: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentChunkDto {
    pub id: String,
    pub file_id: String,
    pub document_name: String,
    pub heading: Option<String>,
    pub page: Option<i64>,
    pub section: Option<String>,
    pub text: String,
}
