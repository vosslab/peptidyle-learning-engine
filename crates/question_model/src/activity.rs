//! Tenant-owned enrollment, run, attempt, and summary records (WP-C3, MOD-RUN).
//!
//! Completion of one run does not end an enrollment. A student can start new
//! runs for practice, and each run owns its question attempts. The explicit
//! three-level model keeps post-completion practice from rewriting the run
//! that first completed an assignment.
//!
//! These records are educational records. Every one carries a [`TenantId`]
//! directly so the future PostgreSQL schema can enforce row-level security on
//! each table without relying on a join through its parent.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ProblemVersionRef;
use crate::RunReference;
use crate::UserId;
use crate::generation::GeneratorReference;
use crate::identity::{ObjectId, ProblemId, VersionId};
use crate::response::StudentResponse;
use crate::run_policy::VariationPolicy;

/// An institution whose educational records share one RLS boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TenantId(Uuid);

/// A tenant-owned assignment offered to students.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssignmentId(Uuid);

/// One stable current-state item within an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssignmentItemId(Uuid);

/// One random-selection group within an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssignmentSelectionGroupId(Uuid);

/// A tenant-owned course or section containing assignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CourseId(Uuid);

/// One current membership group inside a tenant-owned course.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CourseGroupId(Uuid);

/// One durable course-local membership record.
///
/// This identity is historical evidence as well as the single current
/// membership lock target.  It is intentionally distinct from a user and a
/// student record: revocation and a later reinvitation must not rewrite a
/// receipt minted under an earlier membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CourseMembershipId(Uuid);

/// A student enrolled in an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StudentId(Uuid);

/// One current student/group exception attached to an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssignmentPolicyExceptionId(Uuid);

/// One student's durable relationship with one assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EnrollmentId(Uuid);

/// One pass through an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RunId(Uuid);

/// One issued question inside a run.
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

impl_activity_identifier!(TenantId);
impl_activity_identifier!(AssignmentId);
impl_activity_identifier!(AssignmentItemId);
impl_activity_identifier!(AssignmentSelectionGroupId);
impl_activity_identifier!(CourseId);
impl_activity_identifier!(CourseGroupId);
impl_activity_identifier!(CourseMembershipId);
impl_activity_identifier!(StudentId);
impl_activity_identifier!(AssignmentPolicyExceptionId);
impl_activity_identifier!(EnrollmentId);
impl_activity_identifier!(RunId);
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

/// Cross-run completion state for an enrollment.
///
/// This is derived from `first_completed_at`; it is not another stored field
/// that can disagree with the completion record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EnrollmentStatus {
    /// No run has yet satisfied the completion requirement.
    InProgress,
    /// At least one run has satisfied the completion requirement.
    Completed,
}

/// One student's tenant-owned relationship with one assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentEnrollment {
    /// Durable enrollment identity.
    pub id: EnrollmentId,
    /// RLS boundary carried directly on this educational record.
    pub tenant: TenantId,
    /// Assignment the student may run repeatedly.
    pub assignment: AssignmentId,
    /// Authenticated person authorized to act on this enrollment.
    pub user: UserId,
    /// Student who owns the activity.
    ///
    /// This is the institution's pedagogical record identity. It remains
    /// distinct from [`Self::user`] even when one provider maps both to the
    /// same underlying UUID.
    pub student: StudentId,
    /// First server time at which a run satisfied completion.
    pub first_completed_at: Option<ActivityTimestamp>,
    /// Run currently selected by the assignment's grade policy.
    pub current_grade_run: Option<RunId>,
    /// Highest-scoring completed run.
    pub best_grade_run: Option<RunId>,
}

impl AssignmentEnrollment {
    /// Derives the enrollment's cross-run completion state.
    pub fn status(&self) -> EnrollmentStatus {
        if self.first_completed_at.is_some() {
            EnrollmentStatus::Completed
        } else {
            EnrollmentStatus::InProgress
        }
    }
}

