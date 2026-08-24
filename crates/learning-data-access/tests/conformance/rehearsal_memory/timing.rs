use super::*;

use learning_data_access::{
    ClaimRehearsalDeliveryRouteCommand, ReconcileRehearsalDeliveryExpiryRouteCommand,
    RehearsalDeliveryClaimResult, RehearsalIdempotencyKey, RehearsalOperationDigest,
    RehearsalOperationStore, RehearsalRouteIdentity, RehearsalRouteMutationStore,
    RetryRehearsalDeliveryRouteCommand,
};
use question_model::{ActivityTimestamp, TeachingLimitFieldPatch};

fn timed_synthetic_start() -> RehearsalSubjectStart {
    timed_synthetic_start_with_limit(1)
}

fn timed_synthetic_start_with_limit(seconds: u32) -> RehearsalSubjectStart {
    let mut subject = synthetic_start();
    let RehearsalSubjectStart::Synthetic { request } = &mut subject else {
        panic!("fixture constructs a synthetic subject");
    };
    request.modifiers.mode = question_model::PolicyModificationModeView::Override;
    request.modifiers.patch.time_limit_seconds = TeachingLimitFieldPatch::Set {
        value: question_model::TeachingTimeLimitSeconds::try_from(seconds)
            .expect("positive subject limit"),
    };
    subject
}

pub(super) async fn start_timed_rehearsal(
    store: &MemoryStore,
    timing_policy: question_model::run_policy::TimingPolicy,
    subject: RehearsalSubjectStart,
) -> (
    effective_policy::EffectivePolicyFixture,
    RehearsalRouteIdentity,
) {
    let fixture =
        effective_policy::exercise_effective_policy_gate_and_materialization_contract_with_timing(
            store,
            timing_policy,
        )
        .await;
    let assignment = store
        .assignment_reference(fixture.context, fixture.instructor, fixture.assignment)
        .await
        .expect("assignment lookup")
        .expect("assignment reference");
    let revision = TeachingOperationRevision::new(
        store
            .get_assignment_for_edit(fixture.context, fixture.assignment)
            .await
            .expect("assignment record")
            .expect("assignment")
            .revision
            .value(),
    )
    .expect("revision");
    let receipt = store
        .start_rehearsal(
            fixture.context,
            StartRehearsalCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment,
                revision,
                subject,
                start_new_after_completion: false,
            },
        )
        .await
        .expect("start timed rehearsal");
    let route = RehearsalRouteIdentity {
        actor: fixture.instructor,
        course: fixture.course,
        assignment,
        rehearsal: receipt.rehearsal,
        expected_revision: revision,
    };
    (fixture, route)
}

pub(super) async fn prepare_delivery(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
    route: RehearsalRouteIdentity,
    key: &str,
    fingerprint: u8,
) -> learning_data_access::PreparedRehearsalDelivery {
    let RehearsalDeliveryClaimResult::Prepared { prepared } = store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            ClaimRehearsalDeliveryRouteCommand {
                route,
                idempotency_key: RehearsalIdempotencyKey::new(key.into()).expect("key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([fingerprint; 32]),
            },
        )
        .await
        .expect("prepare delivery")
    else {
        panic!("delivery is prepared");
    };
    prepared
}

pub(super) async fn dispatch_and_complete(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
    prepared: learning_data_access::PreparedRehearsalDelivery,
    _screen_state: &str,
) {
    let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { dispatched } = store
        .mark_rehearsal_delivery_dispatched(fixture.context, prepared)
        .await
        .expect("dispatch")
    else {
        panic!("delivery is dispatchable");
    };
    let screen = commit_issued_screen_for_test(store, fixture.context, &dispatched).await;
    store
        .complete_rehearsal_delivery(
            fixture.context,
            learning_data_access::RehearsalDeliveryCompletionCommand { dispatched, screen },
        )
        .await
        .expect("commit issued screen");
}

