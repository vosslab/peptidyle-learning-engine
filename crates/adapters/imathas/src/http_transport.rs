//! Private fixed-dialect HTTP transport for contracted iMathAS scored embeds.
//!
//! This module is feature-gated because deployment composition, not authored
//! question data, selects a provider host. It accepts no browser URL, header,
//! path, redirect, or cookie input.

use std::time::Duration;

use async_trait::async_trait;
use base64::Engine as _;
use reqwest::header::{CONTENT_TYPE, HeaderName, HeaderValue, LOCATION, SET_COOKIE};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};

use crate::SafeProviderRender;
use crate::external_question_provider::{
    ContractedSnapshot, ExternalToolLaunchReference, ProtectedLaunchRequest, ProxyMethod,
    ProxyRequest, ProxyResponse, RenderTransportRequest, ResultTransportRequest,
    ScoredEmbedTransport, ScoredEmbedTransportFailure, SnapshotTransportRequest,
};

const SNAPSHOT_PATH: &str = "v1/imathas/snapshot";
const RENDER_PATH: &str = "v1/imathas/render";
const LAUNCH_PATH: &str = "v1/imathas/launch";
const RESULT_PATH: &str = "v1/imathas/result/";
const PROXY_PATH: &str = "v1/imathas/proxy/";
const MAX_BODY: usize = 1_048_576;
const PRIVATE_AUTH: &str = "x-ple-provider-auth";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpTransportConfigError {
    InvalidBaseUrl,
    InvalidLimits,
    InvalidPrivateAuth,
}
impl std::fmt::Display for HttpTransportConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid private iMathAS transport configuration")
    }
}
impl std::error::Error for HttpTransportConfigError {}

/// Fixed protected deployment configuration. Debug never includes auth value.
#[derive(Clone)]
pub struct HttpContractedScoredEmbedConfig {
    base: Url,
    timeout: Duration,
    max_body: usize,
    auth: Option<HeaderValue>,
}
impl std::fmt::Debug for HttpContractedScoredEmbedConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpContractedScoredEmbedConfig")
            .field("base", &self.base)
            .field("timeout", &self.timeout)
            .field("max_body", &self.max_body)
            .field("auth", &self.auth.as_ref().map(|_| "REDACTED"))
            .finish()
    }
}
impl HttpContractedScoredEmbedConfig {
    pub fn https(
        base: &str,
        timeout: Duration,
        max_body: usize,
    ) -> Result<Self, HttpTransportConfigError> {
        Self::new(base, timeout, max_body, false)
    }
    /// Test-only local fixture constructor; production callers must use HTTPS.
    pub fn loopback_http_for_test(
        base: &str,
        timeout: Duration,
        max_body: usize,
    ) -> Result<Self, HttpTransportConfigError> {
        Self::new(base, timeout, max_body, true)
    }
    fn new(
        base: &str,
        timeout: Duration,
        max_body: usize,
        allow_loopback_http: bool,
    ) -> Result<Self, HttpTransportConfigError> {
        let mut base = Url::parse(base).map_err(|_| HttpTransportConfigError::InvalidBaseUrl)?;
        let loopback = matches!(
            base.host_str(),
            Some("localhost") | Some("127.0.0.1") | Some("[::1]")
        );
        if !(base.scheme() == "https"
            || (allow_loopback_http && base.scheme() == "http" && loopback))
            || base.host_str().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(HttpTransportConfigError::InvalidBaseUrl);
        }
        if timeout.is_zero() || max_body == 0 || max_body > MAX_BODY {
            return Err(HttpTransportConfigError::InvalidLimits);
        }
        if !base.path().ends_with('/') {
            let path = format!("{}/", base.path());
            base.set_path(&path);
        }
        Ok(Self {
            base,
            timeout,
            max_body,
            auth: None,
        })
    }
    pub fn with_private_auth(mut self, value: &str) -> Result<Self, HttpTransportConfigError> {
        if value.is_empty() || value.len() > 4096 || value.contains(['\r', '\n']) {
            return Err(HttpTransportConfigError::InvalidPrivateAuth);
        }
        self.auth = Some(
            HeaderValue::from_str(value)
                .map_err(|_| HttpTransportConfigError::InvalidPrivateAuth)?,
        );
        Ok(self)
    }
}

