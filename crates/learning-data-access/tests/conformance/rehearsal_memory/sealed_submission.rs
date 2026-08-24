//! Sealed submission recovery at the live route boundary.

use super::*;

use learning_data_access::{
    ClaimRehearsalDeliveryRouteCommand, ClaimRehearsalSubmissionRouteCommand,
    CompleteRehearsalDeliveryRouteCommand, ReadRehearsalRouteCommand, RehearsalDeliveryClaimResult,
    RehearsalIdempotencyKey, RehearsalOperationDigest, RehearsalRouteIdentity,
    RehearsalRouteMutationStore, SealedRehearsalSubmissionExecutionPreparation,
    SealedRehearsalSubmissionExecutionStore,
};
use question_model::answer::SelectionCardinality;
use question_model::envelope::ContentBlock;
use question_model::response::{ChoiceId, ChoiceOption};
use question_model::{ResponseDefinition, StudentResponse};

async fn publish_rendered_choice_question(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
) -> question_model::ProblemVersionRef {
    let reference = question_model::ProblemVersionRef {
        problem: question_model::ProblemId::from_uuid(uuid(0x0F41_0001)),
        version: question_model::VersionId::from_uuid(uuid(0x0F41_0002)),
    };
    let mut question = draft_question(question_model::WorkspaceId::from_uuid(uuid(0x0F41_0003)));
    question.response = ResponseDefinition::MultipleChoice {
        choices: vec![
            ChoiceOption {
                id: ChoiceId::new("authored-choice-a"),
                body: vec![ContentBlock::Text {
                    markdown: "First rendered choice".into(),
                }],
            },
            ChoiceOption {
                id: ChoiceId::new("authored-choice-b"),
                body: vec![ContentBlock::Text {
                    markdown: "Second rendered choice".into(),
                }],
            },
        ],
        selection: SelectionCardinality::ExactlyOne,
    };
    let draft = DraftRecord {
        tenant: fixture.context.tenant_id(),
        question,
        derived_from: None,
    };
    let saved = store
        .upsert_draft(fixture.context, fixture.instructor, None, draft.clone())
        .await
        .expect("save rendered-choice draft");
    store
        .publish_draft(
            fixture.context,
            fixture.instructor,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: fixture.instructor,
                scope: question_model::PublicationScope::Public,
                byline: reviewed_byline(),
                capabilities: question_model::BackendCapabilities::from_iter([
                    question_model::Capability::ServerGrading,
                ]),
            },
        )
        .await
        .expect("publish rendered-choice question");
    reference
}

async fn start_rendered_choice_rehearsal(
    store: &MemoryStore,
) -> (
    effective_policy::EffectivePolicyFixture,
    RehearsalRouteIdentity,
) {
    let fixture =
        effective_policy::exercise_effective_policy_gate_and_materialization_contract(store).await;
    let replacement = publish_rendered_choice_question(store, &fixture).await;
    let current = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("read assignment")
        .expect("assignment exists");
    let revised = store
        .replace_assignment_fixed_item(
            fixture.context,
            learning_data_access::ReplaceAssignmentFixedItemCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: fixture.assignment,
                current_item: current.record.items[0].id,
                expected_revision: current.revision,
                replacement,
            },
        )
        .await
        .expect("replace ordinary assignment item");
    let assignment = store
        .assignment_reference(fixture.context, fixture.instructor, fixture.assignment)
        .await
        .expect("assignment reference lookup")
        .expect("assignment reference");
    let started = store
        .start_rehearsal_from_route(
            fixture.context,
            learning_data_access::StartRehearsalRouteCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment,
                expected_revision: TeachingOperationRevision::new(revised.revision.value())
                    .expect("revised teaching operation"),
                subject: synthetic_start(),
                start_new_after_completion: false,
                idempotency_key: RehearsalIdempotencyKey::new("rendered-choice-start".into())
                    .expect("start key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([0xC1; 32]),
            },
        )
        .await
        .expect("start rendered-choice rehearsal");
    let route = RehearsalRouteIdentity {
        actor: fixture.instructor,
        course: fixture.course,
        assignment,
        rehearsal: started.receipt.rehearsal,
        expected_revision: TeachingOperationRevision::new(revised.revision.value())
            .expect("revised teaching operation"),
    };
    (fixture, route)
}

