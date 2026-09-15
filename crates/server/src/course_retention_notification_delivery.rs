//! Provider-neutral, fail-closed delivery for claimed Course-retention notices.
//!
//! The notifier Store owns recipient selection, receipt identity, and lease
//! enforcement.  This module deliberately accepts only its already-verified
//! destination and a fixed, redacted sign-in notice.  It has no Account,
//! Course, invitation, session, object, renderer, or provider configuration
//! surface.

use async_trait::async_trait;
use learning_data_access::{
    ClaimedCourseRetentionNotification, CourseRetentionNotificationFailure,
    CourseRetentionNotificationStore, StoreError, VerifiedCourseRetentionNotificationDestination,
};
use question_model::Timestamp;
use uuid::Uuid;

/// Fixed notice text sent for every retention notification.
///
/// It intentionally has no Course title or identifier, raw identifier,
/// FERPA-bearing detail, capability, recovery link, or recipient-derived
/// content.
pub const RETENTION_NOTIFICATION_SIGN_IN_NOTICE: &str =
    "Sign in to Peptidyle Learning Engine to review an update.";

/// Opaque fixed notice.  Delivery implementations cannot substitute content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionNotificationSignInNotice(());

impl RetentionNotificationSignInNotice {
    fn fixed() -> Self {
        Self(())
    }

    /// Returns the one product-approved, non-sensitive notice body.
    pub fn as_str(self) -> &'static str {
        RETENTION_NOTIFICATION_SIGN_IN_NOTICE
    }
}

/// A provider-neutral submission boundary.
///
/// Implementations receive neither a Course nor an Account identity.  The
/// idempotency key belongs to C847's immutable receipt and must be forwarded
/// unchanged to a configured provider.
#[async_trait]
pub trait CourseRetentionNotificationDelivery: Send + Sync {
    /// Requests a provider to accept one already-claimed redacted notice.
    // ASVS 2.3.1: only the Store-issued destination and receipt key can enter
    // a delivery attempt; providers cannot choose recipients or content.
    async fn submit(
        &self,
        destination: &VerifiedCourseRetentionNotificationDestination,
        provider_idempotency_key: Uuid,
        notice: RetentionNotificationSignInNotice,
    ) -> Result<(), CourseRetentionNotificationFailure>;
}

/// Disabled operational adapter.  It sends nothing and makes the real
/// `NotConfigured` outcome available for recording against the claimed lease.
#[derive(Debug, Default, Clone, Copy)]
pub struct NotConfiguredCourseRetentionNotificationDelivery;

#[async_trait]
impl CourseRetentionNotificationDelivery for NotConfiguredCourseRetentionNotificationDelivery {
    async fn submit(
        &self,
        _destination: &VerifiedCourseRetentionNotificationDestination,
        _provider_idempotency_key: Uuid,
        _notice: RetentionNotificationSignInNotice,
    ) -> Result<(), CourseRetentionNotificationFailure> {
        Err(CourseRetentionNotificationFailure::NotConfigured)
    }
}

/// Sends one Store-claimed notice and records its durable attempt outcome.
///
/// Provider acceptance is terminal for sending.  A receipt write that loses
/// its lease is surfaced as an error, never represented as fake success.
// ASVS 2.3.1 and 2.3.3: the claim's exact lease token is used for the only
// pre-acceptance state transition, and a successful provider submission is
// recorded before the caller may treat the attempt as complete.
pub async fn deliver_claimed_course_retention_notification<S, D>(
    store: &S,
    delivery: &D,
    claim: ClaimedCourseRetentionNotification,
    occurred_at: Timestamp,
) -> Result<(), StoreError>
where
    S: CourseRetentionNotificationStore,
    D: CourseRetentionNotificationDelivery,
{
    let recorded = match delivery
        .submit(
            &claim.verified_destination,
            claim.provider_idempotency_key,
            RetentionNotificationSignInNotice::fixed(),
        )
        .await
    {
        Ok(()) => {
            store
                .record_provider_acceptance(
                    claim.notification_id,
                    claim.lease_token,
                    claim.provider_idempotency_key,
                    occurred_at,
                )
                .await?
        }
        Err(failure) => {
            store
                .fail_before_acceptance(
                    claim.notification_id,
                    claim.lease_token,
                    occurred_at,
                    failure,
                )
                .await?
        }
    };
    if recorded {
        Ok(())
    } else {
        Err(StoreError::LeaseLost)
    }
}

/// Claims and submits at most one receipt through the exact notifier Store.
///
/// A Store that has recorded provider acceptance will never return that
/// receipt again; this helper consequently has no second-send path. `true`
/// means a claimed attempt recorded either acceptance or failure; `false`
/// means no eligible claim. Neither value represents inbox delivery.
pub async fn claim_and_deliver_one_course_retention_notification<S, D>(
    store: &S,
    delivery: &D,
    evaluated_at: Timestamp,
    lease_seconds: u16,
) -> Result<bool, StoreError>
where
    S: CourseRetentionNotificationStore,
    D: CourseRetentionNotificationDelivery,
{
    let Some(claim) = store
        .claim_due_notification(evaluated_at, lease_seconds)
        .await?
    else {
        return Ok(false);
    };
    deliver_claimed_course_retention_notification(store, delivery, claim, evaluated_at)
        .await
        .map(|()| true)
}
