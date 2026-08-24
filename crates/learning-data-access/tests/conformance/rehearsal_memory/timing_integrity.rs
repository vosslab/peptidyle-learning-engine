use super::*;

use super::timing::{dispatch_and_complete, prepare_delivery, start_timed_rehearsal};
use learning_data_access::{
    ReconcileRehearsalDeliveryExpiryRouteCommand, RehearsalIdempotencyKey,
    RehearsalOperationDigest, RehearsalOperationStore, RehearsalRouteIdentity,
    RehearsalRouteMutationStore, RetryRehearsalDeliveryRouteCommand,
    SealedRehearsalDeliveryExecutionStore,
};
use question_model::ActivityTimestamp;

async fn dispatched_generation_for_history_integrity() -> (
    MemoryStore,
    effective_policy::EffectivePolicyFixture,
    RehearsalRouteIdentity,
) {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(0))
        .expect("set deterministic server clock");
    let (fixture, route) = start_timed_rehearsal(
        &store,
        question_model::run_policy::TimingPolicy::Untimed,
        synthetic_start(),
    )
    .await;
    let prepared = prepare_delivery(&store, &fixture, route, "history-corruption", 0x6F).await;
    let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { .. } = store
        .mark_rehearsal_delivery_dispatched(fixture.context, prepared)
        .await
        .expect("dispatch")
    else {
        panic!("dispatch is committed");
    };
    (store, fixture, route)
}

#[tokio::test]
async fn delivery_event_history_rejects_missing_duplicate_and_illegal_chronology() {
    for kind in [0_u8, 1, 2, 3, 4, 5] {
        let (store, fixture, route) = dispatched_generation_for_history_integrity().await;
        let key = RehearsalIdempotencyKey::new("history-corruption".into()).expect("key");
        let corruption = match kind {
            0 => MemoryRehearsalIntegrityTestCorruption::DropLatestDeliveryEvent {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: key,
            },
            1 => MemoryRehearsalIntegrityTestCorruption::DuplicateLatestDeliveryEvent {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: key,
            },
            2 => MemoryRehearsalIntegrityTestCorruption::AppendIllegalDeliveryEvent {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: key,
            },
            3 => MemoryRehearsalIntegrityTestCorruption::ReplaceDeliveryJournalHead {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: key,
            },
            4 => MemoryRehearsalIntegrityTestCorruption::ReplaceDeliveryJournalCount {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: key,
            },
            _ => MemoryRehearsalIntegrityTestCorruption::ReplaceDeliveryJournalPhase {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: key,
            },
        };
        store
            .corrupt_rehearsal_integrity_for_test(corruption)
            .expect("install impossible history");
        assert!(matches!(
            store.verify_rehearsal_archive_for_test(fixture.context.tenant_id(), route.rehearsal),
            Err(StoreError::InvalidRecord(_))
        ));
    }
}

#[tokio::test]
async fn delivery_journal_head_rejects_expired_tail_deletion_with_or_without_screen() {
    for completed_screen in [false, true] {
        let store = MemoryStore::default();
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(0))
            .expect("set deterministic server clock");
        let (fixture, route) = start_timed_rehearsal(
            &store,
            question_model::run_policy::TimingPolicy::PerQuestion {
                seconds: 1,
                grace_seconds: 0,
            },
            synthetic_start(),
        )
        .await;
        let key = if completed_screen {
            "expired-tail-screen"
        } else {
            "expired-tail-no-screen"
        };
        let prepared = prepare_delivery(&store, &fixture, route, key, 0x70).await;
        if completed_screen {
            dispatch_and_complete(&store, &fixture, prepared, "issued").await;
        } else {
            let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { .. } = store
                .mark_rehearsal_delivery_dispatched(fixture.context, prepared)
                .await
                .expect("dispatch")
            else {
                panic!("dispatch is committed");
            };
        }
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_001))
            .expect("advance clock");
        store
            .reconcile_rehearsal_delivery_expiry_from_route(
                fixture.context,
                ReconcileRehearsalDeliveryExpiryRouteCommand { route },
            )
            .await
            .expect("expire generation");
        store
            .corrupt_rehearsal_integrity_for_test(
                MemoryRehearsalIntegrityTestCorruption::DropLatestDeliveryEvent {
                    tenant: fixture.context.tenant_id(),
                    rehearsal: route.rehearsal,
                    idempotency_key: RehearsalIdempotencyKey::new(key.into()).expect("key"),
                },
            )
            .expect("truncate event vector only");
        assert!(matches!(
            store.verify_rehearsal_archive_for_test(fixture.context.tenant_id(), route.rehearsal),
            Err(StoreError::InvalidRecord(_))
        ));
    }
}

