use std::fmt::Write as _;

use domain::draft_preview::materialize_prompt;
use domain::generator::generate;
use question_model::envelope::QuestionVariationPresentation;
use question_model::generation::QuestionSeed;
use question_model::{DraftQuestionRevision, QuestionAttemptReproductionDetails, QuestionRevision};
use sha2::{Digest, Sha256};

use crate::generator::AuthorPresentationContent;
use crate::registry::PleQuestionExecution;
use crate::reproduction::resolve_question_asset_objects;
use crate::{
    MaterializedNativeQuestion, PleDraftAuthorPresentation, PleIssuedQuestion, PleQuestionBackend,
    PleQuestionBackendError, PreparedNativeQuestion, QuestionAssetObjectReference,
};

impl PleQuestionBackend {
    /// Generates one key-free native Issued Question.
    ///
    /// Trusted Question Asset Object References are resolved against the generated envelope,
    /// then canonical immutable object IDs are persisted in the reproduction details.
    pub fn issue(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
        question_asset_object_references: &[QuestionAssetObjectReference],
    ) -> Result<PleIssuedQuestion, PleQuestionBackendError> {
        let prepared = self.prepare(question, seed)?;
        let asset_objects =
            resolve_question_asset_objects(&prepared.envelope, question_asset_object_references)?;
        let reproduction_details = QuestionAttemptReproductionDetails {
            backend: self.current_backend.clone(),
            renderer_version: None,
            generator: prepared.generated.generator.clone(),
            source_object_reference: None,
            source_object_checksum: None,
            asset_objects,
            grader: self.current_grader.clone(),
            rendered_question_sha256: prepared.rendered_question_sha256,
        };
        Ok(PleIssuedQuestion {
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
        question: &DraftQuestionRevision,
        seed: QuestionSeed,
    ) -> Result<Option<PleDraftAuthorPresentation>, PleQuestionBackendError> {
        question
            .metadata
            .validate_title()
            .map_err(PleQuestionBackendError::InvalidTitle)?;
        let generated = generate(seed, &question.question_variation_definition)
            .map_err(PleQuestionBackendError::Generation)?;
        let implementation =
            self.implementation_for_draft(question, generated.generator.as_ref())?;
        let prompt = materialize_prompt(
            &question.prompt,
            seed,
            &question.question_variation_definition,
        )
        .map_err(PleQuestionBackendError::Presentation)?;
        let Some(AuthorPresentationContent {
            question_answer,
            question_answer_explanation,
        }) = implementation.derive_author_presentation(question, &generated, &prompt)?
        else {
            return Ok(None);
        };
        Ok(Some(PleDraftAuthorPresentation {
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
        execution: &PleQuestionExecution,
    ) -> Result<PreparedNativeQuestion, PleQuestionBackendError> {
        question
            .metadata
            .validate_title()
            .map_err(PleQuestionBackendError::InvalidTitle)?;
        let generated = generate(seed, &question.question_variation_definition)
            .map_err(PleQuestionBackendError::Generation)?;
        let implementation =
            self.implementation_for_question(question, generated.generator.as_ref())?;
        let parameter_hash = generated
            .sha256()
            .map_err(PleQuestionBackendError::Generation)?;
        let prompt = materialize_prompt(
            &question.prompt,
            seed,
            &question.question_variation_definition,
        )
        .map_err(PleQuestionBackendError::Presentation)?;
        let materialized = MaterializedNativeQuestion {
            prompt,
            answer_key: execution.derive_answer_key(implementation, question, &generated)?,
        };
        let envelope = QuestionVariationPresentation {
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
    ) -> Result<PreparedNativeQuestion, PleQuestionBackendError> {
        let execution = self.backend_execution_for(&self.current_backend)?;
        self.prepare_with_execution(question, seed, execution)
    }
}

fn hash_json(value: &QuestionVariationPresentation) -> Result<String, PleQuestionBackendError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| PleQuestionBackendError::Serialization(error.to_string()))?;
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}
