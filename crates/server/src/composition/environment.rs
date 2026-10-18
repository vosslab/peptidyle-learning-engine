//! Deployment-owned renderer, delivery, browser, and session configuration.

use std::{net::SocketAddr, sync::Arc};

use adapter_webwork::{HttpWebworkRenderer, HttpWebworkRendererConfig, WebworkAdapter};
use anyhow::{Context, Result, bail};
use learning_data_access::SessionLifetime;
use objects::s3::S3ObjectStore;
use question_model::QuestionRendererVersion;

use crate::auth::{CookieTransport, ProductionBrowserBoundary, SessionConfig};

use super::object_storage;

pub(super) fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} must be set"))?;
    if value.is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value)
}

/// Constructs the one fixed private capture destination for publication.
pub(super) fn question_thumbnail_capture_from_env()
-> Result<crate::question_publication::QuestionThumbnailCaptureClient> {
    crate::question_publication::QuestionThumbnailCaptureClient::new(&required_env(
        "PLE_QUESTION_THUMBNAIL_RENDERER_BASE_URL",
    )?)
    .map_err(anyhow::Error::msg)
}

/// Constructs the private WeBWorK boundary from deployment-owned settings.
/// The renderer version is an attested private file, never a browser input.
pub(super) fn webwork_adapter_from_env() -> Result<Arc<WebworkAdapter<HttpWebworkRenderer>>> {
    let version_file = required_env("PLE_WEBWORK_RENDERER_VERSION_FILE")?;
    let attestation = std::fs::read_to_string(version_file)
        .context("could not read the Question Renderer Version attestation")?;
    let version = attestation
        .lines()
        .find_map(|line| line.strip_prefix("oci_id="))
        .filter(|value| value.starts_with("sha256:") && value.len() == 71)
        .ok_or_else(|| anyhow::anyhow!("Question Renderer Version attestation is invalid"))?;
    let timeout = required_env("PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS")?
        .parse::<u64>()
        .context("PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS must be an integer")?;
    let limit = required_env("PLE_WEBWORK_MAX_RESPONSE_BYTES")?
        .parse::<usize>()
        .context("PLE_WEBWORK_MAX_RESPONSE_BYTES must be an integer")?;
    let config = HttpWebworkRendererConfig::new(
        &required_env("PLE_WEBWORK_RENDERER_BASE_URL")?,
        std::time::Duration::from_secs(timeout),
        limit,
        QuestionRendererVersion {
            name: required_env("PLE_WEBWORK_RENDERER_ID")?,
            version: version.to_string(),
        },
        &required_env("PLE_BROWSER_ORIGIN")?,
    )
    .map_err(anyhow::Error::msg)?;
    let renderer = HttpWebworkRenderer::new(config).map_err(anyhow::Error::msg)?;
    Ok(Arc::new(WebworkAdapter::new(renderer)))
}

/// Builds the public, read-only renderer asset route from the same private
/// renderer address and deadline that issuance and grading use.
pub(super) fn webwork_asset_proxy_from_env()
-> Result<crate::webwork_asset_proxy::WebworkAssetProxyConfig> {
    let timeout = required_env("PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS")?
        .parse::<u64>()
        .context("PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS must be an integer")?;
    crate::webwork_asset_proxy::WebworkAssetProxyConfig::new(
        &required_env("PLE_WEBWORK_RENDERER_BASE_URL")?,
        std::time::Duration::from_secs(timeout),
    )
    .map_err(anyhow::Error::msg)
}

/// Constructs the stateless OS-random issuer used by service and trusted
/// installation publication paths. The public checksum belongs to PublishedQuestionId,
/// so this path needs neither an environment secret nor deployment state.
pub const fn question_id_issuer() -> crate::question_publication::RandomQuestionIdIssuer {
    crate::question_publication::RandomQuestionIdIssuer
}

/// Constructs the API-owned object reader used to compile answer-free Question
/// Library views. The supported topology requires its explicitly configured
/// authenticated disposable MinIO endpoint.
/// Builds the same policy-separated Question-source object store used by the
/// application. Trusted installation publication reuses this boundary rather
/// than accepting object-store paths or credentials as CLI arguments.
pub async fn question_library_object_store_from_env() -> Result<S3ObjectStore> {
    object_storage::object_store_from_env(object_storage::ObjectStoragePrincipal::ApiAndWorker)
}

