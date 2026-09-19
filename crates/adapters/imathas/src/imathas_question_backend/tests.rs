use learning_data_access::ImathasQuestionBackendStatePlaintext;

use super::{ImathasLaunchId, ImathasLaunchState};

#[test]
fn imathas_launch_state_is_strict_versioned_and_redacted() {
    let state = ImathasLaunchState::from_launch_handle(
        ImathasLaunchId::from_server_handle("imathas-handle_1").unwrap(),
    );
    let plaintext = state.encode().unwrap();
    assert_eq!(format!("{state:?}"), "ImathasLaunchState(REDACTED)");
    assert!(!format!("{plaintext:?}").contains("imathas-handle_1"));
    assert_eq!(
        ImathasLaunchState::decode(&plaintext)
            .unwrap()
            .handle()
            .protected_value(),
        "imathas-handle_1"
    );
    for invalid in [
        vec![2, 1, b'x'],
        vec![1, 0],
        vec![1, 2, b'x'],
        vec![1, 1, b'!'],
        vec![1, 1, b'x', b'y'],
    ] {
        let value =
            ImathasQuestionBackendStatePlaintext::from_versioned_adapter_bytes(invalid).unwrap();
        assert!(ImathasLaunchState::decode(&value).is_err());
    }
}

#[cfg(feature = "test-support")]
mod launch_session_bridge {
    use base64::Engine as _;
    use hmac::{Hmac, KeyInit, Mac};
    use learning_data_access::{
        ImathasQuestionBackendSessionPreparationContext,
        ImathasQuestionBackendSessionRestoreExpectation, ImathasQuestionBackendSessionStore,
        ImathasQuestionBackendStateKeyId, ImathasQuestionBackendStateKeyRing,
        ImathasResponseChecksum, MemoryImathasQuestionBackendSessionStore, SessionTokenHash,
    };
    use objects::memory::MemoryObjectStore;
    use objects::{ObjectAddress, ObjectStore, PutObject};
    use question_model::generation::QuestionSeed;
    use question_model::{
        AccountId, AssessmentId, CourseInstanceId, ImathasDeploymentId, ImathasItemId,
        ImathasProfile, ImathasQuestionBackendBinding, ObjectId, QuestionAttemptId, QuestionId,
        QuestionRevisionNumber, QuestionRevisionTuple, SourceObjectChecksum, Timestamp,
    };
    use sha2::{Digest, Sha256};
    use uuid::Uuid;

    use super::*;
    use crate::imathas_question_backend::ProxyMethod;
    use crate::test_support::{
        RecordedImathasQuestionBackendTransport, RecordedImathasQuestionBackendTransportMode,
        recorded_imathas_question_backend_with_transport,
    };
    use crate::{
        ImathasAdapter, ImathasAdapterError, ImathasQuestionBackendFailure,
        ImathasSessionAuthenticationCodec, ResolvedImathasQuestionSource,
    };

    fn now() -> Timestamp {
        Timestamp::from_unix_millis(20)
    }

    fn expires() -> Timestamp {
        Timestamp::from_unix_millis(100)
    }

    fn imathas_binding() -> ImathasQuestionBackendBinding {
        ImathasQuestionBackendBinding::new(
            ImathasDeploymentId::new("self-hosted-imathas").expect("deployment"),
            ImathasItemId::new("item17").expect("item"),
            ImathasProfile::new(crate::result_verification::IMATHAS_GRADING_PROFILE_ID)
                .expect("profile"),
        )
    }

    fn question() -> QuestionRevisionTuple {
        QuestionRevisionTuple {
            question_id: QuestionId::from_random_identifier("ABCDEFG").expect("question ID"),
            revision_number: QuestionRevisionNumber::new(2).expect("revision"),
        }
    }

    async fn source(
        store: &MemoryObjectStore,
    ) -> (
        QuestionRevisionTuple,
        ResolvedImathasQuestionSource,
        ObjectId,
    ) {
        let question = question();
        let object = ObjectId::from_uuid(Uuid::from_u128(4));
        let receipt = store
            .put(PutObject {
                address: ObjectAddress::QuestionSource {
                    question_revision_tuple: question.clone(),
                    object,
                },
                bytes: br#"{"recorded":true}"#.to_vec(),
                media_type: "application/json".into(),
                created_at: Timestamp::from_unix_millis(1),
            })
            .await
            .expect("fixture source");
        let artifact = object;
        let resolved = ResolvedImathasQuestionSource::resolve(
            store,
            question.clone(),
            imathas_binding(),
            artifact,
            SourceObjectChecksum::parse(receipt.sha256.to_string()).expect("checksum"),
        )
        .await
        .expect("resolved source");
        (question, resolved, artifact)
    }