/// Whether a run is initial assigned work or continued practice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunMode {
    /// Work performed before the assignment is first completed.
    Assigned,
    /// A new run started after completion for continued learning.
    Practice,
}

/// Authoritative completion state of one run.
///
/// Successor availability is deliberately separate: a run can have no next
/// attempt because it completed or because it exhausted its attempt policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunCompletionStatus {
    /// The run has not satisfied its assignment completion requirement.
    InProgress,
    /// The run has satisfied its assignment completion requirement.
    Completed,
}

/// One pass through an assignment.
///
/// There is deliberately no stored `complete` boolean. The domain derives
/// within-run completion from current question states, then records the
/// resulting completion timestamp and score as one transition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentRun {
    /// Durable run identity.
    pub id: RunId,
    /// Stable typed locator used in application navigation.
    pub reference: RunReference,
    /// RLS boundary carried directly on this educational record.
    pub tenant: TenantId,
    /// Enrollment that owns this run.
    pub enrollment: EnrollmentId,
    /// One-based run number within the enrollment.
    pub run_number: u32,
    /// Server time at which the run began.
    pub started_at: ActivityTimestamp,
    /// Server time at which derived completion was recorded, if complete.
    pub completed_at: Option<ActivityTimestamp>,
    /// Score fraction recorded on completion, if complete.
    pub score: Option<f64>,
    /// Whether this is assigned work or post-completion practice.
    pub mode: RunMode,
    /// Variation policy applied when this run was issued.
    pub variation: VariationPolicy,
}

impl AssignmentRun {
    /// Returns the completion state recorded by the authoritative run projection.
    pub fn completion_status(&self) -> RunCompletionStatus {
        if self.completed_at.is_some() {
            RunCompletionStatus::Completed
        } else {
            RunCompletionStatus::InProgress
        }
    }
}

/// Immutable question selection and issued order for one run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentRunItem {
    /// Run whose future sequencing is frozen by this row.
    pub run: RunId,
    /// Stable fixed-item or selection-candidate identity.
    pub assignment_item: AssignmentItemId,
    /// Position in the mutable assignment definition when the run began.
    pub source_position: u32,
    /// Expanded zero-based delivery order inside this run.
    pub issued_position: u32,
    /// Exact immutable catalog version selected for delivery.
    pub reference: ProblemVersionRef,
    /// Whether this issued item may contribute to cross-course learning evidence.
    ///
    /// The value is frozen when the run begins so later assignment scoring
    /// changes cannot rewrite the validity of an observed learner response.
    pub statistics_eligible: bool,
    /// Selection group that produced this item, if it was drawn.
    pub selection_group: Option<AssignmentSelectionGroupId>,
    /// Deterministic selection seed, absent for fixed items.
    pub selection_seed: Option<u64>,
}

/// Server-recorded timing inputs for one issued question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptTimerRecord {
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
    /// The learner may still submit a response.
    InProgress,
    /// A learner or instructor submitted the current response.
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

/// One question issued inside an assignment run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAttempt {
    /// Durable question-attempt identity.
    pub id: QuestionAttemptId,
    /// RLS boundary carried directly on this educational record.
    pub tenant: TenantId,
    /// Run that owns this attempt.
    pub run: RunId,
    /// Stable published problem containing the attempted version.
    pub problem: ProblemId,
    /// Immutable published question version used for this attempt.
    pub question_version: VersionId,
    /// Zero-based position of this question in the assignment definition.
    ///
    /// Position, rather than only problem/version identity, keeps repeated
    /// content and retries tied to the correct logical question.
    pub assignment_position: u32,
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
    pub timer: AttemptTimerRecord,
    /// Versions and object identities required to reproduce this attempt.
    pub provenance: AttemptProvenance,
    /// Checksummed immutable capability for the protected issuance payloads.
    pub issued_capability: IssuedAttemptCapabilityV1,
}

