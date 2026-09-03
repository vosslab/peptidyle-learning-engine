//! Resolved execution boundary for immutable PLE Question JSON sources.

use grading::QuestionGradingOutcome;
use objects::{ObjectStore, ResolvedQuestionSource};
use question_model::generation::QuestionSeed;
use question_model::{
    QuestionAttemptReproductionDetails, QuestionBackend, QuestionRevision,
    QuestionRevisionReference, SourceObjectChecksum, SourceObjectReference, StudentResponse,
};

use crate::{
    PleIssuedQuestion, PleQuestionBackend, PleQuestionBackendError, QuestionAssetObjectReference,
    question_json::{
        PLE_QUESTION_JSON_MEDIA_TYPE, PleQuestionJsonDocument, PleQuestionJsonPrivateGrading,
    },
};

/// Verified PLE Question JSON bytes compiled into the exact Question Revision they own.
#[derive(Clone)]
pub struct ResolvedPleQuestionJsonSource {
    source: ResolvedQuestionSource,
    question: QuestionRevision,
    private: PleQuestionJsonPrivateGrading,
}

impl ResolvedPleQuestionJsonSource {
    /// Resolves immutable PLE Question JSON bytes and refuses any public-content mismatch.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        question: &QuestionRevision,
        source_object_reference: SourceObjectReference,
        source_object_checksum: SourceObjectChecksum,
    ) -> Result<Self, PleQuestionBackendError> {
        if question.question_backend != QuestionBackend::Ple
            || question.question_format != question_model::QuestionFormat::PleQuestionJson
        {
            return Err(PleQuestionBackendError::UnsupportedSource);
        }
        let source = ResolvedQuestionSource::resolve(
            store,
            QuestionRevisionReference {
                question_id: question.question_id.clone(),
                revision_number: question.revision_number,
            },
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
            .compile(question.workspace)
            .map_err(PleQuestionBackendError::QuestionSourceDocument)?;
        let expected_question = QuestionRevision::from_draft(
            compiled.draft().clone(),
            question.question_id.clone(),
            question.revision_number,
            None,
        )
        .map_err(|_| PleQuestionBackendError::QuestionSourceDoesNotMatchQuestion)?;
        if expected_question != *question {
            return Err(PleQuestionBackendError::QuestionSourceDoesNotMatchQuestion);
        }
        Ok(Self {
            source,
            question: expected_question,
            private: compiled.private().clone(),
        })
    }

    /// The immutable Question Revision compiled from these verified source bytes.
    pub fn question(&self) -> &QuestionRevision {
        &self.question
    }

    /// Immutable object identity recorded with every issued Question Attempt.
    pub fn source_object_reference(&self) -> &SourceObjectReference {
        self.source.source_object_reference()
    }

    /// SHA-256 verification value recorded with every issued Question Attempt.
    pub fn source_object_checksum(&self) -> &SourceObjectChecksum {
        self.source.source_object_checksum()
    }

    fn private(&self) -> &PleQuestionJsonPrivateGrading {
        &self.private
    }
}

impl PleQuestionBackend {
    /// Issues one Question directly from its verified immutable PLE Question JSON source.
    pub fn issue_question_json(
        &self,
        source: &ResolvedPleQuestionJsonSource,
        seed: QuestionSeed,
        question_asset_object_references: &[QuestionAssetObjectReference],
    ) -> Result<PleIssuedQuestion, PleQuestionBackendError> {
        self.issue(
            source.question(),
            seed,
            source.source_object_reference(),
            source.source_object_checksum(),
            question_asset_object_references,
        )
    }

    /// Reproduces and grades with the private key compiled from verified source bytes.
    pub fn grade_question_json(
        &self,
        source: &ResolvedPleQuestionJsonSource,
        seed: QuestionSeed,
        recorded_reproduction_details: &QuestionAttemptReproductionDetails,
        question_asset_object_references: &[QuestionAssetObjectReference],
        response: &StudentResponse,
    ) -> Result<QuestionGradingOutcome, PleQuestionBackendError> {
        self.reproduce(
            source.question(),
            seed,
            recorded_reproduction_details,
            question_asset_object_references,
        )?;
        Ok(source
            .private()
            .evaluate(source.question(), response)
            .map_err(PleQuestionBackendError::QuestionSourceDocument)?
            .outcome)
    }
}
