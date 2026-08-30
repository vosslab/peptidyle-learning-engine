//! TLS-protected SMTP delivery for course invitation links.

use async_trait::async_trait;
use learning_data_access::AuthenticationEmail;
use lettre::message::{Mailbox, Message, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::response::{Category, Code, Detail, Severity};
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use url::Url;

use super::invitation_capability::{
    CourseInvitationDelivery, CourseInvitationDeliveryAttempt, CourseInvitationDeliveryError,
    CourseInvitationSecret,
};

/// Encrypted SMTP submission mode selected by the external provider.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmtpTlsMode {
    /// Upgrade port 587-style submission with mandatory STARTTLS.
    StartTls,
    /// Establish TLS before SMTP, typically on submission port 465.
    ImplicitTls,
}

/// Validated SMTP and public-link settings. Credential text is never retained
/// in this value after the transport is constructed.
pub struct SmtpCourseInvitationDeliveryConfig {
    pub relay: String,
    pub port: u16,
    pub tls_mode: SmtpTlsMode,
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
            .field("tls_mode", &self.tls_mode)
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
}

/// The only SMTP failure classes permitted in operator telemetry.  The
/// transport keeps the provider response private; this classification uses
/// only Lettre's public error kind and numeric status code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SmtpDeliveryFailureCategory {
    DnsOrConnectivity,
    TlsHandshake,
    Authentication,
    ProviderRejection,
}

impl SmtpDeliveryFailureCategory {
    fn as_str(self) -> &'static str {
        match self {
            Self::DnsOrConnectivity => "dns_or_connectivity",
            Self::TlsHandshake => "tls_handshake",
            Self::Authentication => "authentication",
            Self::ProviderRejection => "provider_rejection",
        }
    }
}

fn smtp_delivery_failure_category(
    error: &lettre::transport::smtp::Error,
) -> SmtpDeliveryFailureCategory {
    if error.is_tls() {
        return SmtpDeliveryFailureCategory::TlsHandshake;
    }
    if error
        .status()
        .is_some_and(smtp_status_is_authentication_failure)
    {
        return SmtpDeliveryFailureCategory::Authentication;
    }
    if error.is_response() {
        return SmtpDeliveryFailureCategory::ProviderRejection;
    }
    SmtpDeliveryFailureCategory::DnsOrConnectivity
}

fn smtp_status_is_authentication_failure(status: Code) -> bool {
    matches!(
        (status.severity, status.category, status.detail),
        (
            Severity::PermanentNegativeCompletion,
            Category::Unspecified3,
            Detail::Zero | Detail::Four | Detail::Five,
        )
    )
}

fn record_smtp_delivery_failure(error: &lettre::transport::smtp::Error) {
    let category = smtp_delivery_failure_category(error);
    let delivery_outcome = smtp_delivery_outcome(error);
    // Invitation delivery inherits the worker's opaque durable delivery ID;
    // synchronous passwordless mail may instead inherit its request span. Do
    // not record the SMTP error itself: it can include an upstream response.
    tracing::warn!(
        event = "smtp_delivery_failed",
        delivery_outcome,
        category = category.as_str(),
    );
}

