use super::*;
use question_model::generation::QuestionSeed;
use question_model::{
    AccountId, AssignmentId, CourseId, ImathasDeploymentReference, ImathasItemReference,
    ImathasProfile, ImathasQuestionBackendBinding, ObjectId, QuestionAttemptId,
    QuestionGradingRule, QuestionId, QuestionRevisionNumber, QuestionRevisionReference,
    SourceObjectChecksum, SourceObjectReference, Timestamp,
};
use uuid::Uuid;
fn facts(
    account: AccountId,
) -> (
    ImathasQuestionBackendSessionCreate,
    ImathasQuestionBackendSessionRestoreExpectation,
) {
    facts_with_rule(account, QuestionGradingRule::PartialCredit { points: 10.0 })
}

fn facts_with_rule(
    account: AccountId,
    grading_rule: QuestionGradingRule,
) -> (
    ImathasQuestionBackendSessionCreate,
    ImathasQuestionBackendSessionRestoreExpectation,
) {
    let course = CourseId::from_uuid(Uuid::from_u128(2));
    let assignment = AssignmentId::from_uuid(Uuid::from_u128(3));
    let attempt = QuestionAttemptId::from_uuid(Uuid::from_u128(4));
    let revision = QuestionRevisionReference {
        question_id: "123-4567".parse::<QuestionId>().expect("question ID"),
        revision_number: QuestionRevisionNumber::new(1).expect("revision"),
    };
    let imathas_question_backend_binding = ImathasQuestionBackendBinding::new(
        ImathasDeploymentReference::new("imathas").expect("deployment"),
        ImathasItemReference::new("item-1").expect("item"),
        ImathasProfile::new("imathas_remote_grading_v1").expect("profile"),
    );
    let source = SourceObjectReference {
        object: ObjectId::from_uuid(Uuid::from_u128(5)),
    };
    let checksum = SourceObjectChecksum::parse("a".repeat(64)).expect("checksum");
    let seed = QuestionSeed::new(7);
    let grading_context = ImathasGradingContext::new(attempt, revision.clone(), seed);
    let authentication = ImathasQuestionBackendSessionAuthentication::from_server_value(format!(
        "aa.{}",
        "b".repeat(64)
    ))
    .expect("authentication");
    let digest = ImathasLaunchBindingChecksum::parse("c".repeat(64))
        .expect("iMathAS Launch Binding Checksum");
    let expectation = ImathasQuestionBackendSessionRestoreExpectation::new(
        account,
        course,
        assignment,
        grading_context.clone(),
        grading_rule.clone(),
        imathas_question_backend_binding.clone(),
        source.clone(),
        checksum.clone(),
        digest.clone(),
        authentication.clone(),
    );
    let preparation = ImathasQuestionBackendSessionPreparationContext::new(
        account,
        course,
        assignment,
        grading_context,
        grading_rule,
        imathas_question_backend_binding,
        source,
        checksum,
        ImathasResponseChecksum::from_bytes([1; 32]),
        ImathasQuestionBackendSessionChallenge::generate().expect("challenge"),
        authentication,
        Timestamp::from_unix_millis(10),
        Timestamp::from_unix_millis(100),
    )
    .expect("preparation");
    let validation = preparation.preparation_validation();
    assert_eq!(
        validation.grading_context.question_seed(),
        QuestionSeed::new(7)
    );
    assert!(format!("{validation:?}").contains("[redacted]"));
    let create = preparation
        .complete(
            digest,
            ImathasQuestionBackendStatePlaintext::from_versioned_adapter_bytes(vec![1, 2, 3])
                .expect("state"),
        )
        .expect("create");
    (create, expectation)
}

fn ring() -> ImathasQuestionBackendStateKeyRing {
    ImathasQuestionBackendStateKeyRing::new(
        ImathasQuestionBackendStateKeyId::parse("imathas-question-backend-state-2026")
            .expect("key ID"),
        [9; 32],
        [],
    )
    .expect("ring")
}

