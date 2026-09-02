use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use objects::memory::MemoryObjectStore;
use question_model::assignment_activity_rules::{QuestionAttemptLimit, QuestionAttemptTimeLimit};
use question_model::classification::QuestionLicense;
use question_model::generation::QuestionVariationRule;
use question_model::{
    DraftQuestionBackendLocator, QuestionFormat, QuestionGradingRule, QuestionMetadata,
    QuestionRevision, QuestionType, WorkspaceId,
};

use super::*;

#[derive(Clone)]
struct RecordedImathasQuestionBackend {
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
    QuestionSeed,
    LaunchSessionAuthentication,
}

impl sealed::QuestionBackendSealed for RecordedImathasQuestionBackend {}

#[async_trait]
impl QuestionBackend for RecordedImathasQuestionBackend {
    async fn snapshot(
        &self,
        locator: &ImathasQuestionLocation,
    ) -> Result<(Vec<u8>, SupportedImathasProfile), ImathasQuestionBackendFailure> {
        assert_eq!(locator.deployment_reference().as_str(), "recorded-imathas");
        assert_eq!(locator.item_reference().as_str(), "item-17");
        Ok((b"{\"recorded\":true}".to_vec(), profile()))
    }

    async fn render(
        &self,
        request: ImathasRenderRequest<'_>,
    ) -> Result<SafeImathasQuestionRender, ImathasQuestionBackendFailure> {
        self.renders.fetch_add(1, Ordering::SeqCst);
        tokio::task::yield_now().await;
        if self.outage {
            return Err(ImathasQuestionBackendFailure::Unavailable);
        }
        assert_eq!(request.snapshot, b"{\"recorded\":true}");
        assert_eq!(request.profile, "recorded-v1");
        Ok(SafeImathasQuestionRender {
            title: "Recorded iMathAS question".into(),
            prompt: vec![QuestionContentBlock::Text {
                markdown: "Complete this iMathAS activity.".into(),
            }],
        })
    }

    async fn verify_result(
        &self,
        request: ImathasResultRequest<'_>,
    ) -> Result<VerifiedImathasResult, ImathasQuestionBackendFailure> {
        self.grades.fetch_add(1, Ordering::SeqCst);
        if self.outage {
            return Err(ImathasQuestionBackendFailure::Timeout);
        }
        let mut verdict = VerifiedImathasResult::verified(
            learning_data_access::ImathasResult::new(
                learning_data_access::ImathasNormalizedScore::try_from_f64(1.0)
                    .expect("recorded score is valid"),
            ),
            request.grading_context().clone(),
            request.launch_session_authentication(),
            verified_token_checksum(),
        );
        match self.mismatch {
            Some(Mismatch::Attempt) => {
                verdict.grading_context = learning_data_access::ImathasGradingContext::new(
                    QuestionAttemptId::from_uuid(Uuid::from_u128(99)),
                    verdict.grading_context.question_revision().clone(),
                    verdict.grading_context.question_seed(),
                )
            }
            Some(Mismatch::Problem) => {
                verdict.grading_context = learning_data_access::ImathasGradingContext::new(
                    verdict.grading_context.question_attempt(),
                    QuestionRevisionReference {
                        question_id: QuestionId::from_canonical_parts("BCDEFG", 'H')
                            .expect("Question ID"),
                        revision_number: verdict
                            .grading_context
                            .question_revision()
                            .revision_number,
                    },
                    verdict.grading_context.question_seed(),
                )
            }
            Some(Mismatch::Version) => {
                verdict.grading_context = learning_data_access::ImathasGradingContext::new(
                    verdict.grading_context.question_attempt(),
                    QuestionRevisionReference {
                        question_id: verdict
                            .grading_context
                            .question_revision()
                            .question_id
                            .clone(),
                        revision_number: QuestionRevisionNumber::new(99).expect("positive version"),
                    },
                    verdict.grading_context.question_seed(),
                )
            }
            Some(Mismatch::QuestionSeed) => {
                verdict.grading_context = learning_data_access::ImathasGradingContext::new(
                    verdict.grading_context.question_attempt(),
                    verdict.grading_context.question_revision().clone(),
                    QuestionSeed::new(99),
                )
            }
            Some(Mismatch::LaunchSessionAuthentication) => verdict.launch_session_authentication =
                learning_data_access::ImathasQuestionBackendSessionAuthentication::from_server_value(
                    format!("aa.{}", "c".repeat(64)),
                )
                .expect("authentication"),
            None => {}
        }
        Ok(verdict)
    }
}

