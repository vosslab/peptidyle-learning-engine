use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use objects::memory::MemoryObjectStore;
use question_model::assignment_activity_rules::{QuestionAttemptLimit, QuestionAttemptTimeLimit};
use question_model::generation::RandomizationDefinition;
use question_model::taxonomy::License;
use question_model::{
    DraftQuestionSource, GradingDefinition, QuestionFormat, QuestionMetadata, QuestionType,
    WorkspaceId,
};

use super::*;

#[derive(Clone)]
struct RecordedProvider {
    renders: Arc<AtomicUsize>,
    grades: Arc<AtomicUsize>,
    outage: bool,
    mismatch: Option<Mismatch>,
}

#[derive(Clone, Copy)]
enum Mismatch {
    Attempt,
    Problem,
    Version,
    Seed,
    Correlation,
}

impl sealed::ProviderSealed for RecordedProvider {}

#[async_trait]
impl ImathasProvider for RecordedProvider {
    async fn snapshot(
        &self,
        locator: &DraftLocator,
    ) -> Result<(Vec<u8>, SupportedProfile), ProviderFailure> {
        assert_eq!(locator.provider(), "recorded-provider");
        assert_eq!(locator.item_ref(), "item-17");
        Ok((b"{\"recorded\":true}".to_vec(), profile()))
    }

    async fn render(
        &self,
        request: ProviderRenderRequest<'_>,
    ) -> Result<SafeProviderRender, ProviderFailure> {
        self.renders.fetch_add(1, Ordering::SeqCst);
        tokio::task::yield_now().await;
        if self.outage {
            return Err(ProviderFailure::Unavailable);
        }
        assert_eq!(request.snapshot, b"{\"recorded\":true}");
        assert_eq!(request.profile, "recorded-v1");
        Ok(SafeProviderRender {
            title: "Recorded external question".into(),
            prompt: vec![ContentBlock::Text {
                markdown: "Complete this iMathAS activity.".into(),
            }],
        })
    }

    async fn verify_grade(
        &self,
        request: ProviderGradeRequest<'_>,
    ) -> Result<VerifiedProviderGrade, ProviderFailure> {
        self.grades.fetch_add(1, Ordering::SeqCst);
        if self.outage {
            return Err(ProviderFailure::Timeout);
        }
        let mut verdict = VerifiedProviderGrade::verified(
            GradingResult {
                correct: true,
                points_earned: 1.0,
                points_possible: 1.0,
            },
            request.attempt(),
            request.question_version().clone(),
            request.seed(),
            request.correlation(),
        );
        match self.mismatch {
            Some(Mismatch::Attempt) => {
                verdict.attempt = QuestionAttemptId::from_uuid(Uuid::from_u128(99))
            }
            Some(Mismatch::Problem) => {
                verdict.question_version = QuestionVersionReference {
                    question_id: QuestionId::from_canonical_parts("BCDEFG", 'H')
                        .expect("Question ID"),
                    version_number: verdict.question_version.version_number,
                }
            }
            Some(Mismatch::Version) => {
                verdict.question_version.version_number =
                    QuestionVersionNumber::new(99).expect("positive version")
            }
            Some(Mismatch::Seed) => verdict.seed = Seed::new(99),
            Some(Mismatch::Correlation) => verdict.correlation = "wrong-server-correlation".into(),
            None => {}
        }
        Ok(verdict)
    }
}

fn profile() -> SupportedProfile {
    SupportedProfile::new("recorded-v1", true, true, true).unwrap()
}

fn provider() -> RecordedProvider {
    RecordedProvider {
        renders: Arc::new(AtomicUsize::new(0)),
        grades: Arc::new(AtomicUsize::new(0)),
        outage: false,
        mismatch: None,
    }
}

fn question(snapshot: ObjectId, digest: String) -> QuestionDefinition {
    QuestionDefinition {
        question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
        version_number: QuestionVersionNumber::new(2).expect("positive version"),
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(3)),
        source: QuestionSource::Imathas {
            provider: "recorded-provider".into(),
            item_ref: "item-17".into(),
            snapshot,
            snapshot_sha256: digest,
            integration_profile: "recorded-v1".into(),
        },
        question_format: QuestionFormat::Imathas,
        prompt: Vec::new(),
        response: question_model::QuestionResponseFormat::ExternalTool {},
        question_type: QuestionType::Numeric,
        question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
        question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Recorded iMathAS question".into(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBySa,
            language: "en-US".into(),
        },
    }
}