#[test]
fn grading_context_authentication_payload_v1_has_the_locked_row_530_bytes() {
    let context = ImathasGradingContext::new(
        QuestionAttemptId::from_uuid(Uuid::from_u128(4)),
        QuestionRevisionReference {
            question_id: "123-4567".parse::<QuestionId>().expect("question ID"),
            revision_number: QuestionRevisionNumber::new(1).expect("revision"),
        },
        QuestionSeed::new(7),
    );
    assert_eq!(
        context.question_attempt(),
        QuestionAttemptId::from_uuid(Uuid::from_u128(4))
    );
    assert_eq!(
        context.question_revision().question_id.to_string(),
        "123-4567"
    );
    assert_eq!(context.question_seed(), QuestionSeed::new(7));
    assert_eq!(
        context.authentication_payload_v1(),
        vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, b'1', b'2', b'3', b'-', b'4', b'5',
            b'6', b'7', 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 7,
        ]
    );
    assert_eq!(format!("{context:?}"), "ImathasGradingContext([redacted])");
}
fn transition(lease: ImathasQuestionBackendSessionLease) -> StageVerifiedImathasResult {
    transition_with_score(lease, 1.0)
}

fn transition_with_score(
    lease: ImathasQuestionBackendSessionLease,
    score: f64,
) -> StageVerifiedImathasResult {
    let token = ImathasResultToken::from_server_adapter_bytes(
        b"accepted iMathAS Question Backend result".to_vec(),
    )
    .expect("bounded token");
    let grading_context = lease.expectation.grading_context.clone();
    let authentication = lease.expectation.authentication.clone();
    StageVerifiedImathasResult::new(
        lease,
        grading_context,
        authentication,
        ImathasResultExchangeIdempotencyKey::parse("exchange-1").expect("key"),
        ImathasResultTokenChecksum::from_verified_token(&token),
        ImathasResult::new(ImathasNormalizedScore::try_from_f64(score).expect("score")),
        Timestamp::from_unix_millis(20),
    )
    .expect("stage")
}

#[tokio::test]
async fn stage_refuses_mixed_context_and_authentication() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    let token = SessionTokenHash::compute(b"mixed");
    let store =
        MemoryImathasQuestionBackendSessionStore::new(ring(), Timestamp::from_unix_millis(20));
    authorize(&store, token, account);
    let (create, expectation) = facts(account);
    let reference = store
        .create_imathas_question_backend_session(token, create)
        .await
        .expect("create");
    let lease = store
        .lease_imathas_question_backend_session(
            token,
            reference,
            expectation,
            Timestamp::from_unix_millis(30),
        )
        .await
        .expect("lease");
    let token_checksum = ImathasResultTokenChecksum::from_verified_token(
        &ImathasResultToken::from_server_adapter_bytes(vec![1]).expect("token"),
    );
    let result = ImathasResult::new(ImathasNormalizedScore::try_from_f64(1.0).expect("score"));
    let wrong_context = ImathasGradingContext::new(
        QuestionAttemptId::from_uuid(Uuid::from_u128(99)),
        lease
            .expectation
            .grading_context
            .question_revision()
            .clone(),
        lease.expectation.grading_context.question_seed(),
    );
    assert_eq!(
        StageVerifiedImathasResult::new(
            lease.clone(),
            wrong_context,
            lease.expectation.authentication.clone(),
            ImathasResultExchangeIdempotencyKey::parse("mixed").expect("key"),
            token_checksum,
            result.clone(),
            Timestamp::from_unix_millis(20)
        ),
        Err(StoreError::Forbidden)
    );
    let wrong_auth = ImathasQuestionBackendSessionAuthentication::from_server_value(format!(
        "bb.{}",
        "a".repeat(64)
    ))
    .expect("auth");
    let correct_context = lease.expectation.grading_context.clone();
    assert_eq!(
        StageVerifiedImathasResult::new(
            lease,
            correct_context,
            wrong_auth,
            ImathasResultExchangeIdempotencyKey::parse("mixed").expect("key"),
            token_checksum,
            result,
            Timestamp::from_unix_millis(20)
        ),
        Err(StoreError::Forbidden)
    );
}

