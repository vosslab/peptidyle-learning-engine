//! TLS-protected SMTP delivery for course invitation links.

use async_trait::async_trait;
use learning_data_access::AuthenticationEmail;
use lettre::message::{Mailbox, Message, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use url::Url;

use crate::auth::{
    PasswordlessEmailAction, PasswordlessEmailDelivery, PasswordlessEmailDeliveryError,
    PasswordlessEmailSecret,
};

use super::roster::{
    CourseInvitationDelivery, CourseInvitationDeliveryError, CourseInvitationSecret,
};

/// Validated SMTP and public-link settings. Credential text is never retained
/// in this value after the transport is constructed.
pub struct SmtpCourseInvitationDeliveryConfig {
    pub relay: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
    pub public_app_base_url: String,
}

impl std::fmt::Debug for SmtpCourseInvitationDeliveryConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SmtpCourseInvitationDeliveryConfig")
            .field("relay", &self.relay)
            .field("port", &self.port)
            .field("username", &"[redacted]")
            .field("password", &"[redacted]")
            .field("from", &"[redacted]")
            .field("public_app_base_url", &self.public_app_base_url)
            .finish()
    }
}

/// Reusable async SMTP transport. All public errors are deliberately coarse.
pub struct SmtpCourseInvitationDelivery {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
    redeem_url: Url,
    email_auth_url: Url,
    email_change_url: Url,
}

impl std::fmt::Debug for SmtpCourseInvitationDelivery {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SmtpCourseInvitationDelivery")
            .field("configured", &true)
            .finish()
    }
}

impl SmtpCourseInvitationDelivery {
    /// Builds a STARTTLS SMTP sender and an HTTPS invitation landing URL.
    pub fn new(
        config: SmtpCourseInvitationDeliveryConfig,
    ) -> Result<Self, CourseInvitationDeliveryError> {
        if config.relay.trim().is_empty()
            || config.username.trim().is_empty()
            || config.password.is_empty()
            || config.port == 0
        {
            return Err(CourseInvitationDeliveryError::Unavailable);
        }
        let from = config
            .from
            .parse::<Mailbox>()
            .map_err(|_| CourseInvitationDeliveryError::Unavailable)?;
        let mut redeem_url = Url::parse(&config.public_app_base_url)
            .map_err(|_| CourseInvitationDeliveryError::Unavailable)?;
        if redeem_url.scheme() != "https"
            || redeem_url.host_str().is_none()
            || redeem_url.cannot_be_a_base()
            || redeem_url.query().is_some()
            || redeem_url.fragment().is_some()
        {
            return Err(CourseInvitationDeliveryError::Unavailable);
        }
        let mut email_auth_url = redeem_url.clone();
        let mut email_change_url = redeem_url.clone();
        redeem_url.set_path("/course-invitations/redeem");
        email_auth_url.set_path("/auth/email/complete");
        email_change_url.set_path("/auth/account/email/complete");
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.relay)
            .map_err(|_| CourseInvitationDeliveryError::Unavailable)?
            .port(config.port)
            .credentials(Credentials::new(config.username, config.password))
            .build();
        Ok(Self {
            transport,
            from,
            redeem_url,
            email_auth_url,
            email_change_url,
        })
    }

    fn message(
        &self,
        email: &AuthenticationEmail,
        secret: &CourseInvitationSecret,
    ) -> Result<Message, CourseInvitationDeliveryError> {
        let to = email
            .delivery()
            .parse::<Mailbox>()
            .map_err(|_| CourseInvitationDeliveryError::Unavailable)?;
        let mut url = self.redeem_url.clone();
        url.set_fragment(Some(&format!("token={}", secret.encoded())));
        Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject("Your PLE course invitation")
            .header(ContentType::TEXT_PLAIN)
            .body(format!(
                "You were invited to a PLE course. Open this one-time link to sign in and claim the invitation:\n\n{url}\n\nIf you did not expect this invitation, you can ignore this message."
            ))
            .map_err(|_| CourseInvitationDeliveryError::Unavailable)
    }

    fn email_authentication_message(
        &self,
        email: &AuthenticationEmail,
        secret: &PasswordlessEmailSecret,
        action: PasswordlessEmailAction,
    ) -> Result<Message, PasswordlessEmailDeliveryError> {
        let to = email
            .delivery()
            .parse::<Mailbox>()
            .map_err(|_| PasswordlessEmailDeliveryError::Unavailable)?;
        let mut url = match action {
            PasswordlessEmailAction::SignIn => self.email_auth_url.clone(),
            PasswordlessEmailAction::ChangeEmail => self.email_change_url.clone(),
        };
        url.set_fragment(Some(&format!("token={}", secret.encoded())));
        Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(match action {
                PasswordlessEmailAction::SignIn => "Sign in to PLE",
                PasswordlessEmailAction::ChangeEmail => "Confirm your new PLE email",
            })
            .header(ContentType::TEXT_PLAIN)
            .body(format!(
                "Open this one-time link in the browser where you requested it:\n\n{url}\n\nThe link expires in ten minutes. If you did not request it, you can ignore this message."
            ))
            .map_err(|_| PasswordlessEmailDeliveryError::Unavailable)
    }
}

#[async_trait]
impl PasswordlessEmailDelivery for SmtpCourseInvitationDelivery {
    fn is_configured(&self) -> bool {
        true
    }

    async fn send_email_authentication(
        &self,
        email: &AuthenticationEmail,
        secret: &PasswordlessEmailSecret,
        action: PasswordlessEmailAction,
    ) -> Result<(), PasswordlessEmailDeliveryError> {
        let message = self.email_authentication_message(email, secret, action)?;
        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|_| PasswordlessEmailDeliveryError::Unavailable)
    }
}

#[async_trait]
impl CourseInvitationDelivery for SmtpCourseInvitationDelivery {
    fn is_configured(&self) -> bool {
        true
    }

    async fn send_course_invitation(
        &self,
        email: &AuthenticationEmail,
        invitation_secret: &CourseInvitationSecret,
    ) -> Result<(), CourseInvitationDeliveryError> {
        let message = self.message(email, invitation_secret)?;
        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|_| CourseInvitationDeliveryError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(base: &str) -> SmtpCourseInvitationDeliveryConfig {
        SmtpCourseInvitationDeliveryConfig {
            relay: "smtp.example.edu".to_string(),
            port: 587,
            username: "ple@example.edu".to_string(),
            password: "fixture-only-secret".to_string(),
            from: "PLE <ple@example.edu>".to_string(),
            public_app_base_url: base.to_string(),
        }
    }

    #[tokio::test]
    async fn production_delivery_requires_clean_https_origin() {
        assert!(SmtpCourseInvitationDelivery::new(config("https://ple.example.edu")).is_ok());
        for base in [
            "http://ple.example.edu",
            "https://ple.example.edu/?secret=query",
            "https://ple.example.edu/#fragment",
            "not a URL",
        ] {
            assert!(SmtpCourseInvitationDelivery::new(config(base)).is_err());
        }
    }

    #[test]
    fn debug_output_redacts_smtp_credentials() {
        let value = config("https://ple.example.edu");
        let debug = format!("{value:?}");
        assert!(!debug.contains("fixture-only-secret"));
        assert!(!debug.contains("ple@example.edu"));
    }
}