#[tokio::test]
async fn per_question_expiry_retries_the_same_ordinary_published_item() {
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
    let prepared = prepare_delivery(&store, &fixture, route, "per-question-continue", 0x61).await;
    let frozen_descriptor = prepared.descriptor().clone();
    dispatch_and_complete(&store, &fixture, prepared, "initial-issued").await;

    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_001))
        .expect("advance server clock past question deadline");
    let expiry = store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand { route },
        )
        .await
        .expect("expire issued question");
    assert_eq!(expiry.verdict, domain::RehearsalTimingVerdictV1::Expired);
    assert_eq!(
        expiry.retry_disposition,
        learning_data_access::RehearsalDeliveryRetryDisposition::Available
    );

    let retry = RetryRehearsalDeliveryRouteCommand {
        route,
        idempotency_key: RehearsalIdempotencyKey::new("per-question-retry".into()).expect("key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([0x63; 32]),
    };
    let learning_data_access::RetryRehearsalDeliveryResult::Prepared { prepared } = store
        .retry_rehearsal_delivery_from_route(fixture.context, retry.clone())
        .await
        .expect("prepare retry")
    else {
        panic!("per-question expiry prepares a successor");
    };
    // The retry repeats the exact ordinary published item binding, including
    // its deterministic seed and immutable source commitment. Private family
    // material remains server-only and is selected by that same binding.
    assert_eq!(prepared.descriptor(), &frozen_descriptor);
    dispatch_and_complete(&store, &fixture, prepared, "retry-issued").await;
    assert!(matches!(
        store
            .retry_rehearsal_delivery_from_route(fixture.context, retry.clone())
            .await
            .expect("retry key replay"),
        learning_data_access::RetryRehearsalDeliveryResult::Replay(_)
    ));
}

#[tokio::test]
async fn corrupted_retry_index_refuses_replay_before_any_mutation() {
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
    let prepared = prepare_delivery(&store, &fixture, route, "retry-index-root", 0x66).await;
    dispatch_and_complete(&store, &fixture, prepared, "initial-issued").await;
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_001))
        .expect("advance past per-question deadline");
    store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand { route },
        )
        .await
        .expect("expire initial generation");
    let retry_key = RehearsalIdempotencyKey::new("retry-index-successor".into()).expect("key");
    let retry = RetryRehearsalDeliveryRouteCommand {
        route,
        idempotency_key: retry_key.clone(),
        request_fingerprint: RehearsalOperationDigest::from_bytes([0x68; 32]),
    };
    assert!(matches!(
        store
            .retry_rehearsal_delivery_from_route(fixture.context, retry.clone())
            .await
            .expect("create successor"),
        learning_data_access::RetryRehearsalDeliveryResult::Prepared { .. }
    ));
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::RedirectDeliveryRetryToPredecessor {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: retry_key,
            },
        )
        .expect("corrupt aggregate retry index");
    let before = store
        .rehearsal_state_effect_fingerprint()
        .expect("effect baseline");
    assert!(matches!(
        store
            .retry_rehearsal_delivery_from_route(fixture.context, retry)
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("no mutation after rejected retry"),
        before
    );
}

#[tokio::test]
async fn expired_retry_successor_conflicts_for_its_original_key_but_new_key_advances() {
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
    let initial = prepare_delivery(&store, &fixture, route, "successor-expiry-root", 0x69).await;
    dispatch_and_complete(&store, &fixture, initial, "initial").await;
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_001))
        .expect("expire initial generation");
    store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand { route },
        )
        .await
        .expect("initial expiry");
    let original_retry = RetryRehearsalDeliveryRouteCommand {
        route,
        idempotency_key: RehearsalIdempotencyKey::new("successor-expiry-retry".into())
            .expect("key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([0x6B; 32]),
    };
    let learning_data_access::RetryRehearsalDeliveryResult::Prepared { prepared } = store
        .retry_rehearsal_delivery_from_route(fixture.context, original_retry.clone())
        .await
        .expect("prepare retry successor")
    else {
        panic!("per-question retry creates successor");
    };
    dispatch_and_complete(&store, &fixture, prepared, "successor").await;
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_002))
        .expect("expire retry successor");
    store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand { route },
        )
        .await
        .expect("successor expiry");
    assert!(matches!(
        store
            .retry_rehearsal_delivery_from_route(fixture.context, original_retry)
            .await
            .expect("original key resolves deterministically"),
        learning_data_access::RetryRehearsalDeliveryResult::Conflict
    ));
    assert!(matches!(
        store
            .retry_rehearsal_delivery_from_route(
                fixture.context,
                RetryRehearsalDeliveryRouteCommand {
                    route,
                    idempotency_key: RehearsalIdempotencyKey::new(
                        "successor-expiry-new-key".into()
                    )
                    .expect("key"),
                    request_fingerprint: RehearsalOperationDigest::from_bytes([0x6D; 32]),
                },
            )
            .await
            .expect("new key advances the next retry cycle"),
        learning_data_access::RetryRehearsalDeliveryResult::Prepared { .. }
    ));
}

