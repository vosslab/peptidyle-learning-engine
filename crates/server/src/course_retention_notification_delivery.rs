//! Provider-neutral, fail-closed delivery for claimed Course-retention notices.
//!
//! The notifier Store owns recipient selection, receipt identity, and lease
//! enforcement.  This module deliberately accepts only its already-verified
//! destination and a fixed, redacted sign-in notice.  It has no Account,
//! Course, invitation, session, object, or renderer surface.  The caller
//! supplies a provider connection; this module does not read the environment.

use async_trait::async_trait;
use learning_data_access::{
    ClaimedCourseRetentionNotification, CourseRetentionNotificationFailure,
    CourseRetentionNotificationStore, StoreError, VerifiedCourseRetentionNotificationDestination,
};
use lettre::message::Mailbox;
use lettre::message::header::{ContentType, HeaderName, HeaderValue};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
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

/// SMTP submission for one already-claimed redacted notice.
///
/// The message body is the fixed sign-in sentence.  The receipt's idempotency
/// key is forwarded unchanged and no Course or Account identity is added.
pub struct SmtpCourseRetentionNotificationDelivery {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpCourseRetentionNotificationDelivery {
    /// Builds a provider from a caller-supplied connection URL and from address.
    pub fn from_provider(connection_url: &str, from: &str) -> Result<Self, String> {
        let from = from
            .parse::<Mailbox>()
            .map_err(|_| "Course-retention notification from address is invalid".to_string())?;
        let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(connection_url)
            .map_err(|_| "Course-retention notification SMTP URL is invalid".to_string())?
            .build();
        Ok(Self { transport, from })
    }
}

#[async_trait]
impl CourseRetentionNotificationDelivery for SmtpCourseRetentionNotificationDelivery {
    async fn submit(
        &self,
        destination: &VerifiedCourseRetentionNotificationDestination,
        provider_idempotency_key: Uuid,
        notice: RetentionNotificationSignInNotice,
    ) -> Result<(), CourseRetentionNotificationFailure> {
        let recipient = destination
            .as_str()
            .parse::<Mailbox>()
            .map_err(|_| CourseRetentionNotificationFailure::ProviderRejected)?;
        let message = Message::builder()
            .from(self.from.clone())
            .to(recipient)
            .subject(RETENTION_NOTIFICATION_SIGN_IN_NOTICE)
            .header(ContentType::TEXT_PLAIN)
            .raw_header(HeaderValue::new(
                HeaderName::new_from_ascii_str("X-Ple-Retention-Idempotency-Key"),
                provider_idempotency_key.to_string(),
            ))
            .body(notice.as_str().to_owned())
            .map_err(|_| CourseRetentionNotificationFailure::ProviderRejected)?;
        self.transport.send(message).await.map_err(smtp_failure)?;
        tracing::info!(
            event = "course_retention_notification_submitted",
            idempotency_key = %provider_idempotency_key,
        );
        Ok(())
    }
}

fn smtp_failure(error: lettre::transport::smtp::Error) -> CourseRetentionNotificationFailure {
    if error.is_permanent() {
        CourseRetentionNotificationFailure::ProviderRejected
    } else {
        CourseRetentionNotificationFailure::ProviderTransient
    }
}

/// The process-selected adapter.  Absent provider settings stay `NotConfigured`.
pub enum CourseRetentionNotificationDeliveryAdapter {
    /// No provider is configured.  Submission records a non-send.
    NotConfigured(NotConfiguredCourseRetentionNotificationDelivery),
    /// A caller-supplied SMTP provider.
    Smtp(SmtpCourseRetentionNotificationDelivery),
}

impl CourseRetentionNotificationDeliveryAdapter {
    /// Uses the disabled adapter.
    pub fn not_configured() -> Self {
        Self::NotConfigured(NotConfiguredCourseRetentionNotificationDelivery)
    }

    /// Uses SMTP for the supplied provider connection and from address.
    pub fn smtp(connection_url: &str, from: &str) -> Result<Self, String> {
        Ok(Self::Smtp(
            SmtpCourseRetentionNotificationDelivery::from_provider(connection_url, from)?,
        ))
    }
}

#[async_trait]
impl CourseRetentionNotificationDelivery for CourseRetentionNotificationDeliveryAdapter {
    async fn submit(
        &self,
        destination: &VerifiedCourseRetentionNotificationDestination,
        provider_idempotency_key: Uuid,
        notice: RetentionNotificationSignInNotice,
    ) -> Result<(), CourseRetentionNotificationFailure> {
        match self {
            Self::NotConfigured(delivery) => {
                delivery
                    .submit(destination, provider_idempotency_key, notice)
                    .await
            }
            Self::Smtp(delivery) => {
                delivery
                    .submit(destination, provider_idempotency_key, notice)
                    .await
            }
        }
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
