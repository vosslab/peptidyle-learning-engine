//! Student-authorized Assessment Access and answer-free initial delivery.

use async_trait::async_trait;
use browser_api_contract::student_assessment_decision::StudentAssessmentDecisionSummary;
use question_model::{
    AssessmentAttemptId, AssessmentId, AssessmentType, CourseInstanceId, CourseTheme,
    GradingResult, QuestionAttemptId, QuestionId, QuestionImageAssetId, QuestionRevisionTuple,
    StudentAssessmentAttemptProgress, StudentFeedback, StudentFeedbackReleaseRule, StudentResponse,
    Timestamp,
};
use serde::{Deserialize, Serialize};

use crate::{SessionTokenHash, StoreError};

/// Answer-free Assessment Access projection for the authenticated Student.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveAssessmentAccess {
    /// Complete policy and decision calculated at authoritative server time.
    pub decision: StudentAssessmentDecisionSummary,
    /// The current authorized unfinished Assessment Attempt, if one exists.
    pub active_assessment_attempt_id: Option<AssessmentAttemptId>,
    /// The effective Student-facing title for the current delivery state.
    pub title: String,
    /// Product-defined pedagogical Type for this Assessment.
    pub assessment_type: AssessmentType,
    /// Exact released or issued Question count, never inferred from grading.
    pub question_count: u32,
    /// Exact released or issued total points, never inferred from grading.
    pub points_possible: f64,
    /// Complete owned Assessment Attempt history, newest first, without
    /// responses, grading details, or private identifiers.
    pub previous_attempts: Vec<LiveAssessmentPreviousAttempt>,
}

/// Answer-free state for one owned prior Assessment Attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveAssessmentPreviousAttempt {
    pub assessment_attempt_id: AssessmentAttemptId,
    pub attempt_number: u32,
    pub state: LiveAssessmentPreviousAttemptState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<LiveAssessmentAttemptScore>,
}

/// The only prior-Attempt states exposed by the access landing projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveAssessmentPreviousAttemptState {
    Submitted,
    Closed,
}

/// Current aggregate score for one independently disclosed prior Attempt.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveAssessmentAttemptScore {
    pub points_earned: f64,
    pub points_possible: f64,
}

/// Server-projected record of one completed, owned Assessment Attempt.
///
/// Protected members are represented by absent fields at the HTTP boundary;
/// this storage type deliberately contains only the answer-free spine.  The
/// delivery server adds independently released presentation facts.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentAssessmentAttemptHistory {
    pub assessment_attempt_id: AssessmentAttemptId,
    pub attempt_number: u32,
    /// Current Course display identity, authorized with the completed Attempt.
    pub course: StudentAssessmentAttemptHistoryCourse,
    pub assessment: StudentAssessmentAttemptHistoryAssessment,
    pub state: LiveAssessmentPreviousAttemptState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<LiveAssessmentAttemptScore>,
    pub questions: Vec<StudentAssessmentAttemptHistoryQuestion>,
}

/// Public Course identity for a selected owned history record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentAssessmentAttemptHistoryCourse {
    pub id: CourseInstanceId,
    pub short_name: String,
    pub long_name: String,
    pub theme: CourseTheme,
}

/// Public Assessment identity for a selected owned history record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentAssessmentAttemptHistoryAssessment {
    pub id: AssessmentId,
    pub title: String,
}

/// One completed issued position.  This remains useful when every disclosure
/// setting is `never`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentAssessmentAttemptHistoryQuestion {
    pub position: u32,
    /// Exact immutable Question Revision retained by the Issued Question.
    ///
    /// This history projection never resolves a current Question or Assessment
    /// entry: its revision identity is part of the durable Student Work
    /// evidence that makes old presentations and their assets interpretable.
    pub question_revision_tuple: QuestionRevisionTuple,
    pub response_state: LiveAssessmentPreviousAttemptState,
    /// Readable submitted response, released independently from all grading
    /// and feedback fields. Omitted when withheld or exact reproduction fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Vec<question_model::QuestionContentBlock>>,
    /// Current permission to fetch a separately authorized backend answer document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_answer_review: Option<BackendAnswerReviewAvailability>,
    /// Independently disclosed current grade and teaching feedback. The
    /// server applies each release gate before this browser-safe projection.
    #[serde(flatten)]
    pub feedback: StudentFeedback,
}

