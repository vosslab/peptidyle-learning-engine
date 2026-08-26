#![cfg(feature = "postgres")]

//! Disposable PostgreSQL 17 authority oracle for group-purpose policy CAS.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    CourseGroupManagementStore, CourseGroupPurposePolicyRevision, SessionTokenHash, StoreError,
    TenantContext, UpdateCourseGroupPurposePolicyCommand,
};
use question_model::{
    CourseGroupPurpose, CourseGroupPurposePolicy, CourseId, MultipleMembershipPolicy, TenantId,
    UserId,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).expect("fixture UUID");
    Uuid::from_bytes(bytes)
}

#[derive(Clone, Copy)]
struct Fixture {
    tenant: TenantId,
    foreign_tenant: TenantId,
    course: CourseId,
    other_course: CourseId,
    instructor_session: SessionTokenHash,
    student_session: SessionTokenHash,
    expired_session: SessionTokenHash,
}

async fn pool() -> PgPool {
    let runtime = load_acceptance_runtime();
    let url = runtime.admin_url().expose();
    let pool = lazy_pool(url).expect("PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("full migrated application schema is compatible");
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int4")
        .fetch_one(&pool)
        .await
        .expect("PostgreSQL version");
    assert!((170_000..180_000).contains(&version));
    pool
}

async fn account(pool: &PgPool, user: Uuid, label: &str) {
    let email_label = label.to_ascii_lowercase().replace(' ', "-");
    sqlx::query(
        "INSERT INTO public.ple_account \
         (user_id,normalized_email,delivery_email,display_name,platform_roles) \
         VALUES($1,$2,$2,$3,'[]'::jsonb)",
    )
    .bind(user)
    .bind(format!("{email_label}-{}@example.edu", user.simple()))
    .bind(label)
    .execute(pool)
    .await
    .expect("account fixture");
}

async fn course(pool: &PgPool, tenant: TenantId, course: CourseId, instructor: Option<UserId>) {
    let mut transaction = pool.begin().await.expect("course fixture transaction");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("course fixture tenant");
    sqlx::query(
        "INSERT INTO public.course \
         (tenant_id,course_id,title,term_start_date,term_end_date,time_zone) \
         VALUES($1,$2,'Policy authority fixture','2026-08-24','2026-12-18','America/Chicago')",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .execute(&mut *transaction)
    .await
    .expect("course fixture");
    if let Some(instructor) = instructor {
        sqlx::query(
            "INSERT INTO public.course_member \
         (tenant_id,course_id,user_id,role,course_membership_id,status,joined_at) \
         VALUES($1,$2,$3,'instructor',$4,'active',transaction_timestamp())",
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(instructor.as_uuid())
        .bind(id())
        .execute(&mut *transaction)
        .await
        .expect("direct Instructor fixture");
    }
    transaction.commit().await.expect("commit course fixture");
}

async fn session(
    pool: &PgPool,
    tenant: TenantId,
    user: UserId,
    roles: serde_json::Value,
    active: bool,
    token_material: &'static [u8],
) -> SessionTokenHash {
    let token = SessionTokenHash::compute(token_material);
    sqlx::query(
        "INSERT INTO public.auth_session \
         (session_hash,tenant_id,user_id,display_name,roles,created_at,expires_at,revoked_at) \
         VALUES($1,$2,$3,'Policy authority fixture',$4,transaction_timestamp()-interval '2 hours', \
                transaction_timestamp()+CASE WHEN $5 THEN interval '1 hour' ELSE interval '-1 hour' END,NULL)",
    )
    .bind(token.to_string())
    .bind(tenant.as_uuid())
    .bind(user.as_uuid())
    .bind(roles)
    .bind(active)
    .execute(pool)
    .await
    .expect("session fixture");
    token
}

async fn fixture(pool: &PgPool) -> Fixture {
    let tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let student = UserId::from_uuid(id());
    account(pool, instructor.as_uuid(), "Policy Instructor").await;
    account(pool, student.as_uuid(), "Policy Student").await;
    let course_id = CourseId::from_uuid(id());
    let other_course = CourseId::from_uuid(id());
    course(pool, tenant, course_id, Some(instructor)).await;
    course(pool, tenant, other_course, None).await;
    let initial_policies: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT purpose,multiple_membership,revision \
           FROM public.course_group_membership_policy \
          WHERE tenant_id=$1 AND course_id=$2 ORDER BY purpose",
    )
    .bind(tenant.as_uuid())
    .bind(course_id.as_uuid())
    .fetch_all(pool)
    .await
    .expect("course policy aggregate fixture");
    assert_eq!(
        initial_policies,
        vec![
            ("accommodation".to_string(), "allow".to_string(), 1),
            ("cohort".to_string(), "allow".to_string(), 1),
            ("lab".to_string(), "allow".to_string(), 1),
            ("section".to_string(), "warn".to_string(), 1),
            ("work".to_string(), "allow".to_string(), 1),
        ],
        "a live course owns the closed five-purpose policy aggregate"
    );
    Fixture {
        tenant,
        foreign_tenant,
        course: course_id,
        other_course,
        instructor_session: session(
            pool,
            tenant,
            instructor,
            json!(["instructor"]),
            true,
            b"group-policy-authority-instructor",
        )
        .await,
        student_session: session(
            pool,
            tenant,
            student,
            json!(["student"]),
            true,
            b"group-policy-authority-student",
        )
        .await,
        expired_session: session(
            pool,
            tenant,
            instructor,
            json!(["instructor"]),
            false,
            b"group-policy-authority-expired",
        )
        .await,
    }
}

fn command(
    session: SessionTokenHash,
    course: CourseId,
    purpose: CourseGroupPurpose,
    expected_revision: CourseGroupPurposePolicyRevision,
    multiple_membership: MultipleMembershipPolicy,
) -> UpdateCourseGroupPurposePolicyCommand {
    UpdateCourseGroupPurposePolicyCommand {
        session,
        course,
        expected_revision,
        policy: CourseGroupPurposePolicy {
            purpose,
            multiple_membership,
        },
    }
}

async fn app_direct_dml_is_denied(pool: &PgPool, fixture: Fixture) {
    let mut transaction = pool.begin().await.expect("application transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("application role");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(fixture.tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("tenant context");
    assert!(
        sqlx::query(
            "UPDATE public.course_group_membership_policy SET multiple_membership='allow' \
         WHERE tenant_id=$1 AND course_id=$2 AND purpose='section'",
        )
        .bind(fixture.tenant.as_uuid())
        .bind(fixture.course.as_uuid())
        .execute(&mut *transaction)
        .await
        .is_err()
    );
    transaction.rollback().await.expect("probe rollback");
}

async fn authority_catalog_is_exact(pool: &PgPool) {
    let flags: (bool, bool, bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT rolcanlogin,rolsuper,rolcreatedb,rolcreaterole,rolinherit,rolreplication,rolbypassrls \
         FROM pg_roles WHERE rolname='ple_course_group_mutator_broker'",
    )
    .fetch_one(pool)
    .await
    .expect("broker role");
    assert_eq!(flags, (false, false, false, false, false, false, false));
    let memberships: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_auth_members \
         WHERE roleid='ple_course_group_mutator_broker'::regrole \
            OR member='ple_course_group_mutator_broker'::regrole",
    )
    .fetch_one(pool)
    .await
    .expect("broker memberships");
    assert_eq!(memberships, 0);
    let function: (String, bool, Vec<String>, bool, bool) = sqlx::query_as(
        "SELECT owner.rolname,procedure.prosecdef,COALESCE(procedure.proconfig,ARRAY[]::text[]), \
                has_function_privilege('ple_app',procedure.oid,'EXECUTE'), \
                has_function_privilege('public',procedure.oid,'EXECUTE') \
         FROM pg_proc procedure \
         JOIN pg_namespace namespace ON namespace.oid=procedure.pronamespace \
         JOIN pg_roles owner ON owner.oid=procedure.proowner \
         WHERE namespace.nspname='public' \
           AND procedure.oid='public.ple_replace_course_group_purpose_policy_v1(uuid,character(64),uuid,text,text,bigint)'::regprocedure",
    )
    .fetch_one(pool)
    .await
    .expect("policy broker catalog");
    assert_eq!(function.0, "ple_course_group_mutator_broker");
    assert!(function.1);
    assert!(
        function
            .2
            .iter()
            .any(|setting| setting == "search_path=pg_catalog, public, pg_temp")
    );
    assert!(function.3);
    assert!(!function.4);
    let policy_rls: (bool, bool, i64) = sqlx::query_as(
        "SELECT relation.relrowsecurity,relation.relforcerowsecurity, \
                count(policy.*) FILTER (WHERE policy.policyname='course_group_mutator_policy_tenant' \
                  AND policy.roles=ARRAY['ple_course_group_mutator_broker']::name[] \
                  AND policy.cmd='ALL') \
         FROM pg_class relation \
         JOIN pg_namespace namespace ON namespace.oid=relation.relnamespace \
         LEFT JOIN pg_policies policy ON policy.schemaname=namespace.nspname \
             AND policy.tablename=relation.relname \
         WHERE namespace.nspname='public' AND relation.relname='course_group_membership_policy' \
         GROUP BY relation.relrowsecurity,relation.relforcerowsecurity",
    )
    .fetch_one(pool)
    .await
    .expect("policy RLS catalog");
    assert_eq!(policy_rls, (true, true, 1));
    let app_writes: (bool, bool) = sqlx::query_as(
        "SELECT has_table_privilege('ple_app','public.course_group_membership_policy','UPDATE'), \
                has_table_privilege('ple_app','public.course_group_member','DELETE')",
    )
    .fetch_one(pool)
    .await
    .expect("application direct-DML catalog");
    assert_eq!(app_writes, (false, false));
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn postgres_course_group_policy_cas_is_session_bound_and_least_privileged() {
    let pool = pool().await;
    let fixture = fixture(&pool).await;
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x55; 32]);
    let context = TenantContext::from_authenticated_session(fixture.tenant);
    let initial = CourseGroupPurposePolicyRevision::INITIAL;
    let updated = store
        .update_course_group_purpose_policy(
            context,
            command(
                fixture.instructor_session,
                fixture.course,
                CourseGroupPurpose::Section,
                initial,
                MultipleMembershipPolicy::Allow,
            ),
        )
        .await
        .expect("session-bound policy update");
    assert_eq!(updated.revision.value(), 2);
    assert_eq!(
        updated.policy.multiple_membership,
        MultipleMembershipPolicy::Allow
    );

    assert_eq!(
        store
            .update_course_group_purpose_policy(
                context,
                command(
                    fixture.instructor_session,
                    fixture.course,
                    CourseGroupPurpose::Section,
                    initial,
                    MultipleMembershipPolicy::Warn,
                ),
            )
            .await,
        Err(StoreError::Conflict)
    );
    for session in [fixture.student_session, fixture.expired_session] {
        assert_eq!(
            store
                .update_course_group_purpose_policy(
                    context,
                    command(
                        session,
                        fixture.course,
                        CourseGroupPurpose::Lab,
                        initial,
                        MultipleMembershipPolicy::Warn,
                    ),
                )
                .await,
            Err(StoreError::NotFound)
        );
    }
    assert_eq!(
        store
            .update_course_group_purpose_policy(
                context,
                command(
                    fixture.instructor_session,
                    fixture.other_course,
                    CourseGroupPurpose::Lab,
                    initial,
                    MultipleMembershipPolicy::Warn,
                ),
            )
            .await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        store
            .update_course_group_purpose_policy(
                TenantContext::from_authenticated_session(fixture.foreign_tenant),
                command(
                    fixture.instructor_session,
                    fixture.course,
                    CourseGroupPurpose::Lab,
                    initial,
                    MultipleMembershipPolicy::Warn,
                ),
            )
            .await,
        Err(StoreError::NotFound)
    );

    let first = store.update_course_group_purpose_policy(
        context,
        command(
            fixture.instructor_session,
            fixture.course,
            CourseGroupPurpose::Cohort,
            initial,
            MultipleMembershipPolicy::Warn,
        ),
    );
    let second = store.update_course_group_purpose_policy(
        context,
        command(
            fixture.instructor_session,
            fixture.course,
            CourseGroupPurpose::Cohort,
            initial,
            MultipleMembershipPolicy::Warn,
        ),
    );
    let (first, second) = tokio::join!(first, second);
    assert_eq!(
        [first.is_ok(), second.is_ok()]
            .into_iter()
            .filter(|value| *value)
            .count(),
        1,
        "one concurrent CAS owns the expected revision"
    );
    assert!(
        matches!(first, Err(StoreError::Conflict)) || matches!(second, Err(StoreError::Conflict))
    );

    app_direct_dml_is_denied(&pool, fixture).await;
    authority_catalog_is_exact(&pool).await;
}
#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;
