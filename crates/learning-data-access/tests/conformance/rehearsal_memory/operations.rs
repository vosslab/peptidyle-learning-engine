use super::*;

use learning_data_access::RehearsalIssuedExecutionTestTampering;
use learning_data_access::{
    RehearsalDeliveryClaimResult, RehearsalDeliveryCompletionCommand,
    RehearsalDeliveryPreDispatchAbandonReason, RehearsalDeliveryPreDispatchCompensationStore,
    RehearsalDeliveryRequest, RehearsalIdempotencyKey, RehearsalOperationDigest,
    RehearsalOperationStore, RehearsalRouteMutationStore, SealedRehearsalDeliveryExecutionStore,
};
use question_model::presentation::build_presentation_v1;
use question_model::{
    AttemptProvenance, ImplementationVersion, PresentationBindingV1, QuestionEnvelope,
    generation::Seed,
};

fn issued_artifact(
    work: &learning_data_access::SealedRehearsalDeliveryIssueWork,
) -> learning_data_access::RehearsalIssuedExecutionArtifactV1 {
    let question = work.issued_snapshot().question();
    let envelope = QuestionEnvelope {
        version: question.version,
        seed: Seed::new(work.descriptor().deterministic_seed()),
        title: question.metadata.title.clone(),
        prompt: question.prompt.clone(),
        response: question.response.clone(),
    };
    let rendered_question_sha256 =
        objects::Sha256Digest::compute(&serde_json::to_vec(&envelope).expect("envelope bytes"))
            .to_string();
    let presentation = build_presentation_v1(&envelope, &[]).expect("presentation");
    learning_data_access::RehearsalIssuedExecutionArtifactV1::from_issue_work(
        work,
        envelope,
        work.descriptor().frozen_content_digest().to_hex(),
        AttemptProvenance {
            adapter: ImplementationVersion {
                id: "memory-native".into(),
                version: "1".into(),
            },
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: ImplementationVersion {
                id: "memory-grader".into(),
                version: "1".into(),
            },
            rendered_question_sha256,
        },
        PresentationBindingV1::new(
            presentation.envelope.presentation_nonce,
            presentation.digest,
        ),
        learning_data_access::ReceiptPresentationSnapshot {
            envelope: presentation.envelope,
            asset_bindings: presentation.asset_bindings,
        },
    )
    .expect("valid sealed issued artifact")
}

