//! Private Authoring Workspace persistence contracts.
//!
//! Browser routes carry a private Draft UUID and an Edit Number.
//! The Store rechecks the active session relationship for every UUID.

use async_trait::async_trait;
use objects::ObjectRecord;
use question_model::{
    DraftQuestionClassification, PublishedQuestionRevisionTuple, QuestionAuthorDisplayName,
    QuestionBackend, QuestionFormat, QuestionMetadata, QuestionType, WorkspaceId,
};
use uuid::Uuid;

use crate::{DraftQuestionEditNumber, DraftQuestionUuid, SessionTokenHash, StoreError};

/// Server-only resolved state for one private Draft Question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoringDraft {
    /// Private transport and persistence identity; never display this as a public ID.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Trusted private workspace identity; never serialize this to a browser.
    pub workspace: WorkspaceId,
    /// Exact immediate source Revision for a forked Draft Question.
    pub parent_published_question_revision_tuple: Option<PublishedQuestionRevisionTuple>,
    /// Positive save concurrency token.
    pub edit_number: DraftQuestionEditNumber,
    /// Exact Draft Question record metadata; native source contains no copy.
    pub metadata: QuestionMetadata,
    /// Current shared classification copied from the exact parent Revision.
    pub classification: Option<DraftQuestionClassification>,
    /// Registered backend; source bytes never select or change it.
    pub question_backend: QuestionBackend,
    /// Registered source representation; source bytes never select or change it.
    pub question_format: QuestionFormat,
    /// Immutable registered WebWork PG location, when this is a WebWork Draft.
    pub webwork_pg_path: Option<String>,
    /// Deliberately authored backend-independent feedback for the next
    /// published Question Revision. It is never source-derived.
    pub general_feedback: Option<String>,
    /// Optional PLE-managed Hint copied onto the next Published Question Revision.
    /// ASVS 8.2.3: this is not Question Backend source.
    pub hint: Option<String>,
    /// Optional PLE-managed Worked Solution copied onto the next Published Question Revision.
    /// ASVS 8.2.3: this is not Question Backend source.
    pub worked_solution: Option<String>,
    /// Saved public author display names copied into this Draft.
    pub authors: Vec<QuestionAuthorDisplayName>,
    /// Current author-declared educational type that publication makes immutable.
    pub question_type: Option<QuestionType>,
    /// Exact current private source object evidence.
    pub source_record: ObjectRecord,
}

impl AuthoringDraft {
    /// Media type established by the persisted source binding.
    pub fn source_media_type(&self) -> Option<&'static str> {
        registered_source_media_type(
            self.question_backend,
            self.question_format,
            self.webwork_pg_path.as_deref(),
        )
    }
}

fn registered_source_media_type(
    question_backend: QuestionBackend,
    question_format: QuestionFormat,
    webwork_pg_path: Option<&str>,
) -> Option<&'static str> {
    match (question_backend, question_format, webwork_pg_path) {
        (QuestionBackend::Ple, QuestionFormat::PleQuestionJson, None) => {
            Some("application/vnd.peptidyle.question+json")
        }
        (
            QuestionBackend::Webwork,
            QuestionFormat::WebworkPg | QuestionFormat::WebworkPgml,
            Some(path),
        ) if valid_webwork_pg_path(path) => Some("text/x-wework-pg"),
        _ => None,
    }
}

/// Answer-free list entry for the current Instructor's My Question Drafts View.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoringDraftSummary {
    pub draft_question_uuid: DraftQuestionUuid,
    /// Exact immediate source Revision for a forked Draft Question.
    pub parent_published_question_revision_tuple: Option<PublishedQuestionRevisionTuple>,
    pub edit_number: DraftQuestionEditNumber,
    pub metadata: QuestionMetadata,
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
    /// Supplied record metadata, kept outside backend source.
    pub metadata: QuestionMetadata,
    /// Native source-derived or existing WebWork Type; absent for unfinished Native source.
    pub question_type: Option<QuestionType>,
}

/// Raw source replacement under the immutable Draft binding and Edit Number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveAuthoringDraftInput {
    /// Draft Question selected from the authorized private UUID.
    pub draft_question_uuid: DraftQuestionUuid,
    /// Current browser concurrency token.
    pub expected_edit_number: DraftQuestionEditNumber,
    /// Exact source Object Record written before persistence registration.
    pub source_record: ObjectRecord,
    /// Native source-derived or existing WebWork Type; absent for unfinished Native source.
    pub question_type: Option<QuestionType>,
}

/// One deliberate metadata-only Draft edit. This is separate from Question
/// source saving so no backend source is parsed, reconstructed, or rewritten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveAuthoringDraftGeneralFeedbackInput {
    pub draft_question_uuid: DraftQuestionUuid,
    pub expected_edit_number: DraftQuestionEditNumber,
    /// Complete record metadata replacement, independent from source and support.
    pub metadata: QuestionMetadata,
    pub general_feedback: Option<String>,
    /// Written only when `replace_support` is true. ASVS 2.2.1.
    pub hint: Option<String>,
    /// Written only when `replace_support` is true. ASVS 2.2.1.
    pub worked_solution: Option<String>,
    /// False leaves the stored Hint and Worked Solution unchanged.
    pub replace_support: bool,
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
    let question_backend = match question_format {
        QuestionFormat::PleQuestionJson => QuestionBackend::Ple,
        QuestionFormat::WebworkPg | QuestionFormat::WebworkPgml => QuestionBackend::Webwork,
        QuestionFormat::Imathas => QuestionBackend::Imathas,
    };
    if registered_source_media_type(question_backend, question_format, webwork_pg_path)
        == Some(media_type)
    {
        Ok(())
    } else {
        Err(StoreError::InvalidRecord(
            "Draft Question source media type and initial source binding are inconsistent"
                .to_string(),
        ))
    }
}

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

    /// Saves metadata/support and optionally updates manual WebWork Type in the
    /// same Draft Edit Number transaction. `None` preserves, `Some(None)` clears.
    async fn save_authoring_draft_general_feedback_with_question_type(
        &self,
        session_token_hash: SessionTokenHash,
        input: SaveAuthoringDraftGeneralFeedbackInput,
        question_type: Option<Option<QuestionType>>,
    ) -> Result<AuthoringDraft, StoreError> {
        if question_type.is_some() {
            return Err(StoreError::InvalidRecord(
                "this Draft store does not support manual Question Type updates".to_owned(),
            ));
        }
        self.save_authoring_draft_general_feedback(session_token_hash, input)
            .await
    }

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