#[tokio::test]
async fn dispatched_delivery_expires_and_retries_after_a_pre_screen_crash_window() {
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
    let prepared = prepare_delivery(&store, &fixture, route, "crash-window-continue", 0x6C).await;
    let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { .. } = store
        .mark_rehearsal_delivery_dispatched(fixture.context, prepared)
        .await
        .expect("commit issueDispatched before renderer completion")
    else {
        panic!("initial issue is dispatchable");
    };
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_001))
        .expect("advance server clock past question deadline");
    let expiry = store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand { route },
        )
        .await
        .expect("expiry survives missing renderer completion");
    assert_eq!(expiry.verdict, domain::RehearsalTimingVerdictV1::Expired);
    let retry = RetryRehearsalDeliveryRouteCommand {
        route,
        idempotency_key: RehearsalIdempotencyKey::new("crash-window-retry".into()).expect("key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([0x6D; 32]),
    };
    let learning_data_access::RetryRehearsalDeliveryResult::Prepared { prepared } = store
        .retry_rehearsal_delivery_from_route(fixture.context, retry)
        .await
        .expect("expired dispatched generation can retry")
    else {
        panic!("retry retains the immutable dispatch binding");
    };
    dispatch_and_complete(&store, &fixture, prepared, "recovered-issued").await;
}

#[test]
fn authoritative_memory_clock_refuses_rewind() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_000))
        .expect("advance clock");
    assert!(matches!(
        store.set_authoritative_time(ActivityTimestamp::from_unix_millis(1_999)),
        Err(StoreError::Conflict)
    ));
}

#[tokio::test]
async fn per_attempt_expiry_is_a_terminal_retry_result() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(0))
        .expect("set deterministic server clock");
    let (fixture, route) = start_timed_rehearsal(
        &store,
        question_model::run_policy::TimingPolicy::PerAttempt {
            seconds: 1,
            grace_seconds: 0,
        },
        synthetic_start(),
    )
    .await;
    let prepared = prepare_delivery(&store, &fixture, route, "per-attempt-continue", 0x65).await;
    dispatch_and_complete(&store, &fixture, prepared, "initial-issued").await;
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_001))
        .expect("advance server clock past attempt deadline");
    let expiry = store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand { route },
        )
        .await
        .expect("expire issued attempt");
    assert_eq!(
        expiry.retry_disposition,
        learning_data_access::RehearsalDeliveryRetryDisposition::RunTimeExhausted
    );
    let retry = RetryRehearsalDeliveryRouteCommand {
        route,
        idempotency_key: RehearsalIdempotencyKey::new("per-attempt-retry".into()).expect("key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([0x67; 32]),
    };
    let learning_data_access::RetryRehearsalDeliveryResult::RunTimeExhausted { deadline } = store
        .retry_rehearsal_delivery_from_route(fixture.context, retry.clone())
        .await
        .expect("terminal retry")
    else {
        panic!("attempt expiry has no successor screen");
    };
    assert_eq!(deadline, ActivityTimestamp::from_unix_millis(1_000));
    assert!(matches!(
        store
            .retry_rehearsal_delivery_from_route(fixture.context, retry)
            .await
            .expect("terminal retry replay"),
        learning_data_access::RetryRehearsalDeliveryResult::RunTimeExhausted { .. }
    ));
}

