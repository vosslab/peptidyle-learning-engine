//! Course-owned Student Record, Assessment Attempt, Issued Question, and
//! Question Attempt records.
//!
//! Completion of one Assessment Attempt does not end a Student Record. A
//! Student can begin another Assessment Attempt when its explicit continuation
//! rules permit it, and each Assessment Attempt owns its Issued Questions and
//! Question Attempts. Historical completion remains immutable.
//!
//! These are educational records. Their exact Course, Student Record,
//! Assessment, and Issued Question relationships provide authorization scope;
//! an installation-wide Account is the global product identity.

use serde::{Deserialize, Serialize};
#[cfg(test)]
use uuid::Uuid;

mod attempt_evidence;
mod grading;
mod source_object_checksum;
#[cfg(test)]
#[path = "student_work/source_object_checksum_tests.rs"]
mod source_object_checksum_tests;

pub use attempt_evidence::{
    AssessmentAttempt, AssessmentAttemptCompletion, AssessmentAttemptEvidence,
    AssessmentAttemptPolicySource, AssessmentAttemptPolicySources, AssessmentGrade,
};
pub use grading::{GradingResult, QuestionEvaluation, QuestionEvaluationError, RecordedCredit};
pub use source_object_checksum::{SourceObjectChecksum, SourceObjectChecksumError};

use crate::QuestionRevisionTuple;
use crate::assessment::{AssessmentEntryScoringRule, AssessmentPointValue};
use crate::generation::{QuestionReproduction, QuestionSourceSelection};
use crate::identity::ObjectId;
use crate::response::StudentResponse;

mod identifiers;

pub use identifiers::{
    AccommodationId, AssessmentAttemptId, AssessmentEntryId, AssessmentId, CourseInstanceId,
    CourseMembershipId, IssuedQuestionId, QuestionAttemptId, QuestionPoolSelectionId,
    QuestionResponseId, StudentRecordId,
};

/// Answer-free, server-authorized navigation state for an issued Assessment
/// Attempt. This is a projection, never a mutable "current question" record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentAssessmentAttemptProgress {
    pub assessment_attempt: AssessmentAttemptId,
    pub question_count: u32,
    pub recommended_position: Option<u32>,
    pub positions: Vec<StudentAssessmentAttemptPosition>,
}

/// One 1-based issued position in an answer-free Student navigation projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentAssessmentAttemptPosition {
    pub position: u32,
    pub response_state: StudentAssessmentAttemptResponseState,
}

/// The only persistence states exposed by Assessment navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudentAssessmentAttemptResponseState {
    Unanswered,
    Saved,
    Submitted,
    Closed,
}

/// A timestamp supplied by the server as Unix milliseconds.
///
/// The value is carried rather than read from a process clock. PostgreSQL is
/// the authoritative clock when these records are created or transitioned.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Wraps server-supplied Unix milliseconds.
    pub fn from_unix_millis(value: i64) -> Self {
        Self(value)
    }

    /// Returns the server-supplied Unix millisecond value.
    pub fn as_unix_millis(&self) -> i64 {
        self.0
    }
}

/// One exact Question Pool Item selected in delivery order.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPoolSelectedItem {
    /// Pool whose current membership supplied this selection.
    pub question_pool_id: crate::QuestionId,
    /// Pool Edit Number current when this member was selected. Not a historical object.
    pub question_pool_edit_number: crate::QuestionPoolEditNumber,
    /// Zero-based position in that Pool's member list at selection.
    pub member_position: u32,
    /// Exact Published Question Revision delivered to the Student.
    pub question_revision: QuestionRevisionTuple,
}

/// Immutable Question Pool result for one Assessment Attempt and one Question Pool Assessment Entry.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPoolSelection {
    /// Durable Question Pool Selection identity.
    pub id: QuestionPoolSelectionId,
    /// Assessment Attempt that owns this selected Question Pool Item set.
    pub assessment_attempt: AssessmentAttemptId,
    /// Question Pool Assessment Entry that supplied the Question Pool Items.
    pub question_pool_assessment_entry: AssessmentEntryId,
    /// Pool whose current membership was sampled.
    pub question_pool_id: crate::QuestionId,
    /// Pool Edit Number current when these items were selected. Not a historical object.
    pub question_pool_edit_number: crate::QuestionPoolEditNumber,
    /// Database-authoritative time at which the server selected these entries.
    pub created_at: Timestamp,
    /// Exact selected Question Pool Items in their frozen delivery order.
    pub selected_items: Vec<QuestionPoolSelectedItem>,
}

