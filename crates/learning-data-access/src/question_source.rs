//! Authorized persistence for private Draft Question Source Bindings.
//!
//! Question Source bytes are recorded first through the Object Record Store.
//! This boundary then binds that immutable byte evidence to one existing
//! Draft Question at one expected Edit Number. It has no browser serialization path and never
//! accepts inline source data.

use std::num::NonZeroU64;

use async_trait::async_trait;
use objects::{ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea};
use question_model::{
    DraftImathasQuestionBackendBinding, QuestionAuthorship, QuestionBackend, QuestionFormat,
    QuestionId, QuestionLicense, QuestionRevisionNumber, QuestionRevisionReason,
    QuestionRevisionReference, SourceObjectChecksum, SourceObjectReference, WorkspaceId,
};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// Server-only UUID identity for one private mutable Draft Question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DraftQuestionUuid(Uuid);

impl DraftQuestionUuid {
    /// Wraps a Draft Question UUID read from private persistence.
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the private persistence UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Positive optimistic-concurrency token for one mutable Draft Question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DraftQuestionEditNumber(NonZeroU64);

impl DraftQuestionEditNumber {
    /// Creates one positive Draft Question Edit Number representable by PostgreSQL `BIGINT`.
    pub fn new(value: u64) -> Result<Self, StoreError> {
        if value == 0 || value > i64::MAX as u64 {
            return Err(StoreError::InvalidRecord(
                "Draft Question Edit Number must fit a positive PostgreSQL bigint".to_string(),
            ));
        }
        Ok(Self(
            NonZeroU64::new(value).expect("positive value checked before construction"),
        ))
    }

    /// Returns the exact positive value for the PostgreSQL `BIGINT` parameter.
    pub const fn as_postgres_bigint(self) -> i64 {
        self.0.get() as i64
    }
}

/// Complete server-validated input for one Draft Question Source Binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestionSourceBindingInput {
    /// Existing private Draft Question that owns the source.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Exact saved Draft Question state required for this binding.
    pub expected_draft_question_edit_number: DraftQuestionEditNumber,
    /// Workspace authorizing that private authored state.
    pub workspace: WorkspaceId,
    /// Exact interpreter for the source bytes.
    pub question_backend: QuestionBackend,
    /// Exact source representation.
    pub question_format: QuestionFormat,
    /// WeBWorK PG Path for a WeBWorK Question Backend only.
    pub webwork_pg_path: Option<String>,
    /// iMathAS Deployment and Item References for an iMathAS Question Backend only.
    pub draft_imathas_question_backend_binding: Option<DraftImathasQuestionBackendBinding>,
    /// Immutable Object Record identifying the Question Source bytes.
    pub source_object_reference: SourceObjectReference,
    /// SHA-256 verification value for those exact bytes.
    pub source_object_checksum: SourceObjectChecksum,
}

impl DraftQuestionSourceBindingInput {
    /// Refuses incoherent backend and source-format combinations before a transaction starts.
    pub fn validate(&self) -> Result<(), StoreError> {
        let fields_match_backend = match self.question_backend {
            QuestionBackend::Ple => {
                self.webwork_pg_path.is_none()
                    && self.draft_imathas_question_backend_binding.is_none()
            }
            QuestionBackend::Webwork => {
                self.webwork_pg_path.is_some()
                    && self.draft_imathas_question_backend_binding.is_none()
            }
            QuestionBackend::Imathas => {
                self.webwork_pg_path.is_none()
                    && self.draft_imathas_question_backend_binding.is_some()
            }
        };
        if !fields_match_backend {
            return Err(StoreError::InvalidRecord(
                "Draft Question Source Binding must use exactly the fields for its Question Backend"
                    .to_string(),
            ));
        }
        let format_matches_backend = matches!(
            (self.question_backend, self.question_format),
            (QuestionBackend::Ple, QuestionFormat::PleQuestionJson)
                | (QuestionBackend::Webwork, QuestionFormat::WebworkPg)
                | (QuestionBackend::Imathas, QuestionFormat::Imathas)
        );
        if !format_matches_backend {
            return Err(StoreError::InvalidRecord(
                "Question Format must be supported by its Question Backend".to_string(),
            ));
        }
        Ok(())
    }
}