#[tokio::test]
async fn prepared_per_question_retry_becomes_a_replayed_run_cap_terminal() {
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
        timed_synthetic_start_with_limit(2),
    )
    .await;
    let prepared = prepare_delivery(&store, &fixture, route, "cap-race-continue", 0x68).await;
    dispatch_and_complete(&store, &fixture, prepared, "initial-issued").await;
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_001))
        .expect("advance monotonically past question deadline");
    let expiry = store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand { route },
        )
        .await
        .expect("expire first generation");
    assert_eq!(
        expiry.retry_disposition,
        learning_data_access::RehearsalDeliveryRetryDisposition::Available
    );
    let retry = RetryRehearsalDeliveryRouteCommand {
        route,
        idempotency_key: RehearsalIdempotencyKey::new("cap-race-retry".into()).expect("key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([0x6A; 32]),
    };
    let learning_data_access::RetryRehearsalDeliveryResult::Prepared { prepared } = store
        .retry_rehearsal_delivery_from_route(fixture.context, retry.clone())
        .await
        .expect("prepare retry before cap")
    else {
        panic!("question retry is prepared while cap is open");
    };
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(2_001))
        .expect("advance monotonically past run cap");
    let learning_data_access::RehearsalDeliveryDispatchResult::RunTimeExhausted { deadline } =
        store
            .mark_rehearsal_delivery_dispatched(fixture.context, prepared)
            .await
            .expect("dispatch reaches authoritative run cap")
    else {
        panic!("run cap prevents a successor screen");
    };
    assert_eq!(deadline, ActivityTimestamp::from_unix_millis(2_000));
    assert!(matches!(
        store
            .retry_rehearsal_delivery_from_route(fixture.context, retry.clone())
            .await
            .expect("retry terminal replay"),
        learning_data_access::RetryRehearsalDeliveryResult::RunTimeExhausted { .. }
    ));
    assert!(matches!(
        store
            .claim_rehearsal_delivery_from_route(
                fixture.context,
                ClaimRehearsalDeliveryRouteCommand {
                    route,
                    idempotency_key: RehearsalIdempotencyKey::new(
                        "cap-race-different-continue".into()
                    )
                    .expect("key"),
                    request_fingerprint: RehearsalOperationDigest::from_bytes([0x6B; 32]),
                },
            )
            .await
            .expect("different continue key replays terminal"),
        RehearsalDeliveryClaimResult::RunTimeExhausted { .. }
    ));
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::ReplaceDeliveryRunTimeExhaustedDeadline {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                idempotency_key: RehearsalIdempotencyKey::new("cap-race-continue".into())
                    .expect("key"),
                deadline: ActivityTimestamp::from_unix_millis(2_001),
            },
        )
        .expect("tamper committed terminal projection");
    let before = store
        .rehearsal_state_effect_fingerprint()
        .expect("effect baseline");
    assert!(matches!(
        store
            .retry_rehearsal_delivery_from_route(fixture.context, retry)
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(matches!(
        store
            .claim_rehearsal_delivery_from_route(
                fixture.context,
                ClaimRehearsalDeliveryRouteCommand {
                    route,
                    idempotency_key: RehearsalIdempotencyKey::new(
                        "cap-terminal-corrupt-continue".into()
                    )
                    .expect("key"),
                    request_fingerprint: RehearsalOperationDigest::from_bytes([0x6E; 32]),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("no mutation after terminal corruption refusal"),
        before
    );
}

#[tokio::test]
async fn expiry_is_server_owned_idempotent_and_retry_preserves_the_same_item() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(0))
        .expect("set deterministic server clock");
    let fixture =
        effective_policy::exercise_effective_policy_gate_and_materialization_contract(&store).await;
    let assignment = store
        .assignment_reference(fixture.context, fixture.instructor, fixture.assignment)
        .await
        .expect("assignment lookup")
        .expect("assignment reference");
    let revision = TeachingOperationRevision::new(
        store
            .get_assignment_for_edit(fixture.context, fixture.assignment)
            .await
            .expect("assignment record")
            .expect("assignment")
            .revision
            .value(),
    )
    .expect("revision");
    let receipt = store
        .start_rehearsal(
            fixture.context,
            StartRehearsalCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment,
                revision,
                subject: timed_synthetic_start(),
                start_new_after_completion: false,
            },
        )
        .await
        .expect("start timed rehearsal");
    let route = RehearsalRouteIdentity {
        actor: fixture.instructor,
        course: fixture.course,
        assignment,
        rehearsal: receipt.rehearsal,
        expected_revision: revision,
    };
    let claim = store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            ClaimRehearsalDeliveryRouteCommand {
                route,
                idempotency_key: RehearsalIdempotencyKey::new("timed-continue".into())
                    .expect("key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([0x71; 32]),
            },
        )
        .await
        .expect("claim issued item");
    let RehearsalDeliveryClaimResult::Prepared { prepared } = claim else {
        panic!("prepared issue");
    };
    let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { dispatched } = store
        .mark_rehearsal_delivery_dispatched(fixture.context, prepared)
        .await
        .expect("dispatch")
    else {
        panic!("initial issue is dispatchable");
    };
    let screen = commit_issued_screen_for_test(&store, fixture.context, &dispatched).await;
    store
        .complete_rehearsal_delivery(
            fixture.context,
            learning_data_access::RehearsalDeliveryCompletionCommand { dispatched, screen },
        )
        .await
        .expect("commit issued screen");
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("deadline boundary");
    let at_deadline = store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand { route },
        )
        .await
        .expect("timing result");
    assert_eq!(at_deadline.verdict, domain::RehearsalTimingVerdictV1::Open);
    assert_eq!(
        at_deadline.deadline,
        Some(ActivityTimestamp::from_unix_millis(1_000))
    );
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_001))
        .expect("past deadline");
    let expired = store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand { route },
        )
        .await
        .expect("expiry transition");
    assert_eq!(expired.verdict, domain::RehearsalTimingVerdictV1::Expired);
    let replayed_expiry = store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand { route },
        )
        .await
        .expect("idempotent expiry");
    assert_eq!(replayed_expiry, expired);
    assert!(matches!(
        store
            .claim_rehearsal_delivery_from_route(
                fixture.context,
                ClaimRehearsalDeliveryRouteCommand {
                    route,
                    idempotency_key: RehearsalIdempotencyKey::new("later-continue".into())
                        .expect("key"),
                    request_fingerprint: RehearsalOperationDigest::from_bytes([0x73; 32])
                },
            )
            .await
            .expect("continue is blocked"),
        RehearsalDeliveryClaimResult::Expired
    ));
    let retry = RetryRehearsalDeliveryRouteCommand {
        route,
        idempotency_key: RehearsalIdempotencyKey::new("retry-expired-item".into()).expect("key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([0x74; 32]),
    };
    let learning_data_access::RetryRehearsalDeliveryResult::RunTimeExhausted { deadline } = store
        .retry_rehearsal_delivery_from_route(fixture.context, retry.clone())
        .await
        .expect("explicit retry")
    else {
        panic!("subject-wide expiry exhausts the rehearsal run");
    };
    assert_eq!(deadline, ActivityTimestamp::from_unix_millis(1_000));
    assert!(matches!(
        store
            .claim_rehearsal_delivery_from_route(
                fixture.context,
                ClaimRehearsalDeliveryRouteCommand {
                    route,
                    idempotency_key: RehearsalIdempotencyKey::new("hard-terminal-continue".into())
                        .expect("key"),
                    request_fingerprint: RehearsalOperationDigest::from_bytes([0x76; 32])
                },
            )
            .await
            .expect("different key replays hard terminal"),
        RehearsalDeliveryClaimResult::Expired
    ));
    let learning_data_access::RetryRehearsalDeliveryResult::RunTimeExhausted {
        deadline: replayed_deadline,
    } = store
        .retry_rehearsal_delivery_from_route(fixture.context, retry)
        .await
        .expect("retry key replay")
    else {
        panic!("retry replays exhausted run");
    };
    assert_eq!(replayed_deadline, deadline);
}