    fn context(
        question: &QuestionRevisionTuple,
        source: &ResolvedImathasQuestionSource,
        artifact: &ObjectId,
    ) -> ImathasQuestionBackendSessionPreparationContext {
        let revision = question.clone();
        let grading_context = learning_data_access::ImathasGradingContext::new(
            QuestionAttemptId::from_uuid(Uuid::from_u128(7)),
            revision,
            QuestionSeed::new(11),
        );
        let challenge = learning_data_access::ImathasQuestionBackendSessionChallenge::generate()
            .expect("challenge");
        let authentication = ImathasSessionAuthenticationCodec::from_server_secret([9; 32])
            .expect("codec")
            .authenticate_for_lda(&grading_context, &challenge);
        let imathas_question_backend_binding = lda_imathas_backend();
        ImathasQuestionBackendSessionPreparationContext::new(
            AccountId::from_debug_serial(1),
            CourseInstanceId::from_debug_serial(2),
            AssessmentId::from_debug_serial(3),
            grading_context,
            imathas_question_backend_binding,
            *artifact,
            source.source_object_checksum().clone(),
            ImathasResponseChecksum::from_bytes([4; 32]),
            challenge,
            authentication,
            Timestamp::from_unix_millis(10),
            expires(),
        )
        .expect("context")
    }

    fn lda_imathas_backend() -> ImathasQuestionBackendBinding {
        imathas_binding()
    }

    fn assert_no_transport_io(transport: &RecordedImathasQuestionBackendTransport) {
        assert_eq!(transport.snapshot_calls(), 0);
        assert_eq!(transport.launch_calls(), 0);
        assert_eq!(transport.proxy_calls(), 0);
        assert_eq!(transport.result_calls(), 0);
    }

    fn signed_result_token(
        validation: &learning_data_access::ImathasQuestionBackendSessionValidation,
        score: &str,
    ) -> learning_data_access::ImathasResultToken {
        let codec = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let header = codec.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = codec.encode(format!(
            r#"{{"id":"{}","score":{score},"ple_launch_challenge":"{}","ple_binding":"{}"}}"#,
            validation
                .imathas_question_backend_binding
                .item_id()
                .as_str(),
            codec.encode(validation.challenge.as_bytes()),
            validation.imathas_launch_binding_checksum.as_str(),
        ));
        let signed = format!("{header}.{payload}");
        let mut mac = Hmac::<Sha256>::new_from_slice(b"recorded-result-secret")
            .expect("fixed recorded result secret");
        mac.update(signed.as_bytes());
        learning_data_access::ImathasResultToken::from_server_adapter_bytes(
            format!("{signed}.{}", codec.encode(mac.finalize().into_bytes())).into_bytes(),
        )
        .expect("bounded signed result")
    }

