//! Pure, server-owned retention policy and course-lifecycle contracts.
//!
//! This module deliberately does not schedule work, read a wall clock, or
//! authorize deletion. Store and worker implementations bind these deterministic
//! values to tenant-scoped configuration, database time, and private jobs.

use std::collections::BTreeSet;
use std::num::NonZeroU16;

use objects::ObjectKey;
use question_model::{ActivityTimestamp, CourseId, TenantId};
use serde::{Deserialize, Serialize};

/// Default number of days after a course ends before instructors are notified.
pub const DEFAULT_RETENTION_NOTIFY_DAYS: u16 = 30;
/// Default number of days after a course ends before student records archive.
pub const DEFAULT_RETENTION_ARCHIVE_DAYS: u16 = 100;
/// Default number of days after a course ends before student records delete.
pub const DEFAULT_RETENTION_DELETE_DAYS: u16 = 365;

/// Largest accepted institutional retention window: one century in whole days.
pub const MAX_RETENTION_DAYS: u16 = 36_500;

/// Maximum number of due retention stages one private scheduler pass may dispatch.
pub const MAX_RETENTION_DISPATCH_BATCH: u16 = 100;
/// Fixed retry budget for a broker-created retention stage job.
pub const RETENTION_JOB_MAX_ATTEMPTS: u16 = 3;

const MILLIS_PER_DAY: i64 = 86_400_000;

/// A validated positive number of whole retention days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RetentionDays(NonZeroU16);

impl RetentionDays {
    /// Validates a bounded, nonzero whole-day retention window.
    pub fn new(days: u16) -> Result<Self, RetentionPolicyError> {
        let Some(days) = NonZeroU16::new(days) else {
            return Err(RetentionPolicyError::ZeroDays);
        };
        if days.get() > MAX_RETENTION_DAYS {
            return Err(RetentionPolicyError::DaysTooLarge);
        }
        Ok(Self(days))
    }

    /// Returns the validated whole-day value.
    pub fn get(self) -> u16 {
        self.0.get()
    }
}

/// A validated private scheduler batch bound.
///
/// It intentionally has no serde implementation: neither browser requests nor
/// durable queue payloads choose scheduling breadth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionDispatchBatch(u16);

impl RetentionDispatchBatch {
    /// Validates a bounded nonzero scheduler work batch.
    pub fn new(value: u16) -> Result<Self, RetentionPolicyError> {
        if value == 0 {
            return Err(RetentionPolicyError::ZeroDispatchBatch);
        }
        if value > MAX_RETENTION_DISPATCH_BATCH {
            return Err(RetentionPolicyError::DispatchBatchTooLarge);
        }
        Ok(Self(value))
    }

    pub(crate) fn get(self) -> u16 {
        self.0
    }
}

/// Rejection reasons for retention policy or timestamp calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetentionPolicyError {
    /// A stage interval was zero days.
    ZeroDays,
    /// A stage interval exceeds the deliberately broad bounded range.
    DaysTooLarge,
    /// Notification, archive, and permanent-delete stages were not increasing.
    StagesNotIncreasing,
    /// Adding a validated stage interval to an authoritative timestamp overflowed.
    TimestampOverflow,
    /// A private stage identity omitted its required positive stale-work fence.
    ZeroGeneration,
    /// A browser revision cannot exceed PostgreSQL's signed bigint fence.
    GenerationTooLarge,
    /// A scheduler pass requested no work.
    ZeroDispatchBatch,
    /// A scheduler pass exceeded the fixed work bound.
    DispatchBatchTooLarge,
}

impl std::fmt::Display for RetentionPolicyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroDays => formatter.write_str("retention days must be positive"),
            Self::DaysTooLarge => {
                formatter.write_str("retention days exceed the supported maximum")
            }
            Self::StagesNotIncreasing => formatter
                .write_str("retention notification, archive, and delete stages must increase"),
            Self::TimestampOverflow => formatter.write_str("retention timestamp overflows"),
            Self::ZeroGeneration => formatter.write_str("retention generation must be positive"),
            Self::GenerationTooLarge => {
                formatter.write_str("retention generation exceeds the supported maximum")
            }
            Self::ZeroDispatchBatch => {
                formatter.write_str("retention dispatch batch must be positive")
            }
            Self::DispatchBatchTooLarge => {
                formatter.write_str("retention dispatch batch exceeds the supported maximum")
            }
        }
    }
}

