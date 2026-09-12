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
    /// Registered OPL-style PG location for a WeBWorK source. Native PLE
    /// Question JSON carries no WeBWorK routing field.
    pub webwork_pg_path: Option<String>,
    /// Source-derived Question Title.
    pub title: String,
    /// Source-derived Question Description.
    pub description: String,
    /// Source-derived language retained when the Draft becomes a Question Revision.
    pub language: String,
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
    /// Source-derived language retained when the Draft becomes a Question Revision.
    pub language: String,
}

/// Confirms that the initial Draft Question binding can be derived exactly
/// from the source media type. The database repeats this invariant inside the
/// creation transaction before it records the mutable Draft state.
#[cfg(feature = "postgres")]
pub(crate) fn validate_initial_draft_source_binding(
    media_type: &str,
    webwork_pg_path: Option<&str>,
) -> Result<(), StoreError> {
    match (media_type, webwork_pg_path) {
        ("application/vnd.peptidyle.question+json", None) => Ok(()),
        ("text/x-wework-pg", Some(path)) if valid_webwork_pg_path(path) => Ok(()),
        _ => Err(StoreError::InvalidRecord(
            "Draft Question source media type and initial source binding are inconsistent"
                .to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
fn valid_webwork_pg_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1_024
        && !value.starts_with('/')
        && !value.contains(['\\', '\0'])
        && !value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
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

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::*;

    #[test]
    fn initial_draft_source_binding_matches_its_supported_media_type() {
        assert_eq!(
            validate_initial_draft_source_binding("application/vnd.peptidyle.question+json", None,),
            Ok(())
        );
        assert_eq!(
            validate_initial_draft_source_binding("text/x-wework-pg", Some("Library/Algebra.pg")),
            Ok(())
        );
        for (media_type, path) in [
            (
                "application/vnd.peptidyle.question+json",
                Some("Library/Algebra.pg"),
            ),
            ("text/x-wework-pg", None),
            ("text/x-wework-pg", Some("../Algebra.pg")),
            ("application/json", None),
        ] {
            assert!(validate_initial_draft_source_binding(media_type, path).is_err());
        }
    }
}