async fn dispatched_timed_generation_for_fail_closed() -> (
    MemoryStore,
    effective_policy::EffectivePolicyFixture,
    RehearsalRouteIdentity,
    RehearsalFrozenItemEvidence,
) {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(0))
        .expect("set deterministic server clock");
    let (fixture, route) = start_timed_rehearsal(
        &store,
        question_model::run_policy::TimingPolicy::PerQuestion {
            seconds: 1,
            grace_seconds: 0,
        },
        synthetic_start(),
    )
    .await;
    let frozen = store
        .frozen_rehearsal_item_for_test(fixture.context, route.rehearsal)
        .expect("frozen ordinary item");
    let prepared = prepare_delivery(&store, &fixture, route, "integrity-gate-continue", 0x72).await;
    let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { .. } = store
        .mark_rehearsal_delivery_dispatched(fixture.context, prepared)
        .await
        .expect("dispatch")
    else {
        panic!("dispatch is committed");
    };
    (store, fixture, route, frozen)
}

async fn assert_corruption_refuses_expiry_and_retry(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
    route: RehearsalRouteIdentity,
) {
    let ordinary_before = store
        .rehearsal_state_effect_fingerprint()
        .expect("ordinary-state baseline");
    let expiry = store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand { route },
        )
        .await;
    assert!(
        matches!(expiry, Err(StoreError::InvalidRecord(_))),
        "{expiry:?}"
    );
    assert!(matches!(
        store
            .retry_rehearsal_delivery_from_route(
                fixture.context,
                RetryRehearsalDeliveryRouteCommand {
                    route,
                    idempotency_key: RehearsalIdempotencyKey::new("integrity-gate-retry".into())
                        .expect("key"),
                    request_fingerprint: RehearsalOperationDigest::from_bytes([0x73; 32]),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("ordinary state remains unchanged"),
        ordinary_before
    );
}

#[tokio::test]
async fn aggregate_corruption_refuses_expiry_and_retry_before_any_mutation() {
    let (store, fixture, route, _) = dispatched_timed_generation_for_fail_closed().await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::DuplicateLatestDeliveryEvent {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: RehearsalIdempotencyKey::new("integrity-gate-continue".into())
                    .expect("key"),
            },
        )
        .expect("corrupt unrelated delivery journal");
    assert!(
        store
            .verify_rehearsal_archive_for_test(fixture.context.tenant_id(), route.rehearsal)
            .is_err(),
        "delivery corruption is visible to aggregate verification"
    );
    assert_corruption_refuses_expiry_and_retry(&store, &fixture, route).await;

    let (store, fixture, route, frozen) = dispatched_timed_generation_for_fail_closed().await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::ReplaceFrozenSourceContentWithRehashedChecksum {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                attempt: frozen.attempt,
            },
        )
        .expect("rehash content-different source snapshot");
    assert_corruption_refuses_expiry_and_retry(&store, &fixture, route).await;

    let (store, fixture, route, _) = dispatched_timed_generation_for_fail_closed().await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::InsertOrphanFrozenSourceSibling {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
            },
        )
        .expect("insert orphan public sibling");
    assert_corruption_refuses_expiry_and_retry(&store, &fixture, route).await;

    let (store, fixture, route, _) = dispatched_timed_generation_for_fail_closed().await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::InsertOrphanFrozenPrivateSibling {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
            },
        )
        .expect("insert orphan private sibling");
    assert_corruption_refuses_expiry_and_retry(&store, &fixture, route).await;

    let (store, fixture, route, _) = dispatched_timed_generation_for_fail_closed().await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::SetFirstFrozenSourceOrdinal {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                ordinal: 1,
            },
        )
        .expect("create ordinal gap");
    assert_corruption_refuses_expiry_and_retry(&store, &fixture, route).await;

    let (store, fixture, route, _) = dispatched_timed_generation_for_fail_closed().await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::CorruptFrozenSourceChecksum {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
            },
        )
        .expect("corrupt public snapshot checksum");
    assert_corruption_refuses_expiry_and_retry(&store, &fixture, route).await;

    let (store, fixture, route, _) = dispatched_timed_generation_for_fail_closed().await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::CorruptFrozenPrivateChecksum {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
            },
        )
        .expect("corrupt private execution checksum");
    assert_corruption_refuses_expiry_and_retry(&store, &fixture, route).await;

    let (store, fixture, route, _) = dispatched_timed_generation_for_fail_closed().await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::DropDeliveryFrozenBinding {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: RehearsalIdempotencyKey::new("integrity-gate-continue".into())
                    .expect("key"),
            },
        )
        .expect("corrupt generation material binding");
    assert_corruption_refuses_expiry_and_retry(&store, &fixture, route).await;

    let (store, fixture, route, frozen) = dispatched_timed_generation_for_fail_closed().await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::RemoveFrozenItem {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                attempt: frozen.attempt,
            },
        )
        .expect("corrupt frozen row");
    assert!(
        store
            .verify_rehearsal_archive_for_test(fixture.context.tenant_id(), route.rehearsal)
            .is_err(),
        "frozen corruption is visible to aggregate verification"
    );
    assert_corruption_refuses_expiry_and_retry(&store, &fixture, route).await;

    let (store, fixture, route, _) = dispatched_timed_generation_for_fail_closed().await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::ReplaceEvidenceHeadLength {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                length: 0,
            },
        )
        .expect("corrupt evidence head");
    assert!(
        store
            .verify_rehearsal_archive_for_test(fixture.context.tenant_id(), route.rehearsal)
            .is_err(),
        "evidence corruption is visible to aggregate verification"
    );
    assert_corruption_refuses_expiry_and_retry(&store, &fixture, route).await;

    let (store, fixture, route, frozen) = dispatched_timed_generation_for_fail_closed().await;
    let locator = RehearsalLocator {
        actor: route.actor,
        course: route.course,
        assignment: route.assignment,
        revision: route.expected_revision,
        rehearsal: route.rehearsal,
    };
    complete_submission(&store, &fixture, locator, &frozen, "integrity-gate-claim").await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::DropLatestClaimEvent {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: RehearsalSubmissionIdempotencyKey::new(
                    "integrity-gate-claim".into(),
                )
                .expect("key"),
            },
        )
        .expect("corrupt unrelated claim history");
    assert!(
        store
            .verify_rehearsal_archive_for_test(fixture.context.tenant_id(), route.rehearsal)
            .is_err(),
        "claim corruption is visible to aggregate verification"
    );
    assert_corruption_refuses_expiry_and_retry(&store, &fixture, route).await;

    let (store, fixture, route, _) = dispatched_timed_generation_for_fail_closed().await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::ClearDeliveryGenerations {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: RehearsalIdempotencyKey::new("integrity-gate-continue".into())
                    .expect("key"),
            },
        )
        .expect("corrupt delivery root generations");
    assert_corruption_refuses_expiry_and_retry(&store, &fixture, route).await;

    let (store, fixture, route, _) = dispatched_timed_generation_for_fail_closed().await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::DropDeliveryTimingWitness {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: RehearsalIdempotencyKey::new("integrity-gate-continue".into())
                    .expect("key"),
            },
        )
        .expect("tamper timing witness");
    assert_corruption_refuses_expiry_and_retry(&store, &fixture, route).await;
}

#[tokio::test]
async fn corrupt_delivery_timing_witness_refuses_sealed_execution_without_mutation() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(0))
        .expect("set deterministic server clock");
    let (fixture, route) = start_timed_rehearsal(
        &store,
        question_model::run_policy::TimingPolicy::PerQuestion {
            seconds: 1,
            grace_seconds: 0,
        },
        synthetic_start(),
    )
    .await;
    let prepared =
        prepare_delivery(&store, &fixture, route, "sealed-integrity-continue", 0x74).await;
    let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { dispatched } = store
        .mark_rehearsal_delivery_dispatched(fixture.context, prepared)
        .await
        .expect("dispatch")
    else {
        panic!("dispatch is committed");
    };
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::DropDeliveryTimingWitness {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: RehearsalIdempotencyKey::new("sealed-integrity-continue".into())
                    .expect("key"),
            },
        )
        .expect("tamper witness");
    let ordinary_before = store
        .rehearsal_state_effect_fingerprint()
        .expect("ordinary state baseline");
    assert!(matches!(
        store
            .sealed_private_execution_store()
            .prepare_sealed_rehearsal_delivery_execution(fixture.context, &dispatched)
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("ordinary state remains unchanged"),
        ordinary_before
    );
}
