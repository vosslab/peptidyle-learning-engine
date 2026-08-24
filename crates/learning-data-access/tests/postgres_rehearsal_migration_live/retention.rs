use super::fixture::{Source, app, id, pool, retention_work, source, start};
use uuid::Uuid;

struct RetentionWitness {
    generation: i64,
    count: i64,
    run_ids: Vec<Uuid>,
}

async fn prepared_witness(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    source: Source,
    job: Uuid,
    lease: Uuid,
) -> RetentionWitness {
    let (generation, count, run_ids): (i64, i64, Vec<Uuid>) = sqlx::query_as(
        "SELECT * FROM public.ple_prepare_retention_delete_rehearsal_verification($1,$2,$3,$4,'deleteStudentRecords',1)",
    )
    .bind(source.tenant)
    .bind(job)
    .bind(lease)
    .bind(source.course)
    .fetch_one(&mut **transaction)
    .await
    .expect("retention source and lease prepare");
    assert_eq!(generation, 1, "the current retention generation is locked");
    assert_eq!(
        i64::try_from(run_ids.len()).expect("witness count fits database integer"),
        count,
        "the count and exact locked run-ID witness agree"
    );
    let mut sorted = run_ids.clone();
    sorted.sort_unstable();
    assert_eq!(
        run_ids, sorted,
        "the capability returns a stable sorted witness"
    );
    RetentionWitness {
        generation,
        count,
        run_ids,
    }
}

async fn commit(pool: &sqlx::PgPool, source: Source, job: Uuid, lease: Uuid) -> bool {
    let mut transaction = app(pool, source.tenant).await;
    let witness = prepared_witness(&mut transaction, source, job, lease).await;
    let committed: bool = sqlx::query_scalar(
        "SELECT public.ple_commit_retention_work($1,$2,$3,$4,'deleteStudentRecords',1,$5)",
    )
    .bind(source.tenant)
    .bind(job)
    .bind(lease)
    .bind(source.course)
    .bind(witness.count)
    .fetch_one(&mut *transaction)
    .await
    .expect("seven-argument retention commit result");
    transaction.commit().await.expect("commit retention work");
    committed
}

async fn seed_student_record(pool: &sqlx::PgPool, source: Source) -> (Uuid, Uuid, Uuid) {
    let user = id();
    let student = id();
    let membership = id();
    let mut owner = pool.begin().await.expect("begin student-record fixture");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(source.tenant.to_string())
        .execute(&mut *owner)
        .await
        .expect("bind student-record tenant");
    sqlx::query(
        "INSERT INTO tenant_learner_identity (tenant_id,user_id,student_id) VALUES ($1,$2,$3)",
    )
    .bind(source.tenant)
    .bind(user)
    .bind(student)
    .execute(&mut *owner)
    .await
    .expect("seed learner identity");
    sqlx::query(
        "INSERT INTO course_member \
         (tenant_id,course_id,user_id,role,course_membership_id,student_id,status,joined_at) \
         VALUES ($1,$2,$3,'student',$4,$5,'active',transaction_timestamp())",
    )
    .bind(source.tenant)
    .bind(source.course)
    .bind(user)
    .bind(membership)
    .bind(student)
    .execute(&mut *owner)
    .await
    .expect("seed learner membership");
    sqlx::query(
        "INSERT INTO course_roster_profile \
         (tenant_id,course_id,course_membership_id,display_name) VALUES ($1,$2,$3,'Retention learner')",
    )
    .bind(source.tenant)
    .bind(source.course)
    .bind(membership)
    .execute(&mut *owner)
    .await
    .expect("seed learner roster profile");
    owner.commit().await.expect("commit student-record fixture");
    (user, student, membership)
}

