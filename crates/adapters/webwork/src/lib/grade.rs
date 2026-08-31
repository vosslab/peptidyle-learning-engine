//! Server-only WeBWorK grading composition.

use grading::GradeOutcome;
use question_model::generation::Seed;
use question_model::{Capability, QuestionDefinition, StudentResponse};

use super::{WebworkAdapterError, WebworkSource};
use crate::renderer_contract::{GradeRequest, WebworkRenderer, WebworkReplayMappingV1};

/// Delegates a student response under the exact source's accepted grading policy.
pub(super) async fn grade<R: WebworkRenderer>(
    renderer: &R,
    question: &QuestionDefinition,
    seed: Seed,
    source: &WebworkSource,
    response: &StudentResponse,
    replay: &WebworkReplayMappingV1,
) -> Result<GradeOutcome, WebworkAdapterError> {
    let (question_version, pg_path) = crate::artifact::webwork_identity(question)?;
    crate::artifact::verify_source(source)?;
    crate::artifact::verify_source_binding(source, &question_version)?;
    let (points_possible, partial_credit) = match question.grading {
        question_model::GradingDefinition::AllOrNothing { points }
            if points.is_finite() && points >= 0.0 =>
        {
            (points, false)
        }
        question_model::GradingDefinition::PartialCredit { points }
            if points.is_finite()
                && points >= 0.0
                && crate::reviewed_webwork_source_capabilities(
                    &question.source,
                    &source.artifact.sha256,
                )?
                .supports(Capability::PartialCredit) =>
        {
            (points, true)
        }
        question_model::GradingDefinition::PartialCredit { .. } => {
            return Err(WebworkAdapterError::InvalidRendererEnvelope(
                "WeBWorK partial credit requires an accepted source profile".to_string(),
            ));
        }
        question_model::GradingDefinition::Ungraded => return Ok(GradeOutcome::Ungraded),
        _ => {
            return Err(WebworkAdapterError::InvalidRendererEnvelope(
                "WeBWorK grading requires finite nonnegative points".to_string(),
            ));
        }
    };
    renderer
        .grade(GradeRequest {
            pg_source: &source.pg_source,
            pg_path,
            question_version: &question_version,
            seed: seed.value(),
            response,
            replay,
            points_possible,
            partial_credit,
        })
        .await
        .map_err(WebworkAdapterError::Renderer)
}
