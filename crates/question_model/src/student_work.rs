//! Course-owned Student Record, Assignment Attempt, Issued Question, and
//! Question Attempt records (WP-C3, MOD-ACTIVITY).
//!
//! Completion of one Assignment Attempt does not end a Student Record. A
//! Student can begin another Assignment Attempt when its explicit continuation
//! rules permit it, and each Assignment Attempt owns its Issued Questions and
//! Question Attempts. Historical completion remains immutable.
//!
//! These are educational records. Their exact Course, Student Record,
//! Assignment, and Issued Question relationships provide authorization scope;
//! an installation-wide Account is the global product identity.

use serde::{Deserialize, Serialize};
#[cfg(test)]
use uuid::Uuid;

mod source_object_checksum;
#[cfg(test)]
#[path = "student_work/source_object_checksum_tests.rs"]
mod source_object_checksum_tests;

pub use source_object_checksum::{SourceObjectChecksum, SourceObjectChecksumError};

use crate::QuestionRevisionReference;
use crate::assignment::{AssignmentEntryScoringRule, AssignmentPointValue};
use crate::assignment_activity_rules::{AssignmentQuestionVariationRule, QuestionPoolReuseRule};
use crate::generation::{QuestionGeneratorReference, QuestionSeed};
use crate::identity::ObjectId;
use crate::response::StudentResponse;
use crate::{AssignmentAttemptReference, AssignmentRevisionReference};

mod identifiers;

pub use identifiers::{
    AccommodationId, AssignmentAttemptId, AssignmentEntryId, AssignmentId, CourseId,
    CourseMembershipId, IssuedQuestionId, QuestionAttemptId, QuestionPoolItemId,
    QuestionPoolSelectionId, QuestionSubmissionId, StudentRecordId,
};

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

/// Authoritative completion state of one Assignment Attempt.
///
/// Successor availability is deliberately separate: an Assignment Attempt can have no next
/// attempt because it completed or because it exhausted its attempt policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentAttemptCompletion {
    /// The Assignment Attempt has not satisfied its assignment completion requirement.
    InProgress,
    /// The Assignment Attempt has satisfied its assignment completion requirement.
    Completed,
}

/// One pass through an assignment.
///
/// There is deliberately no stored `complete` boolean. The domain derives
/// within-Assignment-Attempt completion from current question states, then records the
/// resulting completion timestamp and score as one transition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentAttempt {
    /// Durable Assignment Attempt identity.
    pub id: AssignmentAttemptId,
    /// Stable typed locator used in application navigation.
    pub reference: AssignmentAttemptReference,
    /// Student Record that owns this Assignment Attempt.
    pub student_record: StudentRecordId,
    /// Assignment that this Student Record attempts.
    pub assignment: AssignmentId,
    /// Exact Released Assignment Revision expanded into this Assignment Attempt.
    ///
    /// The stable Assignment identity has immutable revisions; this reference preserves
    /// the immutable authored definition and delivery rules used for this
    /// Student's work.
    pub assignment_revision: AssignmentRevisionReference,
    /// One-based attempt number for this Student Record and Assignment.
    pub attempt_number: u32,
    /// Server time at which the Assignment Attempt began.
    pub started_at: Timestamp,
    /// Server time at which derived completion was recorded, if complete.
    pub completed_at: Option<Timestamp>,
    /// Score fraction recorded on completion, if complete.
    pub score: Option<f64>,
    /// Question Pool Reuse Rule applied when this Assignment Attempt was issued.
    pub question_pool_reuse_rule: QuestionPoolReuseRule,
    /// Question Variation Rule applied when this Assignment Attempt was issued.
    pub question_variation_rule: AssignmentQuestionVariationRule,
}

/// The policy-selected course result for one Student Record and Assignment.
///
/// This record owns selected-score pointers only. Immutable Student Work remains
/// under Assignment Attempts and their Issued Questions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentGrade {
    /// Student Record whose course result this is.
    pub student_record: StudentRecordId,
    /// Assignment whose policy selected this result.
    pub assignment: AssignmentId,
    /// First time an Assignment Attempt satisfied completion.
    pub first_completed_at: Option<Timestamp>,
    /// Assignment Attempt currently selected by the grade rule.
    pub current_assignment_attempt: Option<AssignmentAttemptId>,
    /// Highest-scoring completed Assignment Attempt.
    pub best_assignment_attempt: Option<AssignmentAttemptId>,
}

