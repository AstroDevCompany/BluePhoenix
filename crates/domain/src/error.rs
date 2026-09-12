use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("{0}")]
    Validation(String),
    #[error("invalid semantic version: {0}")]
    InvalidSemver(String),
    #[error("malformed json: {0}")]
    MalformedJson(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("capability not enabled: {0}")]
    Capability(String),
}

pub type DomainResult<T> = Result<T, DomainError>;
