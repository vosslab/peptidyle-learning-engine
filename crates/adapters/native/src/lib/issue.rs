use std::fmt::Write as _;

use domain::draft_preview::materialize_prompt;
use domain::generator::generate;
use question_model::envelope::QuestionPresentation;
use question_model::generation::QuestionSeed;
use question_model::{
    DraftQuestionDefinition, QuestionAttemptReproductionDetails, QuestionRevision,
};
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
    /// then canonical immutable object IDs are persisted in the reproduction details.
    pub fn issue(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
        asset_bindings: &[AssetObjectBinding],
    ) -> Result<NativeIssuedAttempt, NativeAdapterError> {
        let prepared = self.prepare(question, seed)?;
        let asset_objects = resolve_asset_objects(&prepared.envelope, asset_bindings)?;
        let reproduction_details = QuestionAttemptReproductionDetails {
            backend: self.current_backend.clone(),
            renderer_version: None,
            generator: prepared.generated.generator.clone(),
            source_object_reference: None,
            asset_objects,
            grader: self.current_grader.clone(),
            rendered_question_sha256: prepared.rendered_question_sha256,
        };
        Ok(NativeIssuedAttempt {
            envelope: prepared.envelope,
            parameter_hash: prepared.parameter_hash,
            reproduction_details,
        })
    }

    /// Builds one deterministic instructor presentation without returning a key.
    ///
    /// `Ok(None)` means the installed implementation has no safe display-ready author
    /// presentation. Callers surface that state rather than serializing a key.
    pub fn author_presentation(
        &self,
        question: &DraftQuestionDefinition,
        seed: QuestionSeed,
    ) -> Result<Option<NativeDraftAuthorPresentation>, NativeAdapterError> {
        question
            .metadata
            .validate_title()
            .map_err(NativeAdapterError::InvalidTitle)?;
        let generated = generate(seed, &question.question_variation_definition)
            .map_err(NativeAdapterError::Generation)?;
        let implementation =
            self.implementation_for_draft(question, generated.generator.as_ref())?;
        let prompt = materialize_prompt(
            &question.prompt,
            seed,
            &question.question_variation_definition,
        )
        .map_err(NativeAdapterError::Presentation)?;
        let Some(AuthorPresentationContent {
            question_answer,
            question_answer_explanation,
        }) = implementation.derive_author_presentation(question, &generated, &prompt)?
        else {
            return Ok(None);
        };
        Ok(Some(NativeDraftAuthorPresentation {
            title: question.metadata.title.clone(),
            prompt,
            response: question.response.clone(),
            question_answer,
            question_answer_explanation,
        }))
    }

    pub(super) fn prepare_with_execution(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
        execution: &NativeExecution,
    ) -> Result<PreparedNativeQuestion, NativeAdapterError> {
        question
            .metadata
            .validate_title()
            .map_err(NativeAdapterError::InvalidTitle)?;
        let generated = generate(seed, &question.question_variation_definition)
            .map_err(NativeAdapterError::Generation)?;
        let implementation =
            self.implementation_for_question(question, generated.generator.as_ref())?;
        let parameter_hash = generated.sha256().map_err(NativeAdapterError::Generation)?;
        let prompt = materialize_prompt(
            &question.prompt,
            seed,
            &question.question_variation_definition,
        )
        .map_err(NativeAdapterError::Presentation)?;
        let materialized = MaterializedNativeQuestion {
            prompt,
            answer_key: execution.derive_answer_key(implementation, question, &generated)?,
        };
        let envelope = QuestionPresentation {
            variation: question_model::QuestionVariation::from_question_variation_definition(
                question_model::QuestionRevisionReference {
                    question_id: question.question_id.clone(),
                    revision_number: question.revision_number,
                },
                &question.question_variation_definition,
                seed,
            ),
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
        question: &QuestionRevision,
        seed: QuestionSeed,
    ) -> Result<PreparedNativeQuestion, NativeAdapterError> {
        let execution = self.backend_execution_for(&self.current_backend)?;
        self.prepare_with_execution(question, seed, execution)
    }
}

fn hash_json(value: &QuestionPresentation) -> Result<String, NativeAdapterError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| NativeAdapterError::Serialization(error.to_string()))?;
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}
