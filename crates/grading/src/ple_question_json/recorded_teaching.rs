//! Recorded-response teaching-content projection for PLE Question JSON.

use question_model::response::{QuestionResponseFormat, QuestionType, StudentResponse};
use question_model::{GradingResult, QuestionAnswer, QuestionAnswerExplanation, QuestionFeedback};

use super::{PleQuestionJsonError, PleQuestionJsonGradingError, PleQuestionJsonPrivateGrading};

/// Server-only teaching content derived from an exact source and recorded
/// response outcome. This projection never evaluates or creates a grade.
pub struct PleQuestionJsonRecordedTeachingContent {
    pub question_feedback: Option<QuestionFeedback>,
    pub question_answer: Option<QuestionAnswer>,
    pub question_answer_explanation: Option<QuestionAnswerExplanation>,
}

impl PleQuestionJsonPrivateGrading {
    /// Selects display-ready teaching content using only server-trusted recorded
    /// evidence. The supplied result controls outcome feedback; this method
    /// deliberately does not recompute correctness from the response.
    pub fn project_recorded_teaching_content(
        &self,
        source_checksum: &str,
        question_type: QuestionType,
        response_format: &QuestionResponseFormat,
        response: Option<&StudentResponse>,
        recorded_result: Option<GradingResult>,
    ) -> Result<PleQuestionJsonRecordedTeachingContent, PleQuestionJsonError> {
        self.validate_for_source_document(source_checksum, question_type, response_format)?;
        let question_feedback = match (response, recorded_result) {
            (Some(response), Some(result)) => {
                validate_recorded_response(response_format, response)?;
                Some(self.question_feedback_for(response, result.correct)?)
            }
            (Some(response), None) => {
                validate_recorded_response(response_format, response)?;
                Some(self.selected_choice_feedback_for(response)?)
            }
            (None, _) => None,
        };
        Ok(PleQuestionJsonRecordedTeachingContent {
            question_feedback,
            question_answer: Some(self.question_answer_for(response_format)?),
            question_answer_explanation: None,
        })
    }
}

fn validate_recorded_response(
    response_format: &QuestionResponseFormat,
    response: &StudentResponse,
) -> Result<(), PleQuestionJsonError> {
    let check = domain::validation::validate_response_format(response_format, response);
    if check.is_valid() {
        Ok(())
    } else {
        Err(PleQuestionJsonError::Grading(
            PleQuestionJsonGradingError::InvalidResponse(check.issues),
        ))
    }
}
