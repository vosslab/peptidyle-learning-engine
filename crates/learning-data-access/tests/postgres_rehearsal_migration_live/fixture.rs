use learning_data_access::postgres::{apply_migrations, lazy_pool, migration_status};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

pub fn bytes(value: u8) -> Vec<u8> {
    vec![value; 32]
}

pub async fn pool() -> PgPool {
    let url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL names the disposable PostgreSQL database");
    let pool = lazy_pool(&url).expect("valid disposable PostgreSQL URL");
    apply_migrations(&pool)
        .await
        .expect("full migration epoch applies");
    let first = migration_status(&pool).await.expect("migration status");
    apply_migrations(&pool)
        .await
        .expect("migration epoch converges");
    let second = migration_status(&pool)
        .await
        .expect("converged migration status");
    assert_eq!(first, second, "migration application is convergent");
    let present: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM _sqlx_migrations WHERE success AND version=2026081811",
    )
    .fetch_one(&pool)
    .await
    .expect("T4 migration ledger probe");
    assert_eq!(present, 1, "T4 rehearsal migration is present exactly once");
    pool
}

#[derive(Clone, Copy)]
pub struct Source {
    pub tenant: Uuid,
    pub course: Uuid,
    pub assignment: Uuid,
    pub actor: Uuid,
    pub membership: Uuid,
    pub co_instructor: Uuid,
    pub co_membership: Uuid,
    pub assignment_reference: i32,
}

pub async fn source(pool: &PgPool) -> Source {
    let mut fixture = pool
        .begin()
        .await
        .expect("begin source fixture transaction");
    let source = Source {
        tenant: id(),
        course: id(),
        assignment: id(),
        actor: id(),
        membership: id(),
        co_instructor: id(),
        co_membership: id(),
        assignment_reference: 0,
    };
    sqlx::query(
        "INSERT INTO public.course (tenant_id, course_id, title, term_start_date, term_end_date, time_zone) \
         VALUES ($1, $2, 'T4 SQL oracle', DATE '2026-08-24', DATE '2026-12-18', 'America/Chicago')",
    )
    .bind(source.tenant)
    .bind(source.course)
    .execute(&mut *fixture)
    .await
    .expect("owner creates oracle course");
    sqlx::query(
        "INSERT INTO public.course_roster_state (tenant_id, course_id) VALUES ($1,$2) ON CONFLICT (tenant_id, course_id) DO NOTHING",
    )
    .bind(source.tenant)
    .bind(source.course)
    .execute(&mut *fixture)
    .await
    .expect("owner creates roster revision state");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, false)")
        .bind(source.tenant.to_string())
        .execute(&mut *fixture)
        .await
        .expect("owner binds the fixture tenant for guarded source writes");
    sqlx::query(
        "INSERT INTO public.course_member \
         (tenant_id, course_id, user_id, role, course_membership_id, student_id, status, joined_at) \
         VALUES ($1, $2, $3, 'instructor', $4, NULL, 'active', transaction_timestamp())",
    )
    .bind(source.tenant)
    .bind(source.course)
    .bind(source.actor)
    .bind(source.membership)
    .execute(&mut *fixture)
    .await
    .expect("owner creates direct Instructor membership");
    sqlx::query(
        "INSERT INTO public.course_member (tenant_id, course_id, user_id, role, course_membership_id, student_id, status, joined_at) VALUES ($1,$2,$3,'instructor',$4,NULL,'active',transaction_timestamp())",
    )
    .bind(source.tenant)
    .bind(source.course)
    .bind(source.co_instructor)
    .bind(source.co_membership)
    .execute(&mut *fixture)
    .await
    .expect("owner creates co-Instructor for removal authority fixture");
    sqlx::query(
        "INSERT INTO public.assignment \
         (tenant_id, assignment_id, course_id, title, lifecycle, audience_kind, score_disclosure, \
          per_item_correctness_disclosure, feedback_text_disclosure, solution_disclosure, \
          class_statistics_disclosure, revision) \
         VALUES ($1, $2, $3, 'T4 SQL oracle assignment', 'published', 'course_wide', \
                 'after_submit', 'after_submit', 'after_submit', 'after_submit', 'never', 1)",
    )
    .bind(source.tenant)
    .bind(source.assignment)
    .bind(source.course)
    .execute(&mut *fixture)
    .await
    .expect("owner creates published assignment");
    let assignment_reference: i32 = sqlx::query_scalar(
        "SELECT public_id FROM public.assignment WHERE tenant_id=$1 AND assignment_id=$2",
    )
    .bind(source.tenant)
    .bind(source.assignment)
    .fetch_one(&mut *fixture)
    .await
    .expect("generated assignment reference");
    fixture.commit().await.expect("commit source fixture");
    Source {
        assignment_reference,
        ..source
    }
}