    #[tokio::test]
    async fn current_lda_preparation_and_loaded_validation_drive_prepare_proxy_and_result() {
        let object_store = MemoryObjectStore::default();
        let (question, source, artifact) = source(&object_store).await;
        let (question_backend, transport) = recorded_imathas_question_backend_with_transport(
            RecordedImathasQuestionBackendTransportMode::Verified,
        );
        let adapter = ImathasAdapter::new(object_store, question_backend, []);
        let context = context(&question, &source, &artifact);
        let expected_authentication = context.preparation_validation().authentication;
        let preparation = adapter
            .prepare_imathas_question_backend_launch(
                &source,
                &context.preparation_validation(),
                now(),
            )
            .await
            .expect("prepare");
        assert_eq!(transport.snapshot_calls(), 1);
        assert_eq!(transport.launch_calls(), 1);
        assert!(!format!("{preparation:?}").contains("recorded-proxy-session"));

        let account = AccountId::from_debug_serial(1);
        let token = SessionTokenHash::compute(b"adapter-launch-session");
        let store = MemoryImathasQuestionBackendSessionStore::new(
            ImathasQuestionBackendStateKeyRing::new(
                ImathasQuestionBackendStateKeyId::parse("adapter-test-key").expect("key ID"),
                [6; 32],
                [],
            )
            .expect("key ring"),
            now(),
        );
        store.install_authenticated_session(token, account.clone());
        store.install_active_student_authorization(
            account.clone(),
            CourseInstanceId::from_debug_serial(2),
            QuestionAttemptId::from_uuid(Uuid::from_u128(7)),
        );
        let digest = preparation.imathas_launch_binding_checksum().clone();
        let session_id = store
            .create_imathas_question_backend_session(
                token,
                context
                    .complete(digest.clone(), preparation.imathas_launch_state().clone())
                    .expect("create"),
            )
            .await
            .expect("store");
        let imathas_question_backend_binding = lda_imathas_backend();
        let initial_restore_expectation = ImathasQuestionBackendSessionRestoreExpectation::new(
            account,
            CourseInstanceId::from_debug_serial(2),
            AssessmentId::from_debug_serial(3),
            learning_data_access::ImathasGradingContext::new(
                QuestionAttemptId::from_uuid(Uuid::from_u128(7)),
                question.clone(),
                QuestionSeed::new(11),
            ),
            imathas_question_backend_binding,
            artifact,
            source.source_object_checksum().clone(),
            digest,
            expected_authentication,
        );
        let loaded = store
            .load_imathas_question_backend_session(
                token,
                session_id,
                initial_restore_expectation.clone(),
            )
            .await
            .expect("load");
        let state =
            ImathasLaunchState::decode(loaded.imathas_question_backend_state()).expect("state");
        let validation = loaded.imathas_question_backend_validation();
        let proxy = adapter
            .proxy_imathas_question_backend_activity(
                &validation,
                &state,
                ProxyMethod::Get,
                &[],
                now(),
            )
            .await
            .expect("proxy");
        assert!(proxy.html().starts_with(b"<!doctype html>"));
        let result = adapter
            .retrieve_verified_imathas_result(&validation, &state, now())
            .await
            .expect("result");
        let exact_transport_token = transport
            .recorded_result_token_bytes()
            .expect("recorded signed result token");
        let expected_checksum: [u8; 32] = Sha256::digest(exact_transport_token).into();
        assert_eq!(
            result.imathas_result_token_checksum().as_bytes(),
            &expected_checksum,
            "the verified receipt hashes the exact bytes returned by the transport"
        );
        assert!(format!("{:?}", result.imathas_result_token_checksum()).contains("redacted"));
        assert!(!format!("{result:?}").contains("eyJ"));
        assert_eq!(transport.proxy_calls(), 1);
        assert_eq!(transport.result_calls(), 1);
        assert!(!format!("{loaded:?}").contains("recorded-proxy-session"));
        assert!(!format!("{validation:?}").contains("recorded-proxy-session"));

        let verifier = crate::result_verification::ImathasResultVerifier::new(
            crate::result_verification::ImathasGradingProfile::grading_deployment(
                "self-hosted-imathas",
                true,
                true,
            )
            .expect("iMathAS grading profile"),
            b"recorded-result-secret",
        )
        .expect("result verifier");
        for (token_score, expected_score) in [("0.0", 0.0), ("1.0", 1.0)] {
            let verified = verifier
                .verify_result(
                    &validation,
                    &signed_result_token(&validation, token_score),
                    now(),
                )
                .expect("exact score boundary is accepted");
            assert_eq!(
                verified.imathas_result().normalized_score().value(),
                expected_score
            );
        }

        let mut wrong_hmac = validation.clone();
        wrong_hmac.authentication =
            learning_data_access::ImathasQuestionBackendSessionAuthentication::from_server_value(
                format!("aa.{}", "c".repeat(64)),
            )
            .expect("authentication");
        let mut wrong_challenge = validation.clone();
        wrong_challenge.challenge =
            learning_data_access::ImathasQuestionBackendSessionChallenge::generate()
                .expect("challenge");
        let mut wrong_binding_checksum = validation.clone();
        wrong_binding_checksum.imathas_launch_binding_checksum =
            learning_data_access::ImathasLaunchBindingChecksum::parse("d".repeat(64))
                .expect("iMathAS Launch Binding Checksum");
        let mut expired = validation.clone();
        expired.expires_at = now();
        let mut wrong_deployment = validation.clone();
        wrong_deployment.imathas_question_backend_binding = ImathasQuestionBackendBinding::new(
            ImathasDeploymentId::new("wrong-imathas").expect("deployment"),
            wrong_deployment
                .imathas_question_backend_binding
                .item_id()
                .clone(),
            wrong_deployment
                .imathas_question_backend_binding
                .profile()
                .clone(),
        );
        let mut wrong_profile = validation.clone();
        wrong_profile.imathas_question_backend_binding = ImathasQuestionBackendBinding::new(
            wrong_profile
                .imathas_question_backend_binding
                .deployment_id()
                .clone(),
            wrong_profile
                .imathas_question_backend_binding
                .item_id()
                .clone(),
            ImathasProfile::new("wrong-profile").expect("profile"),
        );
        let mut wrong_item = validation.clone();
        wrong_item.imathas_question_backend_binding = ImathasQuestionBackendBinding::new(
            wrong_item
                .imathas_question_backend_binding
                .deployment_id()
                .clone(),
            ImathasItemId::new("wrong-item").expect("item"),
            wrong_item
                .imathas_question_backend_binding
                .profile()
                .clone(),
        );
        let mut wrong_checksum = validation.clone();
        wrong_checksum.source_object_checksum =
            SourceObjectChecksum::parse("a".repeat(64)).expect("checksum");
        let mut wrong_attempt = validation.clone();
        wrong_attempt.grading_context = learning_data_access::ImathasGradingContext::new(
            QuestionAttemptId::from_uuid(Uuid::from_u128(99)),
            wrong_attempt
                .grading_context
                .question_revision_tuple()
                .clone(),
            wrong_attempt.grading_context.question_seed(),
        );
        let mut wrong_question_id = validation.clone();
        wrong_question_id.grading_context = learning_data_access::ImathasGradingContext::new(
            wrong_question_id.grading_context.question_attempt(),
            QuestionRevisionTuple {
                question_id: QuestionId::from_random_identifier("BCDEFGH").expect("Question ID"),
                revision_number: wrong_question_id
                    .grading_context
                    .question_revision_tuple()
                    .revision_number,
            },
            wrong_question_id.grading_context.question_seed(),
        );
        let mut wrong_revision = validation.clone();
        wrong_revision.grading_context = learning_data_access::ImathasGradingContext::new(
            wrong_revision.grading_context.question_attempt(),
            QuestionRevisionTuple {
                question_id: wrong_revision
                    .grading_context
                    .question_revision_tuple()
                    .question_id
                    .clone(),
                revision_number: QuestionRevisionNumber::new(99).expect("revision"),
            },
            wrong_revision.grading_context.question_seed(),
        );
        let mut wrong_seed = validation.clone();
        wrong_seed.grading_context = learning_data_access::ImathasGradingContext::new(
            wrong_seed.grading_context.question_attempt(),
            wrong_seed.grading_context.question_revision_tuple().clone(),
            QuestionSeed::new(12),
        );

        for invalid in [
            wrong_hmac,
            wrong_challenge,
            wrong_binding_checksum,
            expired,
            wrong_deployment,
            wrong_profile,
            wrong_item,
            wrong_checksum,
            wrong_attempt,
            wrong_question_id,
            wrong_revision,
            wrong_seed,
        ] {
            let (question_backend, refused_transport) =
                recorded_imathas_question_backend_with_transport(
                    RecordedImathasQuestionBackendTransportMode::Verified,
                );
            let refused_adapter =
                ImathasAdapter::new(MemoryObjectStore::default(), question_backend, []);
            assert!(matches!(
                refused_adapter
                    .proxy_imathas_question_backend_activity(
                        &invalid,
                        &state,
                        ProxyMethod::Get,
                        &[],
                        now()
                    )
                    .await,
                Err(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication)
            ));
            assert_eq!(
                refused_adapter
                    .retrieve_verified_imathas_result(&invalid, &state, now())
                    .await,
                Err(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication)
            );
            assert_no_transport_io(&refused_transport);
        }
    }

