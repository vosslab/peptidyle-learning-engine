//! Authorized persistence for immutable private Question Sources.
//!
//! Question Source bytes are registered first through the Object Record Store.
//! This boundary then binds those exact bytes to one existing Draft Question
//! Revision. It has no browser serialization path and never accepts inline
//! source data.

use async_trait::async_trait;
use question_model::{
    AuthoringWorkspaceReference, DraftQuestionBackendLocator, DraftQuestionContent,
    DraftQuestionReference, DraftQuestionSummary, QuestionBackend, QuestionFormat, QuestionType,
    SourceObjectChecksum, SourceObjectReference, WorkspaceId,
};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// Server-only UUID identity for one private Draft Question lineage.
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

/// Positive revision number within one private Draft Question lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DraftQuestionRevisionNumber(u32);

impl DraftQuestionRevisionNumber {
    /// Creates one positive Draft Question Revision Number.
    pub fn new(value: u32) -> Result<Self, StoreError> {
        if value == 0 {
            return Err(StoreError::InvalidRecord(
                "Draft Question Revision Number must be positive".to_string(),
            ));
        }
        Ok(Self(value))
    }

    /// Returns the persisted positive revision number.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Exact private persistence identity for one immutable Draft Question Revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DraftQuestionRevisionReference {
    /// Private UUID for the Draft Question lineage.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Positive revision within that lineage.
    pub revision_number: DraftQuestionRevisionNumber,
}

/// Server-held immutable private Draft Question revision.
#[derive(Debug, Clone, PartialEq)]
pub struct DraftQuestionRevision {
    /// Exact private persistence identity.
    pub reference: DraftQuestionRevisionReference,
    /// Authored content accepted into this revision.
    pub content: DraftQuestionContent,
}

impl DraftQuestionRevision {
    /// Builds the answer-free browser summary with opaque public locators only.
    pub fn summary(
        &self,
        draft_question: DraftQuestionReference,
        authoring_workspace: AuthoringWorkspaceReference,
    ) -> DraftQuestionSummary {
        DraftQuestionSummary {
            draft_question,
            workspace: self.content.workspace,
            authoring_workspace,
            title: self.content.metadata.title.clone(),
            question_backend: QuestionBackend::from(&self.content.backend_locator),
        }
    }
}

/// Server-only identity of one immutable Question Source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QuestionSourceUuid(Uuid);

impl QuestionSourceUuid {
    /// Wraps the private database identity returned by an authorized boundary.
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the storage identity for audit and trusted persistence use.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// SHA-256 binding between a Question Source and its answer-free public shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionPublicBindingChecksum(SourceObjectChecksum);

impl QuestionPublicBindingChecksum {
    /// Parses one canonical SHA-256 checksum.
    pub fn parse(value: impl Into<String>) -> Result<Self, StoreError> {
        SourceObjectChecksum::parse(value).map(Self).map_err(|_| {
            StoreError::InvalidRecord(
                "Question Public Binding Checksum must be canonical lowercase SHA-256".to_string(),
            )
        })
    }

    /// Returns the canonical checksum for a parameterized persistence call.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Complete server-validated input for one immutable Draft Question Source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftQuestionSourceInput {
    /// Existing private authored state that owns the source.
    pub draft_question_revision: DraftQuestionRevisionReference,
    /// Workspace authorizing that private authored state.
    pub workspace: WorkspaceId,
    /// Exact interpreter for the source bytes.
    pub question_backend: QuestionBackend,
    /// Exact source representation.
    pub question_format: QuestionFormat,
    /// Educational interaction supported by the representation.
    pub question_type: QuestionType,
    /// Backend-specific location facts, separate from the source bytes.
    pub backend_locator: DraftQuestionBackendLocator,
    /// Immutable Object Record identifying the Question Source bytes.
    pub source_object_reference: SourceObjectReference,
    /// SHA-256 verification value for those exact bytes.
    pub source_object_checksum: SourceObjectChecksum,
    /// SHA-256 binding between private source facts and public Question content.
    pub public_binding_checksum: QuestionPublicBindingChecksum,
}

impl DraftQuestionSourceInput {
    /// Refuses incoherent backend and source-format combinations before a transaction starts.
    pub fn validate(&self) -> Result<(), StoreError> {
        if QuestionBackend::from(&self.backend_locator) != self.question_backend {
            return Err(StoreError::InvalidRecord(
                "Question Backend must match its exact backend-specific location".to_string(),
            ));
        }
        let format_matches_backend = matches!(
            (self.question_backend, self.question_format),
            (
                QuestionBackend::Ple,
                QuestionFormat::PleQuestionJson | QuestionFormat::PleAlgorithmic
            ) | (QuestionBackend::Webwork, QuestionFormat::WebworkPg)
                | (QuestionBackend::Qti, QuestionFormat::Qti)
                | (QuestionBackend::H5p, QuestionFormat::H5p)
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

/// Session-authorized persistence for immutable Draft Question Sources.
#[async_trait]
pub trait DraftQuestionSourceStore: Send + Sync {
    /// Binds an existing Draft Question Revision to its exact immutable Question Source bytes.
    async fn register_draft_question_source(
        &self,
        session_token_hash: SessionTokenHash,
        input: DraftQuestionSourceInput,
    ) -> Result<QuestionSourceUuid, StoreError>;
}

#[cfg(test)]
mod tests {
    use question_model::{ObjectId, SourceObjectChecksum};

    use super::*;

    fn input() -> DraftQuestionSourceInput {
        DraftQuestionSourceInput {
            draft_question_revision: DraftQuestionRevisionReference {
                draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::from_u128(1)),
                revision_number: DraftQuestionRevisionNumber::new(1)
                    .expect("positive revision number"),
            },
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(2)),
            question_backend: QuestionBackend::Ple,
            question_format: QuestionFormat::PleQuestionJson,
            question_type: QuestionType::MultipleChoice,
            backend_locator: DraftQuestionBackendLocator::Ple,
            source_object_reference: SourceObjectReference {
                object: ObjectId::from_uuid(Uuid::from_u128(3)),
            },
            source_object_checksum: SourceObjectChecksum::parse("a".repeat(64))
                .expect("canonical source checksum"),
            public_binding_checksum: QuestionPublicBindingChecksum::parse("b".repeat(64))
                .expect("canonical public binding checksum"),
        }
    }

    #[test]
    fn draft_question_source_requires_backend_and_format_coherence() {
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
}