fn verified_token_checksum() -> learning_data_access::ImathasResultTokenChecksum {
    let token = learning_data_access::ImathasResultToken::from_server_adapter_bytes(
        b"recorded iMathAS result".to_vec(),
    )
    .expect("bounded iMathAS result token");
    learning_data_access::ImathasResultTokenChecksum::from_verified_token(&token)
}

fn profile() -> SupportedImathasProfile {
    SupportedImathasProfile::new(
        ImathasProfile::new("recorded-v1").expect("recorded profile"),
        true,
        true,
        true,
    )
    .expect("recorded capabilities")
}

fn binding() -> ImathasQuestionBackendBinding {
    ImathasQuestionBackendBinding::new(
        ImathasDeploymentReference::new("recorded-imathas").expect("recorded deployment"),
        ImathasItemReference::new("item-17").expect("recorded item"),
        ImathasProfile::new("recorded-v1").expect("recorded profile"),
    )
}

fn draft_binding() -> DraftImathasQuestionBackendBinding {
    DraftImathasQuestionBackendBinding::new(
        ImathasDeploymentReference::new("recorded-imathas").expect("recorded deployment"),
        ImathasItemReference::new("item-17").expect("recorded item"),
    )
}

fn question_backend() -> RecordedImathasQuestionBackend {
    RecordedImathasQuestionBackend {
        renders: Arc::new(AtomicUsize::new(0)),
        grades: Arc::new(AtomicUsize::new(0)),
        outage: false,
        mismatch: None,
    }
}

fn question() -> QuestionRevision {
    QuestionRevision {
        question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
        revision_number: QuestionRevisionNumber::new(2).expect("positive version"),
        workspace: WorkspaceId::from_uuid(Uuid::from_u128(3)),
        backend_locator: QuestionBackendLocator::Imathas { binding: binding() },
        question_format: QuestionFormat::Imathas,
        prompt: Vec::new(),
        response: question_model::QuestionResponseFormat::ImathasQuestionBackend {},
        question_type: QuestionType::Numeric,
        question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
        question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
        question_variation_rule: QuestionVariationRule::Static,
        grading: QuestionGradingRule::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Recorded iMathAS question".into(),
            question_description: "Instructor-facing recorded iMathAS fixture summary.".into(),
            tags: Vec::new(),
            classifications: Vec::new(),
            question_license: Some(QuestionLicense::CcBySa4_0),
            question_citation: None,
            language: "en-US".into(),
        },
    }
}

async fn stored_source(
    store: &MemoryObjectStore,
) -> (
    QuestionRevision,
    ResolvedImathasQuestionSource,
    SourceObjectReference,
) {
    let snapshot = ObjectId::from_uuid(Uuid::from_u128(4));
    let question = question();
    let object = store
        .put(PutObject {
            address: ObjectAddress::QuestionSource {
                question_revision: QuestionRevisionReference {
                    question_id: question.question_id.clone(),
                    revision_number: question.revision_number,
                },
                object: snapshot,
            },
            bytes: b"{\"recorded\":true}".to_vec(),
            media_type: "application/json".into(),
            created_at: Timestamp::from_unix_millis(1),
        })
        .await
        .unwrap();
    let artifact = SourceObjectReference { object: snapshot };
    let source = ResolvedImathasQuestionSource::resolve(
        store,
        &question,
        artifact.clone(),
        SourceObjectChecksum::parse(object.sha256.to_string())
            .expect("stored checksum is canonical"),
    )
    .await
    .expect("stored source should resolve");
    (question, source, artifact)
}

