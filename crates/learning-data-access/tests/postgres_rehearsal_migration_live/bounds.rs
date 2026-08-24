use super::fixture::{app, bytes, id, millis, pool, source};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};

const SUBJECT_MAX_BYTES: i64 = 65_536;
const FROZEN_RESPONSE_MAX_BYTES: i64 = 262_144;
const FROZEN_EVIDENCE_MAX_BYTES: i64 = 263_168;
const SEALED_REQUEST_MAX_BYTES: i64 = 393_728;
const ACCEPTED_EVIDENCE_MAX_BYTES: i64 = 395_264;
const RECEIPT_PROJECTION_MAX_BYTES: i64 = 394_240;
const JSON_OBJECT_OVERHEAD_BYTES: i64 = 9;

fn object_at_jsonb_ceiling(ceiling: i64) -> Value {
    json!({"x": "x".repeat((ceiling - JSON_OBJECT_OVERHEAD_BYTES) as usize)})
}

async fn assert_jsonb_bytes(
    transaction: &mut Transaction<'_, Postgres>,
    value: &Value,
    expected: i64,
) {
    let actual: i64 = sqlx::query_scalar("SELECT public.ple_rehearsal_jsonb_bytes($1)")
        .bind(value)
        .fetch_one(&mut **transaction)
        .await
        .expect("PostgreSQL JSONB size measurement");
    assert_eq!(
        actual, expected,
        "generated JSON hits its exported migration ceiling"
    );
}

async fn start_with_subject(
    transaction: &mut Transaction<'_, Postgres>,
    source: super::fixture::Source,
    run: uuid::Uuid,
    subject: &Value,
) -> Option<i64> {
    let expected_latest_run: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT rehearsal_run_id FROM rehearsal_run WHERE tenant_id=$1 AND course_id=$2 AND assignment_id=$3 AND direct_instructor_membership_id=$4 ORDER BY rehearsal_reference DESC LIMIT 1",
    )
    .bind(source.tenant)
    .bind(source.course)
    .bind(source.assignment)
    .bind(source.membership)
    .fetch_optional(&mut **transaction)
    .await
    .expect("Store-style latest-run witness");
    sqlx::query_scalar("SELECT public.ple_rehearsal_start($1,$2,$3,$4,$5,1,$6,$7,$8,$9,false,$10)")
        .bind(source.tenant)
        .bind(source.actor)
        .bind(source.course)
        .bind(source.assignment)
        .bind(source.assignment_reference)
        .bind(subject)
        .bind(run.as_bytes().repeat(2))
        .bind(bytes(0))
        .bind(run)
        .bind(expected_latest_run)
        .fetch_one(&mut **transaction)
        .await
        .expect("subject admission capability")
}

