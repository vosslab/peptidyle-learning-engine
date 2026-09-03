use grading::{QuestionGradingOutcome, grade};
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
        recorded_reproduction_details: &QuestionAttemptReproductionDetails,
        question_asset_object_references: &[QuestionAssetObjectReference],
    ) -> Result<Option<QuestionHint>, PleQuestionBackendError> {
        self.require_backend_version(&recorded_reproduction_details.backend)?;
        let prepared = self.prepare_with_execution(question, seed)?;
        verify_record(
            &prepared,
            recorded_reproduction_details,
            &resolve_question_asset_objects(
                &prepared.presentation,
                question_asset_object_references,
            )?,
        )?;
        let implementation = self.implementation_for_question(question)?;
        implementation.derive_hint(
            question,
            &prepared.presentation,
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
        recorded_reproduction_details: &QuestionAttemptReproductionDetails,
        question_asset_object_references: &[QuestionAssetObjectReference],
        response: &StudentResponse,
    ) -> Result<QuestionGradingOutcome, PleQuestionBackendError> {
        self.require_backend_version(&recorded_reproduction_details.backend)?;
        self.require_grader_version(&recorded_reproduction_details.grader)?;
        let prepared = self.prepare_with_execution(question, seed)?;
        verify_record(
            &prepared,
            recorded_reproduction_details,
            &resolve_question_asset_objects(
                &prepared.presentation,
                question_asset_object_references,
            )?,
        )?;
        grade(question, response, prepared.derived.answer_key.as_ref())
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
        recorded_reproduction_details: &QuestionAttemptReproductionDetails,
        question_asset_object_references: &[QuestionAssetObjectReference],
        response: &StudentResponse,
    ) -> Result<PleQuestionGradingEvaluation, PleQuestionBackendError> {
        self.require_backend_version(&recorded_reproduction_details.backend)?;
        self.require_grader_version(&recorded_reproduction_details.grader)?;
        let prepared = self.prepare_with_execution(question, seed)?;
        verify_record(
            &prepared,
            recorded_reproduction_details,
            &resolve_question_asset_objects(
                &prepared.presentation,
                question_asset_object_references,
            )?,
        )?;
        let outcome = grade(question, response, prepared.derived.answer_key.as_ref())
            .map_err(PleQuestionBackendError::Grading)?;
        let QuestionGradingOutcome::Graded(result) = &outcome else {
            return Ok(PleQuestionGradingEvaluation {
                outcome,
                question_feedback: Default::default(),
                question_answer: None,
                question_answer_explanation: None,
            });
        };
        let implementation = self.implementation_for_question(question)?;
        let (question_feedback, question_answer, question_answer_explanation) = implementation
            .derive_question_feedback_answer_and_explanation(
                question,
                &prepared.presentation,
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
