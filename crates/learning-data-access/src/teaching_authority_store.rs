//! Narrow persistence contract for operator approvals and co-instructor invitations.
//!
//! This capability is deliberately not composed into `Store`: PostgreSQL must
//! implement the reserved T2 migration before server code can depend on it.

use async_trait::async_trait;
use question_model::{
    CoInstructorInvitation, CoInstructorInvitationId, CourseId, CourseMembershipId,
    InstructorApproval, UserId,
};

use crate::{Page, PageRequest, RosterRevision, SessionTokenHash, StoreError, TenantContext};

macro_rules! positive_revision {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(u64);

        impl $name {
            pub const INITIAL: Self = Self(1);

            pub fn next(self) -> Result<Self, StoreError> {
                let value = self
                    .0
                    .checked_add(1)
                    .filter(|value| *value <= i64::MAX as u64)
                    .ok_or_else(|| {
                        StoreError::Unavailable(concat!($description, " exhausted").to_string())
                    })?;
                Ok(Self(value))
            }

            /// Converts the positive, PostgreSQL-safe storage representation.
            pub fn as_i64(self) -> i64 {
                self.0 as i64
            }

            /// Validates one persisted PostgreSQL revision before constructing it.
            pub fn try_from_i64(value: i64) -> Result<Self, StoreError> {
                u64::try_from(value)
                    .ok()
                    .filter(|value| *value > 0)
                    .map(Self)
                    .ok_or_else(|| {
                        StoreError::InvalidRecord(
                            concat!($description, " must be positive").to_string(),
                        )
                    })
            }
        }
    };
}

positive_revision!(InstructorApprovalRevision, "Instructor approval revision");
positive_revision!(
    CoInstructorInvitationRevision,
    "Co-instructor invitation revision"
);

/// Approval record with its strong revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredInstructorApproval {
    pub approval: InstructorApproval,
    pub revision: InstructorApprovalRevision,
}

/// Revisioned target-bound invitation facts. This is internal Store data, not a DTO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCoInstructorInvitation {
    pub invitation: CoInstructorInvitation,
    pub revision: CoInstructorInvitationRevision,
}

/// Operator request to approve an existing account as invitation-eligible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApproveInstructorAccount {
    /// Persisted authenticated operator session; role claims are resolved by the Store.
    pub session: SessionTokenHash,
    pub target: UserId,
    pub expected_revision: Option<InstructorApprovalRevision>,
}

/// Operator request to revoke invitation eligibility with optimistic concurrency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevokeInstructorApproval {
    /// Persisted authenticated operator session; role claims are resolved by the Store.
    pub session: SessionTokenHash,
    pub target: UserId,
    pub expected_revision: InstructorApprovalRevision,
}

/// Direct-course-Instructor request for one target-bound invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateCoInstructorInvitation {
    /// Persisted authenticated instructor session; the Store derives the actor from it.
    pub session: SessionTokenHash,
    pub actor: UserId,
    pub course: CourseId,
    pub target: UserId,
}

/// Target-only terminal invitation command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RespondToCoInstructorInvitation {
    /// Persisted target session.  The Store derives and verifies the target from it.
    ///
    /// ASVS 8.3.1-8.3.3: a route's displayed actor is not authority for a
    /// terminal invitation transition.
    pub session: SessionTokenHash,
    pub actor: UserId,
    pub invitation: CoInstructorInvitationId,
    pub expected_revision: CoInstructorInvitationRevision,
}

/// Direct Instructor request to revoke a still-pending invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevokeCoInstructorInvitation {
    /// Persisted authenticated instructor session; the Store derives the actor from it.
    pub session: SessionTokenHash,
    pub actor: UserId,
    pub course: CourseId,
    pub invitation: CoInstructorInvitationId,
    pub expected_revision: CoInstructorInvitationRevision,
}

/// Current direct Instructor membership plus the course roster revision required for removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectInstructorMembershipView {
    pub membership: CourseMembershipId,
    pub course: CourseId,
    pub user: UserId,
    pub roster_revision: RosterRevision,
}

/// Explicit direct Instructor membership removal. The actor may remove self or
/// another instructor, but the final active Instructor is never removable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveDirectInstructorMembership {
    pub session: SessionTokenHash,
    pub actor: UserId,
    pub course: CourseId,
    pub membership: CourseMembershipId,
    pub expected_roster_revision: RosterRevision,
}

/// T2 authority capability, intentionally separate from broad `Store`.
#[async_trait]
pub trait TeachingAuthorityStore: Send + Sync {
    async fn approve_instructor_account(
        &self,
        context: TenantContext,
        command: ApproveInstructorAccount,
    ) -> Result<StoredInstructorApproval, StoreError>;
    async fn revoke_instructor_approval(
        &self,
        context: TenantContext,
        command: RevokeInstructorApproval,
    ) -> Result<StoredInstructorApproval, StoreError>;
    async fn create_co_instructor_invitation(
        &self,
        context: TenantContext,
        command: CreateCoInstructorInvitation,
    ) -> Result<StoredCoInstructorInvitation, StoreError>;
    async fn list_course_co_instructor_invitations(
        &self,
        context: TenantContext,
        actor: UserId,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<StoredCoInstructorInvitation>, StoreError>;
    async fn list_pending_co_instructor_invitations(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<StoredCoInstructorInvitation>, StoreError>;
    async fn accept_co_instructor_invitation(
        &self,
        context: TenantContext,
        command: RespondToCoInstructorInvitation,
    ) -> Result<DirectInstructorMembershipView, StoreError>;
    async fn decline_co_instructor_invitation(
        &self,
        context: TenantContext,
        command: RespondToCoInstructorInvitation,
    ) -> Result<(), StoreError>;
    async fn revoke_co_instructor_invitation(
        &self,
        context: TenantContext,
        command: RevokeCoInstructorInvitation,
    ) -> Result<(), StoreError>;
    async fn remove_direct_instructor_membership(
        &self,
        context: TenantContext,
        command: RemoveDirectInstructorMembership,
    ) -> Result<(), StoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revisions_reject_nonpositive_postgres_values() {
        for value in [i64::MIN, -1, 0] {
            assert!(InstructorApprovalRevision::try_from_i64(value).is_err());
            assert!(CoInstructorInvitationRevision::try_from_i64(value).is_err());
        }
        assert_eq!(
            InstructorApprovalRevision::try_from_i64(1)
                .unwrap()
                .as_i64(),
            1
        );
        assert_eq!(
            CoInstructorInvitationRevision::try_from_i64(i64::MAX)
                .unwrap()
                .as_i64(),
            i64::MAX
        );
    }

    #[test]
    fn revisions_stop_at_postgres_i64_ceiling() {
        let maximum = InstructorApprovalRevision::try_from_i64(i64::MAX).unwrap();
        assert!(maximum.next().is_err());
    }
}