/// Immutable question selection and issued order for one Assessment Attempt.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedQuestion {
    /// Durable issued-question identity.
    pub id: IssuedQuestionId,
    /// Assessment Attempt whose future sequencing is frozen by this record.
    pub assessment_attempt: AssessmentAttemptId,
    /// Stable fixed-question or Question Pool Assessment Entry identity.
    pub assessment_entry: AssessmentEntryId,
    /// Entry index in the Assessment Content when the Assessment Attempt began.
    pub assessment_content_entry_index: u32,
    /// Expanded zero-based delivery order inside this Assessment Attempt.
    pub issued_position: u32,
    /// Exact immutable Question Library version selected for delivery.
    pub question_revision: QuestionRevisionTuple,
    /// Pre-render static or seeded source selection. A complete reproduction
    /// descriptor begins only after rendering records its parameter checksum.
    pub source_selection: QuestionSourceSelection,
    /// Exact rendering and reproduction binding retained for this issue.
    pub reproduction_details: QuestionAttemptReproductionDetails,
    /// Exact Assessment-owned point value frozen when this Question was issued.
    pub point_value: AssessmentPointValue,
    /// Exact Assessment scoring treatment frozen when this Question was issued.
    pub scoring_rule: AssessmentEntryScoringRule,
    /// Whether this Issued Question may contribute to cross-course learning evidence.
    ///
    /// The value is frozen when the Assessment Attempt begins so later assessment scoring
    /// changes cannot rewrite the validity of an observed student response.
    pub question_statistics_eligibility: bool,
    /// Immutable Question Pool Selection that produced this Issued Question, if it was drawn.
    pub question_pool_selection: Option<QuestionPoolSelectionId>,
    /// Pool ID when this Issued Question was drawn from a Pool.
    pub question_pool_id: Option<crate::QuestionId>,
    /// Pool Edit Number current at selection. Not a historical membership object.
    pub question_pool_edit_number: Option<crate::QuestionPoolEditNumber>,
    /// Zero-based Pool member position at selection, if drawn from a Pool.
    pub question_pool_member_position: Option<u32>,
}

/// Server-recorded timing inputs for one issued question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAttemptTiming {
    /// Server time at which the question became available.
    pub issued_at: Timestamp,
    /// Server-owned base deadline before authorized pauses, or `None` when untimed.
    pub deadline: Option<Timestamp>,
    /// Server time at which whole-Assessment finalization captured this response.
    pub finalized_at: Option<Timestamp>,
}

/// Current operational state of one issued Question Attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionAttemptState {
    /// The Student may still save a response.
    Open,
    /// The server accepted one finalized Question response.
    ResponseFinalized,
    /// The effective deadline closed this Question Attempt without a finalized response.
    ClosedAtDeadline,
}

/// One immutable accepted Student Response and its current grading result.
///
/// The containing Question Attempt supplies the exact issue-time reproduction details
/// that the grading result reproduces. A Question Attempt has at most one
/// finalized Question response, captured only by whole-Assessment submission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionResponse {
    /// Durable finalized Question response identity.
    pub id: QuestionResponseId,
    /// Exact Question Attempt that accepted this response.
    pub question_attempt: QuestionAttemptId,
    /// Immutable Student Response accepted by the server.
    pub response: StudentResponse,
    /// Server time when whole-Assessment submission finalized this response.
    pub finalized_at: Timestamp,
    /// Present only after grading produced a result for this finalized response.
    pub grading_result: Option<GradingResult>,
}

/// Exact Question Backend Version recorded with one Question Attempt.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionBackendVersion {
    /// Stable Question Backend implementation name.
    pub name: String,
    /// Exact Question Backend software version.
    pub version: String,
}

/// Exact Question Grader Version recorded with one Question Attempt.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionGraderVersion {
    /// Stable Question Grader implementation name.
    pub name: String,
    /// Exact Question Grader software version.
    pub version: String,
}

/// Exact Question Renderer Version recorded with one Question Attempt.
///
/// Question Renderer Version has a distinct role from the Question Backend and
/// Question Grader versions that the same attempt also records.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionRendererVersion {
    /// Stable Question Renderer implementation name.
    pub name: String,
    /// Exact Question Renderer software version.
    pub version: String,
}

/// Versions and object identities required to reproduce one Question Attempt.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAttemptReproductionDetails {
    /// Question Backend that loaded and interpreted the question.
    pub backend: QuestionBackendVersion,
    /// Renderer used to produce the typed Question Content Blocks, when the backend has one.
    pub renderer_version: Option<QuestionRendererVersion>,
    /// Exact source object, when the backend stores source bytes.
    pub source_object_id: Option<ObjectId>,
    /// SHA-256 integrity evidence for the exact source object.
    pub source_object_checksum: Option<SourceObjectChecksum>,
    /// Objects referenced by the rendered question.
    pub asset_objects: Vec<ObjectId>,
    /// Server-only Question Grader that produced the result.
    pub grader: QuestionGraderVersion,
    /// SHA-256 of the rendered question delivered for this attempt.
    pub rendered_question_sha256: String,
}