    #[tokio::test]
    async fn preparation_refuses_mismatches_before_recording_transport_io() {
        let object_store = MemoryObjectStore::default();
        let (question, source, artifact) = source(&object_store).await;
        let (_, hostile_transport) = recorded_imathas_question_backend_with_transport(
            RecordedImathasQuestionBackendTransportMode::Verified,
        );
        let hostile =
            ImathasQuestionBackendStatePlaintext::from_versioned_adapter_bytes(vec![1, 1, b'!'])
                .expect("bounded hostile state");
        assert!(ImathasLaunchState::decode(&hostile).is_err());
        assert_no_transport_io(&hostile_transport);
        for mutator in [
            |validation: &mut learning_data_access::ImathasQuestionBackendLaunchPreparationValidation| {
                validation.imathas_question_backend_binding = ImathasQuestionBackendBinding::new(
                    ImathasDeploymentId::new("wrong-imathas").expect("deployment"),
                    validation
                        .imathas_question_backend_binding
                        .item_id()
                        .clone(),
                    validation
                        .imathas_question_backend_binding
                        .profile()
                        .clone(),
                )
            },
            |validation: &mut learning_data_access::ImathasQuestionBackendLaunchPreparationValidation| {
                validation.source_object = ObjectId::from_uuid(Uuid::from_u128(99));
            },
            |validation: &mut learning_data_access::ImathasQuestionBackendLaunchPreparationValidation| {
                validation.source_object_checksum =
                    SourceObjectChecksum::parse("a".repeat(64)).expect("checksum")
            },
            |validation: &mut learning_data_access::ImathasQuestionBackendLaunchPreparationValidation| {
                validation.imathas_question_backend_binding = ImathasQuestionBackendBinding::new(
                    validation
                        .imathas_question_backend_binding
                        .deployment_id()
                        .clone(),
                    validation
                        .imathas_question_backend_binding
                        .item_id()
                        .clone(),
                    ImathasProfile::new("wrong-profile").expect("profile"),
                )
            },
            |validation: &mut learning_data_access::ImathasQuestionBackendLaunchPreparationValidation| {
                validation.grading_context = learning_data_access::ImathasGradingContext::new(
                    QuestionAttemptId::from_uuid(Uuid::from_u128(99)),
                    validation.grading_context.question_revision_tuple().clone(),
                    validation.grading_context.question_seed(),
                )
            },
            |validation: &mut learning_data_access::ImathasQuestionBackendLaunchPreparationValidation| {
                validation.grading_context = learning_data_access::ImathasGradingContext::new(
                    validation.grading_context.question_attempt(),
                    QuestionRevisionTuple {
                        question_id: QuestionId::from_random_identifier("BCDEFGH")
                            .expect("Question ID"),
                        revision_number: validation
                            .grading_context
                            .question_revision_tuple()
                            .revision_number,
                    },
                    validation.grading_context.question_seed(),
                )
            },
            |validation: &mut learning_data_access::ImathasQuestionBackendLaunchPreparationValidation| {
                validation.grading_context = learning_data_access::ImathasGradingContext::new(
                    validation.grading_context.question_attempt(),
                    QuestionRevisionTuple {
                        question_id: validation
                            .grading_context
                            .question_revision_tuple()
                            .question_id
                            .clone(),
                        revision_number: QuestionRevisionNumber::new(99).expect("revision"),
                    },
                    validation.grading_context.question_seed(),
                )
            },
            |validation: &mut learning_data_access::ImathasQuestionBackendLaunchPreparationValidation| {
                validation.grading_context = learning_data_access::ImathasGradingContext::new(
                    validation.grading_context.question_attempt(),
                    validation.grading_context.question_revision_tuple().clone(),
                    QuestionSeed::new(12),
                )
            },
            |validation: &mut learning_data_access::ImathasQuestionBackendLaunchPreparationValidation| {
                validation.challenge = learning_data_access::ImathasQuestionBackendSessionChallenge::generate()
                    .expect("challenge")
            },
            |validation: &mut learning_data_access::ImathasQuestionBackendLaunchPreparationValidation| {
                validation.authentication =
                    learning_data_access::ImathasQuestionBackendSessionAuthentication::from_server_value(
                        format!("aa.{}", "c".repeat(64)),
                    )
                    .expect("authentication")
            },
            |validation: &mut learning_data_access::ImathasQuestionBackendLaunchPreparationValidation| {
                validation.expires_at = now()
            },
        ] {
            let (question_backend, transport) = recorded_imathas_question_backend_with_transport(
                RecordedImathasQuestionBackendTransportMode::Verified,
            );
            let adapter = ImathasAdapter::new(object_store.clone(), question_backend, []);
            let mut validation = context(&question, &source, &artifact).preparation_validation();
            mutator(&mut validation);
            assert!(matches!(
                adapter
                    .prepare_imathas_question_backend_launch(&source, &validation, now())
                    .await,
                Err(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication)
            ));
            assert_no_transport_io(&transport);
        }
    }