impl std::error::Error for RetentionPolicyError {}

/// Per-institution retention intervals, resolved by trusted server policy.
///
/// The type has no request deserializer: a future sysadmin-only Store
/// boundary owns configuration and prevents a browser request from choosing a
/// shorter or longer deletion window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstitutionRetentionPolicy {
    notify_after: RetentionDays,
    archive_after: RetentionDays,
    delete_after: RetentionDays,
}

impl InstitutionRetentionPolicy {
    /// Creates an ordered retention policy.
    pub fn new(
        notify_after: RetentionDays,
        archive_after: RetentionDays,
        delete_after: RetentionDays,
    ) -> Result<Self, RetentionPolicyError> {
        if !(notify_after < archive_after && archive_after < delete_after) {
            return Err(RetentionPolicyError::StagesNotIncreasing);
        }
        Ok(Self {
            notify_after,
            archive_after,
            delete_after,
        })
    }

    /// Returns the configured interval for one lifecycle stage.
    pub fn days_for(self, stage: RetentionStage) -> RetentionDays {
        match stage {
            RetentionStage::Notify => self.notify_after,
            RetentionStage::ArchiveStudentRecords => self.archive_after,
            RetentionStage::DeleteStudentRecords => self.delete_after,
        }
    }

    /// Returns the configured notification interval.
    pub fn notify_after(self) -> RetentionDays {
        self.notify_after
    }

    /// Returns the configured archive interval.
    pub fn archive_after(self) -> RetentionDays {
        self.archive_after
    }

    /// Returns the configured permanent-delete interval.
    pub fn delete_after(self) -> RetentionDays {
        self.delete_after
    }

    /// Calculates a stage deadline from a server-authoritative course end time.
    pub fn due_at(
        self,
        course_ended_at: ActivityTimestamp,
        stage: RetentionStage,
    ) -> Result<ActivityTimestamp, RetentionPolicyError> {
        let millis = i64::from(self.days_for(stage).get())
            .checked_mul(MILLIS_PER_DAY)
            .ok_or(RetentionPolicyError::TimestampOverflow)?;
        let due_at = course_ended_at
            .as_unix_millis()
            .checked_add(millis)
            .ok_or(RetentionPolicyError::TimestampOverflow)?;
        Ok(ActivityTimestamp::from_unix_millis(due_at))
    }

    /// Returns every lifecycle stage whose authoritative deadline has passed.
    ///
    /// This is eligibility only. It never claims that an archive or deletion
    /// completed: R2's persisted Store lifecycle alone records those effects.
    /// `now` is supplied by a Store's authoritative clock, never a wall clock.
    pub fn due_stages_at(
        self,
        course_ended_at: ActivityTimestamp,
        now: ActivityTimestamp,
    ) -> Result<Vec<RetentionStage>, RetentionPolicyError> {
        let mut due = Vec::with_capacity(3);
        for stage in [
            RetentionStage::Notify,
            RetentionStage::ArchiveStudentRecords,
            RetentionStage::DeleteStudentRecords,
        ] {
            let due_at = self.due_at(course_ended_at, stage)?;
            if now >= due_at {
                due.push(stage);
            }
        }
        Ok(due)
    }
}

impl Default for InstitutionRetentionPolicy {
    fn default() -> Self {
        Self::new(
            RetentionDays::new(DEFAULT_RETENTION_NOTIFY_DAYS).expect("default notify days"),
            RetentionDays::new(DEFAULT_RETENTION_ARCHIVE_DAYS).expect("default archive days"),
            RetentionDays::new(DEFAULT_RETENTION_DELETE_DAYS).expect("default delete days"),
        )
        .expect("default retention stages are increasing")
    }
}

/// One scheduled course-retention transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RetentionStage {
    /// Notify the instructor that student records remain available.
    Notify,
    /// Remove student records from ordinary application access.
    ArchiveStudentRecords,
    /// Permanently remove student records and their exact owned artifacts.
    DeleteStudentRecords,
}