fn route_for(
    fixture: &effective_policy::EffectivePolicyFixture,
    locator: RehearsalLocator,
) -> RehearsalRouteIdentity {
    RehearsalRouteIdentity {
        actor: fixture.instructor,
        course: fixture.course,
        assignment: locator.assignment,
        rehearsal: locator.rehearsal,
        expected_revision: locator.revision,
    }
}

async fn issue_live_screen(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
    route: RehearsalRouteIdentity,
    delivery_key: &str,
    fingerprint: u8,
) -> question_model::PresentationDigestTokenV1 {
    let delivery_key = RehearsalIdempotencyKey::new(delivery_key.into()).expect("delivery key");
    let RehearsalDeliveryClaimResult::Prepared { prepared } = store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            ClaimRehearsalDeliveryRouteCommand {
                route,
                idempotency_key: delivery_key,
                request_fingerprint: RehearsalOperationDigest::from_bytes([fingerprint; 32]),
            },
        )
        .await
        .expect("prepare live delivery")
    else {
        panic!("live delivery must be prepared");
    };
    dispatch_live_screen(store, fixture, route, prepared).await
}

async fn dispatch_live_screen(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
    route: RehearsalRouteIdentity,
    prepared: learning_data_access::PreparedRehearsalDelivery,
) -> question_model::PresentationDigestTokenV1 {
    let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { dispatched } = store
        .mark_rehearsal_delivery_dispatched_from_route(fixture.context, route, prepared)
        .await
        .expect("dispatch live delivery")
    else {
        panic!("live delivery must dispatch");
    };
    let screen = commit_issued_screen_for_test(store, fixture.context, &dispatched).await;
    let presentation_digest = screen
        .commitment()
        .expect("committed screen has a presentation digest")
        .public_token();
    store
        .complete_rehearsal_delivery_from_route(
            fixture.context,
            CompleteRehearsalDeliveryRouteCommand {
                route,
                dispatched,
                screen,
            },
        )
        .await
        .expect("complete live delivery");
    presentation_digest
}

#[tokio::test]
async fn prepared_submission_retry_is_status_only_and_route_keyed_dispatch_recovers_work() {
    let store = MemoryStore::default();
    let (fixture, locator, _frozen) = start_and_freeze(&store).await;
    let route = route_for(&fixture, locator);
    let presentation_digest =
        issue_live_screen(&store, &fixture, route, "sealed-submission-delivery", 0x41).await;
    let key = RehearsalIdempotencyKey::new("sealed-submission".into()).expect("submission key");
    let command = ClaimRehearsalSubmissionRouteCommand {
        route,
        response: StudentResponse::Numeric { value: 3.0 },
        presentation_digest,
        idempotency_key: key.clone(),
    };

    let first = store
        .claim_rehearsal_submission_from_route(fixture.context, command.clone())
        .await
        .expect("create route claim");
    assert!(matches!(
        first,
        learning_data_access::RehearsalSubmissionClaimResult::Claimed(_)
    ));

    // The browser-visible retry can report only status. A process crash after
    // the Prepared transition never turns the public retry into grading work.
    let retry = store
        .claim_rehearsal_submission_from_route(fixture.context, command)
        .await
        .expect("public retry status");
    assert!(matches!(
        retry,
        learning_data_access::RehearsalSubmissionClaimResult::Pending
    ));

    let first_dispatched = store
        .dispatch_rehearsal_submission_from_route(fixture.context, route, key.clone())
        .await
        .expect("route-keyed recovery dispatch");
    let first_claim = first_dispatched.claim();
    let first_operation = first_dispatched.operation();
    let first_generation = first_dispatched.generation();

    let sealed = store.sealed_private_execution_store();
    assert!(matches!(
        sealed
            .prepare_or_resume_sealed_rehearsal_submission_execution(
                fixture.context,
                route,
                key.clone(),
            )
            .await
            .expect("sealed recovery work"),
        SealedRehearsalSubmissionExecutionPreparation::Work(_)
    ));

    // Exact retries, including concurrent retries, are idempotent and return
    // the same opaque claim identity rather than minting another operation.
    let (left, right) = tokio::join!(
        store.dispatch_rehearsal_submission_from_route(fixture.context, route, key.clone()),
        store.dispatch_rehearsal_submission_from_route(fixture.context, route, key.clone()),
    );
    for recovered in [
        left.expect("concurrent recovery A"),
        right.expect("concurrent recovery B"),
    ] {
        assert_eq!(recovered.claim(), first_claim);
        assert_eq!(recovered.operation(), first_operation);
        assert_eq!(recovered.generation(), first_generation);
    }
    let repeated = store
        .dispatch_rehearsal_submission_from_route(fixture.context, route, key)
        .await
        .expect("repeated recovery");
    assert_eq!(repeated.claim(), first_claim);
    assert_eq!(repeated.operation(), first_operation);
    assert_eq!(repeated.generation(), first_generation);
}

