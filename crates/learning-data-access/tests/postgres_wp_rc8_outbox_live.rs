#![cfg(feature = "postgres")]

//! Disposable PostgreSQL 17 authority oracle for invitation-delivery outbox work.

use learning_data_access::postgres::{
    PostgresInvitationDeliveryWorkerStore, apply_migrations, lazy_pool, migration_status,
    verify_invitation_delivery_worker_schema,
};
use learning_data_access::{CompleteCourseInvitationDelivery, CourseInvitationDeliveryWorkerStore};
use question_model::ActivityTimestamp;
use sqlx::Row;
use uuid::Uuid;

const WORKER_LOGIN: &str = "ple_invitation_delivery_worker_login";
const WORKER_PASSWORD: &str = "rc8-disposable-worker-password";

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("disposable live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

async fn insert_delivery_fixture(pool: &sqlx::PgPool, expired: bool) -> (Uuid, Uuid, Uuid, Uuid) {
    let tenant = id();
    let course = id();
    let instructor = id();
    let invitation = id();
    let mut token_hash = [0_u8; 32];
    getrandom::fill(&mut token_hash).expect("disposable invitation token-hash randomness");
    let mut transaction = pool
        .begin()
        .await
        .expect("begin isolated invitation fixture");
    sqlx::query(
        "INSERT INTO course (tenant_id, course_id, title, term_start_date, term_end_date, \
         time_zone) VALUES ($1, $2, $3, DATE '2026-08-24', DATE '2026-12-18', \
         'America/Chicago')",
    )
    .bind(tenant)
    .bind(course)
    .bind("RC8 disposable invitation delivery")
    .execute(&mut *transaction)
    .await
    .expect("insert isolated invitation course");
    sqlx::query("INSERT INTO course_roster_state (tenant_id, course_id) VALUES ($1, $2)")
        .bind(tenant)
        .bind(course)
        .execute(&mut *transaction)
        .await
        .expect("insert isolated invitation roster state");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("scope isolated invitation fixture to its tenant");
    sqlx::query(
        "INSERT INTO course_invitation (tenant_id, course_id, invitation_id, token_hash, normalized_email, \
         delivery_email, roster_id, invited_by, idempotency_key, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $5, '900000001', $6, $7, \
                 transaction_timestamp() + interval '1 day')",
    )
        .bind(tenant)
        .bind(course)
        .bind(invitation)
        .bind(token_hash.as_slice())
        .bind(format!("delivery-{}@example.invalid", id()))
        .bind(instructor)
        .bind(format!("delivery-{}", id()))
        .execute(&mut *transaction)
        .await
        .expect("insert isolated invitation");
    let delivery = id();
    sqlx::query(
        "INSERT INTO course_invitation_delivery (tenant_id, course_id, invitation_id, delivery_id) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(tenant)
    .bind(course)
    .bind(invitation)
    .bind(delivery)
    .execute(&mut *transaction)
    .await
    .expect("create one durable delivery for the invitation");
    if expired {
        sqlx::query(
            "UPDATE course_invitation \
             SET created_at = transaction_timestamp() - interval '2 days', \
                 expires_at = transaction_timestamp() - interval '1 day' \
             WHERE tenant_id = $1 AND course_id = $2 AND invitation_id = $3",
        )
        .bind(tenant)
        .bind(course)
        .bind(invitation)
        .execute(&mut *transaction)
        .await
        .expect("make an otherwise valid invitation naturally expired");
    }
    transaction
        .commit()
        .await
        .expect("commit isolated invitation fixture");
    (tenant, course, invitation, delivery)
}

async fn delivery_state(pool: &sqlx::PgPool, delivery: Uuid) -> (String, bool, bool) {
    let row = sqlx::query(
        "SELECT state, lease_id IS NULL AS lease_cleared, terminal_at IS NOT NULL AS terminal \
         FROM course_invitation_delivery WHERE delivery_id = $1",
    )
    .bind(delivery)
    .fetch_one(pool)
    .await
    .expect("owner reads isolated delivery result");
    (
        row.try_get("state").expect("delivery state"),
        row.try_get("lease_cleared").expect("lease state"),
        row.try_get("terminal").expect("terminal state"),
    )
}

#[tokio::test]
#[ignore = "requires the disposable WP-RC8 PostgreSQL acceptance database"]
async fn postgres_wp_rc8_invitation_delivery_authority_and_outbox() {
    let runtime = load_acceptance_runtime();
    let database_url = runtime.admin_url().expose();
    let owner_pool = lazy_pool(database_url).expect("valid disposable PostgreSQL URL");
    apply_migrations(&owner_pool)
        .await
        .expect("fresh embedded migrations apply");
    let status = migration_status(&owner_pool)
        .await
        .expect("migration ledger is readable by migration owner");
    apply_migrations(&owner_pool)
        .await
        .expect("embedded migrations converge");
    assert_eq!(
        migration_status(&owner_pool)
            .await
            .expect("converged ledger"),
        status
    );

    sqlx::query("CREATE ROLE ple_invitation_delivery_worker_login LOGIN PASSWORD 'rc8-disposable-worker-password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS")
        .execute(&owner_pool)
        .await
        .expect("deployment-owned worker login provisions after migrations");
    sqlx::query("GRANT ple_invitation_delivery_worker TO ple_invitation_delivery_worker_login WITH ADMIN FALSE, INHERIT FALSE, SET TRUE")
        .execute(&owner_pool)
        .await
        .expect("PostgreSQL 17 direct SET-only capability grant");

    // The owner URL contains a login already.  Build an explicit local URL
    // from its authority-free suffix without printing credentials.
    let authority = database_url
        .split_once('@')
        .map(|(_, suffix)| suffix)
        .expect("disposable owner URL contains credentials");
    let worker_url = format!("postgres://{WORKER_LOGIN}:{WORKER_PASSWORD}@{authority}");
    let worker_pool = lazy_pool(&worker_url).expect("worker URL parses");
    verify_invitation_delivery_worker_schema(&worker_pool)
        .await
        .expect("worker verifies exact schema through execute-only function");
    let worker = PostgresInvitationDeliveryWorkerStore::new(worker_pool.clone());
    assert!(
        worker
            .claim_due_course_invitation_deliveries(1, 60)
            .await
            .expect("worker broker claim executes")
            .is_empty()
    );

    // Two independent worker Stores cannot lease one ready row twice.
    let (_, _, _, ready_delivery) = insert_delivery_fixture(&owner_pool, false).await;
    let first = worker
        .claim_due_course_invitation_deliveries(1, 60)
        .await
        .expect("first worker claim");
    assert_eq!(first.len(), 1, "one ready invitation receives one lease");
    assert_eq!(first[0].delivery.id.as_uuid(), ready_delivery);
    let second_worker = PostgresInvitationDeliveryWorkerStore::new(worker_pool.clone());
    assert!(
        second_worker
            .claim_due_course_invitation_deliveries(1, 60)
            .await
            .expect("second worker claim")
            .is_empty()
    );
    assert!(
        worker
            .prepare_course_invitation_delivery(first[0].delivery.id, first[0].lease)
            .await
            .expect("first preparation")
            .is_some()
    );
    assert!(
        worker
            .prepare_course_invitation_delivery(first[0].delivery.id, first[0].lease)
            .await
            .expect("one-shot preparation probe")
            .is_none()
    );

    // A stale lease cannot complete after the broker has fenced it.
    sqlx::query(
        "UPDATE course_invitation_delivery SET lease_expires_at = transaction_timestamp() - interval '1 second' \
         WHERE delivery_id = $1",
    )
    .bind(ready_delivery)
    .execute(&owner_pool)
    .await
    .expect("make only the disposable test lease stale");
    assert!(
        worker
            .claim_due_course_invitation_deliveries(1, 60)
            .await
            .expect("expired prepared lease reconciliation")
            .is_empty()
    );
    assert!(
        !worker
            .complete_course_invitation_delivery(
                first[0].delivery.id,
                first[0].lease,
                CompleteCourseInvitationDelivery::AcceptedByProvider,
            )
            .await
            .expect("stale completion is fenced")
    );
    assert_eq!(
        delivery_state(&owner_pool, ready_delivery).await.0,
        "ambiguous"
    );

    // Natural expiry has distinct pre- and post-prepare terminal outcomes.
    let (_, _, _, expired_delivery) = insert_delivery_fixture(&owner_pool, true).await;
    assert!(
        worker
            .claim_due_course_invitation_deliveries(10, 60)
            .await
            .expect("natural expiry reconciliation")
            .is_empty()
    );
    assert_eq!(
        delivery_state(&owner_pool, expired_delivery).await,
        ("cancelled".to_string(), true, true),
        "unprepared expiry is terminal cancellation"
    );
    let (prepared_expiry_tenant, prepared_expiry_course, _, prepared_expiry_delivery) =
        insert_delivery_fixture(&owner_pool, false).await;
    let prepared_expiry = worker
        .claim_due_course_invitation_deliveries(1, 60)
        .await
        .expect("claim active expiry fixture")
        .pop()
        .expect("active expiry fixture is claimed");
    assert!(
        worker
            .prepare_course_invitation_delivery(prepared_expiry.delivery.id, prepared_expiry.lease)
            .await
            .expect("prepare active expiry fixture")
            .is_some()
    );
    let mut expiry = owner_pool
        .begin()
        .await
        .expect("begin prepared-expiry fixture transaction");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(prepared_expiry_tenant.to_string())
        .execute(&mut *expiry)
        .await
        .expect("scope prepared-expiry fixture to its tenant");
    sqlx::query(
        "UPDATE course_invitation \
         SET created_at = transaction_timestamp() - interval '2 days', \
             expires_at = transaction_timestamp() - interval '1 day' \
         WHERE tenant_id = $1 AND course_id = $2 AND invitation_id = $3",
    )
    .bind(prepared_expiry_tenant)
    .bind(prepared_expiry_course)
    .bind(prepared_expiry.delivery.invitation.as_uuid())
    .execute(&mut *expiry)
    .await
    .expect("advance only the disposable invitation past expiry");
    expiry
        .commit()
        .await
        .expect("commit prepared-expiry fixture");
    assert!(
        worker
            .claim_due_course_invitation_deliveries(10, 60)
            .await
            .expect("prepared natural expiry reconciliation")
            .is_empty()
    );
    assert_eq!(
        delivery_state(&owner_pool, prepared_expiry_delivery)
            .await
            .0,
        "ambiguous",
        "prepared expiry remains terminally ambiguous"
    );

    // Three prepared retryable outcomes reach the closed terminal budget.
    let (_, _, _, retry_delivery) = insert_delivery_fixture(&owner_pool, false).await;
    for _ in 0..3 {
        let claimed = worker
            .claim_due_course_invitation_deliveries(1, 60)
            .await
            .expect("claim retryable delivery")
            .pop()
            .expect("retryable delivery remains claimable before its budget");
        assert!(
            worker
                .prepare_course_invitation_delivery(claimed.delivery.id, claimed.lease)
                .await
                .expect("prepare retryable delivery")
                .is_some()
        );
        assert!(
            worker
                .complete_course_invitation_delivery(
                    claimed.delivery.id,
                    claimed.lease,
                    CompleteCourseInvitationDelivery::RetryableFailed {
                        next_attempt_at: ActivityTimestamp::from_unix_millis(0),
                    },
                )
                .await
                .expect("retryable completion")
        );
    }
    assert_eq!(
        delivery_state(&owner_pool, retry_delivery).await.0,
        "permanent_failed",
        "the closed retry budget terminalizes instead of reclaiming"
    );

    // The application role receives no cross-tenant delivery disclosure.
    let (foreign_tenant, foreign_course, foreign_invitation, _) =
        insert_delivery_fixture(&owner_pool, false).await;
    let mut foreign = owner_pool.begin().await.expect("foreign tenant probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *foreign)
        .await
        .expect("application role for foreign tenant probe");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(id().to_string())
        .execute(&mut *foreign)
        .await
        .expect("foreign tenant context");
    assert!(
        sqlx::query(
            "SELECT state FROM course_invitation_delivery \
             WHERE tenant_id = $1 AND course_id = $2 AND invitation_id = $3",
        )
        .bind(foreign_tenant)
        .bind(foreign_course)
        .bind(foreign_invitation)
        .fetch_optional(&mut *foreign)
        .await
        .expect("foreign delivery query")
        .is_none()
    );
    foreign.rollback().await.expect("foreign probe rollback");

    // The function-only verifier fails closed on a reversible ledger mismatch.
    let mutation = sqlx::query(
        "SELECT version, checksum FROM ple_migration_state ORDER BY version DESC LIMIT 1",
    )
    .fetch_one(&owner_pool)
    .await
    .expect("select one disposable migration checksum");
    let version: i64 = mutation.try_get("version").expect("migration version");
    let checksum: Vec<u8> = mutation.try_get("checksum").expect("migration checksum");
    sqlx::query("UPDATE ple_migration_state SET checksum = $1 WHERE version = $2")
        .bind([0_u8].as_slice())
        .bind(version)
        .execute(&owner_pool)
        .await
        .expect("make reversible disposable schema mismatch");
    assert!(
        verify_invitation_delivery_worker_schema(&worker_pool)
            .await
            .is_err()
    );
    sqlx::query("UPDATE ple_migration_state SET checksum = $1 WHERE version = $2")
        .bind(checksum)
        .bind(version)
        .execute(&owner_pool)
        .await
        .expect("restore disposable migration checksum");
    verify_invitation_delivery_worker_schema(&worker_pool)
        .await
        .expect("restored schema verifier");

    let mut transaction = worker_pool
        .begin()
        .await
        .expect("worker authority transaction");
    sqlx::query("SET LOCAL ROLE ple_invitation_delivery_worker")
        .execute(&mut *transaction)
        .await
        .expect("worker login can assume exactly its direct capability");
    for statement in [
        "SELECT * FROM public.course_invitation_delivery",
        "SELECT * FROM public.ple_migration_state",
        "UPDATE public.course_invitation_delivery SET state = 'cancelled'",
    ] {
        assert!(
            sqlx::query(statement)
                .execute(&mut *transaction)
                .await
                .is_err()
        );
    }
    transaction
        .rollback()
        .await
        .expect("denied table probe rolls back");

    let mut app = owner_pool
        .begin()
        .await
        .expect("application authority probe");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *app)
        .await
        .expect("app role");
    for function in [
        "SELECT * FROM public.ple_claim_course_invitation_deliveries(1, 60)",
        "SELECT * FROM public.ple_prepare_course_invitation_delivery(gen_random_uuid(), gen_random_uuid())",
        "SELECT public.ple_revalidate_course_invitation_delivery_lease(gen_random_uuid(), gen_random_uuid())",
    ] {
        assert!(sqlx::query(function).execute(&mut *app).await.is_err());
    }
    app.rollback()
        .await
        .expect("application function denial rolls back");

    let malformed = sqlx::query("CREATE ROLE rc8_extra_login LOGIN PASSWORD 'unused'")
        .execute(&owner_pool)
        .await;
    assert!(
        malformed.is_ok(),
        "fresh disposable DB accepts an isolated malformed role fixture"
    );
}
#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;