/// Session-authorized persistence for Draft Question Source Bindings.
#[async_trait]
pub trait DraftQuestionSourceBindingStore: Send + Sync {
    /// Binds immutable source-byte evidence to a Draft Question at its expected Edit Number.
    async fn bind_draft_question_source(
        &self,
        session_token_hash: SessionTokenHash,
        input: DraftQuestionSourceBindingInput,
    ) -> Result<(), StoreError>;
}

/// Session-authorized resolution of the exact Draft Question Source selected
/// for a publication attempt.
#[async_trait]
pub trait DraftQuestionPublicationSourceStore: Send + Sync {
    /// Loads the current immutable Workspace Question Source Object Record only
    /// when the Draft Question, Edit Number, and workspace remain exact.
    async fn load_draft_question_publication_source(
        &self,
        session_token_hash: SessionTokenHash,
        draft_question_uuid: DraftQuestionUuid,
        expected_draft_question_edit_number: DraftQuestionEditNumber,
        workspace: WorkspaceId,
    ) -> Result<ObjectRecord, StoreError>;
}

/// Server-only inputs for publishing an exact Draft Question as a new lineage.
///
/// The `question_source_object_record` comes from a completed bytes-first copy
/// to its immutable Question Revision Object Address. It is never accepted
/// from a browser payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewQuestionLineagePublicationInput {
    /// Current private Draft Question selected for publication.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Exact saved Draft Question state validated by the server.
    pub expected_draft_question_edit_number: DraftQuestionEditNumber,
    /// Authoring Workspace that owns the Draft Question.
    pub workspace: WorkspaceId,
    /// Fresh server-minted Published Question lineage identity.
    pub question_id: QuestionId,
    /// Verified immutable target object created by the bytes-first copy.
    pub question_source_object_record: ObjectRecord,
    /// Reviewed ordered Question Authorship snapshot.
    pub question_authorship: QuestionAuthorship,
    /// Compatible Question License for the immutable first revision.
    pub question_license: QuestionLicense,
    /// Reviewed Question Revision Reason recorded with first-revision acceptance.
    pub question_revision_reason: QuestionRevisionReason,
    /// Fresh immutable Question Ownership Event identity.
    pub question_ownership_event_id: Uuid,
    /// Fresh immutable Question Publication Event identity.
    pub question_publication_event_id: Uuid,
    /// Fresh initial Available-event identity.
    pub question_availability_event_id: Uuid,
}

impl NewQuestionLineagePublicationInput {
    /// Exact first Question Revision created by this publication.
    pub fn question_revision(&self) -> QuestionRevisionReference {
        QuestionRevisionReference {
            question_id: self.question_id.clone(),
            revision_number: QuestionRevisionNumber::new(1)
                .expect("first Question Revision Number is positive"),
        }
    }

    /// Refuses target object or acceptance facts that do not match this publication.
    pub fn validate(&self) -> Result<(), StoreError> {
        let expected_revision = self.question_revision();
        let ObjectAddress::QuestionSource {
            question_revision,
            object,
        } = &self.question_source_object_record.address
        else {
            return Err(StoreError::InvalidRecord(
                "Question Publication requires a Question Source Object Address".to_string(),
            ));
        };
        if question_revision != &expected_revision
            || *object != self.question_source_object_record.id
            || self.question_source_object_record.storage_area != ObjectStorageArea::PrivateContent
            || self.question_source_object_record.data_class != ObjectDataClass::QuestionSource
            || self
                .question_source_object_record
                .question_revision
                .as_ref()
                != Some(&expected_revision)
        {
            return Err(StoreError::InvalidRecord(
                "Question Publication Object Record must derive from its exact first Question Revision"
                    .to_string(),
            ));
        }
        Ok(())
    }
}

/// Session-authorized persistence for new-lineage Question Publication.
#[async_trait]
pub trait NewQuestionLineagePublicationStore: Send + Sync {
    /// Atomically records one complete new Published Question aggregate after
    /// the exact source bytes have been copied to immutable object storage.
    async fn publish_new_question_lineage(
        &self,
        session_token_hash: SessionTokenHash,
        input: NewQuestionLineagePublicationInput,
    ) -> Result<QuestionRevisionReference, StoreError>;
}

