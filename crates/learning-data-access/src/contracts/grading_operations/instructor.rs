//! Instructor-facing metadata and recovery capability for grading operations.
//!
//! This module describes the bounded course operations surface. Its values are
//! deliberately limited to navigation, state, and durable action evidence;
//! accepted learner responses, feedback, and score material remain inside the
//! private execution capability.

use async_trait::async_trait;
use base64::Engine as _;
use question_model::{
    ActivityTimestamp, AssignmentId, AssignmentRevision, CourseId, CourseMembershipReference,
    GradingOperationAction, GradingOperationReason, GradingOperationReference,
    GradingOperationState, QuestionId, ScoringGeneration, TeachingDisplayLabel,
};

use super::{
    GradingExecutionGeneration, GradingOperationActionId, GradingOperationReceiptSafeCategory,
    GradingOperationRevision,
};
use crate::{ActorContext, Cursor, Page, PageRequest, SessionTokenHash, StoreError};

/// Maximum human-confirmed retries for one accepted-submission execution thread.
///
/// Store adapters share this limit so a course behaves consistently when its
/// persistence backend changes.
pub const MAX_INSTRUCTOR_GRADING_RETRY_COUNT: u16 = 20;

/// The closed arrangements available for an Instructor operations list.
///
/// A caller selects a learning-oriented grouping rather than providing a
/// storage field or ordering expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradingOperationGroupBy {
    Question,
    Learner,
}

/// Navigation identity paired with an operations-list row.
///
/// The assignment group represents assignment-wide recalculation work that is
/// not attributable to a single question or learner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GradingOperationGroup {
    Question {
        question_id: QuestionId,
        title: String,
    },
    Learner {
        membership: CourseMembershipReference,
        display_name: TeachingDisplayLabel,
    },
    Assignment,
}

/// Versioned opaque seek cursor for an Instructor operations list.
///
/// The cursor binds its continuation to one course, assignment, and
/// grouping arrangement, preventing reuse in a different instructional view.
pub(crate) struct GradingOperationCursor;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GradingOperationCursorSeek {
    pub group_key: String,
    pub operation: GradingOperationReference,
}

impl GradingOperationCursor {
    const VERSION: u8 = 1;

    pub(crate) fn encode(
        course: CourseId,
        assignment: AssignmentId,
        group_by: GradingOperationGroupBy,
        group: &GradingOperationGroup,
        operation: GradingOperationReference,
    ) -> Cursor {
        let group_key = operation_group_key(group);
        let mode = match group_by {
            GradingOperationGroupBy::Question => 1,
            GradingOperationGroupBy::Learner => 2,
        };
        let key_bytes = group_key.as_bytes();
        let key_len = u16::try_from(key_bytes.len()).expect("bounded group key fits cursor");
        let mut wire = Vec::with_capacity(40 + key_bytes.len());
        wire.extend_from_slice(&[Self::VERSION, mode]);
        wire.extend_from_slice(course.as_uuid().as_bytes());
        wire.extend_from_slice(assignment.as_uuid().as_bytes());
        wire.extend_from_slice(&key_len.to_be_bytes());
        wire.extend_from_slice(key_bytes);
        wire.extend_from_slice(&operation.number().to_be_bytes());
        Cursor::from_stable_key(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(wire))
    }

    pub(crate) fn decode(
        cursor: &Cursor,
        course: CourseId,
        assignment: AssignmentId,
        group_by: GradingOperationGroupBy,
    ) -> Result<GradingOperationCursorSeek, StoreError> {
        let wire = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(cursor.as_str())
            .map_err(|_| {
                StoreError::InvalidRecord("invalid grading operation cursor".to_string())
            })?;
        if wire.len() < 40 || wire[0] != Self::VERSION {
            return Err(StoreError::InvalidRecord(
                "invalid grading operation cursor".to_string(),
            ));
        }
        let expected_mode = match group_by {
            GradingOperationGroupBy::Question => 1,
            GradingOperationGroupBy::Learner => 2,
        };
        if wire[1] != expected_mode {
            return Err(StoreError::Conflict);
        }
        if wire[2..18] != *course.as_uuid().as_bytes()
            || wire[18..34] != *assignment.as_uuid().as_bytes()
        {
            return Err(StoreError::Conflict);
        }
        let key_len = usize::from(u16::from_be_bytes([wire[34], wire[35]]));
        if wire.len() != 40 + key_len {
            return Err(StoreError::InvalidRecord(
                "invalid grading operation cursor".to_string(),
            ));
        }
        let group_key = std::str::from_utf8(&wire[36..36 + key_len])
            .map_err(|_| StoreError::InvalidRecord("invalid grading operation cursor".to_string()))?
            .to_string();
        let offset = 36 + key_len;
        let operation = GradingOperationReference::new(u64::from(u32::from_be_bytes([
            wire[offset],
            wire[offset + 1],
            wire[offset + 2],
            wire[offset + 3],
        ])))
        .ok_or_else(|| StoreError::InvalidRecord("invalid grading operation cursor".to_string()))?;
        Ok(GradingOperationCursorSeek {
            group_key,
            operation,
        })
    }
}