impl AssignmentAttempt {
    /// Returns the completion state recorded by the authoritative Assignment Attempt projection.
    pub fn completion(&self) -> AssignmentAttemptCompletion {
        if self.completed_at.is_some() {
            AssignmentAttemptCompletion::Completed
        } else {
            AssignmentAttemptCompletion::InProgress
        }
    }
}

/// One exact Question Pool Item selected in delivery order.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPoolSelectedItem {
    /// Stable identity of the Item in its source Question Pool Assignment Entry.
    pub question_pool_item: QuestionPoolItemId,
    /// Exact immutable Question Revision selected for delivery.
    pub reference: QuestionRevisionReference,
}

/// Immutable Question Pool result for one Assignment Attempt and one Question Pool Assignment Entry.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPoolSelection {
    /// Durable Question Pool Selection identity.
    pub id: QuestionPoolSelectionId,
    /// Assignment Attempt that owns this selected Question Pool Item set.
    pub assignment_attempt: AssignmentAttemptId,
    /// Question Pool Assignment Entry that supplied the Items.
    pub question_pool_assignment_entry: AssignmentEntryId,
    /// Database-authoritative time at which the server selected these entries.
    pub created_at: Timestamp,
    /// Number of exact entries selected for this Assignment Attempt.
    ///
    /// This repeats the selected-entry-row cardinality so storage can reject an
    /// incomplete Selection at transaction commit without consulting mutable
    /// Assignment content.
    pub selected_question_count: u32,
    /// Earlier Selection whose exact entries this later Assignment Attempt retained.
    ///
    /// A reused Selection is still an immutable result owned by this Assignment
    /// Attempt. This link preserves the reason its selected-entry membership repeats
    /// without treating the earlier Attempt's record as mutable shared state.
    pub reused_from_question_pool_selection: Option<QuestionPoolSelectionId>,
    /// Exact selected Items in their frozen delivery order.
    pub selected_items: Vec<QuestionPoolSelectedItem>,
}

/// A requested later-Attempt Selection does not match its earlier source.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionPoolSelectionReuseError {
    /// A Question Pool Selection can only be retained for the same Assignment Entry.
    DifferentQuestionPoolAssignmentEntry,
    /// A Selection cannot use itself as an earlier Assignment Attempt's source.
    SameAssignmentAttempt,
}

impl std::fmt::Display for QuestionPoolSelectionReuseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DifferentQuestionPoolAssignmentEntry => formatter
                .write_str("Question Pool Selection reuse requires the same Assignment Entry"),
            Self::SameAssignmentAttempt => formatter
                .write_str("Question Pool Selection reuse requires a later Assignment Attempt"),
        }
    }
}

impl std::error::Error for QuestionPoolSelectionReuseError {}

impl QuestionPoolSelection {
    /// Copies this immutable Question Pool Selection into a later Assignment Attempt.
    ///
    /// Storage additionally verifies that both attempts belong to the same
    /// Student and Assignment, and that the target attempt number is later.
    /// Keeping those relational checks in storage prevents a caller from
    /// supplying an unrelated Student Work identifier.
    pub fn reused_for_later_attempt(
        &self,
        id: QuestionPoolSelectionId,
        assignment_attempt: AssignmentAttemptId,
        question_pool_assignment_entry: AssignmentEntryId,
        created_at: Timestamp,
    ) -> Result<Self, QuestionPoolSelectionReuseError> {
        if question_pool_assignment_entry != self.question_pool_assignment_entry {
            return Err(QuestionPoolSelectionReuseError::DifferentQuestionPoolAssignmentEntry);
        }
        if assignment_attempt == self.assignment_attempt {
            return Err(QuestionPoolSelectionReuseError::SameAssignmentAttempt);
        }
        Ok(Self {
            id,
            assignment_attempt,
            question_pool_assignment_entry,
            created_at,
            selected_question_count: self.selected_question_count,
            reused_from_question_pool_selection: Some(self.id),
            selected_items: self.selected_items.clone(),
        })
    }
}

