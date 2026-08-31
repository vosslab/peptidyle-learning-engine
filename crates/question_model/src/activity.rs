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
use uuid::Uuid;

use crate::AssignmentAttemptReference;
use crate::QuestionVersionReference;
use crate::assignment_activity_rules::VariationPolicy;
use crate::generation::GeneratorReference;
use crate::identity::ObjectId;
use crate::response::StudentResponse;

/// A course-owned assignment offered to students.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssignmentId(Uuid);

/// One stable current-state item within an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssignmentItemId(Uuid);

/// One random-selection group within an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssignmentSelectionGroupId(Uuid);

/// A course or section containing assignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CourseId(Uuid);

/// One durable course-local membership record.
///
/// This identity is historical evidence as well as the single current
/// membership lock target.  It is intentionally distinct from a user and a
/// student record: revocation and a later reinvitation must not rewrite a
/// receipt minted under an earlier membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CourseMembershipId(Uuid);

/// One durable Student Record in a Course Instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StudentRecordId(Uuid);

/// One direct Student Accommodation attached to an Assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccommodationId(Uuid);

/// One pass through an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssignmentAttemptId(Uuid);

/// One issued question inside a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IssuedQuestionId(Uuid);

/// One server-issued try for one Issued Question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QuestionAttemptId(Uuid);

/// Gives an activity identifier its shared storage and display behavior.
macro_rules! impl_activity_identifier {
    ($name:ident) => {
        impl $name {
            /// Wraps a UUID read from storage or an authenticated boundary.
            pub fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            /// Returns the UUID used by storage and logging.
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }

            /// Mints a fresh server-owned identifier.
            #[cfg(feature = "generate")]
            pub fn generate() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.0)
            }
        }
    };
}

impl_activity_identifier!(AssignmentId);
impl_activity_identifier!(AssignmentItemId);
impl_activity_identifier!(AssignmentSelectionGroupId);
impl_activity_identifier!(CourseId);
impl_activity_identifier!(CourseMembershipId);
impl_activity_identifier!(StudentRecordId);
impl_activity_identifier!(AccommodationId);
impl_activity_identifier!(AssignmentAttemptId);
impl_activity_identifier!(IssuedQuestionId);
impl_activity_identifier!(QuestionAttemptId);

/// A timestamp supplied by the server as Unix milliseconds.
///
/// The value is carried rather than read from a process clock. PostgreSQL is
/// the authoritative clock when these records are created or transitioned.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct ActivityTimestamp(i64);

impl ActivityTimestamp {
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
    /// One-based attempt number for this Student Record and Assignment.
    pub attempt_number: u32,
    /// Server time at which the Assignment Attempt began.
    pub started_at: ActivityTimestamp,
    /// Server time at which derived completion was recorded, if complete.
    pub completed_at: Option<ActivityTimestamp>,
    /// Score fraction recorded on completion, if complete.
    pub score: Option<f64>,
    /// Variation policy applied when this Assignment Attempt was issued.
    pub variation: VariationPolicy,
}

/// The policy-selected course result for one Student Record and Assignment.
///
/// This record owns selected-score pointers only. Immutable activity remains
/// under Assignment Attempts and their Issued Questions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentGrade {
    /// Student Record whose course result this is.
    pub student_record: StudentRecordId,
    /// Assignment whose policy selected this result.
    pub assignment: AssignmentId,
    /// First time an Assignment Attempt satisfied completion.
    pub first_completed_at: Option<ActivityTimestamp>,
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

/// Immutable question selection and issued order for one Assignment Attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedQuestion {
    /// Durable issued-question identity.
    pub id: IssuedQuestionId,
    /// Assignment Attempt whose future sequencing is frozen by this record.
    pub assignment_attempt: AssignmentAttemptId,
    /// Stable fixed-item or selection-candidate identity.
    pub assignment_item: AssignmentItemId,
    /// Position in the mutable assignment definition when the Assignment Attempt began.
    pub source_position: u32,
    /// Expanded zero-based delivery order inside this run.
    pub issued_position: u32,
    /// Exact immutable catalog version selected for delivery.
    pub reference: QuestionVersionReference,
    /// Whether this issued item may contribute to cross-course learning evidence.
    ///
    /// The value is frozen when the run begins so later assignment scoring
    /// changes cannot rewrite the validity of an observed student response.
    pub statistics_eligible: bool,
    /// Selection group that produced this item, if it was drawn.
    pub selection_group: Option<AssignmentSelectionGroupId>,
    /// Deterministic selection seed, absent for fixed items.
    pub selection_seed: Option<u64>,
}

