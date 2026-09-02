//! Feature-gated recorded iMathAS Question Backend for local server integration tests.
//!
//! This module is deliberately absent from the default production dependency
//! closure. It has no network client and never exposes a way to construct a
//! `VerifiedImathasResult` from test or browser data.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use base64::Engine as _;
use hmac::{Hmac, KeyInit, Mac};
use question_model::envelope::QuestionContentBlock;
use sha2::Sha256;

use crate::imathas_question_backend::{
    ImathasLaunchReference, ImathasQuestionBackend, ImathasQuestionBackendConfig,
    ImathasQuestionBackendSnapshot, ImathasQuestionBackendTransport, ImathasTransportFailure,
    ProtectedLaunchRequest, ProxyRequest, ProxyResponse, ResultTransportRequest,
    SnapshotTransportRequest,
};
use crate::result_verification::ImathasGradingProfile;
use crate::{
    ImathasQuestionBackendFailure, ImathasQuestionLocation, ImathasRenderRequest,
    ImathasResultRequest, QuestionBackend, SafeImathasQuestionRender, SupportedImathasProfile,
    VerifiedImathasResult, sealed,
};

/// Safe, deterministic behavior selected by a local server test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordedImathasQuestionBackendMode {
    /// Return a deterministic correct grade after exact request binding.
    Verified,
    /// Inject an iMathAS outage without fabricating student correctness.
    Unavailable,
    /// Inject a bounded iMathAS timeout.
    Timeout,
    /// Inject malformed upstream material.
    InvalidResponse,
}

