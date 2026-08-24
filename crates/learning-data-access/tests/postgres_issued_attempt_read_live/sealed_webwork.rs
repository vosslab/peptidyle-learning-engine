//! Grader-only WebWork private-execution probes for the issued-read oracle.

use super::*;

pub(super) struct SealedWebworkFixture {
    pub(super) context: TenantContext,
    pub(super) tenant: TenantId,
    pub(super) student: UserId,
    pub(super) binding: LearnerWorkRoutingBinding,
    pub(super) attempt: QuestionAttemptId,
    pub(super) mismatched_attempt: QuestionAttemptId,
}

pub(super) async fn assert_sealed_webwork_execution(
    pool: &PgPool,
    store: &PostgresStore,
    grader: &PostgresGraderStore,
    fixture: SealedWebworkFixture,
) {
    let SealedWebworkFixture {
        context,
        tenant,
        student,
        binding,
        attempt,
        mismatched_attempt,
    } = fixture;
    let mut app_visibility = pool
        .begin()
        .await
        .expect("begin application visibility probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *app_visibility)
        .await
        .expect("assume ordinary application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *app_visibility)
        .await
        .expect("bind application probe tenant");
    let private_visible: bool = sqlx::query_scalar(
        "SELECT has_table_privilege(current_user, 'public.issued_attempt_private_execution', 'SELECT')",
    )
    .fetch_one(&mut *app_visibility)
    .await
    .expect("read private-table application grant catalog");
    assert!(
        !private_visible,
        "ordinary application role has no sealed private-execution SELECT grant"
    );
    assert!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM public.issued_attempt_private_execution WHERE tenant_id=$1 AND attempt_id=$2",
        )
        .bind(tenant.as_uuid())
        .bind(attempt.as_uuid())
        .fetch_one(&mut *app_visibility)
        .await
        .is_err(),
        "ordinary application role cannot discover an issued WebWork private contract"
    );
    app_visibility
        .rollback()
        .await
        .expect("roll back application visibility probe");

    let idempotency_key =
        SubmissionIdempotencyKey::parse("issued-read-webwork-sealed").expect("key");
    let ordinary_preparation = store
        .prepare_question_submission(
            context,
            student,
            binding,
            attempt,
            &StudentResponse::Numeric { value: 3.0 },
            &idempotency_key,
        )
        .await
        .expect("ordinary Store authorizes the first effect without exposing WebWork execution");
    let SubmissionPreparation::FirstEffect(authorized_intent) = ordinary_preparation else {
        panic!("fresh WebWork attempt authorizes a first grading effect");
    };
    let webwork_intent = *authorized_intent;
    let sealed_preparation = grader
        .prepare_sealed_private_execution(
            context,
            student,
            binding,
            webwork_intent.clone(),
            &StudentResponse::Numeric { value: 3.0 },
            &idempotency_key,
        )
        .await
        .expect("dedicated grader facade resolves sealed WebWork execution");
    let SealedPrivateExecutionPreparation::Grade(prepared_submission) = sealed_preparation else {
        panic!("fresh sealed WebWork preparation is a grading effect");
    };
    assert!(
        prepared_submission.webwork_grading.is_some(),
        "only the dedicated grader facade receives the immutable WebWork grading contract"
    );
    assert!(
        prepared_submission.webwork_replay.is_some(),
        "only the dedicated grader facade receives the replay mapping bound to the issued presentation"
    );

    let mut mismatched_intent = webwork_intent.clone();
    mismatched_intent.attempt.id = mismatched_attempt;
    let mismatched_sealed = grader
        .prepare_sealed_private_execution(
            context,
            student,
            binding,
            mismatched_intent,
            &StudentResponse::Numeric { value: 3.0 },
            &idempotency_key,
        )
        .await;
    assert!(
        matches!(
            mismatched_sealed,
            Err(StoreError::NotFound | StoreError::Forbidden | StoreError::Unavailable(_))
        ),
        "sealed WebWork projection refuses a route/attempt-mismatched authorized intent: {mismatched_sealed:?}"
    );

    sqlx::query("ALTER TABLE public.issued_attempt_private_execution DISABLE TRIGGER ALL")
        .execute(pool)
        .await
        .expect("open isolated sealed replay corruption probe");
    sqlx::query(
        "UPDATE public.issued_attempt_private_execution \
         SET webwork_replay_payload_sha256='0' || substr(webwork_replay_payload_sha256, 2) \
         WHERE tenant_id=$1 AND attempt_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .execute(pool)
    .await
    .expect("corrupt isolated sealed replay checksum");
    sqlx::query("ALTER TABLE public.issued_attempt_private_execution ENABLE TRIGGER ALL")
        .execute(pool)
        .await
        .expect("restore sealed replay immutability trigger");
    let tampered_sealed = grader
        .prepare_sealed_private_execution(
            context,
            student,
            binding,
            webwork_intent.clone(),
            &StudentResponse::Numeric { value: 3.0 },
            &idempotency_key,
        )
        .await;
    assert!(
        matches!(tampered_sealed, Err(StoreError::Unavailable(_))),
        "checksum-tampered sealed WebWork replay fails closed: {tampered_sealed:?}"
    );

    sqlx::query(
        "DELETE FROM public.issued_attempt_private_execution WHERE tenant_id=$1 AND attempt_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .execute(pool)
    .await
    .expect("delete isolated sealed private execution child");
    let missing_sealed = grader
        .prepare_sealed_private_execution(
            context,
            student,
            binding,
            webwork_intent,
            &StudentResponse::Numeric { value: 3.0 },
            &idempotency_key,
        )
        .await;
    assert!(
        matches!(
            missing_sealed,
            Err(StoreError::NotFound | StoreError::Forbidden | StoreError::Unavailable(_))
        ),
        "missing sealed WebWork execution fails closed: {missing_sealed:?}"
    );
}