/// Closed instructor-facing retention action vocabulary with no recipient,
/// learner, object, or email data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RetentionNotificationIntent {
    Archive,
    Delete,
    Extend,
}

/// Browser-safe durable retention notification and authoritative creation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionNotificationView {
    /// Closed retention action vocabulary.
    pub intent: RetentionNotificationIntent,
    /// Database-authoritative notification timestamp.
    pub created_at: ActivityTimestamp,
}

/// Frozen in-app archive notification copy; delivery channels are outside R3.
pub const RETENTION_ARCHIVE_NOTIFICATION_COPY: &str = "This course ended 30 days ago. Student records are still available. If they are no longer needed, archive or delete the course now. Student records will be automatically removed after 100 days unless the course is archived or the retention period is extended by a sysadmin.";

/// Coarse, browser-safe course retention state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseRetentionState {
    /// Student records remain active; notification is not due yet.
    Active,
    /// Student records remain available, but the instructor notification is due.
    NotificationDue,
    /// Student records are inaccessible while permanent deletion is pending.
    StudentRecordsArchived,
    /// Student records and their student-record artifacts are permanently deleted.
    StudentRecordsDeleted,
}

/// Owner-selected treatment of the tenant-owned assignment definition.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentDefinitionDisposition {
    /// Preserve the reusable tenant-owned assignment definition after record purge.
    #[default]
    Retain,
    /// Delete the assignment definition after its student records are purged.
    Delete,
}

/// Key-free browser-safe projection of retention scheduling state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseRetentionStatus {
    /// Current coarse lifecycle state.
    pub state: CourseRetentionState,
    /// Assignment-definition disposition selected for this course.
    pub assignment_definitions: AssignmentDefinitionDisposition,
}

/// Immutable retention schedule snapshot created when a course explicitly ends.
///
/// This is server-side state, not a browser request. Later worker slices use
/// its positive generation to reject stale scheduled work after a policy edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseRetentionSnapshot {
    ended_at: ActivityTimestamp,
    policy: InstitutionRetentionPolicy,
    assignment_definitions: AssignmentDefinitionDisposition,
    generation: u64,
}

/// Server-side retention record returned only after stored-session and course
/// authorization. The browser-facing route will project only `status` in R4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseRetentionRecord {
    /// Immutable schedule created by the authenticated course-end transition.
    pub snapshot: CourseRetentionSnapshot,
    /// Actual persisted lifecycle, never inferred solely from a passed deadline.
    pub status: CourseRetentionStatus,
}

/// Opaque positive optimistic-concurrency revision for the safe retention API.
///
/// It projects the Store's generation fence without exposing schedule internals,
/// deadlines, jobs, or worker capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RetentionRevision(u64);

impl RetentionRevision {
    /// Validates a revision received from a strong conditional request.
    ///
    /// A revision carries no authority; the Store still derives the actor and
    /// course from its stored session before applying a conditional change.
    pub fn new(value: u64) -> Result<Self, RetentionPolicyError> {
        if value > i64::MAX as u64 {
            return Err(RetentionPolicyError::GenerationTooLarge);
        }
        Self::from_generation(value)
    }

    /// Returns the positive numeric revision used by a strong HTTP ETag.
    pub fn value(self) -> u64 {
        self.0
    }

    pub(crate) fn from_generation(generation: u64) -> Result<Self, RetentionPolicyError> {
        if generation == 0 {
            return Err(RetentionPolicyError::ZeroGeneration);
        }
        Ok(Self(generation))
    }
}

/// Browser-safe current retention lifecycle projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseRetentionView {
    /// Persisted lifecycle only; deadline passage never fabricates this state.
    pub state: CourseRetentionState,
    /// Instructor-selected archive-time definition treatment.
    pub assignment_definitions: AssignmentDefinitionDisposition,
    /// Opaque strong-ETag revision backed by the private schedule generation.
    pub revision: RetentionRevision,
}

/// One private conditional retention API mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetentionApiAction {
    Extend(RetentionDays),
    Archive(AssignmentDefinitionDisposition),
    Delete,
}

