use grading::QuestionGradingOutcome;
use question_model::generation::QuestionSeed;
use question_model::{
    QuestionAttemptReproductionDetails, QuestionHint, QuestionRevision, StudentResponse,
};

use crate::reproduction::{resolve_question_asset_objects, verify_record};
use crate::{
    PleQuestionBackend, PleQuestionBackendError, PleQuestionGradingEvaluation,
    QuestionAssetObjectReference,
};

impl PleQuestionBackend {
    /// Reproduces and verifies an issued Question before providing pre-response support.
    ///
    /// This is intentionally separate from grading: a Question Hint is requested
    /// before a Student selects or submits a response and therefore has no
    /// grading outcome or Student feedback release decision.
    pub fn hint_for_issued_question(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
        recorded_parameter_hash: &str,
        recorded_reproduction_details: &QuestionAttemptReproductionDetails,
        question_asset_object_references: &[QuestionAssetObjectReference],
    ) -> Result<Option<QuestionHint>, PleQuestionBackendError> {
        let backend_execution =
            self.backend_execution_for(&recorded_reproduction_details.backend)?;
        let prepared = self.prepare_with_execution(question, seed, backend_execution)?;
        verify_record(
            &prepared,
            recorded_parameter_hash,
            recorded_reproduction_details,
            &resolve_question_asset_objects(&prepared.envelope, question_asset_object_references)?,
        )?;
        let implementation =
            self.implementation_for_question(question, prepared.generated.generator.as_ref())?;
        implementation.derive_hint(
            question,
            &prepared.generated,
            &prepared.envelope,
            prepared.derived.answer_key.as_ref(),
        )
    }

    /// Reproduces an issued instance, verifies its record, and grades a response.
    ///
    /// The answer key is regenerated only inside this trusted call and never
    /// serialized or returned.
    pub fn grade(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
        recorded_parameter_hash: &str,
        recorded_reproduction_details: &QuestionAttemptReproductionDetails,
        question_asset_object_references: &[QuestionAssetObjectReference],
        response: &StudentResponse,
    ) -> Result<QuestionGradingOutcome, PleQuestionBackendError> {
        let backend_execution =
            self.backend_execution_for(&recorded_reproduction_details.backend)?;
        let grader_execution = self.grader_execution_for(&recorded_reproduction_details.grader)?;
        let prepared = self.prepare_with_execution(question, seed, backend_execution)?;
        verify_record(
            &prepared,
            recorded_parameter_hash,
            recorded_reproduction_details,
            &resolve_question_asset_objects(&prepared.envelope, question_asset_object_references)?,
        )?;
        grader_execution
            .grade(question, response, prepared.derived.answer_key.as_ref())
            .map_err(PleQuestionBackendError::Grading)
    }

    /// Reproduces, verifies, grades, and derives separate private Question
    /// Feedback, Question Answer, and Question Answer Explanation values in one pass.
    ///
    /// Keeping this separate from [`Self::grade`] prevents feedback from being
    /// recreated against a different instance or Question Source.
    pub fn grade_with_feedback_answer_and_explanation(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
        recorded_parameter_hash: &str,
        recorded_reproduction_details: &QuestionAttemptReproductionDetails,
        question_asset_object_references: &[QuestionAssetObjectReference],
        response: &StudentResponse,
    ) -> Result<PleQuestionGradingEvaluation, PleQuestionBackendError> {
        let backend_execution =
            self.backend_execution_for(&recorded_reproduction_details.backend)?;
        let grader_execution = self.grader_execution_for(&recorded_reproduction_details.grader)?;
        let prepared = self.prepare_with_execution(question, seed, backend_execution)?;
        verify_record(
            &prepared,
            recorded_parameter_hash,
            recorded_reproduction_details,
            &resolve_question_asset_objects(&prepared.envelope, question_asset_object_references)?,
        )?;
        let outcome = grader_execution
            .grade(question, response, prepared.derived.answer_key.as_ref())
            .map_err(PleQuestionBackendError::Grading)?;
        let QuestionGradingOutcome::Graded(result) = &outcome else {
            return Ok(PleQuestionGradingEvaluation {
                outcome,
                question_feedback: Default::default(),
                question_answer: None,
                question_answer_explanation: None,
            });
        };
        let implementation =
            self.implementation_for_question(question, prepared.generated.generator.as_ref())?;
        let (question_feedback, question_answer, question_answer_explanation) = implementation
            .derive_question_feedback_answer_and_explanation(
                question,
                &prepared.generated,
                &prepared.envelope,
                prepared.derived.answer_key.as_ref(),
                result,
                response,
            )?;
        Ok(PleQuestionGradingEvaluation {
            outcome,
            question_feedback,
            question_answer,
            question_answer_explanation,
        })
    }
}