/// Server-recorded timing inputs for one issued question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAttemptTiming {
    /// Server time at which the question became available.
    pub issued_at: ActivityTimestamp,
    /// Server-owned base deadline before authorized pauses, or `None` when untimed.
    pub deadline: Option<ActivityTimestamp>,
    /// Server time at which the response arrived, if submitted.
    pub submitted_at: Option<ActivityTimestamp>,
}

/// Current operational state of one issued question attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    /// The student may still submit a response.
    InProgress,
    /// A student or instructor submitted the current response.
    Submitted,
    /// The server submitted automatically at the effective deadline.
    AutoSubmitted,
    /// The attempt is excluded from current scoring but retained as evidence.
    Cleared,
    /// The attempt is explicitly exempt from current scoring.
    Exempt,
}

/// A grading result without an answer key.
///
/// The server may disclose this according to the assignment feedback policy;
/// the correct response and grading implementation remain in `grading`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptResult {
    /// Whether the submitted response was correct.
    pub correct: bool,
    /// Points awarded by server-side grading.
    pub points_earned: f64,
    /// Maximum points available for this question.
    pub points_possible: f64,
}

/// A named implementation and the version needed to execute it again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationVersion {
    /// Stable implementation identifier.
    pub id: String,
    /// Additive implementation version.
    pub version: String,
}

/// Source artifact identity captured for a reproducible attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceArtifact {
    /// Immutable object-store record containing the source bytes.
    pub object: ObjectId,
    /// SHA-256 of those bytes at attempt issue time.
    pub sha256: String,
}

/// Versions and object identities required to reproduce one attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptProvenance {
    /// Adapter that loaded and interpreted the question.
    pub adapter: ImplementationVersion,
    /// Renderer used for supplied markup, when the backend has one.
    pub renderer: Option<ImplementationVersion>,
    /// Generator used for parameterized content, when the backend has one.
    pub generator: Option<GeneratorReference>,
    /// Original source artifact, when the backend stores one.
    pub source_artifact: Option<SourceArtifact>,
    /// Objects referenced by the rendered question.
    pub asset_objects: Vec<ObjectId>,
    /// Server-only grading implementation that produced the result.
    pub grading: ImplementationVersion,
    /// SHA-256 of the rendered question delivered for this attempt.
    pub rendered_question_sha256: String,
}

/// Immutable family capability recorded inside the checksummed attempt payload.
///
/// The database keeps the corresponding private presentation and grading
/// payloads in dedicated protected columns. This tag binds their required or
/// not-applicable shape to the attempt itself, so a damaged column cannot
/// downgrade a flat or WeBWorK attempt into a current-catalog recovery path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IssuedAttemptCapabilityV1 {
    /// A browser-safe `PresentationEnvelopeV1` with no family-specific
    /// private first-grade contract.
    PresentationEnvelope,
    /// A native flat presentation and its required private grading contract.
    FlatPresentation,
    /// A WeBWorK presentation, immutable private definition, and replay map.
    WebworkPresentation,
    /// A QTI presentation and its copied per-attempt private grading payload.
    ///
    /// This is distinct from the generic presentation tag so loss of the
    /// opaque contract fails closed instead of inviting a catalog lookup.
    QtiPresentation,
    /// A family that intentionally issues no `PresentationEnvelopeV1`.
    NotApplicable,
}

/// One server-issued try under an exact Issued Question.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAttempt {
    /// Durable question-attempt identity.
    pub id: QuestionAttemptId,
    /// Immutable delivered Question that owns this attempt.
    pub issued_question: IssuedQuestionId,
    /// Seed used to regenerate the exact question variant.
    pub seed: u64,
    /// SHA-256 of the generated parameters.
    pub parameter_hash: String,
    /// Student response, once submitted.
    pub response: Option<StudentResponse>,
    /// Current operational state, independent of retained response evidence.
    pub status: AttemptStatus,
    /// Server grading result, once graded.
    pub result: Option<AttemptResult>,
    /// Server-owned timing record.
    pub timing: QuestionAttemptTiming,
    /// Versions and object identities required to reproduce this attempt.
    pub provenance: AttemptProvenance,
    /// Checksummed immutable capability for the protected issuance payloads.
    pub issued_capability: IssuedAttemptCapabilityV1,
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
    /// Number of completed Assignment Attempts, including continued activity.
    pub completed_assignment_attempt_count: u32,
    /// Number of Question Attempts recorded across all Assignment Attempts.
    pub total_question_attempts: u64,
    /// Latest server-supplied activity timestamp.
    pub last_activity_at: Option<ActivityTimestamp>,
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
    pub scoring_status: crate::ScoringStatus,
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
    pub last_activity_at: Option<ActivityTimestamp>,
    /// Current anonymous class statistics when the assignment policy permits
    /// their disclosure. Absent means the server withholds this projection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_statistics: Option<crate::StudentClassStatistics>,
}

