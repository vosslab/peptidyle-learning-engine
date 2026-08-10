use super::*;

/// The storage authority for security-sensitive expiry calculations. Server
/// routes must not consult a process wall clock because replicas can disagree.
#[async_trait]
pub trait AuthoritativeTimeStore: Send + Sync {
    async fn authoritative_time(
        &self,
        context: TenantContext,
    ) -> Result<ActivityTimestamp, StoreError>;
}

/// Retention persistence boundary. Every mutator authenticates the supplied
/// stored session and derives administrator or course-instructor authority
/// from persisted session/course data; no request supplies a role or tenant.
#[async_trait]
pub trait RetentionStore: Send + Sync {
    /// Updates this tenant's future-course policy after stored-admin validation.
    async fn configure_retention_policy(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        policy: InstitutionRetentionPolicy,
    ) -> Result<(), StoreError>;

    /// Ends a course at the backend authoritative time and snapshots its policy.
    async fn end_course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseRetentionRecord, StoreError>;

    /// Reads an ended course's retention record after stored-session/course authorization.
    async fn course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseRetentionRecord>, StoreError>;
}

/// Private schedule-control boundary for the retention service.
///
/// The dispatcher accepts only a validated batch size: no caller can name a
/// tenant, course, stage, timestamp, generation, job, or object key. The two
/// authenticated mutators derive all authority from the persisted session.
#[async_trait]
pub trait RetentionScheduleStore: Send + Sync {
    /// Dispatches due current-generation stages into bound closed queue jobs.
    async fn dispatch_due_retention_stages(
        &self,
        batch: RetentionDispatchBatch,
    ) -> Result<u16, StoreError>;

    /// Extends only unstarted current stages after stored-administrator validation.
    async fn extend_course_retention(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        additional_days: RetentionDays,
    ) -> Result<CourseRetentionRecord, StoreError>;

    /// Records the explicit archive-time assignment-definition choice.
    async fn set_archive_disposition(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<CourseRetentionRecord, StoreError>;
}

/// Browser-facing retention control boundary.
///
/// It exposes only a safe lifecycle projection and conditional instructor
/// requests. Archive and delete requests enter the existing broker-owned
/// dispatch contract; they never execute cleanup in a request transaction.
#[async_trait]
pub trait RetentionApiStore: Send + Sync {
    /// Reads the key-free retention projection for an authorized course actor.
    async fn retention_view(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<CourseRetentionView>, StoreError>;

    /// Reads the current durable in-app notification intent, if one exists.
    async fn retention_notification(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<Option<RetentionNotificationView>, StoreError>;

    /// Conditionally extends an active schedule after an exact revision match.
    async fn extend_retention_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        additional_days: RetentionDays,
    ) -> Result<CourseRetentionView, StoreError>;

    /// Conditionally requests immediate archive work through the closed broker.
    async fn request_retention_archive_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
        disposition: AssignmentDefinitionDisposition,
    ) -> Result<RetentionRequestResult, StoreError>;

    /// Conditionally requests immediate permanent-delete work through the broker.
    async fn request_retention_delete_if_revision(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        expected: RetentionRevision,
    ) -> Result<RetentionRequestResult, StoreError>;
}

/// Server-worker retention boundary. The only input is a current queue lease
/// plus the closed course/stage/generation identity; the Store resolves every
/// educational-record object and performs all access revocation itself.
#[async_trait]
pub trait RetentionWorkerStore: Send + Sync {
    /// Claims a current scheduled stage and returns private exact work. A
    /// stale generation or lease is rejected before any object key is exposed.
    async fn prepare_retention_work(
        &self,
        command: RetentionWorkerCommand,
    ) -> Result<RetentionWork, StoreError>;

    /// Finalizes the exact claimed stage only after its worker completed every
    /// external idempotent effect. The implementation also completes `job` in
    /// the same conditional transition.
    async fn commit_retention_work(
        &self,
        command: RetentionWorkerCommand,
    ) -> Result<(), StoreError>;
}

/// Lease- and generation-fenced scoring rebuild command used only by workers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssignmentScoringWorkerCommand {
    pub job: JobId,
    pub lease: JobLeaseToken,
    pub assignment: AssignmentId,
    pub generation: ScoringGeneration,
}

/// Result of atomically publishing one prepared scoring generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignmentScoringCommitOutcome {
    /// This generation replaced every current computed score and summary.
    Committed,
    /// A newer generation superseded this work; its staging rows were discarded.
    Superseded,
    /// The queue lease expired or was reclaimed before publication.
    ClaimNoLongerActive,
}

/// Private staging and atomic-publication boundary for assignment rescoring.
#[async_trait]
pub trait AssignmentScoringWorkerStore: Send + Sync {
    /// Rebuilds private staging rows without changing learner-visible scores.
    async fn prepare_assignment_scoring(
        &self,
        context: TenantContext,
        command: AssignmentScoringWorkerCommand,
    ) -> Result<(), StoreError>;

    /// Conditionally replaces current rows and completes the exact queue lease.
    async fn commit_assignment_scoring(
        &self,
        context: TenantContext,
        command: AssignmentScoringWorkerCommand,
    ) -> Result<AssignmentScoringCommitOutcome, StoreError>;
}

/// Lease- and generation-fenced command for one scheduled auto-submit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttemptAutoSubmitWorkerCommand {
    pub job: JobId,
    pub lease: JobLeaseToken,
    pub attempt: QuestionAttemptId,
    pub timing_generation: u64,
}

/// Result of atomically resolving one scheduled attempt deadline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptAutoSubmitCommitOutcome {
    /// This invocation changed active work to `auto_submitted`.
    AutoSubmitted,
    /// A newer timing policy or terminal attempt made this job obsolete.
    Superseded,
    /// An extension moved the effective deadline and the same job was rescheduled.
    Rescheduled,
    /// The queue lease expired or was reclaimed before the transaction committed.
    ClaimNoLongerActive,
}

/// Durable deadline-finalization boundary used by the stateless worker.
#[async_trait]
pub trait AttemptAutoSubmitWorkerStore: Send + Sync {
    /// Re-resolves the current timing row and either submits, supersedes, or
    /// reschedules the exact leased job in one transaction.
    async fn commit_attempt_auto_submit(
        &self,
        context: TenantContext,
        command: AttemptAutoSubmitWorkerCommand,
    ) -> Result<AttemptAutoSubmitCommitOutcome, StoreError>;
}

/// Database-backed visibility boundary for ordinary learner course records.
///
/// This deliberately returns only a boolean: lifecycle stages and retention
/// schedule details remain Store-private. Implementations must evaluate the
/// course existence and current retention fence in the same backend security
/// context as the later record operation.
#[async_trait]
pub trait CourseRecordsAccessStore: Send + Sync {
    /// Returns false for a missing course, a foreign tenant, an archived or
    /// deleted course, or a current-generation archive stage already started.
    async fn course_records_accessible(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<bool, StoreError>;
}