#[tokio::test]
async fn rendered_choice_submission_reloads_translates_completes_and_replays_without_learner_work()
{
    let store = MemoryStore::default();
    let (fixture, route) = start_rendered_choice_rehearsal(&store).await;
    let ordinary_before = store
        .rehearsal_state_effect_fingerprint()
        .expect("ordinary-state baseline");
    let delivery_key =
        RehearsalIdempotencyKey::new("rendered-choice-delivery".into()).expect("delivery key");
    let RehearsalDeliveryClaimResult::Prepared { prepared } = store
        .claim_rehearsal_delivery_from_route(
            fixture.context,
            ClaimRehearsalDeliveryRouteCommand {
                route,
                idempotency_key: delivery_key,
                request_fingerprint: RehearsalOperationDigest::from_bytes([0xC2; 32]),
            },
        )
        .await
        .expect("prepare rendered-choice delivery")
    else {
        panic!("rendered-choice delivery must prepare");
    };
    let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { dispatched } = store
        .mark_rehearsal_delivery_dispatched_from_route(fixture.context, route, prepared)
        .await
        .expect("dispatch rendered-choice delivery")
    else {
        panic!("rendered-choice delivery must dispatch");
    };
    let screen = commit_issued_screen_for_test(&store, fixture.context, &dispatched).await;
    let presentation_digest = screen
        .commitment()
        .expect("rendered-choice screen commitment")
        .public_token();
    store
        .complete_rehearsal_delivery_from_route(
            fixture.context,
            CompleteRehearsalDeliveryRouteCommand {
                route,
                dispatched,
                screen: screen.clone(),
            },
        )
        .await
        .expect("complete rendered-choice delivery");
    let question_model::RehearsalResponseSchemaV1::SingleChoice { choices } =
        &screen.presentation.response
    else {
        panic!("fixture must expose rendered single-choice IDs");
    };
    let rendered_choice = ChoiceId::new(choices[0].id.as_str());
    let key =
        RehearsalIdempotencyKey::new("rendered-choice-submission".into()).expect("submission key");
    let command = ClaimRehearsalSubmissionRouteCommand {
        route,
        response: StudentResponse::MultipleChoice {
            selected: vec![rendered_choice.clone()],
        },
        presentation_digest: presentation_digest.clone(),
        idempotency_key: key.clone(),
    };
    assert!(matches!(
        store
            .claim_rehearsal_submission_from_route(fixture.context, command.clone())
            .await
            .expect("admit rendered choice"),
        learning_data_access::RehearsalSubmissionClaimResult::Claimed(_)
    ));

    // A reload has only canonical tagged persistence to work from; it must
    // rehydrate through the committed screen rather than a retained mapping.
    store
        .read_rehearsal_from_route(
            fixture.context,
            ReadRehearsalRouteCommand {
                actor: route.actor,
                course: route.course,
                assignment: route.assignment,
                rehearsal: route.rehearsal,
            },
        )
        .await
        .expect("reload rendered claim from canonical tagged input");
    let before_conflict = store
        .rehearsal_state_effect_fingerprint()
        .expect("pre-conflict fingerprint");
    let changed = ClaimRehearsalSubmissionRouteCommand {
        response: StudentResponse::MultipleChoice {
            selected: vec![ChoiceId::new(choices[1].id.as_str())],
        },
        ..command.clone()
    };
    assert!(matches!(
        store
            .claim_rehearsal_submission_from_route(fixture.context, changed)
            .await
            .expect("changed rendered response has a closed claim result"),
        learning_data_access::RehearsalSubmissionClaimResult::Conflict
    ));
    assert_eq!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("post-conflict fingerprint"),
        before_conflict,
        "changed rendered response leaves no partial claim mutation"
    );

    let _handle = store
        .dispatch_rehearsal_submission_from_route(fixture.context, route, key.clone())
        .await
        .expect("dispatch rendered claim after reload");
    let sealed = store.sealed_private_execution_store();
    let SealedRehearsalSubmissionExecutionPreparation::Work(work) = sealed
        .prepare_or_resume_sealed_rehearsal_submission_execution(
            fixture.context,
            route,
            key.clone(),
        )
        .await
        .expect("prepare sealed rendered work")
    else {
        panic!("dispatched rendered claim must provide sealed grading work");
    };
    let (grading, completion) = work.into_grading_and_completion();
    let StudentResponse::MultipleChoice { selected } = grading.response() else {
        panic!("artifact translation must preserve the response family");
    };
    assert_eq!(selected, &[ChoiceId::new("authored-choice-a")]);
    assert!(
        completion
            .backend_receipt_reference()
            .expect("store-minted backend receipt reference")
            .as_str()
            .starts_with("rehearsal-grade-v1:"),
        "the coordinator receives only a bounded Store-minted grader correlation value"
    );
    let before_wrong_context = store
        .rehearsal_state_effect_fingerprint()
        .expect("sealed completion baseline");
    assert!(matches!(
        sealed
            .complete_sealed_rehearsal_submission_execution(
                TenantContext::from_authenticated_session(question_model::TenantId::from_uuid(
                    uuid(0x0F41_0004),
                )),
                completion,
                deterministic_grade(),
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("wrong-context completion fingerprint"),
        before_wrong_context,
        "a consumed mismatched completion capability leaves no mutation"
    );
    let SealedRehearsalSubmissionExecutionPreparation::Work(work) = sealed
        .prepare_or_resume_sealed_rehearsal_submission_execution(
            fixture.context,
            route,
            key.clone(),
        )
        .await
        .expect("crash-style reprepare after rejected capability")
    else {
        panic!("dispatched claim must mint a fresh completion capability");
    };
    let (_grading, completion) = work.into_grading_and_completion();
    let receipt = sealed
        .complete_sealed_rehearsal_submission_execution(
            fixture.context,
            completion,
            deterministic_grade(),
        )
        .await
        .expect("complete rendered submission through normal route");
    let before_replay = store
        .rehearsal_state_effect_fingerprint()
        .expect("completed replay baseline");
    let replay = store
        .claim_rehearsal_submission_from_route(fixture.context, command)
        .await
        .expect("exact rendered replay");
    let learning_data_access::RehearsalSubmissionClaimResult::Replay(replayed_receipt) = replay
    else {
        panic!("exact rendered replay must return the original receipt");
    };
    assert_eq!(replayed_receipt.outcome, receipt.outcome);
    assert!(replayed_receipt.replayed);
    assert_eq!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("post-replay fingerprint"),
        before_replay,
        "receipt replay appends no duplicate claim event or accepted evidence"
    );
    store
        .read_rehearsal_from_route(
            fixture.context,
            ReadRehearsalRouteCommand {
                actor: route.actor,
                course: route.course,
                assignment: route.assignment,
                rehearsal: route.rehearsal,
            },
        )
        .await
        .expect("read completed rendered rehearsal");
    let ordinary_after = store
        .rehearsal_state_effect_fingerprint()
        .expect("ordinary-state result");
    assert!(ordinary_after.has_no_ordinary_effects_from(&ordinary_before));
}