/// SMTP 4xx/5xx rejections are explicit negative replies. All other Lettre
/// failures can occur after `DATA` without a returned reply, so telemetry
/// must not assert that no submission happened.
fn smtp_delivery_outcome(error: &lettre::transport::smtp::Error) -> &'static str {
    if error.is_transient() || error.is_permanent() {
        "known_rejected"
    } else {
        "ambiguous"
    }
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
        redeem_url.set_path("/course-invitations/redeem");
        let transport = match config.tls_mode {
            SmtpTlsMode::StartTls => {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.relay)
            }
            SmtpTlsMode::ImplicitTls => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.relay),
        }
        .map_err(|_| CourseInvitationDeliveryError::Unavailable)?
        .port(config.port)
        .credentials(Credentials::new(config.username, config.password))
        .build();
        Ok(Self {
            transport,
            from,
            redeem_url,
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

    async fn attempt_invitation_message(
        &self,
        message: Message,
    ) -> CourseInvitationDeliveryAttempt {
        match self.transport.send(message).await {
            Ok(_) => CourseInvitationDeliveryAttempt::AcceptedByProvider,
            Err(error) => {
                record_smtp_delivery_failure(&error);
                if error.is_transient() {
                    CourseInvitationDeliveryAttempt::RetryableFailure
                } else if error.is_permanent() {
                    CourseInvitationDeliveryAttempt::PermanentFailure
                } else {
                    // Lettre does not expose the SMTP phase for connection,
                    // TLS, parser, timeout, or response-loss errors. Treating
                    // any of them as retryable could duplicate a DATA submit.
                    CourseInvitationDeliveryAttempt::Ambiguous
                }
            }
        }
    }
}

#[async_trait]
impl CourseInvitationDelivery for SmtpCourseInvitationDelivery {
    fn is_configured(&self) -> bool {
        true
    }

    async fn attempt_course_invitation(
        &self,
        email: &AuthenticationEmail,
        invitation_secret: &CourseInvitationSecret,
    ) -> CourseInvitationDeliveryAttempt {
        match self.message(email, invitation_secret) {
            Ok(message) => self.attempt_invitation_message(message).await,
            // This is a local construction failure before SMTP is attempted.
            Err(CourseInvitationDeliveryError::Unavailable) => {
                CourseInvitationDeliveryAttempt::RetryableFailure
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    use super::*;

    fn config(base: &str) -> SmtpCourseInvitationDeliveryConfig {
        SmtpCourseInvitationDeliveryConfig {
            relay: "smtp.example.edu".to_string(),
            port: 587,
            tls_mode: SmtpTlsMode::StartTls,
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

    #[tokio::test]
    async fn supports_both_encrypted_provider_submission_modes() {
        let mut value = config("https://ple.example.edu");
        assert!(SmtpCourseInvitationDelivery::new(value).is_ok());

        value = config("https://ple.example.edu");
        value.port = 465;
        value.tls_mode = SmtpTlsMode::ImplicitTls;
        assert!(SmtpCourseInvitationDelivery::new(value).is_ok());
    }

    #[test]
    fn debug_output_redacts_smtp_credentials() {
        let value = config("https://ple.example.edu");
        let debug = format!("{value:?}");
        assert!(!debug.contains("fixture-only-secret"));
        assert!(!debug.contains("ple@example.edu"));
    }

    #[test]
    fn authentication_statuses_have_a_redacted_operator_category() {
        let authentication = Code::new(
            Severity::PermanentNegativeCompletion,
            Category::Unspecified3,
            Detail::Five,
        );
        let rejection = Code::new(
            Severity::PermanentNegativeCompletion,
            Category::MailSystem,
            Detail::Zero,
        );

        assert!(smtp_status_is_authentication_failure(authentication));
        assert!(!smtp_status_is_authentication_failure(rejection));
    }

    #[tokio::test]
    async fn smtp_authentication_failure_stays_adapter_coarse() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener");
        let port = listener.local_addr().expect("listener address").port();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("test client");
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            let mut command = String::new();
            writer
                .write_all(b"220 test SMTP\r\n")
                .await
                .expect("greeting");
            reader.read_line(&mut command).await.expect("EHLO");
            writer
                .write_all(b"250-test SMTP\r\n250-AUTH PLAIN\r\n250 OK\r\n")
                .await
                .expect("EHLO reply");
            command.clear();
            reader.read_line(&mut command).await.expect("AUTH");
            writer
                .write_all(b"535 authentication rejected\r\n")
                .await
                .expect("AUTH reply");
        });
        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
            .port(port)
            .credentials(Credentials::new(
                "operator@example.edu".to_string(),
                "fixture-only-secret".to_string(),
            ))
            .build();
        let delivery = SmtpCourseInvitationDelivery {
            transport,
            from: "PLE <ple@example.edu>".parse().expect("sender mailbox"),
            redeem_url: Url::parse("https://ple.example.edu/course-invitations/redeem")
                .expect("redeem URL"),
            email_auth_url: Url::parse("https://ple.example.edu/auth/email/complete")
                .expect("email auth URL"),
            email_change_url: Url::parse("https://ple.example.edu/auth/account/email/complete")
                .expect("email change URL"),
        };
        let message = Message::builder()
            .from("PLE <ple@example.edu>".parse().expect("sender mailbox"))
            .to("learner@example.edu".parse().expect("recipient mailbox"))
            .subject("fixture")
            .body("fixture body".to_string())
            .expect("SMTP message");

        let result = delivery.attempt_invitation_message(message).await;

        assert_eq!(result, CourseInvitationDeliveryAttempt::PermanentFailure);
        server.await.expect("SMTP server task");
    }
}
