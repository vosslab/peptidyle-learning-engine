use super::*;
use question_model::generation::QuestionSeed;
use question_model::{
    AccountId, AssessmentId, CourseId, ImathasDeploymentReference, ImathasItemReference,
    ImathasProfile, ImathasQuestionBackendBinding, ObjectId, QuestionAttemptId, QuestionId,
    QuestionRevisionNumber, QuestionRevisionReference, SourceObjectChecksum, SourceObjectReference,
    Timestamp,
};
use uuid::Uuid;
fn facts(
    account: AccountId,
) -> (
    ImathasQuestionBackendSessionCreate,
    ImathasQuestionBackendSessionRestoreExpectation,
) {
    let course = CourseId::from_uuid(Uuid::from_u128(2));
    let assessment = AssessmentId::from_uuid(Uuid::from_u128(3));
    let attempt = QuestionAttemptId::from_uuid(Uuid::from_u128(4));
    let revision = QuestionRevisionReference {
        question_id: "1234-X567".parse::<QuestionId>().expect("question ID"),
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
        assessment,
        grading_context.clone(),
        imathas_question_backend_binding.clone(),
        source.clone(),
        checksum.clone(),
        digest.clone(),
        authentication.clone(),
    );
    let preparation = ImathasQuestionBackendSessionPreparationContext::new(
        account,
        course,
        assessment,
        grading_context,
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
            question_id: "1234-X567".parse::<QuestionId>().expect("question ID"),
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
        "1234-X567"
    );
    assert_eq!(context.question_seed(), QuestionSeed::new(7));
    assert_eq!(
        context.authentication_payload_v1(),
        vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, b'1', b'2', b'3', b'4', b'X', b'5',
            b'6', b'7', 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 7,
        ]
    );
    assert_eq!(format!("{context:?}"), "ImathasGradingContext([redacted])");
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
    assert!(format!("{token:?}").contains("[redacted]"));
}

#[test]
fn normalized_score_boundaries_are_fixed() {
    assert!(ImathasNormalizedScore::try_from_f64(f64::NAN).is_err());
    assert!(ImathasNormalizedScore::try_from_f64(f64::INFINITY).is_err());
    assert!(ImathasNormalizedScore::try_from_f64(-0.0).is_err());
    assert!(ImathasNormalizedScore::try_from_f64(-0.1).is_err());
    assert!(ImathasNormalizedScore::try_from_f64(1.1).is_err());
    let zero = ImathasResult::new(ImathasNormalizedScore::try_from_f64(0.0).expect("zero"));
    assert!(format!("{zero:?}").contains("[redacted]"));
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
async fn memory_oracle_restores_exact_backend_state() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    let token = SessionTokenHash::compute(b"session");
    let store =
        MemoryImathasQuestionBackendSessionStore::new(ring(), Timestamp::from_unix_millis(20));
    authorize(&store, token, account);
    let (create, expectation) = facts(account);
    let reference = store
        .create_imathas_question_backend_session(token, create)
        .await
        .expect("create");
    assert_eq!(
        store
            .load_imathas_question_backend_session(token, reference, expectation)
            .await
            .expect("load")
            .imathas_question_backend_state()
            .as_bytes(),
        &[1, 2, 3]
    );
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
        question_id: "1234-X568".parse::<QuestionId>().expect("question ID"),
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
fn encrypted_state_aad_uses_the_compact_question_id() {
    let account = AccountId::from_uuid(Uuid::from_u128(1));
    let (create, _) = facts(account);
    let (session, _) = create.into_session(ImathasQuestionBackendSessionReference::from_uuid(
        Uuid::from_u128(7),
    ));

    let aad = protected_state::imathas_question_backend_state_aad(&session);
    let mut cursor = 1;
    let mut fields = Vec::new();
    while cursor < aad.len() {
        let length = u32::from_be_bytes(
            aad[cursor..cursor + 4]
                .try_into()
                .expect("AAD field length"),
        ) as usize;
        cursor += 4;
        fields.push(&aad[cursor..cursor + length]);
        cursor += length;
    }

    assert_eq!(fields[5], b"1234X567");
    assert_ne!(fields[5], b"1234-X567");
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
