//! Immutable grading-operation receipt subtype integrity proof for the W7 oracle.

use question_model::{CourseId, QuestionAttemptId, TenantId, UserId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::broker::admin_tenant_transaction;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ExecutionReceiptSnapshot {
    receipt_id: Uuid,
    attempt_id: Uuid,
    submission_id: Uuid,
    submission_occurred_at: String,
    course_id: Uuid,
    execution_generation: i64,
    resulting_state: String,
    safe_category: String,
    actor_id: Option<Uuid>,
    worker_id: Option<Uuid>,
}

pub(super) struct ExecutionReceiptExpectation<'a> {
    pub(super) attempt: QuestionAttemptId,
    pub(super) submission: Uuid,
    pub(super) generation: i64,
    pub(super) category: &'a str,
    pub(super) state: &'a str,
    pub(super) actor: Option<Uuid>,
    pub(super) worker: Option<Uuid>,
}

pub(super) async fn assert_execution_receipt(
    pool: &PgPool,
    tenant: TenantId,
    expectation: ExecutionReceiptExpectation<'_>,
) -> ExecutionReceiptSnapshot {
    let ExecutionReceiptExpectation {
        attempt,
        submission,
        generation,
        category,
        state,
        actor,
        worker,
    } = expectation;
    let row = sqlx::query(
        "SELECT receipt_id, attempt_id, submission_id, submission_occurred_at::text AS submission_occurred_at, course_id, \
                execution_generation, resulting_state, safe_category, actor_id, worker_id \
         FROM public.grading_execution_receipt \
         WHERE tenant_id=$1 AND attempt_id=$2 AND submission_id=$3 \
           AND execution_generation=$4 AND safe_category=$5",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(submission)
    .bind(generation)
    .bind(category)
    .fetch_one(pool)
    .await
    .expect("read required W7 execution receipt");
    let receipt = ExecutionReceiptSnapshot {
        receipt_id: row.try_get("receipt_id").expect("receipt identity"),
        attempt_id: row.try_get("attempt_id").expect("receipt attempt"),
        submission_id: row.try_get("submission_id").expect("receipt submission"),
        submission_occurred_at: row
            .try_get("submission_occurred_at")
            .expect("receipt submission timestamp"),
        course_id: row.try_get("course_id").expect("receipt course"),
        execution_generation: row
            .try_get("execution_generation")
            .expect("receipt generation"),
        resulting_state: row.try_get("resulting_state").expect("receipt state"),
        safe_category: row.try_get("safe_category").expect("receipt category"),
        actor_id: row.try_get("actor_id").expect("receipt actor"),
        worker_id: row.try_get("worker_id").expect("receipt worker"),
    };
    assert_eq!(receipt.execution_generation, generation);
    assert_eq!(receipt.safe_category, category);
    assert_eq!(receipt.resulting_state, state);
    assert_eq!(receipt.actor_id, actor);
    assert_eq!(receipt.worker_id, worker);
    assert_ne!(receipt.actor_id.is_some(), receipt.worker_id.is_some());
    receipt
}

pub(super) async fn assert_receipt_unchanged(
    pool: &PgPool,
    tenant: TenantId,
    original: &ExecutionReceiptSnapshot,
) {
    let current = sqlx::query(
        "SELECT receipt_id, attempt_id, submission_id, submission_occurred_at::text AS submission_occurred_at, course_id, \
                execution_generation, resulting_state, safe_category, actor_id, worker_id \
         FROM public.grading_execution_receipt WHERE tenant_id=$1 AND receipt_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(original.receipt_id)
    .fetch_one(pool)
    .await
    .expect("read original W7 acceptance receipt after later transitions");
    let current = ExecutionReceiptSnapshot {
        receipt_id: current.try_get("receipt_id").expect("receipt identity"),
        attempt_id: current.try_get("attempt_id").expect("receipt attempt"),
        submission_id: current
            .try_get("submission_id")
            .expect("receipt submission"),
        submission_occurred_at: current
            .try_get("submission_occurred_at")
            .expect("receipt submission timestamp"),
        course_id: current.try_get("course_id").expect("receipt course"),
        execution_generation: current
            .try_get("execution_generation")
            .expect("receipt generation"),
        resulting_state: current.try_get("resulting_state").expect("receipt state"),
        safe_category: current.try_get("safe_category").expect("receipt category"),
        actor_id: current.try_get("actor_id").expect("receipt actor"),
        worker_id: current.try_get("worker_id").expect("receipt worker"),
    };
    assert_eq!(&current, original, "accepted receipt remains immutable");
}

pub(super) async fn prove_subtype_shape_rejection(
    pool: &PgPool,
    tenant: TenantId,
    operation: i32,
    course: CourseId,
    actor: UserId,
) {
    // Each candidate owns a transaction because PostgreSQL marks a transaction
    // failed after a CHECK violation. The missing-field cases cover SQL NULL
    // propagation; the cross-subtype cases cover unrelated-field fences.
    let candidates = [
        (
            "retry requires its resulting operation revision",
            "retry",
            Some(2_i64),
            None,
            Some(2_i64),
            None,
            Some("ready"),
            None,
            None,
        ),
        (
            "retry rejects recalculation-only fields",
            "retry",
            Some(1_i64),
            Some(2_i64),
            Some(2_i64),
            None,
            Some("ready"),
            Some(1_i64),
            None,
        ),
        (
            "recalculation requires its expected assignment revision",
            "recalculate",
            None,
            Some(1_i64),
            None,
            Some(2_i64),
            Some("recalculating"),
            None,
            Some(1_i64),
        ),
        (
            "recalculation rejects retry-only fields",
            "recalculate",
            Some(1_i64),
            Some(2_i64),
            None,
            Some(2_i64),
            Some("recalculating"),
            Some(1_i64),
            Some(1_i64),
        ),
    ];
    for (
        index,
        (
            label,
            action_kind,
            retry_expected,
            retry_resulting,
            execution_generation,
            scoring_generation,
            resulting_state,
            recalculate_expected,
            recalculate_created,
        ),
    ) in candidates.into_iter().enumerate()
    {
        let index = u128::try_from(index).expect("receipt candidate index fits u128");
        let action = Uuid::from_u128(0x7a11_0000_0000_4001_8000_0000_0000_0000 | index);
        let mut transaction = admin_tenant_transaction(pool, tenant).await;
        let error = sqlx::query(
            "INSERT INTO public.grading_operation_receipt (\
             tenant_id, action_id, grading_operation_id, course_id, actor_id, action_kind,\
             safe_category,\
             request_sha256, resulting_execution_generation, resulting_scoring_generation,\
             resulting_state, retry_expected_operation_revision, retry_resulting_operation_revision,\
             recalculate_expected_assignment_revision, recalculate_created_operation_revision)\
             VALUES ($1,$2,$3,$4,$5,$6,$7,repeat('a',64),$8,$9,$10,$11,$12,$13,$14)",
        )
        .bind(tenant.as_uuid())
        .bind(action)
        .bind(i64::from(operation))
        .bind(course.as_uuid())
        .bind(actor.as_uuid())
        .bind(action_kind)
        .bind(if action_kind == "retry" {
            "instructor_retry"
        } else {
            "instructor_recalculation"
        })
        .bind(execution_generation)
        .bind(scoring_generation)
        .bind(resulting_state)
        .bind(retry_expected)
        .bind(retry_resulting)
        .bind(recalculate_expected)
        .bind(recalculate_created)
        .execute(&mut *transaction)
        .await
        .expect_err("malformed receipt must violate a PostgreSQL CHECK constraint");
        assert_eq!(
            error
                .as_database_error()
                .and_then(|database_error| database_error.code())
                .as_deref(),
            Some("23514"),
            "malformed receipt must fail its CHECK constraint: {label}"
        );
        transaction
            .rollback()
            .await
            .expect("rollback malformed receipt probe");
    }
}
