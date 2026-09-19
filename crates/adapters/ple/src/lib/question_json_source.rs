//! Execution of one exact immutable PLE Question JSON source.

use std::fmt::Write as _;

use objects::{ObjectStore, ResolvedQuestionSource};
use question_model::generation::QuestionReproduction;
use question_model::{
    GradingResult, ObjectId, QuestionAttemptReproductionDetails, QuestionBackendVersion,
    QuestionGraderVersion, QuestionRevisionTuple, QuestionVariation, QuestionVariationPresentation,
    SourceObjectChecksum, StudentResponse,
};
use sha2::{Digest, Sha256};

use crate::question_json::{
    CompiledPleQuestionJson, PLE_QUESTION_JSON_MEDIA_TYPE, PleQuestionJsonDocument,
    PleQuestionJsonEvaluation, PleQuestionJsonRecordedTeachingContent,
};
use crate::{
    ADAPTER_ID, ADAPTER_VERSION, GRADING_ID, GRADING_VERSION, PleIssuedQuestion,
    PleQuestionBackend, PleQuestionBackendError,
};

/// Verified PLE source bytes and their exact immutable Question Revision Tuple.
#[derive(Clone)]
pub struct ResolvedPleQuestionJsonSource {
    source: ResolvedQuestionSource,
    compiled: CompiledPleQuestionJson,
}

impl ResolvedPleQuestionJsonSource {
    /// Resolves, parses, and compiles the source attached to this exact revision.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        question_revision_tuple: QuestionRevisionTuple,
        source_object_id: ObjectId,
        source_object_checksum: SourceObjectChecksum,
    ) -> Result<Self, PleQuestionBackendError> {
        let source = ResolvedQuestionSource::resolve(
            store,
            question_revision_tuple,
            source_object_id,
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

    pub fn question_revision_tuple(&self) -> &QuestionRevisionTuple {
        self.source.question_revision_tuple()
    }
    pub fn source_object_id(&self) -> &ObjectId {
        self.source.source_object_id()
    }
    pub fn source_object_checksum(&self) -> &SourceObjectChecksum {
        self.source.source_object_checksum()
    }
    pub fn source_bytes(&self) -> &[u8] {
        self.source.bytes()
    }
}

impl PleQuestionBackend {
    /// Produces one answer-free native presentation without constructing
    /// Attempt reproduction, grading, lifecycle, or persistence values.
    pub fn preview_question_json(
        &self,
        source: &ResolvedPleQuestionJsonSource,
    ) -> QuestionVariationPresentation {
        presentation(source)
    }

    /// Issues an answer-free presentation from verified PLE source bytes.
    pub fn issue_question_json(
        &self,
        source: &ResolvedPleQuestionJsonSource,
    ) -> Result<PleIssuedQuestion, PleQuestionBackendError> {
        let presentation = presentation(source);
        // The generic presentation serialization intentionally excludes raw
        // author source. Its immutable answer-free descriptor is nonetheless
        // part of the retained rendering integrity record.
        let rendered_question_sha256 = sha256_hex(
            &serde_json::to_vec(&(&presentation, source.compiled.author_content()))
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
                source_object_id: Some(source.source_object_id().clone()),
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

    /// Grades from the same verified source and exact attempted Revision Tuple.
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

    /// Projects teaching content from exact source and already-recorded grading
    /// evidence without evaluating a Student response.
    pub fn project_recorded_question_json_teaching_content(
        &self,
        source: &ResolvedPleQuestionJsonSource,
        response: Option<&StudentResponse>,
        recorded_result: Option<GradingResult>,
    ) -> Result<PleQuestionJsonRecordedTeachingContent, PleQuestionBackendError> {
        source
            .compiled
            .private()
            .project_recorded_teaching_content(
                source.compiled.private().public_content_checksum(),
                source.compiled.presentation().question_type(),
                source.compiled.presentation().response(),
                response,
                recorded_result,
            )
            .map_err(PleQuestionBackendError::QuestionSourceDocument)
    }
}

fn presentation(source: &ResolvedPleQuestionJsonSource) -> QuestionVariationPresentation {
    QuestionVariationPresentation {
        variation: QuestionVariation::from_question_revision_and_reproduction(
            source.question_revision_tuple().clone(),
            QuestionReproduction::Static,
        ),
        question_title: source.compiled.presentation().question_title().to_string(),
        prompt: source.compiled.presentation().prompt().to_vec(),
        response: source.compiled.presentation().response().clone(),
        native_choice_order: source.compiled.presentation().native_choice_order(),
        author_content: source.compiled.author_content().cloned(),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}
