use crate::error::{AiError, AiResult};
use crate::settings::AiSettings;

pub fn require_ready(settings: &AiSettings, has_key: bool) -> AiResult<()> {
    if !settings.enabled {
        return Err(AiError::Disabled);
    }
    if !has_key {
        return Err(AiError::MissingKey);
    }
    if settings.active_models().is_empty() {
        return Err(AiError::NoModels);
    }
    Ok(())
}

pub fn looks_like_sentence(query: &str) -> bool {
    let q = query.trim();
    q.len() >= 12 && q.contains(' ') && !q.starts_with('/')
}

#[cfg(test)]
mod extra_tests {
    use super::*;

    #[test]
    fn disabled_and_missing_key() {
        let s = AiSettings::default();
        assert!(matches!(require_ready(&s, false), Err(AiError::MissingKey)));
        let mut off = s.clone();
        off.enabled = false;
        assert!(matches!(require_ready(&off, true), Err(AiError::Disabled)));
    }

    #[test]
    fn sentence_detection() {
        assert!(looks_like_sentence("which course has an exam next week"));
        assert!(!looks_like_sentence("todo"));
        assert!(!looks_like_sentence("/settings"));
    }
}
