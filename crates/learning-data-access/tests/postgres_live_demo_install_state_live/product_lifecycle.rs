use std::str::FromStr;

use base_course_installation::{
    BaseCourseAction, BaseCourseInstallError, BaseCourseInstallPhase, BaseCourseInstallRequest,
    BaseCourseInstallStateOutput, BaseCourseParticipants,
};
use learning_data_access::postgres::{apply_migrations, lazy_pool};
use learning_data_access::{LiveDemoInstallationStore, StoreError};
use question_model::{TenantId, UserId};
use serde_json::Value;
use sqlx::Row;
use sqlx::postgres::PgConnectOptions;
use uuid::Uuid;

use super::{Participants, ProductDatabase, fresh, install_request, prepare_request};

fn alternate_participants(original: Participants) -> BaseCourseParticipants {
    BaseCourseParticipants::try_new(
        TenantId::from_uuid(fresh()),
        UserId::from_uuid(fresh()),
        original.mary,
        original.jack,
        original.avery,
        original.morgan,
    )
    .expect("alternate request has five distinct identities")
}

fn exact_sha256(output: &Value, field: &str) -> String {
    let value = output[field]
        .as_str()
        .unwrap_or_else(|| panic!("completed product output includes {field}"));
    assert!(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "{field} is exactly lowercase hexadecimal"
    );
    value.to_string()
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL for a fresh disposable PostgreSQL database"]
async fn base_course_product_serializes_restart_and_retains_exact_evidence() {
    let url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name a fresh disposable database");
    let admin = super::admin_pool(&url).await;
    super::reset_disposable_course_capability_memberships(&admin).await;
    let pool = lazy_pool(&url).expect("PostgreSQL URL");
    apply_migrations(&pool)
        .await
        .expect("embedded migrations apply to the disposable database");
    let capability = sqlx::query(
        "SELECT procedure.prosecdef, owner_role.rolname, \
                has_function_privilege('ple_auth', procedure.oid, 'EXECUTE') AS auth_execute, \
                has_table_privilege('ple_auth', 'public.live_demo_install_state', 'SELECT') \
                    AS auth_table_select, \
                has_table_privilege(\
                    'ple_auth', 'public.live_demo_install_completion_receipt', 'SELECT'\
                ) AS auth_receipt_select \
         FROM pg_proc AS procedure \
         JOIN pg_roles AS owner_role ON owner_role.oid = procedure.proowner \
         WHERE procedure.proname = 'ple_completed_live_demo_installation_generation'",
    )
    .fetch_one(&pool)
    .await
    .expect("completed-generation broker catalog");
    assert!(capability.try_get::<bool, _>("prosecdef").expect("definer"));
    assert_eq!(
        capability
            .try_get::<String, _>("rolname")
            .expect("broker owner"),
        "ple_live_demo_installation_broker"
    );
    assert!(
        capability
            .try_get::<bool, _>("auth_execute")
            .expect("auth broker execute")
    );
    assert!(
        !capability
            .try_get::<bool, _>("auth_table_select")
            .expect("auth direct table select")
    );
    assert!(
        !capability
            .try_get::<bool, _>("auth_receipt_select")
            .expect("auth direct receipt select")
    );
    sqlx::query("DELETE FROM public.live_demo_install_state")
        .execute(&pool)
        .await
        .expect("disposable oracle clears only its lifecycle marker");

    let database = PgConnectOptions::from_str(&url)
        .expect("PostgreSQL URL")
        .get_database()
        .expect("database name")
        .to_string();
    let product = ProductDatabase::provision(&admin, &url, &database).await;
    let participants = Participants::fresh();
    let store = product.store();
    let api_store = product.api_store();
    let api_contract = sqlx::query(
        "SELECT NOT (login.rolsuper OR login.rolcreatedb OR login.rolcreaterole \
                     OR login.rolinherit OR login.rolreplication OR login.rolbypassrls) \
                    AS closed_login, \
                (SELECT array_agg(parent.rolname::text ORDER BY parent.rolname) \
                   FROM pg_auth_members AS membership \
                   JOIN pg_roles AS parent ON parent.oid=membership.roleid \
                  WHERE membership.member=login.oid) \
                    = ARRAY['ple_app','ple_auth']::text[] AS exact_memberships, \
                NOT EXISTS (\
                    SELECT 1 FROM pg_auth_members AS membership \
                     WHERE membership.member=login.oid \
                       AND (membership.admin_option OR membership.inherit_option \
                            OR NOT membership.set_option)\
                ) AS closed_membership_options \
         FROM pg_roles AS login WHERE login.rolname='ple_api_login'",
    )
    .fetch_one(&admin)
    .await
    .expect("disposable API login catalog contract");
    assert!(
        api_contract
            .try_get::<bool, _>("closed_login")
            .expect("closed API login")
    );
    assert!(
        api_contract
            .try_get::<bool, _>("exact_memberships")
            .expect("exact API memberships")
    );
    assert!(
        api_contract
            .try_get::<bool, _>("closed_membership_options")
            .expect("closed API membership options")
    );

    let interrupted_installer = product.installer_pool();
    let prepared = base_course_installation::install(
        &interrupted_installer,
        &store,
        prepare_request(participants),
    )
    .await
    .expect("first product call prepares a generation");
    assert_eq!(prepared.action(), BaseCourseAction::Prepared);
    assert_eq!(
        prepared.install_state(),
        BaseCourseInstallStateOutput::Installing
    );
    let prepared_value = serde_json::to_value(&prepared).expect("serializable prepare evidence");
    assert_eq!(prepared_value["schemaVersion"], 1);
    assert_eq!(prepared_value["baselineVersion"], "base-course-v1");
    assert_eq!(prepared_value["objectManifest"], Value::Array(Vec::new()));
    assert_eq!(prepared_value["storageReceiptBucket"], "private-content");
    assert_eq!(
        prepared_value["storageReceiptKey"],
        "ple/live-demo/base-course-install-receipt.json"
    );
    let receipt: Value = serde_json::from_str(prepared.storage_receipt_json())
        .expect("canonical storage receipt JSON");
    assert_eq!(receipt["schemaVersion"], 1);
    assert_eq!(receipt["baselineVersion"], "base-course-v1");
    assert_eq!(
        receipt["installationGeneration"],
        prepared.installation_generation().to_string()
    );

    let mismatch = base_course_installation::install(
        &interrupted_installer,
        &store,
        BaseCourseInstallRequest::new(
            alternate_participants(participants),
            BaseCourseInstallPhase::Prepare,
        ),
    )
    .await
    .expect_err("an existing generation rejects different typed inputs");
    assert!(
        matches!(
            &mismatch,
            BaseCourseInstallError::Persistence {
                source: StoreError::AlreadyExists,
                ..
            }
        ),
        "{mismatch:?}"
    );
    drop(interrupted_installer);

    let restarted_installer = product.installer_pool();
    let receipt_json = prepared.storage_receipt_json().to_string();
    let first_pool = restarted_installer.clone();
    let first_store = store.clone();
    let second_pool = restarted_installer.clone();
    let second_store = store.clone();
    let first_receipt = receipt_json.clone();
    let second_receipt = receipt_json.clone();
    let (first, second) = tokio::join!(
        async move {
            base_course_installation::install(
                &first_pool,
                &first_store,
                install_request(participants, &first_receipt),
            )
            .await
        },
        async move {
            base_course_installation::install(
                &second_pool,
                &second_store,
                install_request(participants, &second_receipt),
            )
            .await
        }
    );
    let outputs = [
        first.expect("first concurrent installer converges"),
        second.expect("second concurrent installer converges"),
    ];
    let mut actions = [outputs[0].action(), outputs[1].action()];
    actions.sort_by_key(|action| match action {
        BaseCourseAction::Resumed => 0,
        BaseCourseAction::Retained => 1,
        BaseCourseAction::Prepared | BaseCourseAction::Installed => 2,
    });
    assert_eq!(
        actions,
        [BaseCourseAction::Resumed, BaseCourseAction::Retained],
        "one restarted caller completes and its concurrent peer observes retained state"
    );
    let resumed = outputs
        .iter()
        .find(|output| output.action() == BaseCourseAction::Resumed)
        .expect("one restarted caller owns completion");
    let concurrent_retained = outputs
        .iter()
        .find(|output| output.action() == BaseCourseAction::Retained)
        .expect("one concurrent caller observes completion");
    let resumed_value = serde_json::to_value(resumed).expect("serializable resumed evidence");
    let concurrent_retained_value = serde_json::to_value(concurrent_retained)
        .expect("serializable concurrent retained evidence");
    let storage_receipt_sha256 = exact_sha256(&resumed_value, "storageReceiptSha256");
    let completion_receipt_sha256 = exact_sha256(&resumed_value, "completionReceiptSha256");
    assert_eq!(
        exact_sha256(&concurrent_retained_value, "storageReceiptSha256"),
        storage_receipt_sha256
    );
    assert_eq!(
        exact_sha256(&concurrent_retained_value, "completionReceiptSha256"),
        completion_receipt_sha256
    );
    assert!(
        concurrent_retained_value.get("manifest").is_none(),
        "the retained path does not reread the installed graph"
    );
    let expected_manifest: Value = sqlx::query(
        "SELECT jsonb_build_object(\
         'assignmentId',assignment.assignment_id,\
         'enrollmentId',enrollment.enrollment_id,\
         'questionId',substr(problem.question_id,1,3)||'-'||substr(problem.question_id,4),\
         'problemId',problem.problem_id,\
         'versionId',version.version_id) AS manifest \
         FROM public.assignment AS assignment \
         JOIN public.enrollment AS enrollment \
           ON enrollment.tenant_id=assignment.tenant_id \
          AND enrollment.assignment_id=assignment.assignment_id \
          AND enrollment.user_id=$1 \
         JOIN public.assignment_item AS item \
           ON item.tenant_id=assignment.tenant_id \
          AND item.assignment_id=assignment.assignment_id \
         JOIN public.problem AS problem ON problem.problem_id=item.problem_id \
         JOIN public.problem_version AS version \
           ON version.problem_id=item.problem_id AND version.version_id=item.version_id",
    )
    .bind(participants.mary.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("exact installed host manifest")
    .try_get("manifest")
    .expect("manifest JSON");
    assert_eq!(resumed_value["manifest"], expected_manifest);

    assert_eq!(
        api_store
            .completed_live_demo_installation_generation()
            .await
            .expect("completed generation broker read"),
        Some(prepared.installation_generation())
    );
    sqlx::query(
        "INSERT INTO public.authentication_rate_limit \
         (limit_scope, key_hash, window_started_at, attempt_count, updated_at) VALUES (\
         'network', decode(repeat('44',32),'hex'), transaction_timestamp(), 1, \
         transaction_timestamp())",
    )
    .execute(&pool)
    .await
    .expect("ordinary retained state after complete installation");
    let retained = base_course_installation::install(
        &restarted_installer,
        &store,
        prepare_request(participants),
    )
    .await
    .expect("completed installation is idempotently retained");
    assert_eq!(retained.action(), BaseCourseAction::Retained);
    assert_eq!(
        retained.installation_generation(),
        prepared.installation_generation()
    );
    let retained_value =
        serde_json::to_value(&retained).expect("serializable retained completion evidence");
    assert_eq!(
        exact_sha256(&retained_value, "storageReceiptSha256"),
        storage_receipt_sha256
    );
    assert_eq!(
        exact_sha256(&retained_value, "completionReceiptSha256"),
        completion_receipt_sha256
    );
    assert!(
        retained_value.get("manifest").is_none(),
        "later retained calls also avoid installed-graph inspection"
    );
    let retained_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.authentication_rate_limit \
         WHERE key_hash = decode(repeat('44',32),'hex')",
    )
    .fetch_one(&pool)
    .await
    .expect("retained ordinary state lookup");
    assert_eq!(retained_rows, 1);

    sqlx::query("DELETE FROM public.live_demo_install_state")
        .execute(&pool)
        .await
        .expect("disposable missing-marker setup");
    assert_eq!(
        api_store
            .completed_live_demo_installation_generation()
            .await
            .expect("missing generation broker read"),
        None
    );
    sqlx::query(
        "INSERT INTO public.live_demo_install_state \
         (singleton, state, baseline_version, tenant_id, installation_generation, object_manifest) \
         VALUES (true, 'installing', 'base-course-v1', $1, $2, '[]'::jsonb)",
    )
    .bind(participants.tenant.as_uuid())
    .bind(Uuid::from_u128(9))
    .execute(&pool)
    .await
    .expect("disposable installing marker");
    assert_eq!(
        api_store
            .completed_live_demo_installation_generation()
            .await
            .expect("installing generation broker read"),
        None
    );
}
