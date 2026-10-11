//! Trusted resolution of immutable Question Source bytes.

use question_model::{ObjectId, PublishedQuestionRevisionTuple, SourceObjectChecksum};

use crate::{ObjectAddress, ObjectStore, ObjectStoreError};

/// Immutable Question Source bytes resolved from the exact typed Object Address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedQuestionSource {
    published_question_revision_tuple: PublishedQuestionRevisionTuple,
    source_object_id: ObjectId,
    source_object_checksum: SourceObjectChecksum,
    media_type: String,
    bytes: Vec<u8>,
}

impl ResolvedQuestionSource {
    /// Reads one trusted immutable Question Source and verifies every stored fact.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        published_question_revision_tuple: PublishedQuestionRevisionTuple,
        source_object_id: ObjectId,
        source_object_checksum: SourceObjectChecksum,
    ) -> Result<Self, QuestionSourceResolutionError> {
        let expected_address = ObjectAddress::QuestionSource {
            published_question_revision_tuple: published_question_revision_tuple.clone(),
            object_id: source_object_id,
        };
        let stored = store
            .get(&expected_address)
            .await
            .map_err(QuestionSourceResolutionError::ObjectStore)?;
        if stored.record.address != expected_address
            || stored.record.id != source_object_id
            || stored.record.published_question_revision_tuple
                != Some(published_question_revision_tuple.clone())
            || stored.record.sha256.to_string() != source_object_checksum.as_str()
        {
            return Err(QuestionSourceResolutionError::UntrustedObjectRecord);
        }
        Ok(Self {
            published_question_revision_tuple,
            source_object_id,
            source_object_checksum,
            media_type: stored.record.media_type,
            bytes: stored.bytes,
        })
    }

    /// Exact Question Revision that owns these source bytes.
    pub fn published_question_revision_tuple(&self) -> &PublishedQuestionRevisionTuple {
        &self.published_question_revision_tuple
    }

    /// Immutable Object Record that identifies these source bytes.
    pub fn source_object_id(&self) -> &ObjectId {
        &self.source_object_id
    }

    /// SHA-256 evidence that verifies these exact source bytes.
    pub fn source_object_checksum(&self) -> &SourceObjectChecksum {
        &self.source_object_checksum
    }

    /// Server-verified media type recorded with the immutable source bytes.
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    /// Verified immutable source bytes for backend-specific parsing.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Failure while binding immutable Question Source bytes to a Question Revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionSourceResolutionError {
    /// Object storage could not read the typed immutable address.
    ObjectStore(ObjectStoreError),
    /// Returned metadata did not establish the requested immutable source binding.
    UntrustedObjectRecord,
}

impl std::fmt::Display for QuestionSourceResolutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ObjectStore(error) => error.fmt(formatter),
            Self::UntrustedObjectRecord => formatter
                .write_str("Question Source object record does not match its immutable binding"),
        }
    }
}

impl std::error::Error for QuestionSourceResolutionError {}