impl RecordedImathasQuestionBackend {
    /// Constructs one iMathAS deployment with independent deterministic call counters.
    pub fn from_mode(mode: RecordedImathasQuestionBackendMode) -> Self {
        RecordedImathasQuestionBackend {
            mode,
            snapshot_calls: Arc::new(AtomicUsize::new(0)),
            render_calls: Arc::new(AtomicUsize::new(0)),
            grade_calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

/// Test-only iMathAS Question Backend with safe canned source, render, and verifier behavior.
#[derive(Clone)]
pub struct RecordedImathasQuestionBackend {
    mode: RecordedImathasQuestionBackendMode,
    snapshot_calls: Arc<AtomicUsize>,
    render_calls: Arc<AtomicUsize>,
    grade_calls: Arc<AtomicUsize>,
}

impl std::fmt::Debug for RecordedImathasQuestionBackend {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecordedImathasQuestionBackend")
            .field("mode", &self.mode)
            .field("snapshot_calls", &self.snapshot_calls())
            .field("render_calls", &self.render_calls())
            .field("grade_calls", &self.grade_calls())
            .finish()
    }
}

impl RecordedImathasQuestionBackend {
    /// Number of local snapshot calls.
    pub fn snapshot_calls(&self) -> usize {
        self.snapshot_calls.load(Ordering::SeqCst)
    }
    /// Number of local render calls.
    pub fn render_calls(&self) -> usize {
        self.render_calls.load(Ordering::SeqCst)
    }
    /// Number of local verified-grade calls.
    pub fn grade_calls(&self) -> usize {
        self.grade_calls.load(Ordering::SeqCst)
    }
}

impl sealed::QuestionBackendSealed for RecordedImathasQuestionBackend {}

#[async_trait]
impl QuestionBackend for RecordedImathasQuestionBackend {
    async fn snapshot(
        &self,
        _locator: &ImathasQuestionLocation,
    ) -> Result<(Vec<u8>, SupportedImathasProfile), ImathasQuestionBackendFailure> {
        self.snapshot_calls.fetch_add(1, Ordering::SeqCst);
        mode_result(self.mode)?;
        Ok((
            br#"{"recorded":true}"#.to_vec(),
            SupportedImathasProfile::new(
                question_model::ImathasProfile::new("recorded-v1").expect("valid test profile"),
                true,
                true,
                true,
            )
            .expect("recorded test profile is valid"),
        ))
    }

    async fn render(
        &self,
        _request: ImathasRenderRequest<'_>,
    ) -> Result<SafeImathasQuestionRender, ImathasQuestionBackendFailure> {
        self.render_calls.fetch_add(1, Ordering::SeqCst);
        mode_result(self.mode)?;
        Ok(SafeImathasQuestionRender {
            title: "Recorded iMathAS question".into(),
            prompt: vec![QuestionContentBlock::Text {
                markdown: "Complete the recorded iMathAS Question Backend activity.".into(),
            }],
        })
    }

    async fn verify_result(
        &self,
        request: ImathasResultRequest<'_>,
    ) -> Result<VerifiedImathasResult, ImathasQuestionBackendFailure> {
        self.grade_calls.fetch_add(1, Ordering::SeqCst);
        mode_result(self.mode)?;
        Ok(VerifiedImathasResult::from_test_support(
            learning_data_access::ImathasResult::new(
                learning_data_access::ImathasNormalizedScore::try_from_f64(1.0)
                    .expect("recorded score is valid"),
            ),
            request.grading_context().clone(),
            request.launch_session_authentication(),
            verified_token_checksum(),
        ))
    }
}

fn verified_token_checksum() -> learning_data_access::ImathasResultTokenChecksum {
    let token = learning_data_access::ImathasResultToken::from_server_adapter_bytes(
        b"recorded test iMathAS result".to_vec(),
    )
    .expect("bounded iMathAS result token");
    learning_data_access::ImathasResultTokenChecksum::from_verified_token(&token)
}

fn mode_result(
    mode: RecordedImathasQuestionBackendMode,
) -> Result<(), ImathasQuestionBackendFailure> {
    match mode {
        RecordedImathasQuestionBackendMode::Verified => Ok(()),
        RecordedImathasQuestionBackendMode::Unavailable => {
            Err(ImathasQuestionBackendFailure::Unavailable)
        }
        RecordedImathasQuestionBackendMode::Timeout => Err(ImathasQuestionBackendFailure::Timeout),
        RecordedImathasQuestionBackendMode::InvalidResponse => {
            Err(ImathasQuestionBackendFailure::InvalidResponse)
        }
    }
}

/// Fixed activity behavior for the recorded iMathAS Question Backend transport.
/// It deliberately has no grade-result mode: route tests may exercise launch
/// and proxy isolation but cannot manufacture an iMathAS verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordedImathasQuestionBackendTransportMode {
    /// Launch normally, but do not expose a grade result. This preserves the
    /// route-only fixture used where a verdict would be misleading.
    Available,
    /// Return one deterministic signed, server-verified result after launch.
    /// This is only available behind this crate's `test-support` feature.
    Verified,
    /// A valid signed lower-bound normalized result.
    ZeroScore,
    /// Launch succeeds but server-to-server result retrieval is unavailable.
    /// It proves an upstream failure never manufactures a student grade.
    ResultUnavailable,
    /// A signed iMathAS response with an intentionally wrong exact binding.
    WrongSignedResult,
    /// A signed iMathAS response bound to a different iMathAS Item Reference.
    WrongImathasItemReference,
    /// A signed iMathAS response with a score outside the supported range.
    InvalidScore,
    /// A signed negative-zero score, which the canonical result rejects.
    NegativeZeroScore,
    /// A signed iMathAS response whose optional expiry has elapsed.
    ExpiredSignedResult,
    /// A syntactically valid JWT whose header selects a refused algorithm.
    WrongAlgorithm,
    /// A token with an invalid signature.
    InvalidSignature,
    /// A malformed iMathAS response.
    MalformedResult,
    /// An iMathAS response that cannot be decoded as UTF-8.
    NonUtf8Result,
    /// An iMathAS response above the LDA and adapter byte limits.
    OversizedResult,
    /// The iMathAS deployment's current source no longer matches the immutable snapshot.
    SourceChanged,
    Unavailable,
}

/// Constructs the recorded transport used by the bounded iMathAS Question Backend
/// route tests. It accepts neither a URL nor credentials and exposes only the
/// fixed immutable snapshot and activity document.
pub fn recorded_imathas_question_backend_transport(
    mode: RecordedImathasQuestionBackendTransportMode,
) -> RecordedImathasQuestionBackendTransport {
    RecordedImathasQuestionBackendTransport {
        mode,
        snapshot_calls: Arc::new(AtomicUsize::new(0)),
        launch_calls: Arc::new(AtomicUsize::new(0)),
        proxy_calls: Arc::new(AtomicUsize::new(0)),
        result_calls: Arc::new(AtomicUsize::new(0)),
        launch_claims: Arc::new(Mutex::new(None)),
        result_token_bytes: Arc::new(Mutex::new(None)),
    }
}

/// Constructs only the bounded iMathAS Question Backend used by local route tests. Its signing
/// and verification keys are fixed test constants owned by this adapter.
pub fn recorded_imathas_question_backend(
    mode: RecordedImathasQuestionBackendTransportMode,
) -> ImathasQuestionBackend<RecordedImathasQuestionBackendTransport> {
    recorded_imathas_question_backend_with_transport(mode).0
}

/// Constructs the bounded iMathAS Question Backend with a cloned test-only transport handle for
/// server acceptance tests. The handle exposes no deployment endpoint, launch
/// session, or Student input.
pub fn recorded_imathas_question_backend_with_transport(
    mode: RecordedImathasQuestionBackendTransportMode,
) -> (
    ImathasQuestionBackend<RecordedImathasQuestionBackendTransport>,
    RecordedImathasQuestionBackendTransport,
) {
    let transport = recorded_imathas_question_backend_transport(mode);
    let question_backend = ImathasQuestionBackend::new(
        ImathasQuestionBackendConfig::new(
            ImathasGradingProfile::grading_deployment("self-hosted-imathas", true, true)
                .expect("recorded iMathAS grading profile"),
            b"recorded-launch-secret",
            b"recorded-result-secret",
            crate::ImathasSessionAuthenticationCodec::from_server_secret([9; 32])
                .expect("recorded launch authentication codec"),
            30_000,
        )
        .expect("recorded iMathAS Question Backend configuration"),
        transport.clone(),
    );
    (question_backend, transport)
}

/// Recorded server-only transport. No public method accepts a score, answer,
/// iMathAS result token, URL, JWT, source digest, or iMathAS Question Backend launch reference.
#[derive(Clone)]
pub struct RecordedImathasQuestionBackendTransport {
    mode: RecordedImathasQuestionBackendTransportMode,
    snapshot_calls: Arc<AtomicUsize>,
    launch_calls: Arc<AtomicUsize>,
    proxy_calls: Arc<AtomicUsize>,
    result_calls: Arc<AtomicUsize>,
    launch_claims: Arc<Mutex<Option<RecordedLaunchClaims>>>,
    result_token_bytes: Arc<Mutex<Option<Vec<u8>>>>,
}

#[derive(Clone)]
struct RecordedLaunchClaims {
    item: String,
    challenge: String,
    binding: String,
}

impl RecordedImathasQuestionBackendTransport {
    pub fn snapshot_calls(&self) -> usize {
        self.snapshot_calls.load(Ordering::SeqCst)
    }

