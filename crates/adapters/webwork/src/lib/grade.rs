//! Server-only opaque WeBWorK grading composition.

use grading::QuestionGradingOutcome;
use question_model::generation::QuestionSeed;
use question_model::{BackendOwnedLifecycleState, StudentResponse};

use super::{ResolvedWebworkQuestionSource, WebworkAdapterError};
use crate::renderer_contract::{GradeRequest, WebworkRenderer};

/// Delegates one opaque student payload under the exact immutable source.
pub(super) async fn grade<R: WebworkRenderer>(
    renderer: &R,
    seed: QuestionSeed,
    source: &ResolvedWebworkQuestionSource,
    response: &StudentResponse,
) -> Result<QuestionGradingOutcome, WebworkAdapterError> {
    crate::source_object_id::verify_source(source)?;
    let StudentResponse::BackendOwned { payload } = response else {
        return Err(WebworkAdapterError::Renderer(
            crate::renderer_contract::RendererFailure::InvalidOutput(
                "WeBWorK requires a backend-owned response".to_string(),
            ),
        ));
    };
    let lifecycle_state = BackendOwnedLifecycleState::none();
    renderer
        .grade(GradeRequest {
            pg_source: source.pg_source(),
            pg_path: source.pg_path(),
            question_revision_tuple: source.question_revision_tuple(),
            seed: seed.value(),
            response_payload: payload,
            lifecycle_state: &lifecycle_state,
        })
        .await
        .map_err(WebworkAdapterError::Renderer)
}