#[tokio::test]
async fn expiry_reconciliation_skips_accepted_delivery_when_next_item_is_issued() {
    let store = MemoryStore::default();
    let (fixture, locator, first) = start_and_freeze(&store).await;
    let mut second = first.clone();
    second.attempt = question_model::RehearsalAttemptId::from_uuid(uuid::Uuid::from_u128(0xE2));
    store
        .append_rehearsal_frozen_item(
            fixture.context,
            AppendRehearsalFrozenItemCommand {
                locator,
                frozen: second.clone(),
            },
        )
        .await
        .expect("append second frozen item");
    complete_submission(&store, &fixture, locator, &first, "accept-first-item").await;
    let route = RehearsalRouteIdentity {
        actor: locator.actor,
        course: locator.course,
        assignment: locator.assignment,
        rehearsal: locator.rehearsal,
        expected_revision: locator.revision,
    };
    let RehearsalDeliveryClaimResult::Prepared { prepared } = store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            ClaimRehearsalDeliveryRouteCommand {
                route,
                idempotency_key: RehearsalIdempotencyKey::new("issue-second".into()).expect("key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([0xE3; 32]),
            },
        )
        .await
        .expect("select second item")
    else {
        panic!("second item is prepared");
    };
    assert_eq!(prepared.descriptor().attempt(), second.attempt);
    let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { dispatched } = store
        .mark_rehearsal_delivery_dispatched(fixture.context, prepared)
        .await
        .expect("dispatch second item")
    else {
        panic!("second issue is dispatchable");
    };
    let screen = commit_issued_screen_for_test(&store, fixture.context, &dispatched).await;
    store
        .complete_rehearsal_delivery(
            fixture.context,
            learning_data_access::RehearsalDeliveryCompletionCommand { dispatched, screen },
        )
        .await
        .expect("commit second screen");
    let timing = store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            ReconcileRehearsalDeliveryExpiryRouteCommand { route },
        )
        .await
        .expect("accepted first item is ignored");
    assert_eq!(timing.verdict, domain::RehearsalTimingVerdictV1::Open);
}
