use semver::Version;
use serde::{Deserialize, Serialize};

use crate::error::{DomainError, DomainResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionCheck {
    pub local: String,
    pub remote: String,
    pub newer: bool,
}

pub fn parse_version(value: &str) -> DomainResult<Version> {
    Version::parse(value.trim()).map_err(|err| DomainError::InvalidSemver(err.to_string()))
}

/// Never compare versions as strings.
pub fn is_newer(remote: &str, local: &str) -> DomainResult<bool> {
    Ok(parse_version(remote)? > parse_version(local)?)
}

pub fn parse_version_json(body: &str) -> DomainResult<String> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|err| DomainError::MalformedJson(err.to_string()))?;
    let version = value
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DomainError::MalformedJson("missing version field".into()))?;
    let parsed = parse_version(version)?;
    Ok(parsed.to_string())
}

pub fn is_at_least_one(version: &str) -> DomainResult<bool> {
    Ok(parse_version(version)? >= parse_version("1.0.0")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_not_string_compare() {
        assert!(is_newer("1.10.0", "1.9.0").unwrap());
        assert!(!is_newer("1.9.0", "1.10.0").unwrap());
        assert!(is_newer("1.0.0", "1.0.0-beta.1").unwrap());
    }

    #[test]
    fn rejects_malformed() {
        assert!(parse_version("latest").is_err());
        assert!(parse_version_json("not-json").is_err());
        assert!(parse_version_json("{\"version\":\"nope\"}").is_err());
        assert_eq!(parse_version_json("{\"version\":\"1.4.0\"}").unwrap(), "1.4.0");
    }
}
