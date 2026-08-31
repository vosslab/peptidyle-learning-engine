use grading::QuestionGradingOutcome;
use question_model::generation::QuestionSeed;
use question_model::{
    QuestionAttemptReproductionDetails, QuestionFeedback, QuestionHint, QuestionVersion,
    StudentResponse,
};

use crate::reproduction::{resolve_asset_objects, verify_record};
use crate::{AssetObjectBinding, NativeAdapter, NativeAdapterError};

impl NativeAdapter {
    /// Reproduces and verifies an issued Question before providing pre-response support.
    ///
    /// This is intentionally separate from grading: a Question Hint is requested
    /// before a Student selects or submits a response and therefore has no
    /// grading outcome or Student feedback release decision.
    pub fn hint_for_issued_question(
        &self,
        question: &QuestionVersion,
        seed: QuestionSeed,
        recorded_parameter_hash: &str,
        recorded_reproduction_details: &QuestionAttemptReproductionDetails,
        asset_bindings: &[AssetObjectBinding],
    ) -> Result<Option<QuestionHint>, NativeAdapterError> {
        let backend_execution =
            self.backend_execution_for(&recorded_reproduction_details.backend)?;
        let prepared = self.prepare_with_execution(question, seed, backend_execution)?;
        verify_record(
            &prepared,
            recorded_parameter_hash,
            recorded_reproduction_details,
            &resolve_asset_objects(&prepared.envelope, asset_bindings)?,
        )?;
        let implementation =
            self.implementation_for_question(question, prepared.generated.generator.as_ref())?;
        implementation.derive_hint(
            question,
            &prepared.generated,
            &prepared.envelope,
            prepared.materialized.answer_key.as_ref(),
        )
    }

    /// Reproduces an issued instance, verifies its record, and grades a response.
    ///
    /// The answer key is regenerated only inside this trusted call and never
    /// serialized or returned.
    pub fn grade(
        &self,
        question: &QuestionVersion,
        seed: QuestionSeed,
        recorded_parameter_hash: &str,
        recorded_reproduction_details: &QuestionAttemptReproductionDetails,
        asset_bindings: &[AssetObjectBinding],
        response: &StudentResponse,
    ) -> Result<QuestionGradingOutcome, NativeAdapterError> {
        let backend_execution =
            self.backend_execution_for(&recorded_reproduction_details.backend)?;
        let grader_execution = self.grader_execution_for(&recorded_reproduction_details.grader)?;
        let prepared = self.prepare_with_execution(question, seed, backend_execution)?;
        verify_record(
            &prepared,
            recorded_parameter_hash,
            recorded_reproduction_details,
            &resolve_asset_objects(&prepared.envelope, asset_bindings)?,
        )?;
        grader_execution
            .grade(
                question,
                response,
                prepared.materialized.answer_key.as_ref(),
            )
            .map_err(NativeAdapterError::Grading)
    }

    /// Reproduces, verifies, grades, and materializes private teaching content in one pass.
    ///
    /// Keeping this separate from [`Self::grade`] prevents feedback from being
    /// recreated against a different instance or source record.
    pub fn grade_with_feedback(
        &self,
        question: &QuestionVersion,
        seed: QuestionSeed,
        recorded_parameter_hash: &str,
        recorded_reproduction_details: &QuestionAttemptReproductionDetails,
        asset_bindings: &[AssetObjectBinding],
        response: &StudentResponse,
    ) -> Result<(QuestionGradingOutcome, QuestionFeedback), NativeAdapterError> {
        let backend_execution =
            self.backend_execution_for(&recorded_reproduction_details.backend)?;
        let grader_execution = self.grader_execution_for(&recorded_reproduction_details.grader)?;
        let prepared = self.prepare_with_execution(question, seed, backend_execution)?;
        verify_record(
            &prepared,
            recorded_parameter_hash,
            recorded_reproduction_details,
            &resolve_asset_objects(&prepared.envelope, asset_bindings)?,
        )?;
        let outcome = grader_execution
            .grade(
                question,
                response,
                prepared.materialized.answer_key.as_ref(),
            )
            .map_err(NativeAdapterError::Grading)?;
        let QuestionGradingOutcome::Graded(result) = &outcome else {
            return Ok((outcome, QuestionFeedback::default()));
        };
        let implementation =
            self.implementation_for_question(question, prepared.generated.generator.as_ref())?;
        let feedback = implementation.derive_feedback(
            question,
            &prepared.generated,
            &prepared.envelope,
            prepared.materialized.answer_key.as_ref(),
            result,
            response,
        )?;
        Ok((outcome, feedback))
    }
}
