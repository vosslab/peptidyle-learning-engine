//! Private Authoring Workspace persistence contracts.
//!
//! Browser routes carry a private Draft UUID and an Edit Number.
//! The Store rechecks the active session relationship for every UUID.

use async_trait::async_trait;
use objects::ObjectRecord;
use question_model::{QuestionAssetTuple, QuestionFormat, QuestionType, WorkspaceId};
use uuid::Uuid;

use crate::{DraftQuestionEditNumber, DraftQuestionUuid, SessionTokenHash, StoreError};

/// Server-only resolved state for one private Draft Question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoringDraft {
    /// Private transport and persistence identity; never display this as a public reference.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Trusted private workspace identity; never serialize this to a browser.
    pub workspace: WorkspaceId,
    /// Positive save concurrency token.
    pub edit_number: DraftQuestionEditNumber,
    /// Private Instructor-facing discovery title.
    pub title: String,
    /// Private Instructor-facing discovery description.
    pub description: String,
    /// Deliberately authored backend-independent feedback for the next
    /// published Question Revision. It is never source-derived.
    pub general_feedback: Option<String>,
    /// Current author-declared educational type that publication makes immutable.
    pub question_type: QuestionType,
    /// Exact current private source object evidence.
    pub source_record: ObjectRecord,
}

/// Answer-free list entry for the current Instructor's My Question Drafts View.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoringDraftSummary {
    pub draft_question_uuid: DraftQuestionUuid,
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
    /// Trusted authoring or import provenance for the exact source representation.
    /// This is never inferred from a filename or source bytes.
    pub question_format: QuestionFormat,
    /// Registered OPL-style PG location for a WeBWorK source. Native PLE
    /// Question JSON carries no WeBWorK routing field.
    pub webwork_pg_path: Option<String>,
    /// Required author-declared educational Question Type.
    pub question_type: QuestionType,
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
    /// Exact native HOTSPOT surface derived from validated source, not browser metadata.
    pub hotspot_surface: Option<QuestionAssetTuple>,
    /// Draft Question selected from the authorized private UUID.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Current browser concurrency token.
    pub expected_edit_number: DraftQuestionEditNumber,
    /// Exact source Object Record written before persistence registration.
    pub source_record: ObjectRecord,
    /// Required replacement author-declared educational Question Type.
    pub question_type: QuestionType,
    /// Source-derived Question Title.
    pub title: String,
    /// Source-derived Question Description.
    pub description: String,
    /// Source-derived language retained when the Draft becomes a Question Revision.
    pub language: String,
}

/// One deliberate metadata-only Draft edit. This is separate from Question
/// source saving so no backend source is parsed, reconstructed, or rewritten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveAuthoringDraftGeneralFeedbackInput {
    pub draft_question_uuid: DraftQuestionUuid,
    pub expected_edit_number: DraftQuestionEditNumber,
    pub general_feedback: Option<String>,
}

/// Exact owner-only Draft Question deletion precondition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteAuthoringDraftInput {
    pub draft_question_uuid: DraftQuestionUuid,
    pub expected_edit_number: DraftQuestionEditNumber,
}

/// Confirms that the initial Draft Question binding has an explicit source
/// format compatible with its media type. The database repeats this invariant
/// inside the creation transaction before it records the mutable Draft state.
#[cfg(feature = "postgres")]
pub(crate) fn validate_initial_draft_source_binding(
    media_type: &str,
    question_format: QuestionFormat,
    webwork_pg_path: Option<&str>,
) -> Result<(), StoreError> {
    match (media_type, question_format, webwork_pg_path) {
        ("application/vnd.peptidyle.question+json", QuestionFormat::PleQuestionJson, None) => {
            Ok(())
        }
        (
            "text/x-wework-pg",
            QuestionFormat::WebworkPg | QuestionFormat::WebworkPgml,
            Some(path),
        ) if valid_webwork_pg_path(path) => Ok(()),
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
        draft_question_uuid: DraftQuestionUuid,
    ) -> Result<AuthoringDraft, StoreError>;

    /// Replaces one mutable Draft Question Source using its exact Edit Number.
    async fn save_authoring_draft(
        &self,
        session_token_hash: SessionTokenHash,
        input: SaveAuthoringDraftInput,
    ) -> Result<AuthoringDraft, StoreError>;

    /// Replaces only deliberate general feedback under the ordinary Draft CAS.
    async fn save_authoring_draft_general_feedback(
        &self,
        session_token_hash: SessionTokenHash,
        input: SaveAuthoringDraftGeneralFeedbackInput,
    ) -> Result<AuthoringDraft, StoreError>;

    /// Permanently removes one Draft owned by the current Instructor under
    /// the ordinary Draft Edit Number compare-and-swap contract.
    async fn delete_authoring_draft(
        &self,
        session_token_hash: SessionTokenHash,
        input: DeleteAuthoringDraftInput,
    ) -> Result<(), StoreError>;
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::*;

    #[test]
    fn initial_draft_source_binding_matches_its_supported_media_type() {
        assert_eq!(
            validate_initial_draft_source_binding(
                "application/vnd.peptidyle.question+json",
                QuestionFormat::PleQuestionJson,
                None,
            ),
            Ok(())
        );
        assert_eq!(
            validate_initial_draft_source_binding(
                "text/x-wework-pg",
                QuestionFormat::WebworkPg,
                Some("Library/Algebra.pg"),
            ),
            Ok(())
        );
        assert_eq!(
            validate_initial_draft_source_binding(
                "text/x-wework-pg",
                QuestionFormat::WebworkPgml,
                Some("Library/Algebra.pgml"),
            ),
            Ok(())
        );
        for (media_type, format, path) in [
            (
                "application/vnd.peptidyle.question+json",
                QuestionFormat::PleQuestionJson,
                Some("Library/Algebra.pg"),
            ),
            ("text/x-wework-pg", QuestionFormat::WebworkPg, None),
            (
                "text/x-wework-pg",
                QuestionFormat::WebworkPgml,
                Some("../Algebra.pgml"),
            ),
            (
                "text/x-wework-pg",
                QuestionFormat::PleQuestionJson,
                Some("Library/Algebra.pg"),
            ),
            ("application/json", QuestionFormat::PleQuestionJson, None),
        ] {
            assert!(validate_initial_draft_source_binding(media_type, format, path).is_err());
        }
    }
}