async fn stored_source(
    store: &MemoryObjectStore,
) -> (QuestionDefinition, ImathasSource, SourceObjectReference) {
    let snapshot = ObjectId::from_uuid(Uuid::from_u128(4));
    let digest = hex(Sha256::digest(b"{\"recorded\":true}").as_slice());
    let question = question(snapshot, digest);
    let object = store
        .put(PutObject {
            key: ObjectKey::QuestionSource {
                question_version: QuestionVersionReference {
                    question_id: question.question_id.clone(),
                    version_number: question.version_number,
                },
                object: snapshot,
            },
            bytes: b"{\"recorded\":true}".to_vec(),
            media_type: "application/json".into(),
            license: "CC-BY-SA-4.0".into(),
            provenance: "recorded redacted iMathAS fixture".into(),
            created_at: ActivityTimestamp::from_unix_millis(1),
        })
        .await
        .unwrap();
    let artifact = SourceObjectReference {
        object: snapshot,
        sha256: object.sha256.to_string(),
    };
    let source = ImathasSource {
        question_version: QuestionVersionReference {
            question_id: question.question_id.clone(),
            version_number: question.version_number,
        },
        artifact: artifact.clone(),
        provider: "recorded-provider".into(),
        item_ref: "item-17".into(),
        profile: "recorded-v1".into(),
        bytes: b"{\"recorded\":true}".to_vec(),
    };
    (question, source, artifact)
}

#[tokio::test]
async fn draft_snapshot_is_unversioned_and_publication_handoff_is_digest_pinned() {
    let provider = provider();
    let adapter = ImathasAdapter::new(MemoryObjectStore::default(), provider, [profile()]);
    let prepared = adapter
        .prepare_snapshot(&DraftQuestionSource::Imathas {
            provider: "recorded-provider".into(),
            item_ref: "item-17".into(),
        })
        .await
        .unwrap();
    assert_eq!(prepared.bytes(), b"{\"recorded\":true}");
    assert_eq!(prepared.profile().name(), "recorded-v1");
    assert!(!format!("{prepared:?}").contains("recorded\\\":true"));
    assert!(
        DraftLocator::from_draft(&DraftQuestionSource::Imathas {
            provider: "https://untrusted.example".into(),
            item_ref: "item-17".into(),
        })
        .is_err()
    );
    for item_ref in [
        "https://provider.example/item",
        "17?token=secret",
        "17#fragment",
        "item with-space",
        "item\n17",
        &"a".repeat(129),
    ] {
        assert!(
            DraftLocator::from_draft(&DraftQuestionSource::Imathas {
                provider: "recorded-provider".into(),
                item_ref: item_ref.into(),
            })
            .is_err()
        );
    }
    assert_eq!(
        format!(
            "{:?}",
            DraftLocator::from_draft(&DraftQuestionSource::Imathas {
                provider: "recorded-provider".into(),
                item_ref: "item-17".into(),
            })
            .unwrap()
        ),
        "DraftLocator(REDACTED)"
    );
}

#[tokio::test]
async fn immutable_snapshot_cache_and_verified_grade_are_bound_to_exact_attempt() {
    let store = MemoryObjectStore::default();
    let recorded = provider();
    let renders = recorded.renders.clone();
    let adapter = ImathasAdapter::new(store.clone(), recorded, [profile()]);
    let (question, source, _) = stored_source(&store).await;
    let first = adapter
        .issue(
            &question,
            Seed::new(17),
            &source,
            ActivityTimestamp::from_unix_millis(2),
        )
        .await
        .unwrap();
    let second = adapter
        .issue(
            &question,
            Seed::new(17),
            &source,
            ActivityTimestamp::from_unix_millis(3),
        )
        .await
        .unwrap();
    assert!(!first.cache_hit);
    assert!(second.cache_hit);
    assert_eq!(renders.load(Ordering::SeqCst), 1);
    assert_eq!(first.envelope.title, "Recorded external question");
    assert_eq!(second.envelope.title, first.envelope.title);
    assert!(matches!(
        first.envelope.response,
        question_model::QuestionResponseFormat::ExternalTool {}
    ));
    let serialized = serde_json::to_string(&first.envelope).unwrap();
    for forbidden in ["token", "launch", "score", "correct", "recorded\\\":true"] {
        assert!(!serialized.contains(forbidden));
    }
    let result = adapter
        .grade(
            &question,
            &source,
            QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
            Seed::new(17),
            &correlation(&question, Seed::new(17)),
        )
        .await
        .unwrap();
    assert!(result.result().correct);
}