#[tokio::test]
async fn issued_artifact_construction_rejects_tampered_generated_and_presentation_fields() {
    let store = MemoryStore::default();
    let (fixture, locator, _) = start_and_freeze(&store).await;
    let RehearsalDeliveryClaimResult::Prepared { prepared } = store
        .claim_rehearsal_delivery(
            fixture.context,
            RehearsalDeliveryRequest {
                locator,
                idempotency_key: RehearsalIdempotencyKey::new("artifact-validation".into())
                    .expect("key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([0x41; 32]),
            },
        )
        .await
        .expect("claim")
    else {
        panic!("prepared delivery")
    };
    let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { dispatched } = store
        .mark_rehearsal_delivery_dispatched(fixture.context, prepared)
        .await
        .expect("dispatch")
    else {
        panic!("dispatch")
    };
    let sealed = store.sealed_private_execution_store();
    let learning_data_access::SealedRehearsalDeliveryIssuePreparation::IssueWork(work) = sealed
        .prepare_or_resume_issued_execution(fixture.context, &dispatched)
        .await
        .expect("issue work")
    else {
        panic!("new generation needs issue work")
    };

    // The public constructor is the only test-reachable persistence boundary.
    // Each malformed sibling is rejected before a caller can commit anything.
    let question = work.issued_snapshot().question();
    let envelope = QuestionEnvelope {
        version: question.version,
        seed: Seed::new(work.descriptor().deterministic_seed()),
        title: question.metadata.title.clone(),
        prompt: question.prompt.clone(),
        response: question.response.clone(),
    };
    let rendered_question_sha256 =
        objects::Sha256Digest::compute(&serde_json::to_vec(&envelope).expect("envelope bytes"))
            .to_string();
    let presentation = build_presentation_v1(&envelope, &[]).expect("presentation");
    let provenance = AttemptProvenance {
        adapter: ImplementationVersion {
            id: "memory-native".into(),
            version: "1".into(),
        },
        renderer: None,
        generator: None,
        source_artifact: None,
        asset_objects: Vec::new(),
        grading: ImplementationVersion {
            id: "memory-grader".into(),
            version: "1".into(),
        },
        rendered_question_sha256: rendered_question_sha256.clone(),
    };
    let snapshot = learning_data_access::ReceiptPresentationSnapshot {
        envelope: presentation.envelope.clone(),
        asset_bindings: presentation.asset_bindings.clone(),
    };
    let binding = PresentationBindingV1::new(
        presentation.envelope.presentation_nonce,
        presentation.digest,
    );
    let cases = [
        (
            "parameter hash",
            "parameter hash is not a lowercase SHA-256",
            {
                let mut value = provenance.clone();
                value.rendered_question_sha256 = rendered_question_sha256.clone();
                ("not-a-digest".to_owned(), value, binding, snapshot.clone())
            },
        ),
        ("provenance", "provenance does not bind the envelope", {
            let mut value = provenance.clone();
            value.rendered_question_sha256 = "00".repeat(32);
            (
                work.descriptor().frozen_content_digest().to_hex(),
                value,
                binding,
                snapshot.clone(),
            )
        }),
        (
            "presentation binding",
            "binding does not reproduce the presentation",
            {
                (
                    work.descriptor().frozen_content_digest().to_hex(),
                    provenance.clone(),
                    PresentationBindingV1::new(
                        binding.nonce(),
                        question_model::presentation::PresentationDigestV1::from_bytes([0xA5; 32]),
                    ),
                    snapshot.clone(),
                )
            },
        ),
    ];
    for (
        name,
        message,
        (parameter_hash, tampered_provenance, tampered_binding, tampered_snapshot),
    ) in cases
    {
        let result = learning_data_access::RehearsalIssuedExecutionArtifactV1::from_issue_work(
            &work,
            envelope.clone(),
            parameter_hash,
            tampered_provenance,
            tampered_binding,
            tampered_snapshot,
        );
        assert!(result.is_err(), "{name}: {message}");
    }

    // Validation above consumes no Store capability; the generation remains
    // issuable and can still accept the one valid artifact.
    let artifact = issued_artifact(&work);
    sealed
        .commit_issued_execution(fixture.context, *work, artifact)
        .await
        .expect("valid artifact remains committable after rejected siblings");
}

#[tokio::test]
async fn sealed_issued_artifact_commits_once_and_replays_after_a_pre_screen_crash() {
    let store = MemoryStore::default();
    let (fixture, locator, _) = start_and_freeze(&store).await;
    let RehearsalDeliveryClaimResult::Prepared { prepared } = store
        .claim_rehearsal_delivery(
            fixture.context,
            RehearsalDeliveryRequest {
                locator,
                idempotency_key: RehearsalIdempotencyKey::new("issued-artifact".into())
                    .expect("key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([0x76; 32]),
            },
        )
        .await
        .expect("claim")
    else {
        panic!("prepared delivery")
    };
    let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { dispatched } = store
        .mark_rehearsal_delivery_dispatched(fixture.context, prepared)
        .await
        .expect("dispatch")
    else {
        panic!("dispatch")
    };
    let sealed = store.sealed_private_execution_store();
    let learning_data_access::SealedRehearsalDeliveryIssuePreparation::IssueWork(work) = sealed
        .prepare_or_resume_issued_execution(fixture.context, &dispatched)
        .await
        .expect("issue work")
    else {
        panic!("new generation needs issue work")
    };
    let artifact = issued_artifact(&work);
    let committed = sealed
        .commit_issued_execution(fixture.context, *work, artifact)
        .await
        .expect("commit");
    assert!(committed.has_committed_artifact());
    let learning_data_access::SealedRehearsalDeliveryIssuePreparation::ExistingArtifact(replay) =
        sealed
            .prepare_or_resume_issued_execution(fixture.context, &dispatched)
            .await
            .expect("crash recovery")
    else {
        panic!("exact artifact replay")
    };
    assert!(replay.has_committed_artifact());
    assert_eq!(committed.issued_snapshot(), replay.issued_snapshot());
}

#[tokio::test]
async fn canonical_tampered_issued_artifact_fails_closed_without_state_mutation() {
    for (name, tampering) in [
        (
            "operation binding",
            RehearsalIssuedExecutionTestTampering::OperationBinding,
        ),
        (
            "generation binding",
            RehearsalIssuedExecutionTestTampering::GenerationBinding,
        ),
        (
            "presentation",
            RehearsalIssuedExecutionTestTampering::Presentation,
        ),
        (
            "provenance",
            RehearsalIssuedExecutionTestTampering::Provenance,
        ),
        ("envelope", RehearsalIssuedExecutionTestTampering::Envelope),
    ] {
        let store = MemoryStore::default();
        let (fixture, locator, _frozen) = start_and_freeze(&store).await;
        let idempotency_key =
            RehearsalIdempotencyKey::new(format!("artifact-corruption-{name}")).expect("key");
        let RehearsalDeliveryClaimResult::Prepared { prepared } = store
            .claim_rehearsal_delivery(
                fixture.context,
                RehearsalDeliveryRequest {
                    locator,
                    idempotency_key: idempotency_key.clone(),
                    request_fingerprint: RehearsalOperationDigest::from_bytes([0x51; 32]),
                },
            )
            .await
            .expect("delivery claim")
        else {
            panic!("fresh delivery must prepare");
        };
        let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { dispatched } =
            store
                .mark_rehearsal_delivery_dispatched(fixture.context, prepared)
                .await
                .expect("dispatch")
        else {
            panic!("fresh delivery must dispatch");
        };
        let screen =
            super::commit_issued_screen_for_test(&store, fixture.context, &dispatched).await;
        let presentation_digest = screen.presentation_digest.clone();
        store
            .corrupt_rehearsal_integrity_for_test(
                learning_data_access::in_memory::MemoryRehearsalIntegrityTestCorruption::TamperIssuedExecutionArtifact {
                    tenant: fixture.context.tenant_id(),
                    rehearsal: locator.rehearsal,
                    idempotency_key,
                    tampering,
                },
            )
            .expect("replace committed artifact with canonical tampered bytes");
        let before = store
            .rehearsal_state_effect_fingerprint()
            .expect("corrupted baseline fingerprint");
        assert!(
            store
                .read_rehearsal_from_route(
                    fixture.context,
                    learning_data_access::ReadRehearsalRouteCommand {
                        actor: locator.actor,
                        course: locator.course,
                        assignment: locator.assignment,
                        rehearsal: locator.rehearsal,
                    },
                )
                .await
                .is_err(),
            "{name}: aggregate route read must fail closed"
        );
        assert!(
            store
                .sealed_private_execution_store()
                .prepare_sealed_rehearsal_delivery_execution(fixture.context, &dispatched)
                .await
                .is_err(),
            "{name}: sealed execution preparation must fail closed"
        );
        assert!(
            store
                .complete_rehearsal_delivery(
                    fixture.context,
                    RehearsalDeliveryCompletionCommand { dispatched, screen },
                )
                .await
                .is_err(),
            "{name}: screen completion must fail closed"
        );
        assert!(
            store
                .claim_rehearsal_submission_from_route(
                    fixture.context,
                    learning_data_access::ClaimRehearsalSubmissionRouteCommand {
                        route: learning_data_access::RehearsalRouteIdentity {
                            actor: locator.actor,
                            course: locator.course,
                            assignment: locator.assignment,
                            rehearsal: locator.rehearsal,
                            expected_revision: locator.revision,
                        },
                        response: StudentResponse::Numeric { value: 3.0 },
                        presentation_digest,
                        idempotency_key: RehearsalIdempotencyKey::new(format!(
                            "artifact-submission-{name}"
                        ),)
                        .expect("submission key"),
                    },
                )
                .await
                .is_err(),
            "{name}: submission claim must fail closed"
        );
        assert_eq!(
            before,
            store
                .rehearsal_state_effect_fingerprint()
                .expect("after failed operations fingerprint")
        );
    }
}

#[tokio::test]
async fn delivery_operation_replays_and_requires_committed_dispatch_before_completion() {
    let store = MemoryStore::default();
    let (fixture, locator, frozen) = start_and_freeze(&store).await;
    let request = RehearsalDeliveryRequest {
        locator,
        idempotency_key: RehearsalIdempotencyKey::new("delivery-1".into()).expect("key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([3; 32]),
    };
    let claimed = store
        .claim_rehearsal_delivery(fixture.context, request)
        .await
        .expect("claim");
    let RehearsalDeliveryClaimResult::Prepared { prepared, .. } = claimed else {
        panic!("fresh delivery must claim");
    };
    let replay = store
        .claim_rehearsal_delivery(
            fixture.context,
            RehearsalDeliveryRequest {
                locator,
                idempotency_key: RehearsalIdempotencyKey::new("delivery-1".into()).expect("key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([3; 32]),
            },
        )
        .await
        .expect("prepared recovery");
    assert!(matches!(
        replay,
        RehearsalDeliveryClaimResult::Prepared { .. }
    ));
    let conflict = store
        .claim_rehearsal_delivery(
            fixture.context,
            RehearsalDeliveryRequest {
                locator,
                idempotency_key: RehearsalIdempotencyKey::new("delivery-1".into()).expect("key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([4; 32]),
            },
        )
        .await
        .expect("conflict");
    assert!(matches!(conflict, RehearsalDeliveryClaimResult::Conflict));
    let learning_data_access::RehearsalDeliveryDispatchResult::Dispatched { dispatched } = store
        .mark_rehearsal_delivery_dispatched(fixture.context, prepared)
        .await
        .expect("dispatch")
    else {
        panic!("delivery is dispatchable");
    };
    let sealed_store = store.sealed_private_execution_store();
    let learning_data_access::SealedRehearsalDeliveryIssuePreparation::IssueWork(work) =
        sealed_store
            .prepare_or_resume_issued_execution(fixture.context, &dispatched)
            .await
            .expect("sealed issuance work")
    else {
        panic!("new dispatched generation requires issue work")
    };
    let artifact = issued_artifact(&work);
    let _ = sealed_store
        .commit_issued_execution(fixture.context, *work, artifact)
        .await
        .expect("commit exact issued artifact");
    let sealed = sealed_store
        .prepare_sealed_rehearsal_delivery_execution(fixture.context, &dispatched)
        .await
        .expect("grader-only facade reads the committed issued execution");
    assert_eq!(
        sealed.issued_snapshot().question().problem,
        frozen.problem.problem
    );
    assert_eq!(
        sealed.issued_snapshot().question().version,
        frozen.problem.version
    );
    store
        .complete_rehearsal_delivery(
            fixture.context,
            RehearsalDeliveryCompletionCommand {
                dispatched,
                screen: sealed.active_screen().expect("issued artifact screen"),
            },
        )
        .await
        .expect("complete");
    let final_replay = store
        .claim_rehearsal_delivery(
            fixture.context,
            RehearsalDeliveryRequest {
                locator,
                idempotency_key: RehearsalIdempotencyKey::new("delivery-1".into()).expect("key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([3; 32]),
            },
        )
        .await
        .expect("final replay");
    assert!(matches!(
        final_replay,
        RehearsalDeliveryClaimResult::Replay(_)
    ));
}

#[tokio::test]
async fn same_key_reclaims_an_abandoned_generation_without_replanning() {
    let store = MemoryStore::default();
    let (fixture, locator, _frozen) = start_and_freeze(&store).await;
    let request = RehearsalDeliveryRequest {
        locator,
        idempotency_key: RehearsalIdempotencyKey::new("reclaim-1".into()).expect("key"),
        request_fingerprint: RehearsalOperationDigest::from_bytes([8; 32]),
    };
    let RehearsalDeliveryClaimResult::Prepared { prepared } = store
        .claim_rehearsal_delivery(fixture.context, request.clone())
        .await
        .expect("first prepared generation")
    else {
        panic!("fresh delivery must prepare");
    };
    store
        .abandon_rehearsal_delivery_before_dispatch(
            fixture.context,
            prepared,
            RehearsalDeliveryPreDispatchAbandonReason::LocalPreparationFailed,
        )
        .await
        .expect("definite pre-dispatch abandonment");
    let RehearsalDeliveryClaimResult::Prepared { .. } = store
        .claim_rehearsal_delivery(fixture.context, request)
        .await
        .expect("same key reclaims a fresh generation")
    else {
        panic!("reclaimed delivery must be prepared");
    };
}

#[tokio::test]
async fn distinct_continue_keys_resume_one_store_owned_issue_cycle() {
    let store = MemoryStore::default();
    let (fixture, locator, _) = start_and_freeze(&store).await;
    let first = store
        .claim_rehearsal_delivery(
            fixture.context,
            RehearsalDeliveryRequest {
                locator,
                idempotency_key: RehearsalIdempotencyKey::new("continue-a".into()).expect("key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([9; 32]),
            },
        )
        .await
        .expect("first claim");
    let RehearsalDeliveryClaimResult::Prepared { prepared: first } = first else {
        panic!("first Continue mints a prepared delivery");
    };
    let resumed = store
        .claim_rehearsal_delivery(
            fixture.context,
            RehearsalDeliveryRequest {
                locator,
                idempotency_key: RehearsalIdempotencyKey::new("continue-b".into()).expect("key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([10; 32]),
            },
        )
        .await
        .expect("second Continue resumes the open generation");
    let RehearsalDeliveryClaimResult::Prepared { prepared: resumed } = resumed else {
        panic!("open generation remains prepared");
    };
    assert_eq!(first.descriptor().attempt(), resumed.descriptor().attempt());
}

#[tokio::test]
async fn four_frozen_items_advance_in_order_and_terminalize_on_final_acceptance() {
    let store = MemoryStore::default();
    let (fixture, locator, first) = start_and_freeze(&store).await;
    let mut frozen = vec![first.clone()];
    for value in 2_u128..=4 {
        let mut next = first.clone();
        next.attempt = question_model::RehearsalAttemptId::from_uuid(uuid::Uuid::from_u128(value));
        store
            .append_rehearsal_frozen_item(
                fixture.context,
                AppendRehearsalFrozenItemCommand {
                    locator,
                    frozen: next.clone(),
                },
            )
            .await
            .expect("test fixture appends ordered immutable material");
        frozen.push(next);
    }
    for (index, expected) in frozen.iter().enumerate() {
        let claimed = store
            .claim_rehearsal_delivery(
                fixture.context,
                RehearsalDeliveryRequest {
                    locator,
                    idempotency_key: RehearsalIdempotencyKey::new(format!("ordered-{index}"))
                        .expect("key"),
                    request_fingerprint: RehearsalOperationDigest::from_bytes([index as u8; 32]),
                },
            )
            .await
            .expect("Store selects next frozen ordinal");
        let RehearsalDeliveryClaimResult::Prepared { prepared } = claimed else {
            panic!("next frozen item is prepared");
        };
        assert_eq!(prepared.descriptor().attempt(), expected.attempt);
        complete_submission(
            &store,
            &fixture,
            locator,
            expected,
            &format!("accepted-{index}"),
        )
        .await;
    }
    let receipt = store
        .read_rehearsal(fixture.context, locator)
        .await
        .expect("final rehearsal receipt");
    assert_eq!(
        receipt.lifecycle,
        question_model::RehearsalLifecycle::Completed
    );
}
