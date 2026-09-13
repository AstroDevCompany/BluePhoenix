use serde::{Deserialize, Serialize};

pub const SETTINGS_NAMESPACE: &str = "ai";
pub const OPENROUTER_SECRET_KIND: &str = "openrouter_api_key";
pub const MAX_MODEL_SLOTS: usize = 4;
pub const ACTIVE_FALLBACK_SLOTS: usize = MAX_MODEL_SLOTS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    pub enabled: bool,
    pub models: Vec<String>,
    pub commit_follow_style: bool,
    pub setup_dismissed: bool,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            models: vec![String::new(); MAX_MODEL_SLOTS],
            commit_follow_style: true,
            setup_dismissed: false,
        }
    }
}

impl AiSettings {
    pub fn normalize(mut self) -> Self {
        self.models.resize(MAX_MODEL_SLOTS, String::new());
        self.models.truncate(MAX_MODEL_SLOTS);
        for slot in &mut self.models {
            *slot = slot.trim().to_string();
        }
        self
    }

    pub fn active_models(&self) -> Vec<String> {
        self.models
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .take(ACTIVE_FALLBACK_SLOTS)
            .map(|s| s.to_string())
            .collect()
    }

    pub fn first_model(&self) -> Option<String> {
        self.active_models().into_iter().next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonempty_slots_are_active() {
        let mut s = AiSettings::default();
        s.models = vec!["a".into(), "".into(), "b".into(), "c".into()];
        s = s.normalize();
        assert_eq!(s.active_models(), vec!["a", "b", "c"]);
    }

    #[test]
    fn extra_slots_are_dropped() {
        let mut s = AiSettings::default();
        s.models = vec![
            "a".into(),
            "b".into(),
            "c".into(),
            "d".into(),
            "e".into(),
        ];
        s = s.normalize();
        assert_eq!(s.models.len(), 4);
        assert_eq!(s.active_models(), vec!["a", "b", "c", "d"]);
    }
}