#[tokio::test]
async fn draft_snapshot_is_unversioned_and_publication_handoff_is_digest_pinned() {
    let question_backend = question_backend();
    let adapter = ImathasAdapter::new(MemoryObjectStore::default(), question_backend, [profile()]);
    let prepared = adapter
        .prepare_snapshot(&DraftQuestionBackendLocator::Imathas {
            binding: draft_binding(),
        })
        .await
        .unwrap();
    assert_eq!(prepared.bytes(), b"{\"recorded\":true}");
    assert_eq!(prepared.profile().profile().as_str(), "recorded-v1");
    assert!(!format!("{prepared:?}").contains("recorded\\\":true"));
    assert_eq!(
        format!(
            "{:?}",
            ImathasQuestionLocation::from_draft_backend_locator(
                &DraftQuestionBackendLocator::Imathas {
                    binding: draft_binding(),
                }
            )
            .unwrap()
        ),
        "ImathasQuestionLocation(REDACTED)"
    );
}

#[tokio::test]
async fn immutable_snapshot_cache_and_verified_grade_are_bound_to_exact_attempt() {
    let store = MemoryObjectStore::default();
    let recorded = question_backend();
    let renders = recorded.renders.clone();
    let adapter = ImathasAdapter::new(store.clone(), recorded, [profile()]);
    let (question, source, _) = stored_source(&store).await;
    let first = adapter
        .issue(
            &question,
            QuestionSeed::new(17),
            &source,
            Timestamp::from_unix_millis(2),
        )
        .await
        .unwrap();
    let second = adapter
        .issue(
            &question,
            QuestionSeed::new(17),
            &source,
            Timestamp::from_unix_millis(3),
        )
        .await
        .unwrap();
    assert!(!first.cache_hit);
    assert!(second.cache_hit);
    assert_eq!(renders.load(Ordering::SeqCst), 1);
    assert_eq!(first.presentation.title, "Recorded iMathAS question");
    assert_eq!(second.presentation.title, first.presentation.title);
    assert!(matches!(
        first.presentation.response,
        question_model::QuestionResponseFormat::ImathasQuestionBackend {}
    ));
    let serialized = serde_json::to_string(&first.presentation).unwrap();
    for forbidden in ["token", "launch", "score", "correct", "recorded\\\":true"] {
        assert!(!serialized.contains(forbidden));
    }
    let result = adapter
        .verify_imathas_result(
            &question,
            &source,
            &grading_context(&question, QuestionSeed::new(17)),
            &launch_session_authentication(&grading_context(&question, QuestionSeed::new(17))),
        )
        .await
        .unwrap();
    assert_eq!(result.imathas_result().normalized_score().value(), 1.0);
}

