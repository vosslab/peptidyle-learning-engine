#![cfg(feature = "postgres")]

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use learning_data_access::postgres::{
    PostgresImathasQuestionBackendSessionStore, ProductionLoginProfile, lazy_pool,
    local_development_pool,
};
use learning_data_access::{
    CommitStagedImathasResultGrading, ImathasGradingContext, ImathasLaunchBindingChecksum,
    ImathasNormalizedScore, ImathasQuestionBackendSessionAuthentication,
    ImathasQuestionBackendSessionChallenge, ImathasQuestionBackendSessionPreparationContext,
    ImathasQuestionBackendSessionRestoreExpectation, ImathasQuestionBackendSessionStore,
    ImathasQuestionBackendStateKeyId, ImathasQuestionBackendStateKeyRing,
    ImathasQuestionBackendStatePlaintext, ImathasResponseChecksum, ImathasResult,
    ImathasResultToken, ImathasResultTokenChecksum, SessionTokenHash, StageVerifiedImathasResult,
};
use question_model::generation::QuestionSeed;
use question_model::{
    AccountId, AssignmentId, CourseId, ImathasDeploymentReference, ImathasItemReference,
    ImathasProfile, ImathasQuestionBackendBinding, ObjectId, QuestionAttemptId, QuestionId,
    QuestionRevisionNumber, QuestionRevisionReference, SourceObjectChecksum, SourceObjectReference,
    Timestamp,
};
use sqlx::Executor;
use uuid::Uuid;

const DATABASE_URL_ENV: &str = "PLE_IMATHAS_QUESTION_BACKEND_SESSION_DATABASE_URL";
const GRADING_WORKER_DATABASE_URL_ENV: &str =
    "PLE_IMATHAS_QUESTION_BACKEND_SESSION_GRADING_WORKER_DATABASE_URL";
const ADMIN_DATABASE_URL_ENV: &str = "PLE_IMATHAS_QUESTION_BACKEND_SESSION_ADMIN_DATABASE_URL";
const STUDENT_ACCOUNT: u128 = 0x101;
const STUDENT_MEMBERSHIP: u128 = 0x108;
const COURSE: u128 = 0x105;
const ASSIGNMENT: u128 = 0x110;
const QUESTION_ATTEMPT: u128 = 0xf205;
const PERSISTENCE_QUESTION_ATTEMPT: u128 = 0xf207;
const ELIGIBLE_STATISTICS_QUESTION_ATTEMPT: u128 = 0xf214;
const INELIGIBLE_STATISTICS_QUESTION_ATTEMPT: u128 = 0xf208;
const SOURCE_OBJECT: u128 = 0xf202;
const ORACLE_MEMBERSHIP_END_EVENT: u128 = 0xf204;

struct Oracle {
    token: SessionTokenHash,
    account: AccountId,
    store: PostgresImathasQuestionBackendSessionStore,
    grading_worker_store: PostgresImathasQuestionBackendSessionStore,
    admin: sqlx::postgres::PgPool,
}

fn now() -> Timestamp {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_millis();
    Timestamp::from_unix_millis(i64::try_from(millis).expect("millisecond timestamp fits i64"))
}

fn key_ring(byte: u8) -> ImathasQuestionBackendStateKeyRing {
    ImathasQuestionBackendStateKeyRing::new(
        ImathasQuestionBackendStateKeyId::parse("imathas-question-backend-oracle-2026")
            .expect("key ID"),
        [byte; 32],
        [],
    )
    .expect("key ring")
}

