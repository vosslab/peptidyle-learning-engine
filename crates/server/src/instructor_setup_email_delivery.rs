//! SMTP delivery for the one fixed Instructor Account setup notice.
//!
//! The Account store chooses the validated destination. This module accepts no
//! Account ID, Profile data, role, or caller-controlled message content.

use async_trait::async_trait;
use learning_data_access::{AuthenticationEmail, EmailAuthenticationCode};
use lettre::message::{Mailbox, header::ContentType};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use url::Url;

pub const INSTRUCTOR_SETUP_EMAIL_SUBJECT: &str = "Your Peptidyle Learning Engine account";
pub const INSTRUCTOR_SETUP_EMAIL_BODY_PREFIX: &str =
    "Your Peptidyle Learning Engine Instructor Account is ready. Set up your sign-in method here: ";
pub const EMAIL_AUTHENTICATION_CODE_SUBJECT: &str = "Your Peptidyle Learning Engine sign-in code";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructorSetupEmailFailure {
    NotConfigured,
    ProviderRejected,
    ProviderTransient,
}

#[async_trait]
pub trait InstructorSetupEmailDelivery: Send + Sync {
    /// ASVS 2.2.2 and 14.2.6: only a trusted Store supplies the validated,
    /// private destination; delivery never receives browser-provided content.
    async fn send_setup_email(
        &self,
        destination: &AuthenticationEmail,
    ) -> Result<(), InstructorSetupEmailFailure>;

    /// Sends the short-lived, browser-bound code minted by the ordinary
    /// Authentication Ceremony. The code itself never enters logs or a URL.
    async fn send_email_authentication_code(
        &self,
        destination: &AuthenticationEmail,
        code: &EmailAuthenticationCode,
    ) -> Result<(), InstructorSetupEmailFailure>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NotConfiguredInstructorSetupEmailDelivery;

#[async_trait]
impl InstructorSetupEmailDelivery for NotConfiguredInstructorSetupEmailDelivery {
    async fn send_setup_email(
        &self,
        _: &AuthenticationEmail,
    ) -> Result<(), InstructorSetupEmailFailure> {
        Err(InstructorSetupEmailFailure::NotConfigured)
    }

    async fn send_email_authentication_code(
        &self,
        _: &AuthenticationEmail,
        _: &EmailAuthenticationCode,
    ) -> Result<(), InstructorSetupEmailFailure> {
        Err(InstructorSetupEmailFailure::NotConfigured)
    }
}

#[derive(Clone)]
pub struct SmtpInstructorSetupEmailDelivery {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
    sign_in_url: Url,
}

impl SmtpInstructorSetupEmailDelivery {
    pub fn from_provider(
        connection_url: &str,
        from: &str,
        public_origin: &str,
    ) -> Result<Self, String> {
        let from = from
            .parse::<Mailbox>()
            .map_err(|_| "Instructor setup email from address is invalid".to_string())?;
        validate_production_smtp_url(connection_url)?;
        let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(connection_url)
            .map_err(|_| "Instructor setup email SMTP URL is invalid".to_string())?
            .build();
        let mut sign_in_url = Url::parse(public_origin)
            .map_err(|_| "Instructor setup public origin is invalid".to_string())?;
        if sign_in_url.scheme() != "https" || sign_in_url.host_str().is_none() {
            return Err("Instructor setup public origin must be an HTTPS origin".to_string());
        }
        sign_in_url.set_path("/sign-in");
        sign_in_url.set_query(None);
        sign_in_url.set_fragment(None);
        Ok(Self {
            transport,
            from,
            sign_in_url,
        })
    }
}

fn validate_production_smtp_url(connection_url: &str) -> Result<(), String> {
    let parsed = Url::parse(connection_url)
        .map_err(|_| "Instructor setup email SMTP URL is invalid".to_string())?;
    let encrypted = parsed.scheme() == "smtps"
        || (parsed.scheme() == "smtp"
            && parsed
                .query_pairs()
                .any(|(name, value)| name == "tls" && value.eq_ignore_ascii_case("required")));
    if !encrypted {
        return Err(
            "Instructor setup email SMTP URL must use smtps or smtp with tls=required".to_string(),
        );
    }
    Ok(())
}

#[async_trait]
impl InstructorSetupEmailDelivery for SmtpInstructorSetupEmailDelivery {
    async fn send_setup_email(
        &self,
        destination: &AuthenticationEmail,
    ) -> Result<(), InstructorSetupEmailFailure> {
        let recipient = destination
            .delivery()
            .parse::<Mailbox>()
            .map_err(|_| InstructorSetupEmailFailure::ProviderRejected)?;
        let message = Message::builder()
            .from(self.from.clone())
            .to(recipient)
            .subject(INSTRUCTOR_SETUP_EMAIL_SUBJECT)
            .header(ContentType::TEXT_PLAIN)
            .body(format!(
                "{INSTRUCTOR_SETUP_EMAIL_BODY_PREFIX}{}",
                self.sign_in_url
            ))
            .map_err(|_| InstructorSetupEmailFailure::ProviderRejected)?;
        self.transport.send(message).await.map_err(|error| {
            if error.is_permanent() {
                InstructorSetupEmailFailure::ProviderRejected
            } else {
                InstructorSetupEmailFailure::ProviderTransient
            }
        })?;
        tracing::info!(event = "instructor_setup_email_submitted");
        Ok(())
    }