async fn source_counts(pool: &sqlx::PgPool, source: Source, user: Uuid) -> (i64, i64, i64) {
    let membership: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM course_member WHERE tenant_id=$1 AND course_id=$2 AND role='student'",
    )
    .bind(source.tenant)
    .bind(source.course)
    .fetch_one(pool)
    .await
    .expect("student membership count");
    let profile: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM course_roster_profile WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(source.tenant)
    .bind(source.course)
    .fetch_one(pool)
    .await
    .expect("student profile count");
    let identity: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tenant_learner_identity WHERE tenant_id=$1 AND user_id=$2",
    )
    .bind(source.tenant)
    .bind(user)
    .fetch_one(pool)
    .await
    .expect("learner identity count");
    (membership, profile, identity)
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn retention_delete_prepares_verifies_and_fences_before_live_student_cleanup() {
    let pool = pool().await;
    let source = source(&pool).await;
    let run = id();
    let mut rehearsal = app(&pool, source.tenant).await;
    start(&mut rehearsal, source, run, 1).await;
    rehearsal.commit().await.expect("commit active rehearsal");
    let (user, _, _) = seed_student_record(&pool, source).await;
    let (job, lease) = retention_work(&pool, source, "delete").await;

    assert!(
        commit(&pool, source, job, lease).await,
        "retention work commits"
    );
    let assignment: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM assignment WHERE tenant_id=$1 AND assignment_id=$2",
    )
    .bind(source.tenant)
    .bind(source.assignment)
    .fetch_one(&pool)
    .await
    .expect("assignment after retention");
    let lifecycle: String = sqlx::query_scalar(
        "SELECT lifecycle FROM rehearsal_run WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(source.tenant)
    .bind(run)
    .fetch_one(&pool)
    .await
    .expect("terminal rehearsal archive");
    assert_eq!(
        assignment, 0,
        "destructive cleanup removes the assignment after fencing"
    );
    assert_eq!(lifecycle, "discardedSourceContextRemoved");
    assert_eq!(
        source_counts(&pool, source, user).await,
        (0, 0, 0),
        "student membership, roster profile, and now-unreferenced identity are removed together"
    );
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn retain_disposition_keeps_live_rehearsal_and_student_authority_outside_delete_fence() {
    let pool = pool().await;
    let source = source(&pool).await;
    let run = id();
    let mut rehearsal = app(&pool, source.tenant).await;
    start(&mut rehearsal, source, run, 1).await;
    rehearsal.commit().await.expect("commit live rehearsal");
    let (user, _, _) = seed_student_record(&pool, source).await;
    let (job, lease) = retention_work(&pool, source, "retain").await;
    let mut transaction = app(&pool, source.tenant).await;
    let prepared = sqlx::query_scalar::<_, i64>(
        "SELECT public.ple_prepare_retention_delete_rehearsal_verification($1,$2,$3,$4,'deleteStudentRecords',1)",
    )
    .bind(source.tenant)
    .bind(job)
    .bind(lease)
    .bind(source.course)
    .fetch_one(&mut *transaction)
    .await;
    assert!(
        prepared.is_err(),
        "retain policy cannot enter the destructive rehearsal fence"
    );
    transaction
        .rollback()
        .await
        .expect("close retain rejection");
    let lifecycle: String = sqlx::query_scalar(
        "SELECT lifecycle FROM rehearsal_run WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(source.tenant)
    .bind(run)
    .fetch_one(&pool)
    .await
    .expect("retained rehearsal");
    assert_eq!(
        lifecycle, "active",
        "retain policy leaves the ordinary live rehearsal active"
    );
    assert_eq!(source_counts(&pool, source, user).await, (1, 1, 1));
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn retention_prepare_returns_sorted_exact_run_witness_and_final_capability_acl() {
    let pool = pool().await;
    let source = source(&pool).await;
    let alternate = Source {
        actor: source.co_instructor,
        membership: source.co_membership,
        ..source
    };
    let first_run = id();
    let second_run = id();
    let mut first = app(&pool, source.tenant).await;
    start(&mut first, source, first_run, 1).await;
    first.commit().await.expect("commit first active rehearsal");
    let mut second = app(&pool, source.tenant).await;
    start(&mut second, alternate, second_run, 1).await;
    second
        .commit()
        .await
        .expect("commit second active rehearsal");
    let (job, lease) = retention_work(&pool, source, "delete").await;

    let mut transaction = app(&pool, source.tenant).await;
    let witness = prepared_witness(&mut transaction, source, job, lease).await;
    assert_eq!(witness.generation, 1);
    assert_eq!(
        witness.count, 2,
        "each matching active rehearsal is witnessed"
    );
    assert_eq!(witness.run_ids, {
        let mut expected = vec![first_run, second_run];
        expected.sort_unstable();
        expected
    });
    transaction.rollback().await.expect("release witness locks");

    let final_shape: bool = sqlx::query_scalar(
        "SELECT to_regprocedure('public.ple_commit_retention_work(uuid,uuid,uuid,uuid,text,bigint,bigint)') IS NOT NULL
          AND to_regprocedure('public.ple_commit_retention_work(uuid,uuid,uuid,uuid,text,bigint)') IS NULL
          AND has_function_privilege('ple_app','public.ple_prepare_retention_delete_rehearsal_verification(uuid,uuid,uuid,uuid,text,bigint)','EXECUTE')
          AND has_function_privilege('ple_app','public.ple_commit_retention_work(uuid,uuid,uuid,uuid,text,bigint,bigint)','EXECUTE')
          AND NOT has_function_privilege('public','public.ple_prepare_retention_delete_rehearsal_verification(uuid,uuid,uuid,uuid,text,bigint)','EXECUTE')
          AND NOT has_function_privilege('public','public.ple_commit_retention_work(uuid,uuid,uuid,uuid,text,bigint,bigint)','EXECUTE')",
    )
    .fetch_one(&pool)
    .await
    .expect("final retention capability signature and ACL inventory");
    assert!(
        final_shape,
        "only the final app capability family is callable"
    );
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn retention_count_and_lease_mismatch_refuse_before_any_source_or_rehearsal_change() {
    let pool = pool().await;
    let source = source(&pool).await;
    let run = id();
    let mut rehearsal = app(&pool, source.tenant).await;
    start(&mut rehearsal, source, run, 1).await;
    rehearsal.commit().await.expect("commit active rehearsal");
    let (job, lease) = retention_work(&pool, source, "delete").await;

    for (candidate_lease, candidate_count) in
        [(lease, Some(0_i64)), (lease, None), (id(), Some(1_i64))]
    {
        let mut transaction = app(&pool, source.tenant).await;
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT public.ple_commit_retention_work($1,$2,$3,$4,'deleteStudentRecords',1,$5)",
        )
        .bind(source.tenant)
        .bind(job)
        .bind(candidate_lease)
        .bind(source.course)
        .bind(candidate_count)
        .fetch_one(&mut *transaction)
        .await;
        assert!(result.is_err() || !result.expect("false rejection result"));
        transaction
            .rollback()
            .await
            .expect("rollback rejected candidate");
    }
    let assignment: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM assignment WHERE tenant_id=$1 AND assignment_id=$2",
    )
    .bind(source.tenant)
    .bind(source.assignment)
    .fetch_one(&pool)
    .await
    .expect("source survives rejected mutation");
    let lifecycle: String = sqlx::query_scalar(
        "SELECT lifecycle FROM rehearsal_run WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(source.tenant)
    .bind(run)
    .fetch_one(&pool)
    .await
    .expect("rehearsal survives rejected mutation");
    assert_eq!(assignment, 1);
    assert_eq!(lifecycle, "active");

    let mut transaction = app(&pool, source.tenant).await;
    let bad_generation = sqlx::query_scalar::<_, bool>(
        "SELECT public.ple_commit_retention_work($1,$2,$3,$4,'deleteStudentRecords',2,$5)",
    )
    .bind(source.tenant)
    .bind(job)
    .bind(lease)
    .bind(source.course)
    .bind(1_i64)
    .fetch_one(&mut *transaction)
    .await;
    assert!(
        bad_generation.is_err(),
        "a stale generation cannot commit cleanup"
    );
    transaction
        .rollback()
        .await
        .expect("rollback stale-generation rejection");

    let mut foreign = app(&pool, id()).await;
    let wrong_tenant = sqlx::query_scalar::<_, bool>(
        "SELECT public.ple_commit_retention_work($1,$2,$3,$4,'deleteStudentRecords',1,$5)",
    )
    .bind(source.tenant)
    .bind(job)
    .bind(lease)
    .bind(source.course)
    .bind(1_i64)
    .fetch_one(&mut *foreign)
    .await;
    assert!(
        wrong_tenant.is_err(),
        "a foreign application tenant cannot reuse the lease"
    );
    foreign
        .rollback()
        .await
        .expect("rollback foreign-tenant rejection");
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn retention_profile_residual_aborts_and_rolls_back_fence_and_cleanup() {
    let pool = pool().await;
    let source = source(&pool).await;
    let run = id();
    let mut rehearsal = app(&pool, source.tenant).await;
    start(&mut rehearsal, source, run, 1).await;
    rehearsal.commit().await.expect("commit active rehearsal");
    let (user, _, _) = seed_student_record(&pool, source).await;
    let (job, lease) = retention_work(&pool, source, "delete").await;
    let mut owner = pool
        .begin()
        .await
        .expect("install deliberate residual trigger");
    sqlx::query(
        "CREATE FUNCTION public.ple_t4_retain_profile_for_oracle() RETURNS trigger \
         LANGUAGE plpgsql AS $$ BEGIN RETURN NULL; END $$",
    )
    .execute(&mut *owner)
    .await
    .expect("create residual trigger function");
    sqlx::query(
        "CREATE TRIGGER ple_t4_retain_profile_for_oracle BEFORE DELETE ON course_roster_profile \
         FOR EACH ROW EXECUTE FUNCTION public.ple_t4_retain_profile_for_oracle()",
    )
    .execute(&mut *owner)
    .await
    .expect("install residual trigger");
    owner.commit().await.expect("commit residual trigger");

    let mut transaction = app(&pool, source.tenant).await;
    let witness = prepared_witness(&mut transaction, source, job, lease).await;
    let result = sqlx::query_scalar::<_, bool>(
        "SELECT public.ple_commit_retention_work($1,$2,$3,$4,'deleteStudentRecords',1,$5)",
    )
    .bind(source.tenant)
    .bind(job)
    .bind(lease)
    .bind(source.course)
    .bind(witness.count)
    .fetch_one(&mut *transaction)
    .await;
    assert!(
        result.is_err(),
        "residual profile makes the entire retention transaction fail"
    );
    transaction
        .rollback()
        .await
        .expect("rollback residual retention attempt");

    let assignment: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM assignment WHERE tenant_id=$1 AND assignment_id=$2",
    )
    .bind(source.tenant)
    .bind(source.assignment)
    .fetch_one(&pool)
    .await
    .expect("source assignment rollback");
    let lifecycle: String = sqlx::query_scalar(
        "SELECT lifecycle FROM rehearsal_run WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(source.tenant)
    .bind(run)
    .fetch_one(&pool)
    .await
    .expect("rehearsal lifecycle rollback");
    assert_eq!(assignment, 1);
    assert_eq!(lifecycle, "active");
    assert_eq!(source_counts(&pool, source, user).await, (1, 1, 1));
    let mut owner = pool.begin().await.expect("remove residual trigger");
    sqlx::query("DROP TRIGGER ple_t4_retain_profile_for_oracle ON course_roster_profile")
        .execute(&mut *owner)
        .await
        .expect("remove residual trigger");
    sqlx::query("DROP FUNCTION public.ple_t4_retain_profile_for_oracle()")
        .execute(&mut *owner)
        .await
        .expect("remove residual trigger function");
    owner
        .commit()
        .await
        .expect("commit residual trigger cleanup");
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn direct_instructor_removal_fences_only_active_runs_and_retains_completed_evidence() {
    let pool = pool().await;
    let source = source(&pool).await;
    let completed_run = id();
    let mut completed = app(&pool, source.tenant).await;
    start(&mut completed, source, completed_run, 1).await;
    let terminalized: bool =
        sqlx::query_scalar("SELECT public.ple_rehearsal_terminalize($1,$2,$3,$4,1,$5,'completed')")
            .bind(source.tenant)
            .bind(source.actor)
            .bind(source.course)
            .bind(source.assignment)
            .bind(completed_run)
            .fetch_one(&mut *completed)
            .await
            .expect("complete ordinary rehearsal before instructor removal");
    assert!(
        terminalized,
        "fixture has completed, retained rehearsal evidence"
    );
    completed
        .commit()
        .await
        .expect("commit completed rehearsal");

    let mut removal = app(&pool, source.tenant).await;
    let (roster_revision, witness_count, witness_ids): (i64, i64, Vec<Uuid>) = sqlx::query_as(
        "SELECT * FROM public.ple_prepare_direct_instructor_rehearsal_fence($1,$2,$3,$4,1)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.membership)
    .fetch_one(&mut *removal)
    .await
    .expect("prepare locks direct-Instructor removal authority");
    assert_eq!(roster_revision, 1);
    assert_eq!(
        witness_count, 0,
        "completed evidence is outside the active witness"
    );
    assert!(witness_ids.is_empty(), "empty active witness is explicit");
    let fenced: i64 = sqlx::query_scalar(
        "SELECT public.ple_fence_rehearsals_for_direct_instructor_removal($1,$2,$3,$4,1,0)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(source.membership)
    .fetch_one(&mut *removal)
    .await
    .expect("commit locked direct-Instructor removal");
    assert_eq!(fenced, 0, "completed runs are not re-fenced as active work");
    removal
        .commit()
        .await
        .expect("commit direct-Instructor removal");

    let lifecycle: String = sqlx::query_scalar(
        "SELECT lifecycle FROM rehearsal_run WHERE tenant_id=$1 AND rehearsal_run_id=$2",
    )
    .bind(source.tenant)
    .bind(completed_run)
    .fetch_one(&pool)
    .await
    .expect("completed rehearsal evidence remains readable to the owner connection");
    let membership: String = sqlx::query_scalar(
        "SELECT status FROM course_member WHERE tenant_id=$1 AND course_membership_id=$2",
    )
    .bind(source.tenant)
    .bind(source.membership)
    .fetch_one(&pool)
    .await
    .expect("direct Instructor membership state");
    assert_eq!(
        lifecycle, "completed",
        "completed evidence survives its removed source"
    );
    assert_eq!(
        membership, "revoked",
        "direct-Instructor authority is removed atomically"
    );
}
