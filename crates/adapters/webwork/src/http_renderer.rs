//! Private, bounded HTTP transport for the WeBWorK renderer (MOD-ADP-WW).
//!
//! ## Shipped service dialect assumption
//!
//! PLE does not currently ship a renderer container, so this module keeps the
//! narrow private protocol in one place.  A deployment must configure a
//! private `http` or `https` base URI and an expected renderer identity; the
//! client then calls `POST /v1/render` and `POST /v1/grade`. Requests use
//! camel-case JSON with `pgSourceBase64`, `pgPath`, `version`, and `seed`;
//! grade adds `response`. Render responses are `{ envelope, html, renderer }`
//! and grade responses are `{ outcome, renderer }`, where `outcome` is either
//! `{ kind: "graded", result }` or `{ kind: "ungraded" }`. A future renderer
//! integration changes this module, not the adapter or shared contracts.
//!
//! The URI and optional authentication header are deployment-owned. Nothing
//! supplied by an attempt, a PG file, or a browser selects a network target.

use std::time::Duration;

use async_trait::async_trait;
use base64::Engine as _;
use grading::GradeOutcome;
use question_model::{AttemptResult, QuestionEnvelope, StudentResponse};
use reqwest::header::{CONTENT_TYPE, HeaderName, HeaderValue};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};

use crate::pg_parser_stub::{
    GradeRequest, RenderRequest, RenderedWebworkQuestion, RendererFailure, RendererIdentity,
    WebworkRenderer,
};

const RENDER_PATH: &str = "v1/render";
const GRADE_PATH: &str = "v1/grade";
const JSON_MEDIA_TYPE: &str = "application/json";
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1_048_576;

/// Configuration rejected before any renderer request is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RendererConfigError {
    /// The configured service URI was not an absolute HTTP(S) URI.
    InvalidBaseUri,
    /// The configured request limits cannot safely bound an HTTP exchange.
    InvalidLimits,
    /// The configured renderer identity cannot prove the responder is expected.
    MissingRendererIdentity,
    /// The optional deployment-owned authentication header was not valid HTTP.
    InvalidAuthenticationHeader,
}

impl std::fmt::Display for RendererConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBaseUri => {
                formatter.write_str("renderer base URI must be absolute http or https")
            }
            Self::InvalidLimits => {
                formatter.write_str("renderer deadlines and response limit must be positive")
            }
            Self::MissingRendererIdentity => {
                formatter.write_str("renderer identity must be configured")
            }
            Self::InvalidAuthenticationHeader => {
                formatter.write_str("renderer authentication header is invalid")
            }
        }
    }
}

impl std::error::Error for RendererConfigError {}

/// Server-owned settings for one isolated renderer service.
///
/// The optional authentication header is intentionally accepted only while
/// constructing this server-side object. It has no getter and its value is not
/// included in [`Debug`] output.
#[derive(Clone)]
pub struct HttpWebworkRendererConfig {
    base_uri: Url,
    deadline: Duration,
    max_response_bytes: usize,
    expected_renderer: RendererIdentity,
    authentication: Option<(HeaderName, HeaderValue)>,
}

impl std::fmt::Debug for HttpWebworkRendererConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HttpWebworkRendererConfig")
            .field("base_uri", &self.base_uri)
            .field("deadline", &self.deadline)
            .field("max_response_bytes", &self.max_response_bytes)
            .field("expected_renderer", &self.expected_renderer)
            .field(
                "authentication",
                &self.authentication.as_ref().map(|(name, _)| name),
            )
            .finish()
    }
}

impl HttpWebworkRendererConfig {
    /// Creates bounded settings for a private renderer deployment.
    pub fn new(
        base_uri: &str,
        deadline: Duration,
        max_response_bytes: usize,
        expected_renderer: RendererIdentity,
    ) -> Result<Self, RendererConfigError> {
        let base_uri = Url::parse(base_uri).map_err(|_| RendererConfigError::InvalidBaseUri)?;
        if !matches!(base_uri.scheme(), "http" | "https")
            || base_uri.host_str().is_none()
            || !base_uri.username().is_empty()
            || base_uri.password().is_some()
        {
            return Err(RendererConfigError::InvalidBaseUri);
        }
        if deadline.is_zero() || max_response_bytes == 0 {
            return Err(RendererConfigError::InvalidLimits);
        }
        if expected_renderer.id.trim().is_empty() || expected_renderer.version.trim().is_empty() {
            return Err(RendererConfigError::MissingRendererIdentity);
        }
        Ok(Self {
            base_uri,
            deadline,
            max_response_bytes,
            expected_renderer,
            authentication: None,
        })
    }