/// Returns the stable internal sorting key for a list group.
pub(crate) fn operation_group_key(group: &GradingOperationGroup) -> String {
    match group {
        GradingOperationGroup::Question { question_id, .. } => {
            format!("q:{}", question_id.compact())
        }
        GradingOperationGroup::Learner { membership, .. } => {
            format!("l:{:010}", membership.number())
        }
        GradingOperationGroup::Assignment => "a".to_string(),
    }
}

/// Metadata-only identity and recovery state for one instructor-visible thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructorGradingOperationProjection {
    pub reference: GradingOperationReference,
    pub reason: GradingOperationReason,
    pub state: GradingOperationState,
    pub revision: GradingOperationRevision,
    pub next_action: Option<GradingOperationAction>,
}

/// The durable generation that makes an operation's current state trustworthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradingOperationTrustGeneration {
    Execution(GradingExecutionGeneration),
    AssignmentScoring(ScoringGeneration),
}

/// Safe, bounded metadata for one Instructor operations-list row.
///
/// The value supports recovery and audit navigation without carrying learner
/// responses, feedback, private source material, or scores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructorGradingOperationRow {
    pub operation: InstructorGradingOperationProjection,
    pub group: GradingOperationGroup,
    pub affected_learner_count: u32,
    pub trust_generation: GradingOperationTrustGeneration,
    pub stable_cursor: Cursor,
}

/// Course- and assignment-bound list request with an authenticated session
/// witness for the store's transaction-time authority check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListInstructorGradingOperationsCommand {
    pub session: SessionTokenHash,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub group_by: GradingOperationGroupBy,
    pub page: PageRequest,
}

/// Retry request assembled from authenticated route context and headers.
///
/// The store checks authority and the operation revision while holding the
/// state lock, so a stale recovery request cannot replace newer work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryGradingOperationCommand {
    pub session: SessionTokenHash,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub operation: GradingOperationReference,
    pub action: GradingOperationActionId,
    pub expected_revision: GradingOperationRevision,
}

/// Assignment recalculation request assembled from route and header evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecalculateAssignmentCommand {
    pub session: SessionTokenHash,
    pub course: CourseId,
    pub assignment: AssignmentId,
    pub action: GradingOperationActionId,
    pub expected_assignment_revision: AssignmentRevision,
}

/// Metadata-only receipt for a recovery action accepted by the store.
///
/// Retry and recalculation use separate revision namespaces, represented here
/// as distinct variants instead of a sentinel value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradingOperationActionReceipt {
    Retry {
        action: GradingOperationActionId,
        operation: GradingOperationReference,
        resulting_operation_revision: GradingOperationRevision,
        safe_category: GradingOperationReceiptSafeCategory,
        occurred_at: ActivityTimestamp,
    },
    Recalculation {
        action: GradingOperationActionId,
        operation: GradingOperationReference,
        resulting_operation_revision: GradingOperationRevision,
        assignment_revision: AssignmentRevision,
        scoring_generation: ScoringGeneration,
        safe_category: GradingOperationReceiptSafeCategory,
        occurred_at: ActivityTimestamp,
    },
}

/// Instructor capability for safe automated-grading recovery metadata.
///
/// The capability remains independent from the broad persistence and private
/// execution traits so composition can expose only course-scoped operations.
#[async_trait]
pub trait GradingOperationStore: Send + Sync {
    async fn list_instructor_grading_operations(
        &self,
        context: ActorContext,
        command: ListInstructorGradingOperationsCommand,
    ) -> Result<Page<InstructorGradingOperationRow>, StoreError>;

    async fn retry_instructor_grading_operation(
        &self,
        context: ActorContext,
        command: RetryGradingOperationCommand,
    ) -> Result<GradingOperationActionReceipt, StoreError>;

    async fn recalculate_instructor_assignment(
        &self,
        context: ActorContext,
        command: RecalculateAssignmentCommand,
    ) -> Result<GradingOperationActionReceipt, StoreError>;
}
