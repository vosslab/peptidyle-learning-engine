use super::fixture::{app, id, pool, source, start};

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn rehearsal_relations_are_forced_rls_with_no_direct_application_mutation_or_public_capability()
 {
    let pool = pool().await;
    let source = source(&pool).await;
    let run = id();
    let mut owner = app(&pool, source.tenant).await;
    start(&mut owner, source, run, 1).await;
    owner.commit().await.expect("commit owned run");
    let foreign = id();
    let mut attacker = app(&pool, foreign).await;
    let visible: i64 =
        sqlx::query_scalar("SELECT count(*) FROM rehearsal_run WHERE rehearsal_run_id=$1")
            .bind(run)
            .fetch_one(&mut *attacker)
            .await
            .expect("foreign RLS select");
    assert_eq!(visible, 0, "forced RLS conceals a foreign tenant rehearsal");
    let direct = sqlx::query("INSERT INTO rehearsal_run (tenant_id,rehearsal_run_id,course_id,assignment_id,assignment_reference,direct_instructor_membership_id,actor_id,assignment_revision,subject_payload,subject_fingerprint,evidence_head_digest) VALUES ($1,$2,$3,$4,1,$5,$6,1,'{}',$7,$7)")
        .bind(foreign).bind(id()).bind(id()).bind(id()).bind(id()).bind(id()).bind(vec![0_u8;32])
        .execute(&mut *attacker).await;
    assert!(
        direct.is_err(),
        "application role has no direct rehearsal DML"
    );
    attacker
        .rollback()
        .await
        .expect("close failed direct DML transaction");
    let mut definition_attacker = app(&pool, source.tenant).await;
    let direct_assignment = sqlx::query(
        "UPDATE assignment SET title='unauthorized' WHERE tenant_id=$1 AND assignment_id=$2",
    )
    .bind(source.tenant)
    .bind(source.assignment)
    .execute(&mut *definition_attacker)
    .await;
    assert!(
        direct_assignment.is_err(),
        "application role cannot directly change assignment definition"
    );
    definition_attacker
        .rollback()
        .await
        .expect("close denied assignment update");
    let mut internal_attacker = app(&pool, source.tenant).await;
    let internal = sqlx::query_scalar::<_, i64>(
        "SELECT public.ple_invalidate_rehearsals_for_assignment_internal($1,$2,$3,1,2)",
    )
    .bind(source.tenant)
    .bind(source.course)
    .bind(source.assignment)
    .fetch_one(&mut *internal_attacker)
    .await;
    assert!(
        internal.is_err(),
        "application role cannot invoke the internal invalidator"
    );
    internal_attacker
        .rollback()
        .await
        .expect("close denied internal invalidator");
    let mut membership_attacker = app(&pool, source.tenant).await;
    let direct_revocation = sqlx::query(
        "UPDATE course_member SET status='revoked' WHERE tenant_id=$1 AND course_membership_id=$2",
    )
    .bind(source.tenant)
    .bind(source.membership)
    .execute(&mut *membership_attacker)
    .await;
    assert!(
        direct_revocation.is_err(),
        "application role cannot bypass the direct-Instructor fence with membership DML"
    );
    membership_attacker
        .rollback()
        .await
        .expect("close denied membership revocation");
    let family_acl: bool = sqlx::query_scalar(
        "SELECT has_function_privilege('ple_app','public.ple_replace_assignment_fixed_item(uuid,uuid,uuid,uuid,bigint,uuid,uuid,uuid,bigint)','EXECUTE')
         AND has_function_privilege('ple_app','public.ple_add_assignment_fixed_item(uuid,uuid,uuid,uuid,bigint,uuid,integer,uuid,uuid,numeric,text,text,bigint)','EXECUTE')
         AND has_function_privilege('ple_app','public.ple_remove_assignment_fixed_item(uuid,uuid,uuid,uuid,bigint,uuid,bigint)','EXECUTE')
         AND has_function_privilege('ple_app','public.ple_put_assignment_teaching_settings(uuid,uuid,uuid,uuid,bigint,jsonb,bigint)','EXECUTE')
         AND has_function_privilege('ple_app','public.ple_replace_assignment_definition_v1(uuid,uuid,uuid,uuid,bigint,jsonb,uuid,integer,bigint)','EXECUTE')
         AND has_function_privilege('ple_app','public.ple_put_assignment_group_schedule_offset(uuid,uuid,uuid,uuid,bigint,uuid,integer,bigint)','EXECUTE')
         AND has_function_privilege('ple_app','public.ple_put_assignment_group_accommodation(uuid,uuid,uuid,uuid,bigint,uuid,jsonb,bigint)','EXECUTE')
         AND has_function_privilege('ple_app','public.ple_put_assignment_individual_exception(uuid,uuid,uuid,uuid,bigint,uuid,uuid,jsonb,bigint)','EXECUTE')",
    )
    .fetch_one(&pool)
    .await
    .expect("all assignment-mutator families expose actor-authorized capabilities");
    assert!(
        family_acl,
        "each assignment-definition family is broker-owned and callable only through its typed capability"
    );
    let mut retention_attacker = app(&pool, foreign).await;
    let cross_tenant_retention = sqlx::query_scalar::<_, bool>(
        "SELECT public.ple_commit_retention_work($1,$2,$3,$4,'deleteStudentRecords',1,0)",
    )
    .bind(source.tenant)
    .bind(id())
    .bind(id())
    .bind(source.course)
    .fetch_one(&mut *retention_attacker)
    .await;
    assert!(
        cross_tenant_retention.is_err(),
        "retention wrapper rejects a mismatched incoming tenant before it reads or mutates source state"
    );
    retention_attacker
        .rollback()
        .await
        .expect("close rejected cross-tenant retention transaction");
    let public_execute: bool = sqlx::query_scalar(
        "SELECT has_function_privilege('public', 'public.ple_rehearsal_append_evidence(uuid,uuid,uuid,uuid,bigint,uuid,bytea,bigint,text,bytea,jsonb,bytea,bigint)', 'EXECUTE')",
    ).fetch_one(&pool).await.expect("PUBLIC ACL probe");
    assert!(
        !public_execute,
        "inner evidence function is never a PUBLIC capability"
    );
    let witness_contract: (String, String, bool) = sqlx::query_as(
        "SELECT pg_get_function_result('public.ple_prepare_assignment_rehearsal_verification(uuid,uuid,uuid,uuid,bigint)'::regprocedure),
                pg_get_function_result('public.ple_prepare_direct_instructor_rehearsal_fence(uuid,uuid,uuid,uuid,bigint)'::regprocedure),
                NOT has_function_privilege('ple_app','public.ple_lock_active_rehearsal_source_internal(uuid,uuid,uuid,uuid)','EXECUTE')",
    )
    .fetch_one(&pool)
    .await
    .expect("opaque rehearsal witness contract inventory");
    assert!(
        witness_contract
            .0
            .contains("locked_rehearsal_run_ids uuid[]")
            && witness_contract
                .1
                .contains("locked_rehearsal_run_ids uuid[]"),
        "each public prepare capability returns its sorted opaque run witness"
    );
    assert!(
        witness_contract.2,
        "the lower-level source-lock primitive stays broker-only"
    );
    let start_contract: (bool, bool, String, String) = sqlx::query_as(
        "SELECT to_regprocedure('public.ple_rehearsal_start(uuid,uuid,uuid,uuid,integer,bigint,jsonb,bytea,bytea,uuid,boolean,uuid)') IS NOT NULL,
                to_regprocedure('public.ple_rehearsal_start(uuid,uuid,uuid,uuid,integer,bigint,jsonb,bytea,bytea,uuid)') IS NULL,
                procedure.proowner::regrole::text,
                coalesce(array_to_string(procedure.proconfig, ','), '')
           FROM pg_proc procedure
          WHERE procedure.oid='public.ple_rehearsal_start(uuid,uuid,uuid,uuid,integer,bigint,jsonb,bytea,bytea,uuid,boolean,uuid)'::regprocedure",
    )
    .fetch_one(&pool)
    .await
    .expect("final rehearsal-start capability inventory");
    assert!(
        start_contract.0 && start_contract.1,
        "only the witness-bearing start capability exists"
    );
    assert_eq!(
        start_contract.2, "ple_rehearsal_broker",
        "start remains broker-owned"
    );
    assert!(
        start_contract
            .3
            .contains("search_path=pg_catalog, public, pg_temp"),
        "start pins its safe search path"
    );

    let mut clock_app = app(&pool, source.tenant).await;
    let app_millis: i64 = sqlx::query_scalar("SELECT public.ple_rehearsal_now_millis()")
        .fetch_one(&mut *clock_app)
        .await
        .expect("ple_app invokes the broker-owned rehearsal millisecond capability");
    clock_app
        .commit()
        .await
        .expect("commit successful application clock probe");
    let clock_contract: (bool, String, String, bool, bool, bool) = sqlx::query_as(
        "SELECT millis.prosecdef,
                millis.proowner::regrole::text,
                coalesce(array_to_string(millis.proconfig, ','), ''),
                has_function_privilege('public', millis.oid, 'EXECUTE'),
                has_function_privilege('ple_app', millis.oid, 'EXECUTE'),
                has_function_privilege('ple_app', clock.oid, 'EXECUTE')
           FROM pg_proc millis
           JOIN pg_proc clock ON clock.oid='public.ple_rehearsal_now()'::regprocedure
          WHERE millis.oid='public.ple_rehearsal_now_millis()'::regprocedure",
    )
    .fetch_one(&pool)
    .await
    .expect("rehearsal database-clock capability inventory");
    assert!(
        app_millis > 0,
        "the application role can obtain a database-owned rehearsal timestamp"
    );
    assert!(
        clock_contract.0,
        "the public millis capability runs with the broker's narrowly scoped rights"
    );
    assert_eq!(
        clock_contract.1, "ple_rehearsal_broker",
        "the public millis capability remains broker-owned"
    );
    assert!(
        clock_contract
            .2
            .contains("search_path=pg_catalog, public, pg_temp"),
        "the public millis capability pins its safe search path"
    );
    assert!(
        !clock_contract.3 && clock_contract.4 && !clock_contract.5,
        "only ple_app receives the public millis capability; the inner clock remains private"
    );
}
