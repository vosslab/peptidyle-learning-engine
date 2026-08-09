#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for family-filtered concurrent queue claims.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    EnqueueJob, JobClaimFilter, JobKind, JobLeaseDuration, JobPayload, JobState, JobStore,
    TenantContext,
};
use question_model::{ObjectId, ProblemId, ProblemVersionRef, TenantId, VersionId};
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn export_job(tenant: TenantId) -> EnqueueJob {
    EnqueueJob {
        tenant,
        payload: JobPayload::Export {
            delivery_object: ObjectId::from_uuid(id()),
        },
        max_attempts: 2,
    }
}

fn reserved_render_job(tenant: TenantId) -> EnqueueJob {
    EnqueueJob {
        tenant,
        payload: JobPayload::Render {
            reference: ProblemVersionRef {
                problem: ProblemId::from_uuid(id()),
                version: VersionId::from_uuid(id()),
            },
            seed: 7,
        },
        max_attempts: 1,
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_worker_claim_filter_is_concurrent_and_leaves_reserved_work_untouched() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::new(pool.clone());
    let first_tenant = TenantId::from_uuid(id());
    let second_tenant = TenantId::from_uuid(id());
    let first_context = TenantContext::from_authenticated_session(first_tenant);
    let second_context = TenantContext::from_authenticated_session(second_tenant);

    let reserved = store
        .enqueue_job(first_context, reserved_render_job(first_tenant))
        .await
        .expect("reserved render enqueue");
    let first = store
        .enqueue_job(first_context, export_job(first_tenant))
        .await
        .expect("first supported enqueue");
    let second = store
        .enqueue_job(second_context, export_job(second_tenant))
        .await
        .expect("second supported enqueue");
    let supported = JobClaimFilter::new([JobKind::Export]).expect("supported filter");
    let lease = JobLeaseDuration::from_seconds(30).expect("lease");

    assert_eq!(
        store
            .ready_queue_depth(&supported)
            .await
            .expect("supported depth")
            .ready,
        2
    );
    let (left, right) = tokio::join!(
        store.claim_next_job(&supported, lease),
        store.claim_next_job(&supported, lease)
    );
    let left = left.expect("left claim").expect("left job");
    let right = right.expect("right claim").expect("right job");
    assert_ne!(left.id, right.id);
    assert_eq!(
        [left.id, right.id]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        [first, second]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
    );
    assert!(matches!(left.payload, JobPayload::Export { .. }));
    assert!(matches!(right.payload, JobPayload::Export { .. }));
    assert_eq!(
        store
            .ready_queue_depth(&supported)
            .await
            .expect("drained supported depth")
            .ready,
        0
    );
    assert_eq!(
        store
            .ready_queue_depth(&JobClaimFilter::all())
            .await
            .expect("all-family depth")
            .ready,
        1
    );
    assert_eq!(
        store
            .get_job(first_context, reserved)
            .await
            .expect("reserved view")
            .expect("reserved job")
            .state,
        JobState::Ready
    );
    store
        .complete_job(left.id, left.lease_token)
        .await
        .expect("complete left");
    store
        .complete_job(right.id, right.lease_token)
        .await
        .expect("complete right");

    let render_filter = JobClaimFilter::new([JobKind::Render]).expect("render filter");
    let render_claim = store
        .claim_next_job(
            &render_filter,
            JobLeaseDuration::from_seconds(1).expect("short lease"),
        )
        .await
        .expect("render claim")
        .expect("reserved render");
    assert_eq!(render_claim.id, reserved);
    sqlx::query(
        "UPDATE worker_job SET lease_expires_at = transaction_timestamp() - interval '1 second' \
         WHERE job_id = $1",
    )
    .bind(reserved.as_uuid())
    .execute(&pool)
    .await
    .expect("expire reserved lease in disposable fixture");

    assert!(
        store
            .claim_next_job(&supported, lease)
            .await
            .expect("supported empty claim")
            .is_none()
    );
    assert_eq!(
        store
            .get_job(first_context, reserved)
            .await
            .expect("expired reserved view")
            .expect("expired reserved job")
            .state,
        JobState::Leased,
        "another family must not dead-letter the expired reserved lease"
    );
    assert!(
        store
            .claim_next_job(&render_filter, lease)
            .await
            .expect("render cleanup claim")
            .is_none()
    );
    assert_eq!(
        store
            .get_job(first_context, reserved)
            .await
            .expect("dead reserved view")
            .expect("dead reserved job")
            .state,
        JobState::Dead
    );
}
