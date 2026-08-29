//! W7 authority and denial probes for sealed invalidation capabilities.

use question_model::{AssignmentId, CourseId, TenantId, UserId};
use sqlx::{PgPool, Row};
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
    let catalog = sqlx::query(
        "SELECT to_regprocedure('public.ple_prepare_accepted_submission_retry_v1(uuid,uuid,uuid,uuid)') \
                    IS NULL AS v1_absent, \
                to_regprocedure('public.ple_prepare_accepted_submission_retry_v2(uuid,uuid,uuid,uuid,uuid)') \
                    IS NOT NULL AS v2_present, \
                pg_get_userbyid(proowner) AS owner_name, prosecdef, proconfig, \
                COALESCE(( \
                    SELECT array_agg( \
                        format('%s:%s:%s', \
                            CASE WHEN privilege.grantee = 0 THEN 'PUBLIC' ELSE grantee.rolname END, \
                            privilege.privilege_type, \
                            CASE WHEN privilege.is_grantable THEN 'true' ELSE 'false' END) \
                        ORDER BY privilege.grantee, privilege.privilege_type) \
                    FROM aclexplode(COALESCE(proacl, acldefault('f', proowner))) AS privilege \
                    LEFT JOIN pg_roles AS grantee ON grantee.oid = privilege.grantee \
                    WHERE privilege.grantee <> proowner \
                ), ARRAY[]::text[]) AS execute_grantees \
         FROM pg_proc \
         WHERE oid='public.ple_prepare_accepted_submission_retry_v2(uuid,uuid,uuid,uuid,uuid)'::regprocedure",
    )
    .fetch_one(pool)
    .await
    .expect("read retry V2 catalog authority");
    assert!(
        catalog
            .try_get::<bool, _>("v1_absent")
            .expect("V1 retirement"),
        "retired four-input retry capability is absent"
    );
    assert!(
        catalog
            .try_get::<bool, _>("v2_present")
            .expect("V2 presence"),
        "five-input retry capability is present"
    );
    assert_eq!(
        catalog
            .try_get::<String, _>("owner_name")
            .expect("V2 owner"),
        "ple_accepted_submission_execution_worker"
    );
    assert!(
        catalog
            .try_get::<bool, _>("prosecdef")
            .expect("V2 SECURITY DEFINER"),
        "retry V2 is SECURITY DEFINER"
    );
    assert_eq!(
        catalog
            .try_get::<Option<Vec<String>>, _>("proconfig")
            .expect("V2 search path"),
        Some(vec!["search_path=pg_catalog, public, pg_temp".to_string()])
    );
    assert_eq!(
        catalog
            .try_get::<Vec<String>, _>("execute_grantees")
            .expect("V2 execute grantees"),
        vec!["ple_instructor_grading_operation_broker:EXECUTE:false".to_string()],
        "V2 has one broker-only non-owner EXECUTE grant"
    );
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
    let error = sqlx::query(
        "SELECT * FROM public.ple_prepare_accepted_submission_retry_v2($1,$2,$3,$4,$5)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt)
    .bind(submission)
    .bind(original_execution_job)
    .bind(instructor.as_uuid())
    .fetch_all(&mut *app)
    .await
    .expect_err("ple_app cannot invoke the worker-owned retry preparation capability");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|database_error| database_error.code())
            .as_deref(),
        Some("42501"),
        "retry preparation denial must be PostgreSQL permission denied"
    );
    app.rollback()
        .await
        .expect("rollback denied direct worker-owned retry preparation");
}