/// Closed availability marker; it conveys no answer data or renderer location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BackendAnswerReviewAvailability {
    Available,
}

/// Private, already-authorized evidence used to build one selected history
/// response. This type is intentionally not serializable: disclosure is owned
/// by the delivery server after the Store confirms exact Student ownership.
#[derive(Debug, Clone, PartialEq)]
pub struct StudentAssessmentAttemptHistoryEvidence {
    pub history: StudentAssessmentAttemptHistory,
    /// Assessment Type copied from the current Assessment identity.
    pub assessment_type: AssessmentType,
    pub feedback_rule: StudentFeedbackReleaseRule,
    pub due_at: Option<Timestamp>,
    pub closes_at: Option<Timestamp>,
    pub submitted_at: Option<Timestamp>,
    /// Database-authoritative time at which the disclosure decision is read.
    pub evaluated_at: Timestamp,
    /// Current Course cohort completion calculated by the authorized history reader.
    pub all_students_completed: bool,
    /// True only when every issued position has a graded lifecycle, its exact
    /// grading result, and the matching immutable automated receipt.
    pub grading_is_current: bool,
    /// One result per ordered public position, retained below the HTTP seam.
    pub grading_results: Vec<Option<GradingResult>>,
}

/// Private completed-response evidence for one issued position.
///
/// This crosses only from the Student-authorized history reader to the server
/// reproduction boundary. Neither the canonical response nor its source is a
/// browser value.
#[derive(Debug, Clone, PartialEq)]
pub struct StudentAssessmentAttemptHistoryResponseSource {
    /// One-based public position in the selected completed Attempt.
    pub position: u32,
    /// Canonical durable response identifiers, retained below the HTTP seam.
    pub response: Option<StudentResponse>,
    /// Deliberately authored PLE-managed feedback for the exact issued
    /// Question Revision. It is not backend source or interaction feedback.
    pub general_feedback: Option<String>,
    /// Complete immutable answer-free descriptor and its binding.  Submitted
    /// responses are interpreted from this retained evidence, independently of
    /// the source object or renderer remaining available.
    pub presentation_evidence: StudentAssessmentAttemptPresentationEvidence,
    /// Exact PLE or WeBWorK source retained for released teaching content.
    /// Student response interpretation uses `presentation_evidence`.
    pub presentation_source: Option<StudentAssessmentAttemptPresentationSource>,
}

/// One answer-free fixed Question presentation issued to the Student.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedQuestionPresentation {
    /// Exact released Assessment Entry identity retained only within the
    /// server/store seam. It binds source reproduction to an Issued Question;
    /// `position` is the Student-facing sequence and may be shuffled.
    #[serde(skip_serializing)]
    pub assessment_entry_id: String,
    /// Exact Published Question identity, without an internal row locator.
    pub question_id: QuestionId,
    /// Answer-free Question description for this first delivery slice.
    pub description: String,
    /// Stable one-based Student-facing position in this Assessment Attempt.
    pub position: u32,
    /// Private reproduction facts consumed by the server before serialization.
    #[serde(skip_serializing)]
    pub revision_number: u32,
    #[serde(skip_serializing)]
    /// Static native PLE and seeded renderer-backed reproduction stay tagged
    /// below the browser boundary.
    pub reproduction: question_model::QuestionReproduction,
    #[serde(skip_serializing)]
    pub presentation_nonce: String,
    #[serde(skip_serializing)]
    pub presentation_checksum: String,
}