async fn oracle() -> Oracle {
    let database_url = std::env::var(DATABASE_URL_ENV)
        .expect("PostgreSQL Migration Acceptance Runtime database URL");
    let grading_worker_database_url = std::env::var(GRADING_WORKER_DATABASE_URL_ENV)
        .expect("PostgreSQL Migration Acceptance Runtime grading-worker database URL");
    let admin_url = std::env::var(ADMIN_DATABASE_URL_ENV)
        .expect("PostgreSQL Migration Acceptance Runtime admin database URL");
    let pool = local_development_pool(&database_url, ProductionLoginProfile::Api)
        .expect("production-shaped API pool");
    let current_user: String = sqlx::query_scalar("SELECT current_user")
        .fetch_one(&pool)
        .await
        .expect("acquire the attested production-shaped API connection");
    assert_eq!(
        current_user, "ple_api_login",
        "the oracle must use the exact production API login"
    );
    let grading_worker_pool = local_development_pool(
        &grading_worker_database_url,
        ProductionLoginProfile::ImathasQuestionBackendGradingWorker,
    )
    .expect("production-shaped grading-worker pool");
    let grading_worker_current_user: String = sqlx::query_scalar("SELECT current_user")
        .fetch_one(&grading_worker_pool)
        .await
        .expect("acquire the attested production-shaped grading-worker connection");
    assert_eq!(
        grading_worker_current_user, "ple_worker_login",
        "the oracle must use the exact production grading-worker login"
    );
    let admin = lazy_pool(&admin_url).expect("admin fixture pool");
    Oracle {
        token: SessionTokenHash::compute(&[0x42; 32]),
        account: AccountId::from_uuid(Uuid::from_u128(STUDENT_ACCOUNT)),
        store: PostgresImathasQuestionBackendSessionStore::new(pool, Arc::new(key_ring(9))),
        grading_worker_store: PostgresImathasQuestionBackendSessionStore::new(
            grading_worker_pool,
            Arc::new(key_ring(9)),
        ),
        admin,
    }
}

fn facts(
    account: AccountId,
    issued_at: Timestamp,
    expires_at: Timestamp,
    imathas_item_reference: &str,
    profile: &str,
    seed: u64,
    digest: char,
) -> (
    ImathasQuestionBackendSessionPreparationContext,
    ImathasQuestionBackendSessionRestoreExpectation,
) {
    facts_with_grading_context(
        account,
        issued_at,
        expires_at,
        ImathasGradingContext::new(
            QuestionAttemptId::from_uuid(Uuid::from_u128(QUESTION_ATTEMPT)),
            QuestionRevisionReference {
                question_id: "ABC-DEF0".parse::<QuestionId>().expect("question ID"),
                revision_number: QuestionRevisionNumber::new(1).expect("revision number"),
            },
            QuestionSeed::new(seed),
        ),
        imathas_item_reference,
        profile,
        digest,
    )
}

fn facts_with_grading_context(
    account: AccountId,
    issued_at: Timestamp,
    expires_at: Timestamp,
    grading_context: ImathasGradingContext,
    imathas_item_reference: &str,
    profile: &str,
    digest: char,
) -> (
    ImathasQuestionBackendSessionPreparationContext,
    ImathasQuestionBackendSessionRestoreExpectation,
) {
    let course = CourseId::from_uuid(Uuid::from_u128(COURSE));
    let assignment = AssignmentId::from_uuid(Uuid::from_u128(ASSIGNMENT));
    let imathas_question_backend_binding = ImathasQuestionBackendBinding::new(
        ImathasDeploymentReference::new("self-hosted-imathas").expect("deployment"),
        ImathasItemReference::new(imathas_item_reference).expect("imathas item"),
        ImathasProfile::new(profile).expect("profile"),
    );
    let source = SourceObjectReference {
        object: ObjectId::from_uuid(Uuid::from_u128(SOURCE_OBJECT)),
    };
    let checksum = SourceObjectChecksum::parse("aa".repeat(32)).expect("source checksum");
    let imathas_launch_binding_checksum =
        ImathasLaunchBindingChecksum::parse(digest.to_string().repeat(64))
            .expect("iMathAS Launch Binding Checksum");
    let authentication = ImathasQuestionBackendSessionAuthentication::from_server_value(format!(
        "aa.{}",
        "b".repeat(64)
    ))
    .expect("authentication");
    let expectation = ImathasQuestionBackendSessionRestoreExpectation::new(
        account,
        course,
        assignment,
        grading_context.clone(),
        imathas_question_backend_binding.clone(),
        source.clone(),
        checksum.clone(),
        imathas_launch_binding_checksum.clone(),
        authentication.clone(),
    );
    let preparation = ImathasQuestionBackendSessionPreparationContext::new(
        account,
        course,
        assignment,
        grading_context,
        imathas_question_backend_binding,
        source,
        checksum,
        ImathasResponseChecksum::from_bytes([1; 32]),
        ImathasQuestionBackendSessionChallenge::generate().expect("challenge"),
        authentication,
        issued_at,
        expires_at,
    )
    .expect("preparation context");
    (preparation, expectation)
}

fn grading_context(
    question_attempt: u128,
    question_id: &str,
    revision_number: u32,
    question_seed: u64,
) -> ImathasGradingContext {
    ImathasGradingContext::new(
        QuestionAttemptId::from_uuid(Uuid::from_u128(question_attempt)),
        QuestionRevisionReference {
            question_id: question_id.parse::<QuestionId>().expect("question ID"),
            revision_number: QuestionRevisionNumber::new(revision_number).expect("revision number"),
        },
        QuestionSeed::new(question_seed),
    )
}