#[tokio::test]
async fn memory_job_lease_reclaims_and_commits_once() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    let token = SessionTokenHash::compute(b"worker");
    let store =
        MemoryImathasQuestionBackendSessionStore::new(ring(), Timestamp::from_unix_millis(20));
    authorize(&store, token, account);
    let (create, expectation) = facts(account);
    let reference = store
        .create_imathas_question_backend_session(token, create)
        .await
        .expect("create");
    let lease = store
        .lease_imathas_question_backend_session(
            token,
            reference,
            expectation,
            Timestamp::from_unix_millis(30),
        )
        .await
        .expect("lease");
    let staged = store
        .stage_verified_imathas_result(token, transition_with_score(lease, 0.5))
        .await
        .expect("stage");
    assert!(
        store
            .claim_imathas_result_grading_job(staged.job_id(), Timestamp::from_unix_millis(20))
            .await
            .is_err()
    );
    assert!(
        store
            .claim_imathas_result_grading_job(staged.job_id(), Timestamp::from_unix_millis(320_021))
            .await
            .is_err()
    );
    let first = store
        .claim_imathas_result_grading_job(staged.job_id(), Timestamp::from_unix_millis(30))
        .await
        .expect("claim");
    assert!(
        store
            .claim_imathas_result_grading_job(staged.job_id(), Timestamp::from_unix_millis(31))
            .await
            .is_err()
    );
    store.set_now(Timestamp::from_unix_millis(31));
    let reclaimed = store
        .claim_imathas_result_grading_job(staged.job_id(), Timestamp::from_unix_millis(40))
        .await
        .expect("reclaim");
    assert_ne!(first.capability, reclaimed.capability);
    assert!(
        store
            .commit_staged_imathas_result_grading(CommitStagedImathasResultGrading::new(
                first,
                Timestamp::from_unix_millis(31)
            ))
            .await
            .is_err()
    );
    let command =
        CommitStagedImathasResultGrading::new(reclaimed.clone(), Timestamp::from_unix_millis(31));
    let receipt = store
        .commit_staged_imathas_result_grading(command)
        .await
        .expect("commit");
    assert_eq!(receipt.grading_result().points_earned, 5.0);
    let replay = store
        .commit_staged_imathas_result_grading(CommitStagedImathasResultGrading::new(
            reclaimed,
            Timestamp::from_unix_millis(31),
        ))
        .await
        .expect("replay");
    assert_eq!(receipt.id(), replay.id());
    assert_eq!(receipt.checksum(), replay.checksum());
    assert!(
        store
            .claim_imathas_result_grading_job(staged.job_id(), Timestamp::from_unix_millis(40))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn exhausted_imathas_question_backend_grading_job_preserves_ready_evidence() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    let token = SessionTokenHash::compute(b"exhausted");
    let store =
        MemoryImathasQuestionBackendSessionStore::new(ring(), Timestamp::from_unix_millis(20));
    authorize(&store, token, account);
    let (create, expectation) = facts(account);
    let reference = store
        .create_imathas_question_backend_session(token, create)
        .await
        .expect("create");
    let lease = store
        .lease_imathas_question_backend_session(
            token,
            reference,
            expectation,
            Timestamp::from_unix_millis(30),
        )
        .await
        .expect("lease");
    let stage_command = transition(lease);
    let staged = store
        .stage_verified_imathas_result(token, stage_command.clone())
        .await
        .expect("stage");
    for now in [20_i64, 31, 42] {
        store.set_now(Timestamp::from_unix_millis(now));
        store
            .claim_imathas_result_grading_job(
                staged.job_id(),
                Timestamp::from_unix_millis(now + 10),
            )
            .await
            .expect("claim");
    }
    store.set_now(Timestamp::from_unix_millis(53));
    assert!(
        store
            .claim_imathas_result_grading_job(staged.job_id(), Timestamp::from_unix_millis(60))
            .await
            .is_err()
    );
    let replay = store
        .stage_verified_imathas_result(token, stage_command)
        .await
        .expect("ready evidence remains recoverable");
    assert_eq!(
        (
            staged.question_submission_id(),
            staged.question_submission_grading_id(),
            staged.job_id(),
        ),
        (
            replay.question_submission_id(),
            replay.question_submission_grading_id(),
            replay.job_id(),
        )
    );
}

