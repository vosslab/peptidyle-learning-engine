//! Server-only WeBWorK grading composition.

use grading::QuestionGradingOutcome;
use question_model::StudentResponse;
use question_model::generation::QuestionSeed;

use super::{ResolvedWebworkQuestionSource, WebworkAdapterError};
use crate::renderer_contract::{
    GradeRequest, WebworkQuestionAttemptReplayDetails, WebworkRenderer,
};

/// Delegates a student response under the exact source's accepted grading policy.
pub(super) async fn grade<R: WebworkRenderer>(
    renderer: &R,
    seed: QuestionSeed,
    source: &ResolvedWebworkQuestionSource,
    response: &StudentResponse,
    replay: &WebworkQuestionAttemptReplayDetails,
) -> Result<QuestionGradingOutcome, WebworkAdapterError> {
    crate::source_object_reference::verify_source(source)?;
    renderer
        .grade(GradeRequest {
            pg_source: source.pg_source(),
            pg_path: source.pg_path(),
            question_revision: source.question_revision(),
            seed: seed.value(),
            response,
            replay,
        })
        .await
        .map_err(WebworkAdapterError::Renderer)
}
