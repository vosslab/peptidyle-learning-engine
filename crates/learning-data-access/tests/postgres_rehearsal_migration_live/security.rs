//! PostgreSQL ownership and raw-access evidence for frozen rehearsal material.
//!
//! Canonical route-start state lives in the included Store conformance oracle.
//! These checks inspect only privilege and RLS boundaries, so no test can
//! recreate the retired empty-assignment rehearsal sidecar.

use learning_data_access::postgres::{apply_migrations, lazy_pool};

async fn pool() -> sqlx::PgPool {
    let url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL names a disposable PostgreSQL database");
    let pool = lazy_pool(&url).expect("valid disposable PostgreSQL URL");
    apply_migrations(&pool)
        .await
        .expect("full migration epoch applies");
    pool
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn frozen_material_is_unreadable_and_immutable_to_application_and_grader_roles() {
    let pool = pool().await;
    let access: (bool, bool, bool, bool) = sqlx::query_as(
        "SELECT
            EXISTS (
                SELECT 1 FROM unnest(ARRAY[
                    'rehearsal_frozen_material_set',
                    'rehearsal_frozen_source_snapshot',
                    'rehearsal_frozen_private_execution',
                    'rehearsal_delivery_generation_material_binding',
                    'rehearsal_delivery_issued_execution_artifact',
                    'rehearsal_start_freeze_source_binding'
                ]) AS material_table(name)
                WHERE has_table_privilege('ple_app', format('public.%I', material_table.name), 'SELECT')
                   OR has_table_privilege('ple_app', format('public.%I', material_table.name), 'INSERT')
                   OR has_table_privilege('ple_app', format('public.%I', material_table.name), 'UPDATE')
                   OR has_table_privilege('ple_app', format('public.%I', material_table.name), 'DELETE')
            ),
            EXISTS (
                SELECT 1 FROM unnest(ARRAY[
                    'rehearsal_frozen_material_set',
                    'rehearsal_frozen_source_snapshot',
                    'rehearsal_frozen_private_execution',
                    'rehearsal_delivery_generation_material_binding',
                    'rehearsal_delivery_issued_execution_artifact',
                    'rehearsal_start_freeze_source_binding'
                ]) AS material_table(name)
                WHERE has_table_privilege('ple_grading_reader', format('public.%I', material_table.name), 'SELECT')
                   OR has_table_privilege('ple_grading_reader', format('public.%I', material_table.name), 'INSERT')
                   OR has_table_privilege('ple_grading_reader', format('public.%I', material_table.name), 'UPDATE')
                   OR has_table_privilege('ple_grading_reader', format('public.%I', material_table.name), 'DELETE')
            ),
            has_function_privilege('ple_grading_reader',
                'public.ple_prepare_sealed_rehearsal_delivery_execution(uuid,uuid)', 'EXECUTE'),
            has_function_privilege('ple_app',
                'public.ple_prepare_sealed_rehearsal_delivery_execution(uuid,uuid)', 'EXECUTE')",
    )
    .fetch_one(&pool)
    .await
    .expect("frozen material privilege inventory");

    assert!(
        !access.0,
        "ple_app uses typed brokers rather than raw material rows"
    );
    assert!(
        !access.1,
        "the grader receives private bytes only through its sealed broker"
    );
    assert!(
        access.2 && !access.3,
        "only the grader role can invoke the sealed broker"
    );
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn rehearsal_start_and_material_protocols_are_closed_execute_only_capabilities() {
    let pool = pool().await;
    let contract: (bool, bool, bool, bool, bool, bool, bool, bool, String, String) = sqlx::query_as(
        "SELECT
            has_function_privilege('ple_app',
                'public.ple_prepare_rehearsal_start_idempotent(uuid,uuid,uuid,uuid,integer,bigint,jsonb,bytea,bytea,uuid,boolean,text,bytea)', 'EXECUTE'),
            has_function_privilege('public',
                'public.ple_prepare_rehearsal_start_idempotent(uuid,uuid,uuid,uuid,integer,bigint,jsonb,bytea,bytea,uuid,boolean,text,bytea)', 'EXECUTE'),
            has_function_privilege('ple_app',
                'public.ple_finalize_rehearsal_start_freeze(uuid,uuid,uuid,bytea,bytea,integer,uuid[],uuid[],uuid[],uuid[],jsonb[],bytea[],bytea[],bigint[],integer[],bytea[],bytea[],bytea[],bytea[],text[],integer[],integer[],integer,bytea,bytea)', 'EXECUTE'),
            has_function_privilege('public',
                'public.ple_finalize_rehearsal_start_freeze(uuid,uuid,uuid,bytea,bytea,integer,uuid[],uuid[],uuid[],uuid[],jsonb[],bytea[],bytea[],bigint[],integer[],bytea[],bytea[],bytea[],bytea[],text[],integer[],integer[],integer,bytea,bytea)', 'EXECUTE'),
            has_function_privilege('ple_app',
                'public.ple_complete_rehearsal_start_idempotent(uuid,uuid,uuid,bytea,jsonb,bytea)', 'EXECUTE'),
            has_function_privilege('public',
                'public.ple_complete_rehearsal_start_idempotent(uuid,uuid,uuid,bytea,jsonb,bytea)', 'EXECUTE'),
            has_function_privilege('ple_grader',
                'public.ple_complete_rehearsal_start_idempotent(uuid,uuid,uuid,bytea,jsonb,bytea)', 'EXECUTE'),
            has_function_privilege('ple_grading_reader',
                'public.ple_complete_rehearsal_start_idempotent(uuid,uuid,uuid,bytea,jsonb,bytea)', 'EXECUTE'),
            (SELECT proowner::regrole::text FROM pg_proc WHERE oid='public.ple_prepare_rehearsal_start_idempotent(uuid,uuid,uuid,uuid,integer,bigint,jsonb,bytea,bytea,uuid,boolean,text,bytea)'::regprocedure),
            (SELECT coalesce(array_to_string(proconfig, ','), '') FROM pg_proc WHERE oid='public.ple_finalize_rehearsal_start_freeze(uuid,uuid,uuid,bytea,bytea,integer,uuid[],uuid[],uuid[],uuid[],jsonb[],bytea[],bytea[],bigint[],integer[],bytea[],bytea[],bytea[],bytea[],text[],integer[],integer[],integer,bytea,bytea)'::regprocedure)",
    )
    .fetch_one(&pool)
    .await
    .expect("route start and material broker contract");

    assert!(
        contract.0 && contract.2,
        "ple_app reaches material only through broker functions"
    );
    assert!(
        !contract.1 && !contract.3,
        "rehearsal brokers are never public capabilities"
    );
    assert!(
        !contract.4 && !contract.5 && !contract.6 && !contract.7,
        "only the source-owned material finalizer can complete a prepared start"
    );
    assert_eq!(
        contract.8, "ple_rehearsal_broker",
        "route-start broker has a dedicated owner"
    );
    assert!(
        contract
            .9
            .contains("search_path=pg_catalog, public, pg_temp")
    );
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn route_material_verifier_exposes_only_one_integrity_bit() {
    let pool = pool().await;
    let contract: (
        bool,
        bool,
        bool,
        bool,
        bool,
        bool,
        bool,
        String,
        bool,
        bool,
        bool,
    ) = sqlx::query_as(
        "SELECT
            to_regprocedure('public.ple_preload_rehearsal_delivery_material(uuid,uuid,uuid,integer,bigint,bigint)') IS NULL,
            to_regprocedure('public.ple_verify_rehearsal_delivery_material_from_route(uuid,uuid,uuid,integer,bigint,bigint)') IS NOT NULL,
            has_function_privilege('ple_app', 'public.ple_verify_rehearsal_delivery_material_from_route(uuid,uuid,uuid,integer,bigint,bigint)', 'EXECUTE'),
            has_function_privilege('public', 'public.ple_verify_rehearsal_delivery_material_from_route(uuid,uuid,uuid,integer,bigint,bigint)', 'EXECUTE'),
            has_function_privilege('ple_student', 'public.ple_verify_rehearsal_delivery_material_from_route(uuid,uuid,uuid,integer,bigint,bigint)', 'EXECUTE'),
            has_function_privilege('ple_grader', 'public.ple_verify_rehearsal_delivery_material_from_route(uuid,uuid,uuid,integer,bigint,bigint)', 'EXECUTE'),
            has_function_privilege('ple_grading_reader', 'public.ple_verify_rehearsal_delivery_material_from_route(uuid,uuid,uuid,integer,bigint,bigint)', 'EXECUTE'),
            verifier.proowner::regrole::text,
            verifier.prosecdef,
            coalesce(array_to_string(verifier.proconfig, ','), '') = 'search_path=pg_catalog, public, pg_temp',
            verifier.proretset
                AND verifier.prorettype = 'boolean'::regtype::oid
                AND verifier.pronargs = 6
                AND cardinality(verifier.proallargtypes) = 7
                AND verifier.proargmodes[7] = 't'
                AND verifier.proargnames[7] = 'material_valid'
        FROM pg_proc verifier
        WHERE verifier.oid = 'public.ple_verify_rehearsal_delivery_material_from_route(uuid,uuid,uuid,integer,bigint,bigint)'::regprocedure",
    )
    .fetch_one(&pool)
    .await
    .expect("closed route material verifier contract");

    assert!(
        contract.0,
        "the payload-returning preload capability is retired"
    );
    assert!(
        contract.1 && contract.2,
        "ple_app receives the closed verifier"
    );
    assert!(
        !contract.3 && !contract.4 && !contract.5 && !contract.6,
        "the closed verifier is not a public, learner, or grading capability"
    );
    assert_eq!(contract.7, "ple_rehearsal_source");
    assert!(
        contract.8 && contract.9,
        "the verifier has a fixed definer boundary"
    );
    assert!(
        contract.10,
        "the verifier returns exactly one named boolean and no material rows"
    );
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn generic_claim_and_terminal_functions_are_internal_to_the_route_broker() {
    let pool = pool().await;
    let access: (bool, bool, bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT
            has_function_privilege('ple_app', 'public.ple_rehearsal_create_claim(uuid,uuid,uuid,uuid,bigint,uuid,uuid,uuid,text,uuid,bytea,jsonb)', 'EXECUTE'),
            has_function_privilege('ple_app', 'public.ple_rehearsal_append_claim_event(uuid,uuid,uuid,uuid,bigint,uuid,uuid,uuid,text,text)', 'EXECUTE'),
            has_function_privilege('ple_app', 'public.ple_rehearsal_complete_claim(uuid,uuid,uuid,uuid,bigint,uuid,uuid,uuid,bytea,bigint,bytea,jsonb,bytea,bigint,jsonb,bytea)', 'EXECUTE'),
            has_function_privilege('ple_app', 'public.ple_rehearsal_terminalize(uuid,uuid,uuid,uuid,bigint,uuid,text)', 'EXECUTE'),
            has_function_privilege('ple_app', 'public.ple_rehearsal_route_create_claim(uuid,uuid,uuid,uuid,bigint,uuid,uuid,uuid,text,bytea,bytea,jsonb)', 'EXECUTE'),
            has_function_privilege('ple_app', 'public.ple_rehearsal_route_append_claim_event(uuid,uuid,uuid,uuid,bigint,uuid,uuid,uuid,text,text)', 'EXECUTE'),
            has_function_privilege('ple_app', 'public.ple_rehearsal_route_complete_claim(uuid,uuid,uuid,uuid,bigint,uuid,uuid,uuid,bytea,bigint,bytea,jsonb,bytea,bigint,jsonb,bytea)', 'EXECUTE')",
    )
    .fetch_one(&pool)
    .await
    .expect("claim authority inventory");
    assert_eq!(
        access,
        (false, false, false, false, true, true, true),
        "ple_app reaches an issued screen only through route-bound submission brokers"
    );
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn issued_execution_artifacts_are_grader_only_capabilities() {
    let pool = pool().await;
    let access: (bool, bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT
            to_regprocedure('public.ple_commit_rehearsal_issued_execution_artifact(uuid,uuid,uuid,integer,bigint,bigint,uuid,bytea,bytea,bytea,bytea,bytea)') IS NULL,
            has_function_privilege('ple_app', 'public.ple_prepare_or_resume_rehearsal_issued_execution(uuid,uuid)', 'EXECUTE'),
            has_function_privilege('ple_app', 'public.ple_commit_sealed_rehearsal_issued_execution(uuid,uuid,bytea,bytea)', 'EXECUTE'),
            has_function_privilege('ple_grading_reader', 'public.ple_prepare_or_resume_rehearsal_issued_execution(uuid,uuid)', 'EXECUTE'),
            has_function_privilege('ple_grading_reader', 'public.ple_commit_sealed_rehearsal_issued_execution(uuid,uuid,bytea,bytea)', 'EXECUTE'),
            EXISTS (
                SELECT 1
                FROM pg_trigger
                WHERE tgrelid = 'public.rehearsal_delivery_issued_execution_artifact'::regclass
                  AND tgname = 'rehearsal_issued_execution_artifact_append_only'
                  AND tgenabled = 'O'
            )",
    )
    .fetch_one(&pool)
    .await
    .expect("issued execution capability inventory");
    assert_eq!(
        access,
        (true, false, false, true, true, true),
        "only the sealed grader facade can prepare or commit immutable issued execution bytes"
    );
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn issued_execution_artifacts_keep_forced_rls_append_only_acl_and_owner_boundary() {
    let pool = pool().await;
    let contract: (bool, bool, String, bool, bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT
            table_row.relrowsecurity,
            table_row.relforcerowsecurity,
            table_row.relowner::regrole::text,
            has_table_privilege('ple_rehearsal_source',
                'public.rehearsal_delivery_issued_execution_artifact', 'SELECT'),
            has_table_privilege('ple_rehearsal_source',
                'public.rehearsal_delivery_issued_execution_artifact', 'INSERT'),
            has_table_privilege('ple_rehearsal_source',
                'public.rehearsal_delivery_issued_execution_artifact', 'UPDATE'),
            has_table_privilege('ple_rehearsal_source',
                'public.rehearsal_delivery_issued_execution_artifact', 'DELETE'),
            has_table_privilege('ple_app',
                'public.rehearsal_delivery_issued_execution_artifact', 'UPDATE'),
            has_table_privilege('ple_grading_reader',
                'public.rehearsal_delivery_issued_execution_artifact', 'SELECT')
         FROM pg_class AS table_row
         WHERE table_row.oid =
            'public.rehearsal_delivery_issued_execution_artifact'::regclass",
    )
    .fetch_one(&pool)
    .await
    .expect("issued artifact relation security contract");
    assert!(
        contract.0 && contract.1,
        "issued artifacts use forced row security"
    );
    assert!(
        !matches!(
            contract.2.as_str(),
            "ple_rehearsal_source"
                | "ple_app"
                | "ple_grading_reader"
                | "ple_grader"
                | "ple_student"
        ),
        "artifact table remains owned by the migration principal, not a runtime capability role"
    );
    assert!(
        contract.3 && contract.4,
        "source can read and append artifacts"
    );
    assert!(
        !contract.5 && !contract.6,
        "source cannot mutate committed artifacts"
    );
    assert!(
        !contract.7 && !contract.8,
        "application and reader roles lack raw table access"
    );

    let mut transaction = pool.begin().await.expect("raw artifact ACL transaction");
    sqlx::query("SET LOCAL ROLE ple_rehearsal_source")
        .execute(&mut *transaction)
        .await
        .expect("source capability role");
    let update = sqlx::query(
        "UPDATE public.rehearsal_delivery_issued_execution_artifact
            SET artifact_sha256=artifact_sha256
          WHERE false",
    )
    .execute(&mut *transaction)
    .await;
    assert!(update.is_err(), "source cannot issue raw artifact UPDATE");
    let delete = sqlx::query(
        "DELETE FROM public.rehearsal_delivery_issued_execution_artifact
          WHERE false",
    )
    .execute(&mut *transaction)
    .await;
    assert!(delete.is_err(), "source cannot issue raw artifact DELETE");
    transaction
        .rollback()
        .await
        .expect("raw artifact ACL rollback");

    let trigger: (bool, String) = sqlx::query_as(
        "SELECT tgenabled='O', trigger_fn.proname
           FROM pg_trigger AS trigger_row
           JOIN pg_proc AS trigger_fn ON trigger_fn.oid=trigger_row.tgfoid
          WHERE trigger_row.tgrelid=
             'public.rehearsal_delivery_issued_execution_artifact'::regclass
            AND trigger_row.tgname='rehearsal_issued_execution_artifact_append_only'",
    )
    .fetch_one(&pool)
    .await
    .expect("issued artifact append-only trigger");
    assert!(trigger.0, "append-only trigger remains enabled");
    assert_eq!(trigger.1, "ple_rehearsal_material_append_only");
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn route_dispatch_and_sealed_submission_capabilities_keep_role_boundaries() {
    let pool = pool().await;
    let access: (bool, bool, bool, bool, bool, bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT
            has_function_privilege('ple_app',
                'public.ple_rehearsal_route_dispatch_claim(uuid,uuid,uuid,integer,bigint,bigint,text)', 'EXECUTE'),
            has_function_privilege('ple_grading_reader',
                'public.ple_prepare_or_resume_sealed_rehearsal_submission(uuid,uuid,uuid,integer,bigint,bigint,text)', 'EXECUTE'),
            has_function_privilege('ple_app',
                'public.ple_prepare_or_resume_sealed_rehearsal_submission(uuid,uuid,uuid,integer,bigint,bigint,text)', 'EXECUTE'),
            has_function_privilege('ple_grader',
                'public.ple_rehearsal_route_dispatch_claim(uuid,uuid,uuid,integer,bigint,bigint,text)', 'EXECUTE'),
            has_table_privilege('ple_app', 'public.rehearsal_submission_claim_delivery_binding', 'SELECT'),
            has_table_privilege('ple_app', 'public.rehearsal_submission_claim_event', 'SELECT'),
            has_table_privilege('ple_grading_reader', 'public.rehearsal_submission_claim_delivery_binding', 'SELECT'),
            has_table_privilege('ple_grading_reader', 'public.rehearsal_submission_claim_event', 'SELECT'),
            has_table_privilege('ple_grader', 'public.rehearsal_submission_claim_delivery_binding', 'SELECT'),
            has_table_privilege('ple_grader', 'public.rehearsal_submission_claim_event', 'SELECT')",
    )
    .fetch_one(&pool)
    .await
    .expect("route dispatch capability inventory");
    assert!(access.0, "application owns route-keyed dispatch admission");
    assert!(
        access.1,
        "sealed submission preparation is grader-reader only"
    );
    assert!(
        !access.2 && !access.3,
        "runtime roles cannot cross capability boundaries"
    );
    assert!(
        !access.4 && !access.5,
        "application cannot read private claim journals"
    );
    assert!(
        !access.6 && !access.7,
        "reader uses the sealed submission broker"
    );
    assert!(
        !access.8 && !access.9,
        "grader uses the sealed submission broker"
    );
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn sealed_delivery_helper_chain_has_one_non_login_owner() {
    let pool = pool().await;
    let owners: Vec<(String, String)> = sqlx::query_as(
        "SELECT procedure.proname, procedure.proowner::regrole::text
           FROM pg_proc AS procedure
          WHERE procedure.oid = ANY (ARRAY[
                'public.ple_current_rehearsal_delivery(uuid,uuid)'::regprocedure,
                'public.ple_reconcile_current_rehearsal_delivery(uuid,uuid)'::regprocedure,
                'public.ple_copy_rehearsal_generation_material_binding()'::regprocedure,
                'public.ple_expected_rehearsal_timing_witness(uuid,uuid,uuid,integer,bigint)'::regprocedure,
                'public.ple_verify_persisted_rehearsal_timing_witness(uuid,uuid,uuid,integer)'::regprocedure,
                'public.ple_verify_rehearsal_issued_execution_artifact(uuid,uuid,uuid,integer,uuid)'::regprocedure,
                'public.ple_prepare_or_resume_rehearsal_issued_execution(uuid,uuid)'::regprocedure
          ])
          ORDER BY procedure.proname",
    )
    .fetch_all(&pool)
    .await
    .expect("sealed helper owner inventory");
    assert_eq!(owners.len(), 7, "every sealed helper is present");
    assert!(
        owners
            .iter()
            .all(|(_, owner)| owner == "ple_rehearsal_source")
    );
}