#[cfg(feature = "test-support")]
async fn start_timed_route(
    store: &MemoryStore,
) -> (
    effective_policy::EffectivePolicyFixture,
    RehearsalRouteIdentity,
) {
    let fixture =
        effective_policy::exercise_effective_policy_gate_and_materialization_contract_with_timing(
            store,
            question_model::run_policy::TimingPolicy::PerQuestion {
                seconds: 1,
                grace_seconds: 0,
            },
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
    .expect("assignment revision");
    let receipt = store
        .start_rehearsal(
            fixture.context,
            StartRehearsalCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment,
                revision,
                subject: synthetic_start(),
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

#[cfg(feature = "test-support")]
#[tokio::test]
async fn cross_generation_route_binding_substitution_fails_before_any_mutation() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(question_model::ActivityTimestamp::from_unix_millis(0))
        .expect("set deterministic clock");
    let (fixture, route) = start_timed_route(&store).await;
    let _initial_digest = issue_live_screen(
        &store,
        &fixture,
        route,
        "binding-substitution-initial",
        0x71,
    )
    .await;

    store
        .set_authoritative_time(question_model::ActivityTimestamp::from_unix_millis(1_001))
        .expect("advance deterministic clock");
    store
        .reconcile_rehearsal_delivery_expiry_from_route(
            fixture.context,
            learning_data_access::ReconcileRehearsalDeliveryExpiryRouteCommand { route },
        )
        .await
        .expect("expire initial generation");
    let retry_key =
        RehearsalIdempotencyKey::new("binding-substitution-retry".into()).expect("retry key");
    let retry = store
        .retry_rehearsal_delivery_from_route(
            fixture.context,
            learning_data_access::RetryRehearsalDeliveryRouteCommand {
                route,
                idempotency_key: retry_key.clone(),
                request_fingerprint: RehearsalOperationDigest::from_bytes([0x72; 32]),
            },
        )
        .await
        .expect("prepare same-attempt retry");
    let prepared = match retry {
        learning_data_access::RetryRehearsalDeliveryResult::Prepared { prepared } => prepared,
        learning_data_access::RetryRehearsalDeliveryResult::RunTimeExhausted { .. } => {
            panic!("retry became run-time exhausted")
        }
        learning_data_access::RetryRehearsalDeliveryResult::Conflict => {
            panic!("retry conflicted")
        }
        learning_data_access::RetryRehearsalDeliveryResult::Pending { .. } => {
            panic!("retry unexpectedly pending")
        }
        learning_data_access::RetryRehearsalDeliveryResult::Replay(_) => {
            panic!("retry unexpectedly replayed")
        }
    };
    let retry_digest = dispatch_live_screen(&store, &fixture, route, prepared).await;

    let claim_key =
        RehearsalIdempotencyKey::new("binding-substitution-claim".into()).expect("claim key");
    let claim = store
        .claim_rehearsal_submission_from_route(
            fixture.context,
            ClaimRehearsalSubmissionRouteCommand {
                route,
                response: StudentResponse::Numeric { value: 3.0 },
                presentation_digest: retry_digest,
                idempotency_key: claim_key.clone(),
            },
        )
        .await
        .expect("claim retry issued generation");
    assert!(matches!(
        claim,
        learning_data_access::RehearsalSubmissionClaimResult::Claimed(_)
    ));

    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::SubstituteRouteClaimDeliveryWithIssuedGeneration {
                tenant: fixture.context.tenant_id(),
                rehearsal: route.rehearsal,
                claim_idempotency_key: claim_key.clone(),
                replacement_delivery_idempotency_key: RehearsalIdempotencyKey::new(
                    "binding-substitution-initial".into(),
                )
                .expect("initial delivery key"),
            },
        )
        .expect("substitute only the disposable persisted binding");
    let before = store
        .rehearsal_state_effect_fingerprint()
        .expect("corrupted aggregate fingerprint");
    assert!(
        store
            .read_rehearsal_from_route(
                fixture.context,
                ReadRehearsalRouteCommand {
                    actor: route.actor,
                    course: route.course,
                    assignment: route.assignment,
                    rehearsal: route.rehearsal,
                },
            )
            .await
            .is_err(),
        "aggregate-bound route read must reject substitution"
    );
    assert!(
        store
            .sealed_private_execution_store()
            .prepare_or_resume_sealed_rehearsal_submission_execution(
                fixture.context,
                route,
                claim_key.clone(),
            )
            .await
            .is_err(),
        "sealed preparation must reject substitution"
    );
    assert!(
        store
            .dispatch_rehearsal_submission_from_route(fixture.context, route, claim_key)
            .await
            .is_err(),
        "route mutation must reject substitution"
    );
    assert_eq!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("post-rejection fingerprint"),
        before,
        "integrity rejection must not mutate aggregate state"
    );
}