async fn freeze(
    transaction: &mut Transaction<'_, Postgres>,
    source: super::fixture::Source,
    run: uuid::Uuid,
    evidence: &Value,
    response_definition: &Value,
) -> bool {
    let recorded_at = millis(transaction).await;
    sqlx::query_scalar(
        "SELECT public.ple_rehearsal_append_frozen_item($1,$2,$3,$4,1,$5,$6,0,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$10)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .bind(run)
    .bind(bytes(0))
    .bind(bytes(2))
    .bind(evidence)
    .bind(bytes(3))
    .bind(recorded_at)
    .bind(id())
    .bind(id())
    .bind(id())
    .bind(response_definition)
    .bind(bytes(4))
    .bind(bytes(5))
    .fetch_one(&mut **transaction)
    .await
    .expect("frozen JSON admission capability")
}

async fn create_dispatched_claim(
    transaction: &mut Transaction<'_, Postgres>,
    source: super::fixture::Source,
    run: uuid::Uuid,
    sealed_request: &Value,
) -> (uuid::Uuid, uuid::Uuid) {
    let claim = id();
    let operation = id();
    let created: bool = sqlx::query_scalar(
        "SELECT public.ple_rehearsal_create_claim($1,$2,$3,$4,1,$5,$6,$7,'bounds-claim',$8,$9,$10)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .bind(run)
    .bind(claim)
    .bind(operation)
    .bind(id())
    .bind(bytes(6))
    .bind(sealed_request)
    .fetch_one(&mut **transaction)
    .await
    .expect("sealed-request admission capability");
    assert!(created, "claim at its admitted JSON size is prepared");
    let dispatched: bool = sqlx::query_scalar(
        "SELECT public.ple_rehearsal_append_claim_event($1,$2,$3,$4,1,$5,$6,$7,'gradingDispatched',NULL)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .bind(run)
    .bind(claim)
    .bind(operation)
    .fetch_one(&mut **transaction)
    .await
    .expect("dispatch prepared claim");
    assert!(
        dispatched,
        "prepared claim dispatches before completion bounds probe"
    );
    (claim, operation)
}

async fn head_length(transaction: &mut Transaction<'_, Postgres>, run: uuid::Uuid) -> i64 {
    sqlx::query_scalar("SELECT evidence_length FROM rehearsal_run WHERE rehearsal_run_id=$1")
        .bind(run)
        .fetch_one(&mut **transaction)
        .await
        .expect("rehearsal evidence head")
}

async fn receipt_count(transaction: &mut Transaction<'_, Postgres>, run: uuid::Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_submission_receipt WHERE rehearsal_run_id=$1",
    )
    .bind(run)
    .fetch_one(&mut **transaction)
    .await
    .expect("rehearsal receipt count")
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn subject_json_ceiling_accepts_the_limit_and_rejects_the_next_byte_atomically() {
    let pool = pool().await;
    let source = source(&pool).await;
    let maximum = object_at_jsonb_ceiling(SUBJECT_MAX_BYTES);
    let just_over = object_at_jsonb_ceiling(SUBJECT_MAX_BYTES + 1);
    let mut transaction = app(&pool, source.tenant).await;
    assert_jsonb_bytes(&mut transaction, &maximum, SUBJECT_MAX_BYTES).await;
    assert_jsonb_bytes(&mut transaction, &just_over, SUBJECT_MAX_BYTES + 1).await;
    assert!(
        start_with_subject(&mut transaction, source, id(), &maximum)
            .await
            .is_some(),
        "maximum subject JSON is admitted"
    );
    assert!(
        start_with_subject(&mut transaction, source, id(), &just_over)
            .await
            .is_none(),
        "just-over subject JSON is rejected before a replacement run is created"
    );
    let active: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_run WHERE tenant_id=$1 AND assignment_id=$2 AND lifecycle='active'",
    )
    .bind(source.tenant)
    .bind(source.assignment)
    .fetch_one(&mut *transaction)
    .await
    .expect("subject rejection leaves active projection unchanged");
    assert_eq!(
        active, 1,
        "oversize start changes no active rehearsal state"
    );
    transaction
        .commit()
        .await
        .expect("commit subject bounds probe");
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn frozen_json_ceilings_admit_maximums_and_preserve_the_head_on_rejection() {
    let pool = pool().await;
    let source = source(&pool).await;
    let max_evidence = object_at_jsonb_ceiling(FROZEN_EVIDENCE_MAX_BYTES);
    let max_response = object_at_jsonb_ceiling(FROZEN_RESPONSE_MAX_BYTES);
    let over_evidence = object_at_jsonb_ceiling(FROZEN_EVIDENCE_MAX_BYTES + 1);
    let over_response = object_at_jsonb_ceiling(FROZEN_RESPONSE_MAX_BYTES + 1);
    let mut transaction = app(&pool, source.tenant).await;
    for (value, expected) in [
        (&max_evidence, FROZEN_EVIDENCE_MAX_BYTES),
        (&max_response, FROZEN_RESPONSE_MAX_BYTES),
        (&over_evidence, FROZEN_EVIDENCE_MAX_BYTES + 1),
        (&over_response, FROZEN_RESPONSE_MAX_BYTES + 1),
    ] {
        assert_jsonb_bytes(&mut transaction, value, expected).await;
    }
    let admitted_run = id();
    assert!(
        start_with_subject(&mut transaction, source, admitted_run, &json!({}))
            .await
            .is_some()
    );
    assert!(
        freeze(
            &mut transaction,
            source,
            admitted_run,
            &max_evidence,
            &max_response
        )
        .await,
        "both frozen JSON maximums are admitted together"
    );
    assert_eq!(head_length(&mut transaction, admitted_run).await, 1);

    let response_over_run = id();
    assert!(
        start_with_subject(
            &mut transaction,
            source,
            response_over_run,
            &json!({"s": 1})
        )
        .await
        .is_some()
    );
    assert!(
        !freeze(
            &mut transaction,
            source,
            response_over_run,
            &json!({}),
            &over_response
        )
        .await,
        "just-over response definition is rejected"
    );
    assert_eq!(head_length(&mut transaction, response_over_run).await, 0);
    let evidence_over_run = id();
    assert!(
        start_with_subject(
            &mut transaction,
            source,
            evidence_over_run,
            &json!({"s": 2})
        )
        .await
        .is_some()
    );
    assert!(
        !freeze(
            &mut transaction,
            source,
            evidence_over_run,
            &over_evidence,
            &json!({})
        )
        .await,
        "just-over frozen evidence payload is rejected"
    );
    assert_eq!(head_length(&mut transaction, evidence_over_run).await, 0);
    transaction
        .commit()
        .await
        .expect("commit frozen bounds probe");
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn sealed_request_json_ceiling_rejects_without_claim_root_or_event() {
    let pool = pool().await;
    let source = source(&pool).await;
    let maximum = object_at_jsonb_ceiling(SEALED_REQUEST_MAX_BYTES);
    let just_over = object_at_jsonb_ceiling(SEALED_REQUEST_MAX_BYTES + 1);
    let mut transaction = app(&pool, source.tenant).await;
    assert_jsonb_bytes(&mut transaction, &maximum, SEALED_REQUEST_MAX_BYTES).await;
    assert_jsonb_bytes(&mut transaction, &just_over, SEALED_REQUEST_MAX_BYTES + 1).await;
    let admitted_run = id();
    assert!(
        start_with_subject(&mut transaction, source, admitted_run, &json!({}))
            .await
            .is_some()
    );
    create_dispatched_claim(&mut transaction, source, admitted_run, &maximum).await;
    let rejected_run = id();
    assert!(
        start_with_subject(&mut transaction, source, rejected_run, &json!({"s": 1}))
            .await
            .is_some()
    );
    let rejected: bool = sqlx::query_scalar(
        "SELECT public.ple_rehearsal_create_claim($1,$2,$3,$4,1,$5,$6,$7,'sealed-over',$8,$9,$10)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .bind(rejected_run)
    .bind(id())
    .bind(id())
    .bind(id())
    .bind(bytes(7))
    .bind(&just_over)
    .fetch_one(&mut *transaction)
    .await
    .expect("sealed request rejection result");
    assert!(!rejected, "just-over sealed request is rejected");
    let claim_roots: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_submission_claim_root WHERE rehearsal_run_id=$1",
    )
    .bind(rejected_run)
    .fetch_one(&mut *transaction)
    .await
    .expect("rejected claim root count");
    let claim_events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rehearsal_submission_claim_event WHERE rehearsal_run_id=$1",
    )
    .bind(rejected_run)
    .fetch_one(&mut *transaction)
    .await
    .expect("rejected claim event count");
    assert_eq!(claim_roots, 0, "oversize sealed request creates no root");
    assert_eq!(claim_events, 0, "oversize sealed request creates no event");
    transaction
        .commit()
        .await
        .expect("commit sealed-request bounds probe");
}

async fn complete_with(
    transaction: &mut Transaction<'_, Postgres>,
    source: super::fixture::Source,
    run: uuid::Uuid,
    claim: uuid::Uuid,
    operation: uuid::Uuid,
    payload: &Value,
    projection: &Value,
) -> bool {
    let recorded_at = millis(transaction).await;
    sqlx::query_scalar(
        "SELECT public.ple_rehearsal_complete_claim($1,$2,$3,$4,1,$5,$6,$7,$8,1,$9,$10,$11,$12,$13,$14)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .bind(run)
    .bind(claim)
    .bind(operation)
    .bind(bytes(2))
    .bind(bytes(8))
    .bind(payload)
    .bind(bytes(9))
    .bind(recorded_at)
    .bind(projection)
    .bind(bytes(10))
    .fetch_one(&mut **transaction)
    .await
    .expect("accepted evidence and receipt admission capability")
}

async fn ready_for_completion(
    transaction: &mut Transaction<'_, Postgres>,
    source: super::fixture::Source,
    subject_tag: i64,
) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid) {
    let run = id();
    assert!(
        start_with_subject(transaction, source, run, &json!({"s": subject_tag}))
            .await
            .is_some()
    );
    assert!(freeze(transaction, source, run, &json!({}), &json!({})).await);
    let (claim, operation) = create_dispatched_claim(transaction, source, run, &json!({})).await;
    (run, claim, operation)
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn completion_json_ceilings_preserve_head_claim_and_receipt_on_rejection() {
    let pool = pool().await;
    let source = source(&pool).await;
    let max_evidence = object_at_jsonb_ceiling(ACCEPTED_EVIDENCE_MAX_BYTES);
    let max_projection = object_at_jsonb_ceiling(RECEIPT_PROJECTION_MAX_BYTES);
    let over_evidence = object_at_jsonb_ceiling(ACCEPTED_EVIDENCE_MAX_BYTES + 1);
    let over_projection = object_at_jsonb_ceiling(RECEIPT_PROJECTION_MAX_BYTES + 1);
    let mut transaction = app(&pool, source.tenant).await;
    for (value, expected) in [
        (&max_evidence, ACCEPTED_EVIDENCE_MAX_BYTES),
        (&max_projection, RECEIPT_PROJECTION_MAX_BYTES),
        (&over_evidence, ACCEPTED_EVIDENCE_MAX_BYTES + 1),
        (&over_projection, RECEIPT_PROJECTION_MAX_BYTES + 1),
    ] {
        assert_jsonb_bytes(&mut transaction, value, expected).await;
    }
    let (admitted_run, admitted_claim, admitted_operation) =
        ready_for_completion(&mut transaction, source, 1).await;
    assert!(
        complete_with(
            &mut transaction,
            source,
            admitted_run,
            admitted_claim,
            admitted_operation,
            &max_evidence,
            &max_projection,
        )
        .await,
        "maximum accepted evidence and receipt projection are admitted"
    );
    assert_eq!(head_length(&mut transaction, admitted_run).await, 2);
    assert_eq!(receipt_count(&mut transaction, admitted_run).await, 1);

    let (evidence_run, evidence_claim, evidence_operation) =
        ready_for_completion(&mut transaction, source, 2).await;
    assert!(
        !complete_with(
            &mut transaction,
            source,
            evidence_run,
            evidence_claim,
            evidence_operation,
            &over_evidence,
            &json!({}),
        )
        .await,
        "just-over accepted evidence is rejected"
    );
    assert_eq!(
        head_length(&mut transaction, evidence_run).await,
        1,
        "rejected accepted evidence preserves head"
    );
    assert_eq!(
        receipt_count(&mut transaction, evidence_run).await,
        0,
        "rejected accepted evidence creates no receipt"
    );
    let evidence_phase: String = sqlx::query_scalar(
        "SELECT phase FROM rehearsal_submission_claim_event WHERE rehearsal_run_id=$1 ORDER BY sequence DESC LIMIT 1",
    )
    .bind(evidence_run)
    .fetch_one(&mut *transaction)
    .await
    .expect("rejected accepted evidence retains dispatched claim");
    assert_eq!(
        evidence_phase, "gradingDispatched",
        "oversize accepted evidence leaves the claim phase unchanged"
    );
    let (projection_run, projection_claim, projection_operation) =
        ready_for_completion(&mut transaction, source, 3).await;
    assert!(
        !complete_with(
            &mut transaction,
            source,
            projection_run,
            projection_claim,
            projection_operation,
            &json!({}),
            &over_projection,
        )
        .await,
        "just-over receipt projection is rejected"
    );
    assert_eq!(
        head_length(&mut transaction, projection_run).await,
        1,
        "rejected receipt projection preserves head"
    );
    assert_eq!(
        receipt_count(&mut transaction, projection_run).await,
        0,
        "rejected receipt projection creates no receipt"
    );
    let projection_phase: String = sqlx::query_scalar(
        "SELECT phase FROM rehearsal_submission_claim_event WHERE rehearsal_run_id=$1 ORDER BY sequence DESC LIMIT 1",
    )
    .bind(projection_run)
    .fetch_one(&mut *transaction)
    .await
    .expect("rejected receipt projection retains dispatched claim");
    assert_eq!(
        projection_phase, "gradingDispatched",
        "oversize receipt projection leaves the claim phase unchanged"
    );
    transaction
        .commit()
        .await
        .expect("commit completion bounds probe");

    let mut alternate = app(&pool, source.tenant).await;
    let bypass = sqlx::query_scalar::<_, bool>(
        "SELECT public.ple_rehearsal_append_evidence($1,$2,$3,$4,1,$5,$6,1,'acceptedSubmission',$7,$8,$9,$10)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .bind(evidence_run)
    .bind(bytes(2))
    .bind(bytes(11))
    .bind(&over_evidence)
    .bind(bytes(12))
    .bind(0_i64)
    .fetch_one(&mut *alternate)
    .await;
    assert!(
        bypass.is_err(),
        "application callers cannot bypass completion through the inner evidence primitive"
    );
    alternate
        .rollback()
        .await
        .expect("close denied inner primitive call");
}
