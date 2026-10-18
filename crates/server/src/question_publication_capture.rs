//! Private synchronous capture of the one validated seed-zero renderer document.

use std::time::Duration;

use objects::{PreparedQuestionThumbnail, Sha256Checksum};
use reqwest::{Client, Url, redirect::Policy};

/// A bounded capture failed or returned invalid output. Private content and
/// helper diagnostics stay outside this safe server-facing failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionThumbnailCaptureError;

impl std::fmt::Display for QuestionThumbnailCaptureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Question thumbnail capture is unavailable")
    }
}

impl std::error::Error for QuestionThumbnailCaptureError {}

/// Capture output bound to the exact source bytes used for its seed-zero render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPublicationThumbnail {
    pub(crate) source_checksum: Sha256Checksum,
    pub(crate) thumbnail: PreparedQuestionThumbnail,
}

impl PreparedPublicationThumbnail {
    /// Binds already validated encoded bytes to the source that was rendered.
    /// The publisher repeats this checksum comparison against its authorized read.
    pub const fn new(
        source_checksum: Sha256Checksum,
        thumbnail: PreparedQuestionThumbnail,
    ) -> Self {
        Self {
            source_checksum,
            thumbnail,
        }
    }
}

/// Deployment-configured client for the private, credential-free capture helper.
#[derive(Clone)]
pub struct QuestionThumbnailCaptureClient {
    client: Client,
    endpoint: Url,
}

impl QuestionThumbnailCaptureClient {
    /// Accepts a trusted service origin; browser input never selects this destination.
    pub fn new(base_url: &str) -> Result<Self, &'static str> {
        let base = Url::parse(base_url).map_err(|_| "invalid thumbnail renderer origin")?;
        if !matches!(base.scheme(), "http" | "https")
            || base.host_str().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.path() != "/"
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err("invalid thumbnail renderer origin");
        }
        // ASVS 13.2.4/13.2.6: one fixed destination, no proxy/redirect escape,
        // and a deadline beyond the helper's 20s capture plus 2s cleanup.
        let client = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(23))
            .build()
            .map_err(|_| "could not construct thumbnail renderer client")?;
        let endpoint = base
            .join("capture")
            .map_err(|_| "invalid capture endpoint")?;
        Ok(Self { client, endpoint })
    }

    /// Captures one retained renderer document using shared preview preparation.
    pub async fn capture(
        &self,
        document: String,
        source_checksum: Sha256Checksum,
    ) -> Result<PreparedPublicationThumbnail, QuestionThumbnailCaptureError> {
        let document = crate::webwork_preview_document::prepare_preview_document(document)
            .ok_or(QuestionThumbnailCaptureError)?;
        if document.html.len() > 1_048_576 || document.content_security_policy.len() > 8_192 {
            return Err(QuestionThumbnailCaptureError);
        }
        let mut response = self
            .client
            .post(self.endpoint.clone())
            .json(&serde_json::json!({
                "html": document.html,
                "contentSecurityPolicy": document.content_security_policy,
            }))
            .send()
            .await
            .map_err(|_| QuestionThumbnailCaptureError)?;
        const MAX_CAPTURE_BYTES: usize = 2_000_000;
        if !response.status().is_success()
            || response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(';').next())
                != Some("image/webp")
            || response
                .content_length()
                .is_some_and(|size| size > MAX_CAPTURE_BYTES as u64)
        {
            return Err(QuestionThumbnailCaptureError);
        }
        // ASVS 2.2.1: cap streaming bytes as well as the optional declared size.
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| QuestionThumbnailCaptureError)?
        {
            if chunk.len() > MAX_CAPTURE_BYTES.saturating_sub(bytes.len()) {
                return Err(QuestionThumbnailCaptureError);
            }
            bytes.extend_from_slice(&chunk);
        }
        let thumbnail =
            PreparedQuestionThumbnail::new(bytes).map_err(|_| QuestionThumbnailCaptureError)?;
        Ok(PreparedPublicationThumbnail::new(
            source_checksum,
            thumbnail,
        ))
    }
}