/// Immutable issued-presentation capability recorded inside the checksummed attempt payload.
///
/// The database keeps the corresponding private presentation and grading
/// payloads in dedicated protected columns. This tag binds their required or
/// not-applicable shape to the attempt itself, so a damaged column cannot
/// downgrade a PLE Question JSON or WeBWorK attempt into a current Question Library recovery path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IssuedAttemptCapability {
    /// A browser-safe `QuestionPresentation` with no format-specific
    /// private first-grade contract.
    QuestionPresentation,
    /// A PLE Question JSON presentation and its required private grading contract.
    PleQuestionJsonPresentation,
    /// The WeBWorK capability requires a backend-owned document and opaque Student Response.
    WebworkPresentation,
    /// iMathAS session and launch lifecycle state without a format-specific
    /// private grading contract. Student delivery remains a Question Presentation.
    NotApplicable,
}

/// One server-issued try under an exact Issued Question.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAttempt {
    /// Durable question-attempt identity.
    pub id: QuestionAttemptId,
    /// Immutable delivered Question that owns this attempt.
    pub issued_question: IssuedQuestionId,
    /// Explicit static or seeded facts used to reproduce this Question Attempt.
    /// This server model is never a browser wire contract.
    pub reproduction: QuestionReproduction,
    /// Immutable accepted Student Response, when the server accepted one.
    pub finalized_response: Option<QuestionResponse>,
    /// Current operational state, independent of retained response evidence.
    pub state: QuestionAttemptState,
    /// Server-owned timing record.
    pub timing: QuestionAttemptTiming,
    /// Exact reproduction details required to reproduce this Question Attempt.
    #[serde(skip_serializing)]
    pub reproduction_details: QuestionAttemptReproductionDetails,
    /// Checksummed immutable capability for the protected issuance payloads.
    pub issued_capability: IssuedAttemptCapability,
}

/// Answer-free Student read of one Question Attempt.
///
/// The server constructs this from the durable Question Attempt after it has
/// applied the Student's disclosure and scoring policy. Reproduction details
/// and generated-parameter evidence intentionally have no representation here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentQuestionAttemptView {
    /// Durable Question Attempt identity.
    pub id: QuestionAttemptId,
    /// Immutable delivered Question that owns this attempt.
    pub issued_question: IssuedQuestionId,
    /// Immutable accepted Student Response, when the server accepted one.
    pub finalized_response: Option<QuestionResponse>,
    /// Current operational state.
    pub state: QuestionAttemptState,
    /// Student-visible timing record.
    pub timing: QuestionAttemptTiming,
    /// Checksummed immutable capability for the protected issuance payloads.
    pub issued_capability: IssuedAttemptCapability,
}

impl From<&QuestionAttempt> for StudentQuestionAttemptView {
    fn from(attempt: &QuestionAttempt) -> Self {
        Self {
            id: attempt.id,
            issued_question: attempt.issued_question,
            finalized_response: attempt.finalized_response.clone(),
            state: attempt.state,
            timing: attempt.timing,
            issued_capability: attempt.issued_capability,
        }
    }
}

/// Compact Assessment Progress Record read by course pages and the Gradebook.
///
/// Historical Assessment Attempts remain separate. Updating this Assessment Progress Record from the same
/// Assessment Attempt transition lets storage commit the history and progress atomically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AssessmentProgressRecord {
    /// Student Record summarized by this view.
    pub student_record: StudentRecordId,
    /// Assessment summarized by this view.
    pub assessment: AssessmentId,
    /// Number of completed Assessment Attempts, including continued Student work.
    pub completed_assessment_attempt_count: u32,
    /// Number of Question Attempts recorded across all Assessment Attempts.
    pub total_question_attempts: u64,
    /// Latest server-supplied Student Work timestamp.
    pub last_activity_at: Option<Timestamp>,
}

/// Browser-safe status of the Student's Assessment Grade.
///
/// This is a presentation state, not an authorization input.  The server
/// derives it from the current assessment disclosure policy and never sends
/// the policy, clock, or Student Record to the browser for inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentGradeScoreState {
    /// The Student has not submitted any response for this assessment.
    NoActivity,
    /// The Student has activity, but the current policy withholds scores.
    Withheld,
    /// The current policy permits score disclosure.
    Available,
}