#[test]
fn automated_grading_receipt_checksum_v1_known_vector() {
    let checksum = automated_grading_receipt_checksum_v1(
        AutomatedGradingReceiptId::from_uuid(Uuid::from_u128(1)),
        GradingResultId::from_uuid(Uuid::from_u128(2)),
        QuestionSubmissionGradingId::from_uuid(Uuid::from_u128(3)),
        question_model::QuestionSubmissionId::from_uuid(Uuid::from_u128(4)),
        QuestionAttemptId::from_uuid(Uuid::from_u128(5)),
        JobId::from_uuid(Uuid::from_u128(6)),
        ImathasQuestionBackendSessionReference::from_uuid(Uuid::from_u128(7)),
        ImathasResultTokenChecksum::from_storage_bytes([8; 32]),
        ImathasResultChecksum::from_bytes([9; 32]),
        question_model::GradingResult {
            correct: true,
            points_earned: 2.5,
            points_possible: 5.0,
        },
        Timestamp::from_unix_millis(1_700_000_000_123),
    );
    assert_eq!(
        checksum.as_bytes(),
        &[
            10, 25, 33, 104, 153, 82, 4, 94, 216, 130, 114, 44, 224, 180, 63, 65, 6, 157, 229, 13,
            165, 154, 138, 104, 46, 82, 250, 69, 124, 90, 214, 171
        ]
    );
}

#[tokio::test]
async fn changed_stage_replays_refuse_without_replacing_first_receipt() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    for changed in 0..3 {
        let token = SessionTokenHash::compute(format!("changed-{changed}").as_bytes());
        let store =
            MemoryImathasQuestionBackendSessionStore::new(ring(), Timestamp::from_unix_millis(20));
        authorize(&store, token, account);
        let (create, expectation) = facts(account);
        let reference = store
            .create_imathas_question_backend_session(token, create)
            .await
            .expect("create");
        let lease = store
            .lease_imathas_question_backend_session(
                token,
                reference,
                expectation,
                Timestamp::from_unix_millis(30),
            )
            .await
            .expect("lease");
        let first = store
            .stage_verified_imathas_result(token, transition(lease.clone()))
            .await
            .expect("first");
        let mut changed_stage = transition(lease.clone());
        match changed {
            0 => {
                changed_stage.idempotency_key =
                    ImathasResultExchangeIdempotencyKey::parse("changed-key").expect("key")
            }
            1 => {
                changed_stage.imathas_result_token_checksum =
                    ImathasResultTokenChecksum::from_verified_token(
                        &ImathasResultToken::from_server_adapter_bytes(vec![9]).expect("token"),
                    )
            }
            _ => {
                changed_stage.imathas_result =
                    ImathasResult::new(ImathasNormalizedScore::try_from_f64(0.5).expect("score"))
            }
        }
        assert_eq!(
            store
                .stage_verified_imathas_result(token, changed_stage)
                .await,
            Err(StoreError::Conflict)
        );
        let replay = store
            .stage_verified_imathas_result(token, transition(lease.clone()))
            .await
            .expect("first receipt retained");
        assert_eq!(first.job_id(), replay.job_id());
    }
}

#[test]
fn session_rejects_invalid_points_and_ungraded_reaches_only_launch() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    for points in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.0, -1.0] {
        assert!(
            preparation_with_rule(account, QuestionGradingRule::AllOrNothing { points }).is_err()
        );
    }
}

