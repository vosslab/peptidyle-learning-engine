//! Student-authorized Assignment Access and answer-free initial delivery.

use async_trait::async_trait;
use question_model::{
    AssignmentAttemptReference, AssignmentReference, CourseInstanceReference, CourseTheme,
    QuestionAssetId, QuestionAttemptId, QuestionId, StudentAssignmentAttemptProgress,
    StudentResponse,
};
use serde::Serialize;

use crate::{SessionTokenHash, StoreError};

/// The current server-calculated ability to start one released Assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveAssignmentStartDecision {
    /// The exact active Student Record may start now.
    MayStart,
    /// The released Assignment has not reached its availability time.
    NotYetAvailable,
    /// The Assignment is not available for a new Attempt.
    Closed,
    /// The Student has already used every allowed Attempt.
    AttemptLimitReached,
    /// The released late-work policy refuses a new Attempt.
    LateWorkRefused,
}

/// Answer-free Assignment Access projection for the authenticated Student.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveAssignmentAccess {
    /// The calculated decision at authoritative server time.
    pub start_decision: LiveAssignmentStartDecision,
    /// The current authorized unfinished Assignment Attempt, if one exists.
    pub active_assignment_attempt: Option<AssignmentAttemptReference>,
}

/// One answer-free fixed Question presentation issued to the Student.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedQuestionPresentation {
    /// Exact Published Question identity, without an internal row locator.
    pub question_id: QuestionId,
    /// Answer-free Question description for this first delivery slice.
    pub description: String,
    /// Stable zero-based position in the released Assignment Revision.
    pub position: u32,
    /// Private reproduction facts consumed by the server before serialization.
    #[serde(skip_serializing)]
    pub revision_number: u32,
    #[serde(skip_serializing)]
    pub question_seed: u64,
    #[serde(skip_serializing)]
    pub presentation_nonce: String,
    #[serde(skip_serializing)]
    pub presentation_checksum: String,
}

/// A started or resumed Assignment Attempt with no response or grading input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveAssignmentAttempt {
    /// Public attempt identity used only after server authorization.
    pub assignment_attempt: AssignmentAttemptReference,
    /// Public Assignment locator; the private Attempt identity stays server-side.
    pub assignment: AssignmentReference,
    /// One-based Student-specific Attempt sequence.
    pub attempt_number: u32,
    /// Whether the existing unfinished Attempt was returned.
    pub resumed: bool,
    /// Released Student-facing Assignment title.
    pub title: String,
    /// Released Student-facing Assignment instructions.
    pub instructions: String,
    /// Ordered answer-free Issued Question presentations.
    pub questions: Vec<IssuedQuestionPresentation>,
}

/// Server-only immutable-source facts for native PLE issuance.
///
/// This is deliberately not serializable: the HTTP boundary receives only the
/// resulting answer-free Question Presentation, never its source locator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePleIssuanceSource {
    pub assignment_entry_id: String,
    pub position: u32,
    pub question_id: QuestionId,
    pub revision_number: u32,
    pub source_object_id: String,
    pub source_object_address: serde_json::Value,
    pub source_object_checksum: String,
    /// True only when the durable attempt is available for exact reproduction.
    pub resumed: bool,
    /// Persisted only for an existing Question Attempt; never browser data here.
    pub question_seed: Option<u64>,
    pub presentation_nonce: Option<String>,
    pub presentation_checksum: Option<String>,
    /// Ready, exact-revision public asset renditions. These contain no object
    /// locator or source bytes and are bound into the Question Presentation.
    pub question_asset_renditions: Vec<ReadyQuestionAssetRendition>,
}

/// One server-selected Ready public rendition for an authored Question Asset.
/// The Question Asset checksum is content identity, not an object-store locator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadyQuestionAssetRendition {
    pub question_asset: QuestionAssetId,
    pub question_asset_checksum: String,
    pub rendition_checksum: String,
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,
}

/// Private server-prepared evidence for one atomic native PLE issue commit.
#[derive(Debug, Clone)]
pub struct NativePlePresentationInput {
    pub issued_question_id: String,
    pub assignment_entry_id: String,
    pub position: u32,
    pub question_id: QuestionId,
    pub revision_number: u32,
    pub question_seed: u64,
    pub parameter_hash: String,
    pub reproduction_details: serde_json::Value,
    pub presentation_nonce: String,
    pub presentation_checksum: String,
    /// Exact ready public asset renditions incorporated into this immutable
    /// Question Presentation. This remains server-only commit evidence.
    pub question_asset_renditions: Vec<ReadyQuestionAssetRendition>,
}

/// Server-only immutable-source facts for native WeBWorK issuance.
///
/// The registered PG path and source pins are never serialized.  They cross
/// only from the Student-authorized procedure to the API process that talks
/// to the private renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeWebworkIssuanceSource {
    pub assignment_entry_id: String,
    pub position: u32,
    pub question_id: QuestionId,
    pub revision_number: u32,
    pub source_object_id: String,
    pub source_object_checksum: String,
    pub webwork_pg_path: String,
    pub resumed: bool,
}

