//! Solution-free, audited Instructor inspection of one Student's submitted run.
//!
//! This contract names only public route locators.  Implementations resolve the
//! full tenant/course/membership/assignment/run composite before reading a
//! private response.  ASVS 8.2.1, 8.3.1, and 14.1.1 apply at this boundary.

use async_trait::async_trait;
use question_model::{
    ActivityTimestamp, AssignmentReference, CourseMembershipReference, CourseReference,
    RunReference, ScoringGeneration, ScoringStatus,
    presentation::{InspectedStudentResponseV1, PresentationDigestV1},
};

use crate::{ReceiptPresentationSnapshot, SessionTokenHash, StoreError, TenantContext};

/// Server-owned purpose for the paired successful-record audit writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudentWorkInspectionAuditIntent {
    GradebookInspection,
}

/// Closed caller return destination.  Detail responses never accept a URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudentWorkInspectionReturnContext {
    Gradebook {
        course: CourseReference,
        membership: CourseMembershipReference,
        assignment: AssignmentReference,
        focus: StudentWorkInspectionFocusTarget,
    },
    GradingOperation {
        course: CourseReference,
        membership: CourseMembershipReference,
        assignment: AssignmentReference,
        operation: question_model::GradingOperationReference,
        focus: StudentWorkInspectionFocusTarget,
    },
}

/// Closed focus target restored after a no-store inspection detail return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudentWorkInspectionFocusTarget {
    /// Restore the Gradebook cell for the resolved Student and assignment.
    GradebookCell {
        /// The exact Student membership represented by the cell.
        membership: CourseMembershipReference,
        /// The exact assignment represented by the cell.
        assignment: AssignmentReference,
    },
    /// Restore the grading-operation control that opened the detail.
    GradingOperationControl {
        /// The exact Student membership resolved for this inspection.
        membership: CourseMembershipReference,
        /// The exact assignment resolved for this inspection.
        assignment: AssignmentReference,
        /// The operation control to restore.
        operation: question_model::GradingOperationReference,
    },
}

/// Exact public composite selected by an Instructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectStudentWorkRequest {
    pub course: CourseReference,
    pub membership: CourseMembershipReference,
    pub assignment: AssignmentReference,
    pub run: RunReference,
    pub return_context: StudentWorkInspectionReturnContext,
}

/// One immutable submitted response in the selected run.
#[derive(Debug, Clone, PartialEq)]
pub struct InspectedStudentSubmissionV1 {
    /// The immutable receipt timestamp for this submitted response.
    pub submitted_at: ActivityTimestamp,
    /// The exact issued presentation retained by the immutable receipt.
    pub presentation: ReceiptPresentationSnapshot,
    /// The server-verified digest of the issued presentation.
    pub issued_presentation_digest: PresentationDigestV1,
    /// The page-local scoring generation observed with this detail read.
    pub scoring_generation: ScoringGeneration,
    /// The page-local scoring state observed with this detail read.
    pub response: InspectedStudentResponseV1,
    /// The current status for the assignment scoring generation.
    pub scoring_status: ScoringStatus,
}

/// Closed successful Student-record access fact retained with an inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudentWorkInspectionRecordAccess {
    /// Instructor identity authorized for this read.
    pub actor: question_model::UserId,
    /// Fixed server-owned purpose for the read.
    pub intent: StudentWorkInspectionAuditIntent,
    /// Authoritative timestamp for the paired facts.
    pub occurred_at: ActivityTimestamp,
    /// Course selected by the Instructor.
    pub course: CourseReference,
    /// Student membership selected by the Instructor.
    pub membership: CourseMembershipReference,
    /// Assignment selected by the Instructor.
    pub assignment: AssignmentReference,
    /// Run selected by the Instructor.
    pub run: RunReference,
    /// Verified issued-presentation digests in submitted-at order.
    pub issued_presentation_digests: Vec<PresentationDigestV1>,
    /// Server-owned scoring generation.
    pub scoring_generation: ScoringGeneration,
    /// Server-owned scoring status.
    pub scoring_status: ScoringStatus,
}

/// Metadata-only successful-record audit fact retained by the Memory oracle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudentWorkInspectionAudit {
    /// Instructor identity authorized for this read.
    pub actor: question_model::UserId,
    pub intent: StudentWorkInspectionAuditIntent,
    pub occurred_at: ActivityTimestamp,
    pub course: CourseReference,
    pub membership: CourseMembershipReference,
    pub assignment: AssignmentReference,
    pub run: RunReference,
    /// Verified issued-presentation digests in submitted-at order.
    pub issued_presentation_digests: Vec<PresentationDigestV1>,
    /// Server-owned scoring generation.
    pub scoring_generation: ScoringGeneration,
    /// Server-owned scoring status.
    pub scoring_status: ScoringStatus,
}

/// Closed, solution-free inspection detail.
#[derive(Debug, Clone, PartialEq)]
pub struct InspectedStudentWorkDetailV1 {
    pub course: CourseReference,
    pub membership: CourseMembershipReference,
    pub assignment: AssignmentReference,
    pub run: RunReference,
    pub submissions: Vec<InspectedStudentSubmissionV1>,
    pub return_context: StudentWorkInspectionReturnContext,
}

/// Store capability for the one audit-recorded Student-work detail read.
///
/// The method's fixed audit intent keeps application callers from supplying an
/// action, actor, target, or audit payload.  Implementations expose generic
/// `NotFound` for concealed authorization and evidence failures.
#[async_trait]
pub trait StudentWorkInspectionStore: Send + Sync {
    async fn inspect_student_work(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: InspectStudentWorkRequest,
    ) -> Result<InspectedStudentWorkDetailV1, StoreError>;
}