    pub fn launch_calls(&self) -> usize {
        self.launch_calls.load(Ordering::SeqCst)
    }

    pub fn proxy_calls(&self) -> usize {
        self.proxy_calls.load(Ordering::SeqCst)
    }

    /// Number of server-only result retrievals. It is intentionally separate
    /// from launch/proxy activity so replay tests can prove no second grade.
    pub fn result_calls(&self) -> usize {
        self.result_calls.load(Ordering::SeqCst)
    }

    /// Exact signed result bytes returned by this recorded transport.
    ///
    /// This test-support observation proves the receipt binds the transport
    /// response rather than a separately reconstructed token.
    pub fn recorded_result_token_bytes(&self) -> Option<Vec<u8>> {
        self.result_token_bytes
            .lock()
            .expect("recorded result token")
            .clone()
    }
}

#[async_trait]
impl ImathasQuestionBackendTransport for RecordedImathasQuestionBackendTransport {
    async fn fetch_snapshot(
        &self,
        _request: SnapshotTransportRequest<'_>,
    ) -> Result<ImathasQuestionBackendSnapshot, ImathasTransportFailure> {
        self.snapshot_calls.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            RecordedImathasQuestionBackendTransportMode::Available
            | RecordedImathasQuestionBackendTransportMode::Verified
            | RecordedImathasQuestionBackendTransportMode::ZeroScore
            | RecordedImathasQuestionBackendTransportMode::ResultUnavailable
            | RecordedImathasQuestionBackendTransportMode::WrongSignedResult
            | RecordedImathasQuestionBackendTransportMode::WrongImathasItemReference
            | RecordedImathasQuestionBackendTransportMode::InvalidScore
            | RecordedImathasQuestionBackendTransportMode::NegativeZeroScore
            | RecordedImathasQuestionBackendTransportMode::ExpiredSignedResult
            | RecordedImathasQuestionBackendTransportMode::WrongAlgorithm
            | RecordedImathasQuestionBackendTransportMode::InvalidSignature
            | RecordedImathasQuestionBackendTransportMode::MalformedResult
            | RecordedImathasQuestionBackendTransportMode::NonUtf8Result
            | RecordedImathasQuestionBackendTransportMode::OversizedResult => {
                ImathasQuestionBackendSnapshot::from_protected_bytes(
                    br#"{"recorded":true}"#.to_vec(),
                )
            }
            RecordedImathasQuestionBackendTransportMode::SourceChanged => {
                ImathasQuestionBackendSnapshot::from_protected_bytes(
                    br#"{"recorded":false}"#.to_vec(),
                )
            }
            RecordedImathasQuestionBackendTransportMode::Unavailable => {
                Err(ImathasTransportFailure::Unavailable)
            }
        }
    }