    async fn send_email_authentication_code(
        &self,
        destination: &AuthenticationEmail,
        code: &EmailAuthenticationCode,
    ) -> Result<(), InstructorSetupEmailFailure> {
        let recipient = destination
            .delivery()
            .parse::<Mailbox>()
            .map_err(|_| InstructorSetupEmailFailure::ProviderRejected)?;
        let message = Message::builder()
            .from(self.from.clone())
            .to(recipient)
            .subject(EMAIL_AUTHENTICATION_CODE_SUBJECT)
            .header(ContentType::TEXT_PLAIN)
            .body(format!("Your sign-in code is: {}", code.as_text()))
            .map_err(|_| InstructorSetupEmailFailure::ProviderRejected)?;
        self.transport.send(message).await.map_err(|error| {
            if error.is_permanent() {
                InstructorSetupEmailFailure::ProviderRejected
            } else {
                InstructorSetupEmailFailure::ProviderTransient
            }
        })?;
        tracing::info!(event = "email_authentication_code_submitted");
        Ok(())
    }
}

#[derive(Clone)]
pub enum InstructorSetupEmailDeliveryAdapter {
    NotConfigured(NotConfiguredInstructorSetupEmailDelivery),
    Smtp(SmtpInstructorSetupEmailDelivery),
}

impl InstructorSetupEmailDeliveryAdapter {
    pub fn not_configured() -> Self {
        Self::NotConfigured(NotConfiguredInstructorSetupEmailDelivery)
    }

    pub fn smtp(connection_url: &str, from: &str, public_origin: &str) -> Result<Self, String> {
        Ok(Self::Smtp(SmtpInstructorSetupEmailDelivery::from_provider(
            connection_url,
            from,
            public_origin,
        )?))
    }

    /// Email-code routes exist only when a real encrypted provider is configured.
    pub fn is_configured(&self) -> bool {
        matches!(self, Self::Smtp(_))
    }
}

#[async_trait]
impl InstructorSetupEmailDelivery for InstructorSetupEmailDeliveryAdapter {
    async fn send_setup_email(
        &self,
        destination: &AuthenticationEmail,
    ) -> Result<(), InstructorSetupEmailFailure> {
        match self {
            Self::NotConfigured(delivery) => delivery.send_setup_email(destination).await,
            Self::Smtp(delivery) => delivery.send_setup_email(destination).await,
        }
    }

    async fn send_email_authentication_code(
        &self,
        destination: &AuthenticationEmail,
        code: &EmailAuthenticationCode,
    ) -> Result<(), InstructorSetupEmailFailure> {
        match self {
            Self::NotConfigured(delivery) => {
                delivery
                    .send_email_authentication_code(destination, code)
                    .await
            }
            Self::Smtp(delivery) => {
                delivery
                    .send_email_authentication_code(destination, code)
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smtp_configuration_rejects_an_invalid_sender() {
        assert!(
            SmtpInstructorSetupEmailDelivery::from_provider(
                "smtp://127.0.0.1:2525",
                "not a mailbox",
                "https://ple.example",
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn smtp_configuration_requires_tls() {
        assert!(
            SmtpInstructorSetupEmailDelivery::from_provider(
                "smtp://127.0.0.1:2525",
                "ple@example.edu",
                "https://ple.example",
            )
            .is_err()
        );
        assert!(
            SmtpInstructorSetupEmailDelivery::from_provider(
                "smtp://smtp.example:587?tls=required",
                "ple@example.edu",
                "https://ple.example",
            )
            .is_ok()
        );
    }

    #[tokio::test]
    async fn absent_provider_fails_closed_for_setup_notice() {
        let destination =
            AuthenticationEmail::parse("instructor@example.edu").expect("valid local destination");
        let result = NotConfiguredInstructorSetupEmailDelivery
            .send_setup_email(&destination)
            .await;
        assert_eq!(result, Err(InstructorSetupEmailFailure::NotConfigured));
    }
}
