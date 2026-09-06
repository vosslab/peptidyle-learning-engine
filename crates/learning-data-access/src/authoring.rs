//! Private Authoring Workspace persistence contracts.
//!
//! Browser routes carry only a Draft Question Reference and an Edit Number.
//! This module deliberately keeps workspace and draft UUIDs on the trusted
//! server side while the Store rechecks the active session relationship.

use async_trait::async_trait;
use objects::ObjectRecord;
use question_model::{DraftQuestionReference, WorkspaceId};
use uuid::Uuid;

use crate::{DraftQuestionEditNumber, DraftQuestionUuid, SessionTokenHash, StoreError};

/// Server-only resolved state for one private Draft Question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoringDraft {
    /// Trusted persistence identity; never serialize this to a browser.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Trusted private workspace identity; never serialize this to a browser.
    pub workspace: WorkspaceId,
    /// Authorized browser navigation locator.
    pub reference: DraftQuestionReference,
    /// Positive save concurrency token.
    pub edit_number: DraftQuestionEditNumber,
    /// Private Instructor-facing discovery title.
    pub title: String,
    /// Private Instructor-facing discovery description.
    pub description: String,
    /// Exact current private source object evidence.
    pub source_record: ObjectRecord,
}

/// Answer-free list entry for the current Instructor's My Question Drafts View.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoringDraftSummary {
    pub reference: DraftQuestionReference,
    pub edit_number: DraftQuestionEditNumber,
    pub title: String,
    pub description: String,
}

/// Complete server-validated first save for a newly created Draft Question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAuthoringDraftInput {
    /// Fresh server-minted opaque persistence identity.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Exact source Object Record written before persistence registration.
    pub source_record: ObjectRecord,
    /// Source-derived Question Title.
    pub title: String,
    /// Source-derived Question Description.
    pub description: String,
}

/// Complete server-validated replacement for a saved Draft Question Source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveAuthoringDraftInput {
    /// Draft Question selected from the authorized opaque reference.
    pub reference: DraftQuestionReference,
    /// Current browser concurrency token.
    pub expected_edit_number: DraftQuestionEditNumber,
    /// Exact source Object Record written before persistence registration.
    pub source_record: ObjectRecord,
    /// Source-derived Question Title.
    pub title: String,
    /// Source-derived Question Description.
    pub description: String,
}

/// Session-authorized private Authoring Workspace and Draft Question Store.
#[async_trait]
pub trait AuthoringDraftStore: Send + Sync {
    /// Returns the caller's active Authoring Workspace, creating its private
    /// owner workspace once when the current Instructor has none.
    async fn ensure_own_authoring_workspace(
        &self,
        session_token_hash: SessionTokenHash,
        proposed_workspace_id: Uuid,
    ) -> Result<WorkspaceId, StoreError>;

    /// Lists only Draft Questions reachable through the current workspace relationship.
    async fn list_authoring_drafts(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<AuthoringDraftSummary>, StoreError>;

    /// Registers a bytes-first private source as a new Draft Question.
    async fn create_authoring_draft(
        &self,
        session_token_hash: SessionTokenHash,
        workspace: WorkspaceId,
        input: CreateAuthoringDraftInput,
    ) -> Result<AuthoringDraft, StoreError>;

    /// Resolves one exact authorized Draft Question and its private source evidence.
    async fn load_authoring_draft(
        &self,
        session_token_hash: SessionTokenHash,
        reference: DraftQuestionReference,
    ) -> Result<AuthoringDraft, StoreError>;

    /// Replaces one mutable Draft Question Source using its exact Edit Number.
    async fn save_authoring_draft(
        &self,
        session_token_hash: SessionTokenHash,
        input: SaveAuthoringDraftInput,
    ) -> Result<AuthoringDraft, StoreError>;
}
