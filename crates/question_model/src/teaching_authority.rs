//! Shared, internal approval and target-bound co-instructor invitation facts.
//!
//! These are persistence contracts, not browser projections. Later HTTP
//! contracts must expose only authorized, opaque course-scoped actions and use
//! `deny_unknown_fields` on every browser-facing shape.

use uuid::Uuid;

use crate::{AccountId, ActivityTimestamp, CourseId, CourseMembershipId};

/// Operator-owned global eligibility for Instructor invitations.
///
/// This record is intentionally separate from both `AccountRole` and direct
/// course membership. Possessing it grants no course authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructorApproval {
    /// Existing user approved by a platform operator.
    pub account: AccountId,
    /// Operator account that recorded this approval.
    pub approved_by: AccountId,
    /// Authoritative operator time of approval.
    pub approved_at: ActivityTimestamp,
    /// Authoritative revocation time when eligibility is no longer active.
    pub revoked_at: Option<ActivityTimestamp>,
}

/// Stable internal identifier for a target-bound co-instructor invitation.
///
/// It has no display implementation because it is never a user-facing
/// locator. Later HTTP contracts should use a course-scoped opaque action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CoInstructorInvitationId(Uuid);

impl CoInstructorInvitationId {
    /// Wraps an internal identifier read from storage.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the identifier for storage and transaction lookup only.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Closed lifecycle state of a target-bound co-instructor invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoInstructorInvitationState {
    Pending,
    Expired,
    Accepted,
    Declined,
    Revoked,
}

/// Target-bound co-instructor invitation facts persisted by a later Store.
///
/// No email field exists: acceptance is available to the authenticated target
/// account. The 30-day expiry is validated by pure domain code with a supplied
/// authoritative time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoInstructorInvitation {
    /// Internal storage identity, never a visible reference.
    pub id: CoInstructorInvitationId,
    /// Exact course that can receive one ordinary direct membership.
    pub course: CourseId,
    /// Exact direct Instructor membership episode that initiated the invitation.
    pub invited_by: CourseMembershipId,
    /// Existing approved account invited to this exact course.
    pub target: AccountId,
    /// Authoritative creation time.
    pub created_at: ActivityTimestamp,
    /// Required authoritative expiry, exactly 30 days after creation.
    pub expires_at: ActivityTimestamp,
    /// Authoritative acceptance time, if accepted.
    pub accepted_at: Option<ActivityTimestamp>,
    /// Authoritative target-decline time, if the target declined.
    pub declined_at: Option<ActivityTimestamp>,
    /// Authoritative revocation time, if revoked.
    pub revoked_at: Option<ActivityTimestamp>,
}
