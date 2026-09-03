//! Conversion of validated upstream score output.

use grading::QuestionGradingOutcome;
use question_model::QuestionEvaluation;

use crate::renderer_contract::RendererFailure;

/// Converts one bounded percentage under the published grading policy.
pub(super) fn score(score: f64) -> Result<QuestionGradingOutcome, RendererFailure> {
    let correct = score == 100.0;
    let evaluation = QuestionEvaluation::new(correct, score / 100.0).map_err(|_| {
        RendererFailure::InvalidOutput("renderer returned malformed normalized score".into())
    })?;
    Ok(QuestionGradingOutcome::Evaluated(evaluation))
}
