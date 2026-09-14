use serde::{Deserialize, Serialize};

pub const SETTINGS_NAMESPACE: &str = "ai";
pub const OPENROUTER_SECRET_KIND: &str = "openrouter_api_key";
pub const MAX_MODEL_SLOTS: usize = 4;
pub const ACTIVE_FALLBACK_SLOTS: usize = MAX_MODEL_SLOTS;
pub const DEFAULT_LOCAL_CTX_LEN: u32 = 8192;
pub const MIN_LOCAL_CTX_LEN: u32 = 512;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AiProviderKind {
    #[default]
    OpenRouter,
    Local,
}

impl AiProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenRouter => "openrouter",
            Self::Local => "local",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" => Self::Local,
            _ => Self::OpenRouter,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    pub enabled: bool,
    pub models: Vec<String>,
    pub commit_follow_style: bool,
    pub setup_dismissed: bool,
    #[serde(default)]
    pub provider: AiProviderKind,
    #[serde(default)]
    pub local_model_id: Option<String>,
    #[serde(default = "default_local_ctx_len")]
    pub local_ctx_len: u32,
    #[serde(default = "default_true")]
    pub local_gpu_offload: bool,
    #[serde(default)]
    pub local_idle_unload_minutes: u32,
}

fn default_local_ctx_len() -> u32 {
    DEFAULT_LOCAL_CTX_LEN
}

/// Trained GGUF context length. `None` if the file has no usable value.
pub fn local_ctx_len_for_model(n_ctx_train: Option<u32>) -> Option<u32> {
    n_ctx_train.filter(|n| *n > 0)
}

fn default_true() -> bool {
    true
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            models: vec![String::new(); MAX_MODEL_SLOTS],
            commit_follow_style: true,
            setup_dismissed: false,
            provider: AiProviderKind::OpenRouter,
            local_model_id: None,
            local_ctx_len: DEFAULT_LOCAL_CTX_LEN,
            local_gpu_offload: true,
            local_idle_unload_minutes: 0,
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
        self.local_model_id = self
            .local_model_id
            .take()
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty());
        self.local_ctx_len = self.local_ctx_len.max(MIN_LOCAL_CTX_LEN);
        self.local_idle_unload_minutes = self.local_idle_unload_minutes.min(24 * 60);
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

    pub fn apply_trained_context(&mut self, n_ctx_train: Option<u32>) {
        if let Some(n) = local_ctx_len_for_model(n_ctx_train) {
            self.local_ctx_len = n;
        }
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
        s.models = vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()];
        s = s.normalize();
        assert_eq!(s.models.len(), 4);
        assert_eq!(s.active_models(), vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn defaults_to_openrouter() {
        let s = AiSettings::default();
        assert_eq!(s.provider, AiProviderKind::OpenRouter);
        assert!(s.local_gpu_offload);
        assert_eq!(s.local_ctx_len, DEFAULT_LOCAL_CTX_LEN);
    }

    #[test]
    fn blank_local_id_becomes_none() {
        let mut s = AiSettings::default();
        s.local_model_id = Some("  ".into());
        s = s.normalize();
        assert!(s.local_model_id.is_none());
    }

    #[test]
    fn trained_context_uses_model_max() {
        assert_eq!(local_ctx_len_for_model(Some(32_768)), Some(32_768));
        assert_eq!(local_ctx_len_for_model(Some(8)), Some(8));
        assert_eq!(local_ctx_len_for_model(Some(262_144)), Some(262_144));
        assert_eq!(local_ctx_len_for_model(Some(1_000_000)), Some(1_000_000));
        assert_eq!(local_ctx_len_for_model(None), None);
        assert_eq!(local_ctx_len_for_model(Some(0)), None);

        let mut s = AiSettings::default();
        s.apply_trained_context(Some(262_144));
        s = s.normalize();
        assert_eq!(s.local_ctx_len, 262_144);
        s.apply_trained_context(None);
        assert_eq!(s.local_ctx_len, 262_144);
    }
}