fn preparation_with_rule(
    account: AccountId,
    rule: QuestionGradingRule,
) -> Result<ImathasQuestionBackendSessionPreparationContext, StoreError> {
    ImathasQuestionBackendSessionPreparationContext::new(
        account,
        CourseId::from_uuid(Uuid::from_u128(2)),
        AssignmentId::from_uuid(Uuid::from_u128(3)),
        ImathasGradingContext::new(
            QuestionAttemptId::from_uuid(Uuid::from_u128(4)),
            QuestionRevisionReference {
                question_id: "123-4567".parse::<QuestionId>().expect("question"),
                revision_number: QuestionRevisionNumber::new(1).expect("revision"),
            },
            QuestionSeed::new(7),
        ),
        rule,
        ImathasQuestionBackendBinding::new(
            ImathasDeploymentReference::new("imathas").expect("deployment"),
            ImathasItemReference::new("item-1").expect("item"),
            ImathasProfile::new("imathas_remote_grading_v1").expect("profile"),
        ),
        SourceObjectReference {
            object: ObjectId::from_uuid(Uuid::from_u128(5)),
        },
        SourceObjectChecksum::parse("a".repeat(64)).expect("checksum"),
        ImathasResponseChecksum::from_bytes([1; 32]),
        ImathasQuestionBackendSessionChallenge::generate().expect("challenge"),
        ImathasQuestionBackendSessionAuthentication::from_server_value(format!(
            "aa.{}",
            "b".repeat(64)
        ))
        .expect("auth"),
        Timestamp::from_unix_millis(10),
        Timestamp::from_unix_millis(100),
    )
}

#[tokio::test]
async fn ungraded_session_refuses_stage_before_exchange_creation() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    let token = SessionTokenHash::compute(b"ungraded");
    let store =
        MemoryImathasQuestionBackendSessionStore::new(ring(), Timestamp::from_unix_millis(20));
    authorize(&store, token, account);
    let (create, expectation) = facts_with_rule(account, QuestionGradingRule::Ungraded);
    let reference = store
        .create_imathas_question_backend_session(token, create)
        .await
        .expect("create");
    let lease = store
        .lease_imathas_question_backend_session(
            token,
            reference,
            expectation,
            Timestamp::from_unix_millis(30),
        )
        .await
        .expect("lease");
    assert!(
        StageVerifiedImathasResult::new(
            lease.clone(),
            lease.expectation.grading_context.clone(),
            lease.expectation.authentication.clone(),
            ImathasResultExchangeIdempotencyKey::parse("ungraded").expect("key"),
            ImathasResultTokenChecksum::from_verified_token(
                &ImathasResultToken::from_server_adapter_bytes(vec![1]).expect("token")
            ),
            ImathasResult::new(ImathasNormalizedScore::try_from_f64(1.0).expect("score")),
            Timestamp::from_unix_millis(20)
        )
        .is_err()
    );
    assert!(
        store
            .lease_imathas_question_backend_session(
                token,
                reference,
                lease.expectation,
                Timestamp::from_unix_millis(30)
            )
            .await
            .is_err()
    );
}

#[test]
fn imathas_question_backend_result_token_bounds_redaction_and_checksum_are_exact() {
    assert!(ImathasResultToken::from_server_adapter_bytes(Vec::new()).is_err());
    assert!(ImathasResultToken::from_server_adapter_bytes(vec![0; 8_193]).is_err());

    let one = ImathasResultToken::from_server_adapter_bytes(vec![7]).expect("one byte");
    let maximum =
        ImathasResultToken::from_server_adapter_bytes(vec![7; 8_192]).expect("maximum bytes");
    let token =
        ImathasResultToken::from_server_adapter_bytes(b"abc".to_vec()).expect("known vector");
    let checksum = ImathasResultTokenChecksum::from_verified_token(&token);
    let restored = ImathasResultTokenChecksum::from_storage_bytes(*checksum.as_bytes());

    assert_eq!(one.as_server_adapter_bytes(), &[7]);
    assert_eq!(maximum.as_server_adapter_bytes().len(), 8_192);
    assert_eq!(
        checksum.as_bytes(),
        &[
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ]
    );
    assert_eq!(restored, checksum);
    assert!(format!("{token:?}").contains("[redacted]"));
}