/// A started or resumed Assessment Attempt with no response or grading input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveAssessmentAttempt {
    /// Public attempt identity used only after server authorization.
    pub assessment_attempt_id: AssessmentAttemptId,
    /// Public Assessment locator; the private Attempt identity stays server-side.
    pub assessment_id: AssessmentId,
    /// One-based Student-specific Attempt sequence.
    pub attempt_number: u32,
    /// Released Student-facing Assessment title.
    pub title: String,
    /// Released Student-facing Assessment instructions.
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
    /// Canonical Issued Question identity selected by PostgreSQL for this
    /// presentation commit. It is absent only when decoding an already
    /// committed presentation source, which never creates a new commit.
    pub issued_question_id: Option<uuid::Uuid>,
    pub assessment_entry_id: String,
    pub position: u32,
    pub question_id: QuestionId,
    pub revision_number: u32,
    pub source_object_id: String,
    pub source_object_address: serde_json::Value,
    pub source_object_checksum: String,
    /// Server-only pre-render issuance input. Native PLE is explicitly static;
    /// it never borrows a seed-shaped placeholder from a renderer backend.
    pub reproduction: QuestionIssuanceReproductionInput,
    pub presentation_nonce: Option<String>,
    pub presentation_checksum: Option<String>,
    /// Complete immutable descriptor when this position was already committed.
    /// New issuance leaves this absent until its one commit succeeds.
    pub retained_presentation: Option<StudentAssessmentAttemptPresentationEvidence>,
    /// Ready, exact-revision Question Image Renditions. These contain no object
    /// locator or source bytes and are bound into the Question Presentation.
    pub question_image_renditions: Vec<ReadyQuestionImageRendition>,
}

/// One server-selected Ready public rendition for an authored Question Image Asset.
/// The Question Image Asset checksum is content identity, not an object-store locator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadyQuestionImageRendition {
    pub question_image_asset_id: QuestionImageAssetId,
    pub question_image_checksum: String,
    pub rendition_checksum: String,
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,
}

/// Server-only immutable-source facts for native WeBWorK issuance.
///
/// The registered PG path and source pins are never serialized.  They cross
/// only from the Student-authorized procedure to the API process that talks
/// to the private renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeWebworkIssuanceSource {
    /// Canonical Issued Question identity selected by PostgreSQL for this
    /// presentation commit. It is absent only when decoding an already
    /// committed presentation source, which never creates a new commit.
    pub issued_question_id: Option<uuid::Uuid>,
    pub assessment_entry_id: String,
    pub position: u32,
    pub question_id: QuestionId,
    pub revision_number: u32,
    pub source_object_id: String,
    pub source_object_checksum: String,
    pub webwork_pg_path: String,
    /// Server-only pre-render issuance input. Its seed exists before the
    /// renderer produces the paired generated-parameter checksum.
    pub reproduction: QuestionIssuanceReproductionInput,
    /// Complete immutable descriptor when this position was already committed.
    /// New issuance leaves this absent until its one commit succeeds.
    pub retained_presentation: Option<StudentAssessmentAttemptPresentationEvidence>,
    /// Exact retained Question Image Renditions. Resume uses these bindings rather than
    /// consulting mutable current publication state.
    pub question_image_renditions: Vec<ReadyQuestionImageRendition>,
}

/// Server-only reproduction input for the narrow period before a renderer has
/// produced its generated-parameter checksum.
///
/// This is deliberately distinct from [`question_model::QuestionReproduction`]:
/// persisted/read attempt evidence is always either Static or has the complete
/// seeded `(seed, checksum)` pair. The server uses this input only to invoke a
/// backend renderer and then commits the complete durable tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionIssuanceReproductionInput {
    /// Immutable native source with no generated variation.
    Static,
    /// Renderer-generated variation before its checksum exists.
    Seeded {
        question_seed: question_model::QuestionSeed,
    },
}

/// Private, write-once evidence for one WeBWorK Question Attempt.
#[derive(Debug, Clone)]
pub struct NativePresentationInput {
    pub issued_question_id: String,
    pub assessment_entry_id: String,
    pub position: u32,
    pub question_id: QuestionId,
    pub revision_number: u32,
    /// Cohesive static-or-seeded reproduction facts.  The database codec maps
    /// this to its paired nullable seed/hash columns without a sentinel.
    pub reproduction: question_model::QuestionReproduction,
    pub reproduction_details: serde_json::Value,
    /// Complete answer-free rendered descriptor retained with the Attempt.
    pub presentation: serde_json::Value,
    pub presentation_nonce: String,
    pub presentation_checksum: String,
    /// Immutable answer-free isolated author-content descriptor. This is
    /// retained below generic browser presentation DTOs.
    pub author_content: Option<question_model::AuthorContentPresentation>,
    /// Exact durable response-item mappings extracted before answer-free
    /// presentation persistence. Scalar formats carry an empty vector.
    pub response_item_bindings: Vec<question_model::presentation::DurableResponseItemBinding>,
    /// Exact ready PLE renditions for a mixed native Assessment. WeBWorK-only
    /// entries carry an empty list; this never crosses the browser boundary.
    pub question_image_renditions: Vec<ReadyQuestionImageRendition>,
    /// Explicit issued capability.  It is recorded independently from the
    /// backend document so PLE never infers a backend from document presence.
    pub issued_capability: String,
    /// Immutable backend-owned document.  WeBWorK supplies one; PLE-native
    /// presentations leave this absent.
    pub backend_document: Option<String>,
}

