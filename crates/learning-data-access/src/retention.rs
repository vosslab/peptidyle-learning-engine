//! Narrow executor-only boundary for stored Course-retention work.
//!
//! PostgreSQL owns the schedule and transition predicates.  This Store neither
//! calculates dates nor sends notices: it can read already-due action records
//! and invoke the one-way inactivity, archive, and deletion procedures.

use async_trait::async_trait;
use question_model::{CourseInstanceId, Timestamp};

use crate::StoreError;

/// One stored retention action that is due at the supplied database instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseRetentionDueAction {
    /// The Course whose stored schedule produced this action.
    pub course: CourseInstanceId,
    /// Closed action kind selected by the database policy function.
    pub action: CourseRetentionDueActionKind,
    /// Database-authoritative due instant.
    pub due_at: Timestamp,
    /// Present only after the Course's Student records have been archived.
    pub archive_marked_at: Option<Timestamp>,
}

/// Closed stored retention action kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseRetentionDueActionKind {
    /// Persist the Course's due Active-to-Inactive transition.
    MarkInactive,
    WarnInactive,
    NotifyArchive,
    Archive,
    Delete,
}

/// Least-privilege persistence capability for the retention executor.
///
/// The worker will own notification delivery and iteration policy.  This
/// interface carries no process-clock calculation or notification operation.
#[async_trait]
pub trait CourseRetentionStore: Send + Sync {
    /// Reads actions already due at the explicit database-attested instant.
    async fn list_due_course_retention_actions(
        &self,
        evaluated_at: Timestamp,
    ) -> Result<Vec<CourseRetentionDueAction>, StoreError>;

    /// Commits inactivity at the stored Active cutoff, independently of retention.
    ///
    /// Returns false when already Inactive; an early instant or unavailable Course
    /// returns a Store error without changing the Course.
    async fn mark_course_instance_inactive(
        &self,
        course: CourseInstanceId,
        evaluated_at: Timestamp,
    ) -> Result<bool, StoreError>;

    /// Commits the database-owned archive transition for one Course.
    async fn archive_course_student_records(
        &self,
        course: CourseInstanceId,
        evaluated_at: Timestamp,
    ) -> Result<bool, StoreError>;

    /// Commits the database-owned deletion transition for one Course.
    async fn delete_course_student_records(
        &self,
        course: CourseInstanceId,
        evaluated_at: Timestamp,
    ) -> Result<bool, StoreError>;
}
