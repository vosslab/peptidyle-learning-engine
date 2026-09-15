//! Typed notifier-only persistence boundary for Course-retention notices.
//!
//! C847 owns notice identity, recipient selection, leases, and terminal
//! provider acceptance.  This boundary exposes no Account, invitation,
//! Student Work, session, object, renderer, or raw receipt-table operation.

use async_trait::async_trait;
use question_model::Timestamp;
use uuid::Uuid;

use crate::StoreError;

/// A database-verified Instructor delivery destination.  Only C847's claim
/// procedure constructs this type; delivery code cannot substitute a raw
/// recipient string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedCourseRetentionNotificationDestination(String);

impl VerifiedCourseRetentionNotificationDestination {
    pub(crate) fn from_verified(value: String) -> Result<Self, StoreError> {
        if value != value.trim() || !(3..=320).contains(&value.chars().count()) {
            return Err(StoreError::InvalidRecord(
                "Course-retention notification destination is invalid".to_string(),
            ));
        }
        Ok(Self(value))
    }

    /// Returns the trusted provider destination selected by C847.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Constructs a verified destination solely for black-box adapter tests.
    ///
    /// Production code receives this value exclusively from C847's claim
    /// procedure, preserving the no-arbitrary-recipient boundary.
    #[cfg(feature = "test-support")]
    pub fn for_test_verified_destination(value: String) -> Result<Self, StoreError> {
        Self::from_verified(value)
    }
}

/// One database-selected recipient and receipt lease for a retention notice.
///
/// The destination is a verified Instructor delivery destination returned only
/// to the dedicated notifier capability; it is not an Account lookup result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedCourseRetentionNotification {
    pub notification_id: Uuid,
    pub verified_destination: VerifiedCourseRetentionNotificationDestination,
    pub provider_idempotency_key: Uuid,
    pub lease_token: Uuid,
}

/// A recorded pre-acceptance send failure.  `NotConfigured` is a real
/// non-send outcome, never a successful or terminal provider acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseRetentionNotificationFailure {
    NotConfigured,
    ProviderTransient,
    ProviderRejected,
}

/// The exact notifier capability: claim a stored notice, then record only a
/// provider acceptance or pre-acceptance failure.
///
/// Provider acceptance is terminal for sending.  A later claim therefore
/// never returns that receipt. A retry remains possible only before
/// acceptance.
#[async_trait]
pub trait CourseRetentionNotificationStore: Send + Sync {
    async fn claim_due_notification(
        &self,
        evaluated_at: Timestamp,
        lease_seconds: u16,
    ) -> Result<Option<ClaimedCourseRetentionNotification>, StoreError>;

    async fn record_provider_acceptance(
        &self,
        notification_id: Uuid,
        lease_token: Uuid,
        provider_idempotency_key: Uuid,
        accepted_at: Timestamp,
    ) -> Result<bool, StoreError>;

    async fn fail_before_acceptance(
        &self,
        notification_id: Uuid,
        lease_token: Uuid,
        failed_at: Timestamp,
        failure: CourseRetentionNotificationFailure,
    ) -> Result<bool, StoreError>;
}
