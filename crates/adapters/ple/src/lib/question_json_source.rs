//! Execution of one exact immutable PLE Question JSON source.

use std::fmt::Write as _;

use objects::{ObjectStore, ResolvedQuestionSource};
use question_model::generation::QuestionSeed;
use question_model::{
    QuestionAttemptReproductionDetails, QuestionBackendVersion, QuestionGraderVersion,
    QuestionRevisionReference, QuestionVariation, QuestionVariationPresentation,
    SourceObjectChecksum, SourceObjectReference, StudentResponse,
};
use sha2::{Digest, Sha256};

use crate::question_json::{
    CompiledPleQuestionJson, PLE_QUESTION_JSON_MEDIA_TYPE, PleQuestionJsonDocument,
    PleQuestionJsonEvaluation,
};
use crate::{
    ADAPTER_ID, ADAPTER_VERSION, GRADING_ID, GRADING_VERSION, PleIssuedQuestion,
    PleQuestionBackend, PleQuestionBackendError,
};

/// Verified PLE source bytes and their exact immutable Question Revision reference.
#[derive(Clone)]
pub struct ResolvedPleQuestionJsonSource {
    source: ResolvedQuestionSource,
    compiled: CompiledPleQuestionJson,
}

impl ResolvedPleQuestionJsonSource {
    /// Resolves, parses, and compiles the source attached to this exact revision.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        question_revision: QuestionRevisionReference,
        source_object_reference: SourceObjectReference,
        source_object_checksum: SourceObjectChecksum,
    ) -> Result<Self, PleQuestionBackendError> {
        let source = ResolvedQuestionSource::resolve(
            store,
            question_revision,
            source_object_reference,
            source_object_checksum,
        )
        .await
        .map_err(PleQuestionBackendError::QuestionSourceResolution)?;
        if source.media_type() != PLE_QUESTION_JSON_MEDIA_TYPE {
            return Err(PleQuestionBackendError::UnexpectedQuestionSourceMediaType {
                media_type: source.media_type().to_string(),
            });
        }
        let document = PleQuestionJsonDocument::parse(source.bytes())
            .map_err(PleQuestionBackendError::QuestionSourceDocument)?;
        let compiled = document
            .compile()
            .map_err(PleQuestionBackendError::QuestionSourceDocument)?;
        Ok(Self { source, compiled })
    }

    pub fn question_revision(&self) -> &QuestionRevisionReference {
        self.source.question_revision()
    }
    pub fn source_object_reference(&self) -> &SourceObjectReference {
        self.source.source_object_reference()
    }
    pub fn source_object_checksum(&self) -> &SourceObjectChecksum {
        self.source.source_object_checksum()
    }
    pub fn source_bytes(&self) -> &[u8] {
        self.source.bytes()
    }
}

impl PleQuestionBackend {
    /// Issues an answer-free presentation from verified PLE source bytes.
    pub fn issue_question_json(
        &self,
        source: &ResolvedPleQuestionJsonSource,
        seed: QuestionSeed,
    ) -> Result<PleIssuedQuestion, PleQuestionBackendError> {
        let presentation = QuestionVariationPresentation {
            variation: QuestionVariation::from_question_revision_and_seed(
                source.question_revision().clone(),
                seed,
            ),
            title: source.compiled.presentation().title().to_string(),
            prompt: source.compiled.presentation().prompt().to_vec(),
            response: source.compiled.presentation().response().clone(),
        };
        let rendered_question_sha256 = sha256_hex(
            &serde_json::to_vec(&presentation)
                .map_err(|error| PleQuestionBackendError::Serialization(error.to_string()))?,
        );
        Ok(PleIssuedQuestion {
            presentation,
            reproduction_details: QuestionAttemptReproductionDetails {
                backend: QuestionBackendVersion {
                    name: ADAPTER_ID.to_string(),
                    version: ADAPTER_VERSION.to_string(),
                },
                renderer_version: None,
                source_object_reference: Some(source.source_object_reference().clone()),
                source_object_checksum: Some(source.source_object_checksum().clone()),
                asset_objects: Vec::new(),
                grader: QuestionGraderVersion {
                    name: GRADING_ID.to_string(),
                    version: GRADING_VERSION.to_string(),
                },
                rendered_question_sha256,
            },
        })
    }

    /// Grades from the same verified source and exact attempted revision reference.
    pub fn grade_question_json(
        &self,
        source: &ResolvedPleQuestionJsonSource,
        response: &StudentResponse,
    ) -> Result<PleQuestionJsonEvaluation, PleQuestionBackendError> {
        source
            .compiled
            .private()
            .evaluate(
                source.compiled.private().public_content_checksum(),
                source.compiled.presentation().question_type(),
                source.compiled.presentation().response(),
                response,
            )
            .map_err(PleQuestionBackendError::QuestionSourceDocument)
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}
