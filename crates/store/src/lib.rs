//! Backend-neutral persistence contract (WP-C4, MOD-STO).
//!
//! Shared published content has no tenant context. Every educational-record
//! operation requires [`TenantContext`], which has no default. Lists require a
//! bounded [`PageRequest`]; the trait has no unbounded or positional paging
//! method. No SQL type appears in this contract.

use async_trait::async_trait;
use domain::run::RunModelError;
use domain::scoring::RunTransition;
use question_model::{
    ActivityTimestamp, AssignmentEnrollment, AssignmentId, AssignmentRun, EnrollmentId,
    GradePolicy, ProblemId, QuestionAttempt, QuestionAttemptId, QuestionDefinition, RunId,
    RunPolicies, StudentAssignmentSummary, TenantId, VersionId, WorkspaceId,
};

/// In-memory backend used by tests and lanes waiting for PostgreSQL.
pub mod memory;
/// Cursor and bounded-page types shared by every list method.
pub mod pagination;
/// PostgreSQL health and future backend implementation.
pub mod postgres;
/// Explicit tenant context used by every educational-record operation.
pub mod rls;

pub use crate::pagination::{Cursor, Page, PageRequest, PageSize, PaginationError};
pub use crate::rls::TenantContext;

/// Immutable published problem/version reference stored by assignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishedVersionRef {
    /// Stable published problem.
    pub problem: ProblemId,
    /// Exact immutable version assigned.
    pub version: VersionId,
}

/// Tenant-owned editable question draft.
#[derive(Debug, Clone, PartialEq)]
pub struct DraftRecord {
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Browser-safe question definition with no `ProblemId`.
    pub question: QuestionDefinition,
}

/// Shared immutable published content.
#[derive(Debug, Clone, PartialEq)]
pub struct PublishedProblemRecord {
    /// Stable published problem.
    pub problem: ProblemId,
    /// Exact immutable version.
    pub version: VersionId,
    /// Browser-safe definition whose IDs match this record.
    pub question: QuestionDefinition,
}

/// Tenant-owned assignment that references shared immutable content.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentRecord {
    /// Durable assignment identity.
    pub id: AssignmentId,
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Shared problem versions selected for the assignment.
    pub problems: Vec<PublishedVersionRef>,
    /// Four independent run policies.
    pub policies: RunPolicies,
}

/// One atomic activity write applied with its compact summary projection.
#[derive(Debug, Clone, PartialEq)]
pub enum ActivityTransition {
    /// Creates a new incomplete run and records its start activity.
    StartRun {
        /// New run with server-supplied start time.
        run: AssignmentRun,
    },
    /// Appends one immutable question-attempt record.
    RecordQuestionAttempt {
        /// Attempt carrying response, result, timing, and reproducibility data.
        attempt: Box<QuestionAttempt>,
    },
    /// Completes an existing run and projects its score.
    CompleteRun {
        /// Existing run to complete.
        run: RunId,
        /// Final score fraction.
        score: f64,
        /// Authoritative PostgreSQL timestamp.
        at: ActivityTimestamp,
    },
}

/// Portable persistence failure with no SQL type in its variants.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreError {
    /// Requested record is absent in the active tenant or shared catalog.
    NotFound,
    /// Immutable identity already exists.
    AlreadyExists,
    /// A tenant-owned record disagrees with authenticated context.
    TenantMismatch,
    /// Record shape violates a model invariant.
    InvalidRecord(String),
    /// Pure activity projection rejected the transition.
    RunModel(RunModelError),
    /// Backend state is temporarily unavailable.
    Unavailable(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "record not found"),
            Self::AlreadyExists => write!(formatter, "immutable record already exists"),
            Self::TenantMismatch => write!(formatter, "record tenant does not match context"),
            Self::InvalidRecord(message) => write!(formatter, "invalid record: {message}"),
            Self::RunModel(error) => write!(formatter, "activity transition rejected: {error}"),
            Self::Unavailable(message) => write!(formatter, "store unavailable: {message}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<RunModelError> for StoreError {
    fn from(error: RunModelError) -> Self {
        Self::RunModel(error)
    }
}

/// Persistence operations consumed by catalog, course, run, and worker lanes.
#[async_trait]
pub trait Store: Send + Sync {
    /// Creates or replaces a tenant-owned editable draft.
    async fn upsert_draft(
        &self,
        context: TenantContext,
        draft: DraftRecord,
    ) -> Result<(), StoreError>;

    /// Reads a draft only inside the active tenant.
    async fn get_draft(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
    ) -> Result<Option<DraftRecord>, StoreError>;

    /// Inserts one immutable shared published version.
    async fn publish_problem(&self, record: PublishedProblemRecord) -> Result<(), StoreError>;

    /// Resolves one exact shared published version.
    async fn get_published_problem(
        &self,
        problem: ProblemId,
        version: VersionId,
    ) -> Result<Option<PublishedProblemRecord>, StoreError>;

    /// Lists shared published versions in stable cursor order.
    async fn list_published_problems(
        &self,
        page: PageRequest,
    ) -> Result<Page<PublishedProblemRecord>, StoreError>;

    /// Creates or replaces a tenant-owned assignment definition.
    async fn upsert_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
    ) -> Result<(), StoreError>;

    /// Reads one assignment inside the active tenant.
    async fn get_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentRecord>, StoreError>;

    /// Lists assignments inside the active tenant in stable cursor order.
    async fn list_assignments(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError>;

    /// Creates one enrollment and its empty compact summary.
    async fn create_enrollment(
        &self,
        context: TenantContext,
        enrollment: AssignmentEnrollment,
    ) -> Result<(), StoreError>;

    /// Reads one enrollment inside the active tenant.
    async fn get_enrollment(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError>;

    /// Atomically writes activity and its compact summary projection.
    async fn apply_activity_transition(
        &self,
        context: TenantContext,
        transition: ActivityTransition,
    ) -> Result<StudentAssignmentSummary, StoreError>;

    /// Reads one run inside the active tenant.
    async fn get_run(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError>;

    /// Lists runs for one enrollment in stable cursor order.
    async fn list_runs(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRun>, StoreError>;

    /// Reads one question attempt inside the active tenant.
    async fn get_question_attempt(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError>;

    /// Reads the transactionally maintained summary for one enrollment.
    async fn get_summary(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummary>, StoreError>;
}

/// Extracts the grade policy used by an activity projection.
pub(crate) fn grade_policy(assignment: &AssignmentRecord) -> GradePolicy {
    assignment.policies.grade
}

/// Maps a storage activity write to the pure domain transition.
pub(crate) fn summary_transition(transition: &ActivityTransition) -> RunTransition {
    match transition {
        ActivityTransition::StartRun { run } => RunTransition::Started { at: run.started_at },
        ActivityTransition::RecordQuestionAttempt { attempt } => {
            RunTransition::QuestionAttemptRecorded {
                at: attempt
                    .timer
                    .submitted_at
                    .unwrap_or(attempt.timer.issued_at),
            }
        }
        ActivityTransition::CompleteRun { score, at, .. } => RunTransition::Completed {
            score: *score,
            at: *at,
        },
    }
}