/// Replay-safe state of a manual archive or delete request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RetentionRequestOutcome {
    /// A bound current-generation job was scheduled.
    Scheduled,
    /// The requested current-generation stage is already leased.
    InProgress,
    /// The requested current-generation stage already completed.
    Completed,
}

/// Safe retention view plus the manual request's replay-safe outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionRequestResult {
    /// Current key-free lifecycle projection.
    pub retention: CourseRetentionView,
    /// Current state of the exact requested stage.
    pub outcome: RetentionRequestOutcome,
}

impl CourseRetentionRecord {
    /// Removes policy, deadline, generation, and all worker state for HTTP use.
    pub fn safe_view(self) -> Result<CourseRetentionView, RetentionPolicyError> {
        Ok(CourseRetentionView {
            state: self.status.state,
            assignment_definitions: self.status.assignment_definitions,
            revision: RetentionRevision::from_generation(self.snapshot.generation())?,
        })
    }
}

impl CourseRetentionSnapshot {
    /// Creates the first immutable schedule snapshot for an ended course.
    pub(crate) fn new(
        ended_at: ActivityTimestamp,
        policy: InstitutionRetentionPolicy,
        assignment_definitions: AssignmentDefinitionDisposition,
        generation: u64,
    ) -> Result<Self, RetentionPolicyError> {
        if generation == 0 {
            return Err(RetentionPolicyError::ZeroGeneration);
        }
        Ok(Self {
            ended_at,
            policy,
            assignment_definitions,
            generation,
        })
    }

    /// Returns the server-authoritative course-end time.
    pub fn ended_at(self) -> ActivityTimestamp {
        self.ended_at
    }
    /// Returns the immutable policy effective at course end.
    pub fn policy(self) -> InstitutionRetentionPolicy {
        self.policy
    }
    /// Returns the selected assignment-definition disposition.
    pub fn assignment_definitions(self) -> AssignmentDefinitionDisposition {
        self.assignment_definitions
    }
    /// Returns the positive stale-work fence.
    pub fn generation(self) -> u64 {
        self.generation
    }

    /// Rebinds an existing trusted snapshot to a later schedule generation.
    ///
    /// Only the Store's generation-fenced schedule transaction uses this; it
    /// cannot be constructed from a browser timestamp or policy request.
    pub(crate) fn with_generation_and_disposition(
        self,
        generation: u64,
        assignment_definitions: AssignmentDefinitionDisposition,
    ) -> Result<Self, RetentionPolicyError> {
        Self::new(
            self.ended_at,
            self.policy,
            assignment_definitions,
            generation,
        )
    }
}

impl CourseRetentionStatus {
    /// Builds a safe projection from a Store-persisted lifecycle outcome.
    ///
    /// This constructor deliberately accepts no clock or policy. A passed
    /// deadline makes a stage eligible; only the R2 Store transaction may
    /// report the archive or permanent-deletion states to a browser.
    pub fn from_persisted(
        state: CourseRetentionState,
        assignment_definitions: AssignmentDefinitionDisposition,
    ) -> Self {
        Self {
            state,
            assignment_definitions,
        }
    }
}

/// Private idempotency identity for one scheduled retention job.
///
/// This is intentionally non-serializable and crate-private. R2 binds it to a
/// tenant-scoped Store command; later queue payloads carry only this immutable
/// course/stage/generation identity, never student IDs, object keys, URLs, or
/// record payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "R2 binds this private identity to the retention Store and worker command"
)]
pub(crate) struct RetentionJobIdentity {
    pub(crate) course: CourseId,
    pub(crate) stage: RetentionStage,
    pub(crate) generation: u64,
}

/// Private, typed object manifest resolved by the Store for one worker claim.
///
/// It is intentionally neither serializable nor constructible from a route or
/// queue payload. Every entry must be a tenant-owned `StudentRecord` object;
/// the worker deletes these exact keys only after the Store has revoked their
/// protected deliveries.
#[derive(Clone, PartialEq, Eq)]
pub struct RetentionCleanupManifest {
    /// Exact authoritative object metadata, never a bucket prefix.
    pub(crate) objects: Vec<ObjectKey>,
}