fn create(
    account: AccountId,
    issued_at: Timestamp,
    expires_at: Timestamp,
) -> (
    learning_data_access::ImathasQuestionBackendSessionCreate,
    ImathasQuestionBackendSessionRestoreExpectation,
) {
    create_for_attempt(account, QUESTION_ATTEMPT, issued_at, expires_at)
}

fn create_for_attempt(
    account: AccountId,
    question_attempt: u128,
    issued_at: Timestamp,
    expires_at: Timestamp,
) -> (
    learning_data_access::ImathasQuestionBackendSessionCreate,
    ImathasQuestionBackendSessionRestoreExpectation,
) {
    let (preparation, expectation) = facts_with_grading_context(
        account,
        issued_at,
        expires_at,
        grading_context(question_attempt, "ABC-DEF0", 1, 1),
        "oracle-item",
        "imathas_remote_grading_v1",
        'c',
    );
    (
        preparation
            .complete(
                ImathasLaunchBindingChecksum::parse("c".repeat(64))
                    .expect("iMathAS Launch Binding Checksum"),
                ImathasQuestionBackendStatePlaintext::from_versioned_adapter_bytes(vec![1, 2, 3])
                    .expect("imathas state"),
            )
            .expect("create"),
        expectation,
    )
}

fn transition(
    lease: learning_data_access::ImathasQuestionBackendSessionLease,
    transitioned_at: Timestamp,
) -> StageVerifiedImathasResult {
    transition_with_score(lease, transitioned_at, 1.0)
}

