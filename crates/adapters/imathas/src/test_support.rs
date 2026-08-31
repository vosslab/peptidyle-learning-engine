//! Feature-gated recorded iMathAS provider for local server integration tests.
//!
//! This module is deliberately absent from the default production dependency
//! closure. It has no network client and never exposes a way to construct a
//! `VerifiedProviderGrade` from test or browser data.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use base64::Engine as _;
use hmac::{Hmac, KeyInit, Mac};
use question_model::AttemptResult;
use question_model::envelope::ContentBlock;
use sha2::Sha256;

use crate::broker_provider::{
    ContractedScoredEmbedConfig, ContractedScoredEmbedProvider, ContractedSnapshot,
    ProtectedLaunchRequest, ProviderLaunchHandle, ProxyRequest, ProxyResponse,
    ResultTransportRequest, ScoredEmbedTransport, ScoredEmbedTransportFailure,
    SnapshotTransportRequest,
};
use crate::scored_embed::ScoredEmbedProfileConfig;
use crate::{
    DraftLocator, ImathasProvider, ProviderFailure, ProviderGradeRequest, ProviderRenderRequest,
    SafeProviderRender, SupportedProfile, VerifiedProviderGrade, sealed,
};

/// Safe, deterministic behavior selected by a local server test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordedProviderMode {
    /// Return a deterministic correct grade after exact request binding.
    Verified,
    /// Inject a provider outage without fabricating student correctness.
    Unavailable,
    /// Inject a bounded provider timeout.
    Timeout,
    /// Inject malformed upstream material.
    InvalidResponse,
}

/// Factory for the feature-gated provider. The factory cannot accept arbitrary
/// answer keys, tokens, URLs, source text, or grade payloads.
#[derive(Debug, Clone, Copy)]
pub struct RecordedImathasProviderFactory {
    mode: RecordedProviderMode,
}

impl RecordedImathasProviderFactory {
    /// Selects one deterministic local test mode.
    pub fn new(mode: RecordedProviderMode) -> Self {
        Self { mode }
    }

    /// Builds a provider with independent deterministic call counters.
    pub fn build(self) -> RecordedImathasProvider {
        RecordedImathasProvider {
            mode: self.mode,
            snapshot_calls: Arc::new(AtomicUsize::new(0)),
            render_calls: Arc::new(AtomicUsize::new(0)),
            grade_calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

/// Test-only provider with safe canned source, render, and verifier behavior.
#[derive(Clone)]
pub struct RecordedImathasProvider {
    mode: RecordedProviderMode,
    snapshot_calls: Arc<AtomicUsize>,
    render_calls: Arc<AtomicUsize>,
    grade_calls: Arc<AtomicUsize>,
}

impl std::fmt::Debug for RecordedImathasProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecordedImathasProvider")
            .field("mode", &self.mode)
            .field("snapshot_calls", &self.snapshot_calls())
            .field("render_calls", &self.render_calls())
            .field("grade_calls", &self.grade_calls())
            .finish()
    }
}

impl RecordedImathasProvider {
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

impl sealed::ProviderSealed for RecordedImathasProvider {}

#[async_trait]
impl ImathasProvider for RecordedImathasProvider {
    async fn snapshot(
        &self,
        _locator: &DraftLocator,
    ) -> Result<(Vec<u8>, SupportedProfile), ProviderFailure> {
        self.snapshot_calls.fetch_add(1, Ordering::SeqCst);
        mode_result(self.mode)?;
        Ok((
            br#"{"recorded":true}"#.to_vec(),
            SupportedProfile::new("recorded-v1", true, true, true)
                .expect("recorded test profile is valid"),
        ))
    }

    async fn render(
        &self,
        _request: ProviderRenderRequest<'_>,
    ) -> Result<SafeProviderRender, ProviderFailure> {
        self.render_calls.fetch_add(1, Ordering::SeqCst);
        mode_result(self.mode)?;
        Ok(SafeProviderRender {
            title: "Recorded iMathAS question".into(),
            prompt: vec![ContentBlock::Text {
                markdown: "Complete the recorded external activity.".into(),
            }],
        })
    }