pub(super) fn course_retention_notification_delivery_from_env()
-> Result<crate::course_retention_notification_delivery::CourseRetentionNotificationDeliveryAdapter>
{
    let url = optional_env("PLE_COURSE_RETENTION_NOTIFICATION_SMTP_URL");
    let from = optional_env("PLE_COURSE_RETENTION_NOTIFICATION_FROM");
    match (url, from) {
        (None, None) => {
            tracing::info!(
                event = "course_retention_notification_delivery",
                adapter = "not_configured",
            );
            Ok(crate::course_retention_notification_delivery::CourseRetentionNotificationDeliveryAdapter::not_configured())
        }
        (Some(url), Some(from)) => {
            tracing::info!(
                event = "course_retention_notification_delivery",
                adapter = "smtp",
            );
            crate::course_retention_notification_delivery::CourseRetentionNotificationDeliveryAdapter::smtp(
                &url,
                &from,
            )
            .map_err(anyhow::Error::msg)
            .context("could not configure Course-retention notification delivery")
        }
        _ => bail!(
            "Course-retention notification delivery requires both PLE_COURSE_RETENTION_NOTIFICATION_SMTP_URL and PLE_COURSE_RETENTION_NOTIFICATION_FROM"
        ),
    }
}

pub(super) fn instructor_setup_email_delivery_from_env()
-> Result<crate::instructor_setup_email_delivery::InstructorSetupEmailDeliveryAdapter> {
    let url = optional_env("PLE_INSTRUCTOR_SETUP_SMTP_URL");
    let from = optional_env("PLE_INSTRUCTOR_SETUP_FROM");
    let origin = optional_env("PLE_BROWSER_ORIGIN");
    match (url, from, origin) {
        (None, None, _) => Ok(crate::instructor_setup_email_delivery::InstructorSetupEmailDeliveryAdapter::not_configured()),
        (Some(url), Some(from), Some(origin)) => crate::instructor_setup_email_delivery::InstructorSetupEmailDeliveryAdapter::smtp(&url, &from, &origin)
            .map_err(anyhow::Error::msg)
            .context("could not configure Instructor setup email delivery"),
        _ => bail!(
            "Instructor setup email delivery requires PLE_INSTRUCTOR_SETUP_SMTP_URL, PLE_INSTRUCTOR_SETUP_FROM, and PLE_BROWSER_ORIGIN"
        ),
    }
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// The address the binary binds, parsed once at startup.
pub fn bind_address_from_env() -> Result<SocketAddr> {
    std::env::var("PLE_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()
        .context("PLE_BIND_ADDR must be a socket address")
}

/// Returns the canonical browser `Host` authority after validating the configured origin.
///
/// Container health checks reach the API over loopback but must still satisfy
/// the same public-host boundary as browser traffic.
pub fn browser_authority_from_env() -> Result<String> {
    Ok(production_browser_boundary_from_env()?
        .authority
        .to_string())
}

pub(super) fn production_browser_boundary_from_env() -> Result<ProductionBrowserBoundary> {
    let boundary = ProductionBrowserBoundary::new(Arc::from(required_env("PLE_BROWSER_ORIGIN")?))
        .map_err(anyhow::Error::msg)?;
    // Absent or empty keeps the exact production origin. Compose interpolation
    // supplies an empty value when the Live Demo writer does not set the flag.
    // ASVS 13.1.1: only the explicit local-demo value 1 changes the boundary.
    match std::env::var("PLE_ACCEPT_REQUEST_BROWSER_HOST") {
        Err(std::env::VarError::NotPresent) => Ok(boundary),
        Ok(value) if value.is_empty() => Ok(boundary),
        Ok(value) if value == "1" => Ok(boundary.accepting_request_host()),
        Ok(_) | Err(_) => {
            bail!("PLE_ACCEPT_REQUEST_BROWSER_HOST must be 1 when set")
        }
    }
}

/// Returns the one deployment-owned, recipient-independent HTTPS signup URL.
pub(super) fn invitation_signup_url_from_env() -> Result<Arc<str>> {
    let origin = required_env("PLE_BROWSER_ORIGIN")?;
    let parsed = url::Url::parse(&origin).context("PLE_BROWSER_ORIGIN must be an absolute URL")?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || parsed.username() != ""
        || parsed.password().is_some()
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        bail!("PLE_BROWSER_ORIGIN must be a canonical credential-free HTTPS origin");
    }
    Ok(Arc::from(parsed.origin().ascii_serialization()))
}

pub(super) fn production_session_config() -> SessionConfig {
    SessionConfig::new(
        SessionLifetime::from_seconds(8 * 60 * 60).expect("positive session lifetime"),
        CookieTransport::FirstPartyHttps,
    )
}
