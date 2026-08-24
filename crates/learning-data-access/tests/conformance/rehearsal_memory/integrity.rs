//! Test-support integrity corruption matrix for the isolated rehearsal aggregate.

#[cfg(feature = "test-support")]
use super::*;

/// The destructive semantic-corruption matrix is a test-support-only fixture.
/// Normal conformance keeps the ordinary rehearsal lifecycle and idempotency
/// checks in the parent module, while this matrix runs explicitly with
/// `--features test-support`.

#[cfg(feature = "test-support")]
#[tokio::test]
async fn tampered_no_claim_aggregate_refuses_every_lifecycle_decision_atomically() {
    let store = MemoryStore::default();
    let (fixture, locator, frozen) = start_and_freeze(&store).await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::RemoveFrozenItem {
                tenant: fixture.context.tenant_id(),
                rehearsal: locator.rehearsal,
                attempt: frozen.attempt,
            },
        )
        .expect("corrupt test fixture");
    let before = store.rehearsal_state_effect_fingerprint().expect("before");
    assert!(
        store
            .start_rehearsal(
                fixture.context,
                StartRehearsalCommand {
                    actor: locator.actor,
                    course: locator.course,
                    assignment: locator.assignment,
                    revision: locator.revision,
                    subject: synthetic_start(),
                    start_new_after_completion: false,
                },
            )
            .await
            .is_err()
    );
    assert!(
        store
            .discard_rehearsal(fixture.context, locator)
            .await
            .is_err()
    );
    assert!(
        store
            .complete_rehearsal(fixture.context, locator)
            .await
            .is_err()
    );
    assert!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("after")
            .is_unchanged_from(&before)
    );
}

#[cfg(feature = "test-support")]
#[tokio::test]
async fn duplicate_frozen_evidence_refuses_every_rehearsal_and_revision_mutation() {
    let store = MemoryStore::default();
    let (fixture, locator, frozen) = start_and_freeze(&store).await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::DuplicateFrozenEvidence {
                tenant: fixture.context.tenant_id(),
                rehearsal: locator.rehearsal,
                attempt: frozen.attempt,
            },
        )
        .expect("rehash duplicate frozen evidence");
    assert_semantic_corruption_refuses_all_mutations(&store, &fixture, locator).await;
}

#[cfg(feature = "test-support")]
#[tokio::test]
async fn orphan_accepted_evidence_without_claims_refuses_every_rehearsal_and_revision_mutation() {
    let store = MemoryStore::default();
    let (fixture, locator, frozen) = start_and_freeze(&store).await;
    complete_submission(&store, &fixture, locator, &frozen, "orphan-accepted").await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::RemoveAllSubmissionClaims {
                tenant: fixture.context.tenant_id(),
                rehearsal: locator.rehearsal,
            },
        )
        .expect("rehash orphan accepted evidence");
    assert_semantic_corruption_refuses_all_mutations(&store, &fixture, locator).await;
}

#[cfg(feature = "test-support")]
#[tokio::test]
async fn wrong_claim_accepted_evidence_refuses_every_rehearsal_and_revision_mutation() {
    let store = MemoryStore::default();
    let (fixture, locator, frozen) = start_and_freeze(&store).await;
    complete_submission(&store, &fixture, locator, &frozen, "claim-a").await;
    let second_frozen = RehearsalFrozenItemEvidence {
        attempt: RehearsalAttemptId::from_uuid(uuid(850_002)),
        ..frozen.clone()
    };
    store
        .append_rehearsal_frozen_item(
            fixture.context,
            AppendRehearsalFrozenItemCommand {
                locator,
                frozen: second_frozen.clone(),
            },
        )
        .await
        .expect("second freeze");
    complete_submission(&store, &fixture, locator, &second_frozen, "claim-b").await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::ReplaceAcceptedEvidence {
                tenant: fixture.context.tenant_id(),
                rehearsal: locator.rehearsal,
                source_sequence: 2,
                target_sequence: 4,
            },
        )
        .expect("rehash wrong-claim accepted evidence");
    assert_semantic_corruption_refuses_all_mutations(&store, &fixture, locator).await;
}

