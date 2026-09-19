//! Trusted resolution of immutable Question Source bytes.

use question_model::{ObjectId, QuestionRevisionTuple, SourceObjectChecksum};

use crate::{ObjectAddress, ObjectStore, ObjectStoreError};

/// Immutable Question Source bytes resolved from the exact typed Object Address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedQuestionSource {
    question_revision_tuple: QuestionRevisionTuple,
    source_object_id: ObjectId,
    source_object_checksum: SourceObjectChecksum,
    media_type: String,
    bytes: Vec<u8>,
}

impl ResolvedQuestionSource {
    /// Reads one trusted immutable Question Source and verifies every stored fact.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        question_revision_tuple: QuestionRevisionTuple,
        source_object_id: ObjectId,
        source_object_checksum: SourceObjectChecksum,
    ) -> Result<Self, QuestionSourceResolutionError> {
        let expected_address = ObjectAddress::QuestionSource {
            question_revision_tuple: question_revision_tuple.clone(),
            object: source_object_id,
        };
        let stored = store
            .get(&expected_address)
            .await
            .map_err(QuestionSourceResolutionError::ObjectStore)?;
        if stored.record.address != expected_address
            || stored.record.id != source_object_id
            || stored.record.question_revision_tuple != Some(question_revision_tuple.clone())
            || stored.record.sha256.to_string() != source_object_checksum.as_str()
        {
            return Err(QuestionSourceResolutionError::UntrustedObjectRecord);
        }
        Ok(Self {
            question_revision_tuple,
            source_object_id,
            source_object_checksum,
            media_type: stored.record.media_type,
            bytes: stored.bytes,
        })
    }

    /// Exact Question Revision that owns these source bytes.
    pub fn question_revision_tuple(&self) -> &QuestionRevisionTuple {
        &self.question_revision_tuple
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

#[cfg(test)]
mod tests {
    use crate::{PutObject, Timestamp, memory::MemoryObjectStore};
    use question_model::{ObjectId, QuestionId, QuestionRevisionNumber, QuestionRevisionTuple};
    use uuid::Uuid;

    use super::*;

    fn question_revision_tuple() -> QuestionRevisionTuple {
        QuestionRevisionTuple {
            question_id: QuestionId::from_random_identifier("ABCDEFG").expect("valid Question ID"),
            revision_number: QuestionRevisionNumber::new(1).expect("positive revision"),
        }
    }

    #[tokio::test]
    async fn resolve_returns_only_the_exact_immutable_question_source() {
        let store = MemoryObjectStore::default();
        let question_revision_tuple = question_revision_tuple();
        let source_object_id = ObjectId::from_uuid(Uuid::from_u128(9));
        let bytes = br#"{\"format\":\"pleQuestionJson\"}"#.to_vec();
        let record = store
            .put(PutObject {
                address: ObjectAddress::QuestionSource {
                    question_revision_tuple: question_revision_tuple.clone(),
                    object: source_object_id,
                },
                bytes: bytes.clone(),
                media_type: "application/vnd.peptidyle.question+json".to_string(),
                created_at: Timestamp::from_unix_millis(1),
            })
            .await
            .expect("source should store");
        let source_object_checksum =
            SourceObjectChecksum::parse(record.sha256.to_string()).expect("canonical checksum");

        let resolved = ResolvedQuestionSource::resolve(
            &store,
            question_revision_tuple.clone(),
            source_object_id.clone(),
            source_object_checksum.clone(),
        )
        .await
        .expect("matching source should resolve");

        assert_eq!(resolved.question_revision_tuple(), &question_revision_tuple);
        assert_eq!(resolved.source_object_id(), &source_object_id);
        assert_eq!(resolved.source_object_checksum(), &source_object_checksum);
        assert_eq!(
            resolved.media_type(),
            "application/vnd.peptidyle.question+json"
        );
        assert_eq!(resolved.bytes(), bytes);
    }

    #[tokio::test]
    async fn resolve_refuses_a_mismatched_source_checksum() {
        let store = MemoryObjectStore::default();
        let question_revision_tuple = question_revision_tuple();
        let source_object_id = ObjectId::from_uuid(Uuid::from_u128(10));
        store
            .put(PutObject {
                address: ObjectAddress::QuestionSource {
                    question_revision_tuple: question_revision_tuple.clone(),
                    object: source_object_id,
                },
                bytes: b"trusted bytes".to_vec(),
                media_type: "application/json".to_string(),
                created_at: Timestamp::from_unix_millis(1),
            })
            .await
            .expect("source should store");

        assert_eq!(
            ResolvedQuestionSource::resolve(
                &store,
                question_revision_tuple,
                source_object_id,
                SourceObjectChecksum::parse("a".repeat(64)).expect("canonical checksum"),
            )
            .await,
            Err(QuestionSourceResolutionError::UntrustedObjectRecord)
        );
    }
}
