use std::fmt::Write as _;

use domain::draft_preview::build_question_prompt;
use question_model::QuestionVariationPresentation;
use question_model::generation::QuestionSeed;
use question_model::{
    DraftQuestionContent, QuestionAttemptReproductionDetails, QuestionRevision,
    SourceObjectChecksum, SourceObjectReference,
};
use sha2::{Digest, Sha256};

use crate::generator::AuthorPresentationContent;
use crate::reproduction::resolve_question_asset_objects;
use crate::{
    DerivedPleQuestion, PleDraftAuthorPresentation, PleIssuedQuestion, PleQuestionBackend,
    PleQuestionBackendError, PreparedPleQuestion, QuestionAssetObjectReference,
};

impl PleQuestionBackend {
    /// Generates one key-free PLE Issued Question.
    ///
    /// Trusted Question Asset Object References are resolved against the generated Question Presentation,
    /// then canonical immutable object IDs are persisted in the reproduction details.
    pub fn issue(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
        source_object_reference: &SourceObjectReference,
        source_object_checksum: &SourceObjectChecksum,
        question_asset_object_references: &[QuestionAssetObjectReference],
    ) -> Result<PleIssuedQuestion, PleQuestionBackendError> {
        let prepared = self.prepare(question, seed)?;
        let asset_objects = resolve_question_asset_objects(
            &prepared.presentation,
            question_asset_object_references,
        )?;
        let reproduction_details = QuestionAttemptReproductionDetails {
            backend: self.current_backend.clone(),
            renderer_version: None,
            source_object_reference: Some(source_object_reference.clone()),
            source_object_checksum: Some(source_object_checksum.clone()),
            asset_objects,
            grader: self.current_grader.clone(),
            rendered_question_sha256: prepared.rendered_question_sha256,
        };
        Ok(PleIssuedQuestion {
            presentation: prepared.presentation,
            reproduction_details,
        })
    }

    /// Builds one deterministic instructor presentation without returning a key.
    ///
    /// `Ok(None)` means the installed implementation has no safe display-ready author
    /// presentation. Callers surface that state rather than serializing a key.
    pub fn author_presentation(
        &self,
        question: &DraftQuestionContent,
    ) -> Result<Option<PleDraftAuthorPresentation>, PleQuestionBackendError> {
        question
            .metadata
            .validate_title()
            .map_err(PleQuestionBackendError::InvalidTitle)?;
        let implementation = self.implementation_for_draft(question)?;
        let prompt = build_question_prompt(&question.prompt);
        let Some(AuthorPresentationContent {
            question_answer,
            question_answer_explanation,
        }) = implementation.derive_author_presentation(question, &prompt)?
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
    ) -> Result<PreparedPleQuestion, PleQuestionBackendError> {
        question
            .metadata
            .validate_title()
            .map_err(PleQuestionBackendError::InvalidTitle)?;
        let implementation = self.implementation_for_question(question)?;
        let prompt = build_question_prompt(&question.prompt);
        let derived = DerivedPleQuestion {
            prompt,
            answer_key: implementation.derive_answer_key(question)?,
        };
        let presentation = QuestionVariationPresentation {
            variation: question_model::QuestionVariation::from_question_revision_and_seed(
                question_model::QuestionRevisionReference {
                    question_id: question.question_id.clone(),
                    revision_number: question.revision_number,
                },
                seed,
            ),
            title: question.metadata.title.clone(),
            prompt: derived.prompt.clone(),
            response: question.response.clone(),
        };
        let rendered_question_sha256 = hash_json(&presentation)?;
        Ok(PreparedPleQuestion {
            derived,
            presentation,
            rendered_question_sha256,
        })
    }

    pub(super) fn prepare(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
    ) -> Result<PreparedPleQuestion, PleQuestionBackendError> {
        self.require_backend_version(&self.current_backend)?;
        self.prepare_with_execution(question, seed)
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