/// One Student-authorized backend document read result for an issued position.
/// The retained document remains immutable evidence. A saved backend-owned
/// response adds only private inputs for an ephemeral backend-authored resume
/// render; neither form crosses the public presentation boundary.
#[derive(Clone, PartialEq)]
pub struct StudentAssessmentAttemptBackendDocument {
    pub backend_document: String,
    /// Private inputs for a backend-authored resume render.  This is present
    /// only for a saved backend-owned response and never crosses the HTTP
    /// boundary as PLE presentation data.
    pub resume: Option<StudentAssessmentAttemptBackendDocumentResume>,
}

/// Exact immutable source and opaque response for one backend-authored resume
/// render.  The server consumes this below the document route boundary.
#[derive(Clone, PartialEq)]
pub struct StudentAssessmentAttemptBackendDocumentResume {
    pub source: NativeWebworkIssuanceSource,
    pub saved_response: StudentResponse,
}

/// One complete native issuance operation. The Attempt identity and start
/// decision belong to this batch, rather than being repeated on every source.
#[derive(Debug, Clone)]
pub struct NativeAssessmentIssuanceBatch {
    pub assessment_attempt_id: uuid::Uuid,
    /// The result of the one canonical Attempt-start operation.
    pub attempt_was_resumed: bool,
    /// All native positions already have a complete immutable presentation.
    pub presentation_is_committed: bool,
    pub committed_attempt: Option<LiveAssessmentAttempt>,
    pub retained_presentations: Vec<StudentAssessmentAttemptPresentationEvidence>,
    pub ple_sources: Vec<NativePleIssuanceSource>,
    pub webwork_sources: Vec<NativeWebworkIssuanceSource>,
}

/// Private, immutable answer-free evidence for one issued Question Attempt.
///
/// PostgreSQL returns this only after proving Student ownership.  It contains
/// no current Assessment or Question configuration and is the authoritative
/// input for selected-presentation and saved-response interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudentAssessmentAttemptPresentationEvidence {
    pub question_revision_tuple: QuestionRevisionTuple,
    /// Static or seeded reproduction facts. These never cross the HTTP seam.
    pub reproduction: question_model::QuestionReproduction,
    pub presentation_nonce: String,
    pub presentation_checksum: String,
    pub presentation: serde_json::Value,
    /// Isolated author-content evidence for the separately authorized document
    /// route. Generic Student Question presentation responses omit this.
    pub author_content: Option<question_model::AuthorContentPresentation>,
    /// Exact normalized durable identities for each public response item.
    pub response_item_bindings: Vec<question_model::presentation::DurableResponseItemBinding>,
    pub question_image_renditions: Vec<ReadyQuestionImageRendition>,
}

impl NativeAssessmentIssuanceBatch {
    /// New source pins require one commit; a complete retained bundle is only
    /// reproduced and is never written again.
    pub fn requires_presentation_commit(&self) -> bool {
        !self.presentation_is_committed
    }
}

/// Private immutable facts needed to reproduce one selected issued Question.
/// This type must never be serialized at the browser boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudentAssessmentAttemptPresentationSource {
    Ple {
        question_attempt: QuestionAttemptId,
        source: NativePleIssuanceSource,
    },
    Webwork {
        question_attempt: QuestionAttemptId,
        source: NativeWebworkIssuanceSource,
        reproduction: question_model::QuestionReproduction,
        presentation_nonce: String,
        presentation_checksum: String,
    },
}

#[cfg(test)]
mod presentation_source_tests {
    use super::*;

