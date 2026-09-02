//! Server-only WeBWorK grading composition.

use grading::QuestionGradingOutcome;
use question_model::generation::QuestionSeed;
use question_model::{Capability, QuestionRevision, StudentResponse};

use super::{ResolvedWebworkQuestionSource, WebworkAdapterError};
use crate::renderer_contract::{
    GradeRequest, WebworkQuestionAttemptReplayDetails, WebworkRenderer,
};

/// Delegates a student response under the exact source's accepted grading policy.
pub(super) async fn grade<R: WebworkRenderer>(
    renderer: &R,
    question: &QuestionRevision,
    seed: QuestionSeed,
    source: &ResolvedWebworkQuestionSource,
    response: &StudentResponse,
    replay: &WebworkQuestionAttemptReplayDetails,
) -> Result<QuestionGradingOutcome, WebworkAdapterError> {
    let (question_revision, pg_path) = crate::source_object_reference::webwork_identity(question)?;
    crate::source_object_reference::verify_source(source)?;
    crate::source_object_reference::verify_source_binding(source, &question_revision)?;
    let (points_possible, partial_credit) = match question.grading {
        question_model::QuestionGradingRule::AllOrNothing { points }
            if points.is_finite() && points >= 0.0 =>
        {
            (points, false)
        }
        question_model::QuestionGradingRule::PartialCredit { points }
            if points.is_finite()
                && points >= 0.0
                && crate::reviewed_webwork_source_capabilities(
                    &question.backend_locator,
                    source.source_object_checksum().as_str(),
                )?
                .supports(Capability::PartialCredit) =>
        {
            (points, true)
        }
        question_model::QuestionGradingRule::PartialCredit { .. } => {
            return Err(WebworkAdapterError::InvalidRendererQuestionPresentation(
                "WeBWorK partial credit requires an accepted source profile".to_string(),
            ));
        }
        question_model::QuestionGradingRule::Ungraded => {
            return Ok(QuestionGradingOutcome::Ungraded);
        }
        _ => {
            return Err(WebworkAdapterError::InvalidRendererQuestionPresentation(
                "WeBWorK grading requires finite nonnegative points".to_string(),
            ));
        }
    };
    renderer
        .grade(GradeRequest {
            pg_source: source.pg_source(),
            pg_path,
            question_revision: &question_revision,
            seed: seed.value(),
            response,
            replay,
            points_possible,
            partial_credit,
        })
        .await
        .map_err(WebworkAdapterError::Renderer)
}
