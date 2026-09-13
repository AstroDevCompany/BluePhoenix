//! Modular OpenRouter AI subsystem. Non-AI app features must not depend on a key.

pub mod documents;
pub mod error;
pub mod fallback;
pub mod openrouter;
pub mod parsers;
pub mod prompts;
pub mod provider;
pub mod readiness;
pub mod schema;
pub mod settings;

pub use documents::{chunk_sections, system_prompt, DocumentChunk};
pub use error::{AiError, AiFailureKind, AiResult, AttemptRecord};
pub use fallback::{complete_with_fallback, FallbackOutcome};
pub use openrouter::{parse_completion, parse_sse_delta, stream_with_fallback, OpenRouterProvider, OPENROUTER_URL};
pub use parsers::{parse_document, semantic_chunks, DocumentParser, ParsedDocument};
pub use provider::{redact, AiProvider, ChatMessage, CompletionRequest, CompletionResponse};
pub use readiness::{looks_like_sentence, require_ready};
pub use schema::{
    parse_agent_prompt, parse_changelog, parse_commit, parse_prioritize, parse_search, parse_software_folder_draft,
    parse_todos, AgentPrompt, ChangelogDraft, CommitSuggestion, PrioritizeResult, SearchInterpretation,
    SoftwareFolderDraft, TodoOrderItem, TodoProposal,
};
pub use settings::{AiSettings, ACTIVE_FALLBACK_SLOTS, MAX_MODEL_SLOTS, OPENROUTER_SECRET_KIND, SETTINGS_NAMESPACE};

pub const JOB_INDEX_FILES: &str = "index_files";
pub const JOB_INDEX_DOCUMENTS: &str = "index_documents";
pub const JOB_AI_GENERATE: &str = "ai_generate";

use bluephoenix_domain::context::ProjectContext;
use bluephoenix_domain::ids::CategoryKind;

pub trait AiAdapter: Send + Sync {
    fn kind(&self) -> CategoryKind;
    fn summarize_project(&self, ctx: &ProjectContext) -> AiResult<String>;
    fn study_coach(&self, _ctx: &ProjectContext) -> AiResult<String> {
        Err(AiError::Message("not used".into()))
    }
}
