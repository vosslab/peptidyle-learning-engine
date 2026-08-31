use std::fmt::Write as _;

use domain::draft_preview::materialize_prompt;
use domain::generator::generate;
use question_model::envelope::QuestionEnvelope;
use question_model::generation::Seed;
use question_model::{AttemptProvenance, DraftQuestionDefinition, QuestionDefinition};
use sha2::{Digest, Sha256};

use crate::generator::AuthorPresentationContent;
use crate::registry::NativeExecution;
use crate::reproduction::resolve_asset_objects;
use crate::{
    AssetObjectBinding, MaterializedNativeQuestion, NativeAdapter, NativeAdapterError,
    NativeDraftAuthorPresentation, NativeIssuedAttempt, PreparedNativeQuestion,
};

impl NativeAdapter {
    /// Generates one key-free native Issued Question.
    ///
    /// Trusted asset bindings are resolved against the generated envelope,
    /// then canonical immutable object IDs are persisted in provenance.
    pub fn issue(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
        asset_bindings: &[AssetObjectBinding],
    ) -> Result<NativeIssuedAttempt, NativeAdapterError> {
        let prepared = self.prepare(question, seed)?;
        let asset_objects = resolve_asset_objects(&prepared.envelope, asset_bindings)?;
        let provenance = AttemptProvenance {
            adapter: self.current_adapter.clone(),
            renderer: None,
            generator: prepared.generated.generator.clone(),
            source_artifact: None,
            asset_objects,
            grading: self.current_grading.clone(),
            rendered_question_sha256: prepared.rendered_question_sha256,
        };
        Ok(NativeIssuedAttempt {
            envelope: prepared.envelope,
            parameter_hash: prepared.parameter_hash,
            provenance,
        })
    }

    /// Builds one deterministic instructor presentation without returning a key.
    ///
    /// `Ok(None)` means the installed implementation has no safe display-ready author
    /// presentation. Callers surface that state rather than serializing a key.
    pub fn author_presentation(
        &self,
        question: &DraftQuestionDefinition,
        seed: Seed,
    ) -> Result<Option<NativeDraftAuthorPresentation>, NativeAdapterError> {
        question
            .metadata
            .validate_title()
            .map_err(NativeAdapterError::InvalidTitle)?;
        let generated =
            generate(seed, &question.randomization).map_err(NativeAdapterError::Generation)?;
        let implementation =
            self.implementation_for_draft(question, generated.generator.as_ref())?;
        let prompt = materialize_prompt(&question.prompt, seed, &question.randomization)
            .map_err(NativeAdapterError::Presentation)?;
        let Some(AuthorPresentationContent {
            correct_response,
            rationale,
        }) = implementation.derive_author_presentation(question, &generated, &prompt)?
        else {
            return Ok(None);
        };
        Ok(Some(NativeDraftAuthorPresentation {
            title: question.metadata.title.clone(),
            prompt,
            response: question.response.clone(),
            correct_response,
            rationale,
        }))
    }

    pub(super) fn prepare_with_execution(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
        execution: &NativeExecution,
    ) -> Result<PreparedNativeQuestion, NativeAdapterError> {
        question
            .metadata
            .validate_title()
            .map_err(NativeAdapterError::InvalidTitle)?;
        let generated =
            generate(seed, &question.randomization).map_err(NativeAdapterError::Generation)?;
        let implementation =
            self.implementation_for_question(question, generated.generator.as_ref())?;
        let parameter_hash = generated.sha256().map_err(NativeAdapterError::Generation)?;
        let prompt = materialize_prompt(&question.prompt, seed, &question.randomization)
            .map_err(NativeAdapterError::Presentation)?;
        let materialized = MaterializedNativeQuestion {
            prompt,
            answer_key: execution.derive_answer_key(implementation, question, &generated)?,
        };
        let envelope = QuestionEnvelope {
            question_version: question_model::QuestionVersionReference {
                question_id: question.question_id.clone(),
                version_number: question.version_number,
            },
            seed,
            title: question.metadata.title.clone(),
            prompt: materialized.prompt.clone(),
            response: question.response.clone(),
        };
        let rendered_question_sha256 = hash_json(&envelope)?;
        Ok(PreparedNativeQuestion {
            generated,
            materialized,
            envelope,
            parameter_hash,
            rendered_question_sha256,
        })
    }

    pub(super) fn prepare(
        &self,
        question: &QuestionDefinition,
        seed: Seed,
    ) -> Result<PreparedNativeQuestion, NativeAdapterError> {
        let execution = self.execution_for(
            &self.adapter_implementations,
            &self.current_adapter,
            "adapter",
        )?;
        self.prepare_with_execution(question, seed, execution)
    }
}

fn hash_json(value: &QuestionEnvelope) -> Result<String, NativeAdapterError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| NativeAdapterError::Serialization(error.to_string()))?;
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}