/// Immutable question selection and issued order for one Assignment Attempt.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedQuestion {
    /// Durable issued-question identity.
    pub id: IssuedQuestionId,
    /// Assignment Attempt whose future sequencing is frozen by this record.
    pub assignment_attempt: AssignmentAttemptId,
    /// Stable fixed-question or Question Pool Assignment Entry identity.
    pub assignment_entry: AssignmentEntryId,
    /// Entry index in the Assignment Content when the Assignment Attempt began.
    pub assignment_content_entry_index: u32,
    /// Expanded zero-based delivery order inside this Assignment Attempt.
    pub issued_position: u32,
    /// Exact immutable Question Library version selected for delivery.
    pub reference: QuestionRevisionReference,
    /// Exact Assignment-owned point value frozen when this Question was issued.
    pub point_value: AssignmentPointValue,
    /// Exact Assignment scoring treatment frozen when this Question was issued.
    pub scoring_rule: AssignmentEntryScoringRule,
    /// Whether this issued item may contribute to cross-course learning evidence.
    ///
    /// The value is frozen when the Assignment Attempt begins so later assignment scoring
    /// changes cannot rewrite the validity of an observed student response.
    pub statistics_eligible: bool,
    /// Immutable Question Pool Selection that produced this item, if it was drawn.
    pub question_pool_selection: Option<QuestionPoolSelectionId>,
    /// Exact Question Pool Item in that Question Pool Selection, if it was drawn.
    pub question_pool_item: Option<QuestionPoolItemId>,
}

/// Server-recorded timing inputs for one issued question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAttemptTiming {
    /// Server time at which the question became available.
    pub issued_at: Timestamp,
    /// Server-owned base deadline before authorized pauses, or `None` when untimed.
    pub deadline: Option<Timestamp>,
    /// Server time at which the response arrived, if submitted.
    pub submitted_at: Option<Timestamp>,
}

/// Current operational state of one issued Question Attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionAttemptState {
    /// The student may still submit a response.
    Open,
    /// The server accepted one Question Submission.
    SubmissionAccepted,
    /// The effective deadline closed this Question Attempt without a submission.
    ClosedAtDeadline,
}

/// A grading result without an answer key.
///
/// The server may disclose this according to the assignment feedback policy;
/// the correct response and Question Grader code remain in `grading`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradingResult {
    /// Whether the submitted response was correct.
    pub correct: bool,
    /// Points awarded by server-side grading.
    pub points_earned: f64,
    /// Maximum points available for this question.
    pub points_possible: f64,
}

/// One immutable accepted Student Response and its current grading result.
///
/// The containing Question Attempt supplies the exact issue-time reproduction details
/// that the grading result reproduces. A Question Attempt has at most one
/// accepted Question Submission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionSubmission {
    /// Durable Question Submission identity.
    pub id: QuestionSubmissionId,
    /// Exact Question Attempt that accepted this response.
    pub question_attempt: QuestionAttemptId,
    /// Immutable Student Response accepted by the server.
    pub response: StudentResponse,
    /// Server time when the response was accepted.
    pub submitted_at: Timestamp,
    /// Present only after grading produced a result for this submission.
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

/// Source Object Reference captured for a reproducible Question Attempt.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceObjectReference {
    /// Immutable Object Record containing the source bytes.
    pub object: ObjectId,
}

/// Versions and object identities required to reproduce one Question Attempt.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAttemptReproductionDetails {
    /// Question Backend that loaded and interpreted the question.
    pub backend: QuestionBackendVersion,
    /// Renderer used for supplied markup, when the backend has one.
    pub renderer_version: Option<QuestionRendererVersion>,
    /// Generator used for parameterized content, when the backend has one.
    pub generator: Option<QuestionGeneratorReference>,
    /// Exact source object, when the backend stores source bytes.
    pub source_object_reference: Option<SourceObjectReference>,
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
    PresentationEnvelope,
    /// A PLE Question JSON presentation and its required private grading contract.
    PleQuestionJsonPresentation,
    /// A WeBWorK presentation, immutable private definition, and replay map.
    WebworkPresentation,
    /// A QTI presentation and its copied per-attempt private grading payload.
    ///
    /// This is distinct from the generic presentation tag so loss of the
    /// opaque contract fails closed instead of inviting a Question Library lookup.
    QtiPresentation,
    /// A Question Backend that intentionally issues no `QuestionPresentation`.
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
    /// Question Seed used to regenerate the exact Question Variation.
    pub seed: QuestionSeed,
    /// SHA-256 of the generated parameters.
    #[serde(skip_serializing)]
    pub parameter_hash: String,
    /// Immutable accepted Student Response, when the server accepted one.
    pub submission: Option<QuestionSubmission>,
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
    /// Question Seed used by the issued presentation binding.
    pub seed: QuestionSeed,
    /// Immutable accepted Student Response, when the server accepted one.
    pub submission: Option<QuestionSubmission>,
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
            seed: attempt.seed,
            submission: attempt.submission.clone(),
            state: attempt.state,
            timing: attempt.timing,
            issued_capability: attempt.issued_capability,
        }
    }
}

