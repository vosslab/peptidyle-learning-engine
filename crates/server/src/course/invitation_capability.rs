//! Server-only invitation secret issuance and mail-delivery capability.

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use learning_data_access::{
    AuthenticationEmail, CourseInvitationSecretHash, CourseRosterId, RosterIdempotencyKey,
};
use question_model::CourseId;

const INVITATION_TOKEN_BYTES: usize = 32;

/// Redacted raw invitation capability used only between issuer and mailer.
pub struct CourseInvitationSecret([u8; INVITATION_TOKEN_BYTES]);

impl CourseInvitationSecret {
    /// Canonical URL-safe value consumed by an invitation delivery or the
    /// instructor-only one-time response. It must never be logged or persisted.
    pub fn encoded(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }

    pub(crate) fn redemption_path(&self) -> String {
        format!("/course-invitations/redeem#token={}", self.encoded())
    }

    pub(crate) fn hash(&self) -> CourseInvitationSecretHash {
        CourseInvitationSecretHash::compute(&self.0)
    }
}

impl std::fmt::Debug for CourseInvitationSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CourseInvitationSecret([redacted])")
    }
}

/// Server-held keyed issuer. Replaying the same idempotent request reproduces
/// the same secret without storing it or returning it to the browser.
#[derive(Clone)]
pub struct CourseInvitationIssuer(Option<[u8; 32]>);

impl CourseInvitationIssuer {
    /// Creates a configured issuer from a dedicated 256-bit server secret.
    pub fn from_server_secret(secret: [u8; 32]) -> Self {
        Self(Some(secret))
    }

    /// Fail-closed issuer for deployments without invitation configuration.
    pub fn unavailable() -> Self {
        Self(None)
    }

    pub(crate) fn issue(
        &self,
        tenant: question_model::TenantId,
        course: CourseId,
        email: &AuthenticationEmail,
        roster_id: &CourseRosterId,
        idempotency_key: &RosterIdempotencyKey,
    ) -> Result<CourseInvitationSecret, CourseInvitationDeliveryError> {
        let secret = self.0.ok_or(CourseInvitationDeliveryError::Unavailable)?;
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(&secret)
            .map_err(|_| CourseInvitationDeliveryError::Unavailable)?;
        mac.update(b"ple-course-invitation-v1\0");
        update_mac_part(&mut mac, tenant.as_uuid().as_bytes());
        update_mac_part(&mut mac, course.as_uuid().as_bytes());
        update_mac_part(&mut mac, email.normalized().as_bytes());
        update_mac_part(&mut mac, roster_id.as_str().as_bytes());
        update_mac_part(&mut mac, idempotency_key.as_str().as_bytes());
        Ok(CourseInvitationSecret(mac.finalize().into_bytes().into()))
    }

    pub(crate) fn issue_import(
        &self,
        tenant: question_model::TenantId,
        course: CourseId,
        import: learning_data_access::CourseRosterImportId,
        row_number: u16,
        idempotency_key: &RosterIdempotencyKey,
    ) -> Result<(CourseInvitationSecret, RosterIdempotencyKey), CourseInvitationDeliveryError> {
        let secret = self.0.ok_or(CourseInvitationDeliveryError::Unavailable)?;
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(&secret)
            .map_err(|_| CourseInvitationDeliveryError::Unavailable)?;
        mac.update(b"ple-course-roster-import-v1\0");
        update_mac_part(&mut mac, tenant.as_uuid().as_bytes());
        update_mac_part(&mut mac, course.as_uuid().as_bytes());
        update_mac_part(&mut mac, import.as_uuid().as_bytes());
        update_mac_part(&mut mac, &row_number.to_be_bytes());
        update_mac_part(&mut mac, idempotency_key.as_str().as_bytes());
        let invitation_secret = CourseInvitationSecret(mac.finalize().into_bytes().into());
        let row_key = format!(
            "bulk-{}",
            URL_SAFE_NO_PAD.encode(invitation_secret.hash().as_bytes())
        );
        let row_key = RosterIdempotencyKey::parse(&row_key)
            .map_err(|_| CourseInvitationDeliveryError::Unavailable)?;
        Ok((invitation_secret, row_key))
    }
}

impl std::fmt::Debug for CourseInvitationIssuer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CourseInvitationIssuer")
            .field("configured", &self.0.is_some())
            .finish()
    }
}

fn update_mac_part(mac: &mut Hmac<sha2::Sha256>, value: &[u8]) {
    mac.update(&(value.len() as u64).to_be_bytes());
    mac.update(value);
}

/// Mail-delivery failure with no recipient, token, or provider diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseInvitationDeliveryError {
    /// No mail service is configured or the service cannot accept the message.
    Unavailable,
}

/// Closed worker result for one SMTP submission attempt. This distinguishes a
/// known SMTP refusal from transport uncertainty: ordinary SMTP cannot prove
/// whether a lost response after `DATA` was accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseInvitationDeliveryAttempt {
    AcceptedByProvider,
    RetryableFailure,
    PermanentFailure,
    Ambiguous,
}

/// Server-only invitation delivery. Implementations must never log the URL.
#[async_trait]
pub trait CourseInvitationDelivery: Send + Sync {
    /// Returns false so an unconfigured worker leaves durable intent pending.
    fn is_configured(&self) -> bool;

    /// Makes one worker-owned provider attempt. This trait has no general
    /// send operation, so HTTP route code cannot bypass the durable outbox.
    async fn attempt_course_invitation(
        &self,
        email: &AuthenticationEmail,
        invitation_secret: &CourseInvitationSecret,
    ) -> CourseInvitationDeliveryAttempt;
}

/// Fail-closed delivery used when production mail settings are absent.
#[derive(Debug, Clone, Copy)]
pub struct UnavailableCourseInvitationDelivery;

#[async_trait]
impl CourseInvitationDelivery for UnavailableCourseInvitationDelivery {
    fn is_configured(&self) -> bool {
        false
    }

    async fn attempt_course_invitation(
        &self,
        _email: &AuthenticationEmail,
        _invitation_secret: &CourseInvitationSecret,
    ) -> CourseInvitationDeliveryAttempt {
        CourseInvitationDeliveryAttempt::Ambiguous
    }
}