/// Compact projection read by course pages and the gradebook.
///
/// Historical runs remain separate. Updating this projection from the same
/// run transition lets storage commit the history and summary atomically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentAssignmentSummary {
    /// RLS boundary carried directly on this educational record.
    pub tenant: TenantId,
    /// Enrollment summarized by this row.
    pub enrollment: EnrollmentId,
    /// Score selected by the assignment's grade policy.
    pub current_score: Option<f64>,
    /// Highest completed-run score seen so far.
    pub best_score: Option<f64>,
    /// Most recently completed-run score.
    pub latest_score: Option<f64>,
    /// Number of completed runs, including continued practice runs.
    pub completed_run_count: u32,
    /// Number of question responses recorded across all runs.
    pub total_question_attempts: u64,
    /// Latest server-supplied activity timestamp.
    pub last_activity_at: Option<ActivityTimestamp>,
}

/// Browser-safe status of the Student's aggregate assignment score.
///
/// This is a presentation state, not an authorization input.  The server
/// derives it from the current assignment disclosure policy and never sends
/// the policy, clock, enrollment, or tenant to the browser for inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudentScoreState {
    /// The Student has not submitted any response for this assignment.
    NoActivity,
    /// The Student has activity, but the current policy withholds scores.
    Withheld,
    /// The current policy permits score disclosure.
    Available,
}

/// Key-free Student projection of an assignment's aggregate progress.
///
/// It deliberately excludes the tenant and enrollment identifiers carried by
/// [`StudentAssignmentSummary`].  Browser routes use this type instead of the
/// storage projection so score totals are omitted while withheld.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StudentAssignmentProgress {
    /// Whether aggregate score values are absent because there is no submitted
    /// response, are currently withheld, or are available for display.
    pub score_state: StudentScoreState,
    /// Current freshness and visibility of the assignment's computed scores.
    pub scoring_status: crate::ScoringStatus,
    /// Score selected by the assignment's grade policy when available.
    pub current_score: Option<f64>,
    /// Highest completed-run score when available.
    pub best_score: Option<f64>,
    /// Most recently completed-run score when available.
    pub latest_score: Option<f64>,
    /// Number of completed runs. This is not a score total.
    pub completed_run_count: u32,
    /// Number of recorded responses. This is not a score total.
    pub total_question_attempts: u64,
    /// Latest server-recorded activity time, if any.
    pub last_activity_at: Option<ActivityTimestamp>,
    /// Current anonymous class statistics when the assignment policy permits
    /// their disclosure. Absent means the server withholds this projection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_statistics: Option<crate::StudentClassStatistics>,
}

impl StudentAssignmentProgress {
    /// Projects an entitled Student's assignment before the first durable
    /// educational receipt exists. Reading progress must not create an
    /// enrollment merely to represent the valid no-activity state.
    pub fn no_activity(scoring_status: crate::ScoringStatus) -> Self {
        Self {
            score_state: StudentScoreState::NoActivity,
            scoring_status,
            current_score: None,
            best_score: None,
            latest_score: None,
            completed_run_count: 0,
            total_question_attempts: 0,
            last_activity_at: None,
            class_statistics: None,
        }
    }

    /// Projects the internal summary after the server has made its disclosure
    /// decision. No-activity takes precedence over the disclosure setting.
    pub fn from_summary(
        summary: &StudentAssignmentSummary,
        score_disclosed: bool,
        scoring_status: crate::ScoringStatus,
    ) -> Self {
        let score_state = if summary.total_question_attempts == 0 {
            StudentScoreState::NoActivity
        } else if score_disclosed {
            StudentScoreState::Available
        } else {
            StudentScoreState::Withheld
        };
        let scores = matches!(score_state, StudentScoreState::Available)
            && matches!(scoring_status, crate::ScoringStatus::Current);
        Self {
            score_state,
            scoring_status,
            current_score: scores.then_some(summary.current_score).flatten(),
            best_score: scores.then_some(summary.best_score).flatten(),
            latest_score: scores.then_some(summary.latest_score).flatten(),
            completed_run_count: summary.completed_run_count,
            total_question_attempts: summary.total_question_attempts,
            last_activity_at: summary.last_activity_at,
            class_statistics: None,
        }
    }
}