impl std::fmt::Debug for RetentionCleanupManifest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RetentionCleanupManifest")
            .field("object_count", &self.objects.len())
            .finish()
    }
}

impl RetentionCleanupManifest {
    /// Returns the private exact object records for a server worker.
    pub fn objects(&self) -> &[ObjectKey] {
        &self.objects
    }

    pub(crate) fn from_iter(objects: BTreeSet<ObjectKey>) -> Self {
        Self {
            objects: objects.into_iter().collect(),
        }
    }
}

/// Private durable cleanup manifest lifecycle for one stage identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetentionCleanupManifestState {
    Prepared,
    Completed,
}

/// Private, persisted in-memory state of one cleanup manifest.
#[derive(Debug, Clone)]
pub(crate) struct StoredRetentionCleanupManifest {
    pub(crate) job: crate::JobId,
    pub(crate) state: RetentionCleanupManifestState,
    pub(crate) objects: BTreeSet<ObjectKey>,
}

/// Store-resolved work for a lease-bound retention stage.
#[derive(Clone, PartialEq, Eq)]
pub enum RetentionWork {
    /// Create the durable, in-app instructor notification exactly once.
    Notify,
    /// Delete only the Store-resolved student-record object manifest.
    Cleanup(RetentionCleanupManifest),
}

impl std::fmt::Debug for RetentionWork {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Notify => formatter.write_str("RetentionWork::Notify"),
            Self::Cleanup(manifest) => formatter
                .debug_tuple("RetentionWork::Cleanup")
                .field(manifest)
                .finish(),
        }
    }
}

