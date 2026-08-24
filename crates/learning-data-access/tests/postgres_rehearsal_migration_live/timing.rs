//! Connected PostgreSQL 17 timing, expiry, and retry oracle.
//!
//! The clock seam is deliberately database-owned and tenant-scoped.  It only
//! exists in this disposable test database, so production routes still obtain
//! time exclusively from PostgreSQL; no command accepts a browser timestamp.

use std::sync::OnceLock;

use learning_data_access::{
    ClaimRehearsalDeliveryRouteCommand, ClaimRehearsalSubmissionRouteCommand,
    CompleteRehearsalDeliveryRouteCommand, ReconcileRehearsalDeliveryExpiryRouteCommand,
    RehearsalDeliveryClaimResult, RehearsalDeliveryDispatchResult,
    RehearsalDeliveryRetryDisposition, RehearsalIdempotencyKey, RehearsalOperationDigest,
    RehearsalRouteMutationStore, RetryRehearsalDeliveryResult, RetryRehearsalDeliveryRouteCommand,
};
use question_model::{
    ActivityTimestamp, RehearsalActiveScreenV1, StudentResponse, run_policy::TimingPolicy,
};
use sqlx::PgPool;
use tokio::sync::{Mutex, MutexGuard};

use super::canonical_store::{StartedFixture, started_fixture_with_timing};
use super::post_start::{delivery, route};
use super::progression::{commit_or_resume_issued_execution, grader};

static CLOCK_SERIAL: OnceLock<Mutex<()>> = OnceLock::new();

/// A tenant-scoped, migrator-external clock seam for this ignored disposable
/// database target.  The replacement lives for this disposable database's
/// lifetime, so an assertion panic cannot leave a later test with a missing
/// clock relation.  Routes and brokers continue to call `ple_rehearsal_now`,
/// and neither Rust nor a browser supplies a timestamp.
struct TestClock<'a> {
    pool: &'a PgPool,
    tenant: uuid::Uuid,
    _serial: MutexGuard<'static, ()>,
}

impl<'a> TestClock<'a> {
    async fn install(pool: &'a PgPool, tenant: uuid::Uuid, initial_millis: i64) -> Self {
        let serial = CLOCK_SERIAL.get_or_init(|| Mutex::new(())).lock().await;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS public.ple_rehearsal_test_clock \
             (tenant_id uuid PRIMARY KEY, now_at timestamptz NOT NULL)",
        )
        .execute(pool)
        .await
        .expect("create disposable database-owned test clock");
        sqlx::query("ALTER TABLE public.ple_rehearsal_test_clock OWNER TO ple_rehearsal_broker")
            .execute(pool)
            .await
            .expect("test clock is owned by the rehearsal broker");
        sqlx::query("REVOKE ALL ON public.ple_rehearsal_test_clock FROM PUBLIC, ple_app")
            .execute(pool)
            .await
            .expect("application role cannot read the test clock table");
        sqlx::query(
            "CREATE OR REPLACE FUNCTION public.ple_rehearsal_now() \
             RETURNS timestamptz LANGUAGE sql STABLE SECURITY DEFINER \
             SET search_path TO pg_catalog,public,pg_temp AS $$ \
             SELECT date_trunc('milliseconds', COALESCE( \
               (SELECT now_at FROM public.ple_rehearsal_test_clock \
                 WHERE tenant_id=NULLIF(current_setting('ple.tenant_id', true),'')::uuid), \
               transaction_timestamp())) $$",
        )
        .execute(pool)
        .await
        .expect("install tenant-scoped server clock seam");
        sqlx::query("ALTER FUNCTION public.ple_rehearsal_now() OWNER TO ple_rehearsal_broker")
            .execute(pool)
            .await
            .expect("test clock function remains broker owned");
        sqlx::query("REVOKE ALL ON FUNCTION public.ple_rehearsal_now() FROM PUBLIC")
            .execute(pool)
            .await
            .expect("test clock has no public execute privilege");
        sqlx::query(
            "GRANT EXECUTE ON FUNCTION public.ple_rehearsal_now() \
             TO ple_rehearsal_broker,ple_rehearsal_source,ple_app",
        )
        .execute(pool)
        .await
        .expect("only rehearsal capabilities may call the test clock");
        let clock = Self {
            pool,
            tenant,
            _serial: serial,
        };
        clock.set(initial_millis).await;
        clock
    }

    async fn set(&self, millis: i64) {
        sqlx::query(
            "INSERT INTO public.ple_rehearsal_test_clock(tenant_id,now_at) \
             VALUES($1, public.ple_rehearsal_timestamp_from_millis($2)) \
             ON CONFLICT(tenant_id) DO UPDATE SET now_at=EXCLUDED.now_at",
        )
        .bind(self.tenant)
        .bind(millis)
        .execute(self.pool)
        .await
        .expect("advance database-owned test clock");
    }
}