    async fn render_safe(
        &self,
        _request: crate::imathas_question_backend::RenderTransportRequest<'_>,
    ) -> Result<SafeImathasQuestionRender, ImathasTransportFailure> {
        Ok(SafeImathasQuestionRender {
            title: "Recorded iMathAS Question Backend question".into(),
            prompt: vec![QuestionContentBlock::Text {
                markdown: "Complete the protected activity.".into(),
            }],
        })
    }

    async fn start_protected_launch(
        &self,
        request: ProtectedLaunchRequest,
    ) -> Result<ImathasLaunchReference, ImathasTransportFailure> {
        self.launch_calls.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            RecordedImathasQuestionBackendTransportMode::Available
            | RecordedImathasQuestionBackendTransportMode::Verified
            | RecordedImathasQuestionBackendTransportMode::ZeroScore
            | RecordedImathasQuestionBackendTransportMode::ResultUnavailable
            | RecordedImathasQuestionBackendTransportMode::WrongSignedResult
            | RecordedImathasQuestionBackendTransportMode::WrongImathasItemReference
            | RecordedImathasQuestionBackendTransportMode::InvalidScore
            | RecordedImathasQuestionBackendTransportMode::NegativeZeroScore
            | RecordedImathasQuestionBackendTransportMode::ExpiredSignedResult
            | RecordedImathasQuestionBackendTransportMode::WrongAlgorithm
            | RecordedImathasQuestionBackendTransportMode::InvalidSignature
            | RecordedImathasQuestionBackendTransportMode::MalformedResult
            | RecordedImathasQuestionBackendTransportMode::NonUtf8Result
            | RecordedImathasQuestionBackendTransportMode::OversizedResult
            | RecordedImathasQuestionBackendTransportMode::SourceChanged => {
                let claims = recorded_launch_claims(request.signed_launch_jwt())?;
                *self.launch_claims.lock().expect("recorded launch claims") = Some(claims);
                ImathasLaunchReference::from_server_handle("recorded-proxy-session")
            }
            RecordedImathasQuestionBackendTransportMode::Unavailable => {
                Err(ImathasTransportFailure::Unavailable)
            }
        }
    }

    async fn fetch_signed_grade_get(
        &self,
        _request: ResultTransportRequest<'_>,
    ) -> Result<Vec<u8>, ImathasTransportFailure> {
        self.result_calls.fetch_add(1, Ordering::SeqCst);
        let response = match self.mode {
            RecordedImathasQuestionBackendTransportMode::Verified => {
                let claims = self
                    .launch_claims
                    .lock()
                    .expect("recorded launch claims")
                    .clone()
                    .ok_or(ImathasTransportFailure::InvalidResponse)?;
                Ok(recorded_result_token(&claims).into_bytes())
            }
            RecordedImathasQuestionBackendTransportMode::ZeroScore => {
                let claims = self
                    .launch_claims
                    .lock()
                    .expect("recorded launch claims")
                    .clone()
                    .ok_or(ImathasTransportFailure::InvalidResponse)?;
                Ok(recorded_result_token_with_score(&claims, "0.0").into_bytes())
            }
            RecordedImathasQuestionBackendTransportMode::WrongSignedResult => {
                let mut claims = self
                    .launch_claims
                    .lock()
                    .expect("recorded launch claims")
                    .clone()
                    .ok_or(ImathasTransportFailure::InvalidResponse)?;
                claims.binding = "0".repeat(64);
                Ok(recorded_result_token(&claims).into_bytes())
            }
            RecordedImathasQuestionBackendTransportMode::WrongImathasItemReference => {
                let mut claims = self
                    .launch_claims
                    .lock()
                    .expect("recorded launch claims")
                    .clone()
                    .ok_or(ImathasTransportFailure::InvalidResponse)?;
                claims.item = "wrong-item".into();
                Ok(recorded_result_token(&claims).into_bytes())
            }
            RecordedImathasQuestionBackendTransportMode::InvalidScore => {
                let claims = self
                    .launch_claims
                    .lock()
                    .expect("recorded launch claims")
                    .clone()
                    .ok_or(ImathasTransportFailure::InvalidResponse)?;
                Ok(recorded_result_token_with_score(&claims, "1.1").into_bytes())
            }
            RecordedImathasQuestionBackendTransportMode::NegativeZeroScore => {
                let claims = self
                    .launch_claims
                    .lock()
                    .expect("recorded launch claims")
                    .clone()
                    .ok_or(ImathasTransportFailure::InvalidResponse)?;
                Ok(recorded_result_token_with_score(&claims, "-0.0").into_bytes())
            }
            RecordedImathasQuestionBackendTransportMode::ExpiredSignedResult => {
                let claims = self
                    .launch_claims
                    .lock()
                    .expect("recorded launch claims")
                    .clone()
                    .ok_or(ImathasTransportFailure::InvalidResponse)?;
                Ok(recorded_result_token_with_extra(&claims, ",\"exp\":0").into_bytes())
            }
            RecordedImathasQuestionBackendTransportMode::WrongAlgorithm => {
                let claims = self
                    .launch_claims
                    .lock()
                    .expect("recorded launch claims")
                    .clone()
                    .ok_or(ImathasTransportFailure::InvalidResponse)?;
                Ok(recorded_result_token_with_header(&claims, br#"{"alg":"none"}"#).into_bytes())
            }
            RecordedImathasQuestionBackendTransportMode::InvalidSignature => {
                let claims = self
                    .launch_claims
                    .lock()
                    .expect("recorded launch claims")
                    .clone()
                    .ok_or(ImathasTransportFailure::InvalidResponse)?;
                let mut token = recorded_result_token(&claims).into_bytes();
                let last = token.last_mut().expect("recorded token is nonempty");
                *last = if *last == b'a' { b'b' } else { b'a' };
                Ok(token)
            }
            RecordedImathasQuestionBackendTransportMode::MalformedResult => {
                Ok(b"not-a-jwt".to_vec())
            }
            RecordedImathasQuestionBackendTransportMode::NonUtf8Result => Ok(vec![0xff]),
            RecordedImathasQuestionBackendTransportMode::OversizedResult => Ok(vec![b'x'; 8_193]),
            // Route-only and outage fixtures must never manufacture a grade.
            RecordedImathasQuestionBackendTransportMode::Available => {
                Err(ImathasTransportFailure::Unsupported)
            }
            RecordedImathasQuestionBackendTransportMode::ResultUnavailable => {
                Err(ImathasTransportFailure::Unavailable)
            }
            RecordedImathasQuestionBackendTransportMode::SourceChanged => {
                Err(ImathasTransportFailure::InvalidResponse)
            }
            RecordedImathasQuestionBackendTransportMode::Unavailable => {
                Err(ImathasTransportFailure::Unavailable)
            }
        };
        if let Ok(token_bytes) = &response {
            *self
                .result_token_bytes
                .lock()
                .expect("recorded result token") = Some(token_bytes.clone());
        }
        response
    }

    async fn proxy_activity(
        &self,
        _request: ProxyRequest<'_>,
    ) -> Result<ProxyResponse, ImathasTransportFailure> {
        self.proxy_calls.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            RecordedImathasQuestionBackendTransportMode::Available
            | RecordedImathasQuestionBackendTransportMode::Verified
            | RecordedImathasQuestionBackendTransportMode::ZeroScore
            | RecordedImathasQuestionBackendTransportMode::ResultUnavailable
            | RecordedImathasQuestionBackendTransportMode::WrongSignedResult
            | RecordedImathasQuestionBackendTransportMode::WrongImathasItemReference
            | RecordedImathasQuestionBackendTransportMode::InvalidScore
            | RecordedImathasQuestionBackendTransportMode::NegativeZeroScore
            | RecordedImathasQuestionBackendTransportMode::ExpiredSignedResult
            | RecordedImathasQuestionBackendTransportMode::WrongAlgorithm
            | RecordedImathasQuestionBackendTransportMode::InvalidSignature
            | RecordedImathasQuestionBackendTransportMode::MalformedResult
            | RecordedImathasQuestionBackendTransportMode::NonUtf8Result
            | RecordedImathasQuestionBackendTransportMode::OversizedResult
            | RecordedImathasQuestionBackendTransportMode::SourceChanged => {
                ProxyResponse::protected_html(
                    b"<!doctype html><title>Recorded protected activity</title>".to_vec(),
                )
            }
            RecordedImathasQuestionBackendTransportMode::Unavailable => {
                Err(ImathasTransportFailure::Unavailable)
            }
        }
    }
}