/// One lease- and generation-fenced retention worker command.
///
/// This value crosses only the server worker/Store boundary. It has no serde
/// implementation, so it cannot become a browser command or queue payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionWorkerCommand {
    /// Active tenant inherited from the claimed durable job.
    pub tenant: TenantId,
    /// Course identity from the closed queue payload.
    pub course: CourseId,
    /// Retention stage from the closed queue payload.
    pub stage: RetentionStage,
    /// Positive schedule generation from the closed queue payload.
    pub generation: u64,
    /// Claimed durable queue identity.
    pub job: crate::JobId,
    /// Opaque active queue lease; never serialized or logged.
    pub lease: crate::JobLeaseToken,
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn notification_intent_and_copy_are_closed_and_key_free() {
        assert_eq!(RetentionNotificationIntent::Archive as u8, 0);
        assert_eq!(
            RETENTION_ARCHIVE_NOTIFICATION_COPY,
            "This course ended 30 days ago. Student records are still available. If they are no longer needed, archive or delete the course now. Student records will be automatically removed after 100 days unless the course is archived or the retention period is extended by a sysadmin."
        );
        assert!(RETENTION_ARCHIVE_NOTIFICATION_COPY.contains("course ended"));
        assert!(
            RETENTION_ARCHIVE_NOTIFICATION_COPY.contains("Student records are still available")
        );
        assert!(RETENTION_ARCHIVE_NOTIFICATION_COPY.contains("archive or delete"));
        assert!(RETENTION_ARCHIVE_NOTIFICATION_COPY.contains("retention period is extended"));
        for forbidden in ["@", "object", "key", "email"] {
            assert!(
                !RETENTION_ARCHIVE_NOTIFICATION_COPY
                    .to_ascii_lowercase()
                    .contains(forbidden)
            );
        }
    }

    fn timestamp(days: i64) -> ActivityTimestamp {
        ActivityTimestamp::from_unix_millis(days * MILLIS_PER_DAY)
    }

    #[test]
    fn policy_rejects_zero_reversed_and_excessive_windows() {
        assert_eq!(
            RetentionRevision::new(0),
            Err(RetentionPolicyError::ZeroGeneration)
        );
        assert_eq!(
            RetentionRevision::new(i64::MAX as u64 + 1),
            Err(RetentionPolicyError::GenerationTooLarge)
        );
        assert_eq!(RetentionDays::new(0), Err(RetentionPolicyError::ZeroDays));
        assert_eq!(
            RetentionDays::new(MAX_RETENTION_DAYS + 1),
            Err(RetentionPolicyError::DaysTooLarge)
        );
        let thirty = RetentionDays::new(30).expect("valid days");
        let hundred = RetentionDays::new(100).expect("valid days");
        assert_eq!(
            InstitutionRetentionPolicy::new(hundred, thirty, RetentionDays::new(365).unwrap()),
            Err(RetentionPolicyError::StagesNotIncreasing)
        );
        assert_eq!(
            RetentionDispatchBatch::new(0),
            Err(RetentionPolicyError::ZeroDispatchBatch)
        );
        assert_eq!(
            RetentionDispatchBatch::new(MAX_RETENTION_DISPATCH_BATCH + 1),
            Err(RetentionPolicyError::DispatchBatchTooLarge)
        );
        assert_eq!(
            RetentionDispatchBatch::new(MAX_RETENTION_DISPATCH_BATCH)
                .expect("bounded batch")
                .get(),
            MAX_RETENTION_DISPATCH_BATCH
        );
        assert_eq!(
            InstitutionRetentionPolicy::default().due_at(
                ActivityTimestamp::from_unix_millis(i64::MAX),
                RetentionStage::Notify,
            ),
            Err(RetentionPolicyError::TimestampOverflow)
        );
    }

    #[test]
    fn default_policy_reports_due_stages_without_claiming_worker_effects() {
        let policy = InstitutionRetentionPolicy::default();
        let ended_at = timestamp(10_000);
        for (offset, expected_due) in [
            (29, vec![]),
            (30, vec![RetentionStage::Notify]),
            (99, vec![RetentionStage::Notify]),
            (
                100,
                vec![
                    RetentionStage::Notify,
                    RetentionStage::ArchiveStudentRecords,
                ],
            ),
            (
                364,
                vec![
                    RetentionStage::Notify,
                    RetentionStage::ArchiveStudentRecords,
                ],
            ),
            (
                365,
                vec![
                    RetentionStage::Notify,
                    RetentionStage::ArchiveStudentRecords,
                    RetentionStage::DeleteStudentRecords,
                ],
            ),
        ] {
            let due = policy
                .due_stages_at(ended_at, timestamp(10_000 + offset))
                .expect("valid time");
            assert_eq!(due, expected_due, "day {offset}");
        }
        assert_eq!(
            policy.due_at(ended_at, RetentionStage::Notify),
            Ok(timestamp(10_030))
        );
        assert_eq!(
            policy.due_at(ended_at, RetentionStage::ArchiveStudentRecords),
            Ok(timestamp(10_100))
        );
        assert_eq!(
            policy.due_at(ended_at, RetentionStage::DeleteStudentRecords),
            Ok(timestamp(10_365))
        );
    }

    #[test]
    fn explicit_persisted_status_serializes_without_private_record_material() {
        let status = CourseRetentionStatus::from_persisted(
            CourseRetentionState::StudentRecordsArchived,
            AssignmentDefinitionDisposition::Retain,
        );
        assert_eq!(
            status.assignment_definitions,
            AssignmentDefinitionDisposition::Retain
        );
        assert_eq!(
            serde_json::to_value(status).expect("safe status serializes"),
            json!({
                "state": "studentRecordsArchived",
                "assignmentDefinitions": "retain"
            })
        );
        let deleted = CourseRetentionStatus::from_persisted(
            CourseRetentionState::StudentRecordsDeleted,
            AssignmentDefinitionDisposition::Retain,
        );
        assert_eq!(
            serde_json::to_value(deleted).expect("deleted status serializes"),
            json!({"state": "studentRecordsDeleted", "assignmentDefinitions": "retain"})
        );
        let private_identity = RetentionJobIdentity {
            course: CourseId::from_uuid(Uuid::from_u128(1)),
            stage: RetentionStage::Notify,
            generation: 7,
        };
        assert_eq!(private_identity.generation, 7);
        assert_eq!(private_identity.stage, RetentionStage::Notify);
        assert_eq!(private_identity.course.as_uuid(), Uuid::from_u128(1));
    }
}