#[cfg(feature = "test-support")]
#[tokio::test]
async fn duplicated_accepted_evidence_with_completed_owner_refuses_all_mutations() {
    let store = MemoryStore::default();
    let (fixture, locator, frozen) = start_and_freeze(&store).await;
    complete_submission(&store, &fixture, locator, &frozen, "duplicate-accepted").await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::DuplicateAcceptedEvidence {
                tenant: fixture.context.tenant_id(),
                rehearsal: locator.rehearsal,
                sequence: 2,
            },
        )
        .expect("rehash duplicate accepted evidence");
    assert_semantic_corruption_refuses_all_mutations(&store, &fixture, locator).await;
}

#[cfg(feature = "test-support")]
#[tokio::test]
async fn cross_run_accepted_evidence_copy_refuses_every_rehearsal_and_revision_mutation() {
    let store = MemoryStore::default();
    let (fixture, source, frozen) = start_and_freeze(&store).await;
    complete_submission(&store, &fixture, source, &frozen, "source-accepted").await;
    store
        .complete_rehearsal(fixture.context, source)
        .await
        .expect("terminal source rehearsal");
    let target_receipt = store
        .start_rehearsal(
            fixture.context,
            StartRehearsalCommand {
                actor: source.actor,
                course: source.course,
                assignment: source.assignment,
                revision: source.revision,
                subject: synthetic_start(),
                start_new_after_completion: true,
            },
        )
        .await
        .expect("explicit replacement rehearsal");
    let target = RehearsalLocator {
        rehearsal: target_receipt.rehearsal,
        ..source
    };
    let target_frozen = RehearsalFrozenItemEvidence {
        attempt: RehearsalAttemptId::from_uuid(uuid(850_003)),
        ..frozen
    };
    store
        .append_rehearsal_frozen_item(
            fixture.context,
            AppendRehearsalFrozenItemCommand {
                locator: target,
                frozen: target_frozen,
            },
        )
        .await
        .expect("target freeze");
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::CopyAcceptedEvidenceFromRehearsal {
                tenant: fixture.context.tenant_id(),
                rehearsal: target.rehearsal,
                source_rehearsal: source.rehearsal,
            },
        )
        .expect("rehash cross-run evidence copy");
    assert_semantic_corruption_refuses_all_mutations(&store, &fixture, target).await;
}

#[cfg(feature = "test-support")]
#[tokio::test]
async fn corrupt_rehearsal_rolls_back_assignment_definition_revision_and_references() {
    let store = MemoryStore::default();
    let (fixture, locator, frozen) = start_and_freeze(&store).await;
    let before_assignment = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment read")
        .expect("assignment");
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::RemoveFrozenItem {
                tenant: fixture.context.tenant_id(),
                rehearsal: locator.rehearsal,
                attempt: frozen.attempt,
            },
        )
        .expect("corrupt test fixture");
    let before = store.rehearsal_state_effect_fingerprint().expect("before");
    assert!(
        store
            .replace_assignment_fixed_item(
                fixture.context,
                learning_data_access::ReplaceAssignmentFixedItemCommand {
                    actor: fixture.instructor,
                    course: fixture.course,
                    assignment: fixture.assignment,
                    current_item: before_assignment.record.items[0].id,
                    expected_revision: before_assignment.revision,
                    replacement: before_assignment.record.items[0].reference,
                },
            )
            .await
            .is_err()
    );
    assert_eq!(
        store
            .get_assignment_for_edit(fixture.context, fixture.assignment)
            .await
            .expect("assignment read"),
        Some(before_assignment),
    );
    assert!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("after")
            .is_unchanged_from(&before)
    );
}

