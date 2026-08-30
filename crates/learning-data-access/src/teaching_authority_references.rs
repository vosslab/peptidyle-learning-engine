//! Narrow authorized projections for T2 public locators.
//!
//! These locators are deliberately separate from `Store`: a reference is not
//! authority, and none of these methods offers an unscoped account or
//! membership resolver.

use async_trait::async_trait;
use question_model::teaching_operations::{
    CoInstructorTargetSearchPage, CoInstructorTargetSearchRequest, InstructorApprovalStateView,
    SysadminInstructorCandidateSearchPage, SysadminInstructorCandidateSearchRequest,
};
use question_model::{
    AccountReference, CoInstructorInvitationReference, CoInstructorInvitationState,
    CourseGroupReference, CourseId, CourseMembershipId, CourseMembershipReference,
    CourseMembershipRole, StudentId, UserId,
};

use crate::{
    ActorContext, CoInstructorInvitationRevision, CourseMemberStatus, InstructorApprovalRevision,
    Page, PageRequest, RosterRevision, SessionTokenHash, StoreError,
};

/// A session-bound self projection. It deliberately contains no email.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnAccountReferenceView {
    pub reference: AccountReference,
    pub display_name: String,
}

/// Exact-course Instructor projection of a co-instructor invitation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseCoInstructorInvitationReferenceView {
    pub reference: CoInstructorInvitationReference,
    pub target: AccountReference,
    pub target_display_name: String,
    /// Current operator eligibility, which remains distinct from course authority.
    pub target_approval_state: InstructorApprovalStateView,
    pub target_approval_revision: InstructorApprovalRevision,
    pub state: CoInstructorInvitationState,
    pub created_at: question_model::ActivityTimestamp,
    pub expires_at: question_model::ActivityTimestamp,
    pub revision: CoInstructorInvitationRevision,
}

/// Current direct Instructor row selected by an exact-course Instructor.
///
/// Every row is active, so the page roster revision is the exact `If-Match`
/// token accepted by direct-Instructor removal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseInstructorMembershipReferenceView {
    pub membership: CourseMembershipReference,
    pub account: AccountReference,
    pub account_display_name: String,
}

/// Bounded active direct-Instructor rows and their shared roster revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseInstructorMembershipReferencePage {
    pub page: Page<CourseInstructorMembershipReferenceView>,
    pub roster_revision: RosterRevision,
}

/// Target-only pending invitation projection. A target need not yet hold a
/// course membership, so the course is represented by its safe title only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCoInstructorInvitationReferenceView {
    pub reference: CoInstructorInvitationReference,
    pub course_title: String,
    pub expires_at: question_model::ActivityTimestamp,
    pub revision: CoInstructorInvitationRevision,
}

/// Exact-course Instructor projection for roster and group targeting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseMembershipReferenceView {
    pub reference: CourseMembershipReference,
    pub display_name: String,
    pub role: CourseMembershipRole,
    pub status: CourseMemberStatus,
}

/// Internal active-student identity selected by an exact-course Instructor.
///
/// This is intentionally not serialized or re-exported to browser contracts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructorStudentTargetView {
    pub course: CourseId,
    pub membership: CourseMembershipId,
    pub user: UserId,
    pub student: StudentId,
}

/// T2 public-locator boundary. Internal identifiers returned here are only
/// for the already-authorized server command that invoked the method.
#[async_trait]
pub trait TeachingAuthorityReferenceStore: Send + Sync {
    /// Finds accounts for a persisted Sysadmin session by a bounded display
    /// name fragment. Implementations authenticate the session and its stored
    /// Sysadmin role before parsing or resolving any account reference.
    async fn search_sysadmin_instructor_candidates(
        &self,
        _context: ActorContext,
        _session: SessionTokenHash,
        _request: SysadminInstructorCandidateSearchRequest,
    ) -> Result<SysadminInstructorCandidateSearchPage, StoreError> {
        Err(StoreError::Unavailable(
            "sysadmin instructor candidate search is not implemented by this store".to_owned(),
        ))
    }

    /// Finds eligible targets only for an already-authorized direct Instructor
    /// in one exact course. Implementations must reject empty/browse input,
    /// avoid email and internal identities, and mint public references only
    /// after course authority has been checked.
    async fn search_course_co_instructor_targets(
        &self,
        _context: ActorContext,
        _actor: UserId,
        _course: CourseId,
        _request: CoInstructorTargetSearchRequest,
    ) -> Result<CoInstructorTargetSearchPage, StoreError> {
        Err(StoreError::Unavailable(
            "co-instructor target search is not implemented by this store".to_owned(),
        ))
    }

    async fn own_account_reference(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
    ) -> Result<OwnAccountReferenceView, StoreError>;

    async fn resolve_account_reference_for_operator(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: AccountReference,
    ) -> Result<Option<UserId>, StoreError>;

    async fn resolve_approved_account_reference_for_course(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        reference: AccountReference,
    ) -> Result<Option<UserId>, StoreError>;

    async fn list_course_co_instructor_invitation_reference_views(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<CourseCoInstructorInvitationReferenceView>, StoreError>;

    async fn list_pending_co_instructor_invitation_reference_views(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<PendingCoInstructorInvitationReferenceView>, StoreError>;

    async fn list_course_membership_reference_views(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<CourseMembershipReferenceView>, StoreError>;

    /// Lists only active Student memberships for an exact-course direct
    /// Instructor. The projection intentionally contains no identity or
    /// contact fields beyond the safe membership reference and display facts.
    async fn list_course_active_student_membership_reference_views(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<CourseMembershipReferenceView>, StoreError>;

    async fn list_course_instructor_membership_reference_views(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        page: PageRequest,
    ) -> Result<CourseInstructorMembershipReferencePage, StoreError>;

    async fn list_course_group_membership_reference_views(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        group: CourseGroupReference,
        page: PageRequest,
    ) -> Result<Page<CourseMembershipReferenceView>, StoreError>;

    async fn course_membership_reference(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        membership: CourseMembershipId,
    ) -> Result<Option<CourseMembershipReference>, StoreError>;

    async fn resolve_course_membership_reference(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        reference: CourseMembershipReference,
    ) -> Result<Option<CourseMembershipId>, StoreError>;

    async fn resolve_active_student_target_reference(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        reference: CourseMembershipReference,
    ) -> Result<Option<InstructorStudentTargetView>, StoreError>;

    async fn active_student_membership_reference_view(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        student: StudentId,
    ) -> Result<Option<CourseMembershipReferenceView>, StoreError>;

    async fn co_instructor_invitation_reference(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        invitation: question_model::CoInstructorInvitationId,
    ) -> Result<Option<CoInstructorInvitationReference>, StoreError>;

    /// Resolves one current pending invitation for an already-authorized direct
    /// Instructor in the exact course. This is intentionally separate from
    /// the target-bound pending resolver below.
    async fn resolve_pending_course_co_instructor_invitation_reference(
        &self,
        context: ActorContext,
        actor: UserId,
        course: CourseId,
        reference: CoInstructorInvitationReference,
    ) -> Result<Option<question_model::CoInstructorInvitationId>, StoreError>;

    async fn resolve_pending_co_instructor_invitation_reference(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        reference: CoInstructorInvitationReference,
    ) -> Result<Option<question_model::CoInstructorInvitationId>, StoreError>;
}
