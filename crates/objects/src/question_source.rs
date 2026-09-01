//! Trusted resolution of immutable Question Source bytes.

use question_model::{QuestionRevisionReference, SourceObjectChecksum, SourceObjectReference};

use crate::{ObjectAddress, ObjectStore, ObjectStoreError};

/// Immutable Question Source bytes resolved from the exact typed Object Address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedQuestionSource {
    question_revision: QuestionRevisionReference,
    source_object_reference: SourceObjectReference,
    source_object_checksum: SourceObjectChecksum,
    media_type: String,
    bytes: Vec<u8>,
}

impl ResolvedQuestionSource {
    /// Reads one trusted immutable Question Source and verifies every stored fact.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        question_revision: QuestionRevisionReference,
        source_object_reference: SourceObjectReference,
        source_object_checksum: SourceObjectChecksum,
    ) -> Result<Self, QuestionSourceResolutionError> {
        let expected_address = ObjectAddress::QuestionSource {
            question_revision: question_revision.clone(),
            object: source_object_reference.object,
        };
        let stored = store
            .get(&expected_address)
            .await
            .map_err(QuestionSourceResolutionError::ObjectStore)?;
        if stored.record.address != expected_address
            || stored.record.id != source_object_reference.object
            || stored.record.question_revision != Some(question_revision.clone())
            || stored.record.sha256.to_string() != source_object_checksum.as_str()
        {
            return Err(QuestionSourceResolutionError::UntrustedObjectRecord);
        }
        Ok(Self {
            question_revision,
            source_object_reference,
            source_object_checksum,
            media_type: stored.record.media_type,
            bytes: stored.bytes,
        })
    }

    /// Exact Question Revision that owns these source bytes.
    pub fn question_revision(&self) -> &QuestionRevisionReference {
        &self.question_revision
    }

    /// Immutable Object Record that identifies these source bytes.
    pub fn source_object_reference(&self) -> &SourceObjectReference {
        &self.source_object_reference
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
    use question_model::{ObjectId, QuestionId, QuestionRevisionNumber, QuestionRevisionReference};
    use uuid::Uuid;

    use super::*;

    fn question_revision() -> QuestionRevisionReference {
        QuestionRevisionReference {
            question_id: QuestionId::from_canonical_parts("ABCDEF", 'G')
                .expect("valid Question ID"),
            revision_number: QuestionRevisionNumber::new(1).expect("positive revision"),
        }
    }

    #[tokio::test]
    async fn resolve_returns_only_the_exact_immutable_question_source() {
        let store = MemoryObjectStore::default();
        let question_revision = question_revision();
        let source_object_reference = SourceObjectReference {
            object: ObjectId::from_uuid(Uuid::from_u128(9)),
        };
        let bytes = br#"{\"format\":\"pleQuestionJson\"}"#.to_vec();
        let record = store
            .put(PutObject {
                address: ObjectAddress::QuestionSource {
                    question_revision: question_revision.clone(),
                    object: source_object_reference.object,
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
            question_revision.clone(),
            source_object_reference.clone(),
            source_object_checksum.clone(),
        )
        .await
        .expect("matching source should resolve");

        assert_eq!(resolved.question_revision(), &question_revision);
        assert_eq!(resolved.source_object_reference(), &source_object_reference);
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
        let question_revision = question_revision();
        let source_object_reference = SourceObjectReference {
            object: ObjectId::from_uuid(Uuid::from_u128(10)),
        };
        store
            .put(PutObject {
                address: ObjectAddress::QuestionSource {
                    question_revision: question_revision.clone(),
                    object: source_object_reference.object,
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
                question_revision,
                source_object_reference,
                SourceObjectChecksum::parse("a".repeat(64)).expect("canonical checksum"),
            )
            .await,
            Err(QuestionSourceResolutionError::UntrustedObjectRecord)
        );
    }
}
