use super::*;

fn render_job(tenant: TenantId, value: u128, max_attempts: u16) -> EnqueueJob {
    EnqueueJob {
        tenant,
        payload: JobPayload::Render {
            reference: ProblemVersionRef {
                problem: ProblemId::from_uuid(uuid(value)),
                version: VersionId::from_uuid(uuid(value + 1)),
            },
            seed: u64::try_from(value).expect("fixture seed fits"),
        },
        max_attempts,
    }
}

fn export_job(tenant: TenantId, value: u128) -> EnqueueJob {
    EnqueueJob {
        tenant,
        payload: JobPayload::Export {
            delivery_object: ObjectId::from_uuid(uuid(value)),
        },
        max_attempts: 2,
    }
}

async fn exercise_job_store_claim_boundary<S>(store: &S)
where
    S: JobStore,
{
    let tenant = TenantId::from_uuid(uuid(9_100));
    let foreign = TenantId::from_uuid(uuid(9_101));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign);
    let lease = JobLeaseDuration::from_seconds(30).expect("bounded lease");
    let first = store
        .enqueue_job(context, render_job(tenant, 9_110, 1))
        .await
        .expect("tenant enqueue");
    store
        .enqueue_job(context, render_job(tenant, 9_120, 1))
        .await
        .expect("second tenant enqueue");
    assert_eq!(
        store
            .get_job(foreign_context, first)
            .await
            .expect("foreign lookup"),
        None
    );
    let filter = JobClaimFilter::all();
    let (left, right) = tokio::join!(
        store.claim_next_job(&filter, lease),
        store.claim_next_job(&filter, lease)
    );
    let left = left.expect("left claim").expect("left job");
    let right = right.expect("right claim").expect("right job");
    assert_ne!(
        left.id, right.id,
        "two workers must not claim one row twice"
    );
    assert_eq!(left.tenant, tenant, "broker returns the stored tenant");
    assert_eq!(right.tenant, tenant, "broker returns the stored tenant");
    assert!(matches!(
        store
            .complete_job(left.id, JobLeaseToken::generate().expect("test token"))
            .await,
        Err(StoreError::Conflict)
    ));
    store
        .complete_job(left.id, left.lease_token)
        .await
        .expect("current token completes left job");
    assert_eq!(
        store
            .fail_job(right.id, right.lease_token, JobFailureKind::Permanent)
            .await
            .expect("current token can dead-letter right job"),
        JobFailureDisposition::Dead
    );
    assert_eq!(
        store
            .get_job(context, right.id)
            .await
            .expect("owner lookup")
            .expect("dead row remains inspectable")
            .state,
        JobState::Dead
    );
    assert_eq!(
        store
            .ready_queue_depth(&JobClaimFilter::all())
            .await
            .expect("depth after broker finalization")
            .ready,
        0
    );
}

#[tokio::test]
async fn memory_job_store_claim_boundary_conforms() {
    exercise_job_store_claim_boundary(&MemoryStore::default()).await;
}

#[tokio::test]
async fn memory_job_store_claim_filter_leaves_reserved_and_expired_work_untouched() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(uuid(9_200));
    let context = TenantContext::from_authenticated_session(tenant);
    let lease = JobLeaseDuration::from_seconds(1).expect("lease");
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("clock");
    let reserved = store
        .enqueue_job(context, render_job(tenant, 9_210, 1))
        .await
        .expect("reserved render");
    let supported = store
        .enqueue_job(context, export_job(tenant, 9_220))
        .await
        .expect("supported export");
    let render_filter = JobClaimFilter::new([JobKind::Render]).expect("render filter");
    let export_filter = JobClaimFilter::new([JobKind::Export]).expect("export filter");

    assert_eq!(
        store
            .ready_queue_depth(&export_filter)
            .await
            .expect("filtered depth")
            .ready,
        1
    );
    let render_claim = store
        .claim_next_job(&render_filter, lease)
        .await
        .expect("render claim")
        .expect("render exists");
    assert_eq!(render_claim.id, reserved);
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_001))
        .expect("lease expiry");

    let export_claim = store
        .claim_next_job(&export_filter, lease)
        .await
        .expect("export claim")
        .expect("export exists");
    assert_eq!(export_claim.id, supported);
    assert_eq!(
        store
            .get_job(context, reserved)
            .await
            .expect("reserved inspection")
            .expect("reserved job")
            .state,
        JobState::Leased,
        "another worker family must not dead-letter an expired reserved lease"
    );
    assert_eq!(
        store
            .claim_next_job(&render_filter, lease)
            .await
            .expect("render cleanup"),
        None
    );
    assert_eq!(
        store
            .get_job(context, reserved)
            .await
            .expect("dead inspection")
            .expect("dead job")
            .state,
        JobState::Dead
    );
}

