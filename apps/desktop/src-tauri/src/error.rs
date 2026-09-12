use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Msg(String),
    #[error("Git repository not found")]
    GitRepoNotFound,
    #[error("Git is unavailable")]
    GitUnavailable,
    #[error("VS Code not found")]
    VsCodeNotFound,
    #[error("Folder does not exist")]
    FolderMissing,
    #[error("Unable to open file")]
    OpenFile,
    #[error("Cloud synchronization failed")]
    Sync,
    #[error("Invalid exam grade")]
    InvalidGrade,
    #[error("Invalid version")]
    InvalidVersion,
    #[error("Network unavailable")]
    Network,
    #[error("This command requires confirmation")]
    NeedsConfirm,
}

impl AppError {
    pub fn msg(value: impl Into<String>) -> Self {
        Self::Msg(value.into())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Msg(value.to_string())
    }
}

impl From<bluephoenix_domain::error::DomainError> for AppError {
    fn from(value: bluephoenix_domain::error::DomainError) -> Self {
        Self::Msg(value.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Msg(value.to_string())
    }
}

impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