    #[test]
    fn batch_keeps_one_started_attempt_identity_and_separates_commit_state() {
        let assessment_attempt_id = uuid::Uuid::nil();
        let new = NativeAssessmentIssuanceBatch {
            assessment_attempt_id,
            attempt_was_resumed: false,
            presentation_is_committed: false,
            committed_attempt: None,
            retained_presentations: Vec::new(),
            ple_sources: Vec::new(),
            webwork_sources: Vec::new(),
        };
        assert_eq!(new.assessment_attempt_id, assessment_attempt_id);
        assert!(new.requires_presentation_commit());

        let committed = NativeAssessmentIssuanceBatch {
            presentation_is_committed: true,
            ..new
        };
        assert!(!committed.requires_presentation_commit());
    }
}

/// Confirmation that one owned working response was saved at its fixed position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudentAssessmentAttemptSavedResponse {
    /// Assessment Attempt identity that owns the saved response.
    pub assessment_attempt_id: AssessmentAttemptId,
    /// One-based fixed issued Question position.
    pub position: u32,
}

/// Browser-safe Student delivery chrome for one authorized Assessment Attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentAssessmentAttemptContext {
    pub assessment_attempt_id: AssessmentAttemptId,
    pub attempt_number: u32,
    pub course_instance_id: CourseInstanceId,
    pub course_short_name: String,
    pub course_long_name: String,
    pub course_theme: CourseTheme,
    pub assessment_id: AssessmentId,
    pub assessment_title: String,
    /// Authenticated Student's selected IANA display zone.
    pub display_time_zone: question_model::AccountTimeZone,
    /// Immutable server-owned deadline recorded when this Attempt started.
    pub expires_at: Option<Timestamp>,
    /// Server-evaluated nonnegative duration from the same deadline read.
    pub timer_remaining_milliseconds: Option<u64>,
}

/// Result of the single explicit Assessment Attempt submission action.
#[derive(Debug, Clone, PartialEq)]
pub enum StudentAssessmentAttemptFinalization {
    /// Saved working responses became immutable submission evidence; unanswered
    /// positions remain explicit in the completed Attempt history.
    Submitted {
        /// Immediate grading returns a score. A grading result that becomes
        /// current later is absent without changing submission finality.
        score: Option<LiveAssessmentAttemptScore>,
    },
}

/// Database-selected reason for finalizing one Assessment Attempt. The server
/// never supplies this fact: PostgreSQL derives it from `expires_at` and its
/// own clock while authorizing the snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudentAssessmentAttemptFinalizationKind {
    Student,
    Deadline,
}

/// One exact saved response selected for direct backend evaluation. This is
/// private server/store evidence: response bytes and source locators never
/// serialize to the browser.
#[derive(Debug, Clone)]
pub struct StudentAssessmentAttemptFinalizationSource {
    pub question_attempt_id: uuid::Uuid,
    pub saved_at: Timestamp,
    pub question_id: QuestionId,
    pub revision_number: u32,
    pub source_object_id: String,
    pub source_object_checksum: String,
    /// Committed static-or-seeded reproduction facts selected with this saved
    /// response. Native PLE finalization is explicitly Static.
    pub reproduction: question_model::QuestionReproduction,
    pub student_response: StudentResponse,
    pub backend: StudentAssessmentAttemptFinalizationBackend,
}

/// The source binding selected for one saved response.
#[derive(Debug, Clone)]
pub enum StudentAssessmentAttemptFinalizationBackend {
    Ple,
    Webwork { pg_path: String },
}

/// Immutable snapshot returned before backend I/O. PostgreSQL accepts its
/// evaluations only when every saved-response version still matches.
#[derive(Debug, Clone)]
pub struct StudentAssessmentAttemptFinalizationPreparation {
    pub kind: StudentAssessmentAttemptFinalizationKind,
    pub saved_responses: Vec<StudentAssessmentAttemptFinalizationSource>,
}

/// One backend credit bound to a saved-response version from a preparation.
#[derive(Debug, Clone, PartialEq)]
pub struct StudentAssessmentAttemptFinalizationEvaluation {
    pub question_attempt_id: uuid::Uuid,
    pub saved_at: Timestamp,
    /// Exact saved bytes the backend evaluated. PostgreSQL compares this with
    /// the current saved response before accepting the returned credit.
    pub student_response: StudentResponse,
    pub normalized_credit: f64,
}

/// The direct-finalization preparation outcome.
#[derive(Debug, Clone)]
pub enum StudentAssessmentAttemptFinalizationPreparationOutcome {
    AlreadySubmitted {
        score: Option<LiveAssessmentAttemptScore>,
    },
    Ready(StudentAssessmentAttemptFinalizationPreparation),
}