    /// Uses a conservative response limit suitable for ordinary PG prompts.
    pub fn with_default_response_limit(
        base_uri: &str,
        deadline: Duration,
        expected_renderer: RendererIdentity,
    ) -> Result<Self, RendererConfigError> {
        Self::new(
            base_uri,
            deadline,
            DEFAULT_MAX_RESPONSE_BYTES,
            expected_renderer,
        )
    }

    /// Adds an optional server-only authentication header from deployment config.
    pub fn with_authentication_header(
        mut self,
        name: &str,
        value: &str,
    ) -> Result<Self, RendererConfigError> {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| RendererConfigError::InvalidAuthenticationHeader)?;
        let value = HeaderValue::from_str(value)
            .map_err(|_| RendererConfigError::InvalidAuthenticationHeader)?;
        self.authentication = Some((name, value));
        Ok(self)
    }
}

/// Private HTTP implementation of [`WebworkRenderer`].
#[derive(Clone)]
pub struct HttpWebworkRenderer {
    client: Client,
    settings: HttpWebworkRendererConfig,
}

impl HttpWebworkRenderer {
    /// Builds a redirect-free client with explicit connect and request deadlines.
    pub fn new(settings: HttpWebworkRendererConfig) -> Result<Self, RendererConfigError> {
        let client = Client::builder()
            .connect_timeout(settings.deadline)
            .timeout(settings.deadline)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| RendererConfigError::InvalidLimits)?;
        Ok(Self { client, settings })
    }

    async fn post_json<T: Serialize>(
        &self,
        path: &str,
        request: &T,
    ) -> Result<Vec<u8>, RendererFailure> {
        let target =
            self.settings.base_uri.join(path).map_err(|_| {
                RendererFailure::InvalidOutput("renderer URI is invalid".to_string())
            })?;
        let mut builder = self
            .client
            .post(target)
            .header(CONTENT_TYPE, JSON_MEDIA_TYPE)
            .json(request);
        if let Some((name, value)) = &self.settings.authentication {
            builder = builder.header(name, value);
        }
        let response = builder.send().await.map_err(map_request_error)?;
        map_status(response.status())?;
        validate_content_type(&response)?;
        read_bounded(response, self.settings.max_response_bytes).await
    }
}

#[async_trait]
impl WebworkRenderer for HttpWebworkRenderer {
    async fn render(
        &self,
        request: RenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        let bytes = self
            .post_json(RENDER_PATH, &RenderWireRequest::from(request))
            .await?;
        let response: RenderWireResponse = serde_json::from_slice(&bytes).map_err(|_| {
            RendererFailure::InvalidOutput("renderer returned malformed JSON".to_string())
        })?;
        self.verify_identity(&response.renderer)?;
        Ok(RenderedWebworkQuestion {
            envelope: response.envelope,
            html: response.html,
            renderer: response.renderer,
        })
    }

    async fn grade(&self, request: GradeRequest<'_>) -> Result<GradeOutcome, RendererFailure> {
        let bytes = self
            .post_json(GRADE_PATH, &GradeWireRequest::from(request))
            .await?;
        let response: GradeWireResponse = serde_json::from_slice(&bytes).map_err(|_| {
            RendererFailure::InvalidOutput("renderer returned malformed JSON".to_string())
        })?;
        self.verify_identity(&response.renderer)?;
        Ok(response.outcome.into())
    }
}