#[cfg(feature = "test-support")]
#[tokio::test]
async fn corrupt_later_claim_history_stages_no_partial_terminal_revocation() {
    let store = MemoryStore::default();
    let (fixture, locator, frozen) = start_and_freeze(&store).await;
    let key = RehearsalSubmissionIdempotencyKey::new("later-claim".into()).expect("key");
    let claimed = store
        .claim_rehearsal_submission(
            fixture.context,
            ClaimRehearsalSubmissionCommand {
                locator,
                attempt: frozen.attempt,
                response: StudentResponse::Numeric { value: 3.0 },
                idempotency_key: key.clone(),
            },
        )
        .await
        .expect("claim");
    assert!(matches!(
        claimed,
        RehearsalSubmissionClaimResult::Claimed(_)
    ));
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::DropLatestClaimEvent {
                tenant: fixture.context.tenant_id(),
                rehearsal: locator.rehearsal,
                idempotency_key: key,
            },
        )
        .expect("corrupt claim fixture");
    let before = store.rehearsal_state_effect_fingerprint().expect("before");
    assert!(
        store
            .discard_rehearsal(fixture.context, locator)
            .await
            .is_err()
    );
    assert!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("after")
            .is_unchanged_from(&before)
    );
}

#[cfg(feature = "test-support")]
#[tokio::test]
async fn persisted_claim_request_and_fingerprint_tampering_refuse_reads_claims_and_all_mutations() {
    for corruption in ["request", "fingerprint"] {
        let store = MemoryStore::default();
        let (fixture, locator, frozen) = start_and_freeze(&store).await;
        let key =
            RehearsalSubmissionIdempotencyKey::new(format!("root-{corruption}")).expect("key");
        let claimed = store
            .claim_rehearsal_submission(
                fixture.context,
                ClaimRehearsalSubmissionCommand {
                    locator,
                    attempt: frozen.attempt,
                    response: StudentResponse::Numeric { value: 3.0 },
                    idempotency_key: key.clone(),
                },
            )
            .await
            .expect("prepared claim");
        let RehearsalSubmissionClaimResult::Claimed(claimed) = claimed else {
            panic!("fixture claim is prepared");
        };
        let selector = match corruption {
            "request" => {
                MemoryRehearsalIntegrityTestCorruption::ReplaceClaimRequestWithoutFingerprint {
                    tenant: fixture.context.tenant_id(),
                    rehearsal: locator.rehearsal,
                    idempotency_key: key.clone(),
                    response: StudentResponse::Numeric { value: 4.0 },
                }
            }
            "fingerprint" => {
                MemoryRehearsalIntegrityTestCorruption::ReplaceClaimFingerprintWithoutRequest {
                    tenant: fixture.context.tenant_id(),
                    rehearsal: locator.rehearsal,
                    idempotency_key: key.clone(),
                    response: StudentResponse::Numeric { value: 4.0 },
                }
            }
            _ => unreachable!("closed corruption table"),
        };
        store
            .corrupt_rehearsal_integrity_for_test(selector)
            .expect("test-only root corruption");
        let before = store
            .rehearsal_state_effect_fingerprint()
            .expect("baseline");
        assert!(
            store
                .read_rehearsal(fixture.context, locator)
                .await
                .is_err()
        );
        assert!(
            store
                .claim_rehearsal_submission(
                    fixture.context,
                    ClaimRehearsalSubmissionCommand {
                        locator,
                        attempt: frozen.attempt,
                        response: StudentResponse::Numeric { value: 3.0 },
                        idempotency_key: RehearsalSubmissionIdempotencyKey::new(format!(
                            "root-read-{corruption}"
                        ))
                        .expect("key"),
                    },
                )
                .await
                .is_err()
        );
        assert!(
            store
                .mark_rehearsal_submission_dispatched(
                    fixture.context,
                    MarkRehearsalSubmissionDispatchedCommand {
                        locator,
                        handle: claimed.handle,
                    },
                )
                .await
                .is_err()
        );
        assert_semantic_corruption_refuses_all_mutations(&store, &fixture, locator).await;
        assert!(
            store
                .rehearsal_state_effect_fingerprint()
                .expect("after")
                .is_unchanged_from(&before)
        );
    }
}