#[test]
fn normalized_score_boundaries_and_result_checksum_are_fixed() {
    assert!(ImathasNormalizedScore::try_from_f64(f64::NAN).is_err());
    assert!(ImathasNormalizedScore::try_from_f64(f64::INFINITY).is_err());
    assert!(ImathasNormalizedScore::try_from_f64(-0.0).is_err());
    assert!(ImathasNormalizedScore::try_from_f64(-0.1).is_err());
    assert!(ImathasNormalizedScore::try_from_f64(1.1).is_err());
    let zero = ImathasResult::new(ImathasNormalizedScore::try_from_f64(0.0).expect("zero"));
    let one = ImathasResult::new(ImathasNormalizedScore::try_from_f64(1.0).expect("one"));
    assert_ne!(zero.checksum(), one.checksum());
    assert_eq!(
        zero.checksum().as_bytes(),
        &[
            0xdf, 0xb9, 0x00, 0x6e, 0xb4, 0xa5, 0x34, 0x4a, 0x0a, 0x78, 0x5b, 0x26, 0xad, 0x76,
            0x12, 0x85, 0x14, 0xa0, 0xaa, 0xcb, 0x60, 0xa9, 0x76, 0xd3, 0xf3, 0xc3, 0x68, 0x02,
            0x0e, 0x20, 0x51, 0x1b,
        ]
    );
    assert_eq!(
        zero.checksum(),
        ImathasResult::new(ImathasNormalizedScore::try_from_f64(0.0).expect("zero")).checksum()
    );
    assert!(format!("{zero:?}").contains("[redacted]"));
}

#[test]
fn exact_imathas_question_backend_grading_rule_translation_is_closed() {
    let partial = ImathasResult::new(ImathasNormalizedScore::try_from_f64(0.5).expect("score"));
    assert_eq!(
        derive_imathas_question_backend_grading_result(
            &partial,
            &QuestionGradingRule::AllOrNothing { points: 4.0 }
        )
        .expect("result")
        .points_earned,
        0.0
    );
    assert_eq!(
        derive_imathas_question_backend_grading_result(
            &partial,
            &QuestionGradingRule::PartialCredit { points: 4.0 }
        )
        .expect("result")
        .points_earned,
        2.0
    );
    assert!(
        derive_imathas_question_backend_grading_result(&partial, &QuestionGradingRule::Ungraded)
            .is_err()
    );
    for points in [f64::NAN, f64::INFINITY, -0.0, -1.0] {
        assert!(
            validate_question_grading_rule(&QuestionGradingRule::AllOrNothing { points }).is_err()
        );
    }
}
fn authorize(
    store: &MemoryImathasQuestionBackendSessionStore,
    token: SessionTokenHash,
    account: AccountId,
) {
    store.install_authenticated_session(token, account);
    store.install_active_student_authorization(
        account,
        CourseId::from_uuid(Uuid::from_u128(2)),
        QuestionAttemptId::from_uuid(Uuid::from_u128(4)),
    );
}

#[tokio::test]
async fn memory_oracle_restores_exact_context_and_consumes_through_exchange_transition() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    let token = SessionTokenHash::compute(b"session");
    let store =
        MemoryImathasQuestionBackendSessionStore::new(ring(), Timestamp::from_unix_millis(20));
    authorize(&store, token, account);
    let (create, expectation) = facts(account);
    let (session, _) =
        facts(account)
            .0
            .into_session(ImathasQuestionBackendSessionReference::from_uuid(
                Uuid::from_u128(98),
            ));
    assert!(!format!("{session:?}").contains("imathas_result_token_checksum"));
    let reference = store
        .create_imathas_question_backend_session(token, create)
        .await
        .expect("create");
    assert_eq!(
        store
            .load_imathas_question_backend_session(token, reference, expectation.clone())
            .await
            .expect("load")
            .imathas_question_backend_state()
            .as_bytes(),
        &[1, 2, 3]
    );
    let lease = store
        .lease_imathas_question_backend_session(
            token,
            reference,
            expectation.clone(),
            Timestamp::from_unix_millis(30),
        )
        .await
        .expect("lease");
    let transition = transition(lease);
    let expected_checksum = transition.imathas_result_token_checksum;
    assert!(transition.lease().store_predicate() == expectation.store_predicate());
    store
        .stage_verified_imathas_result(token, transition)
        .await
        .expect("consume");
    assert_eq!(
        store.imathas_result_token_checksum(reference),
        Some(expected_checksum)
    );
    assert_eq!(
        store
            .load_imathas_question_backend_session(token, reference, expectation)
            .await,
        Err(StoreError::Conflict)
    );
}