impl HttpWebworkRenderer {
    fn verify_identity(&self, actual: &RendererIdentity) -> Result<(), RendererFailure> {
        if actual == &self.settings.expected_renderer {
            Ok(())
        } else {
            Err(RendererFailure::InvalidOutput(
                "renderer identity did not match deployment configuration".to_string(),
            ))
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderWireRequest<'a> {
    pg_source_base64: String,
    pg_path: &'a str,
    version: &'a str,
    seed: u64,
}

impl<'a> From<RenderRequest<'a>> for RenderWireRequest<'a> {
    fn from(request: RenderRequest<'a>) -> Self {
        Self {
            pg_source_base64: base64::engine::general_purpose::STANDARD.encode(request.pg_source),
            pg_path: request.pg_path,
            version: request.version,
            seed: request.seed,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GradeWireRequest<'a> {
    #[serde(flatten)]
    render: RenderWireRequest<'a>,
    response: &'a StudentResponse,
}

impl<'a> From<GradeRequest<'a>> for GradeWireRequest<'a> {
    fn from(request: GradeRequest<'a>) -> Self {
        Self {
            render: RenderWireRequest {
                pg_source_base64: base64::engine::general_purpose::STANDARD
                    .encode(request.pg_source),
                pg_path: request.pg_path,
                version: request.version,
                seed: request.seed,
            },
            response: request.response,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RenderWireResponse {
    envelope: QuestionEnvelope,
    html: String,
    renderer: RendererIdentity,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GradeWireResponse {
    outcome: GradeWireOutcome,
    renderer: RendererIdentity,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum GradeWireOutcome {
    Graded { result: AttemptResult },
    Ungraded,
}

impl From<GradeWireOutcome> for GradeOutcome {
    fn from(value: GradeWireOutcome) -> Self {
        match value {
            GradeWireOutcome::Graded { result } => Self::Graded(result),
            GradeWireOutcome::Ungraded => Self::Ungraded,
        }
    }
}

fn map_request_error(error: reqwest::Error) -> RendererFailure {
    if error.is_timeout() {
        RendererFailure::TimedOut
    } else {
        RendererFailure::Unavailable
    }
}

fn map_status(status: StatusCode) -> Result<(), RendererFailure> {
    if status.is_success() {
        return Ok(());
    }
    if matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS | StatusCode::PAYLOAD_TOO_LARGE
    ) {
        return Err(RendererFailure::ResourceExhausted);
    }
    if matches!(
        status,
        StatusCode::REQUEST_TIMEOUT | StatusCode::GATEWAY_TIMEOUT
    ) {
        return Err(RendererFailure::TimedOut);
    }
    if status.is_server_error() {
        return Err(RendererFailure::Unavailable);
    }
    Err(RendererFailure::InvalidOutput(
        "renderer rejected a trusted server request".to_string(),
    ))
}

fn validate_content_type(response: &reqwest::Response) -> Result<(), RendererFailure> {
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if content_type
        .split(';')
        .next()
        .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case(JSON_MEDIA_TYPE))
    {
        Ok(())
    } else {
        Err(RendererFailure::InvalidOutput(
            "renderer response was not JSON".to_string(),
        ))
    }
}

async fn read_bounded(
    mut response: reqwest::Response,
    maximum: usize,
) -> Result<Vec<u8>, RendererFailure> {
    if response
        .content_length()
        .is_some_and(|length| length > maximum as u64)
    {
        return Err(RendererFailure::ResourceExhausted);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_request_error)? {
        if chunk.len() > maximum.saturating_sub(bytes.len()) {
            return Err(RendererFailure::ResourceExhausted);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use question_model::generation::Seed;
    use question_model::response::ResponseDefinition;
    use question_model::{QuestionEnvelope, VersionId};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    use super::*;

    fn identity() -> RendererIdentity {
        RendererIdentity {
            id: "private-webwork".to_string(),
            version: "1".to_string(),
        }
    }

    fn renderer(base_uri: &str) -> HttpWebworkRenderer {
        HttpWebworkRenderer::new(
            HttpWebworkRendererConfig::new(base_uri, Duration::from_millis(100), 1024, identity())
                .expect("test URI should be valid"),
        )
        .expect("test client should build")
    }

    fn envelope() -> QuestionEnvelope {
        QuestionEnvelope {
            version: VersionId::from_uuid(Uuid::nil()),
            seed: Seed::new(7),
            title: "HTTP renderer fixture".to_string(),
            prompt: Vec::new(),
            response: ResponseDefinition::ShortText {
                match_mode: question_model::answer::TextMatchMode::Exact,
                max_length: 16,
            },
        }
    }

    fn render_request<'a>() -> RenderRequest<'a> {
        RenderRequest {
            pg_source: b"DOCUMENT();",
            pg_path: "Library/Test.pg",
            version: "00000000-0000-0000-0000-000000000000",
            seed: 7,
        }
    }

    async fn mock_server(responses: Vec<String>) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock listener");
        let address = listener.local_addr().expect("mock address");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_task = Arc::clone(&seen);
        tokio::spawn(async move {
            for response in responses {
                let (mut stream, _) = listener.accept().await.expect("accept mock request");
                let request = read_request(&mut stream).await;
                seen_task.lock().await.push(request);
                stream
                    .write_all(response.as_bytes())
                    .await
                    .expect("write mock response");
            }
        });
        (format!("http://{address}/"), seen)
    }

    async fn read_request(stream: &mut tokio::net::TcpStream) -> String {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let count = stream.read(&mut buffer).await.expect("read mock request");
            assert_ne!(count, 0, "mock request ended before headers");
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..header_end + 4]);
                let length = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .or_else(|| {
                        headers
                            .lines()
                            .find_map(|line| line.strip_prefix("Content-Length: "))
                    })
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .expect("content length");
                if bytes.len() >= header_end + 4 + length {
                    return String::from_utf8(bytes).expect("request UTF-8");
                }
            }
        }
    }

    fn response(status: &str, content_type: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    #[tokio::test]
    async fn render_uses_private_fixed_endpoint_and_verified_identity() {
        let body = serde_json::json!({
            "envelope": envelope(), "html": "<p>Question</p>", "renderer": identity(),
        })
        .to_string();
        let (base_uri, seen) =
            mock_server(vec![response("200 OK", "application/json", &body)]).await;
        let issued = renderer(&base_uri)
            .render(render_request())
            .await
            .expect("render succeeds");
        assert_eq!(issued.envelope, envelope());
        let request = seen.lock().await.remove(0);
        assert!(request.starts_with("POST /v1/render HTTP/1.1"));
        assert!(request.contains("\"pgSourceBase64\":\"RE9DVU1FTlQoKTs=\""));
    }

    #[tokio::test]
    async fn renderer_refuses_html_malformed_status_and_oversize_responses() {
        let cases = [
            response("200 OK", "text/html", "<html>no</html>"),
            response("200 OK", "application/json", "{"),
            response("503 Service Unavailable", "application/json", "{}"),
            response("200 OK", "application/json", &"x".repeat(2048)),
        ];
        for response_text in cases {
            let (base_uri, _) = mock_server(vec![response_text]).await;
            assert!(renderer(&base_uri).render(render_request()).await.is_err());
        }
    }

    #[tokio::test]
    async fn timeout_is_backend_local_failure() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock listener");
        let address = listener.local_addr().expect("mock address");
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept mock request");
            let _ = read_request(&mut stream).await;
            tokio::time::sleep(Duration::from_millis(200)).await;
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await;
        });
        assert_eq!(
            renderer(&format!("http://{address}/"))
                .render(render_request())
                .await,
            Err(RendererFailure::TimedOut)
        );
    }

