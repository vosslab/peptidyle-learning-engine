//! Conversion of validated upstream score output.

use grading::GradeOutcome;
use question_model::AttemptResult;

use crate::renderer_contract::RendererFailure;

/// Converts one bounded percentage under the published grading policy.
pub(super) fn score(
    score: f64,
    points_possible: f64,
    partial_credit: bool,
) -> Result<GradeOutcome, RendererFailure> {
    if !partial_credit && score != 0.0 && score != 100.0 {
        return Err(RendererFailure::InvalidOutput(
            "renderer returned unsupported partial score".into(),
        ));
    }
    let correct = score == 100.0;
    Ok(GradeOutcome::Graded(AttemptResult {
        correct,
        points_earned: points_possible * score / 100.0,
        points_possible,
    }))
}
