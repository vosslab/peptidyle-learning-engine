use grading::GradeOutcome;
use question_model::generation::Seed;
use question_model::{AttemptProvenance, FeedbackContent, QuestionDefinition, StudentResponse};

use crate::reproduction::{resolve_asset_objects, verify_record};
use crate::{AssetObjectBinding, NativeAdapter, NativeAdapterError};

impl NativeAdapter {
    /// Reproduces an issued instance, verifies its record, and grades a response.
    ///
    /// The answer key is regenerated only inside this trusted call and never
    /// serialized or returned.
    pub fn grade(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
        recorded_parameter_hash: &str,
        recorded_provenance: &AttemptProvenance,
        asset_bindings: &[AssetObjectBinding],
        response: &StudentResponse,
    ) -> Result<GradeOutcome, NativeAdapterError> {
        let adapter_execution = self.execution_for(
            &self.adapter_implementations,
            &recorded_provenance.adapter,
            "adapter",
        )?;
        let grading_execution = self.execution_for(
            &self.grading_implementations,
            &recorded_provenance.grading,
            "grading",
        )?;
        let prepared = self.prepare_with_execution(question, seed, adapter_execution)?;
        verify_record(
            &prepared,
            recorded_parameter_hash,
            recorded_provenance,
            &resolve_asset_objects(&prepared.envelope, asset_bindings)?,
        )?;
        grading_execution
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
    /// recreated against a different instance or provenance record.
    pub fn grade_with_feedback(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
        recorded_parameter_hash: &str,
        recorded_provenance: &AttemptProvenance,
        asset_bindings: &[AssetObjectBinding],
        response: &StudentResponse,
    ) -> Result<(GradeOutcome, FeedbackContent), NativeAdapterError> {
        let adapter_execution = self.execution_for(
            &self.adapter_implementations,
            &recorded_provenance.adapter,
            "adapter",
        )?;
        let grading_execution = self.execution_for(
            &self.grading_implementations,
            &recorded_provenance.grading,
            "grading",
        )?;
        let prepared = self.prepare_with_execution(question, seed, adapter_execution)?;
        verify_record(
            &prepared,
            recorded_parameter_hash,
            recorded_provenance,
            &resolve_asset_objects(&prepared.envelope, asset_bindings)?,
        )?;
        let outcome = grading_execution
            .grade(
                question,
                response,
                prepared.materialized.answer_key.as_ref(),
            )
            .map_err(NativeAdapterError::Grading)?;
        let GradeOutcome::Graded(result) = &outcome else {
            return Ok((outcome, FeedbackContent::default()));
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