#[tokio::test]
async fn memory_job_store_enforces_atomic_leases_retries_depth_and_tenants() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(uuid(9_001));
    let foreign = TenantId::from_uuid(uuid(9_002));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign);
    let lease = JobLeaseDuration::from_seconds(1).expect("bounded lease");
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("memory clock should be writable");

    let first = store
        .enqueue_job(context, render_job(tenant, 9_010, 2))
        .await
        .expect("tenant should enqueue its job");
    let _second = store
        .enqueue_job(context, render_job(tenant, 9_020, 1))
        .await
        .expect("tenant should enqueue another job");
    assert_eq!(
        store
            .ready_queue_depth(&JobClaimFilter::all())
            .await
            .expect("depth")
            .ready,
        2
    );
    assert_eq!(
        store
            .get_job(foreign_context, first)
            .await
            .expect("foreign read is bounded"),
        None,
        "tenant inspection must not reveal another tenant's job"
    );

    let filter = JobClaimFilter::all();
    let (claim_one, claim_two) = tokio::join!(
        store.claim_next_job(&filter, lease),
        store.claim_next_job(&filter, lease)
    );
    let claim_one = claim_one.expect("first claim").expect("first queued job");
    let claim_two = claim_two.expect("second claim").expect("second queued job");
    assert_ne!(
        claim_one.id, claim_two.id,
        "two claims must never duplicate a job"
    );
    assert_eq!(claim_one.tenant, tenant);
    assert_eq!(claim_two.tenant, tenant);
    assert_eq!(
        store
            .ready_queue_depth(&JobClaimFilter::all())
            .await
            .expect("depth")
            .ready,
        0
    );
    let (reclaimable_claim, exhausted_claim) = if claim_one.id == first {
        (claim_one, claim_two)
    } else {
        (claim_two, claim_one)
    };

    // Let the first lease expire. Its token can no longer complete after the
    // reclaimed lease is issued.
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_001))
        .expect("memory clock should advance");
    let reclaimed = store
        .claim_next_job(&JobClaimFilter::all(), lease)
        .await
        .expect("reclaim should succeed")
        .expect("expired job should be reclaimable");
    assert_eq!(reclaimed.id, reclaimable_claim.id);
    assert_eq!(reclaimed.attempt_count, 2);
    assert!(matches!(
        store
            .complete_job(reclaimable_claim.id, reclaimable_claim.lease_token)
            .await,
        Err(StoreError::Conflict)
    ));
    store
        .complete_job(reclaimed.id, reclaimed.lease_token)
        .await
        .expect("current lease token completes exactly once");
    assert_eq!(
        store
            .get_job(context, reclaimed.id)
            .await
            .expect("owner lookup")
            .expect("completed row retained")
            .state,
        JobState::Completed
    );

    // The one-attempt job was left leased by the parallel claim. Its expiry
    // becomes a dead row and never inflates ready depth.
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(3_002))
        .expect("memory clock should advance");
    assert_eq!(
        store
            .claim_next_job(&JobClaimFilter::all(), lease)
            .await
            .expect("claim"),
        None
    );
    assert_eq!(
        store
            .get_job(context, exhausted_claim.id)
            .await
            .expect("owner lookup")
            .expect("dead row retained")
            .state,
        JobState::Dead
    );

    let retry = store
        .enqueue_job(context, render_job(tenant, 9_030, 2))
        .await
        .expect("retry fixture enqueue");
    let retry_claim = store
        .claim_next_job(&JobClaimFilter::all(), lease)
        .await
        .expect("claim retry fixture")
        .expect("retry fixture ready");
    assert_eq!(retry_claim.id, retry);
    assert_eq!(
        store
            .fail_job(retry, retry_claim.lease_token, JobFailureKind::Transient)
            .await
            .expect("first transient failure"),
        JobFailureDisposition::Retrying
    );
    assert_eq!(
        store
            .ready_queue_depth(&JobClaimFilter::all())
            .await
            .expect("delayed depth")
            .ready,
        0
    );
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(4_003))
        .expect("memory clock should advance through backoff");
    let final_claim = store
        .claim_next_job(&JobClaimFilter::all(), lease)
        .await
        .expect("second retry claim")
        .expect("retry becomes eligible");
    assert_eq!(final_claim.id, retry);
    assert_eq!(
        store
            .fail_job(retry, final_claim.lease_token, JobFailureKind::Transient)
            .await
            .expect("attempt exhaustion"),
        JobFailureDisposition::Dead
    );
    assert_eq!(
        store
            .ready_queue_depth(&JobClaimFilter::all())
            .await
            .expect("dead depth")
            .ready,
        0
    );
}
