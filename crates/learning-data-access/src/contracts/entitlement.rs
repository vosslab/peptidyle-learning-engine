//! Current learner authority and durable educational-receipt materialization.
//!
//! An enrollment is historical evidence, never a continuing access grant.  A
//! backend evaluates the normalized current membership and assignment audience
//! in the same transaction that creates the first receipt for a legitimate
//! learner action.  Callers must use this capability rather than creating an
//! enrollment or inferring authority from a prior receipt.

use async_trait::async_trait;
use domain::entitlement::{ApplicablePolicyScopes, EntitlementDecision, EntitlementDenial};
use question_model::{
    AssignmentEnrollment, AssignmentId, CourseId, EntitlementMaterialization, EntitlementPurpose,
    MaterializationDisposition, StudentAssignmentSummary, UserId,
};

use crate::{AssignmentRecord, Page, PageRequest, StoreError, TenantContext};

/// One learner action that may require an educational receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterializeAssignmentEntitlementCommand {
    /// Learner whose present authority is evaluated.
    learner: UserId,
    /// Course selected by the route or issued work.
    course: CourseId,
    /// Assignment selected by the route or issued work.
    assignment: AssignmentId,
    /// The bounded action which justifies receipt creation.
    purpose: EntitlementPurpose,
    /// Closed provenance authority; never a caller-supplied free-form label.
    authority: question_model::MaterializationAuthority,
}

impl MaterializeAssignmentEntitlementCommand {
    /// Builds a learner-owned command.  Learner events cannot forge another
    /// actor into immutable provenance.
    pub(crate) fn for_learner_action(
        learner: UserId,
        course: CourseId,
        assignment: AssignmentId,
        purpose: EntitlementPurpose,
    ) -> Result<Self, StoreError> {
        matches!(
            purpose,
            EntitlementPurpose::StartRun | EntitlementPurpose::GradeBearingAction
        )
        .then_some(Self {
            learner,
            course,
            assignment,
            purpose,
            authority: question_model::MaterializationAuthority::Actor(learner),
        })
        .ok_or_else(|| {
            StoreError::InvalidRecord("instructor issue requires an instructor command".to_string())
        })
    }

    /// Builds an instructor-authorized command.  Instructor issue and
    /// grade-bearing actions retain the actual instructor in provenance.  The
    /// Store validates current exact-course instructor membership inside its
    /// transaction, because constructor validation has no access to that
    /// state.
    pub fn for_instructor_action(
        learner: UserId,
        course: CourseId,
        assignment: AssignmentId,
        instructor: UserId,
        purpose: EntitlementPurpose,
    ) -> Result<Self, StoreError> {
        matches!(
            purpose,
            EntitlementPurpose::InstructorIssue | EntitlementPurpose::GradeBearingAction
        )
        .then_some(Self {
            learner,
            course,
            assignment,
            purpose,
            authority: question_model::MaterializationAuthority::Actor(instructor),
        })
        .ok_or_else(|| {
            StoreError::InvalidRecord("start-run requires a learner command".to_string())
        })
    }

    /// Builds a system rule-backed grade operation.  Rules can never start a
    /// run or issue instructor work.
    pub fn for_rule_grade(
        learner: UserId,
        course: CourseId,
        assignment: AssignmentId,
        rule: question_model::MaterializationRule,
    ) -> Self {
        Self {
            learner,
            course,
            assignment,
            purpose: EntitlementPurpose::GradeBearingAction,
            authority: question_model::MaterializationAuthority::Rule(rule),
        }
    }

    pub fn learner(&self) -> UserId {
        self.learner
    }
    pub fn course(&self) -> CourseId {
        self.course
    }
    pub fn assignment(&self) -> AssignmentId {
        self.assignment
    }
    pub fn purpose(&self) -> EntitlementPurpose {
        self.purpose
    }
    pub fn authority(&self) -> question_model::MaterializationAuthority {
        self.authority
    }
}

/// A successful, atomic entitlement evaluation and receipt materialization.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterializedAssignmentEntitlement {
    /// Immutable educational receipt, newly created or previously retained.
    pub enrollment: AssignmentEnrollment,
    /// Explicit derived projection maintained beside the receipt.
    pub summary: StudentAssignmentSummary,
    /// Immutable evaluator provenance for the receipt's first materialization.
    pub provenance: EntitlementMaterialization,
    /// Whether this operation inserted the receipt or found the one it owns.
    pub disposition: MaterializationDisposition,
    /// Current evaluator-approved scopes, for the still-owned S3 timing
    /// resolver.  This is not reconstructible by consumers.
    pub applicable_policy_scopes: ApplicablePolicyScopes,
}

/// Result of evaluating present learner authority.
///
/// Denial deliberately contains no receipt, summary, or provenance; a denied
/// action must leave educational records untouched.  The typed reason is for
/// internal policy composition and must not be serialized as a learner DTO.
#[allow(clippy::large_enum_variant)] // Callers inspect denials often; boxing grants obscures the seam.
#[derive(Debug, Clone, PartialEq)]
pub enum AssignmentEntitlementMaterialization {
    Granted(MaterializedAssignmentEntitlement),
    Denied(EntitlementDenial),
}

/// Atomic evaluator and receipt seam consumed by every learner start,
/// grade-bearing action, and instructor-issued learner operation.
#[async_trait]
pub trait EntitlementStore: Send + Sync {
    /// Lists only assignments the learner may presently access.  Filtering and
    /// pagination happen after the one evaluator-owned authority decision, so
    /// callers cannot turn a historical enrollment into list visibility.
    async fn list_learner_entitled_assignments_impl(
        &self,
        context: TenantContext,
        learner: UserId,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError>;

    /// Evaluates present authority without creating an educational record.
    /// Learner lists use this query; they never infer visibility from an old
    /// receipt or reconstruct roster/group policy themselves.
    async fn evaluate_assignment_entitlement_impl(
        &self,
        context: TenantContext,
        learner: UserId,
        course: CourseId,
        assignment: AssignmentId,
    ) -> Result<EntitlementDecision, StoreError>;

    /// Explicit instructor issuance or a closed system-grade rule only.
    /// Learner actions materialize internally in their owning Store transaction.
    async fn issue_assignment_entitlement_impl(
        &self,
        context: TenantContext,
        command: MaterializeAssignmentEntitlementCommand,
    ) -> Result<AssignmentEntitlementMaterialization, StoreError>;
}