    async fn verify_grade(
        &self,
        request: ProviderGradeRequest<'_>,
    ) -> Result<VerifiedProviderGrade, ProviderFailure> {
        self.grade_calls.fetch_add(1, Ordering::SeqCst);
        mode_result(self.mode)?;
        Ok(VerifiedProviderGrade::from_scored_embed(
            AttemptResult {
                correct: true,
                points_earned: 1.0,
                points_possible: 1.0,
            },
            crate::GradeBinding {
                attempt: request.attempt(),
                question_version: request.question_version().clone(),
                seed: request.seed(),
            },
            request.correlation(),
        ))
    }
}

fn mode_result(mode: RecordedProviderMode) -> Result<(), ProviderFailure> {
    match mode {
        RecordedProviderMode::Verified => Ok(()),
        RecordedProviderMode::Unavailable => Err(ProviderFailure::Unavailable),
        RecordedProviderMode::Timeout => Err(ProviderFailure::Timeout),
        RecordedProviderMode::InvalidResponse => Err(ProviderFailure::InvalidResponse),
    }
}

/// Fixed activity behavior for the recorded contracted-provider transport.
/// It deliberately has no grade-result mode: route tests may exercise launch
/// and proxy isolation but cannot manufacture a provider verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordedContractedTransportMode {
    /// Launch normally, but do not expose a grade result. This preserves the
    /// route-only fixture used where a verdict would be misleading.
    Available,
    /// Return one deterministic signed, server-verified result after launch.
    /// This is only available behind this crate's `test-support` feature.
    Verified,
    /// Launch succeeds but server-to-server result retrieval is unavailable.
    /// It proves an upstream failure never manufactures a student grade.
    ResultUnavailable,
    Unavailable,
}

/// Default-off transport factory for server route tests. It accepts neither a
/// URL nor credentials and exposes only the fixed immutable snapshot and
/// activity document used by the contracted scored-embed seam.
#[derive(Debug, Clone, Copy)]
pub struct RecordedContractedTransportFactory {
    mode: RecordedContractedTransportMode,
}

impl RecordedContractedTransportFactory {
    pub fn new(mode: RecordedContractedTransportMode) -> Self {
        Self { mode }
    }

    pub fn build(self) -> RecordedContractedTransport {
        RecordedContractedTransport {
            mode: self.mode,
            proxy_calls: Arc::new(AtomicUsize::new(0)),
            result_calls: Arc::new(AtomicUsize::new(0)),
            launch_claims: Arc::new(Mutex::new(None)),
        }
    }

    /// Builds only the bounded, explicitly contracted server provider used by
    /// local route tests. The signing and verification keys are fixed test
    /// constants and never leave this adapter-owned fixture.
    pub fn contracted_provider(self) -> ContractedScoredEmbedProvider<RecordedContractedTransport> {
        self.contracted_provider_with_transport().0
    }

    /// Builds the bounded provider together with a cloned, counter-only
    /// transport handle for server acceptance tests. The handle exposes no
    /// provider endpoint, launch session, signed result, or student input.
    pub fn contracted_provider_with_transport(
        self,
    ) -> (
        ContractedScoredEmbedProvider<RecordedContractedTransport>,
        RecordedContractedTransport,
    ) {
        let transport = self.build();
        let provider = ContractedScoredEmbedProvider::new(
            ContractedScoredEmbedConfig::new(
                ScoredEmbedProfileConfig::contracted_self_hosted("self-hosted-imathas", true, true)
                    .expect("recorded contracted profile"),
                b"recorded-launch-secret",
                b"recorded-result-secret",
                30_000,
            )
            .expect("recorded contracted config"),
            transport.clone(),
        );
        (provider, transport)
    }
}

/// Recorded server-only transport. No public method accepts a score, answer,
/// provider result token, URL, JWT, source digest, or launch handle.
#[derive(Clone)]
pub struct RecordedContractedTransport {
    mode: RecordedContractedTransportMode,
    proxy_calls: Arc<AtomicUsize>,
    result_calls: Arc<AtomicUsize>,
    launch_claims: Arc<Mutex<Option<RecordedLaunchClaims>>>,
}

#[derive(Clone)]
struct RecordedLaunchClaims {
    nonce: String,
    binding: String,
}

impl RecordedContractedTransport {
    pub fn proxy_calls(&self) -> usize {
        self.proxy_calls.load(Ordering::SeqCst)
    }

