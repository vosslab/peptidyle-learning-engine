//! Server-derived Published Question fork persistence.
//!
//! The browser names only an already-public Question Revision in the request
//! path. This contract keeps the source pin, future Question ID, private
//! workspace, Draft identity, and idempotency receipt inside trusted code.

use async_trait::async_trait;
use question_model::{DraftQuestionReference, QuestionId, QuestionRevisionReference, WorkspaceId};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// Inputs the trusted server has resolved or minted for one fork attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkPublishedQuestionInput {
    /// Existing available immutable source Revision, resolved from the path.
    pub source_question_revision: QuestionRevisionReference,
    /// Fresh HMAC-validated identity reserved for this fork's later publication.
    pub forked_question_id: QuestionId,
    /// Opaque server persistence identities; neither crosses the browser boundary.
    pub proposed_workspace_id: Uuid,
    pub proposed_draft_question_id: Uuid,
    /// Opaque caller retry key, scoped by PostgreSQL to the authenticated actor.
    pub idempotency_key: Uuid,
}

/// Browser-safe result for an authorized private fork.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkedPublishedQuestionDraft {
    /// Private Draft navigation reference for the authenticated Instructor only.
    pub draft_question: DraftQuestionReference,
    /// Server-only workspace that owns the Draft.
    pub workspace: WorkspaceId,
    /// Server-minted future Published Question identity.
    pub forked_question_id: QuestionId,
}

/// Only a collision of the reserved fork identity is conclusive and retryable.
#[derive(Debug, Clone, PartialEq)]
pub enum ForkPublishedQuestionError {
    /// A freshly HMAC-issued future Question ID was already reserved by a fork.
    IdentityCollision,
    /// Every other persistence result is not safe for the coordinator to retry.
    Store(StoreError),
}

/// Session-authorized source resolution and atomic Draft fork capability.
#[async_trait]
pub trait QuestionForkStore: Send + Sync {
    /// Resolves one current available Published Question Revision for a fork.
    ///
    /// PostgreSQL derives Instructor authority from the installed session and
    /// returns no source bytes or author identity.
    async fn load_available_question_fork_source(
        &self,
        session_token_hash: SessionTokenHash,
        question_revision: &QuestionRevisionReference,
    ) -> Result<QuestionRevisionReference, StoreError>;

    /// Creates or returns the actor/key's one private Draft fork atomically.
    async fn fork_published_question_to_draft(
        &self,
        session_token_hash: SessionTokenHash,
        input: ForkPublishedQuestionInput,
    ) -> Result<ForkedPublishedQuestionDraft, ForkPublishedQuestionError>;
}
