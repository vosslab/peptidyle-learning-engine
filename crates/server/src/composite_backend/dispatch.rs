//! Published-source capability dispatch for the composite backend.

use question_model::{BackendCapabilities, DraftQuestionSource};

use crate::catalog::{BackendRegistry, BackendRegistryError};

use super::CompositeBackend;

impl<S, O, R> BackendRegistry for CompositeBackend<S, O, R>
where
    S: Send + Sync,
    O: Send + Sync,
    R: Send + Sync,
{
    fn capabilities(
        &self,
        source: &DraftQuestionSource,
    ) -> Result<BackendCapabilities, BackendRegistryError> {
        match source {
            DraftQuestionSource::Native { .. } => self.native.capabilities(source),
            DraftQuestionSource::Webwork { pg_path } if self.webwork.is_some() => {
                adapter_webwork::webwork_source_capabilities(
                    &question_model::QuestionSource::Webwork {
                        pg_path: pg_path.clone(),
                    },
                )
                .map_err(|_| BackendRegistryError::Unsupported)
            }
            DraftQuestionSource::Imathas { provider, .. }
                if self
                    .imathas
                    .as_ref()
                    .is_some_and(|backend| backend.serves_provider(provider)) =>
            {
                Ok(BackendCapabilities::from_iter([
                    question_model::Capability::AlgorithmicGeneration,
                    question_model::Capability::ServerGrading,
                    question_model::Capability::PartialCredit,
                ]))
            }
            DraftQuestionSource::Qti { .. } if self.qti.is_configured() => {
                Ok(BackendCapabilities::from_iter([
                    question_model::Capability::ServerGrading,
                ]))
            }
            _ => Err(BackendRegistryError::Unsupported),
        }
    }
}
