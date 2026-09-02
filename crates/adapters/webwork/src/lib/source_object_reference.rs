//! Trusted immutable PG-source resolution and binding checks.

use objects::{ObjectStore, QuestionSourceResolutionError, ResolvedQuestionSource};
use question_model::{
    QuestionBackend, QuestionRevision, QuestionRevisionReference, SourceObjectChecksum,
    SourceObjectReference,
};

use super::WebworkAdapterError;

/// Immutable PG source resolved from trusted object storage before adapter use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedWebworkQuestionSource {
    resolved: ResolvedQuestionSource,
}

impl ResolvedWebworkQuestionSource {
    /// Resolves PG source only from its exact immutable published Object Address.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        question_revision: QuestionRevisionReference,
        source_object_reference: SourceObjectReference,
        source_object_checksum: SourceObjectChecksum,
    ) -> Result<Self, WebworkAdapterError> {
        let resolved = ResolvedQuestionSource::resolve(
            store,
            question_revision,
            source_object_reference,
            source_object_checksum,
        )
        .await
        .map_err(|error| match error {
            QuestionSourceResolutionError::ObjectStore(error) => {
                WebworkAdapterError::ObjectStore(error)
            }
            QuestionSourceResolutionError::UntrustedObjectRecord => {
                WebworkAdapterError::UntrustedSource
            }
        })?;
        Ok(Self { resolved })
    }

    /// Immutable Source Object Reference carried into Question Attempt Reproduction Details.
    pub fn source_object_reference(&self) -> &SourceObjectReference {
        self.resolved.source_object_reference()
    }

    /// SHA-256 evidence for the immutable source object bytes.
    pub fn source_object_checksum(&self) -> &SourceObjectChecksum {
        self.resolved.source_object_checksum()
    }

    /// Verified immutable PG source bytes for the renderer.
    pub(super) fn pg_source(&self) -> &[u8] {
        self.resolved.bytes()
    }
}

pub(super) fn webwork_identity(
    question: &QuestionRevision,
) -> Result<(QuestionRevisionReference, &str), WebworkAdapterError> {
    match (
        question.question_backend,
        question.webwork_pg_path.as_deref(),
    ) {
        (QuestionBackend::Webwork, Some(pg_path)) => Ok((
            QuestionRevisionReference {
                question_id: question.question_id.clone(),
                revision_number: question.revision_number,
            },
            pg_path,
        )),
        _ => Err(WebworkAdapterError::UnsupportedSource),
    }
}

pub(super) fn verify_source(
    source: &ResolvedWebworkQuestionSource,
) -> Result<(), WebworkAdapterError> {
    let actual = objects::Sha256Checksum::compute(source.pg_source()).to_string();
    if actual == source.source_object_checksum().as_str() {
        Ok(())
    } else {
        Err(WebworkAdapterError::SourceChecksumMismatch)
    }
}

pub(super) fn verify_source_binding(
    source: &ResolvedWebworkQuestionSource,
    question_revision: &QuestionRevisionReference,
) -> Result<(), WebworkAdapterError> {
    if source.resolved.question_revision() == question_revision {
        Ok(())
    } else {
        Err(WebworkAdapterError::SourceDoesNotMatchQuestion)
    }
}
