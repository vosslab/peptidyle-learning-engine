use super::load_acceptance_runtime;
use std::str::FromStr;

use base_course_installation::{
    BaseCourseAction, BaseCourseInstallError, BaseCourseInstallStateOutput,
};
use learning_data_access::StoreError;
use learning_data_access::postgres::{
    BaseCourseInstallerPool, PostgresStore, lazy_pool, verify_application_schema,
};
use question_model::UserRole;
use sqlx::postgres::PgConnectOptions;

use super::{Participants, ProductDatabase, install_request, prepare_request};

type AccountRow = (
    uuid::Uuid,
    String,
    String,
    String,
    sqlx::types::Json<Vec<UserRole>>,
);

async fn expect_account_conflict(
    installer: &BaseCourseInstallerPool,
    store: &PostgresStore,
    participants: Participants,
    receipt: &str,
) {
    let error =
        base_course_installation::install(installer, store, install_request(participants, receipt))
            .await
            .expect_err("drifted product accounts fail closed");
    assert!(
        matches!(
            &error,
            BaseCourseInstallError::Persistence {
                source: StoreError::AlreadyExists,
                ..
            }
        ),
        "{error:?}"
    );
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn base_course_product_atomically_converges_exact_accounts_without_credentials() {
    let runtime = load_acceptance_runtime();
    let url = runtime.admin_url().expose();
    let admin = super::admin_pool(url).await;
    super::reset_disposable_course_capability_memberships(&admin).await;
    let pool = lazy_pool(url).expect("PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("full migrated application schema is compatible");
    let database = PgConnectOptions::from_str(url)
        .expect("PostgreSQL URL")
        .get_database()
        .expect("database name")
        .to_string();
    let product = ProductDatabase::provision(&admin, url, &database).await;
    let participants = Participants::fresh();
    let store = product.store();
    let installer = product.installer_pool();
    let prepared =
        base_course_installation::install(&installer, &store, prepare_request(participants))
            .await
            .expect("prepare exact Base Course product accounts");
    sqlx::query(
        "INSERT INTO public.ple_account \
         (user_id,normalized_email,delivery_email,display_name,platform_roles) \
         VALUES ($1,'mary.okafor@live-demo.ple.example',\
         'mary.okafor@live-demo.ple.example','Drifted Account','[]'::jsonb)",
    )
    .bind(participants.mary.as_uuid())
    .execute(&pool)
    .await
    .expect("seed display-name drift before account convergence");
    expect_account_conflict(
        &installer,
        &store,
        participants,
        prepared.storage_receipt_json(),
    )
    .await;
    let rolled_back_accounts: i64 = sqlx::query_scalar("SELECT count(*) FROM public.ple_account")
        .fetch_one(&pool)
        .await
        .expect("failed account batch lookup");
    assert_eq!(
        rolled_back_accounts, 1,
        "the failed account batch inserts no prefix"
    );
    let rolled_back_missing: i64 =
        sqlx::query_scalar("SELECT count(*) FROM public.ple_account WHERE user_id=$1")
            .bind(participants.jack.as_uuid())
            .fetch_one(&pool)
            .await
            .expect("rolled-back inserted account lookup");
    assert_eq!(rolled_back_missing, 0);

    sqlx::query(
        "UPDATE public.ple_account SET display_name='Mary Okafor', \
         normalized_email='drifted@example.invalid' WHERE user_id=$1",
    )
    .bind(participants.mary.as_uuid())
    .execute(&pool)
    .await
    .expect("restore display name and seed normalized-email drift");
    expect_account_conflict(
        &installer,
        &store,
        participants,
        prepared.storage_receipt_json(),
    )
    .await;

    sqlx::query(
        "UPDATE public.ple_account \
         SET normalized_email='mary.okafor@live-demo.ple.example', \
         delivery_email='Drift@Example.invalid' WHERE user_id=$1",
    )
    .bind(participants.mary.as_uuid())
    .execute(&pool)
    .await
    .expect("restore normalized email and seed delivery-email drift");
    expect_account_conflict(
        &installer,
        &store,
        participants,
        prepared.storage_receipt_json(),
    )
    .await;

    sqlx::query(
        "UPDATE public.ple_account \
         SET delivery_email='mary.okafor@live-demo.ple.example', \
         platform_roles='[\"sysadmin\"]'::jsonb WHERE user_id=$1",
    )
    .bind(participants.mary.as_uuid())
    .execute(&pool)
    .await
    .expect("restore delivery email and seed platform-role drift");
    expect_account_conflict(
        &installer,
        &store,
        participants,
        prepared.storage_receipt_json(),
    )
    .await;

    sqlx::query("UPDATE public.ple_account SET platform_roles='[]'::jsonb WHERE user_id=$1")
        .bind(participants.mary.as_uuid())
        .execute(&pool)
        .await
        .expect("restore ordinary platform roles");
    let completed = base_course_installation::install(
        &installer,
        &store,
        install_request(participants, prepared.storage_receipt_json()),
    )
    .await
    .expect("install exact Base Course product accounts");
    assert_eq!(completed.action(), BaseCourseAction::Resumed);
    assert_eq!(
        completed.install_state(),
        BaseCourseInstallStateOutput::Complete
    );

    let rows: Vec<AccountRow> = sqlx::query_as(
        "SELECT user_id, normalized_email, delivery_email, display_name, platform_roles \
             FROM public.ple_account WHERE user_id = ANY($1) ORDER BY display_name",
    )
    .bind(participants.user_ids().to_vec())
    .fetch_all(&pool)
    .await
    .expect("exact installed account rows");
    assert_eq!(
        rows,
        vec![
            (
                participants.avery.as_uuid(),
                "avery.singh@live-demo.ple.example".to_string(),
                "avery.singh@live-demo.ple.example".to_string(),
                "Avery Singh".to_string(),
                sqlx::types::Json(Vec::new()),
            ),
            (
                participants.instructor.as_uuid(),
                "elena.rivera@live-demo.ple.example".to_string(),
                "elena.rivera@live-demo.ple.example".to_string(),
                "Dr. Elena Rivera".to_string(),
                sqlx::types::Json(Vec::new()),
            ),
            (
                participants.jack.as_uuid(),
                "jack.chen@live-demo.ple.example".to_string(),
                "jack.chen@live-demo.ple.example".to_string(),
                "Jack Chen".to_string(),
                sqlx::types::Json(Vec::new()),
            ),
            (
                participants.mary.as_uuid(),
                "mary.okafor@live-demo.ple.example".to_string(),
                "mary.okafor@live-demo.ple.example".to_string(),
                "Mary Okafor".to_string(),
                sqlx::types::Json(Vec::new()),
            ),
            (
                participants.morgan.as_uuid(),
                "morgan.reyes@live-demo.ple.example".to_string(),
                "morgan.reyes@live-demo.ple.example".to_string(),
                "Morgan Reyes".to_string(),
                sqlx::types::Json(vec![UserRole::Sysadmin]),
            ),
        ]
    );
    let credential_count: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM public.account_passkey WHERE user_id=ANY($1)) + \
                (SELECT count(*) FROM public.account_authentication_session WHERE user_id=ANY($1)) + \
                (SELECT count(*) FROM public.webauthn_ceremony WHERE user_id=ANY($1))",
    )
    .bind(participants.user_ids().to_vec())
    .fetch_one(&pool)
    .await
    .expect("seeded account credential boundary");
    assert_eq!(
        credential_count, 0,
        "installation creates identities, not credentials"
    );

    let replay =
        base_course_installation::install(&installer, &store, prepare_request(participants))
            .await
            .expect("exact account product replay is idempotent");
    assert_eq!(replay.action(), BaseCourseAction::Retained);

    let mut auth_attempt = pool.begin().await.expect("auth privilege transaction");
    sqlx::query("SET LOCAL ROLE ple_auth")
        .execute(&mut *auth_attempt)
        .await
        .expect("assume the application authentication role");
    let grant_result = sqlx::query(
        "UPDATE public.ple_account SET platform_roles = '[\"sysadmin\"]'::jsonb \
         WHERE user_id = $1",
    )
    .bind(participants.mary.as_uuid())
    .execute(&mut *auth_attempt)
    .await;
    assert!(
        grant_result.is_err(),
        "ple_auth cannot grant Sysadmin ownership"
    );
    auth_attempt
        .rollback()
        .await
        .expect("discard denied privilege attempt");
    let account_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM public.ple_account WHERE user_id=ANY($1)")
            .bind(participants.user_ids().to_vec())
            .fetch_one(&pool)
            .await
            .expect("recovered account count");
    assert_eq!(account_count, 5);
}