    /// Number of server-only result retrievals. It is intentionally separate
    /// from launch/proxy activity so replay tests can prove no second grade.
    pub fn result_calls(&self) -> usize {
        self.result_calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ScoredEmbedTransport for RecordedContractedTransport {
    async fn fetch_snapshot(
        &self,
        _request: SnapshotTransportRequest<'_>,
    ) -> Result<ContractedSnapshot, ScoredEmbedTransportFailure> {
        match self.mode {
            RecordedContractedTransportMode::Available
            | RecordedContractedTransportMode::Verified
            | RecordedContractedTransportMode::ResultUnavailable => {
                ContractedSnapshot::from_protected_bytes(br#"{"recorded":true}"#.to_vec())
            }
            RecordedContractedTransportMode::Unavailable => {
                Err(ScoredEmbedTransportFailure::Unavailable)
            }
        }
    }

    async fn render_safe(
        &self,
        _request: crate::broker_provider::RenderTransportRequest<'_>,
    ) -> Result<SafeProviderRender, ScoredEmbedTransportFailure> {
        Ok(SafeProviderRender {
            title: "Recorded contracted iMathAS question".into(),
            prompt: vec![ContentBlock::Text {
                markdown: "Complete the protected activity.".into(),
            }],
        })
    }

    async fn start_protected_launch(
        &self,
        request: ProtectedLaunchRequest,
    ) -> Result<ProviderLaunchHandle, ScoredEmbedTransportFailure> {
        match self.mode {
            RecordedContractedTransportMode::Available
            | RecordedContractedTransportMode::Verified
            | RecordedContractedTransportMode::ResultUnavailable => {
                let claims = recorded_launch_claims(request.signed_launch_jwt())?;
                *self.launch_claims.lock().expect("recorded launch claims") = Some(claims);
                ProviderLaunchHandle::from_server_handle("recorded-proxy-session")
            }
            RecordedContractedTransportMode::Unavailable => {
                Err(ScoredEmbedTransportFailure::Unavailable)
            }
        }
    }

    async fn fetch_signed_grade_get(
        &self,
        _request: ResultTransportRequest<'_>,
    ) -> Result<Vec<u8>, ScoredEmbedTransportFailure> {
        self.result_calls.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            RecordedContractedTransportMode::Verified => {
                let claims = self
                    .launch_claims
                    .lock()
                    .expect("recorded launch claims")
                    .clone()
                    .ok_or(ScoredEmbedTransportFailure::InvalidResponse)?;
                Ok(recorded_result_token(&claims).into_bytes())
            }
            // Route-only and outage fixtures must never manufacture a grade.
            RecordedContractedTransportMode::Available => {
                Err(ScoredEmbedTransportFailure::Unsupported)
            }
            RecordedContractedTransportMode::ResultUnavailable => {
                Err(ScoredEmbedTransportFailure::Unavailable)
            }
            RecordedContractedTransportMode::Unavailable => {
                Err(ScoredEmbedTransportFailure::Unavailable)
            }
        }
    }

    async fn proxy_activity(
        &self,
        _request: ProxyRequest<'_>,
    ) -> Result<ProxyResponse, ScoredEmbedTransportFailure> {
        self.proxy_calls.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            RecordedContractedTransportMode::Available
            | RecordedContractedTransportMode::Verified
            | RecordedContractedTransportMode::ResultUnavailable => ProxyResponse::protected_html(
                b"<!doctype html><title>Recorded protected activity</title>".to_vec(),
            ),
            RecordedContractedTransportMode::Unavailable => {
                Err(ScoredEmbedTransportFailure::Unavailable)
            }
        }
    }
}

fn recorded_launch_claims(
    signed_launch_jwt: &str,
) -> Result<RecordedLaunchClaims, ScoredEmbedTransportFailure> {
    let payload = signed_launch_jwt
        .split('.')
        .nth(1)
        .ok_or(ScoredEmbedTransportFailure::InvalidResponse)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| ScoredEmbedTransportFailure::InvalidResponse)?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| ScoredEmbedTransportFailure::InvalidResponse)?;
    let nonce = value
        .get("ple_nonce")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 256)
        .ok_or(ScoredEmbedTransportFailure::InvalidResponse)?;
    let binding = value
        .get("ple_binding")
        .and_then(serde_json::Value::as_str)
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or(ScoredEmbedTransportFailure::InvalidResponse)?;
    Ok(RecordedLaunchClaims {
        nonce: nonce.to_owned(),
        binding: binding.to_owned(),
    })
}

fn recorded_result_token(claims: &RecordedLaunchClaims) -> String {
    let codec = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = codec.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = codec.encode(format!(
        r#"{{"id":17,"score":1.0,"ple_nonce":"{}","ple_binding":"{}"}}"#,
        claims.nonce, claims.binding
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
    async fn recorded_provider_is_feature_gated_and_counts_safe_calls() {
        let provider =
            RecordedImathasProviderFactory::new(RecordedProviderMode::Unavailable).build();
        assert_eq!(provider.snapshot_calls(), 0);
        let source = question_model::DraftQuestionSource::Imathas {
            provider: "recorded-provider".into(),
            item_ref: "item-17".into(),
        };
        let locator = DraftLocator::from_draft(&source).unwrap();
        assert_eq!(
            provider.snapshot(&locator).await,
            Err(ProviderFailure::Unavailable)
        );
        assert_eq!(provider.snapshot_calls(), 1);
        assert!(!format!("{provider:?}").contains("answer"));
    }
}