/// Private, write-once evidence for one WeBWorK Question Attempt.
#[derive(Debug, Clone)]
pub struct NativeWebworkPresentationInput {
    pub issued_question_id: String,
    pub assignment_entry_id: String,
    pub position: u32,
    pub question_id: QuestionId,
    pub revision_number: u32,
    pub question_seed: u64,
    pub parameter_hash: String,
    pub reproduction_details: serde_json::Value,
    pub presentation_nonce: String,
    pub presentation_checksum: String,
    /// Exact ready PLE renditions for a mixed native Assignment. WeBWorK-only
    /// entries carry an empty list; this never crosses the browser boundary.
    pub question_asset_renditions: Vec<ReadyQuestionAssetRendition>,
    /// Provider form/value mapping; this is never browser data.
    pub replay_details: Option<serde_json::Value>,
}

/// Private immutable facts needed to reproduce one selected issued Question.
/// This type must never be serialized at the browser boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudentAssignmentAttemptPresentationSource {
    Ple {
        question_attempt: QuestionAttemptId,
        source: NativePleIssuanceSource,
    },
    Webwork {
        question_attempt: QuestionAttemptId,
        source: NativeWebworkIssuanceSource,
        question_seed: u64,
        presentation_nonce: String,
        presentation_checksum: String,
    },
}

/// Confirmation that one owned working response was saved at its fixed position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudentAssignmentAttemptSavedResponse {
    /// Public Assignment Attempt reference that owns the saved response.
    pub assignment_attempt: AssignmentAttemptReference,
    /// One-based fixed issued Question position.
    pub position: u32,
}

/// Browser-safe Student delivery chrome for one authorized Assignment Attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentAssignmentAttemptContext {
    pub assignment_attempt: AssignmentAttemptReference,
    pub attempt_number: u32,
    pub course: CourseInstanceReference,
    pub course_title: String,
    pub course_theme: CourseTheme,
    pub assignment: AssignmentReference,
    pub assignment_title: String,
    /// Server-evaluated nonnegative duration; no absolute deadline reaches the browser.
    pub timer_remaining_milliseconds: Option<u64>,
}

/// Result of the single explicit Assignment Attempt submission action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudentAssignmentAttemptFinalization {
    /// All saved working responses became immutable submission evidence.
    Submitted,
    /// The Attempt remains open because these one-based positions need saved responses.
    MissingResponses { positions: Vec<u32> },
}

/// Store boundary for Student Assignment Access and initial issue.
#[async_trait]
pub trait LiveAssignmentDeliveryStore: Send + Sync {
    /// Loads the minimal authorized route context for an open or submitted Student Attempt.
    async fn student_assignment_attempt_context(
        &self,
        session_token_hash: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
    ) -> Result<StudentAssignmentAttemptContext, StoreError>;

    /// Saves one canonical Student response for an active, owned issued position.
    ///
    /// The server validates and translates presentation references before this
    /// boundary. PostgreSQL serializes this write with finalization.
    async fn save_student_assignment_attempt_response(
        &self,
        session_token_hash: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
        position: u32,
        response: StudentResponse,
    ) -> Result<StudentAssignmentAttemptSavedResponse, StoreError>;

    /// Reads the canonical saved response for one owned active issued position.
    ///
    /// `None` means that this exact Question has no saved working response.
    async fn student_assignment_attempt_saved_response(
        &self,
        session_token_hash: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
        position: u32,
    ) -> Result<Option<StudentResponse>, StoreError>;

    /// Finalizes the entire owned Assignment Attempt in one database transition.
    async fn finalize_student_assignment_attempt(
        &self,
        session_token_hash: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
    ) -> Result<StudentAssignmentAttemptFinalization, StoreError>;

    /// Loads an answer-free, Student-owned progress projection by public Attempt reference.
    async fn student_assignment_attempt_progress(
        &self,
        session_token_hash: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
    ) -> Result<StudentAssignmentAttemptProgress, StoreError>;

    /// Resolves exactly one already-issued position inside the authenticated Student boundary.
    async fn student_assignment_attempt_presentation_source(
        &self,
        session_token_hash: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
        position: u32,
    ) -> Result<StudentAssignmentAttemptPresentationSource, StoreError>;
    /// Resolves only the Student-authorized fixed WeBWorK source pins for the
    /// private renderer boundary.
    async fn prepare_native_webwork_issuance(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<Vec<NativeWebworkIssuanceSource>, StoreError>;

    /// Commits immutable WeBWorK presentation and replay evidence, or returns
    /// the prior durable issue set without rewriting it.
    async fn commit_native_webwork_issuance(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        presentations: Vec<NativeWebworkPresentationInput>,
    ) -> Result<LiveAssignmentAttempt, StoreError>;
    /// Resolves only the Student-authorized fixed PLE source pins for server issuance.
    async fn prepare_native_ple_issuance(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<Vec<NativePleIssuanceSource>, StoreError>;

    /// Commits all private native issue evidence, or returns its prior durable issue set.
    async fn commit_native_ple_issuance(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        presentations: Vec<NativePlePresentationInput>,
    ) -> Result<LiveAssignmentAttempt, StoreError>;
    /// Calculates current Assignment Access for the authenticated Student only.
    async fn live_assignment_access(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<LiveAssignmentAccess, StoreError>;

    /// Atomically starts or resumes the exact released Assignment Revision.
    async fn start_live_assignment(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<LiveAssignmentAttempt, StoreError>;
}