#[tokio::test]
async fn memory_exchange_checksum_is_single_use() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    let token = SessionTokenHash::compute(b"single-use");
    let store =
        MemoryImathasQuestionBackendSessionStore::new(ring(), Timestamp::from_unix_millis(20));
    authorize(&store, token, account);
    let (create, expectation) = facts(account);
    let reference = store
        .create_imathas_question_backend_session(token, create)
        .await
        .expect("create");
    let lease = store
        .lease_imathas_question_backend_session(
            token,
            reference,
            expectation,
            Timestamp::from_unix_millis(30),
        )
        .await
        .expect("lease");

    let first = store
        .stage_verified_imathas_result(token, transition(lease.clone()))
        .await
        .expect("first verified exchange");
    let replay = store
        .stage_verified_imathas_result(token, transition(lease))
        .await
        .expect("exact replay");
    assert_eq!(first.job_id(), replay.job_id());
}

#[tokio::test]
async fn memory_oracle_refuses_wrong_restore_context_and_revoked_student_authorization() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    let token = SessionTokenHash::compute(b"owner");
    let store =
        MemoryImathasQuestionBackendSessionStore::new(ring(), Timestamp::from_unix_millis(20));
    authorize(&store, token, account);
    let (create, expectation) = facts(account);
    let reference = store
        .create_imathas_question_backend_session(token, create)
        .await
        .expect("create");
    let (_, wrong) = facts(AccountId::from_uuid(Uuid::from_u128(99)));
    assert_eq!(
        store
            .load_imathas_question_backend_session(token, reference, wrong)
            .await,
        Err(StoreError::Forbidden)
    );
    let (_, wrong) = facts(AccountId::from_uuid(Uuid::from_u128(99)));
    assert_eq!(
        store
            .lease_imathas_question_backend_session(
                token,
                reference,
                wrong,
                Timestamp::from_unix_millis(30),
            )
            .await,
        Err(StoreError::Forbidden)
    );
    store.revoke_active_student_authorization(
        account,
        CourseId::from_uuid(Uuid::from_u128(2)),
        QuestionAttemptId::from_uuid(Uuid::from_u128(4)),
    );
    assert_eq!(
        store
            .load_imathas_question_backend_session(token, reference, expectation)
            .await,
        Err(StoreError::Forbidden)
    );
}

#[tokio::test]
async fn memory_oracle_refuses_every_changed_imathas_question_backend_grading_context_fact() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    let token = SessionTokenHash::compute(b"owner");
    let store =
        MemoryImathasQuestionBackendSessionStore::new(ring(), Timestamp::from_unix_millis(20));
    authorize(&store, token, account);
    let (create, expectation) = facts(account);
    let reference = store
        .create_imathas_question_backend_session(token, create)
        .await
        .expect("create");

    let replacement_revision = QuestionRevisionReference {
        question_id: "123-4568".parse::<QuestionId>().expect("question ID"),
        revision_number: QuestionRevisionNumber::new(2).expect("revision"),
    };
    let contexts = [
        ImathasGradingContext::new(
            QuestionAttemptId::from_uuid(Uuid::from_u128(40)),
            expectation.grading_context.question_revision().clone(),
            expectation.grading_context.question_seed(),
        ),
        ImathasGradingContext::new(
            expectation.grading_context.question_attempt(),
            replacement_revision,
            expectation.grading_context.question_seed(),
        ),
        ImathasGradingContext::new(
            expectation.grading_context.question_attempt(),
            expectation.grading_context.question_revision().clone(),
            QuestionSeed::new(70),
        ),
    ];

    for grading_context in contexts {
        let mut wrong = expectation.clone();
        wrong.grading_context = grading_context;
        assert_eq!(
            store
                .load_imathas_question_backend_session(token, reference, wrong)
                .await,
            Err(StoreError::Forbidden)
        );
    }
}

