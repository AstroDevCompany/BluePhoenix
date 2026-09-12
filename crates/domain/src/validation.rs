use crate::error::{DomainError, DomainResult};

pub fn required_name(value: &str, field: &str) -> DomainResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(DomainError::Validation(format!("{field} is required")))
    } else if trimmed.len() > 200 {
        Err(DomainError::Validation(format!("{field} is too long")))
    } else {
        Ok(trimmed.to_string())
    }
}

pub fn optional_url(value: Option<&str>) -> DomainResult<Option<String>> {
    let Some(raw) = value.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if !(raw.starts_with("https://") || raw.starts_with("http://")) {
        return Err(DomainError::Validation(format!("Invalid URL: {raw}")));
    }
    Ok(Some(raw.to_string()))
}

pub fn validate_cfu(cfu: Option<f64>) -> DomainResult<Option<f64>> {
    match cfu {
        None => Ok(None),
        Some(value) if value >= 0.0 && value <= 1000.0 => Ok(Some(value)),
        Some(value) => Err(DomainError::Validation(format!("Invalid CFU: {value}"))),
    }
}

pub fn validate_command_name(name: &str) -> DomainResult<String> {
    required_name(name, "command name")
}

pub fn validate_command_line(command: &str) -> DomainResult<String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        Err(DomainError::Validation("command is required".into()))
    } else {
        Ok(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_project_name() {
        assert!(required_name("   ", "name").is_err());
        assert_eq!(required_name(" Damien Doc ", "name").unwrap(), "Damien Doc");
    }

    #[test]
    fn urls_must_be_http() {
        assert!(optional_url(Some("github.com/x")).is_err());
        assert!(optional_url(Some("https://github.com/x")).is_ok());
        assert!(optional_url(Some("")).unwrap().is_none());
    }
}
