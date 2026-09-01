//! Trusted immutable PG-source resolution and binding checks.

use objects::{ObjectAddress, ObjectStore};
use question_model::{
    QuestionBackendLocator, QuestionRevision, QuestionRevisionReference, SourceObjectChecksum,
    SourceObjectReference,
};

use super::WebworkAdapterError;

/// Immutable PG source resolved from trusted object storage before adapter use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebworkSource {
    pub(super) question_revision: QuestionRevisionReference,
    pub(super) source_object_reference: SourceObjectReference,
    pub(super) source_object_checksum: SourceObjectChecksum,
    pub(super) pg_source: Vec<u8>,
}

impl WebworkSource {
    /// Resolves PG source only from its exact immutable published Object Address.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        question_revision: QuestionRevisionReference,
        source_object_reference: SourceObjectReference,
        source_object_checksum: SourceObjectChecksum,
    ) -> Result<Self, WebworkAdapterError> {
        let expected_key = ObjectAddress::QuestionSource {
            question_revision: question_revision.clone(),
            object: source_object_reference.object,
        };
        let stored = store
            .get(&expected_key)
            .await
            .map_err(WebworkAdapterError::ObjectStore)?;
        if stored.record.address != expected_key
            || stored.record.id != source_object_reference.object
            || stored.record.question_revision != Some(question_revision.clone())
            || stored.record.sha256.to_string() != source_object_checksum.as_str()
        {
            return Err(WebworkAdapterError::UntrustedSource);
        }
        Ok(Self {
            question_revision,
            source_object_reference,
            source_object_checksum,
            pg_source: stored.bytes,
        })
    }

    /// Immutable Source Object Reference carried into Question Attempt Reproduction Details.
    pub fn source_object_reference(&self) -> &SourceObjectReference {
        &self.source_object_reference
    }

    /// SHA-256 evidence for the immutable source object bytes.
    pub fn source_object_checksum(&self) -> &SourceObjectChecksum {
        &self.source_object_checksum
    }
}

pub(super) fn webwork_identity(
    question: &QuestionRevision,
) -> Result<(QuestionRevisionReference, &str), WebworkAdapterError> {
    match &question.backend_locator {
        QuestionBackendLocator::Webwork { pg_path } => Ok((
            QuestionRevisionReference {
                question_id: question.question_id.clone(),
                revision_number: question.revision_number,
            },
            pg_path,
        )),
        _ => Err(WebworkAdapterError::UnsupportedSource),
    }
}

pub(super) fn verify_source(source: &WebworkSource) -> Result<(), WebworkAdapterError> {
    let actual = objects::Sha256Digest::compute(&source.pg_source).to_string();
    if actual == source.source_object_checksum.as_str() {
        Ok(())
    } else {
        Err(WebworkAdapterError::SourceChecksumMismatch)
    }
}

pub(super) fn verify_source_binding(
    source: &WebworkSource,
    question_revision: &QuestionRevisionReference,
) -> Result<(), WebworkAdapterError> {
    if &source.question_revision == question_revision {
        Ok(())
    } else {
        Err(WebworkAdapterError::SourceDoesNotMatchQuestion)
    }
}