    #[tokio::test]
    async fn grade_serializes_only_response_and_repeated_inputs_are_idempotent() {
        let body = serde_json::json!({"outcome": {"kind": "ungraded"}, "renderer": identity()})
            .to_string();
        let (base_uri, seen) = mock_server(vec![
            response("200 OK", "application/json", &body),
            response("200 OK", "application/json", &body),
        ])
        .await;
        let client = renderer(&base_uri);
        let student_response = StudentResponse::ShortText {
            text: "student work".to_string(),
        };
        let request = || GradeRequest {
            pg_source: b"DOCUMENT();",
            pg_path: "Library/Test.pg",
            version: "00000000-0000-0000-0000-000000000000",
            seed: 7,
            response: &student_response,
        };
        assert_eq!(client.grade(request()).await, Ok(GradeOutcome::Ungraded));
        assert_eq!(client.grade(request()).await, Ok(GradeOutcome::Ungraded));
        let seen = seen.lock().await;
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0], seen[1]);
        assert!(seen[0].starts_with("POST /v1/grade HTTP/1.1"));
        assert!(
            seen[0].contains("\"response\":{\"kind\":\"shortText\",\"text\":\"student work\"}")
        );
        assert!(!seen[0].contains("answerKey"));
        assert!(!seen[0].contains("correctAnswer"));
    }
}
