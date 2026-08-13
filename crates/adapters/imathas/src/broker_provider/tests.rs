use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use hmac::{Hmac, KeyInit, Mac};
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::License;
use question_model::{
    GradingDefinition, ObjectId, ProblemId, QuestionMetadata, SourceArtifact, VersionId,
    WorkspaceId,
};
use uuid::Uuid;

use super::*;
use crate::CorrelationIssuer;

#[derive(Clone)]
struct RecordedTransport {
    snapshot: Arc<Mutex<Result<Vec<u8>, ScoredEmbedTransportFailure>>>,
    result: Arc<Mutex<Result<Vec<u8>, ScoredEmbedTransportFailure>>>,
    launches: Arc<Mutex<Vec<(String, String, String)>>>,
    result_calls: Arc<AtomicUsize>,
}

impl RecordedTransport {
    fn stable() -> Self {
        Self {
            snapshot: Arc::new(Mutex::new(Ok(br#"{"recorded":true}"#.to_vec()))),
            result: Arc::new(Mutex::new(Ok(Vec::new()))),
            launches: Arc::new(Mutex::new(Vec::new())),
            result_calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl ScoredEmbedTransport for RecordedTransport {
    async fn fetch_snapshot(
        &self,
        _request: SnapshotTransportRequest<'_>,
    ) -> Result<ContractedSnapshot, ScoredEmbedTransportFailure> {
        let bytes = self.snapshot.lock().unwrap().clone()?;
        ContractedSnapshot::from_protected_bytes(bytes)
    }
    async fn render_safe(
        &self,
        _request: RenderTransportRequest<'_>,
    ) -> Result<SafeProviderRender, ScoredEmbedTransportFailure> {
        Ok(SafeProviderRender {
            title: "Recorded broker question".into(),
            prompt: vec![ContentBlock::Text {
                markdown: "Use the protected activity.".into(),
            }],
        })
    }
    async fn start_protected_launch(
        &self,
        request: ProtectedLaunchRequest,
    ) -> Result<ProviderLaunchHandle, ScoredEmbedTransportFailure> {
        self.launches.lock().unwrap().push((
            request.item_ref().to_owned(),
            request.source_digest().to_owned(),
            request.signed_launch_jwt().to_owned(),
        ));
        ProviderLaunchHandle::from_server_handle("recorded-proxy-session")
    }
    async fn fetch_signed_grade_get(
        &self,
        request: ResultTransportRequest<'_>,
    ) -> Result<Vec<u8>, ScoredEmbedTransportFailure> {
        self.result_calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(request.handle().protected_value(), "recorded-proxy-session");
        assert!(format!("{:?}", request.correlation()).contains("REDACTED"));
        self.result.lock().unwrap().clone()
    }
    async fn proxy_activity(
        &self,
        request: ProxyRequest<'_>,
    ) -> Result<ProxyResponse, ScoredEmbedTransportFailure> {
        assert_eq!(request.handle().protected_value(), "recorded-proxy-session");
        assert!(matches!(
            request.method(),
            ProxyMethod::Get | ProxyMethod::Post
        ));
        ProxyResponse::protected_html(
            b"<!doctype html><title>Recorded protected activity</title>".to_vec(),
        )
    }
}

fn config() -> ContractedScoredEmbedConfig {
    ContractedScoredEmbedConfig::new(
        ScoredEmbedProfileConfig::contracted_self_hosted("institution-imathas", true, true)
            .unwrap(),
        b"launch-secret",
        b"result-secret",
        30_000,
    )
    .unwrap()
}

fn question_and_source() -> (QuestionDefinition, ImathasSource) {
    let problem = ProblemId::from_uuid(Uuid::from_u128(1));
    let version = VersionId::from_uuid(Uuid::from_u128(2));
    let bytes = br#"{"recorded":true}"#.to_vec();
    let digest = hex(Sha256::digest(&bytes).as_slice());
    let object = ObjectId::from_uuid(Uuid::from_u128(3));
    let source_artifact = SourceArtifact {
        object,
        sha256: digest.clone(),
    };
    let question = QuestionDefinition {
        problem,
        version,
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(4)),
        source: QuestionSource::Imathas {
            provider: "institution-imathas".into(),
            item_ref: "17".into(),
            snapshot: object,
            snapshot_sha256: digest,
            integration_profile: SCORED_EMBED_BROKER_PROFILE_ID.into(),
        },
        prompt: Vec::new(),
        response: question_model::ResponseDefinition::ExternalTool {},
        attempt_policy: AttemptPolicy {
            max_attempts: None,
            feedback: FeedbackDisclosure::ImmediateCorrectness,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Recorded broker question".into(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBySa,
            language: "en-US".into(),
        },
    };
    let source = ImathasSource {
        problem,
        version,
        artifact: source_artifact,
        provider: "institution-imathas".into(),
        item_ref: "17".into(),
        profile: SCORED_EMBED_BROKER_PROFILE_ID.into(),
        bytes,
    };
    (question, source)
}

fn correlation(question: &QuestionDefinition, seed: Seed) -> ServerCorrelation {
    let binding = GradeBinding {
        tenant: TenantId::from_uuid(Uuid::from_u128(5)),
        attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
        problem: question.problem,
        version: question.version,
        seed,
    };
    let issuer = CorrelationIssuer::from_server_secret([3; 32]);
    issuer.restore(binding, &issuer.begin(binding)).unwrap()
}

fn result_token(session: &ContractedLaunchSession, score: f64) -> Vec<u8> {
    let claims = session.ledger.signed_launch_claims();
    let header =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!(
        r#"{{"id":17,"score":{score},"ple_nonce":"{}","ple_binding":"{}"}}"#,
        claims.nonce(),
        claims.binding_digest(),
    ));
    let signed = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(b"result-secret").unwrap();
    mac.update(signed.as_bytes());
    format!(
        "{signed}.{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    )
    .into_bytes()
}

async fn launch(
    provider: &ContractedScoredEmbedProvider<RecordedTransport>,
    question: &QuestionDefinition,
    source: &ImathasSource,
    nonce: u8,
) -> ContractedLaunchSession {
    provider
        .begin_launch(
            question,
            source,
            TenantId::from_uuid(Uuid::from_u128(5)),
            QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
            Seed::new(10_001),
            correlation(question, Seed::new(10_001)),
            ScoredEmbedNonce::from_server_random([nonce; 32]).unwrap(),
            ActivityTimestamp::from_unix_millis(1_000),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn recorded_transport_launches_and_verifies_only_a_bound_result() {
    let transport = RecordedTransport::stable();
    let provider = ContractedScoredEmbedProvider::new(config(), transport.clone());
    let (question, source) = question_and_source();
    let mut session = launch(&provider, &question, &source, 7).await;
    *transport.result.lock().unwrap() = Ok(result_token(&session, 1.0));
    assert!(
        provider
            .retrieve_and_verify(&mut session, ActivityTimestamp::from_unix_millis(2_000))
            .await
            .unwrap()
            .result
            .correct
    );
    let launches = transport.launches.lock().unwrap();
    assert_eq!(launches.len(), 1);
    assert!(!launches[0].2.contains("result-secret"));
    assert!(!format!("{:?}", session).contains("eyJ"));
}

#[tokio::test]
async fn mutation_outage_timeout_oversize_and_cross_binding_refuse() {
    let (question, source) = question_and_source();
    let transport = RecordedTransport::stable();
    let provider = ContractedScoredEmbedProvider::new(config(), transport.clone());
    *transport.snapshot.lock().unwrap() = Ok(b"changed".to_vec());
    assert_eq!(
        provider
            .begin_launch(
                &question,
                &source,
                TenantId::from_uuid(Uuid::from_u128(5)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                Seed::new(10_001),
                correlation(&question, Seed::new(10_001)),
                ScoredEmbedNonce::from_server_random([7; 32]).unwrap(),
                ActivityTimestamp::from_unix_millis(1_000),
            )
            .await
            .unwrap_err(),
        ImathasAdapterError::SourceChecksumMismatch
    );
    *transport.snapshot.lock().unwrap() = Err(ScoredEmbedTransportFailure::Unavailable);
    assert!(matches!(
        provider
            .begin_launch(
                &question,
                &source,
                TenantId::from_uuid(Uuid::from_u128(5)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                Seed::new(10_001),
                correlation(&question, Seed::new(10_001)),
                ScoredEmbedNonce::from_server_random([7; 32]).unwrap(),
                ActivityTimestamp::from_unix_millis(1_000),
            )
            .await,
        Err(ImathasAdapterError::Provider(ProviderFailure::Unavailable))
    ));
    *transport.snapshot.lock().unwrap() = Ok(br#"{"recorded":true}"#.to_vec());
    let mut first = launch(&provider, &question, &source, 7).await;
    let second = launch(&provider, &question, &source, 8).await;
    *transport.result.lock().unwrap() = Ok(result_token(&second, 1.0));
    assert_eq!(
        provider
            .retrieve_and_verify(&mut first, ActivityTimestamp::from_unix_millis(2_000))
            .await,
        Err(ImathasAdapterError::VerificationRefused)
    );
    *transport.result.lock().unwrap() = Err(ScoredEmbedTransportFailure::Timeout);
    let mut third = launch(&provider, &question, &source, 9).await;
    assert!(matches!(
        provider
            .retrieve_and_verify(&mut third, ActivityTimestamp::from_unix_millis(2_000))
            .await,
        Err(ImathasAdapterError::Provider(ProviderFailure::Timeout))
    ));
    *transport.result.lock().unwrap() = Ok(vec![b'x'; MAX_RESULT_BYTES + 1]);
    let mut fourth = launch(&provider, &question, &source, 10).await;
    assert!(matches!(
        provider
            .retrieve_and_verify(&mut fourth, ActivityTimestamp::from_unix_millis(2_000))
            .await,
        Err(ImathasAdapterError::Provider(
            ProviderFailure::InvalidResponse
        ))
    ));
}

#[tokio::test]
async fn cross_provider_draft_and_published_sources_refuse_before_transport() {
    let transport = RecordedTransport::stable();
    let provider = ContractedScoredEmbedProvider::new(config(), transport.clone());
    let foreign_draft = question_model::DraftQuestionSource::Imathas {
        provider: "foreign-imathas".into(),
        item_ref: "17".into(),
    };
    let locator = DraftLocator::from_draft(&foreign_draft).unwrap();
    assert_eq!(
        provider.snapshot(&locator).await,
        Err(ProviderFailure::UnsupportedProfile)
    );
    assert!(transport.launches.lock().unwrap().is_empty());

    let (mut question, mut source) = question_and_source();
    source.provider = "foreign-imathas".into();
    if let QuestionSource::Imathas { provider, .. } = &mut question.source {
        *provider = "foreign-imathas".into();
    }
    assert!(matches!(
        provider
            .begin_launch(
                &question,
                &source,
                TenantId::from_uuid(Uuid::from_u128(5)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                Seed::new(10_001),
                correlation(&question, Seed::new(10_001)),
                ScoredEmbedNonce::from_server_random([7; 32]).unwrap(),
                ActivityTimestamp::from_unix_millis(1_000),
            )
            .await,
        Err(ImathasAdapterError::UnsupportedProfile)
    ));
    assert!(transport.launches.lock().unwrap().is_empty());
}

#[tokio::test]
async fn launch_session_storage_is_replica_safe_and_hostile_input_refuses() {
    let transport = RecordedTransport::stable();
    let provider = ContractedScoredEmbedProvider::new(config(), transport.clone());
    let (question, source) = question_and_source();
    let session = launch(&provider, &question, &source, 7).await;
    let codec = LaunchSessionCodec::from_server_secret([11; 32]).unwrap();
    let expected = ContractedLaunchExpectation::new(
        GradeBinding {
            tenant: TenantId::from_uuid(Uuid::from_u128(5)),
            attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
            problem: question.problem,
            version: question.version,
            seed: Seed::new(10_001),
        },
        "institution-imathas",
        source.artifact.sha256.clone(),
    )
    .unwrap();
    let persisted = codec.seal(&session).unwrap();
    let storage = persisted.to_storage_value();
    assert!(!format!("{persisted:?}").contains("17"));
    assert!(!storage.contains("eyJ"));
    let persisted = PersistedContractedLaunchSession::from_storage_value(&storage).unwrap();
    let mut restored = codec.restore(&persisted, &expected).unwrap();
    *transport.result.lock().unwrap() = Ok(result_token(&restored, 1.0));
    assert!(
        provider
            .retrieve_and_verify(&mut restored, ActivityTimestamp::from_unix_millis(2_000))
            .await
            .unwrap()
            .result
            .correct
    );

    let mut mutated = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&storage)
        .unwrap();
    mutated[20] ^= 1;
    let mutated = PersistedContractedLaunchSession::from_storage_value(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mutated),
    )
    .unwrap();
    assert!(codec.restore(&mutated, &expected).is_err());
    assert!(
        LaunchSessionCodec::from_server_secret([12; 32])
            .unwrap()
            .restore(&persisted, &expected)
            .is_err()
    );
    assert!(
        PersistedContractedLaunchSession::from_storage_value(&storage[..storage.len() - 1])
            .is_err()
    );
    assert!(
        PersistedContractedLaunchSession::from_storage_value(&(storage.clone() + "=")).is_err()
    );
    assert!(PersistedContractedLaunchSession::from_storage_value(&"a".repeat(8_193)).is_err());
    let wrong_version = ContractedLaunchExpectation::new(
        GradeBinding {
            version: VersionId::from_uuid(Uuid::from_u128(99)),
            ..expected.binding
        },
        "institution-imathas",
        source.artifact.sha256,
    )
    .unwrap();
    assert!(codec.restore(&persisted, &wrong_version).is_err());
}

#[tokio::test]
async fn restored_expired_or_consumed_sessions_do_not_fetch_provider_results() {
    let transport = RecordedTransport::stable();
    let provider = ContractedScoredEmbedProvider::new(config(), transport.clone());
    let (question, source) = question_and_source();
    let expected = ContractedLaunchExpectation::new(
        GradeBinding {
            tenant: TenantId::from_uuid(Uuid::from_u128(5)),
            attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
            problem: question.problem,
            version: question.version,
            seed: Seed::new(10_001),
        },
        "institution-imathas",
        source.artifact.sha256.clone(),
    )
    .unwrap();
    let codec = LaunchSessionCodec::from_server_secret([11; 32]).unwrap();

    let mut expired = launch(&provider, &question, &source, 7).await;
    let mut parts = expired.ledger.storage_parts();
    parts.expires_at = ActivityTimestamp::from_unix_millis(999);
    expired.ledger = ScoredEmbedLaunchLedger::from_storage_parts(parts).unwrap();
    let expired_blob = codec.seal(&expired).unwrap();
    let mut expired = codec.restore(&expired_blob, &expected).unwrap();
    let before = transport.result_calls.load(Ordering::SeqCst);
    let before_blob = codec.seal(&expired).unwrap().to_storage_value();
    assert_eq!(
        provider
            .retrieve_and_verify(&mut expired, ActivityTimestamp::from_unix_millis(1_000))
            .await,
        Err(ImathasAdapterError::InvalidCorrelation)
    );
    assert_eq!(transport.result_calls.load(Ordering::SeqCst), before);
    assert_eq!(
        codec.seal(&expired).unwrap().to_storage_value(),
        before_blob
    );

    let mut consumed = launch(&provider, &question, &source, 8).await;
    *transport.result.lock().unwrap() = Ok(result_token(&consumed, 1.0));
    provider
        .retrieve_and_verify(&mut consumed, ActivityTimestamp::from_unix_millis(2_000))
        .await
        .unwrap();
    let consumed_blob = codec.seal(&consumed).unwrap();
    let mut consumed = codec.restore(&consumed_blob, &expected).unwrap();
    let before = transport.result_calls.load(Ordering::SeqCst);
    let before_blob = codec.seal(&consumed).unwrap().to_storage_value();
    assert_eq!(
        provider
            .retrieve_and_verify(&mut consumed, ActivityTimestamp::from_unix_millis(2_000))
            .await,
        Err(ImathasAdapterError::InvalidCorrelation)
    );
    assert_eq!(transport.result_calls.load(Ordering::SeqCst), before);
    assert_eq!(
        codec.seal(&consumed).unwrap().to_storage_value(),
        before_blob
    );
}