#[cfg(feature = "test-support")]
#[tokio::test]
async fn rehashed_frozen_item_commitment_mutations_refuse_the_complete_rehearsal_boundary() {
    for corruption in ["content", "schema"] {
        let store = MemoryStore::default();
        let (fixture, locator, frozen) = start_and_freeze(&store).await;
        let key =
            RehearsalSubmissionIdempotencyKey::new(format!("frozen-{corruption}")).expect("key");
        let claimed = store
            .claim_rehearsal_submission(
                fixture.context,
                ClaimRehearsalSubmissionCommand {
                    locator,
                    attempt: frozen.attempt,
                    response: StudentResponse::Numeric { value: 3.0 },
                    idempotency_key: key,
                },
            )
            .await
            .expect("prepared claim");
        let RehearsalSubmissionClaimResult::Claimed(claimed) = claimed else {
            panic!("new request is prepared");
        };
        let selector = match corruption {
            "content" => MemoryRehearsalIntegrityTestCorruption::ReplaceFrozenContentDigest {
                tenant: fixture.context.tenant_id(),
                rehearsal: locator.rehearsal,
                attempt: frozen.attempt,
                digest: question_model::RehearsalEvidenceDigest::from_bytes([9; 32]),
            },
            "schema" => MemoryRehearsalIntegrityTestCorruption::ReplaceFrozenResponseDefinition {
                tenant: fixture.context.tenant_id(),
                rehearsal: locator.rehearsal,
                attempt: frozen.attempt,
                response_definition: ResponseDefinition::Numeric {
                    tolerance: NumericTolerance::Exact,
                    unit: Some("mL".into()),
                },
            },
            _ => unreachable!("closed corruption table"),
        };
        store
            .corrupt_rehearsal_integrity_for_test(selector)
            .expect("rehash frozen commitment fixture");
        let before = store
            .rehearsal_state_effect_fingerprint()
            .expect("baseline");
        assert!(
            store
                .read_rehearsal(fixture.context, locator)
                .await
                .is_err()
        );
        assert!(
            store
                .mark_rehearsal_submission_dispatched(
                    fixture.context,
                    MarkRehearsalSubmissionDispatchedCommand {
                        locator,
                        handle: claimed.handle,
                    },
                )
                .await
                .is_err()
        );
        assert_semantic_corruption_refuses_all_mutations(&store, &fixture, locator).await;
        assert!(
            store
                .rehearsal_state_effect_fingerprint()
                .expect("after")
                .is_unchanged_from(&before)
        );
    }
}