fn recorded_launch_claims(
    signed_launch_jwt: &str,
) -> Result<RecordedLaunchClaims, ImathasTransportFailure> {
    let payload = signed_launch_jwt
        .split('.')
        .nth(1)
        .ok_or(ImathasTransportFailure::InvalidResponse)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| ImathasTransportFailure::InvalidResponse)?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| ImathasTransportFailure::InvalidResponse)?;
    let challenge = value
        .get("ple_launch_challenge")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 256)
        .ok_or(ImathasTransportFailure::InvalidResponse)?;
    let item = value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .ok_or(ImathasTransportFailure::InvalidResponse)?;
    let binding = value
        .get("ple_binding")
        .and_then(serde_json::Value::as_str)
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or(ImathasTransportFailure::InvalidResponse)?;
    Ok(RecordedLaunchClaims {
        item: item.to_owned(),
        challenge: challenge.to_owned(),
        binding: binding.to_owned(),
    })
}

fn recorded_result_token(claims: &RecordedLaunchClaims) -> String {
    recorded_result_token_with_header_and_extra(claims, br#"{"alg":"HS256","typ":"JWT"}"#, "")
}

fn recorded_result_token_with_extra(claims: &RecordedLaunchClaims, extra: &str) -> String {
    recorded_result_token_with_header_and_extra(claims, br#"{"alg":"HS256","typ":"JWT"}"#, extra)
}

fn recorded_result_token_with_score(claims: &RecordedLaunchClaims, score: &str) -> String {
    let codec = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = codec.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = codec.encode(format!(
        r#"{{"id":"{}","score":{score},"ple_launch_challenge":"{}","ple_binding":"{}"}}"#,
        claims.item, claims.challenge, claims.binding,
    ));
    let signed = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(b"recorded-result-secret")
        .expect("fixed recorded result secret");
    mac.update(signed.as_bytes());
    format!("{signed}.{}", codec.encode(mac.finalize().into_bytes()))
}

fn recorded_result_token_with_header(claims: &RecordedLaunchClaims, header: &[u8]) -> String {
    recorded_result_token_with_header_and_extra(claims, header, "")
}

fn recorded_result_token_with_header_and_extra(
    claims: &RecordedLaunchClaims,
    header: &[u8],
    extra: &str,
) -> String {
    let codec = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = codec.encode(header);
    let payload = codec.encode(format!(
        r#"{{"id":"{}","score":1.0,"ple_launch_challenge":"{}","ple_binding":"{}"{extra}}}"#,
        claims.item, claims.challenge, claims.binding,
    ));
    let signed = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(b"recorded-result-secret")
        .expect("fixed recorded result secret");
    mac.update(signed.as_bytes());
    format!("{signed}.{}", codec.encode(mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recorded_imathas_question_backend_is_feature_gated_and_counts_safe_calls() {
        let question_backend = RecordedImathasQuestionBackend::from_mode(
            RecordedImathasQuestionBackendMode::Unavailable,
        );
        assert_eq!(question_backend.snapshot_calls(), 0);
        let source = question_model::DraftQuestionBackendLocator::Imathas {
            binding: question_model::DraftImathasQuestionBackendBinding::new(
                question_model::ImathasDeploymentReference::new("recorded-imathas")
                    .expect("deployment"),
                question_model::ImathasItemReference::new("item-17").expect("item"),
            ),
        };
        let locator = ImathasQuestionLocation::from_draft_backend_locator(&source).unwrap();
        assert_eq!(
            question_backend.snapshot(&locator).await,
            Err(ImathasQuestionBackendFailure::Unavailable)
        );
        assert_eq!(question_backend.snapshot_calls(), 1);
        assert!(!format!("{question_backend:?}").contains("answer"));
    }
}