#[tokio::test]
async fn historical_invalid_metadata_title_is_refused_before_provider_or_cache() {
    let store = MemoryObjectStore::default();
    let recorded = provider();
    let renders = recorded.renders.clone();
    let adapter = ImathasAdapter::new(store.clone(), recorded, [profile()]);
    let (mut question, source, _) = stored_source(&store).await;
    question.metadata.title = " \n ".into();
    assert!(matches!(
        adapter
            .issue(
                &question,
                Seed::new(17),
                &source,
                ActivityTimestamp::from_unix_millis(2),
            )
            .await,
        Err(ImathasAdapterError::InvalidTitle(_))
    ));
    assert_eq!(renders.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn snapshot_mutation_wrong_binding_and_outage_refuse_without_fabricating_incorrectness() {
    let store = MemoryObjectStore::default();
    let (question, source, _) = stored_source(&store).await;
    let mut changed_source = question.clone();
    if let QuestionSource::Imathas {
        snapshot_sha256, ..
    } = &mut changed_source.source
    {
        *snapshot_sha256 = "00".repeat(32);
    }
    assert_eq!(
        verify_binding(&changed_source, &source),
        Err(ImathasAdapterError::SourceDoesNotMatchQuestion)
    );
    let wrong = ImathasAdapter::new(
        store.clone(),
        RecordedProvider {
            mismatch: Some(Mismatch::Version),
            ..provider()
        },
        [profile()],
    );
    let error = wrong
        .grade(
            &question,
            &source,
            QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
            Seed::new(17),
            &correlation(&question, Seed::new(17)),
        )
        .await
        .unwrap_err();
    assert_eq!(error, ImathasAdapterError::VerificationRefused);
    let outage = ImathasAdapter::new(
        store,
        RecordedProvider {
            outage: true,
            ..provider()
        },
        [profile()],
    );
    assert!(matches!(
        outage
            .issue(
                &question,
                Seed::new(18),
                &source,
                ActivityTimestamp::from_unix_millis(2)
            )
            .await,
        Err(ImathasAdapterError::Provider(ProviderFailure::Unavailable))
    ));
}

#[tokio::test]
async fn every_verified_grade_binding_dimension_and_restored_handle_is_checked() {
    let store = MemoryObjectStore::default();
    let (question, source, _) = stored_source(&store).await;
    for mismatch in [
        Mismatch::Attempt,
        Mismatch::Problem,
        Mismatch::Version,
        Mismatch::Seed,
        Mismatch::Correlation,
    ] {
        let adapter = ImathasAdapter::new(
            store.clone(),
            RecordedProvider {
                mismatch: Some(mismatch),
                ..provider()
            },
            [profile()],
        );
        assert_eq!(
            adapter
                .grade(
                    &question,
                    &source,
                    QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                    Seed::new(17),
                    &correlation(&question, Seed::new(17)),
                )
                .await
                .unwrap_err(),
            ImathasAdapterError::VerificationRefused
        );
    }
    let issuer = CorrelationIssuer::from_server_secret([8; 32]);
    let binding = GradeBinding {
        attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
        question_version: QuestionVersionReference {
            question_id: question.question_id.clone(),
            version_number: question.version_number,
        },
        seed: Seed::new(17),
    };
    let persisted = issuer.begin(binding.clone());
    let stored_value = persisted.to_storage_value();
    let after_restart = PersistedCorrelation::from_storage_value(&stored_value).unwrap();
    let restored = issuer.restore(binding.clone(), &after_restart).unwrap();
    let adapter = ImathasAdapter::new(store.clone(), provider(), [profile()]);
    assert!(
        adapter
            .grade(&question, &source, binding.attempt, binding.seed, &restored,)
            .await
            .unwrap()
            .result()
            .correct
    );
    let mut altered = stored_value.clone().into_bytes();
    altered[0] = if altered[0] == b'f' { b'e' } else { b'f' };
    let altered = String::from_utf8(altered).unwrap();
    let altered = PersistedCorrelation::from_storage_value(&altered).unwrap();
    assert!(issuer.restore(binding.clone(), &altered).is_err());
    let wrong_issuer = CorrelationIssuer::from_server_secret([9; 32]);
    assert!(
        wrong_issuer
            .restore(binding.clone(), &after_restart)
            .is_err()
    );
    assert!(
        PersistedCorrelation::from_storage_value(&stored_value[..stored_value.len() - 1]).is_err()
    );
    assert!(PersistedCorrelation::from_storage_value(&"a".repeat(1024)).is_err());
    assert!(
        issuer
            .restore(
                GradeBinding {
                    seed: Seed::new(18),
                    ..binding
                },
                &persisted
            )
            .is_err()
    );
}

#[tokio::test]
async fn malformed_stored_cache_and_grade_outage_remain_local_and_redacted() {
    let store = MemoryObjectStore::default();
    let (question, source, _) = stored_source(&store).await;
    let key = render_key(
        &QuestionVersionReference {
            question_id: question.question_id.clone(),
            version_number: question.version_number,
        },
        Seed::new(31),
    );
    store
        .put(PutObject {
            key,
            bytes: b"{malformed".to_vec(),
            media_type: "application/json".into(),
            license: "test".into(),
            provenance: "test".into(),
            created_at: ActivityTimestamp::from_unix_millis(1),
        })
        .await
        .unwrap();
    let adapter = ImathasAdapter::new(store.clone(), provider(), [profile()]);
    assert_eq!(
        adapter
            .issue(
                &question,
                Seed::new(31),
                &source,
                ActivityTimestamp::from_unix_millis(2)
            )
            .await
            .unwrap_err(),
        ImathasAdapterError::InvalidCache
    );
    let outage = ImathasAdapter::new(
        store,
        RecordedProvider {
            outage: true,
            ..provider()
        },
        [profile()],
    );
    assert!(matches!(
        outage
            .grade(
                &question,
                &source,
                QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
                Seed::new(17),
                &correlation(&question, Seed::new(17)),
            )
            .await,
        Err(ImathasAdapterError::Provider(ProviderFailure::Timeout))
    ));
    let text = ImathasAdapterError::Provider(ProviderFailure::Unavailable).to_string();
    assert!(!text.contains("token"));
    assert!(text.len() < 100);
}

#[tokio::test]
async fn concurrent_replicas_reuse_the_winning_immutable_render() {
    let store = MemoryObjectStore::default();
    let recorded = provider();
    let adapter = ImathasAdapter::new(store.clone(), recorded, [profile()]);
    let (question, source, _) = stored_source(&store).await;
    let first = adapter.issue(
        &question,
        Seed::new(41),
        &source,
        ActivityTimestamp::from_unix_millis(2),
    );
    let second = adapter.issue(
        &question,
        Seed::new(41),
        &source,
        ActivityTimestamp::from_unix_millis(2),
    );
    let (first, second) = tokio::join!(first, second);
    let first = first.unwrap();
    let second = second.unwrap();
    assert!(first.cache_hit || second.cache_hit);
    assert_eq!(first.envelope, second.envelope);
}

fn correlation(question: &QuestionDefinition, seed: Seed) -> ServerCorrelation {
    let issuer = CorrelationIssuer::from_server_secret([7; 32]);
    let binding = GradeBinding {
        attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
        question_version: QuestionVersionReference {
            question_id: question.question_id.clone(),
            version_number: question.version_number,
        },
        seed,
    };
    let persisted = issuer.begin(binding.clone());
    issuer.restore(binding, &persisted).unwrap()
}