/// Store boundary for Student Assessment Access and initial issue.
#[async_trait]
pub trait LiveAssessmentDeliveryStore: Send + Sync {
    /// Loads the minimal authorized route context for an open or submitted Student Attempt.
    async fn student_assessment_attempt_context(
        &self,
        session_token_hash: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
    ) -> Result<StudentAssessmentAttemptContext, StoreError>;

    /// Saves one canonical Student response for an active, owned issued position.
    ///
    /// The server validates and translates presentation IDs before this
    /// boundary. PostgreSQL serializes this write with finalization.
    async fn save_student_assessment_attempt_response(
        &self,
        session_token_hash: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
        position: u32,
        response: StudentResponse,
    ) -> Result<StudentAssessmentAttemptSavedResponse, StoreError>;

    /// Reads the canonical saved response for one owned active issued position.
    ///
    /// `None` means that this exact Question has no saved working response.
    async fn student_assessment_attempt_saved_response(
        &self,
        session_token_hash: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
        position: u32,
    ) -> Result<Option<StudentResponse>, StoreError>;

    /// Captures an authorized saved-response snapshot before backend I/O. The
    /// database chooses deadline versus Student finalization from its clock.
    async fn prepare_student_assessment_attempt_finalization(
        &self,
        session_token_hash: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
    ) -> Result<StudentAssessmentAttemptFinalizationPreparationOutcome, StoreError>;

    /// Atomically accepts only the still-current prepared snapshot and stores
    /// immutable backend credits, finalized Question responses, and the terminal
    /// Assessment Submission.
    async fn commit_student_assessment_attempt_finalization(
        &self,
        session_token_hash: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
        preparation: StudentAssessmentAttemptFinalizationPreparation,
        evaluations: Vec<StudentAssessmentAttemptFinalizationEvaluation>,
    ) -> Result<StudentAssessmentAttemptFinalization, StoreError>;

    /// Loads an answer-free, Student-owned progress projection by Assessment Attempt ID.
    async fn student_assessment_attempt_progress(
        &self,
        session_token_hash: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
    ) -> Result<StudentAssessmentAttemptProgress, StoreError>;

    /// Reads exactly one immutable issued-presentation bundle inside the authenticated Student boundary.
    async fn student_assessment_attempt_presentation_evidence(
        &self,
        session_token_hash: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
        position: u32,
    ) -> Result<StudentAssessmentAttemptPresentationEvidence, StoreError>;

    /// Reads one backend-owned document for an owned WeBWorK issued position.
    /// Its immutable retained document is always present; source pins, seed,
    /// and saved opaque response are private resume inputs when applicable.
    async fn student_assessment_attempt_backend_document(
        &self,
        session_token_hash: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
        position: u32,
    ) -> Result<StudentAssessmentAttemptBackendDocument, StoreError>;
    /// Starts exactly one current Attempt and resolves every native source pin
    /// for its one presentation transaction.
    async fn prepare_native_assessment_issuance(
        &self,
        session_token_hash: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
    ) -> Result<NativeAssessmentIssuanceBatch, StoreError>;

    /// Commits all native presentation evidence in one atomic operation, or
    /// returns its already-complete immutable issue set.
    async fn commit_native_assessment_issuance(
        &self,
        session_token_hash: SessionTokenHash,
        assessment_attempt_id: uuid::Uuid,
        presentations: Vec<NativePresentationInput>,
    ) -> Result<LiveAssessmentAttempt, StoreError>;
    /// Calculates current Assessment Access for the authenticated Student only.
    async fn live_assessment_access(
        &self,
        session_token_hash: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
    ) -> Result<LiveAssessmentAccess, StoreError>;

    /// Reads one completed Attempt after the store has re-authorized exact
    /// Student ownership and active Course membership.
    async fn student_assessment_attempt_history(
        &self,
        session_token_hash: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
    ) -> Result<StudentAssessmentAttemptHistoryEvidence, StoreError>;

    /// Loads submitted canonical responses and their exact pinned presentation
    /// sources after re-authorizing the selected owned Attempt.
    async fn student_assessment_attempt_history_response_sources(
        &self,
        session_token_hash: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
    ) -> Result<Vec<StudentAssessmentAttemptHistoryResponseSource>, StoreError>;
}
