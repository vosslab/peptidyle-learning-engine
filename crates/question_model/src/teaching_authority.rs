//! Shared, internal target-bound Course Invitation facts.
//!
//! These are persistence contracts, not browser projections. Later HTTP
//! contracts must expose only authorized, opaque course-scoped actions and use
//! `deny_unknown_fields` on every browser-facing shape.

use uuid::Uuid;

use crate::{AccountId, CourseId, CourseMembershipId, CourseMembershipRole, Timestamp};

/// Stable internal identifier for one target-bound Course Invitation.
///
/// It has no display implementation because it is never a user-facing
/// Reference. Later HTTP contracts should use a course-scoped opaque action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CourseInvitationId(Uuid);

impl CourseInvitationId {
    /// Wraps an internal identifier read from storage.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the identifier for storage and transaction lookup only.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Closed lifecycle state of one target-bound Course Invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseInvitationState {
    Pending,
    Expired,
    Accepted,
    Declined,
    Revoked,
}

/// One immutable terminal transition for an exact Course Invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseInvitationEvent {
    /// Exact invitation whose state changes.
    pub invitation: CourseInvitationId,
    /// Closed terminal transition selected once for the invitation.
    pub kind: CourseInvitationEventKind,
    /// Account that performed the transition.
    pub performed_by: AccountId,
    /// Authoritative transition time.
    pub occurred_at: Timestamp,
}

/// Closed terminal transitions for a Course Invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseInvitationEventKind {
    Accepted,
    Declined,
    Revoked,
}

/// Target-bound Course Invitation facts persisted by a later Store.
///
/// No email field exists: acceptance is available to the authenticated target
/// account. The 30-day expiry is validated by pure domain code with a supplied
/// authoritative time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseInvitation {
    /// Internal storage identity, never a visible reference.
    pub id: CourseInvitationId,
    /// Exact course that can receive one ordinary direct membership.
    pub course: CourseId,
    /// Exact direct Instructor membership episode that initiated the invitation.
    pub invited_by: CourseMembershipId,
    /// Exact Course Membership Role granted if the target accepts this invitation.
    pub membership_role: CourseMembershipRole,
    /// Existing account invited to this exact course with that exact role.
    pub target: AccountId,
    /// Authoritative creation time.
    pub created_at: Timestamp,
    /// Required authoritative expiry, exactly 30 days after creation.
    pub expires_at: Timestamp,
    /// The one persisted terminal event, if a transition occurred.
    ///
    /// Its absence derives Pending or Expired from `expires_at`; it is not a
    /// mutable invitation-state field.
    pub terminal_event: Option<CourseInvitationEvent>,
}