#[cfg(test)]
mod tests {
    use objects::{ObjectDataClass, ObjectStorageArea, Sha256Checksum};
    use question_model::{
        ObjectId, QuestionAuthor, QuestionAuthorDisplayName, QuestionLicense, Timestamp,
    };

    use super::*;

    fn input() -> DraftQuestionSourceBindingInput {
        DraftQuestionSourceBindingInput {
            draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::from_u128(1)),
            expected_draft_question_edit_number: DraftQuestionEditNumber::new(1)
                .expect("positive PostgreSQL bigint"),
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(2)),
            question_backend: QuestionBackend::Ple,
            question_format: QuestionFormat::PleQuestionJson,
            webwork_pg_path: None,
            draft_imathas_question_backend_binding: None,
            source_object_reference: SourceObjectReference {
                object: ObjectId::from_uuid(Uuid::from_u128(3)),
            },
            source_object_checksum: SourceObjectChecksum::parse("a".repeat(64))
                .expect("canonical source checksum"),
        }
    }

    #[test]
    fn draft_question_source_binding_requires_backend_and_format_coherence() {
        assert_eq!(input().validate(), Ok(()));

        let mut wrong_format = input();
        wrong_format.question_format = QuestionFormat::WebworkPg;
        assert!(matches!(
            wrong_format.validate(),
            Err(StoreError::InvalidRecord(_))
        ));
    }

    #[test]
    fn draft_question_edit_number_requires_a_positive_postgresql_bigint() {
        let maximum = DraftQuestionEditNumber::new(i64::MAX as u64)
            .expect("positive PostgreSQL bigint maximum");
        assert_eq!(maximum.as_postgres_bigint(), i64::MAX);

        assert!(matches!(
            DraftQuestionEditNumber::new(0),
            Err(StoreError::InvalidRecord(_))
        ));
        assert!(matches!(
            DraftQuestionEditNumber::new(i64::MAX as u64 + 1),
            Err(StoreError::InvalidRecord(_))
        ));
    }

    fn publication_input() -> NewQuestionLineagePublicationInput {
        let question_id =
            QuestionId::from_canonical_parts("ABCDEF", 'G').expect("canonical Question ID");
        let question_revision = QuestionRevisionReference {
            question_id: question_id.clone(),
            revision_number: QuestionRevisionNumber::new(1)
                .expect("positive Question Revision Number"),
        };
        let object = ObjectId::from_uuid(Uuid::from_u128(7));
        NewQuestionLineagePublicationInput {
            draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::from_u128(1)),
            expected_draft_question_edit_number: DraftQuestionEditNumber::new(2)
                .expect("positive Draft Question Edit Number"),
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(2)),
            question_id,
            question_source_object_record: ObjectRecord {
                id: object,
                storage_area: ObjectStorageArea::PrivateContent,
                data_class: ObjectDataClass::QuestionSource,
                address: ObjectAddress::QuestionSource {
                    question_revision: question_revision.clone(),
                    object,
                },
                sha256: Sha256Checksum::compute(b"complete Question Source"),
                size_bytes: 24,
                media_type: "application/json".to_string(),
                question_revision: Some(question_revision),
                created_at: Timestamp::from_unix_millis(1_000),
            },
            question_authorship: QuestionAuthorship::new(vec![QuestionAuthor {
                display_name: QuestionAuthorDisplayName::new("Ada Lovelace".to_string())
                    .expect("reviewed Question Author"),
            }])
            .expect("bounded Question Authorship"),
            question_license: QuestionLicense::CcBy4_0,
            question_revision_reason: QuestionRevisionReason::new(
                "Initial reviewed publication".to_string(),
            )
            .expect("reviewed Question Revision Reason"),
            question_ownership_event_id: Uuid::from_u128(8),
            question_publication_event_id: Uuid::from_u128(9),
            question_availability_event_id: Uuid::from_u128(10),
        }
    }

    #[test]
    fn new_lineage_publication_requires_its_exact_revision_owned_object() {
        let input = publication_input();
        assert_eq!(input.validate(), Ok(()));

        let mut wrong_address = input;
        wrong_address.question_source_object_record.address =
            ObjectAddress::WorkspaceQuestionSource {
                workspace: wrong_address.workspace,
                object: wrong_address.question_source_object_record.id,
            };
        assert!(matches!(
            wrong_address.validate(),
            Err(StoreError::InvalidRecord(_))
        ));
    }
}