pub async fn app<'a>(pool: &'a PgPool, tenant: Uuid) -> Transaction<'a, Postgres> {
    let mut transaction = pool
        .begin()
        .await
        .expect("restricted application transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("activate application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("bind application tenant");
    transaction
}

pub async fn millis(transaction: &mut Transaction<'_, Postgres>) -> i64 {
    sqlx::query_scalar(
        "SELECT (extract(epoch FROM date_trunc('milliseconds', transaction_timestamp())) * 1000)::bigint",
    )
    .fetch_one(&mut **transaction)
    .await
    .expect("database-owned rehearsal millisecond timestamp")
}

pub async fn start(
    transaction: &mut Transaction<'_, Postgres>,
    source: Source,
    run: Uuid,
    subject: u8,
) -> i64 {
    start_with_intent(transaction, source, run, subject, false, None)
        .await
        .expect("first rehearsal start capability")
}

pub async fn start_with_intent(
    transaction: &mut Transaction<'_, Postgres>,
    source: Source,
    run: Uuid,
    subject: u8,
    start_new_after_completion: bool,
    expected_latest_run: Option<Uuid>,
) -> Option<i64> {
    sqlx::query_scalar(
        "SELECT public.ple_rehearsal_start($1,$2,$3,$4,$5,1,'{}'::jsonb,$6,$7,$8,$9,$10)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.assignment)
    .bind(source.assignment_reference)
    .bind(bytes(subject))
    .bind(bytes(0))
    .bind(run)
    .bind(start_new_after_completion)
    .bind(expected_latest_run)
    .fetch_one(&mut **transaction)
    .await
    .expect("rehearsal start capability")
}

pub async fn retention_work(pool: &PgPool, source: Source, disposition: &str) -> (Uuid, Uuid) {
    let mut fixture = pool.begin().await.expect("begin retention-work fixture");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(source.tenant.to_string())
        .execute(&mut *fixture)
        .await
        .expect("bind retention fixture tenant");
    let job = id();
    let lease = id();
    sqlx::query("INSERT INTO course_retention (tenant_id,course_id,ended_at,notify_days,archive_days,delete_days,assignment_disposition,generation,lifecycle) VALUES ($1,$2,transaction_timestamp(),1,2,3,$3,1,'archived')")
        .bind(source.tenant).bind(source.course).bind(disposition).execute(&mut *fixture).await.expect("archived retention state");
    sqlx::query("INSERT INTO worker_job (job_id,tenant_id,payload,state,lease_token,lease_expires_at,max_attempts) VALUES ($1,$2,jsonb_build_object('kind','retention','course',$3::text,'stage','deleteStudentRecords','generation',1),'leased',$4,transaction_timestamp()+interval '5 minutes',1)")
        .bind(job).bind(source.tenant).bind(source.course).bind(lease).execute(&mut *fixture).await.expect("leased retention worker job");
    sqlx::query("INSERT INTO course_retention_stage (tenant_id,course_id,stage,generation,due_at,state,job_id,lease_token,claimed_at) VALUES ($1,$2,'deleteStudentRecords',1,transaction_timestamp(),'started',$3,$4,transaction_timestamp())")
        .bind(source.tenant).bind(source.course).bind(job).bind(lease).execute(&mut *fixture).await.expect("started retention stage");
    sqlx::query("INSERT INTO course_retention_dispatch (tenant_id,course_id,stage,generation,job_id) VALUES ($1,$2,'deleteStudentRecords',1,$3)")
        .bind(source.tenant).bind(source.course).bind(job).execute(&mut *fixture).await.expect("retention dispatch");
    sqlx::query("INSERT INTO course_retention_cleanup_manifest (tenant_id,course_id,generation,stage,job_id,state,object_count) VALUES ($1,$2,1,'deleteStudentRecords',$3,'prepared',0)")
        .bind(source.tenant).bind(source.course).bind(job).execute(&mut *fixture).await.expect("prepared empty cleanup manifest");
    fixture
        .commit()
        .await
        .expect("commit retention-work fixture");
    (job, lease)
}
