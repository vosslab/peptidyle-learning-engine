//! Durable, provider-neutral delivery intent for one course invitation.
//!
//! This contract intentionally records a correlation identifier, not an SMTP
//! idempotency promise.  A lost response after SMTP `DATA` is ambiguous and
//! must never be retried automatically.

use async_trait::async_trait;
use question_model::{ActivityTimestamp, CourseId};
use uuid::Uuid;

use crate::{
    ActorContext, CourseInvitationId, CourseRosterId, RosterIdempotencyKey, SessionTokenHash,
    StoreError,
};

macro_rules! delivery_id_type {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Uuid);

        impl $name {
            pub fn generate() -> Result<Self, StoreError> {
                crate::random_uuid::random_uuid_v4(|error| {
                    StoreError::Unavailable(format!("{} randomness unavailable: {error}", $label))
                })
                .map(Self)
            }

            pub fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }
    };
}

delivery_id_type!(CourseInvitationDeliveryId, "course invitation delivery ID");

/// Private per-attempt capability encoded in the existing UUID storage column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CourseInvitationDeliveryLeaseId(Uuid);

impl CourseInvitationDeliveryLeaseId {
    pub fn generate() -> Result<Self, StoreError> {
        crate::random_uuid::random_128_bits(|error| {
            StoreError::Unavailable(format!(
                "course invitation delivery lease ID randomness unavailable: {error}"
            ))
        })
        .map(crate::random_uuid::uuid_storage_from_128_random_bits)
        .map(Self)
    }

    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

/// Deliberately closed retry budget. A new send requires a fresh invitation
/// after this many leased attempts.
pub const MAX_COURSE_INVITATION_DELIVERY_ATTEMPTS: u32 = 3;

/// Closed durable delivery state.  Only pending and retryable failure may be
/// claimed; ambiguous results deliberately require an explicit resend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseInvitationDeliveryState {
    Pending,
    AcceptedByProvider,
    RetryableFailed,
    Ambiguous,
    PermanentFailed,
    Cancelled,
}

/// Safe, closed diagnostic category.  Raw provider responses never enter the
/// Store contract, logs, metrics, or queue rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseInvitationDeliveryOutcomeCode {
    Accepted,
    TemporaryFailure,
    PermanentFailure,
    AmbiguousTransport,
    Cancelled,
}

/// One durable outbox row, keyed one-to-one by its invitation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseInvitationDelivery {
    pub course: CourseId,
    pub invitation: CourseInvitationId,
    pub id: CourseInvitationDeliveryId,
    pub state: CourseInvitationDeliveryState,
    pub attempt_count: u32,
    pub next_attempt_at: ActivityTimestamp,
    pub last_attempt_at: Option<ActivityTimestamp>,
    pub lease: Option<CourseInvitationDeliveryLeaseId>,
    pub lease_expires_at: Option<ActivityTimestamp>,
    pub dispatch_started_at: Option<ActivityTimestamp>,
    pub outcome_code: Option<CourseInvitationDeliveryOutcomeCode>,
    pub created_at: ActivityTimestamp,
    pub updated_at: ActivityTimestamp,
    pub accepted_at: Option<ActivityTimestamp>,
    pub terminal_at: Option<ActivityTimestamp>,
}

/// Worker-owned claimed intent. The delivery ID is correlation-only, never a
/// provider exactly-once key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedCourseInvitationDelivery {
    pub delivery: CourseInvitationDelivery,
    pub lease: CourseInvitationDeliveryLeaseId,
}

/// Worker-only reissuance input. It omits the raw invitation secret and is
/// returned only by the broker after a matching lease revalidation.
#[derive(Clone, PartialEq, Eq)]
pub struct PreparedCourseInvitationDelivery {
    pub delivery: CourseInvitationDeliveryId,
    pub lease: CourseInvitationDeliveryLeaseId,
    pub delivery_email: String,
    pub expected_token_hash: crate::CourseInvitationSecretHash,
    pub reissuance: InvitationDeliveryReissuance,
}

impl std::fmt::Debug for PreparedCourseInvitationDelivery {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PreparedCourseInvitationDelivery([protected])")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum InvitationDeliveryReissuance {
    Single {
        course: CourseId,
        roster_id: CourseRosterId,
        idempotency_key: RosterIdempotencyKey,
    },
    Import {
        course: CourseId,
        import: crate::CourseRosterImportId,
        row_number: u16,
        commit_idempotency_key: RosterIdempotencyKey,
    },
}

impl std::fmt::Debug for InvitationDeliveryReissuance {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("InvitationDeliveryReissuance([protected])")
    }
}

/// Closed completion result supplied by a provider adapter after it returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompleteCourseInvitationDelivery {
    AcceptedByProvider,
    RetryableFailed { next_attempt_at: ActivityTimestamp },
    Ambiguous,
    PermanentFailed,
}

/// Authorized, coarse roster projection only.
#[async_trait]
pub trait CourseInvitationDeliveryStore: Send + Sync {
    async fn course_invitation_delivery_state(
        &self,
        context: ActorContext,
        session: SessionTokenHash,
        course: CourseId,
        invitation: CourseInvitationId,
    ) -> Result<Option<CourseInvitationDeliveryState>, StoreError>;
}

/// Dedicated worker capability. Implementations must atomically fence leases
/// and cancel jobs whose invitation has been claimed, revoked, or expired.
#[async_trait]
pub trait CourseInvitationDeliveryWorkerStore: Send + Sync {
    async fn prepare_course_invitation_delivery(
        &self,
        delivery: CourseInvitationDeliveryId,
        lease: CourseInvitationDeliveryLeaseId,
    ) -> Result<Option<PreparedCourseInvitationDelivery>, StoreError>;
    async fn claim_due_course_invitation_deliveries(
        &self,
        maximum: u16,
        lease_duration_seconds: u32,
    ) -> Result<Vec<ClaimedCourseInvitationDelivery>, StoreError>;

    async fn complete_course_invitation_delivery(
        &self,
        delivery: CourseInvitationDeliveryId,
        lease: CourseInvitationDeliveryLeaseId,
        completion: CompleteCourseInvitationDelivery,
    ) -> Result<bool, StoreError>;

    async fn revalidate_course_invitation_delivery_lease(
        &self,
        delivery: CourseInvitationDeliveryId,
        lease: CourseInvitationDeliveryLeaseId,
    ) -> Result<bool, StoreError>;
}