async fn run_started_millis(fixture: &StartedFixture) -> i64 {
    sqlx::query_scalar(
        "SELECT (extract(epoch FROM started_at) * 1000)::bigint \
         FROM rehearsal_run WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("frozen rehearsal start timestamp")
}

async fn prepare_and_dispatch(
    fixture: &StartedFixture,
    key: &str,
    fingerprint: u8,
) -> learning_data_access::DispatchedRehearsalDelivery {
    let identity = route(fixture);
    let claim = fixture
        .store
        .claim_rehearsal_delivery_from_route(fixture.context, delivery(identity, key, fingerprint))
        .await
        .expect("route prepares frozen delivery");
    let RehearsalDeliveryClaimResult::Prepared { prepared } = claim else {
        panic!("initial delivery is prepared");
    };
    let dispatched = fixture
        .store
        .mark_rehearsal_delivery_dispatched_from_route(fixture.context, identity, prepared)
        .await
        .expect("database-owned timing dispatch");
    let RehearsalDeliveryDispatchResult::Dispatched { dispatched } = dispatched else {
        panic!("initial delivery dispatches before its deadline");
    };
    dispatched
}

async fn prepare_only(
    fixture: &StartedFixture,
    key: &str,
    fingerprint: u8,
) -> learning_data_access::PreparedRehearsalDelivery {
    let identity = route(fixture);
    let claim = fixture
        .store
        .claim_rehearsal_delivery_from_route(fixture.context, delivery(identity, key, fingerprint))
        .await
        .expect("route prepares frozen delivery");
    let RehearsalDeliveryClaimResult::Prepared { prepared } = claim else {
        panic!("delivery is prepared");
    };
    prepared
}

async fn complete_issued(
    fixture: &StartedFixture,
    dispatched: learning_data_access::DispatchedRehearsalDelivery,
    _label: &str,
) -> RehearsalActiveScreenV1 {
    let grader = grader().await;
    let sealed = commit_or_resume_issued_execution(fixture, &grader, &dispatched).await;
    let active = sealed.active_screen().expect("issued artifact screen");
    fixture
        .store
        .complete_rehearsal_delivery_from_route(
            fixture.context,
            CompleteRehearsalDeliveryRouteCommand {
                route: route(fixture),
                dispatched,
                screen: active.clone(),
            },
        )
        .await
        .expect("answer-free issued screen");
    active
}

async fn timing_row(fixture: &StartedFixture) -> (i32, String, i64, i64) {
    sqlx::query_as(
        "SELECT octet_length(witness_bytes), deadline_source, \
                (extract(epoch FROM deadline_at)*1000)::bigint, \
                (extract(epoch FROM expires_at)*1000)::bigint \
           FROM rehearsal_delivery_timing_witness \
          WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("immutable timing witness")
}

/// Rehearsal timing is isolated execution evidence, never learner work,
/// effective-policy, submission, or course-grade state.
async fn learner_trace_counts(fixture: &StartedFixture) -> (i64, i64, i64, i64, i64, i64) {
    sqlx::query_as(
        "SELECT \
          (SELECT count(*) FROM assignment_run WHERE tenant_id=$1), \
          (SELECT count(*) FROM question_attempt WHERE tenant_id=$1), \
          (SELECT count(*) FROM attempt_effective_policy_receipt WHERE tenant_id=$1), \
          (SELECT count(*) FROM submission WHERE tenant_id=$1), \
          (SELECT count(*) FROM submission_evaluation WHERE tenant_id=$1), \
          (SELECT count(*) FROM course_grade_export_audit WHERE tenant_id=$1)",
    )
    .bind(fixture.tenant.as_uuid())
    .fetch_one(&fixture.pool)
    .await
    .expect("learner-work trace counts")
}

async fn prepared_operation(fixture: &StartedFixture, key: &str) -> uuid::Uuid {
    sqlx::query_scalar(
        "SELECT generation.operation_id \
           FROM rehearsal_delivery_operation_root AS root \
           JOIN rehearsal_delivery_operation_generation AS generation \
             ON generation.tenant_id=root.tenant_id \
            AND generation.rehearsal_run_id=root.rehearsal_run_id \
            AND generation.root_id=root.root_id \
          WHERE root.tenant_id=$1 AND root.rehearsal_run_id=$2 \
            AND root.idempotency_key=$3",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .bind(key)
    .fetch_one(&fixture.pool)
    .await
    .expect("prepared operation identity")
}

async fn finalize_with(
    fixture: &StartedFixture,
    operation: uuid::Uuid,
    issued_at: i64,
    witness: Vec<u8>,
    commitment: Vec<u8>,
) -> bool {
    let mut transaction = fixture.pool.begin().await.expect("application transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("application role");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(fixture.tenant.as_uuid().to_string())
        .execute(&mut *transaction)
        .await
        .expect("application tenant");
    let accepted: bool = sqlx::query_scalar(
        "SELECT public.ple_finalize_rehearsal_timing_dispatch( \
           $1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(operation)
    .bind(issued_at)
    .bind(Some(issued_at + 1_000))
    .bind(Some(0_i64))
    .bind(Some(issued_at + 1_000))
    .bind(Some("perQuestion"))
    .bind(witness)
    .bind(commitment)
    .fetch_one(&mut *transaction)
    .await
    .expect("finalizer call returns an explicit refusal");
    transaction
        .commit()
        .await
        .expect("finalizer refusal commits no mutation");
    accepted
}

async fn corrupt_frozen_snapshot(fixture: &StartedFixture) {
    let mut transaction = fixture
        .pool
        .begin()
        .await
        .expect("snapshot corruption transaction");
    sqlx::query(
        "ALTER TABLE public.rehearsal_frozen_source_snapshot \
         DISABLE TRIGGER rehearsal_source_snapshot_append_only",
    )
    .execute(&mut *transaction)
    .await
    .expect("disable only disposable snapshot append trigger");
    let updated = sqlx::query(
        "UPDATE public.rehearsal_frozen_source_snapshot \
            SET issued_snapshot_bytes=issued_snapshot_bytes||decode('00','hex'), \
                issued_snapshot_sha256=digest(issued_snapshot_bytes||decode('00','hex'),'sha256') \
          WHERE tenant_id=$1 AND rehearsal_run_id=$2 AND ordinal=0",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .execute(&mut *transaction)
    .await
    .expect("corrupt disposable frozen snapshot while preserving local checksum");
    assert_eq!(updated.rows_affected(), 1);
    sqlx::query(
        "ALTER TABLE public.rehearsal_frozen_source_snapshot \
         ENABLE TRIGGER rehearsal_source_snapshot_append_only",
    )
    .execute(&mut *transaction)
    .await
    .expect("restore snapshot append trigger");
    transaction
        .commit()
        .await
        .expect("commit disposable snapshot corruption");
}

async fn corrupt_timing_witness(fixture: &StartedFixture) {
    let mut transaction = fixture
        .pool
        .begin()
        .await
        .expect("witness corruption transaction");
    sqlx::query(
        "ALTER TABLE public.rehearsal_delivery_timing_witness \
         DISABLE TRIGGER rehearsal_delivery_timing_witness_append_only",
    )
    .execute(&mut *transaction)
    .await
    .expect("disable only disposable witness append trigger");
    let updated = sqlx::query(
        "UPDATE public.rehearsal_delivery_timing_witness \
            SET witness_bytes=decode(repeat('00',99),'hex'), \
                witness_sha256=digest(convert_to('ple:rehearsal:timing-witness:v1','UTF8') \
                  ||decode('00','hex')||decode(repeat('00',99),'hex'),'sha256') \
          WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .execute(&mut *transaction)
    .await
    .expect("corrupt witness while retaining its database checksum");
    assert_eq!(updated.rows_affected(), 1);
    sqlx::query(
        "ALTER TABLE public.rehearsal_delivery_timing_witness \
         ENABLE TRIGGER rehearsal_delivery_timing_witness_append_only",
    )
    .execute(&mut *transaction)
    .await
    .expect("restore witness append trigger");
    transaction
        .commit()
        .await
        .expect("commit disposable witness corruption");
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn per_question_expiry_retries_exact_frozen_material_with_a_database_owned_clock() {
    let fixture = started_fixture_with_timing(
        TimingPolicy::PerQuestion {
            seconds: 1,
            grace_seconds: 0,
        },
        None,
    )
    .await;
    let start = run_started_millis(&fixture).await;
    let clock = TestClock::install(&fixture.pool, fixture.tenant.as_uuid(), start).await;
    let initial = prepare_and_dispatch(&fixture, "timing-continue", 0x71).await;
    let initial_binding: (uuid::Uuid, Vec<u8>, Vec<u8>, i32) = sqlx::query_as(
        "SELECT attempt_id,issued_snapshot_sha256,private_execution_sha256,generation \
           FROM rehearsal_delivery_generation_material_binding \
          WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("initial immutable material binding");
    assert_eq!(
        timing_row(&fixture).await,
        (99, "perQuestion".into(), start + 1_000, start + 1_000)
    );
    complete_issued(&fixture, initial, "per-question issued").await;

    clock.set(start + 1_001).await;
    let expiry = fixture
        .store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand {
                route: route(&fixture),
            },
        )
        .await
        .expect("server-owned expiry reconciliation");
    assert_eq!(expiry.verdict, domain::RehearsalTimingVerdictV1::Expired);
    assert_eq!(
        expiry.retry_disposition,
        RehearsalDeliveryRetryDisposition::Available
    );

    let retry = RetryRehearsalDeliveryRouteCommand {
        route: route(&fixture),
        idempotency_key: RehearsalIdempotencyKey::new("timing-retry".into()).expect("retry key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([0x73; 32]),
    };
    let concurrent_store = fixture.store.clone();
    let (left, right) = tokio::join!(
        fixture
            .store
            .retry_rehearsal_delivery_from_route(fixture.context, retry.clone()),
        concurrent_store.retry_rehearsal_delivery_from_route(fixture.context, retry.clone()),
    );
    let RetryRehearsalDeliveryResult::Prepared { prepared } = left.expect("first retry result")
    else {
        panic!("per-question expiry creates a same-item successor");
    };
    assert!(matches!(
        right.expect("same retry-key concurrent replay"),
        RetryRehearsalDeliveryResult::Prepared { .. }
    ));
    let retry_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_delivery_retry_operation \
          WHERE tenant_id=$1 AND rehearsal_run_id=$2 AND idempotency_key='timing-retry'",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("one retry idempotency mapping");
    assert_eq!(
        retry_rows, 1,
        "concurrent retry key creates exactly one successor mapping"
    );
    let successor: (uuid::Uuid, Vec<u8>, Vec<u8>, i32) = sqlx::query_as(
        "SELECT attempt_id,issued_snapshot_sha256,private_execution_sha256,generation \
           FROM rehearsal_delivery_generation_material_binding \
          WHERE tenant_id=$1 AND rehearsal_run_id=$2 ORDER BY generation DESC LIMIT 1",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("successor retains immutable material binding");
    assert_eq!(successor.0, initial_binding.0);
    assert_eq!(successor.1, initial_binding.1);
    assert_eq!(successor.2, initial_binding.2);
    assert_eq!(successor.3, initial_binding.3 + 1);

    assert!(matches!(
        fixture
            .store
            .retry_rehearsal_delivery_from_route(fixture.context, retry.clone())
            .await
            .expect("exact retry replays preparation"),
        RetryRehearsalDeliveryResult::Prepared { .. }
    ));
    assert!(matches!(
        fixture
            .store
            .retry_rehearsal_delivery_from_route(
                fixture.context,
                RetryRehearsalDeliveryRouteCommand {
                    request_fingerprint: RehearsalOperationDigest::from_bytes([0x74; 32]),
                    ..retry
                }
            )
            .await
            .expect("fingerprint conflict is an ordinary visible result"),
        RetryRehearsalDeliveryResult::Conflict
    ));
    assert!(matches!(
        fixture
            .store
            .claim_rehearsal_delivery_from_route(
                fixture.context,
                ClaimRehearsalDeliveryRouteCommand {
                    route: route(&fixture),
                    idempotency_key: RehearsalIdempotencyKey::new("other-continue".into())
                        .expect("continue key"),
                    request_fingerprint: RehearsalOperationDigest::from_bytes([0x75; 32]),
                },
            )
            .await
            .expect("different Continue is fenced after expiry"),
        RehearsalDeliveryClaimResult::Conflict
            | RehearsalDeliveryClaimResult::RunTimeExhausted { .. }
    ));
    let _ = prepared;
    let _ = clock;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn per_attempt_expiry_is_a_persisted_terminal_without_a_successor_screen() {
    let fixture = started_fixture_with_timing(
        TimingPolicy::PerAttempt {
            seconds: 1,
            grace_seconds: 0,
        },
        None,
    )
    .await;
    let start = run_started_millis(&fixture).await;
    let clock = TestClock::install(&fixture.pool, fixture.tenant.as_uuid(), start).await;
    let dispatched = prepare_and_dispatch(&fixture, "attempt-continue", 0x81).await;
    complete_issued(&fixture, dispatched, "per-attempt issued").await;
    clock.set(start + 1_001).await;

    let expiry = fixture
        .store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand {
                route: route(&fixture),
            },
        )
        .await
        .expect("per-attempt expiry");
    assert_eq!(
        expiry.retry_disposition,
        RehearsalDeliveryRetryDisposition::RunTimeExhausted
    );
    let retry = RetryRehearsalDeliveryRouteCommand {
        route: route(&fixture),
        idempotency_key: RehearsalIdempotencyKey::new("attempt-retry".into()).expect("retry key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([0x83; 32]),
    };
    let RetryRehearsalDeliveryResult::RunTimeExhausted { deadline } = fixture
        .store
        .retry_rehearsal_delivery_from_route(fixture.context, retry.clone())
        .await
        .expect("per-attempt terminal retry")
    else {
        panic!("per-attempt timeout never creates a successor screen");
    };
    assert_eq!(deadline, ActivityTimestamp::from_unix_millis(start + 1_000));
    let generations: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_delivery_operation_generation \
          WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("generation count");
    assert_eq!(
        generations, 1,
        "terminal retry creates no successor generation"
    );
    assert!(matches!(
        fixture
            .store
            .retry_rehearsal_delivery_from_route(fixture.context, retry)
            .await
            .expect("terminal retry is idempotent"),
        RetryRehearsalDeliveryResult::RunTimeExhausted { .. }
    ));
    let _ = clock;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn subject_limit_expiry_is_a_persisted_terminal_without_a_successor_screen() {
    let fixture = started_fixture_with_timing(TimingPolicy::Untimed, Some(1)).await;
    let start = run_started_millis(&fixture).await;
    let clock = TestClock::install(&fixture.pool, fixture.tenant.as_uuid(), start).await;
    let dispatched = prepare_and_dispatch(&fixture, "subject-continue", 0x91).await;
    assert_eq!(
        timing_row(&fixture).await,
        (99, "subjectLimit".into(), start + 1_000, start + 1_000)
    );
    complete_issued(&fixture, dispatched, "subject-limit issued").await;
    clock.set(start + 1_001).await;
    let expiry = fixture
        .store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand {
                route: route(&fixture),
            },
        )
        .await
        .expect("subject limit expiry");
    assert_eq!(
        expiry.retry_disposition,
        RehearsalDeliveryRetryDisposition::RunTimeExhausted
    );
    let retry = RetryRehearsalDeliveryRouteCommand {
        route: route(&fixture),
        idempotency_key: RehearsalIdempotencyKey::new("subject-retry".into()).expect("retry key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([0x93; 32]),
    };
    assert!(matches!(
        fixture
            .store
            .retry_rehearsal_delivery_from_route(fixture.context, retry.clone())
            .await
            .expect("subject terminal retry"),
        RetryRehearsalDeliveryResult::RunTimeExhausted { deadline }
            if deadline == ActivityTimestamp::from_unix_millis(start + 1_000)
    ));
    let generations: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_delivery_operation_generation \
          WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("generation count");
    assert_eq!(generations, 1, "subject terminal has no successor");
    assert!(matches!(
        fixture
            .store
            .retry_rehearsal_delivery_from_route(fixture.context, retry)
            .await
            .expect("subject terminal replay"),
        RetryRehearsalDeliveryResult::RunTimeExhausted { .. }
    ));
    assert_eq!(
        learner_trace_counts(&fixture).await,
        (0, 0, 0, 0, 0, 0),
        "timing terminal work creates no learner or gradebook trace"
    );
    let _ = clock;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn prepared_per_question_retry_becomes_a_replayed_subject_cap_terminal() {
    let fixture = started_fixture_with_timing(
        TimingPolicy::PerQuestion {
            seconds: 1,
            grace_seconds: 0,
        },
        Some(2),
    )
    .await;
    let start = run_started_millis(&fixture).await;
    let clock = TestClock::install(&fixture.pool, fixture.tenant.as_uuid(), start).await;
    let initial = prepare_and_dispatch(&fixture, "cap-continue", 0xA1).await;
    complete_issued(&fixture, initial, "cap-race issued").await;
    clock.set(start + 1_001).await;
    let expiry = fixture
        .store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand {
                route: route(&fixture),
            },
        )
        .await
        .expect("question timeout before overall cap");
    assert_eq!(
        expiry.retry_disposition,
        RehearsalDeliveryRetryDisposition::Available
    );
    let retry = RetryRehearsalDeliveryRouteCommand {
        route: route(&fixture),
        idempotency_key: RehearsalIdempotencyKey::new("cap-retry".into()).expect("retry key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([0xA3; 32]),
    };
    let RetryRehearsalDeliveryResult::Prepared { prepared } = fixture
        .store
        .retry_rehearsal_delivery_from_route(fixture.context, retry.clone())
        .await
        .expect("retry prepares before overall cap")
    else {
        panic!("per-question retry is prepared");
    };
    clock.set(start + 2_001).await;
    assert!(matches!(
        fixture
            .store
            .mark_rehearsal_delivery_dispatched_from_route(fixture.context, route(&fixture), prepared)
            .await
            .expect("dispatch checks the immutable run cap"),
        RehearsalDeliveryDispatchResult::RunTimeExhausted { deadline }
            if deadline == ActivityTimestamp::from_unix_millis(start + 2_000)
    ));
    let tail: (String, i64) = sqlx::query_as(
        "SELECT phase,(extract(epoch FROM deadline_at)*1000)::bigint \
           FROM rehearsal_delivery_operation_event \
          WHERE tenant_id=$1 AND rehearsal_run_id=$2 \
          ORDER BY generation DESC,sequence DESC LIMIT 1",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("persisted pre-dispatch terminal event");
    assert_eq!(
        tail,
        ("runTimeExhaustedBeforeDispatch".into(), start + 2_000)
    );
    assert!(matches!(
        fixture
            .store
            .retry_rehearsal_delivery_from_route(fixture.context, retry)
            .await
            .expect("terminal retry replay"),
        RetryRehearsalDeliveryResult::RunTimeExhausted { .. }
    ));
    let screens: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_delivery_receipt \
          WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("answer-free screen count");
    assert_eq!(screens, 1, "terminal successor creates no screen");
    let _ = clock;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn timing_finalizer_refuses_altered_scalar_witness_and_commitment_without_dispatching() {
    let fixture = started_fixture_with_timing(
        TimingPolicy::PerQuestion {
            seconds: 1,
            grace_seconds: 0,
        },
        None,
    )
    .await;
    let start = run_started_millis(&fixture).await;
    let clock = TestClock::install(&fixture.pool, fixture.tenant.as_uuid(), start).await;
    let _prepared = prepare_only(&fixture, "finalizer-refusal", 0xB1).await;
    let operation = prepared_operation(&fixture, "finalizer-refusal").await;
    let commitment = vec![0_u8; 32];
    assert!(
        !finalize_with(
            &fixture,
            operation,
            start + 1,
            vec![0_u8; 99],
            commitment.clone(),
        )
        .await,
        "a finalizer scalar that differs from the database clock is refused"
    );
    assert!(
        !finalize_with(
            &fixture,
            operation,
            start,
            vec![0_u8; 98],
            commitment.clone(),
        )
        .await,
        "a malformed 99-byte witness is refused"
    );
    assert!(
        !finalize_with(&fixture, operation, start, vec![0_u8; 99], commitment).await,
        "a witness whose commitment does not authenticate it is refused"
    );
    let events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_delivery_operation_event \
          WHERE tenant_id=$1 AND rehearsal_run_id=$2 AND phase='issueDispatched'",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("dispatch event count");
    assert_eq!(
        events, 0,
        "invalid finalizer inputs append no dispatch event"
    );
    let _ = clock;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn frozen_material_and_timing_witness_corruption_fail_closed_before_live_mutation() {
    let frozen_fixture = started_fixture_with_timing(
        TimingPolicy::PerQuestion {
            seconds: 1,
            grace_seconds: 0,
        },
        None,
    )
    .await;
    let frozen_start = run_started_millis(&frozen_fixture).await;
    let frozen_clock = TestClock::install(
        &frozen_fixture.pool,
        frozen_fixture.tenant.as_uuid(),
        frozen_start,
    )
    .await;
    let prepared = prepare_only(&frozen_fixture, "corrupt-frozen", 0xC1).await;
    corrupt_frozen_snapshot(&frozen_fixture).await;
    assert!(
        frozen_fixture
            .store
            .mark_rehearsal_delivery_dispatched_from_route(
                frozen_fixture.context,
                route(&frozen_fixture),
                prepared,
            )
            .await
            .is_err(),
        "a locally checksummed but aggregate-corrupt frozen snapshot cannot dispatch"
    );
    drop(frozen_clock);

    let timing_fixture = started_fixture_with_timing(
        TimingPolicy::PerQuestion {
            seconds: 1,
            grace_seconds: 0,
        },
        None,
    )
    .await;
    let timing_start = run_started_millis(&timing_fixture).await;
    let timing_clock = TestClock::install(
        &timing_fixture.pool,
        timing_fixture.tenant.as_uuid(),
        timing_start,
    )
    .await;
    let dispatched = prepare_and_dispatch(&timing_fixture, "corrupt-timing", 0xC2).await;
    complete_issued(&timing_fixture, dispatched, "corrupt timing issued").await;
    timing_clock.set(timing_start + 1_001).await;
    assert_eq!(
        timing_fixture
            .store
            .reconcile_rehearsal_delivery_expiry_from_route(
                timing_fixture.context,
                ReconcileRehearsalDeliveryExpiryRouteCommand {
                    route: route(&timing_fixture),
                },
            )
            .await
            .expect("clean expiry before corruption")
            .verdict,
        domain::RehearsalTimingVerdictV1::Expired
    );
    corrupt_timing_witness(&timing_fixture).await;
    assert!(
        timing_fixture
            .store
            .reconcile_rehearsal_delivery_expiry_from_route(
                timing_fixture.context,
                ReconcileRehearsalDeliveryExpiryRouteCommand {
                    route: route(&timing_fixture),
                },
            )
            .await
            .is_err(),
        "a timing witness that violates canonical derivation cannot be reconciled"
    );
    assert!(
        timing_fixture
            .store
            .retry_rehearsal_delivery_from_route(
                timing_fixture.context,
                RetryRehearsalDeliveryRouteCommand {
                    route: route(&timing_fixture),
                    idempotency_key: RehearsalIdempotencyKey::new("corrupt-retry".into())
                        .expect("retry key"),
                    request_fingerprint: RehearsalOperationDigest::from_bytes([0xC4; 32]),
                },
            )
            .await
            .is_err(),
        "a corrupt timing aggregate cannot mint a retry"
    );
    let _ = timing_clock;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL 17 application and grader connections"]
async fn expiry_and_first_submission_serialize_to_one_expired_outcome() {
    let fixture = started_fixture_with_timing(
        TimingPolicy::PerQuestion {
            seconds: 1,
            grace_seconds: 0,
        },
        None,
    )
    .await;
    let start = run_started_millis(&fixture).await;
    let clock = TestClock::install(&fixture.pool, fixture.tenant.as_uuid(), start).await;
    let dispatched = prepare_and_dispatch(&fixture, "submission-race-continue", 0xD1).await;
    let active = complete_issued(&fixture, dispatched, "submission race issued").await;
    clock.set(start + 1_001).await;
    let submission_store = fixture.store.clone();
    let (expiry, submission) = tokio::join!(
        fixture
            .store
            .reconcile_rehearsal_delivery_expiry_from_route(
                fixture.context,
                ReconcileRehearsalDeliveryExpiryRouteCommand {
                    route: route(&fixture),
                },
            ),
        submission_store.claim_rehearsal_submission_from_route(
            fixture.context,
            ClaimRehearsalSubmissionRouteCommand {
                route: route(&fixture),
                response: StudentResponse::Numeric { value: 3.0 },
                presentation_digest: active.presentation_digest,
                idempotency_key: RehearsalIdempotencyKey::new("submission-race".into())
                    .expect("submission key"),
            },
        ),
    );
    assert_eq!(
        expiry.expect("expiry reconciliation").verdict,
        domain::RehearsalTimingVerdictV1::Expired,
        "the locked expiry operation wins at the authoritative expiry instant"
    );
    assert!(
        submission.is_err(),
        "the same locked first submission cannot claim an expired generation"
    );
    let claims: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_submission_claim_root \
          WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .fetch_one(&fixture.pool)
    .await
    .expect("submission claim count");
    assert_eq!(
        claims, 0,
        "expired race appends no learner submission claim"
    );
    let _ = clock;
}
