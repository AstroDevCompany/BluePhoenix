use crate::error::{AiError, AiFailureKind, AiResult, AttemptRecord};
use crate::provider::{AiProvider, CompletionRequest, CompletionResponse};
use crate::settings::MAX_MODEL_SLOTS;

#[derive(Debug)]
pub struct FallbackOutcome {
    pub response: CompletionResponse,
    pub attempted: Vec<String>,
    pub fallback_used: bool,
}

pub async fn complete_with_fallback<P: AiProvider>(
    provider: &P,
    models: &[String],
    mut request: CompletionRequest,
) -> AiResult<FallbackOutcome> {
    if models.is_empty() {
        return Err(AiError::NoModels);
    }
    let mut attempts = Vec::new();
    for (index, model) in models.iter().take(MAX_MODEL_SLOTS).enumerate() {
        request.model = model.clone();
        match provider.complete(request.clone()).await {
            Ok(response) => {
                return Ok(FallbackOutcome {
                    fallback_used: index > 0,
                    attempted: {
                        let mut used = attempts.iter().map(|a: &AttemptRecord| a.model.clone()).collect::<Vec<_>>();
                        used.push(model.clone());
                        used
                    },
                    response,
                });
            }
            Err(err) => {
                let kind = err.kind().unwrap_or(AiFailureKind::Other);
                attempts.push(AttemptRecord {
                    model: model.clone(),
                    kind,
                    message: err.user_message(),
                });
                if !kind.should_fallback() {
                    return Err(err);
                }
            }
        }
    }
    Err(AiError::AllFailed { attempts })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ChatMessage, CompletionRequest};
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct Scripted {
        results: Mutex<Vec<AiResult<CompletionResponse>>>,
    }

    #[async_trait]
    impl AiProvider for Scripted {
        async fn complete(&self, request: CompletionRequest) -> AiResult<CompletionResponse> {
            let next = self.results.lock().unwrap().remove(0);
            next.map(|mut r| {
                r.model = request.model;
                r
            })
        }
    }

    fn req() -> CompletionRequest {
        CompletionRequest::chat(
            "unused",
            vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
        )
    }

    fn ok() -> CompletionResponse {
        CompletionResponse {
            text: "hello".into(),
            model: String::new(),
            finish_reason: Some("stop".into()),
        }
    }

    fn err(kind: AiFailureKind) -> AiError {
        AiError::provider(kind, "fail", Some("m".into()))
    }

    #[tokio::test]
    async fn first_success() {
        let p = Scripted {
            results: Mutex::new(vec![Ok(ok())]),
        };
        let out = complete_with_fallback(&p, &["a".into()], req()).await.unwrap();
        assert!(!out.fallback_used);
        assert_eq!(out.response.model, "a");
    }

    #[tokio::test]
    async fn second_success_after_unavailable() {
        let p = Scripted {
            results: Mutex::new(vec![Err(err(AiFailureKind::Unavailable)), Ok(ok())]),
        };
        let out = complete_with_fallback(&p, &["a".into(), "b".into()], req()).await.unwrap();
        assert!(out.fallback_used);
        assert_eq!(out.response.model, "b");
    }

    #[tokio::test]
    async fn third_success() {
        let p = Scripted {
            results: Mutex::new(vec![
                Err(err(AiFailureKind::RateLimit)),
                Err(err(AiFailureKind::Timeout)),
                Ok(ok()),
            ]),
        };
        let out = complete_with_fallback(&p, &["a".into(), "b".into(), "c".into()], req()).await.unwrap();
        assert_eq!(out.response.model, "c");
    }

    #[tokio::test]
    async fn all_four_fail() {
        let p = Scripted {
            results: Mutex::new(vec![
                Err(err(AiFailureKind::Unavailable)),
                Err(err(AiFailureKind::RateLimit)),
                Err(err(AiFailureKind::Timeout)),
                Err(err(AiFailureKind::Network)),
            ]),
        };
        let err = complete_with_fallback(
            &p,
            &["a".into(), "b".into(), "c".into(), "d".into()],
            req(),
        )
        .await
        .unwrap_err();
        match err {
            AiError::AllFailed { attempts } => assert_eq!(attempts.len(), 4),
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn auth_does_not_fallback() {
        let p = Scripted {
            results: Mutex::new(vec![Err(err(AiFailureKind::Auth)), Ok(ok())]),
        };
        let err = complete_with_fallback(&p, &["a".into(), "b".into()], req())
            .await
            .unwrap_err();
        assert!(matches!(err, AiError::Provider { kind: AiFailureKind::Auth, .. }));
    }

    #[tokio::test]
    async fn fewer_than_four_models() {
        let p = Scripted {
            results: Mutex::new(vec![Err(err(AiFailureKind::Unavailable)), Ok(ok())]),
        };
        let out = complete_with_fallback(&p, &["only".into(), "two".into()], req()).await.unwrap();
        assert_eq!(out.response.model, "two");
    }

    #[tokio::test]
    async fn extra_configured_still_tries_four() {
        let p = Scripted {
            results: Mutex::new(vec![
                Err(err(AiFailureKind::Unavailable)),
                Err(err(AiFailureKind::Unavailable)),
                Err(err(AiFailureKind::Unavailable)),
                Err(err(AiFailureKind::Unavailable)),
            ]),
        };
        let models: Vec<String> = (1..=8).map(|n| format!("m{n}")).collect();
        let err = complete_with_fallback(&p, &models, req()).await.unwrap_err();
        match err {
            AiError::AllFailed { attempts } => {
                assert_eq!(attempts.len(), 4);
                assert_eq!(attempts[3].model, "m4");
            }
            other => panic!("{other:?}"),
        }
    }
}