#[test]
fn session_validity_interval_starts_at_issue_time() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    let (create, _) = facts(account);
    let (session, _) = create.into_session(ImathasQuestionBackendSessionReference::from_uuid(
        Uuid::from_u128(99),
    ));

    assert_eq!(
        session.active_at(Timestamp::from_unix_millis(9)),
        Err(StoreError::Conflict)
    );
    assert_eq!(session.active_at(Timestamp::from_unix_millis(10)), Ok(()));
    assert_eq!(
        session.active_at(Timestamp::from_unix_millis(100)),
        Err(StoreError::Conflict)
    );
}

#[test]
fn lease_storage_retains_the_complete_restore_expectation() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    let (_, expectation) = facts(account);
    let lease = ImathasQuestionBackendSessionLease::from_server_capability(
        ImathasQuestionBackendSessionReference::from_uuid(Uuid::from_u128(99)),
        [7; 32],
        Timestamp::from_unix_millis(20),
        expectation.clone(),
    );
    let restore = lease.storage_parts().restore;
    let expected = expectation.storage_parts();

    assert_eq!(
        restore.imathas_question_backend_binding,
        expected.imathas_question_backend_binding
    );
    assert_eq!(restore.grading_context, expected.grading_context);
    assert_eq!(
        restore.imathas_launch_binding_checksum,
        expected.imathas_launch_binding_checksum
    );
}

#[test]
fn imathas_item_reference_uses_the_question_model_contract() {
    assert!(ImathasItemReference::new("a".repeat(128)).is_ok());
    assert!(ImathasItemReference::new("a".repeat(129)).is_err());
    assert!(ImathasItemReference::new("item-1").is_ok());
    assert!(ImathasItemReference::new("item:1").is_err());
    assert!(ImathasItemReference::new("item..1").is_err());
}

struct FixedNonce([u8; IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES]);
impl ImathasQuestionBackendStateNonceSource for FixedNonce {
    fn fill_nonce(
        &self,
        nonce: &mut [u8; IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES],
    ) -> Result<(), StoreError> {
        *nonce = self.0;
        Ok(())
    }
}

#[test]
fn cipher_binds_every_immutable_fact_with_deterministic_nonces_and_redaction() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    let (create, _) = facts(account);
    let (session, plaintext) = create.into_session(
        ImathasQuestionBackendSessionReference::from_uuid(Uuid::from_u128(7)),
    );
    let key_ring = ring();
    let source = FixedNonce([9; IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES]);
    let cipher = ImathasQuestionBackendStateCipher::seal_with_nonce_source(
        &key_ring, &session, &plaintext, &source,
    )
    .expect("seal");
    assert_eq!(
        cipher.nonce(),
        &[9; IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES]
    );
    assert!(format!("{cipher:?}").contains("[redacted]"));
    let mut altered = session.clone();
    altered.grading_context.question_seed = QuestionSeed::new(8);
    assert!(cipher.open(&key_ring, &altered).is_err());
    altered = session.clone();
    altered.challenge = ImathasQuestionBackendSessionChallenge::generate().expect("challenge");
    assert!(cipher.open(&key_ring, &altered).is_err());
    let wrong = ImathasQuestionBackendStateKeyRing::new(
        ImathasQuestionBackendStateKeyId::parse("other").expect("key ID"),
        [8; 32],
        [],
    )
    .expect("ring");
    assert!(cipher.open(&wrong, &session).is_err());
    let mut tampered = cipher.clone();
    tampered.ciphertext[0] ^= 1;
    assert!(tampered.open(&key_ring, &session).is_err());
}

#[test]
fn launch_challenge_generation_is_nonzero_and_redacted() {
    let challenge = ImathasQuestionBackendSessionChallenge::generate().expect("challenge");

    assert_eq!(challenge.as_bytes().len(), 32);
    assert!(challenge.as_bytes().iter().any(|byte| *byte != 0));
    assert_eq!(
        format!("{challenge:?}"),
        "ImathasQuestionBackendSessionChallenge([redacted])"
    );
}

#[test]
fn launch_challenge_rejects_an_invalid_stored_value() {
    assert_eq!(
        ImathasQuestionBackendSessionChallenge::from_storage_bytes([0; 32]),
        Err(StoreError::InvalidRecord(
            "iMathAS Session Challenge must not be all zero".into()
        ))
    );
}
