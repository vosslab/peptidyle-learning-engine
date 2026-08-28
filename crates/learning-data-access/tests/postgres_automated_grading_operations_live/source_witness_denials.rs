//! Independent denial probes for sealed invalidation capabilities.

use question_model::{AssignmentId, CourseId, TenantId, UserId};
use sqlx::PgPool;
use uuid::Uuid;

use super::{app_transaction, fresh_uuid};

pub(super) struct SourceWitnessDenialScenario<'a> {
    pub(super) pool: &'a PgPool,
    pub(super) tenant: TenantId,
    pub(super) session: &'a str,
    pub(super) instructor: UserId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) assignment_revision: i64,
    pub(super) original_execution_job: Uuid,
    pub(super) attempt: Uuid,
    pub(super) submission: Uuid,
}

pub(super) async fn prove_source_witness_denials(scenario: SourceWitnessDenialScenario<'_>) {
    let SourceWitnessDenialScenario {
        pool,
        tenant,
        session,
        instructor,
        course,
        assignment,
        assignment_revision,
        original_execution_job,
        attempt,
        submission,
    } = scenario;
    let mut app = app_transaction(pool, tenant, session).await;
    assert!(
        sqlx::query(
            "SELECT * FROM public.ple_request_scoring_invalidation_v1(\
             $1,$2,$3,'instructor_recalculation',$4,$4,$5,10)",
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(assignment.as_uuid())
        .bind(fresh_uuid())
        .bind(instructor.as_uuid())
        .fetch_all(&mut *app)
        .await
        .is_err(),
        "ple_app cannot allocate a scoring invalidation directly"
    );
    app.rollback()
        .await
        .expect("rollback denied generic invalidation request");
    let mut app = app_transaction(pool, tenant, session).await;
    assert!(
        sqlx::query(
            "SELECT * FROM public.ple_bind_scoring_invalidation_origin_v1(\
             $1,$2,$3,1,$4,'instructor_recalculation',$5,$6,NULL)",
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(assignment.as_uuid())
        .bind(fresh_uuid())
        .bind(fresh_uuid())
        .bind(instructor.as_uuid())
        .fetch_all(&mut *app)
        .await
        .is_err(),
        "ple_app cannot bind an arbitrary invalidation origin"
    );
    app.rollback()
        .await
        .expect("rollback denied generic invalidation binding");
    let mut app = app_transaction(pool, tenant, session).await;
    assert!(
        sqlx::query("SELECT * FROM public.ple_bind_manual_grade_invalidation_v1($1,$2,$3)")
            .bind(tenant.as_uuid())
            .bind(fresh_uuid())
            .bind(fresh_uuid())
            .fetch_all(&mut *app)
            .await
            .is_err(),
        "the typed manual-grade wrapper requires its authoritative receipt"
    );
    app.rollback()
        .await
        .expect("rollback denied unproven typed source binding");
    let mut app = app_transaction(pool, tenant, session).await;
    assert!(
        sqlx::query("SELECT * FROM public.ple_bind_attempt_support_invalidation_v1($1,$2,$3)")
            .bind(tenant.as_uuid())
            .bind(fresh_uuid())
            .bind(fresh_uuid())
            .fetch_all(&mut *app)
            .await
            .is_err(),
        "the typed learner-support wrapper requires its authoritative audit event"
    );
    app.rollback()
        .await
        .expect("rollback denied unproven learner-support binding");
    let mut app = app_transaction(pool, tenant, session).await;
    assert!(
        sqlx::query(
            "SELECT * FROM public.ple_bind_assignment_definition_invalidation_v1(\
             $1,$2,$3,$4,$5,$6)",
        )
        .bind(tenant.as_uuid())
        .bind(instructor.as_uuid())
        .bind(course.as_uuid())
        .bind(assignment.as_uuid())
        .bind(assignment_revision)
        .bind(fresh_uuid())
        .fetch_all(&mut *app)
        .await
        .is_err(),
        "the definition wrapper rejects a job without a matching authoritative change"
    );
    app.rollback()
        .await
        .expect("rollback denied unproven definition binding");
    let mut app = app_transaction(pool, tenant, session).await;
    assert!(
        sqlx::query(
            "SELECT * FROM public.ple_bind_accepted_completion_invalidation_v1(\
             $1,$2,$3,$4,$5)",
        )
        .bind(tenant.as_uuid())
        .bind(original_execution_job)
        .bind(submission)
        .bind(1_i64)
        .bind(fresh_uuid())
        .fetch_all(&mut *app)
        .await
        .is_err(),
        "ple_app cannot invoke the worker-only accepted-completion wrapper"
    );
    app.rollback()
        .await
        .expect("rollback denied worker-only completion binding");
    let mut app = app_transaction(pool, tenant, session).await;
    assert!(
        sqlx::query("SELECT * FROM public.ple_prepare_accepted_submission_retry_v1($1,$2,$3,$4)",)
            .bind(tenant.as_uuid())
            .bind(attempt)
            .bind(submission)
            .bind(fresh_uuid())
            .fetch_all(&mut *app)
            .await
            .is_err(),
        "ple_app cannot invoke the worker-owned retry preparation capability"
    );
    app.rollback()
        .await
        .expect("rollback denied direct worker-owned retry preparation");
}
