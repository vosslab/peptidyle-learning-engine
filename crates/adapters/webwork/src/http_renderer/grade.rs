//! Conversion of validated upstream score output.

use grading::QuestionGradingOutcome;
use question_model::GradingResult;

use crate::renderer_contract::RendererFailure;

/// Converts one bounded percentage under the published grading policy.
pub(super) fn score(
    score: f64,
    points_possible: f64,
    partial_credit: bool,
) -> Result<QuestionGradingOutcome, RendererFailure> {
    if !partial_credit && score != 0.0 && score != 100.0 {
        return Err(RendererFailure::InvalidOutput(
            "renderer returned unsupported partial score".into(),
        ));
    }
    let correct = score == 100.0;
    Ok(QuestionGradingOutcome::Graded(GradingResult {
        correct,
        points_earned: points_possible * score / 100.0,
        points_possible,
    }))
}