fn transition_with_score(
    lease: learning_data_access::ImathasQuestionBackendSessionLease,
    transitioned_at: Timestamp,
    score: f64,
) -> StageVerifiedImathasResult {
    let imathas_result_token =
        ImathasResultToken::from_server_adapter_bytes(b"postgres oracle result".to_vec())
            .expect("bounded imathas result token");
    let grading_context = lease.grading_context();
    let authentication = lease.launch_session_authentication();
    StageVerifiedImathasResult::new(
        lease,
        grading_context,
        authentication,
        ImathasResultTokenChecksum::from_verified_token(&imathas_result_token),
        ImathasResult::new(ImathasNormalizedScore::try_from_f64(score).expect("score")),
        transitioned_at,
    )
    .expect("verified result stage")
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 iMathAS Question Backend Session oracle"]
async fn postgres_store_persists_opens_and_consumes_one_exact_session() {
    let oracle = oracle().await;
    oracle.admin.execute("UPDATE ple_private.issued_question SET point_value = 2.5, scoring_rule = 'full_credit', question_statistics_eligibility = false WHERE issued_question_id = '00000000-0000-5000-8000-000000000115'").await.expect("set Full Credit issued scoring fixture");
    let issued_at = Timestamp::from_unix_millis(now().as_unix_millis() - 5_000);
    let expires_at = Timestamp::from_unix_millis(issued_at.as_unix_millis() + 180_000);
    let (baseline_create, expectation) = create_for_attempt(
        oracle.account,
        PERSISTENCE_QUESTION_ATTEMPT,
        issued_at,
        expires_at,
    );
    let reference = oracle
        .store
        .create_imathas_question_backend_session(oracle.token, baseline_create)
        .await
        .expect("create through authenticated Store");
    assert_eq!(
        oracle
            .store
            .load_imathas_question_backend_session(oracle.token, reference, expectation.clone())
            .await
            .expect("exact restore and AEAD open")
            .imathas_question_backend_state()
            .as_bytes(),
        &[1, 2, 3]
    );
    let lease = oracle
        .store
        .lease_imathas_question_backend_session(
            oracle.token,
            reference,
            expectation.clone(),
            Timestamp::from_unix_millis(issued_at.as_unix_millis() + 90_000),
        )
        .await
        .expect("lease");
    let stage = oracle
        .store
        .stage_verified_imathas_result(oracle.token, transition(lease, now()))
        .await
        .expect("verified result stage");
    let claim = oracle
        .grading_worker_store
        .claim_imathas_result_grading_job(
            stage.job_id(),
            Timestamp::from_unix_millis(now().as_unix_millis() + 120_000),
        )
        .await
        .expect("claim Full Credit grading Job");
    let receipt = oracle
        .grading_worker_store
        .commit_staged_imathas_result_grading(CommitStagedImathasResultGrading::new(claim, now()))
        .await
        .expect("commit Full Credit grade");
    assert_eq!(receipt.grading_result().points_earned, 2.5);
    assert!(
        oracle
            .store
            .load_imathas_question_backend_session(oracle.token, reference, expectation)
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 iMathAS Question Backend Session oracle"]
async fn postgres_store_commits_statistics_from_the_stored_grade_exactly_once() {
    let oracle = oracle().await;
    oracle.admin.execute("UPDATE ple_private.issued_question SET point_value = 2.5, scoring_rule = CASE issued_question_id WHEN '00000000-0000-0000-0000-00000000f213'::uuid THEN 'normal' WHEN '00000000-0000-0000-0000-00000000f209'::uuid THEN 'excluded' END WHERE issued_question_id IN ('00000000-0000-0000-0000-00000000f213', '00000000-0000-0000-0000-00000000f209')").await.expect("set issued scoring fixtures");
    let issued_at = Timestamp::from_unix_millis(now().as_unix_millis() - 5_000);
    let expires_at = Timestamp::from_unix_millis(issued_at.as_unix_millis() + 180_000);
    let (eligible_create, eligible_expectation) = create_for_attempt(
        oracle.account,
        ELIGIBLE_STATISTICS_QUESTION_ATTEMPT,
        issued_at,
        expires_at,
    );
    let eligible_reference = oracle
        .store
        .create_imathas_question_backend_session(oracle.token, eligible_create)
        .await
        .expect("create eligible Session");
    let eligible_lease = oracle
        .store
        .lease_imathas_question_backend_session(
            oracle.token,
            eligible_reference,
            eligible_expectation,
            Timestamp::from_unix_millis(now().as_unix_millis() + 120_000),
        )
        .await
        .expect("lease eligible grading Job");
    let eligible_stage = oracle
        .store
        .stage_verified_imathas_result(
            oracle.token,
            transition_with_score(eligible_lease.clone(), now(), 0.4),
        )
        .await
        .expect("stage eligible verified result");
    let eligible_claim = oracle
        .grading_worker_store
        .claim_imathas_result_grading_job(
            eligible_stage.job_id(),
            Timestamp::from_unix_millis(now().as_unix_millis() + 120_000),
        )
        .await
        .expect("claim eligible grading Job");
    let eligible_receipt = oracle
        .grading_worker_store
        .commit_staged_imathas_result_grading(CommitStagedImathasResultGrading::new(
            eligible_claim.clone(),
            now(),
        ))
        .await
        .expect("commit eligible grading");
    assert!(!eligible_receipt.grading_result().correct);
    assert_eq!(eligible_receipt.grading_result().points_possible, 2.5);
    assert_eq!(eligible_receipt.grading_result().points_earned, 1.0);
    let stored_grade_matches_observation: bool = sqlx::query_scalar(
        "SELECT observation.correct = result.correct \
         FROM ple_private.question_statistics_observation_receipt AS observation \
         JOIN ple_audit.automated_grading_receipt AS receipt \
           ON receipt.automated_grading_receipt_id = observation.automated_grading_receipt_id \
         JOIN ple_private.grading_result AS result \
           ON result.grading_result_id = receipt.grading_result_id \
         WHERE observation.automated_grading_receipt_id = $1",
    )
    .bind(eligible_receipt.id().as_uuid())
    .fetch_one(&oracle.admin)
    .await
    .expect("eligible observation binds the stored grading result");
    assert!(stored_grade_matches_observation);
    let eligible_counts: (i64, i64) = sqlx::query_as(
        "SELECT accepted_graded_attempt_count, correct_count \
         FROM ple_data.question_revision_statistics \
         WHERE question_id = 'ABC-DEF0' AND revision_number = 1",
    )
    .fetch_one(&oracle.admin)
    .await
    .expect("eligible statistics counters");
    assert_eq!(eligible_counts, (1, 0));
    let replay = oracle
        .grading_worker_store
        .commit_staged_imathas_result_grading(CommitStagedImathasResultGrading::new(
            eligible_claim,
            now(),
        ))
        .await
        .expect("exact committed replay");
    assert_eq!(eligible_receipt.id(), replay.id());
    assert_eq!(eligible_receipt.checksum(), replay.checksum());
    let replay_counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT statistics.accepted_graded_attempt_count, statistics.correct_count, \
                (SELECT count(*) FROM ple_private.question_statistics_observation_receipt) \
         FROM ple_data.question_revision_statistics AS statistics \
         WHERE statistics.question_id = 'ABC-DEF0' AND statistics.revision_number = 1",
    )
    .fetch_one(&oracle.admin)
    .await
    .expect("replay leaves statistics unchanged");
    assert_eq!(replay_counts, (1, 0, 1));

    let (ineligible_create, ineligible_expectation) = create_for_attempt(
        oracle.account,
        INELIGIBLE_STATISTICS_QUESTION_ATTEMPT,
        issued_at,
        expires_at,
    );
    let ineligible_reference = oracle
        .store
        .create_imathas_question_backend_session(oracle.token, ineligible_create)
        .await
        .expect("create ineligible Session");
    let ineligible_lease = oracle
        .store
        .lease_imathas_question_backend_session(
            oracle.token,
            ineligible_reference,
            ineligible_expectation,
            Timestamp::from_unix_millis(now().as_unix_millis() + 120_000),
        )
        .await
        .expect("lease ineligible grading Job");
    let ineligible_stage = oracle
        .store
        .stage_verified_imathas_result(
            oracle.token,
            transition_with_score(ineligible_lease, now(), 0.4),
        )
        .await
        .expect("stage ineligible verified result");
    let ineligible_claim = oracle
        .grading_worker_store
        .claim_imathas_result_grading_job(
            ineligible_stage.job_id(),
            Timestamp::from_unix_millis(now().as_unix_millis() + 120_000),
        )
        .await
        .expect("claim ineligible grading Job");
    let ineligible_receipt = oracle
        .grading_worker_store
        .commit_staged_imathas_result_grading(CommitStagedImathasResultGrading::new(
            ineligible_claim,
            now(),
        ))
        .await
        .expect("commit ineligible grading");
    assert_eq!(ineligible_receipt.grading_result().points_earned, 0.0);
    let ineligible_counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT statistics.accepted_graded_attempt_count, statistics.correct_count, \
                (SELECT count(*) FROM ple_private.question_statistics_observation_receipt) \
         FROM ple_data.question_revision_statistics AS statistics \
         WHERE statistics.question_id = 'ABC-DEF0' AND statistics.revision_number = 1",
    )
    .fetch_one(&oracle.admin)
    .await
    .expect("non-statistics issued scoring leaves statistics unchanged");
    assert_eq!(ineligible_counts, (1, 0, 1));
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 iMathAS Question Backend Session oracle"]
async fn postgres_store_rejects_context_lifecycle_and_authority_bypasses() {
    let oracle = oracle().await;
    let issued_at = Timestamp::from_unix_millis(now().as_unix_millis() - 5_000);
    let expires_at = Timestamp::from_unix_millis(issued_at.as_unix_millis() + 180_000);
    let (baseline_create, expectation) = create(oracle.account, issued_at, expires_at);
    let reference = oracle
        .store
        .create_imathas_question_backend_session(oracle.token, baseline_create)
        .await
        .expect("create baseline session");
    let mut failures = Vec::new();
    for (label, hostile) in [
        (
            "Question Attempt",
            facts_with_grading_context(
                oracle.account,
                issued_at,
                expires_at,
                grading_context(0xf206, "ABC-DEF0", 1, 1),
                "oracle-item",
                "imathas_remote_grading_v1",
                'c',
            )
            .1,
        ),
        (
            "Question ID",
            facts_with_grading_context(
                oracle.account,
                issued_at,
                expires_at,
                grading_context(QUESTION_ATTEMPT, "ABC-DEF1", 1, 1),
                "oracle-item",
                "imathas_remote_grading_v1",
                'c',
            )
            .1,
        ),
        (
            "Question Revision Number",
            facts_with_grading_context(
                oracle.account,
                issued_at,
                expires_at,
                grading_context(QUESTION_ATTEMPT, "ABC-DEF0", 2, 1),
                "oracle-item",
                "imathas_remote_grading_v1",
                'c',
            )
            .1,
        ),
        (
            "imathas item",
            facts(
                oracle.account,
                issued_at,
                expires_at,
                "other-item",
                "imathas_remote_grading_v1",
                1,
                'c',
            )
            .1,
        ),
        (
            "integration profile",
            facts(
                oracle.account,
                issued_at,
                expires_at,
                "oracle-item",
                "other-profile",
                1,
                'c',
            )
            .1,
        ),
        (
            "Question Seed",
            facts(
                oracle.account,
                issued_at,
                expires_at,
                "oracle-item",
                "imathas_remote_grading_v1",
                2,
                'c',
            )
            .1,
        ),
        (
            "iMathAS Launch Binding Checksum",
            facts(
                oracle.account,
                issued_at,
                expires_at,
                "oracle-item",
                "imathas_remote_grading_v1",
                1,
                'd',
            )
            .1,
        ),
    ] {
        let (hostile_baseline_create, _) = create(oracle.account, issued_at, expires_at);
        let hostile_reference = oracle
            .store
            .create_imathas_question_backend_session(oracle.token, hostile_baseline_create)
            .await
            .expect("create hostile-context baseline session");
        if let Ok(lease) = oracle
            .store
            .lease_imathas_question_backend_session(
                oracle.token,
                hostile_reference,
                hostile,
                Timestamp::from_unix_millis(issued_at.as_unix_millis() + 90_000),
            )
            .await
        {
            failures.push(format!("lease accepted hostile {label} restore facts"));
            if oracle
                .store
                .stage_verified_imathas_result(
                    oracle.token,
                    transition(
                        lease,
                        Timestamp::from_unix_millis(issued_at.as_unix_millis() + 30_000),
                    ),
                )
                .await
                .is_ok()
            {
                failures.push(format!("consume accepted hostile {label} restore facts"));
            }
        }
    }
    let (_, wrong_account) = facts(
        AccountId::from_uuid(Uuid::from_u128(0x999)),
        issued_at,
        expires_at,
        "oracle-item",
        "imathas_remote_grading_v1",
        1,
        'c',
    );
    if oracle
        .store
        .load_imathas_question_backend_session(oracle.token, reference, wrong_account)
        .await
        .is_ok()
    {
        failures.push("load accepted a different resolved Account".to_string());
    }
    let future = Timestamp::from_unix_millis(issued_at.as_unix_millis() + 60_000);
    let (future_create, _) = create(
        oracle.account,
        future,
        Timestamp::from_unix_millis(future.as_unix_millis() + 60_000),
    );
    if oracle
        .store
        .create_imathas_question_backend_session(oracle.token, future_create)
        .await
        .is_ok()
    {
        failures.push("create accepted a future-issued Session".to_string());
    }
    let (expired_create, _) = create(
        oracle.account,
        Timestamp::from_unix_millis(issued_at.as_unix_millis() - 120_000),
        Timestamp::from_unix_millis(issued_at.as_unix_millis() - 60_000),
    );
    if oracle
        .store
        .create_imathas_question_backend_session(oracle.token, expired_create)
        .await
        .is_ok()
    {
        failures.push("create accepted an expired Session".to_string());
    }
    let wrong_ring = PostgresImathasQuestionBackendSessionStore::new(
        local_development_pool(
            &std::env::var(DATABASE_URL_ENV)
                .expect("PostgreSQL Migration Acceptance Runtime database URL"),
            ProductionLoginProfile::Api,
        )
        .expect("API pool"),
        Arc::new(key_ring(8)),
    );
    if wrong_ring
        .load_imathas_question_backend_session(oracle.token, reference, expectation.clone())
        .await
        .is_ok()
    {
        failures.push("load accepted imathas state under a wrong AEAD key".to_string());
    }
    for (label, statement) in [
        (
            "immutable Question Attempt",
            "UPDATE ple_private.imathas_question_backend_session \
             SET question_attempt_id = '00000000-0000-0000-0000-00000000f206' \
             WHERE imathas_question_backend_session_id = $1",
        ),
        (
            "immutable Question ID",
            "UPDATE ple_private.imathas_question_backend_session \
             SET question_id = 'ABC-DEF1' WHERE imathas_question_backend_session_id = $1",
        ),
        (
            "immutable Question Revision Number",
            "UPDATE ple_private.imathas_question_backend_session \
             SET revision_number = 2 WHERE imathas_question_backend_session_id = $1",
        ),
        (
            "immutable Question Seed",
            "UPDATE ple_private.imathas_question_backend_session \
             SET question_seed = 2 WHERE imathas_question_backend_session_id = $1",
        ),
        (
            "immutable imathas item",
            "UPDATE ple_private.imathas_question_backend_session \
             SET imathas_item_reference = 'mutated-item' WHERE imathas_question_backend_session_id = $1",
        ),
        (
            "immutable imathas ciphertext",
            "UPDATE ple_private.imathas_question_backend_session \
             SET imathas_question_backend_state_ciphertext = decode(repeat('00', 17), 'hex') \
             WHERE imathas_question_backend_session_id = $1",
        ),
        (
            "direct consumed_at",
            "UPDATE ple_private.imathas_question_backend_session \
             SET consumed_at = clock_timestamp() WHERE imathas_question_backend_session_id = $1",
        ),
    ] {
        if sqlx::query(statement)
            .bind(reference.as_uuid())
            .execute(&oracle.admin)
            .await
            .is_ok()
        {
            failures.push(format!("admin update accepted {label}"));
        }
    }
    let (tamper_create, tamper_expectation) = create(oracle.account, issued_at, expires_at);
    let tamper_reference = oracle
        .store
        .create_imathas_question_backend_session(oracle.token, tamper_create)
        .await
        .expect("create tamper session");
    let mut tamper_transaction = oracle.admin.begin().await.expect("tamper transaction");
    tamper_transaction
        .execute("SET LOCAL session_replication_role = replica")
        .await
        .expect("trusted tamper seam");
    sqlx::query(
        "UPDATE ple_private.imathas_question_backend_session \
         SET imathas_question_backend_state_ciphertext = decode(repeat('00', 17), 'hex') \
         WHERE imathas_question_backend_session_id = $1",
    )
    .bind(tamper_reference.as_uuid())
    .execute(&mut *tamper_transaction)
    .await
    .expect("tamper ciphertext under trusted seam");
    tamper_transaction
        .commit()
        .await
        .expect("commit tamper seam");
    if oracle
        .store
        .load_imathas_question_backend_session(oracle.token, tamper_reference, tamper_expectation)
        .await
        .is_ok()
    {
        failures.push("load accepted tampered imathas ciphertext".to_string());
    }
    let (expired_load_create, expired_load_expectation) =
        create(oracle.account, issued_at, expires_at);
    let expired_load_reference = oracle
        .store
        .create_imathas_question_backend_session(oracle.token, expired_load_create)
        .await
        .expect("create expiry session");
    let mut expiry_transaction = oracle.admin.begin().await.expect("expiry transaction");
    expiry_transaction
        .execute("SET LOCAL session_replication_role = replica")
        .await
        .expect("trusted expiry seam");
    sqlx::query(
        "UPDATE ple_private.imathas_question_backend_session \
         SET issued_at = clock_timestamp() - interval '120 seconds', \
             expires_at = clock_timestamp() - interval '60 seconds' \
         WHERE imathas_question_backend_session_id = $1",
    )
    .bind(expired_load_reference.as_uuid())
    .execute(&mut *expiry_transaction)
    .await
    .expect("set trusted expired Session");
    expiry_transaction
        .commit()
        .await
        .expect("commit expiry seam");
    if oracle
        .store
        .load_imathas_question_backend_session(
            oracle.token,
            expired_load_reference,
            expired_load_expectation,
        )
        .await
        .is_ok()
    {
        failures.push("load accepted an expired Session".to_string());
    }
    let app_direct = lazy_pool(
        &std::env::var(DATABASE_URL_ENV)
            .expect("PostgreSQL Migration Acceptance Runtime database URL"),
    )
    .expect("direct API probe pool");
    let mut app_direct_transaction = app_direct
        .begin()
        .await
        .expect("direct API probe transaction");
    let direct_private_read = app_direct_transaction
        .execute("SET LOCAL ROLE ple_app")
        .await
        .is_ok()
        && sqlx::query("SELECT 1 FROM ple_private.imathas_question_backend_session")
            .fetch_one(&mut *app_direct_transaction)
            .await
            .is_ok();
    if direct_private_read {
        failures.push(
            "ple_app directly read the private iMathAS Question Backend Session table".to_string(),
        );
    }
    let api_owner_writes: bool = sqlx::query_scalar(
        "SELECT has_table_privilege(\
            'ple_api_owner', 'ple_private.question_revision_source_binding', 'INSERT'\
        )",
    )
    .fetch_one(&oracle.admin)
    .await
    .expect("owner privilege catalog probe");
    if api_owner_writes {
        failures.push(
            "ple_api_owner can write unrelated Question Revision Source Bindings".to_string(),
        );
    }
    oracle.admin.execute("UPDATE ple_private.issued_question SET point_value = 2.5, scoring_rule = 'extra_credit', question_statistics_eligibility = false WHERE issued_question_id = '00000000-0000-5000-8000-000000000115'").await.expect("set Extra Credit issued scoring fixture");
    let (contention_create, contention_expectation) = create(oracle.account, issued_at, expires_at);
    let contention_reference = oracle
        .store
        .create_imathas_question_backend_session(oracle.token, contention_create)
        .await
        .expect("create contention session");
    let lease_expires_at = Timestamp::from_unix_millis(issued_at.as_unix_millis() + 90_000);
    let left_store = oracle.store.clone();
    let right_store = oracle.store.clone();
    let (left, right) = tokio::join!(
        left_store.lease_imathas_question_backend_session(
            oracle.token,
            contention_reference,
            contention_expectation.clone(),
            lease_expires_at,
        ),
        right_store.lease_imathas_question_backend_session(
            oracle.token,
            contention_reference,
            contention_expectation,
            lease_expires_at,
        )
    );
    let mut leases = [left, right].into_iter().filter_map(Result::ok);
    if let Some(lease) = leases.next() {
        if leases.next().is_some() {
            failures.push("lease contention admitted more than one winner".to_string());
        }
        let first_transition = transition_with_score(lease.clone(), now(), 0.4);
        let second_transition = first_transition.clone();
        let left_store = oracle.store.clone();
        let right_store = oracle.store.clone();
        let (left, right) = tokio::join!(
            left_store.stage_verified_imathas_result(oracle.token, first_transition),
            right_store.stage_verified_imathas_result(oracle.token, second_transition),
        );
        let staged_job = match (left, right) {
            (Ok(left_receipt), Ok(right_receipt))
                if left_receipt.job_id() == right_receipt.job_id() =>
            {
                Some(left_receipt.job_id())
            }
            (Ok(receipt), Err(_)) | (Err(_), Ok(receipt)) => Some(receipt.job_id()),
            _ => {
                failures
                    .push("verified result stage contention did not retain one result".to_string());
                None
            }
        };
        if oracle
            .store
            .stage_verified_imathas_result(oracle.token, transition_with_score(lease, now(), 0.5))
            .await
            .is_ok()
        {
            failures.push("verified result stage accepted a changed terminal replay".to_string());
        }
        if let Some(job_id) = staged_job {
            let claim = oracle
                .grading_worker_store
                .claim_imathas_result_grading_job(
                    job_id,
                    Timestamp::from_unix_millis(now().as_unix_millis() + 120_000),
                )
                .await
                .expect("claim Extra Credit grading Job");
            let receipt = oracle
                .grading_worker_store
                .commit_staged_imathas_result_grading(CommitStagedImathasResultGrading::new(
                    claim,
                    now(),
                ))
                .await
                .expect("commit Extra Credit grade");
            if receipt.grading_result().points_earned != 1.0 {
                failures.push("Extra Credit issued scoring result was not 1.0".to_string());
            }
        }
    } else {
        failures.push("lease contention had no winner".to_string());
    }
    let (revocation_create, revocation_expectation) = create(oracle.account, issued_at, expires_at);
    let revocation_reference = oracle
        .store
        .create_imathas_question_backend_session(oracle.token, revocation_create)
        .await
        .expect("create revocation session");
    sqlx::query(
        "UPDATE ple_private.imathas_question_backend_session \
         SET revoked_at = clock_timestamp() WHERE imathas_question_backend_session_id = $1",
    )
    .bind(revocation_reference.as_uuid())
    .execute(&oracle.admin)
    .await
    .expect("forward revocation");
    if oracle
        .store
        .load_imathas_question_backend_session(
            oracle.token,
            revocation_reference,
            revocation_expectation,
        )
        .await
        .is_ok()
    {
        failures.push("load accepted a revoked Session".to_string());
    }
    sqlx::query(
        "INSERT INTO ple_data.course_membership_event \
         (course_membership_event_id, membership_id, event_kind, occurred_at, reason) \
         VALUES ($1, $2, 'ended', clock_timestamp(), 'oracle revocation')",
    )
    .bind(Uuid::from_u128(ORACLE_MEMBERSHIP_END_EVENT))
    .bind(Uuid::from_u128(STUDENT_MEMBERSHIP))
    .execute(&oracle.admin)
    .await
    .expect("end Student Course Membership");
    if oracle
        .store
        .load_imathas_question_backend_session(oracle.token, reference, expectation)
        .await
        .is_ok()
    {
        failures.push("load accepted an inactive Student Course Membership".to_string());
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
