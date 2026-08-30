//! Solution-free, audited Instructor inspection of one Student's submitted run.
//!
//! This contract names only public route locators.  Implementations resolve the
//! full course/membership/assignment/run composite before reading a
//! private response.  ASVS 8.2.1, 8.3.1, and 14.1.1 apply at this boundary.

use async_trait::async_trait;
use question_model::{
    ActivityTimestamp, AssignmentId, AssignmentReference, CourseId, CourseMembershipId,
    CourseMembershipReference, CourseReference, InspectedStudentScoreFeedbackV1, QuestionAttemptId,
    RunId, RunReference, ScoringGeneration, ScoringStatus, TeachingDisplayLabel,
    presentation::{InspectedStudentResponseV1, PresentationDigestV1},
};

use crate::{ActorContext, ReceiptPresentationSnapshot, SessionTokenHash, StoreError};

/// Server-owned purpose for the paired successful-record audit writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudentWorkInspectionAuditIntent {
    /// Records an Instructor's successful Gradebook-initiated inspection.
    GradebookInspection,
}

/// Closed caller return destination.  Detail responses never accept a URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudentWorkInspectionReturnContext {
    Gradebook {
        /// Course that owns the restored Gradebook cell.
        course: CourseReference,
        /// Student membership represented by the restored cell.
        membership: CourseMembershipReference,
        /// Assignment represented by the restored cell.
        assignment: AssignmentReference,
        /// Exact browser focus target after return.
        focus: StudentWorkInspectionFocusTarget,
    },
    GradingOperation {
        /// Course that owns the restored grading operation.
        course: CourseReference,
        /// Student membership selected by the operation.
        membership: CourseMembershipReference,
        /// Assignment selected by the operation.
        assignment: AssignmentReference,
        /// Operation control that opened the inspection.
        operation: question_model::GradingOperationReference,
        /// Exact browser focus target after return.
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
    /// Public reference for the selected course.
    pub course: CourseReference,
    /// Public reference for the selected Student membership.
    pub membership: CourseMembershipReference,
    /// Public reference for the selected assignment.
    pub assignment: AssignmentReference,
    /// Public reference for the selected Student run.
    pub run: RunReference,
    /// Closed return destination and focus target.
    pub return_context: StudentWorkInspectionReturnContext,
}

/// One immutable submitted response in the selected run.
#[derive(Clone, PartialEq)]
pub struct InspectedStudentSubmissionV1 {
    /// Internal immutable attempt identity used only for deterministic ordering.
    pub(crate) attempt: QuestionAttemptId,
    /// Immutable assignment position used only for deterministic ordering.
    pub(crate) assignment_position: u32,
    /// The immutable receipt timestamp for this submitted response.
    pub submitted_at: ActivityTimestamp,
    /// Closed evidence proving how this response may be interpreted.
    pub evidence: InspectedSubmissionEvidenceV1,
    /// The page-local scoring generation observed with this detail read.
    pub scoring_generation: ScoringGeneration,
    /// Current score/correctness-only feedback with no instructional content.
    pub feedback: InspectedStudentScoreFeedbackV1,
    /// Safe rendering of the Student's submitted response, bound to issued IDs.
    pub response: InspectedStudentResponseV1,
    /// The current status for the assignment scoring generation.
    pub scoring_status: ScoringStatus,
}

/// Browser-safe immutable evidence for one inspected Student submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectedSubmissionEvidenceV1 {
    /// A receipt retained and verified one issued presentation.
    IssuedPresentation {
        /// Exact answer-free issued presentation snapshot.
        presentation: Box<ReceiptPresentationSnapshot>,
        /// Recomputed digest of that issued presentation.
        issued_presentation_digest: PresentationDigestV1,
    },
    /// The immutable issued capability correctly has no presentation.
    PresentationNotApplicable,
}

/// Internal receipt witness retained in the paired protected audit facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudentWorkInspectionSubmissionWitness {
    /// Internal immutable attempt identity selected by the completed receipt.
    pub attempt: QuestionAttemptId,
    /// Immutable receipt timestamp for the submitted attempt.
    pub submitted_at: ActivityTimestamp,
    /// Closed evidence classification retained without response content.
    pub evidence: StudentWorkInspectionEvidenceWitness,
}

/// Internal evidence witness for one inspected submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudentWorkInspectionEvidenceWitness {
    /// A verified issued-presentation digest without the presentation payload.
    IssuedPresentation { digest: PresentationDigestV1 },
    /// A verified ExternalTool receipt whose issued capability has no presentation.
    PresentationNotApplicable,
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
    /// Course owning this protected Student record.
    pub course: CourseId,
    /// Student membership selected by the Instructor.
    pub membership: CourseMembershipId,
    /// Assignment selected by the Instructor.
    pub assignment: AssignmentId,
    /// Run selected by the Instructor.
    pub run: RunId,
    /// Verified immutable submission evidence in total inspection order.
    pub submissions: Vec<StudentWorkInspectionSubmissionWitness>,
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
    /// Fixed server-owned purpose for this successful read.
    pub intent: StudentWorkInspectionAuditIntent,
    /// Authoritative timestamp shared with the paired access fact.
    pub occurred_at: ActivityTimestamp,
    /// Course selected by the Instructor.
    pub course: CourseId,
    /// Student membership selected by the Instructor.
    pub membership: CourseMembershipId,
    /// Assignment selected by the Instructor.
    pub assignment: AssignmentId,
    /// Run selected by the Instructor.
    pub run: RunId,
    /// Verified immutable submission evidence in total inspection order.
    pub submissions: Vec<StudentWorkInspectionSubmissionWitness>,
    /// Server-owned scoring generation.
    pub scoring_generation: ScoringGeneration,
    /// Server-owned scoring status.
    pub scoring_status: ScoringStatus,
}

/// Closed, solution-free inspection detail.
#[derive(Clone, PartialEq)]
pub struct InspectedStudentWorkDetailV1 {
    /// Selected course reference.
    pub course: CourseReference,
    /// Selected Student membership reference.
    pub membership: CourseMembershipReference,
    /// Selected assignment reference.
    pub assignment: AssignmentReference,
    /// Selected run reference.
    pub run: RunReference,
    /// Current, validated display label for the inspected active Student.
    ///
    /// This presentation fact is resolved from the active course roster after
    /// the exact inspection composite is authorized; it is not audit evidence.
    pub student_display_label: TeachingDisplayLabel,
    /// Current title for the inspected course assignment.
    ///
    /// This presentation fact is resolved from the authorized assignment and
    /// is not immutable submission or audit evidence.
    pub assignment_title: String,
    /// Receipt-verified submissions in submitted-at order.
    pub submissions: Vec<InspectedStudentSubmissionV1>,
    /// Closed destination used when the client returns from this detail.
    pub return_context: StudentWorkInspectionReturnContext,
}

impl std::fmt::Debug for InspectedStudentSubmissionV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InspectedStudentSubmissionV1")
            .field("evidence", &"[REDACTED]")
            .field("scoring_generation", &self.scoring_generation)
            .field("scoring_status", &self.scoring_status)
            .field("response", &"[REDACTED]")
            .field("feedback", &"[REDACTED]")
            .finish()
    }
}

impl std::fmt::Debug for InspectedStudentWorkDetailV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InspectedStudentWorkDetailV1")
            .field("submission_count", &self.submissions.len())
            .field("submissions", &"[REDACTED]")
            .finish()
    }
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
        context: ActorContext,
        session: SessionTokenHash,
        request: InspectStudentWorkRequest,
    ) -> Result<InspectedStudentWorkDetailV1, StoreError>;
}