/// Compact projection read by course pages and the gradebook.
///
/// Historical Assignment Attempts remain separate. Updating this projection from the same
/// Assignment Attempt transition lets storage commit the history and summary atomically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentProgressRecord {
    /// Student Record summarized by this view.
    pub student_record: StudentRecordId,
    /// Assignment summarized by this view.
    pub assignment: AssignmentId,
    /// Score selected by the assignment's grade policy.
    pub current_score: Option<f64>,
    /// Highest completed Assignment Attempt score seen so far.
    pub best_score: Option<f64>,
    /// Most recently completed Assignment Attempt score.
    pub latest_score: Option<f64>,
    /// Number of completed Assignment Attempts, including continued Student work.
    pub completed_assignment_attempt_count: u32,
    /// Number of Question Attempts recorded across all Assignment Attempts.
    pub total_question_attempts: u64,
    /// Latest server-supplied Student Work timestamp.
    pub last_activity_at: Option<Timestamp>,
}

/// Browser-safe status of the Student's aggregate assignment score.
///
/// This is a presentation state, not an authorization input.  The server
/// derives it from the current assignment disclosure policy and never sends
/// the policy, clock, or Student Record to the browser for inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentProgressScoreState {
    /// The Student has not submitted any response for this assignment.
    NoActivity,
    /// The Student has activity, but the current policy withholds scores.
    Withheld,
    /// The current policy permits score disclosure.
    Available,
}

/// Key-free Student projection of an assignment's aggregate progress.
///
/// It deliberately excludes the Student Record and Assignment identifiers
/// carried by [`AssignmentProgressRecord`]. Browser routes use this type instead of the
/// storage projection so score totals are omitted while withheld.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AssignmentProgress {
    /// Whether aggregate score values are absent because there is no submitted
    /// response, are currently withheld, or are available for display.
    pub score_state: AssignmentProgressScoreState,
    /// Current freshness and visibility of the assignment's computed scores.
    pub assignment_scoring_state: crate::AssignmentScoringState,
    /// Score selected by the assignment's grade policy when available.
    pub current_score: Option<f64>,
    /// Highest completed Assignment Attempt score when available.
    pub best_score: Option<f64>,
    /// Most recently completed Assignment Attempt score when available.
    pub latest_score: Option<f64>,
    /// Number of completed Assignment Attempts. This is not a score total.
    pub completed_assignment_attempt_count: u32,
    /// Number of recorded responses. This is not a score total.
    pub total_question_attempts: u64,
    /// Latest server-recorded activity time, if any.
    pub last_activity_at: Option<Timestamp>,
    /// Current anonymous class statistics when the assignment policy permits
    /// their disclosure. Absent means the server withholds this projection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_statistics: Option<crate::StudentClassStatistics>,
}

impl AssignmentProgress {
    /// Projects an entitled Student's assignment before the first durable
    /// educational receipt exists. Reading progress must not create an
    /// Assignment Attempt merely to represent the valid no-Student-work state.
    pub fn no_activity(assignment_scoring_state: crate::AssignmentScoringState) -> Self {
        Self {
            score_state: AssignmentProgressScoreState::NoActivity,
            assignment_scoring_state,
            current_score: None,
            best_score: None,
            latest_score: None,
            completed_assignment_attempt_count: 0,
            total_question_attempts: 0,
            last_activity_at: None,
            class_statistics: None,
        }
    }

