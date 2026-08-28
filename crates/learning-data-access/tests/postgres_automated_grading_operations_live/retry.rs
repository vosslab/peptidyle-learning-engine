//! Connected proof of the Instructor retry capability and its replay fence.

use super::broker::{InstructorBroker, admin_tenant_transaction};
use super::fresh_uuid;
use question_model::{QuestionAttemptId, TenantId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub(super) struct RetryScenario<'pool, 'broker> {
    pub(super) pool: &'pool PgPool,
    pub(super) tenant: TenantId,
    pub(super) operation: i32,
    pub(super) attempt: QuestionAttemptId,
    pub(super) submission: Uuid,
    pub(super) instructor_broker: &'broker InstructorBroker<'pool>,
    pub(super) rotated_instructor_broker: &'broker InstructorBroker<'pool>,
    pub(super) other_instructor_broker: &'broker InstructorBroker<'pool>,
}

pub(super) async fn prove_retry_and_replay(scenario: RetryScenario<'_, '_>) {
    let RetryScenario {
        pool,
        tenant,
        operation,
        attempt,
        submission,
        instructor_broker,
        rotated_instructor_broker,
        other_instructor_broker,
    } = scenario;
    // The ceiling is a durable execution boundary, not merely an API hint:
    // once reached, the same operation remains exceptioned and allocates no
    // queue work or receipt.
    let mut ceiling_update = admin_tenant_transaction(pool, tenant).await;
    sqlx::query(
        "UPDATE public.grading_execution SET retry_count=20 \
         WHERE tenant_id=$1 AND attempt_id=$2 AND submission_id=$3",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(submission)
    .execute(&mut *ceiling_update)
    .await
    .expect("set connected retry ceiling");
    ceiling_update
        .commit()
        .await
        .expect("commit connected retry ceiling");

    let before_ceiling = sqlx::query(
        "SELECT execution_generation, state, current_job_id, retry_count \
         FROM public.grading_execution \
         WHERE tenant_id=$1 AND attempt_id=$2 AND submission_id=$3",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(submission)
    .fetch_one(pool)
    .await
    .expect("read retry ceiling execution");
    let before_receipts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.grading_execution_receipt \
         WHERE tenant_id=$1 AND attempt_id=$2 AND submission_id=$3",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(submission)
    .fetch_one(pool)
    .await
    .expect("count retry ceiling receipts");
    let before_jobs: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.worker_job \
         WHERE tenant_id=$1 AND payload->>'attempt'=$2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.to_string())
    .fetch_one(pool)
    .await
    .expect("count retry ceiling jobs");
    assert!(
        instructor_broker
            .retry(operation, 1, fresh_uuid())
            .await
            .is_err(),
        "retry ceiling rejects a new Instructor action"
    );
    let after_ceiling = sqlx::query(
        "SELECT execution_generation, state, current_job_id, retry_count \
         FROM public.grading_execution \
         WHERE tenant_id=$1 AND attempt_id=$2 AND submission_id=$3",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(submission)
    .fetch_one(pool)
    .await
    .expect("read post-ceiling execution");
    assert_eq!(
        before_ceiling
            .try_get::<i64, _>("execution_generation")
            .unwrap(),
        after_ceiling
            .try_get::<i64, _>("execution_generation")
            .unwrap(),
        "retry ceiling preserves generation"
    );
    assert_eq!(
        before_ceiling.try_get::<String, _>("state").unwrap(),
        after_ceiling.try_get::<String, _>("state").unwrap(),
        "retry ceiling preserves exception state"
    );
    assert_eq!(
        before_ceiling.try_get::<Uuid, _>("current_job_id").unwrap(),
        after_ceiling.try_get::<Uuid, _>("current_job_id").unwrap(),
        "retry ceiling preserves current job"
    );
    assert_eq!(
        after_ceiling.try_get::<i32, _>("retry_count").unwrap(),
        20,
        "retry ceiling preserves the maximum retry count"
    );
    assert_eq!(
        before_receipts,
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM public.grading_execution_receipt \
             WHERE tenant_id=$1 AND attempt_id=$2 AND submission_id=$3",
        )
        .bind(tenant.as_uuid())
        .bind(attempt.as_uuid())
        .bind(submission)
        .fetch_one(pool)
        .await
        .expect("count post-ceiling receipts"),
        "retry ceiling allocates no execution receipt"
    );
    assert_eq!(
        before_jobs,
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM public.worker_job \
             WHERE tenant_id=$1 AND payload->>'attempt'=$2",
        )
        .bind(tenant.as_uuid())
        .bind(attempt.to_string())
        .fetch_one(pool)
        .await
        .expect("count post-ceiling jobs"),
        "retry ceiling allocates no worker job"
    );

    let mut reset_ceiling = admin_tenant_transaction(pool, tenant).await;
    sqlx::query(
        "UPDATE public.grading_execution SET retry_count=0 \
         WHERE tenant_id=$1 AND attempt_id=$2 AND submission_id=$3",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(submission)
    .execute(&mut *reset_ceiling)
    .await
    .expect("reset connected retry ceiling");
    reset_ceiling
        .commit()
        .await
        .expect("commit retry ceiling reset");

    let action = fresh_uuid();
    let accepted_retry = instructor_broker
        .retry(operation, 1, action)
        .await
        .expect("Instructor retry accepted");
    assert_eq!(
        accepted_retry.try_get::<String, _>("disposition").unwrap(),
        "accepted"
    );
    assert_eq!(
        accepted_retry
            .try_get::<i64, _>("resulting_execution_generation")
            .unwrap(),
        2
    );
    let execution_after_retry = sqlx::query(
        "SELECT execution_generation, state, retry_count, current_job_id \
         FROM public.grading_execution \
         WHERE tenant_id=$1 AND attempt_id=$2 AND submission_id=$3",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(submission)
    .fetch_one(pool)
    .await
    .expect("read accepted retry execution");
    assert_eq!(
        execution_after_retry
            .try_get::<i64, _>("execution_generation")
            .unwrap(),
        2,
        "accepted retry advances one execution generation"
    );
    assert_eq!(
        execution_after_retry.try_get::<String, _>("state").unwrap(),
        "ready"
    );
    assert_eq!(
        execution_after_retry
            .try_get::<i32, _>("retry_count")
            .unwrap(),
        1,
        "accepted retry increments retry count exactly once"
    );
    assert_eq!(
        execution_after_retry
            .try_get::<Uuid, _>("current_job_id")
            .unwrap(),
        action,
        "accepted retry owns its deterministic job"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i32>(
            "SELECT max_attempts FROM public.worker_job WHERE tenant_id=$1 AND job_id=$2",
        )
        .bind(tenant.as_uuid())
        .bind(action)
        .fetch_one(pool)
        .await
        .expect("read deterministic retry job budget"),
        3,
        "retry job uses the bounded accepted-submission budget"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM public.grading_execution_receipt \
             WHERE tenant_id=$1 AND attempt_id=$2 AND submission_id=$3 \
               AND execution_generation=2 AND resulting_state='ready' AND worker_id IS NULL",
        )
        .bind(tenant.as_uuid())
        .bind(attempt.as_uuid())
        .bind(submission)
        .fetch_one(pool)
        .await
        .expect("count generation-two retry receipt"),
        1,
        "accepted retry emits one worker-unassigned generation-two receipt"
    );
    let jobs_after_retry: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.worker_job \
         WHERE tenant_id=$1 AND payload->>'attempt'=$2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.to_string())
    .fetch_one(pool)
    .await
    .expect("count jobs after accepted retry");
    let receipts_after_retry: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM public.grading_execution_receipt \
         WHERE tenant_id=$1 AND attempt_id=$2 AND submission_id=$3",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(submission)
    .fetch_one(pool)
    .await
    .expect("count receipts after accepted retry");

    let replayed_retry = rotated_instructor_broker
        .retry(operation, 1, action)
        .await
        .expect("same actor replay through rotated session");
    assert_eq!(
        replayed_retry.try_get::<String, _>("disposition").unwrap(),
        "replayed"
    );
    assert_eq!(
        jobs_after_retry,
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM public.worker_job \
             WHERE tenant_id=$1 AND payload->>'attempt'=$2",
        )
        .bind(tenant.as_uuid())
        .bind(attempt.to_string())
        .fetch_one(pool)
        .await
        .expect("count replayed retry jobs"),
        "retry replay allocates no extra worker job"
    );
    assert_eq!(
        receipts_after_retry,
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM public.grading_execution_receipt \
             WHERE tenant_id=$1 AND attempt_id=$2 AND submission_id=$3",
        )
        .bind(tenant.as_uuid())
        .bind(attempt.as_uuid())
        .bind(submission)
        .fetch_one(pool)
        .await
        .expect("count replayed retry receipts"),
        "retry replay emits no extra execution receipt"
    );
    assert!(
        other_instructor_broker
            .retry(operation, 1, action)
            .await
            .is_err(),
        "a different Instructor cannot replay another actor's action"
    );
    assert!(
        instructor_broker
            .retry(operation, 1, fresh_uuid())
            .await
            .is_err(),
        "stale retry revision is fenced"
    );
}
