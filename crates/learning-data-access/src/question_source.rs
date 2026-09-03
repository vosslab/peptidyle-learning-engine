//! Authorized persistence for private Question Source Registrations.
//!
//! Question Source bytes are registered first through the Object Record Store.
//! This boundary then registers that immutable byte evidence to one existing
//! Draft Question at one expected Edit Number. It has no browser serialization path and never
//! accepts inline source data.

use std::num::NonZeroU64;

use async_trait::async_trait;
use question_model::{
    DraftImathasQuestionBackendBinding, QuestionBackend, QuestionFormat, SourceObjectChecksum,
    SourceObjectReference, WorkspaceId, WorkspaceImportId,
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

/// Complete server-validated input for one Draft Question Source Registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestionSourceRegistrationInput {
    /// Existing private Draft Question that owns the source.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Exact saved Draft Question state required for this registration.
    pub expected_draft_question_edit_number: DraftQuestionEditNumber,
    /// Workspace authorizing that private authored state.
    pub workspace: WorkspaceId,
    /// Exact interpreter for the source bytes.
    pub question_backend: QuestionBackend,
    /// Exact source representation.
    pub question_format: QuestionFormat,
    /// WeBWorK PG Path for a WeBWorK Question Backend only.
    pub webwork_pg_path: Option<String>,
    /// QTI package item identifier for a QTI Question Backend only.
    pub qti_package_item_identifier: Option<String>,
    /// Workspace Import ID for a draft QTI Question Source only.
    pub workspace_import_id: Option<WorkspaceImportId>,
    /// iMathAS Deployment and Item References for an iMathAS Question Backend only.
    pub draft_imathas_question_backend_binding: Option<DraftImathasQuestionBackendBinding>,
    /// Immutable Object Record identifying the Question Source bytes.
    pub source_object_reference: SourceObjectReference,
    /// SHA-256 verification value for those exact bytes.
    pub source_object_checksum: SourceObjectChecksum,
}

impl DraftQuestionSourceRegistrationInput {
    /// Refuses incoherent backend and source-format combinations before a transaction starts.
    pub fn validate(&self) -> Result<(), StoreError> {
        let fields_match_backend = match self.question_backend {
            QuestionBackend::Ple => {
                self.webwork_pg_path.is_none()
                    && self.qti_package_item_identifier.is_none()
                    && self.workspace_import_id.is_none()
                    && self.draft_imathas_question_backend_binding.is_none()
            }
            QuestionBackend::Webwork => {
                self.webwork_pg_path.is_some()
                    && self.qti_package_item_identifier.is_none()
                    && self.workspace_import_id.is_none()
                    && self.draft_imathas_question_backend_binding.is_none()
            }
            QuestionBackend::Qti => {
                self.webwork_pg_path.is_none()
                    && self.qti_package_item_identifier.is_some()
                    && self.workspace_import_id.is_some()
                    && self.draft_imathas_question_backend_binding.is_none()
            }
            QuestionBackend::Imathas => {
                self.webwork_pg_path.is_none()
                    && self.qti_package_item_identifier.is_none()
                    && self.workspace_import_id.is_none()
                    && self.draft_imathas_question_backend_binding.is_some()
            }
        };
        if !fields_match_backend {
            return Err(StoreError::InvalidRecord(
                "Question Source Registration must use exactly the fields for its Question Backend"
                    .to_string(),
            ));
        }
        let format_matches_backend = matches!(
            (self.question_backend, self.question_format),
            (QuestionBackend::Ple, QuestionFormat::PleQuestionJson)
                | (QuestionBackend::Webwork, QuestionFormat::WebworkPg)
                | (QuestionBackend::Qti, QuestionFormat::Qti)
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

/// Session-authorized persistence for Draft Question Source Registrations.
#[async_trait]
pub trait DraftQuestionSourceRegistrationStore: Send + Sync {
    /// Registers immutable source-byte evidence for a Draft Question at its expected Edit Number.
    async fn register_draft_question_source_registration(
        &self,
        session_token_hash: SessionTokenHash,
        input: DraftQuestionSourceRegistrationInput,
    ) -> Result<(), StoreError>;
}

#[cfg(test)]
mod tests {
    use question_model::{ObjectId, SourceObjectChecksum};

    use super::*;

    fn input() -> DraftQuestionSourceRegistrationInput {
        DraftQuestionSourceRegistrationInput {
            draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::from_u128(1)),
            expected_draft_question_edit_number: DraftQuestionEditNumber::new(1)
                .expect("positive PostgreSQL bigint"),
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(2)),
            question_backend: QuestionBackend::Ple,
            question_format: QuestionFormat::PleQuestionJson,
            webwork_pg_path: None,
            qti_package_item_identifier: None,
            workspace_import_id: None,
            draft_imathas_question_backend_binding: None,
            source_object_reference: SourceObjectReference {
                object: ObjectId::from_uuid(Uuid::from_u128(3)),
            },
            source_object_checksum: SourceObjectChecksum::parse("a".repeat(64))
                .expect("canonical source checksum"),
        }
    }

    #[test]
    fn draft_question_source_registration_requires_backend_and_format_coherence() {
        assert_eq!(input().validate(), Ok(()));

        let mut wrong_backend = input();
        wrong_backend.question_backend = QuestionBackend::Qti;
        assert!(matches!(
            wrong_backend.validate(),
            Err(StoreError::InvalidRecord(_))
        ));

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
}