    /// Projects the internal summary after the server has made its disclosure
    /// decision. No-activity takes precedence over the disclosure setting.
    pub fn from_summary(
        summary: &AssignmentProgressRecord,
        score_disclosed: bool,
        assignment_scoring_state: crate::AssignmentScoringState,
    ) -> Self {
        let score_state = if summary.total_question_attempts == 0 {
            AssignmentProgressScoreState::NoActivity
        } else if score_disclosed {
            AssignmentProgressScoreState::Available
        } else {
            AssignmentProgressScoreState::Withheld
        };
        let scores = matches!(score_state, AssignmentProgressScoreState::Available)
            && matches!(
                assignment_scoring_state,
                crate::AssignmentScoringState::Current
            );
        Self {
            score_state,
            assignment_scoring_state,
            current_score: scores.then_some(summary.current_score).flatten(),
            best_score: scores.then_some(summary.best_score).flatten(),
            latest_score: scores.then_some(summary.latest_score).flatten(),
            completed_assignment_attempt_count: summary.completed_assignment_attempt_count,
            total_question_attempts: summary.total_question_attempts,
            last_activity_at: summary.last_activity_at,
            class_statistics: None,
        }
    }
}

impl AssignmentProgressRecord {
    /// Creates the empty Student Work view for one Student Record and Assignment.
    pub fn empty(student_record: StudentRecordId, assignment: AssignmentId) -> Self {
        Self {
            student_record,
            assignment,
            current_score: None,
            best_score: None,
            latest_score: None,
            completed_assignment_attempt_count: 0,
            total_question_attempts: 0,
            last_activity_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_attempt_binds_a_student_record_and_released_assignment_revision() {
        let attempt = AssignmentAttempt {
            id: AssignmentAttemptId::from_uuid(Uuid::from_u128(1)),
            reference: AssignmentAttemptReference::new(1).expect("valid attempt reference"),
            student_record: StudentRecordId::from_uuid(Uuid::from_u128(2)),
            assignment: AssignmentId::from_uuid(Uuid::from_u128(3)),
            assignment_revision: AssignmentRevisionReference {
                assignment: crate::AssignmentReference::new(3).expect("valid assignment reference"),
                revision_number: crate::AssignmentRevisionNumber::INITIAL,
            },
            attempt_number: 1,
            started_at: Timestamp::from_unix_millis(1_000),
            completed_at: None,
            score: None,
            question_pool_reuse_rule: QuestionPoolReuseRule::ReuseSelection,
            question_variation_rule: AssignmentQuestionVariationRule::NewVariation,
        };

        assert_eq!(attempt.student_record.as_uuid(), Uuid::from_u128(2));
        assert_eq!(attempt.assignment.as_uuid(), Uuid::from_u128(3));
        assert_eq!(attempt.assignment_revision.revision_number.value(), 1);
    }

    #[test]
    fn question_pool_selection_retains_exact_entries_and_issued_question_link() {
        let selection_id = QuestionPoolSelectionId::from_uuid(Uuid::from_u128(10));
        let question_pool_item_id = QuestionPoolItemId::from_uuid(Uuid::from_u128(11));
        let reference = QuestionRevisionReference {
            question_id: "123-4567".parse().expect("valid Question ID"),
            revision_number: crate::QuestionRevisionNumber::new(1).expect("positive version"),
        };
        let selection = QuestionPoolSelection {
            id: selection_id,
            assignment_attempt: AssignmentAttemptId::from_uuid(Uuid::from_u128(12)),
            question_pool_assignment_entry: AssignmentEntryId::from_uuid(Uuid::from_u128(13)),
            created_at: Timestamp::from_unix_millis(1_000),
            selected_question_count: 1,
            reused_from_question_pool_selection: None,
            selected_items: vec![QuestionPoolSelectedItem {
                question_pool_item: question_pool_item_id,
                reference: reference.clone(),
            }],
        };
        let issued_question = IssuedQuestion {
            id: IssuedQuestionId::from_uuid(Uuid::from_u128(14)),
            assignment_attempt: selection.assignment_attempt,
            assignment_entry: selection.question_pool_assignment_entry,
            assignment_content_entry_index: 0,
            issued_position: 0,
            reference,
            point_value: crate::AssignmentPointValue::from_whole(1),
            scoring_rule: crate::AssignmentEntryScoringRule::Normal,
            statistics_eligible: true,
            question_pool_selection: Some(selection_id),
            question_pool_item: Some(question_pool_item_id),
        };

        assert_eq!(selection.selected_items.len(), 1);
        assert_eq!(issued_question.question_pool_selection, Some(selection.id));
        assert_eq!(
            issued_question.question_pool_item,
            Some(question_pool_item_id)
        );

        let reused = selection
            .reused_for_later_attempt(
                QuestionPoolSelectionId::from_uuid(Uuid::from_u128(15)),
                AssignmentAttemptId::from_uuid(Uuid::from_u128(16)),
                selection.question_pool_assignment_entry,
                Timestamp::from_unix_millis(2_000),
            )
            .expect("same Question Pool may retain its exact entries");
        assert_eq!(
            reused.reused_from_question_pool_selection,
            Some(selection.id)
        );
        assert_eq!(reused.selected_items, selection.selected_items);
    }

    #[test]
    fn question_pool_selection_refuses_reuse_for_a_different_entry_or_same_attempt() {
        let selection = QuestionPoolSelection {
            id: QuestionPoolSelectionId::from_uuid(Uuid::from_u128(1)),
            assignment_attempt: AssignmentAttemptId::from_uuid(Uuid::from_u128(2)),
            question_pool_assignment_entry: AssignmentEntryId::from_uuid(Uuid::from_u128(3)),
            created_at: Timestamp::from_unix_millis(1_000),
            selected_question_count: 1,
            reused_from_question_pool_selection: None,
            selected_items: Vec::new(),
        };

        assert_eq!(
            selection.reused_for_later_attempt(
                QuestionPoolSelectionId::from_uuid(Uuid::from_u128(4)),
                selection.assignment_attempt,
                selection.question_pool_assignment_entry,
                Timestamp::from_unix_millis(2_000),
            ),
            Err(QuestionPoolSelectionReuseError::SameAssignmentAttempt),
        );
        assert_eq!(
            selection.reused_for_later_attempt(
                QuestionPoolSelectionId::from_uuid(Uuid::from_u128(4)),
                AssignmentAttemptId::from_uuid(Uuid::from_u128(5)),
                AssignmentEntryId::from_uuid(Uuid::from_u128(6)),
                Timestamp::from_unix_millis(2_000),
            ),
            Err(QuestionPoolSelectionReuseError::DifferentQuestionPoolAssignmentEntry),
        );
    }

    #[test]
    fn student_progress_distinguishes_no_activity_withheld_and_available_scores() {
        assert_eq!(
            AssignmentProgress::no_activity(crate::AssignmentScoringState::Current),
            AssignmentProgress {
                score_state: AssignmentProgressScoreState::NoActivity,
                assignment_scoring_state: crate::AssignmentScoringState::Current,
                current_score: None,
                best_score: None,
                latest_score: None,
                completed_assignment_attempt_count: 0,
                total_question_attempts: 0,
                last_activity_at: None,
                class_statistics: None,
            }
        );
        let mut summary = AssignmentProgressRecord::empty(
            StudentRecordId::from_uuid(Uuid::from_u128(2)),
            AssignmentId::from_uuid(Uuid::from_u128(3)),
        );
        assert_eq!(
            AssignmentProgress::from_summary(
                &summary,
                true,
                crate::AssignmentScoringState::Current
            )
            .score_state,
            AssignmentProgressScoreState::NoActivity
        );

        summary.total_question_attempts = 1;
        summary.current_score = Some(0.5);
        summary.best_score = Some(0.5);
        summary.latest_score = Some(0.5);
        let withheld = AssignmentProgress::from_summary(
            &summary,
            false,
            crate::AssignmentScoringState::Current,
        );
        assert_eq!(withheld.score_state, AssignmentProgressScoreState::Withheld);
        assert_eq!(
            (
                withheld.current_score,
                withheld.best_score,
                withheld.latest_score
            ),
            (None, None, None)
        );

        let available = AssignmentProgress::from_summary(
            &summary,
            true,
            crate::AssignmentScoringState::Current,
        );
        assert_eq!(
            available.score_state,
            AssignmentProgressScoreState::Available
        );
        assert_eq!(available.current_score, Some(0.5));
        assert!(available.class_statistics.is_none());
    }

    #[test]
    fn student_progress_hides_scores_while_scoring_is_not_current() {
        let mut summary = AssignmentProgressRecord::empty(
            StudentRecordId::from_uuid(Uuid::from_u128(2)),
            AssignmentId::from_uuid(Uuid::from_u128(3)),
        );
        summary.total_question_attempts = 1;
        summary.current_score = Some(0.5);
        for assignment_scoring_state in [
            crate::AssignmentScoringState::Recalculating,
            crate::AssignmentScoringState::Failed,
        ] {
            let progress =
                AssignmentProgress::from_summary(&summary, true, assignment_scoring_state);
            assert_eq!(
                progress.score_state,
                AssignmentProgressScoreState::Available
            );
            assert_eq!(progress.assignment_scoring_state, assignment_scoring_state);
            assert_eq!(progress.current_score, None);
        }
    }

    #[test]
    fn every_activity_identifier_stays_distinct_but_round_trips() {
        let raw = Uuid::from_u128(7);
        let assignment_attempt = AssignmentAttemptId::from_uuid(raw);
        let attempt = QuestionAttemptId::from_uuid(raw);

        assert_eq!(
            (assignment_attempt.as_uuid(), attempt.as_uuid()),
            (raw, raw)
        );
    }

    #[test]
    fn question_attempt_state_uses_the_closed_operational_wire_vocabulary() {
        assert_eq!(
            serde_json::to_value(QuestionAttemptState::Open).expect("open state serializes"),
            serde_json::json!("open")
        );
        assert_eq!(
            serde_json::to_value(QuestionAttemptState::SubmissionAccepted)
                .expect("accepted-submission state serializes"),
            serde_json::json!("submission_accepted")
        );
        assert_eq!(
            serde_json::to_value(QuestionAttemptState::ClosedAtDeadline)
                .expect("deadline-closed state serializes"),
            serde_json::json!("closed_at_deadline")
        );
    }

    #[test]
    fn reproduction_details_serialize_role_specific_versions() {
        let record = QuestionAttemptReproductionDetails {
            backend: QuestionBackendVersion {
                name: "ple-question-backend".to_string(),
                version: "1".to_string(),
            },
            renderer_version: None,
            generator: None,
            source_object_reference: Some(SourceObjectReference {
                object: ObjectId::from_uuid(Uuid::from_u128(7)),
            }),
            source_object_checksum: Some(
                SourceObjectChecksum::parse("a".repeat(64)).expect("canonical checksum"),
            ),
            asset_objects: Vec::new(),
            grader: QuestionGraderVersion {
                name: "generic-grader".to_string(),
                version: "1".to_string(),
            },
            rendered_question_sha256: "a".repeat(64),
        };

        let wire = serde_json::to_value(record).expect("reproduction details serialize");
        assert!(wire.get("backend").is_some());
        assert!(wire.get("grader").is_some());
        assert_eq!(
            wire["sourceObjectReference"],
            serde_json::json!({ "object": "00000000-0000-0000-0000-000000000007" })
        );
        assert_eq!(
            wire["sourceObjectChecksum"],
            serde_json::json!("a".repeat(64))
        );
        assert!(wire.get("adapter").is_none());
        assert!(wire.get("grading").is_none());
    }

    #[test]
    fn question_attempt_browser_wire_omits_reproduction_details() {
        let attempt = QuestionAttempt {
            id: QuestionAttemptId::from_uuid(Uuid::from_u128(1)),
            issued_question: IssuedQuestionId::from_uuid(Uuid::from_u128(2)),
            seed: QuestionSeed::new(3),
            parameter_hash: "a".repeat(64),
            submission: None,
            state: QuestionAttemptState::Open,
            timing: QuestionAttemptTiming {
                issued_at: Timestamp::from_unix_millis(4),
                deadline: None,
                submitted_at: None,
            },
            reproduction_details: QuestionAttemptReproductionDetails {
                backend: QuestionBackendVersion {
                    name: "ple-question-backend".to_string(),
                    version: "1".to_string(),
                },
                renderer_version: None,
                generator: None,
                source_object_reference: None,
                source_object_checksum: None,
                asset_objects: Vec::new(),
                grader: QuestionGraderVersion {
                    name: "generic-grader".to_string(),
                    version: "1".to_string(),
                },
                rendered_question_sha256: "b".repeat(64),
            },
            issued_capability: IssuedAttemptCapability::NotApplicable,
        };

        let view = StudentQuestionAttemptView::from(&attempt);
        let wire = serde_json::to_value(view).expect("Student Question Attempt View serializes");
        assert!(wire.get("parameterHash").is_none());
        assert!(wire.get("reproductionDetails").is_none());
        assert_eq!(
            wire.get("id"),
            Some(&serde_json::json!(attempt.id.to_string()))
        );
        assert_eq!(
            wire.get("issuedQuestion"),
            Some(&serde_json::json!(attempt.issued_question.to_string()))
        );
        assert!(wire.get("issuedCapability").is_some());
    }
}