/// Rehashing the frozen map and evidence after changing `frozen_at` without
/// advancing the independently stored evidence head must be rejected by every
/// aggregate boundary, with no observable partial mutation.
#[cfg(feature = "test-support")]
#[tokio::test]
async fn rehashed_frozen_timestamp_mutation_is_rejected_by_the_aggregate_boundary() {
    let store = MemoryStore::default();
    let (fixture, locator, frozen) = start_and_freeze(&store).await;
    let dispatched_claim = store
        .claim_rehearsal_submission(
            fixture.context,
            ClaimRehearsalSubmissionCommand {
                locator,
                attempt: frozen.attempt,
                response: StudentResponse::Numeric { value: 3.0 },
                idempotency_key: RehearsalSubmissionIdempotencyKey::new(
                    "timestamp-anchor-dispatched".into(),
                )
                .expect("key"),
            },
        )
        .await
        .expect("prepared claim");
    let RehearsalSubmissionClaimResult::Claimed(dispatched_claim) = dispatched_claim else {
        panic!("timestamp fixture claim is prepared");
    };
    let dispatched = store
        .mark_rehearsal_submission_dispatched(
            fixture.context,
            MarkRehearsalSubmissionDispatchedCommand {
                locator,
                handle: dispatched_claim.handle,
            },
        )
        .await
        .expect("dispatched claim");
    let prepared_claim = store
        .claim_rehearsal_submission(
            fixture.context,
            ClaimRehearsalSubmissionCommand {
                locator,
                attempt: frozen.attempt,
                response: StudentResponse::Numeric { value: 3.0 },
                idempotency_key: RehearsalSubmissionIdempotencyKey::new(
                    "timestamp-anchor-prepared".into(),
                )
                .expect("key"),
            },
        )
        .await
        .expect("prepared claim");
    let RehearsalSubmissionClaimResult::Claimed(prepared_claim) = prepared_claim else {
        panic!("timestamp fixture second claim is prepared");
    };
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::ReplaceFrozenTimestamp {
                tenant: fixture.context.tenant_id(),
                rehearsal: locator.rehearsal,
                attempt: frozen.attempt,
                frozen_at: ActivityTimestamp::from_unix_millis(501),
            },
        )
        .expect("rehash timestamp mutation fixture");
    let before = store.rehearsal_state_effect_fingerprint().expect("before");
    assert!(
        store
            .read_rehearsal(fixture.context, locator)
            .await
            .is_err(),
        "a persisted frozen timestamp is immutable evidence and must be anchor-bound"
    );
    assert!(
        store
            .claim_rehearsal_submission(
                fixture.context,
                ClaimRehearsalSubmissionCommand {
                    locator,
                    attempt: frozen.attempt,
                    response: StudentResponse::Numeric { value: 3.0 },
                    idempotency_key: RehearsalSubmissionIdempotencyKey::new(
                        "timestamp-anchor-new".into(),
                    )
                    .expect("key"),
                },
            )
            .await
            .is_err()
    );
    assert!(
        store
            .mark_rehearsal_submission_dispatched(
                fixture.context,
                MarkRehearsalSubmissionDispatchedCommand {
                    locator,
                    handle: prepared_claim.handle,
                },
            )
            .await
            .is_err()
    );
    assert!(
        store
            .complete_rehearsal_submission(
                fixture.context,
                CompleteRehearsalSubmissionCommand {
                    locator,
                    handle: dispatched,
                    grading: deterministic_grade(),
                },
            )
            .await
            .is_err()
    );
    assert_semantic_corruption_refuses_all_mutations(&store, &fixture, locator).await;
    assert!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("after")
            .is_unchanged_from(&before)
    );
}

/// The aggregate head is independently persisted: either field changing must
/// refuse the exact same authorized read boundary as rewritten evidence.
#[cfg(feature = "test-support")]
#[tokio::test]
async fn evidence_head_digest_and_length_tampering_fail_closed() {
    for corruption in [
        MemoryRehearsalIntegrityTestCorruption::ReplaceEvidenceHeadDigest {
            tenant: question_model::TenantId::from_uuid(uuid::Uuid::nil()),
            rehearsal: question_model::RehearsalReference::new(1).expect("reference"),
            digest: RehearsalEvidenceDigest::from_bytes([91; 32]),
        },
        MemoryRehearsalIntegrityTestCorruption::ReplaceEvidenceHeadLength {
            tenant: question_model::TenantId::from_uuid(uuid::Uuid::nil()),
            rehearsal: question_model::RehearsalReference::new(1).expect("reference"),
            length: 99,
        },
    ] {
        let store = MemoryStore::default();
        let (fixture, locator, _) = start_and_freeze(&store).await;
        let corruption = match corruption {
            MemoryRehearsalIntegrityTestCorruption::ReplaceEvidenceHeadDigest {
                digest, ..
            } => MemoryRehearsalIntegrityTestCorruption::ReplaceEvidenceHeadDigest {
                tenant: fixture.context.tenant_id(),
                rehearsal: locator.rehearsal,
                digest,
            },
            MemoryRehearsalIntegrityTestCorruption::ReplaceEvidenceHeadLength {
                length, ..
            } => MemoryRehearsalIntegrityTestCorruption::ReplaceEvidenceHeadLength {
                tenant: fixture.context.tenant_id(),
                rehearsal: locator.rehearsal,
                length,
            },
            _ => unreachable!("closed selector table"),
        };
        store
            .corrupt_rehearsal_integrity_for_test(corruption)
            .expect("head corruption fixture");
        assert!(
            store
                .read_rehearsal(fixture.context, locator)
                .await
                .is_err()
        );
        assert_semantic_corruption_refuses_all_mutations(&store, &fixture, locator).await;
    }
}