impl AssignmentProgress {
    /// Projects an entitled Student's assignment before the first durable
    /// educational receipt exists. Reading progress must not create an
    /// Assignment Attempt merely to represent the valid no-activity state.
    pub fn no_activity(scoring_status: crate::ScoringStatus) -> Self {
        Self {
            score_state: AssignmentProgressScoreState::NoActivity,
            scoring_status,
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
        scoring_status: crate::ScoringStatus,
    ) -> Self {
        let score_state = if summary.total_question_attempts == 0 {
            AssignmentProgressScoreState::NoActivity
        } else if score_disclosed {
            AssignmentProgressScoreState::Available
        } else {
            AssignmentProgressScoreState::Withheld
        };
        let scores = matches!(score_state, AssignmentProgressScoreState::Available)
            && matches!(scoring_status, crate::ScoringStatus::Current);
        Self {
            score_state,
            scoring_status,
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
    /// Creates the empty activity view for one Student Record and Assignment.
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
    fn assignment_attempt_binds_a_student_record_and_assignment() {
        let attempt = AssignmentAttempt {
            id: AssignmentAttemptId::from_uuid(Uuid::from_u128(1)),
            reference: AssignmentAttemptReference::new(1).expect("valid attempt reference"),
            student_record: StudentRecordId::from_uuid(Uuid::from_u128(2)),
            assignment: AssignmentId::from_uuid(Uuid::from_u128(3)),
            attempt_number: 1,
            started_at: ActivityTimestamp::from_unix_millis(1_000),
            completed_at: None,
            score: None,
            variation: VariationPolicy::NewSeeds,
        };

        assert_eq!(attempt.student_record.as_uuid(), Uuid::from_u128(2));
        assert_eq!(attempt.assignment.as_uuid(), Uuid::from_u128(3));
    }

    #[test]
    fn student_progress_distinguishes_no_activity_withheld_and_available_scores() {
        assert_eq!(
            AssignmentProgress::no_activity(crate::ScoringStatus::Current),
            AssignmentProgress {
                score_state: AssignmentProgressScoreState::NoActivity,
                scoring_status: crate::ScoringStatus::Current,
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
            AssignmentProgress::from_summary(&summary, true, crate::ScoringStatus::Current)
                .score_state,
            AssignmentProgressScoreState::NoActivity
        );

        summary.total_question_attempts = 1;
        summary.current_score = Some(0.5);
        summary.best_score = Some(0.5);
        summary.latest_score = Some(0.5);
        let withheld =
            AssignmentProgress::from_summary(&summary, false, crate::ScoringStatus::Current);
        assert_eq!(withheld.score_state, AssignmentProgressScoreState::Withheld);
        assert_eq!(
            (
                withheld.current_score,
                withheld.best_score,
                withheld.latest_score
            ),
            (None, None, None)
        );

        let available =
            AssignmentProgress::from_summary(&summary, true, crate::ScoringStatus::Current);
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
        for scoring_status in [
            crate::ScoringStatus::Recalculating,
            crate::ScoringStatus::Failed,
        ] {
            let progress = AssignmentProgress::from_summary(&summary, true, scoring_status);
            assert_eq!(
                progress.score_state,
                AssignmentProgressScoreState::Available
            );
            assert_eq!(progress.scoring_status, scoring_status);
            assert_eq!(progress.current_score, None);
        }
    }

    #[test]
    fn every_activity_identifier_stays_distinct_but_round_trips() {
        let raw = Uuid::from_u128(7);
        let run = AssignmentAttemptId::from_uuid(raw);
        let attempt = QuestionAttemptId::from_uuid(raw);

        assert_eq!((run.as_uuid(), attempt.as_uuid()), (raw, raw));
    }
}