#[tokio::test]
async fn historical_invalid_metadata_title_is_refused_before_question_backend_or_cache() {
    let store = MemoryObjectStore::default();
    let recorded = question_backend();
    let renders = recorded.renders.clone();
    let adapter = ImathasAdapter::new(store.clone(), recorded, [profile()]);
    let (mut question, source, _) = stored_source(&store).await;
    question.metadata.title = " \n ".into();
    assert!(matches!(
        adapter
            .issue(
                &question,
                QuestionSeed::new(17),
                &source,
                Timestamp::from_unix_millis(2),
            )
            .await,
        Err(ImathasAdapterError::InvalidTitle(_))
    ));
    assert_eq!(renders.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn wrong_locator_binding_and_outage_refuse_without_fabricating_incorrectness() {
    let store = MemoryObjectStore::default();
    let (question, source, _) = stored_source(&store).await;
    let mut changed_source = source.clone();
    changed_source.binding = ImathasQuestionBackendBinding::new(
        ImathasDeploymentReference::new("different-imathas").expect("different deployment"),
        ImathasItemReference::new("item-17").expect("recorded item"),
        ImathasProfile::new("recorded-v1").expect("recorded profile"),
    );
    assert_eq!(
        verify_binding(&question, &changed_source),
        Err(ImathasAdapterError::SourceDoesNotMatchQuestion)
    );
    let wrong = ImathasAdapter::new(
        store.clone(),
        RecordedImathasQuestionBackend {
            mismatch: Some(Mismatch::Version),
            ..question_backend()
        },
        [profile()],
    );
    let error = wrong
        .verify_imathas_result(
            &question,
            &source,
            &grading_context(&question, QuestionSeed::new(17)),
            &launch_session_authentication(&grading_context(&question, QuestionSeed::new(17))),
        )
        .await
        .unwrap_err();
    assert_eq!(error, ImathasAdapterError::VerificationRefused);
    let outage = ImathasAdapter::new(
        store,
        RecordedImathasQuestionBackend {
            outage: true,
            ..question_backend()
        },
        [profile()],
    );
    assert!(matches!(
        outage
            .issue(
                &question,
                QuestionSeed::new(18),
                &source,
                Timestamp::from_unix_millis(2)
            )
            .await,
        Err(ImathasAdapterError::QuestionBackend(
            ImathasQuestionBackendFailure::Unavailable
        ))
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
        Mismatch::QuestionSeed,
        Mismatch::LaunchSessionAuthentication,
    ] {
        let adapter = ImathasAdapter::new(
            store.clone(),
            RecordedImathasQuestionBackend {
                mismatch: Some(mismatch),
                ..question_backend()
            },
            [profile()],
        );
        assert_eq!(
            adapter
                .verify_imathas_result(
                    &question,
                    &source,
                    &grading_context(&question, QuestionSeed::new(17)),
                    &launch_session_authentication(&grading_context(
                        &question,
                        QuestionSeed::new(17)
                    )),
                )
                .await
                .unwrap_err(),
            ImathasAdapterError::VerificationRefused
        );
    }
    let codec = ImathasSessionAuthenticationCodec::from_server_secret([8; 32]).unwrap();
    let binding = grading_context(&question, QuestionSeed::new(17));
    let challenge =
        learning_data_access::ImathasQuestionBackendSessionChallenge::generate().unwrap();
    let restored = codec.authenticate_for_lda(&binding, &challenge);
    let adapter = ImathasAdapter::new(store.clone(), question_backend(), [profile()]);
    assert_eq!(
        adapter
            .verify_imathas_result(&question, &source, &binding, &restored)
            .await
            .unwrap()
            .imathas_result()
            .normalized_score()
            .value(),
        1.0
    );
    let altered_challenge =
        learning_data_access::ImathasQuestionBackendSessionChallenge::generate().unwrap();
    assert_ne!(
        restored,
        codec.authenticate_for_lda(&binding, &altered_challenge)
    );
    assert_ne!(
        restored,
        ImathasSessionAuthenticationCodec::from_server_secret([9; 32])
            .unwrap()
            .authenticate_for_lda(&binding, &challenge)
    );
    assert_ne!(
        restored,
        codec.authenticate_for_lda(
            &learning_data_access::ImathasGradingContext::new(
                binding.question_attempt(),
                binding.question_revision().clone(),
                QuestionSeed::new(18),
            ),
            &challenge
        )
    );
}

#[test]
fn grading_context_dimensions_change_hmac_and_binding_digest() {
    let question = question();
    let baseline = grading_context(&question, QuestionSeed::new(17));
    let challenge =
        learning_data_access::ImathasQuestionBackendSessionChallenge::generate().unwrap();
    let codec = ImathasSessionAuthenticationCodec::from_server_secret([8; 32]).unwrap();
    let baseline_authentication = codec.authenticate_for_lda(&baseline, &challenge);
    let baseline_digest = crate::result_verification::launch_binding_digest(
        &baseline,
        "item-17",
        &"a".repeat(64),
        crate::result_verification::normalize_imathas_seed(baseline.question_seed()),
        baseline_authentication.as_str(),
    );

    let alternatives = [
        learning_data_access::ImathasGradingContext::new(
            QuestionAttemptId::from_uuid(Uuid::from_u128(99)),
            baseline.question_revision().clone(),
            baseline.question_seed(),
        ),
        learning_data_access::ImathasGradingContext::new(
            baseline.question_attempt(),
            QuestionRevisionReference {
                question_id: QuestionId::from_canonical_parts("BCDEFG", 'H').unwrap(),
                revision_number: baseline.question_revision().revision_number,
            },
            baseline.question_seed(),
        ),
        learning_data_access::ImathasGradingContext::new(
            baseline.question_attempt(),
            QuestionRevisionReference {
                question_id: baseline.question_revision().question_id.clone(),
                revision_number: QuestionRevisionNumber::new(99).unwrap(),
            },
            baseline.question_seed(),
        ),
        learning_data_access::ImathasGradingContext::new(
            baseline.question_attempt(),
            baseline.question_revision().clone(),
            QuestionSeed::new(99),
        ),
    ];

    for alternative in alternatives {
        let authentication = codec.authenticate_for_lda(&alternative, &challenge);
        assert_ne!(authentication, baseline_authentication);
        assert_ne!(
            crate::result_verification::launch_binding_digest(
                &alternative,
                "item-17",
                &"a".repeat(64),
                crate::result_verification::normalize_imathas_seed(alternative.question_seed()),
                authentication.as_str(),
            ),
            baseline_digest
        );
    }
}

#[tokio::test]
async fn malformed_stored_cache_and_grade_outage_remain_local_and_redacted() {
    let store = MemoryObjectStore::default();
    let (question, source, _) = stored_source(&store).await;
    let key = render_key(
        &QuestionRevisionReference {
            question_id: question.question_id.clone(),
            revision_number: question.revision_number,
        },
        QuestionSeed::new(31),
    );
    store
        .put(PutObject {
            address: key,
            bytes: b"{malformed".to_vec(),
            media_type: "application/json".into(),
            created_at: Timestamp::from_unix_millis(1),
        })
        .await
        .unwrap();
    let adapter = ImathasAdapter::new(store.clone(), question_backend(), [profile()]);
    assert_eq!(
        adapter
            .issue(
                &question,
                QuestionSeed::new(31),
                &source,
                Timestamp::from_unix_millis(2)
            )
            .await
            .unwrap_err(),
        ImathasAdapterError::InvalidCache
    );
    let outage = ImathasAdapter::new(
        store,
        RecordedImathasQuestionBackend {
            outage: true,
            ..question_backend()
        },
        [profile()],
    );
    assert!(matches!(
        outage
            .verify_imathas_result(
                &question,
                &source,
                &grading_context(&question, QuestionSeed::new(17)),
                &launch_session_authentication(&grading_context(&question, QuestionSeed::new(17))),
            )
            .await,
        Err(ImathasAdapterError::QuestionBackend(
            ImathasQuestionBackendFailure::Timeout
        ))
    ));
    let text = ImathasAdapterError::QuestionBackend(ImathasQuestionBackendFailure::Unavailable)
        .to_string();
    assert!(!text.contains("token"));
    assert!(text.len() < 100);
}

#[tokio::test]
async fn concurrent_replicas_reuse_the_winning_immutable_render() {
    let store = MemoryObjectStore::default();
    let recorded = question_backend();
    let adapter = ImathasAdapter::new(store.clone(), recorded, [profile()]);
    let (question, source, _) = stored_source(&store).await;
    let first = adapter.issue(
        &question,
        QuestionSeed::new(41),
        &source,
        Timestamp::from_unix_millis(2),
    );
    let second = adapter.issue(
        &question,
        QuestionSeed::new(41),
        &source,
        Timestamp::from_unix_millis(2),
    );
    let (first, second) = tokio::join!(first, second);
    let first = first.unwrap();
    let second = second.unwrap();
    assert!(first.cache_hit || second.cache_hit);
    assert_eq!(first.presentation, second.presentation);
}

fn grading_context(
    question: &QuestionRevision,
    question_seed: QuestionSeed,
) -> learning_data_access::ImathasGradingContext {
    learning_data_access::ImathasGradingContext::new(
        QuestionAttemptId::from_uuid(Uuid::from_u128(6)),
        QuestionRevisionReference {
            question_id: question.question_id.clone(),
            revision_number: question.revision_number,
        },
        question_seed,
    )
}

fn launch_session_authentication(
    grading_context: &learning_data_access::ImathasGradingContext,
) -> learning_data_access::ImathasQuestionBackendSessionAuthentication {
    let codec = ImathasSessionAuthenticationCodec::from_server_secret([7; 32]).unwrap();
    let challenge =
        learning_data_access::ImathasQuestionBackendSessionChallenge::generate().unwrap();
    codec.authenticate_for_lda(grading_context, &challenge)
}