#[derive(Clone)]
pub struct HttpContractedScoredEmbedTransport {
    client: Client,
    config: HttpContractedScoredEmbedConfig,
}
impl std::fmt::Debug for HttpContractedScoredEmbedTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpContractedScoredEmbedTransport")
            .field("config", &self.config)
            .finish()
    }
}
impl HttpContractedScoredEmbedTransport {
    pub fn new(config: HttpContractedScoredEmbedConfig) -> Result<Self, HttpTransportConfigError> {
        let client = Client::builder()
            .connect_timeout(config.timeout)
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| HttpTransportConfigError::InvalidLimits)?;
        Ok(Self { client, config })
    }
    fn url(&self, path: &str) -> Result<Url, ScoredEmbedTransportFailure> {
        self.config
            .base
            .join(path)
            .map_err(|_| ScoredEmbedTransportFailure::InvalidResponse)
    }
    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, ScoredEmbedTransportFailure> {
        let mut request = self
            .client
            .request(method, self.url(path)?)
            .header(CONTENT_TYPE, "application/json");
        if let Some(value) = &self.config.auth {
            request = request.header(HeaderName::from_static(PRIVATE_AUTH), value);
        }
        Ok(request)
    }
    async fn body(
        &self,
        response: reqwest::Response,
        accepted: &[StatusCode],
    ) -> Result<Vec<u8>, ScoredEmbedTransportFailure> {
        if response.status().is_server_error() {
            return Err(ScoredEmbedTransportFailure::Unavailable);
        }
        if !accepted.contains(&response.status())
            || response.headers().contains_key(LOCATION)
            || response.headers().contains_key(SET_COOKIE)
            || !response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.starts_with("application/json"))
        {
            return Err(ScoredEmbedTransportFailure::InvalidResponse);
        }
        if response
            .content_length()
            .is_some_and(|n| n as usize > self.config.max_body)
        {
            return Err(ScoredEmbedTransportFailure::InvalidResponse);
        }
        let mut out = Vec::new();
        let mut response = response;
        while let Some(chunk) = response.chunk().await.map_err(map_error)? {
            if out.len().saturating_add(chunk.len()) > self.config.max_body {
                return Err(ScoredEmbedTransportFailure::InvalidResponse);
            }
            out.extend_from_slice(&chunk);
        }
        Ok(out)
    }
}
fn map_error(error: reqwest::Error) -> ScoredEmbedTransportFailure {
    if error.is_timeout() {
        ScoredEmbedTransportFailure::Timeout
    } else if error.is_connect() {
        ScoredEmbedTransportFailure::Unavailable
    } else {
        ScoredEmbedTransportFailure::InvalidResponse
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotRequest<'a> {
    provider: &'a str,
    item_ref: &'a str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderRequest<'a> {
    provider: &'a str,
    snapshot_base64: String,
    version: String,
    seed: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchRequest<'a> {
    provider: &'a str,
    item_ref: &'a str,
    provider_seed: u16,
    source_digest: &'a str,
    signed_launch_jwt: &'a str,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HandleResponse {
    handle: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RenderResponse {
    title: String,
    prompt: Vec<question_model::envelope::QuestionContentBlock>,
}

#[async_trait]
impl ScoredEmbedTransport for HttpContractedScoredEmbedTransport {
    async fn fetch_snapshot(
        &self,
        request: SnapshotTransportRequest<'_>,
    ) -> Result<ContractedSnapshot, ScoredEmbedTransportFailure> {
        let response = self
            .request(reqwest::Method::POST, SNAPSHOT_PATH)?
            .json(&SnapshotRequest {
                provider: request.provider_key(),
                item_ref: request.item_ref(),
            })
            .send()
            .await
            .map_err(map_error)?;
        let bytes = self.body(response, &[StatusCode::OK]).await?;
        let payload: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|_| ScoredEmbedTransportFailure::InvalidResponse)?;
        let source = payload
            .get("snapshotBase64")
            .and_then(|v| v.as_str())
            .ok_or(ScoredEmbedTransportFailure::InvalidResponse)?;
        let snapshot = base64::engine::general_purpose::STANDARD
            .decode(source)
            .map_err(|_| ScoredEmbedTransportFailure::InvalidResponse)?;
        ContractedSnapshot::from_protected_bytes(snapshot)
    }
    async fn render_safe(
        &self,
        request: RenderTransportRequest<'_>,
    ) -> Result<SafeProviderRender, ScoredEmbedTransportFailure> {
        let response = self
            .request(reqwest::Method::POST, RENDER_PATH)?
            .json(&RenderRequest {
                provider: request.provider_key(),
                snapshot_base64: base64::engine::general_purpose::STANDARD
                    .encode(request.snapshot()),
                version: request.question_revision().revision_number.to_string(),
                seed: request.seed().value(),
            })
            .send()
            .await
            .map_err(map_error)?;
        let bytes = self.body(response, &[StatusCode::OK]).await?;
        let parsed: RenderResponse = serde_json::from_slice(&bytes)
            .map_err(|_| ScoredEmbedTransportFailure::InvalidResponse)?;
        Ok(SafeProviderRender {
            title: parsed.title,
            prompt: parsed.prompt,
        })
    }
    async fn start_protected_launch(
        &self,
        request: ProtectedLaunchRequest,
    ) -> Result<ExternalToolLaunchReference, ScoredEmbedTransportFailure> {
        let response = self
            .request(reqwest::Method::POST, LAUNCH_PATH)?
            .json(&LaunchRequest {
                provider: request.provider_key(),
                item_ref: request.item_ref(),
                provider_seed: request.provider_seed(),
                source_digest: request.source_digest(),
                signed_launch_jwt: request.signed_launch_jwt(),
            })
            .send()
            .await
            .map_err(map_error)?;
        let bytes = self
            .body(response, &[StatusCode::OK, StatusCode::CREATED])
            .await?;
        let parsed: HandleResponse = serde_json::from_slice(&bytes)
            .map_err(|_| ScoredEmbedTransportFailure::InvalidResponse)?;
        ExternalToolLaunchReference::from_server_handle(parsed.handle)
    }
    async fn fetch_signed_grade_get(
        &self,
        request: ResultTransportRequest<'_>,
    ) -> Result<Vec<u8>, ScoredEmbedTransportFailure> {
        let path = format!("{RESULT_PATH}{}", request.handle().protected_value());
        let response = self
            .request(reqwest::Method::GET, &path)?
            .send()
            .await
            .map_err(map_error)?;
        self.body(response, &[StatusCode::OK]).await
    }
    async fn proxy_activity(
        &self,
        request: ProxyRequest<'_>,
    ) -> Result<ProxyResponse, ScoredEmbedTransportFailure> {
        let path = format!("{PROXY_PATH}{}", request.handle().protected_value());
        let method = match request.method() {
            ProxyMethod::Get => reqwest::Method::GET,
            ProxyMethod::Post => reqwest::Method::POST,
        };
        let mut call = self.client.request(method, self.url(&path)?);
        if let Some(value) = &self.config.auth {
            call = call.header(HeaderName::from_static(PRIVATE_AUTH), value);
        }
        if matches!(request.method(), ProxyMethod::Post) {
            call = call
                .header(CONTENT_TYPE, "application/octet-stream")
                .body(request.body().to_vec());
        }
        let response = call.send().await.map_err(map_error)?;
        if response.status().is_server_error() {
            return Err(ScoredEmbedTransportFailure::Unavailable);
        }
        if response.status() != StatusCode::OK
            || response.headers().contains_key(LOCATION)
            || response.headers().contains_key(SET_COOKIE)
            || !response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.starts_with("text/html"))
        {
            return Err(ScoredEmbedTransportFailure::InvalidResponse);
        }
        if response
            .content_length()
            .is_some_and(|n| n as usize > self.config.max_body)
        {
            return Err(ScoredEmbedTransportFailure::InvalidResponse);
        }
        let mut out = Vec::new();
        let mut response = response;
        while let Some(chunk) = response.chunk().await.map_err(map_error)? {
            if out.len().saturating_add(chunk.len()) > self.config.max_body {
                return Err(ScoredEmbedTransportFailure::InvalidResponse);
            }
            out.extend_from_slice(&chunk);
        }
        ProxyResponse::protected_html(out)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::Router;
    use axum::extract::State;
    use axum::http::{HeaderValue, StatusCode as AxumStatus};
    use axum::response::IntoResponse;
    use axum::routing::{any, get, post};

    use super::*;

    #[derive(Clone)]
    struct Fixture(Arc<Mutex<&'static str>>);
    async fn snapshot(State(fixture): State<Fixture>) -> axum::response::Response {
        let mode = *fixture.0.lock().unwrap();
        match mode {
            "ok" => (
                [(CONTENT_TYPE, HeaderValue::from_static("application/json"))],
                r#"{"snapshotBase64":"eyJyZWNvcmRlZCI6dHJ1ZX0="}"#,
            )
                .into_response(),
            "redirect" => (
                AxumStatus::FOUND,
                [(
                    LOCATION,
                    HeaderValue::from_static("https://foreign.example/"),
                )],
            )
                .into_response(),
            "cookie" => (
                [
                    (CONTENT_TYPE, HeaderValue::from_static("application/json")),
                    (SET_COOKIE, HeaderValue::from_static("secret=bad")),
                ],
                r#"{"snapshotBase64":"eyJyZWNvcmRlZCI6dHJ1ZX0="}"#,
            )
                .into_response(),
            "wrong-type" => (
                [(CONTENT_TYPE, HeaderValue::from_static("text/plain"))],
                "no",
            )
                .into_response(),
            "oversize" => (
                [(CONTENT_TYPE, HeaderValue::from_static("application/json"))],
                "x".repeat(2048),
            )
                .into_response(),
            "slow" => {
                tokio::time::sleep(Duration::from_millis(100)).await;
                (
                    [(CONTENT_TYPE, HeaderValue::from_static("application/json"))],
                    r#"{"snapshotBase64":"eyJyZWNvcmRlZCI6dHJ1ZX0="}"#,
                )
                    .into_response()
            }
            _ => (
                AxumStatus::SERVICE_UNAVAILABLE,
                [(CONTENT_TYPE, HeaderValue::from_static("application/json"))],
                "{}",
            )
                .into_response(),
        }
    }
    async fn render() -> impl IntoResponse {
        (
            [(CONTENT_TYPE, HeaderValue::from_static("application/json"))],
            r#"{"title":"Recorded","prompt":[{"kind":"text","markdown":"Hi"}]}"#,
        )
    }
    async fn launch() -> impl IntoResponse {
        (
            [(CONTENT_TYPE, HeaderValue::from_static("application/json"))],
            r#"{"handle":"fixture-handle"}"#,
        )
    }
    async fn result() -> impl IntoResponse {
        (
            [(CONTENT_TYPE, HeaderValue::from_static("application/json"))],
            "signed-result",
        )
    }
    async fn proxy() -> impl IntoResponse {
        (
            [(CONTENT_TYPE, HeaderValue::from_static("text/html"))],
            "<main>fixture</main>",
        )
    }
    async fn fixture() -> (Fixture, String) {
        let fixture = Fixture(Arc::new(Mutex::new("ok")));
        let app = Router::new()
            .route("/v1/imathas/snapshot", post(snapshot))
            .route("/v1/imathas/render", post(render))
            .route("/v1/imathas/launch", post(launch))
            .route("/v1/imathas/result/fixture-handle", get(result))
            .route("/v1/imathas/proxy/fixture-handle", any(proxy))
            .with_state(fixture.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (fixture, format!("http://{address}/"))
    }
    fn locator() -> crate::ImathasQuestionLocation {
        crate::ImathasQuestionLocation::from_draft_backend_locator(
            &question_model::DraftQuestionBackendLocator::Imathas {
                provider: "self-hosted-imathas".into(),
                item_ref: "17".into(),
            },
        )
        .unwrap()
    }
    #[tokio::test]
    #[ignore = "opt-in loopback HTTP transport acceptance"]
    async fn recorded_http_snapshot_is_bounded_fixed_and_hostile_responses_refuse_redacted() {
        let (fixture, base) = fixture().await;
        let config = HttpContractedScoredEmbedConfig::loopback_http_for_test(
            &base,
            Duration::from_secs(1),
            1024,
        )
        .unwrap()
        .with_private_auth("fixture-secret")
        .unwrap();
        assert!(!format!("{config:?}").contains("fixture-secret"));
        let transport = HttpContractedScoredEmbedTransport::new(config).unwrap();
        let request = SnapshotTransportRequest {
            locator: &locator(),
            provider_key: "self-hosted-imathas",
        };
        let snapshot = transport.fetch_snapshot(request).await.unwrap();
        assert_eq!(snapshot.bytes(), br#"{"recorded":true}"#);
        for mode in ["redirect", "cookie", "wrong-type", "oversize"] {
            *fixture.0.lock().unwrap() = mode;
            let error = transport
                .fetch_snapshot(SnapshotTransportRequest {
                    locator: &locator(),
                    provider_key: "self-hosted-imathas",
                })
                .await
                .unwrap_err();
            assert_eq!(error, ScoredEmbedTransportFailure::InvalidResponse);
            assert!(!format!("{error:?}").contains("fixture-secret"));
        }
        *fixture.0.lock().unwrap() = "error";
        assert_eq!(
            transport
                .fetch_snapshot(SnapshotTransportRequest {
                    locator: &locator(),
                    provider_key: "self-hosted-imathas"
                })
                .await
                .unwrap_err(),
            ScoredEmbedTransportFailure::Unavailable
        );
        assert!(
            HttpContractedScoredEmbedConfig::https("http://127.0.0.1/", Duration::from_secs(1), 1)
                .is_err()
        );
        assert!(
            HttpContractedScoredEmbedConfig::https(
                "https://x.example/?token=bad",
                Duration::from_secs(1),
                1
            )
            .is_err()
        );
    }

    #[tokio::test]
    #[ignore = "opt-in loopback HTTP transport acceptance"]
    async fn recorded_http_exercises_remaining_fixed_operations() {
        let (_, base) = fixture().await;
        let transport = HttpContractedScoredEmbedTransport::new(
            HttpContractedScoredEmbedConfig::loopback_http_for_test(
                &base,
                Duration::from_secs(1),
                1024,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            transport
                .render_safe(RenderTransportRequest {
                    snapshot: b"{}",
                    provider_key: "self-hosted-imathas",
                    question_revision: question_model::QuestionRevisionReference {
                        question_id: question_model::QuestionId::from_canonical_parts(
                            "ABCDEF", 'G'
                        )
                        .expect("Question ID"),
                        revision_number: question_model::QuestionRevisionNumber::new(1)
                            .expect("positive version"),
                    },
                    seed: question_model::generation::QuestionSeed::new(7)
                })
                .await
                .unwrap()
                .title,
            "Recorded"
        );
        let handle = transport
            .start_protected_launch(ProtectedLaunchRequest {
                provider_key: "self-hosted-imathas".into(),
                item_ref: "17".into(),
                provider_seed: 7,
                source_digest: "a".repeat(64),
                signed_launch_jwt: "protected.jwt.value".into(),
            })
            .await
            .unwrap();
        let binding = crate::GradeBinding {
            attempt: question_model::QuestionAttemptId::from_uuid(uuid::Uuid::from_u128(2)),
            question_revision: question_model::QuestionRevisionReference {
                question_id: question_model::QuestionId::from_canonical_parts("BCDEFG", 'H')
                    .expect("Question ID"),
                revision_number: question_model::QuestionRevisionNumber::new(4)
                    .expect("positive version"),
            },
            seed: question_model::generation::QuestionSeed::new(7),
        };
        let issuer = crate::CorrelationIssuer::from_server_secret([1; 32]);
        let correlation = issuer
            .restore(binding.clone(), &issuer.begin(binding.clone()))
            .unwrap();
        assert_eq!(
            transport
                .fetch_signed_grade_get(ResultTransportRequest {
                    handle: &handle,
                    correlation: &correlation,
                    provider_key: "self-hosted-imathas"
                })
                .await
                .unwrap(),
            b"signed-result"
        );
        assert_eq!(
            transport
                .proxy_activity(ProxyRequest {
                    handle: &handle,
                    method: ProxyMethod::Get,
                    body: &[]
                })
                .await
                .unwrap()
                .html(),
            b"<main>fixture</main>"
        );
        assert_eq!(
            transport
                .proxy_activity(ProxyRequest {
                    handle: &handle,
                    method: ProxyMethod::Post,
                    body: b"bounded"
                })
                .await
                .unwrap()
                .html(),
            b"<main>fixture</main>"
        );
    }

    #[tokio::test]
    #[ignore = "opt-in loopback HTTP transport acceptance"]
    async fn timeout_and_refused_connection_are_redacted_unavailable_categories() {
        let (fixture, base) = fixture().await;
        *fixture.0.lock().unwrap() = "slow";
        let timeout = HttpContractedScoredEmbedTransport::new(
            HttpContractedScoredEmbedConfig::loopback_http_for_test(
                &base,
                Duration::from_millis(5),
                1024,
            )
            .unwrap()
            .with_private_auth("timeout-secret")
            .unwrap(),
        )
        .unwrap();
        let error = timeout
            .fetch_snapshot(SnapshotTransportRequest {
                locator: &locator(),
                provider_key: "self-hosted-imathas",
            })
            .await
            .unwrap_err();
        assert_eq!(error, ScoredEmbedTransportFailure::Timeout);
        assert!(!format!("{error:?}").contains("timeout-secret"));
        let refused = HttpContractedScoredEmbedTransport::new(
            HttpContractedScoredEmbedConfig::loopback_http_for_test(
                "http://127.0.0.1:9/",
                Duration::from_millis(30),
                1024,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            refused
                .fetch_snapshot(SnapshotTransportRequest {
                    locator: &locator(),
                    provider_key: "self-hosted-imathas"
                })
                .await
                .unwrap_err(),
            ScoredEmbedTransportFailure::Unavailable
        );
    }
}
