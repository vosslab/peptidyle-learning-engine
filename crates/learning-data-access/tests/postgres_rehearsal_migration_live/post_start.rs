//! Post-start operation and concealment evidence over a real frozen assignment.
//!
//! The older direct-SQL start tests are intentionally not compiled: their
//! empty assignment state could not prove the normal instructor workflow.

use learning_data_access::{
    ClaimRehearsalDeliveryRouteCommand, DiscardRehearsalRouteCommand, RehearsalDeliveryClaimResult,
    RehearsalDeliveryMaterialStore, RehearsalIdempotencyKey, RehearsalOperationDigest,
    RehearsalRouteIdentity, RehearsalRouteMutationStore, RehearsalSafeProjection, StoreError,
    TenantContext,
};
use question_model::{TeachingOperationRevision, TenantId, UserId};

use super::canonical_store::started_fixture;

pub(super) fn route(fixture: &super::canonical_store::StartedFixture) -> RehearsalRouteIdentity {
    RehearsalRouteIdentity {
        actor: fixture.actor,
        course: fixture.course,
        assignment: fixture.assignment,
        rehearsal: fixture.rehearsal,
        expected_revision: fixture.revision,
    }
}

pub(super) fn delivery(
    route: RehearsalRouteIdentity,
    key: &str,
    byte: u8,
) -> ClaimRehearsalDeliveryRouteCommand {
    ClaimRehearsalDeliveryRouteCommand {
        route,
        idempotency_key: RehearsalIdempotencyKey::new(key.into()).expect("idempotency key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([byte; 32]),
    }
}

fn screen() -> RehearsalSafeProjection {
    RehearsalSafeProjection::new(serde_json::json!({"state": "discarded"}), 1024)
        .expect("answer-free projection")
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn post_start_delivery_replay_conflict_and_concealment_use_frozen_material() {
    let fixture = started_fixture().await;
    let route = route(&fixture);
    fixture
        .store
        .verify_rehearsal_delivery_material_from_route(
            fixture.context,
            learning_data_access::VerifyRehearsalDeliveryMaterialRouteCommand { route },
        )
        .await
        .expect("active route verifies immutable material without returning it");
    let first = fixture
        .store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            delivery(route, "post-start-delivery", 0x61),
        )
        .await
        .expect("route claim over frozen material");
    assert!(matches!(
        first,
        RehearsalDeliveryClaimResult::Prepared { .. }
    ));

    let replay = fixture
        .store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            delivery(route, "post-start-delivery", 0x61),
        )
        .await
        .expect("same route request recovers its prepared operation");
    assert!(matches!(
        replay,
        RehearsalDeliveryClaimResult::Prepared { .. }
    ));

    assert!(matches!(
        fixture
            .store
            .claim_rehearsal_delivery_from_route(
                fixture.context,
                delivery(route, "post-start-delivery", 0x62),
            )
            .await,
        Ok(RehearsalDeliveryClaimResult::Conflict)
    ));

    let foreign = RehearsalRouteIdentity {
        actor: UserId::from_uuid(super::canonical_store::id()),
        ..route
    };
    assert!(matches!(
        fixture
            .store
            .verify_rehearsal_delivery_material_from_route(
                fixture.context,
                learning_data_access::VerifyRehearsalDeliveryMaterialRouteCommand {
                    route: foreign
                },
            )
            .await,
        Err(StoreError::NotFound)
    ));
    let foreign_tenant = TenantContext::from_authenticated_session(TenantId::from_uuid(
        super::canonical_store::id(),
    ));
    assert!(matches!(
        fixture
            .store
            .verify_rehearsal_delivery_material_from_route(
                foreign_tenant,
                learning_data_access::VerifyRehearsalDeliveryMaterialRouteCommand { route },
            )
            .await,
        Err(StoreError::NotFound)
    ));
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn post_start_discard_is_route_scoped_and_idempotent() {
    let fixture = started_fixture().await;
    let route = route(&fixture);
    let command = DiscardRehearsalRouteCommand {
        route,
        idempotency_key: RehearsalIdempotencyKey::new("post-start-discard".into())
            .expect("idempotency key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([0x71; 32]),
        response: screen(),
        response_digest: RehearsalOperationDigest::from_bytes([0x72; 32]),
    };
    let first = fixture
        .store
        .discard_rehearsal_from_route(fixture.context, command.clone())
        .await
        .expect("first route discard");
    let replay = fixture
        .store
        .discard_rehearsal_from_route(fixture.context, command)
        .await
        .expect("discard replay");
    assert!(matches!(
        first,
        learning_data_access::RehearsalIdempotentProjectionResult::Applied(_)
    ));
    assert!(matches!(
        replay,
        learning_data_access::RehearsalIdempotentProjectionResult::Replay(_)
    ));

    let stale = RehearsalRouteIdentity {
        expected_revision: TeachingOperationRevision::new(fixture.revision.value() + 1)
            .expect("stale revision"),
        ..route
    };
    assert!(matches!(
        fixture
            .store
            .discard_rehearsal_from_route(
                fixture.context,
                DiscardRehearsalRouteCommand {
                    route: stale,
                    idempotency_key: RehearsalIdempotencyKey::new("stale-discard".into())
                        .expect("idempotency key"),
                    request_fingerprint: RehearsalOperationDigest::from_bytes([0x73; 32]),
                    response: screen(),
                    response_digest: RehearsalOperationDigest::from_bytes([0x74; 32]),
                },
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        fixture
            .store
            .verify_rehearsal_delivery_material_from_route(
                fixture.context,
                learning_data_access::VerifyRehearsalDeliveryMaterialRouteCommand { route },
            )
            .await,
        Err(StoreError::NotFound)
    ));
}

#[tokio::test]
#[ignore = "requires PLE_TEST_DATABASE_URL and disposable PostgreSQL 17"]
async fn route_material_verification_fails_closed_on_corrupt_frozen_material() {
    let fixture = started_fixture().await;
    let route = route(&fixture);
    let mut transaction = fixture.pool.begin().await.expect("fault transaction");
    sqlx::query(
        "ALTER TABLE public.rehearsal_frozen_source_snapshot
         DISABLE TRIGGER rehearsal_source_snapshot_append_only",
    )
    .execute(&mut *transaction)
    .await
    .expect("disable append-only trigger in disposable fault transaction");
    let updated = sqlx::query(
        "UPDATE public.rehearsal_frozen_source_snapshot
            SET issued_snapshot_bytes=issued_snapshot_bytes || decode('00', 'hex'),
                issued_snapshot_sha256=digest(
                    issued_snapshot_bytes || decode('00', 'hex'), 'sha256')
          WHERE tenant_id=$1 AND rehearsal_run_id=$2 AND ordinal=0",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.run_id)
    .execute(&mut *transaction)
    .await
    .expect("corrupt one disposable frozen snapshot commitment");
    assert_eq!(updated.rows_affected(), 1);
    sqlx::query(
        "ALTER TABLE public.rehearsal_frozen_source_snapshot
         ENABLE TRIGGER rehearsal_source_snapshot_append_only",
    )
    .execute(&mut *transaction)
    .await
    .expect("restore append-only trigger");
    transaction.commit().await.expect("commit disposable fault");

    assert!(matches!(
        fixture
            .store
            .verify_rehearsal_delivery_material_from_route(
                fixture.context,
                learning_data_access::VerifyRehearsalDeliveryMaterialRouteCommand { route },
            )
            .await,
        Err(StoreError::Unavailable(_))
    ));
}