    #[tokio::test]
    async fn source_revalidation_wrong_results_and_outages_stay_bound_and_redacted() {
        let object_store = MemoryObjectStore::default();
        let (question, source, artifact) = source(&object_store).await;
        let context = context(&question, &source, &artifact);
        let changed = recorded_imathas_question_backend_with_transport(
            RecordedImathasQuestionBackendTransportMode::SourceChanged,
        );
        let changed_adapter = ImathasAdapter::new(object_store.clone(), changed.0, []);
        assert!(matches!(
            changed_adapter
                .prepare_imathas_question_backend_launch(
                    &source,
                    &context.preparation_validation(),
                    now()
                )
                .await,
            Err(ImathasAdapterError::SourceChecksumMismatch)
        ));
        assert_eq!(changed.1.snapshot_calls(), 1);
        assert_eq!(changed.1.launch_calls(), 0);

        for mode in [
            RecordedImathasQuestionBackendTransportMode::WrongSignedResult,
            RecordedImathasQuestionBackendTransportMode::WrongImathasItemId,
            RecordedImathasQuestionBackendTransportMode::InvalidScore,
            RecordedImathasQuestionBackendTransportMode::NegativeZeroScore,
            RecordedImathasQuestionBackendTransportMode::ExpiredSignedResult,
            RecordedImathasQuestionBackendTransportMode::WrongAlgorithm,
            RecordedImathasQuestionBackendTransportMode::InvalidSignature,
            RecordedImathasQuestionBackendTransportMode::MalformedResult,
            RecordedImathasQuestionBackendTransportMode::NonUtf8Result,
            RecordedImathasQuestionBackendTransportMode::OversizedResult,
            RecordedImathasQuestionBackendTransportMode::ResultUnavailable,
        ] {
            let (question_backend, transport) =
                recorded_imathas_question_backend_with_transport(mode);
            let adapter = ImathasAdapter::new(object_store.clone(), question_backend, []);
            let prevalidation = context.preparation_validation();
            let preparation = adapter
                .prepare_imathas_question_backend_launch(&source, &prevalidation, now())
                .await
                .expect("prepare");
            let validation = learning_data_access::ImathasQuestionBackendSessionValidation {
                grading_context: prevalidation.grading_context,
                imathas_question_backend_binding: prevalidation.imathas_question_backend_binding,
                source_object: prevalidation.source_object,
                source_object_checksum: prevalidation.source_object_checksum,
                response_checksum: prevalidation.response_checksum,
                challenge: prevalidation.challenge,
                authentication: prevalidation.authentication,
                imathas_launch_binding_checksum: preparation
                    .imathas_launch_binding_checksum()
                    .clone(),
                expires_at: prevalidation.expires_at,
            };
            let state =
                ImathasLaunchState::decode(preparation.imathas_launch_state()).expect("state");
            let expected = match mode {
                RecordedImathasQuestionBackendTransportMode::WrongSignedResult => {
                    ImathasAdapterError::VerificationRefused
                }
                RecordedImathasQuestionBackendTransportMode::WrongImathasItemId
                | RecordedImathasQuestionBackendTransportMode::InvalidScore
                | RecordedImathasQuestionBackendTransportMode::NegativeZeroScore
                | RecordedImathasQuestionBackendTransportMode::WrongAlgorithm
                | RecordedImathasQuestionBackendTransportMode::InvalidSignature
                | RecordedImathasQuestionBackendTransportMode::MalformedResult => {
                    ImathasAdapterError::VerificationRefused
                }
                RecordedImathasQuestionBackendTransportMode::ExpiredSignedResult => {
                    ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication
                }
                RecordedImathasQuestionBackendTransportMode::NonUtf8Result => {
                    ImathasAdapterError::VerificationRefused
                }
                RecordedImathasQuestionBackendTransportMode::OversizedResult => {
                    ImathasAdapterError::QuestionBackend(
                        ImathasQuestionBackendFailure::InvalidResponse,
                    )
                }
                RecordedImathasQuestionBackendTransportMode::ResultUnavailable => {
                    ImathasAdapterError::QuestionBackend(ImathasQuestionBackendFailure::Unavailable)
                }
                _ => unreachable!(),
            };
            assert_eq!(
                adapter
                    .retrieve_verified_imathas_result(&validation, &state, now())
                    .await,
                Err(expected)
            );
            assert_eq!(transport.result_calls(), 1);
        }
    }
}