impl StudentAssignmentSummary {
    /// Creates the empty projection for a new enrollment.
    pub fn empty(tenant: TenantId, enrollment: EnrollmentId) -> Self {
        Self {
            tenant,
            enrollment,
            current_score: None,
            best_score: None,
            latest_score: None,
            completed_run_count: 0,
            total_question_attempts: 0,
            last_activity_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enrollment_status_comes_from_first_completion() {
        let enrollment = AssignmentEnrollment {
            id: EnrollmentId::from_uuid(Uuid::from_u128(1)),
            tenant: TenantId::from_uuid(Uuid::from_u128(2)),
            assignment: AssignmentId::from_uuid(Uuid::from_u128(3)),
            user: UserId::from_uuid(Uuid::from_u128(5)),
            student: StudentId::from_uuid(Uuid::from_u128(4)),
            first_completed_at: Some(ActivityTimestamp::from_unix_millis(1_000)),
            current_grade_run: None,
            best_grade_run: None,
        };

        assert_eq!(enrollment.status(), EnrollmentStatus::Completed);
    }

    #[test]
    fn student_progress_distinguishes_no_activity_withheld_and_available_scores() {
        assert_eq!(
            StudentAssignmentProgress::no_activity(crate::ScoringStatus::Current),
            StudentAssignmentProgress {
                score_state: StudentScoreState::NoActivity,
                scoring_status: crate::ScoringStatus::Current,
                current_score: None,
                best_score: None,
                latest_score: None,
                completed_run_count: 0,
                total_question_attempts: 0,
                last_activity_at: None,
                class_statistics: None,
            }
        );
        let mut summary = StudentAssignmentSummary::empty(
            TenantId::from_uuid(Uuid::from_u128(1)),
            EnrollmentId::from_uuid(Uuid::from_u128(2)),
        );
        assert_eq!(
            StudentAssignmentProgress::from_summary(&summary, true, crate::ScoringStatus::Current)
                .score_state,
            StudentScoreState::NoActivity
        );

        summary.total_question_attempts = 1;
        summary.current_score = Some(0.5);
        summary.best_score = Some(0.5);
        summary.latest_score = Some(0.5);
        let withheld =
            StudentAssignmentProgress::from_summary(&summary, false, crate::ScoringStatus::Current);
        assert_eq!(withheld.score_state, StudentScoreState::Withheld);
        assert_eq!(
            (
                withheld.current_score,
                withheld.best_score,
                withheld.latest_score
            ),
            (None, None, None)
        );

        let available =
            StudentAssignmentProgress::from_summary(&summary, true, crate::ScoringStatus::Current);
        assert_eq!(available.score_state, StudentScoreState::Available);
        assert_eq!(available.current_score, Some(0.5));
        assert!(available.class_statistics.is_none());
    }

    #[test]
    fn student_progress_hides_scores_while_scoring_is_not_current() {
        let mut summary = StudentAssignmentSummary::empty(
            TenantId::from_uuid(Uuid::from_u128(1)),
            EnrollmentId::from_uuid(Uuid::from_u128(2)),
        );
        summary.total_question_attempts = 1;
        summary.current_score = Some(0.5);
        for scoring_status in [
            crate::ScoringStatus::Recalculating,
            crate::ScoringStatus::Failed,
        ] {
            let progress = StudentAssignmentProgress::from_summary(&summary, true, scoring_status);
            assert_eq!(progress.score_state, StudentScoreState::Available);
            assert_eq!(progress.scoring_status, scoring_status);
            assert_eq!(progress.current_score, None);
        }
    }

    #[test]
    fn every_activity_identifier_stays_distinct_but_round_trips() {
        let raw = Uuid::from_u128(7);
        let run = RunId::from_uuid(raw);
        let attempt = QuestionAttemptId::from_uuid(raw);

        assert_eq!((run.as_uuid(), attempt.as_uuid()), (raw, raw));
    }
}