/// Key-free Student Assessment Grade for one Assessment.
///
/// It deliberately excludes the Student Record and Assessment identifiers
/// carried by [`AssessmentGrade`]. The server derives disclosure and sends no
/// score totals while the current Assessment settings withhold them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StudentAssessmentGrade {
    /// Whether aggregate score values are absent because there is no submitted
    /// response, are currently withheld, or are available for display.
    pub score_state: AssessmentGradeScoreState,
    /// Current freshness and visibility of the assessment's computed scores.
    pub assessment_scoring_state: crate::AssessmentScoringState,
    /// Score selected by the assessment's grade policy when available.
    pub current_score: Option<f64>,
    /// Highest completed Assessment Attempt score when available.
    pub best_score: Option<f64>,
    /// Most recently completed Assessment Attempt score when available.
    pub latest_score: Option<f64>,
    /// Current anonymous class statistics when the assessment policy permits
    /// their disclosure. Absent means the server withholds Class Statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_statistics: Option<crate::ClassStatistics>,
}

/// Key-free Student Assessment activity facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AssessmentProgress {
    /// Number of completed Assessment Attempts, including continued Student work.
    pub completed_assessment_attempt_count: u32,
    /// Number of Question Attempts recorded across all Assessment Attempts.
    pub total_question_attempts: u64,
    /// Latest server-recorded Student Work time, if any.
    pub last_activity_at: Option<Timestamp>,
}

/// Key-free Student response that keeps Assessment activity and grade facts separate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StudentAssessmentProgress {
    /// Student-safe activity facts for this Assessment.
    pub assessment_progress: AssessmentProgress,
    /// Student-safe disclosed grade facts for this Assessment.
    pub student_assessment_grade: StudentAssessmentGrade,
}

impl StudentAssessmentGrade {
    /// Projects an entitled Student's Assessment Grade before the first durable
    /// educational receipt exists. Reading the grade must not create an
    /// Assessment Attempt merely to represent the valid no-Student-work state.
    pub fn no_activity(assessment_scoring_state: crate::AssessmentScoringState) -> Self {
        Self {
            score_state: AssessmentGradeScoreState::NoActivity,
            assessment_scoring_state,
            current_score: None,
            best_score: None,
            latest_score: None,
            class_statistics: None,
        }
    }

    /// Projects the internal Assessment Grade after the server has made its disclosure
    /// decision. No-activity takes precedence over the disclosure setting.
    pub fn from_assessment_grade(
        grade: &AssessmentGrade,
        progress: &AssessmentProgressRecord,
        score_disclosed: bool,
        assessment_scoring_state: crate::AssessmentScoringState,
    ) -> Self {
        let score_state = if progress.total_question_attempts == 0 {
            AssessmentGradeScoreState::NoActivity
        } else if score_disclosed {
            AssessmentGradeScoreState::Available
        } else {
            AssessmentGradeScoreState::Withheld
        };
        let scores = matches!(score_state, AssessmentGradeScoreState::Available)
            && matches!(
                assessment_scoring_state,
                crate::AssessmentScoringState::Current
            );
        Self {
            score_state,
            assessment_scoring_state,
            current_score: scores.then_some(grade.current_score).flatten(),
            best_score: scores.then_some(grade.best_score).flatten(),
            latest_score: scores.then_some(grade.latest_score).flatten(),
            class_statistics: None,
        }
    }
}

impl From<&AssessmentProgressRecord> for AssessmentProgress {
    fn from(progress: &AssessmentProgressRecord) -> Self {
        Self {
            completed_assessment_attempt_count: progress.completed_assessment_attempt_count,
            total_question_attempts: progress.total_question_attempts,
            last_activity_at: progress.last_activity_at,
        }
    }
}

impl AssessmentProgressRecord {
    /// Creates the empty Student Work view for one Student Record and Assessment.
    pub fn empty(student_record: StudentRecordId, assessment: AssessmentId) -> Self {
        Self {
            student_record,
            assessment,
            completed_assessment_attempt_count: 0,
            total_question_attempts: 0,
            last_activity_at: None,
        }
    }
}

impl AssessmentGrade {
    /// Creates the empty selected-grade record for one Student Record and Assessment.
    pub fn empty(student_record: StudentRecordId, assessment: AssessmentId) -> Self {
        Self {
            student_record,
            assessment,
            first_completed_at: None,
            current_assessment_attempt: None,
            current_score: None,
            best_assessment_attempt: None,
            best_score: None,
            latest_assessment_attempt: None,
            latest_score: None,
        }
    }
}

#[cfg(test)]
#[path = "student_work/model_tests.rs"]
mod tests;
