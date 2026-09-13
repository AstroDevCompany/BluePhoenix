use async_trait::async_trait;

use super::{LocalEngine, LocalModelSpec};
use crate::error::AiResult;
use crate::provider::{AiProvider, CompletionRequest, CompletionResponse};

pub struct LocalLlamaProvider {
    pub engine: LocalEngine,
    pub spec: LocalModelSpec,
}

impl LocalLlamaProvider {
    pub fn new(engine: LocalEngine, spec: LocalModelSpec) -> Self {
        Self { engine, spec }
    }
}

#[async_trait]
impl AiProvider for LocalLlamaProvider {
    async fn complete(&self, request: CompletionRequest) -> AiResult<CompletionResponse> {
        self.engine
            .submit(self.spec.clone(), request, &mut |_| {})
            .await
    }

    async fn stream(
        &self,
        request: CompletionRequest,
        on_delta: &mut (dyn FnMut(String) + Send),
    ) -> AiResult<CompletionResponse> {
        self.engine
            .submit(self.spec.clone(), request, on_delta)
            .await
    }
}
