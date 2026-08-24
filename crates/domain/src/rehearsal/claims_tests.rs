use super::*;
use question_model::{
    ActivityTimestamp, AssignmentReference, CourseId, CourseMembershipId, ProblemId,
    ProblemVersionRef, RehearsalAttemptId, RehearsalGradeOperationId, RehearsalRunId,
    RehearsalSubmissionClaimId, TenantId, VersionId,
};
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
fn claim(value: u128) -> RehearsalSubmissionClaimId {
    RehearsalSubmissionClaimId::from_uuid(id(value))
}
fn operation(value: u128) -> RehearsalGradeOperationId {
    RehearsalGradeOperationId::from_uuid(id(value))
}
fn revision(value: u64) -> TeachingOperationRevision {
    TeachingOperationRevision::new(value).unwrap()
}
fn fingerprint(value: u8) -> RehearsalSubmissionRequestFingerprint {
    RehearsalSubmissionRequestFingerprint([value; 32])
}
fn rehearsal_context(run: u128) -> RehearsalGenesisContext {
    RehearsalGenesisContext {
        rehearsal: RehearsalRunId::from_uuid(id(run)),
        tenant: TenantId::from_uuid(id(2)),
        course: CourseId::from_uuid(id(3)),
        assignment: AssignmentReference::new(1).unwrap(),
        direct_instructor_membership: CourseMembershipId::from_uuid(id(4)),
        revision: revision(1),
        subject_fingerprint: RehearsalSubjectFingerprint([5; 32]),
    }
}
fn frozen() -> RehearsalFrozenItemEvidence {
    RehearsalFrozenItemEvidence {
        attempt: RehearsalAttemptId::from_uuid(id(6)),
        problem: ProblemVersionRef {
            problem: ProblemId::from_uuid(id(7)),
            version: VersionId::from_uuid(id(8)),
        },
        response_definition: question_model::ResponseDefinition::Numeric {
            tolerance: question_model::answer::NumericTolerance::Exact,
            unit: None,
        },
        canonical_content_digest: RehearsalEvidenceDigest::from_bytes([9; 32]),
        frozen_at: ActivityTimestamp::from_unix_millis(10),
    }
}
fn request(
    frozen: &RehearsalFrozenItemEvidence,
    value: f64,
) -> RehearsalValidatedSubmissionRequest {
    RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
        frozen,
        frozen.attempt,
        question_model::StudentResponse::Numeric { value },
    )
    .unwrap()
}
fn claim_root(
    context: RehearsalGenesisContext,
    claim_id: RehearsalSubmissionClaimId,
    request: RehearsalValidatedSubmissionRequest,
) -> RehearsalClaimRoot {
    let frozen = frozen();
    let fp = rehearsal_submission_request_fingerprint(context, &frozen, &request).unwrap();
    RehearsalClaimRoot::verify_persisted(
        context,
        &frozen,
        RehearsalPersistedClaimRoot::from_persisted(context.rehearsal, claim_id, fp, request),
    )
    .unwrap()
}
fn event(
    root: &RehearsalClaimRoot,
    sequence: u64,
    operation: RehearsalGradeOperationId,
    generation: RehearsalClaimGeneration,
    phase: RehearsalSubmissionClaimPhase,
) -> RehearsalClaimTransitionEvent {
    root.restore_transition(
        sequence,
        operation,
        generation,
        phase,
        ActivityTimestamp::from_unix_millis(i64::try_from(sequence).unwrap()),
        None,
        None,
    )
}
fn abandoned(
    root: &RehearsalClaimRoot,
    sequence: u64,
    operation: RehearsalGradeOperationId,
    generation: RehearsalClaimGeneration,
) -> RehearsalClaimTransitionEvent {
    root.restore_transition(
        sequence,
        operation,
        generation,
        RehearsalSubmissionClaimPhase::AbandonedBeforeDispatch,
        ActivityTimestamp::from_unix_millis(i64::try_from(sequence).unwrap()),
        Some(RehearsalPreDispatchAbandonReason::LocalPreparationFailed),
        None,
    )
}
fn graded_result() -> RehearsalPrivateGradingResult {
    RehearsalPrivateGradingResult::Graded {
        result: question_model::AttemptResult {
            correct: true,
            points_earned: 1.0,
            points_possible: 1.0,
        },
        feedback: question_model::DisclosedFeedback::empty(),
        backend_receipt_reference: question_model::RehearsalBackendReceiptReference::new(
            "native:claims".into(),
        )
        .unwrap(),
    }
}
fn evidence_entries(
    context: RehearsalGenesisContext,
    root: &RehearsalClaimRoot,
) -> Vec<RehearsalEvidenceChainEntry> {
    let frozen = frozen();
    let accepted = RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
        root,
        root.sealed_request().clone(),
        &frozen,
        graded_result(),
        ActivityTimestamp::from_unix_millis(12),
    )
    .unwrap();
    rehashed_entries(
        context,
        vec![
            RehearsalEvidencePayload::FrozenItem(frozen),
            RehearsalEvidencePayload::AcceptedSubmission(accepted),
        ],
    )
}

fn rehashed_entries(
    context: RehearsalGenesisContext,
    payloads: Vec<RehearsalEvidencePayload>,
) -> Vec<RehearsalEvidenceChainEntry> {
    let mut previous = evidence_genesis_digest(context);
    payloads
        .into_iter()
        .enumerate()
        .map(|(index, payload)| {
            let sequence = u32::try_from(index + 1).unwrap();
            let recorded_at = ActivityTimestamp::from_unix_millis(i64::from(sequence) + 20);
            let digest = evidence_entry_digest(
                sequence,
                payload.kind(),
                previous,
                private_payload_digest(&payload),
                recorded_at,
            );
            let result = RehearsalEvidenceChainEntry {
                record: question_model::RehearsalEvidenceRecord {
                    sequence,
                    kind: payload.kind(),
                    previous_digest: Some(previous),
                    digest,
                    recorded_at,
                },
                payload,
            };
            previous = digest;
            result
        })
        .collect()
}
fn evidence_head(entries: &[RehearsalEvidenceChainEntry]) -> RehearsalEvidenceHead {
    entries.last().map_or_else(
        || evidence_genesis_head(rehearsal_context(1)),
        |entry| RehearsalEvidenceHead::from_persisted(entry.record.digest, entry.record.sequence),
    )
}
fn proof(
    context: RehearsalGenesisContext,
    root: &RehearsalClaimRoot,
) -> VerifiedRehearsalClaimCompletionProof {
    let entries = evidence_entries(context, root);
    verify_rehearsal_claim_completion_proof(context, evidence_head(&entries), root, &entries)
        .unwrap()
}

#[test]
fn full_history_tracks_three_generations_and_fences_nonadjacent_operation_reuse() {
    let context = rehearsal_context(1);
    let frozen = frozen();
    let root = claim_root(context, claim(1), request(&frozen, 1.0));
    let one = RehearsalClaimGeneration::first();
    let two = one.next().unwrap();
    let three = two.next().unwrap();
    let legal = [
        event(
            &root,
            1,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::Prepared,
        ),
        abandoned(&root, 2, operation(11), one),
        event(
            &root,
            3,
            operation(12),
            two,
            RehearsalSubmissionClaimPhase::Prepared,
        ),
        abandoned(&root, 4, operation(12), two),
        event(
            &root,
            5,
            operation(13),
            three,
            RehearsalSubmissionClaimPhase::Prepared,
        ),
    ];
    let snapshot = hydrate_claim_history(&root, &legal, None).unwrap();
    assert_eq!(snapshot.generation(), three);
    assert!(matches!(
        decide_submission_claim(
            RehearsalLifecycle::Active,
            true,
            Some(&snapshot),
            root.fingerprint(),
            &root,
            operation(14)
        ),
        RehearsalSubmissionClaimDecision::Pending
    ));
    let reused = [
        event(
            &root,
            1,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::Prepared,
        ),
        abandoned(&root, 2, operation(11), one),
        event(
            &root,
            3,
            operation(12),
            two,
            RehearsalSubmissionClaimPhase::Prepared,
        ),
        abandoned(&root, 4, operation(12), two),
        event(
            &root,
            5,
            operation(11),
            three,
            RehearsalSubmissionClaimPhase::Prepared,
        ),
    ];
    assert_eq!(
        hydrate_claim_history(&root, &reused, None),
        Err(RehearsalClaimHydrationError::ReusedOperation)
    );
    let abandoned_snapshot = hydrate_claim_history(&root, &legal[..4], None).unwrap();
    assert!(matches!(
        decide_submission_claim(
            RehearsalLifecycle::Active,
            true,
            Some(&abandoned_snapshot),
            root.fingerprint(),
            &root,
            operation(11)
        ),
        RehearsalSubmissionClaimDecision::ReclaimRefused(
            RehearsalClaimReclaimError::ReusedOperation
        )
    ));
}

#[test]
fn root_binding_rejects_swapped_reordered_and_duplicate_histories_before_handles_exist() {
    let context = rehearsal_context(1);
    let frozen = frozen();
    let root = claim_root(context, claim(1), request(&frozen, 1.0));
    let other = claim_root(context, claim(2), request(&frozen, 1.0));
    let one = RehearsalClaimGeneration::first();
    let prepared = event(
        &root,
        1,
        operation(11),
        one,
        RehearsalSubmissionClaimPhase::Prepared,
    );
    let dispatched = event(
        &root,
        2,
        operation(11),
        one,
        RehearsalSubmissionClaimPhase::GradingDispatched,
    );
    assert_eq!(
        hydrate_claim_history(&other, &[prepared], None),
        Err(RehearsalClaimHydrationError::RootMismatch)
    );
    assert_eq!(
        hydrate_claim_history(&root, &[dispatched, prepared], None),
        Err(RehearsalClaimHydrationError::FirstEventNotPreparedGenerationOne)
    );
    assert_eq!(
        hydrate_claim_history(&root, &[prepared, prepared], None),
        Err(RehearsalClaimHydrationError::SequenceNotContiguous)
    );
}

#[test]
fn persisted_root_must_verify_sealed_request_before_prepared_or_dispatched_history_can_hydrate() {
    let context = rehearsal_context(1);
    let frozen = frozen();
    let request_a = request(&frozen, 1.0);
    let verified = claim_root(context, claim(1), request_a);
    let one = RehearsalClaimGeneration::first();
    let prepared = event(
        &verified,
        1,
        operation(11),
        one,
        RehearsalSubmissionClaimPhase::Prepared,
    );
    let dispatched = event(
        &verified,
        2,
        operation(11),
        one,
        RehearsalSubmissionClaimPhase::GradingDispatched,
    );
    let tampered_fingerprint = RehearsalPersistedClaimRoot::from_persisted(
        context.rehearsal,
        claim(1),
        fingerprint(42),
        verified.sealed_request().clone(),
    );
    assert!(matches!(
        RehearsalClaimRoot::verify_persisted(context, &frozen, tampered_fingerprint),
        Err(RehearsalClaimRootVerificationError::FingerprintMismatch)
    ));

    let tampered_request = RehearsalPersistedClaimRoot::from_persisted(
        context.rehearsal,
        claim(1),
        verified.fingerprint(),
        request(&frozen, 2.0),
    );
    assert!(matches!(
        RehearsalClaimRoot::verify_persisted(context, &frozen, tampered_request),
        Err(RehearsalClaimRootVerificationError::FingerprintMismatch)
    ));

    // Only a verified root has `restore_transition` and can be passed to
    // hydration. The legal persisted history above therefore cannot yield a
    // Prepared/Dispatched handle or grader input after either verification
    // failure.
    assert!(
        hydrate_claim_history(&verified, &[prepared], None)
            .unwrap()
            .into_prepared_handle()
            .is_ok()
    );
    assert!(
        hydrate_claim_history(&verified, &[prepared, dispatched], None)
            .unwrap()
            .into_dispatched_handle()
            .is_ok()
    );
}

#[test]
fn completed_history_requires_chain_verified_aggregate_bound_proof_and_replays_its_receipt() {
    let context = rehearsal_context(1);
    let frozen = frozen();
    let root = claim_root(context, claim(1), request(&frozen, 1.0));
    let one = RehearsalClaimGeneration::first();
    let verified = proof(context, &root);
    let history = [
        event(
            &root,
            1,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::Prepared,
        ),
        event(
            &root,
            2,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::GradingDispatched,
        ),
        root.restore_transition(
            3,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::Completed,
            ActivityTimestamp::from_unix_millis(3),
            None,
            Some(verified.completion_material()),
        ),
    ];
    assert_eq!(
        hydrate_claim_history(&root, &history, None),
        Err(RehearsalClaimHydrationError::MissingCompletionProof)
    );
    let snapshot = hydrate_claim_history(&root, &history, Some(verified)).unwrap();
    assert!(matches!(
        decide_submission_claim(
            RehearsalLifecycle::Active,
            true,
            Some(&snapshot),
            root.fingerprint(),
            &root,
            operation(12)
        ),
        RehearsalSubmissionClaimDecision::Replay {
            receipt: question_model::RehearsalPublicOutcome::Submitted { .. }
        }
    ));
}

#[test]
fn completion_proof_rejects_cross_run_fingerprint_and_duplicate_accepted_evidence() {
    let context = rehearsal_context(1);
    let frozen = frozen();
    let root = claim_root(context, claim(1), request(&frozen, 1.0));
    assert_eq!(
        verify_rehearsal_claim_completion_proof(
            rehearsal_context(99),
            evidence_head(&evidence_entries(context, &root)),
            &root,
            &evidence_entries(context, &root)
        ),
        Err(RehearsalClaimCompletionProofError::ContextRunMismatch)
    );
    let wrong_root = RehearsalPersistedClaimRoot::from_persisted(
        context.rehearsal,
        claim(1),
        fingerprint(42),
        root.sealed_request().clone(),
    );
    assert!(matches!(
        RehearsalClaimRoot::verify_persisted(context, &frozen, wrong_root),
        Err(RehearsalClaimRootVerificationError::FingerprintMismatch)
    ));
    let mut entries = evidence_entries(context, &root);
    let duplicate = entries[1].clone();
    let previous = entries[1].record.digest;
    let payload = duplicate.payload.clone();
    entries.push(RehearsalEvidenceChainEntry {
        record: question_model::RehearsalEvidenceRecord {
            sequence: 3,
            kind: payload.kind(),
            previous_digest: Some(previous),
            digest: evidence_entry_digest(
                3,
                payload.kind(),
                previous,
                private_payload_digest(&payload),
                ActivityTimestamp::from_unix_millis(23),
            ),
            recorded_at: ActivityTimestamp::from_unix_millis(23),
        },
        payload,
    });
    assert_eq!(
        verify_rehearsal_claim_completion_proof(context, evidence_head(&entries), &root, &entries),
        Err(RehearsalClaimCompletionProofError::DuplicateAcceptedSubmission)
    );
}

#[test]
fn completion_proof_requires_one_frozen_attempt_strictly_before_accepted_submission() {
    let context = rehearsal_context(1);
    let frozen = frozen();
    let root = claim_root(context, claim(1), request(&frozen, 1.0));
    let accepted = RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
        &root,
        root.sealed_request().clone(),
        &frozen,
        graded_result(),
        ActivityTimestamp::from_unix_millis(12),
    )
    .unwrap();

    let reverse = rehashed_entries(
        context,
        vec![
            RehearsalEvidencePayload::AcceptedSubmission(accepted.clone()),
            RehearsalEvidencePayload::FrozenItem(frozen.clone()),
        ],
    );
    assert_eq!(
        verify_rehearsal_claim_completion_proof(context, evidence_head(&reverse), &root, &reverse),
        Err(RehearsalClaimCompletionProofError::FrozenAttemptNotBeforeAcceptedSubmission)
    );

    let missing = rehashed_entries(
        context,
        vec![RehearsalEvidencePayload::AcceptedSubmission(
            accepted.clone(),
        )],
    );
    assert_eq!(
        verify_rehearsal_claim_completion_proof(context, evidence_head(&missing), &root, &missing),
        Err(RehearsalClaimCompletionProofError::FrozenAttemptMissing)
    );

    let duplicate = rehashed_entries(
        context,
        vec![
            RehearsalEvidencePayload::FrozenItem(frozen.clone()),
            RehearsalEvidencePayload::FrozenItem(frozen),
            RehearsalEvidencePayload::AcceptedSubmission(accepted),
        ],
    );
    assert_eq!(
        verify_rehearsal_claim_completion_proof(
            context,
            evidence_head(&duplicate),
            &root,
            &duplicate
        ),
        Err(RehearsalClaimCompletionProofError::DuplicateFrozenAttempt)
    );
}

#[test]
fn completion_proof_with_later_frozen_attempt_cannot_hydrate_completed_or_replay() {
    let context = rehearsal_context(1);
    let frozen = frozen();
    let root = claim_root(context, claim(1), request(&frozen, 1.0));
    let one = RehearsalClaimGeneration::first();
    let accepted = RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
        &root,
        root.sealed_request().clone(),
        &frozen,
        graded_result(),
        ActivityTimestamp::from_unix_millis(12),
    )
    .unwrap();
    let later_frozen = rehashed_entries(
        context,
        vec![
            RehearsalEvidencePayload::AcceptedSubmission(accepted),
            RehearsalEvidencePayload::FrozenItem(frozen),
        ],
    );
    assert!(matches!(
        verify_rehearsal_claim_completion_proof(
            context,
            evidence_head(&later_frozen),
            &root,
            &later_frozen,
        ),
        Err(RehearsalClaimCompletionProofError::FrozenAttemptNotBeforeAcceptedSubmission)
    ));

    let asserted_material = RehearsalClaimCompletionMaterial::from_persisted(
        u64::from(later_frozen[0].record.sequence),
        later_frozen[0].record.digest,
        rehearsal_public_receipt_digest(&question_model::RehearsalPublicOutcome::Submitted {
            feedback: question_model::DisclosedFeedback::empty(),
        }),
    )
    .unwrap();
    // Even a fully rehashed, persisted-looking completion material cannot
    // hydrate without a verified proof, and therefore cannot expose Replay.
    let completed = [
        event(
            &root,
            1,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::Prepared,
        ),
        event(
            &root,
            2,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::GradingDispatched,
        ),
        root.restore_transition(
            3,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::Completed,
            ActivityTimestamp::from_unix_millis(3),
            None,
            Some(asserted_material),
        ),
    ];
    assert_eq!(
        hydrate_claim_history(&root, &completed, None),
        Err(RehearsalClaimHydrationError::MissingCompletionProof)
    );
}

#[test]
fn transition_matrix_allows_prepared_and_dispatched_revocations_and_refuses_post_terminal_events() {
    let context = rehearsal_context(1);
    let root = claim_root(context, claim(1), request(&frozen(), 1.0));
    let one = RehearsalClaimGeneration::first();
    for terminal in [
        RehearsalSubmissionClaimPhase::RevokedStaleRevision,
        RehearsalSubmissionClaimPhase::RevokedTerminalLifecycle,
        RehearsalSubmissionClaimPhase::RevokedSourceContextRemoved,
    ] {
        let prepared_history = [
            event(
                &root,
                1,
                operation(11),
                one,
                RehearsalSubmissionClaimPhase::Prepared,
            ),
            event(&root, 2, operation(11), one, terminal),
        ];
        assert!(hydrate_claim_history(&root, &prepared_history, None).is_ok());
        let dispatched_history = [
            event(
                &root,
                1,
                operation(11),
                one,
                RehearsalSubmissionClaimPhase::Prepared,
            ),
            event(
                &root,
                2,
                operation(11),
                one,
                RehearsalSubmissionClaimPhase::GradingDispatched,
            ),
            event(&root, 3, operation(11), one, terminal),
        ];
        assert!(hydrate_claim_history(&root, &dispatched_history, None).is_ok());
    }
    let illegal = [
        event(
            &root,
            1,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::Prepared,
        ),
        event(
            &root,
            2,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::GradingDispatched,
        ),
        abandoned(&root, 3, operation(11), one),
    ];
    assert_eq!(
        hydrate_claim_history(&root, &illegal, None),
        Err(RehearsalClaimHydrationError::IllegalTransition)
    );
}

#[test]
fn source_context_revocation_is_terminal_and_has_no_terminal_material() {
    let context = rehearsal_context(1);
    let root = claim_root(context, claim(1), request(&frozen(), 1.0));
    let one = RehearsalClaimGeneration::first();
    let source_revoked = [
        event(
            &root,
            1,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::Prepared,
        ),
        event(
            &root,
            2,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::RevokedSourceContextRemoved,
        ),
    ];
    let snapshot = hydrate_claim_history(&root, &source_revoked, None).unwrap();
    assert_eq!(
        snapshot.state(),
        RehearsalSubmissionClaimState::RevokedSourceContextRemoved
    );
    assert!(matches!(
        decide_submission_claim(
            RehearsalLifecycle::Active,
            true,
            Some(&snapshot),
            root.fingerprint(),
            &root,
            operation(12),
        ),
        RehearsalSubmissionClaimDecision::TerminalLifecycle
    ));
    let post_terminal = [
        source_revoked[0],
        source_revoked[1],
        event(
            &root,
            3,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::GradingDispatched,
        ),
    ];
    assert_eq!(
        hydrate_claim_history(&root, &post_terminal, None),
        Err(RehearsalClaimHydrationError::IllegalTransition)
    );

    let malformed_abandon_reason = [
        source_revoked[0],
        root.restore_transition(
            2,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::RevokedSourceContextRemoved,
            ActivityTimestamp::from_unix_millis(2),
            Some(RehearsalPreDispatchAbandonReason::LocalPreparationFailed),
            None,
        ),
    ];
    assert_eq!(
        hydrate_claim_history(&root, &malformed_abandon_reason, None),
        Err(RehearsalClaimHydrationError::PhaseMaterialMismatch)
    );
    let malformed_completion_material = [
        source_revoked[0],
        root.restore_transition(
            2,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::RevokedSourceContextRemoved,
            ActivityTimestamp::from_unix_millis(2),
            None,
            Some(proof(context, &root).completion_material()),
        ),
    ];
    assert_eq!(
        hydrate_claim_history(&root, &malformed_completion_material, None),
        Err(RehearsalClaimHydrationError::PhaseMaterialMismatch)
    );
}

#[test]
fn every_closed_abandonment_reason_is_pre_dispatch_only_and_terminal_material_is_refused() {
    let context = rehearsal_context(1);
    let root = claim_root(context, claim(1), request(&frozen(), 1.0));
    let one = RehearsalClaimGeneration::first();
    for reason in [
        RehearsalPreDispatchAbandonReason::LocalPreparationFailed,
        RehearsalPreDispatchAbandonReason::NativeBackendAdmissionRejected,
        RehearsalPreDispatchAbandonReason::TrustedRendererAdmissionRejected,
    ] {
        let history = [
            event(
                &root,
                1,
                operation(11),
                one,
                RehearsalSubmissionClaimPhase::Prepared,
            ),
            root.restore_transition(
                2,
                operation(11),
                one,
                RehearsalSubmissionClaimPhase::AbandonedBeforeDispatch,
                ActivityTimestamp::from_unix_millis(2),
                Some(reason),
                None,
            ),
        ];
        assert!(hydrate_claim_history(&root, &history, None).is_ok());
    }
    let malformed_terminal = [
        event(
            &root,
            1,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::Prepared,
        ),
        root.restore_transition(
            2,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::RevokedStaleRevision,
            ActivityTimestamp::from_unix_millis(2),
            Some(RehearsalPreDispatchAbandonReason::LocalPreparationFailed),
            None,
        ),
    ];
    assert_eq!(
        hydrate_claim_history(&root, &malformed_terminal, None),
        Err(RehearsalClaimHydrationError::PhaseMaterialMismatch)
    );
}

#[test]
fn completed_history_refuses_a_verified_proof_from_another_root() {
    let context = rehearsal_context(1);
    let source = claim_root(context, claim(1), request(&frozen(), 1.0));
    let other = claim_root(context, claim(2), request(&frozen(), 1.0));
    let one = RehearsalClaimGeneration::first();
    let source_proof = proof(context, &source);
    let history = [
        event(
            &other,
            1,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::Prepared,
        ),
        event(
            &other,
            2,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::GradingDispatched,
        ),
        other.restore_transition(
            3,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::Completed,
            ActivityTimestamp::from_unix_millis(3),
            None,
            Some(source_proof.completion_material()),
        ),
    ];
    assert_eq!(
        hydrate_claim_history(&other, &history, Some(source_proof)),
        Err(RehearsalClaimHydrationError::CompletionProofMismatch)
    );
}

#[test]
fn completed_history_rejects_persisted_receipt_or_evidence_digest_substitution() {
    let context = rehearsal_context(1);
    let root = claim_root(context, claim(1), request(&frozen(), 1.0));
    let one = RehearsalClaimGeneration::first();
    let verified = proof(context, &root);
    let material = verified.completion_material();
    for replacement in [
        RehearsalClaimCompletionMaterial::from_persisted(
            material.accepted_evidence_sequence(),
            RehearsalEvidenceDigest::from_bytes([99; 32]),
            material.receipt_digest(),
        )
        .unwrap(),
        RehearsalClaimCompletionMaterial::from_persisted(
            material.accepted_evidence_sequence(),
            material.accepted_evidence_digest(),
            RehearsalEvidenceDigest::from_bytes([98; 32]),
        )
        .unwrap(),
    ] {
        let history = [
            event(
                &root,
                1,
                operation(11),
                one,
                RehearsalSubmissionClaimPhase::Prepared,
            ),
            event(
                &root,
                2,
                operation(11),
                one,
                RehearsalSubmissionClaimPhase::GradingDispatched,
            ),
            root.restore_transition(
                3,
                operation(11),
                one,
                RehearsalSubmissionClaimPhase::Completed,
                ActivityTimestamp::from_unix_millis(3),
                None,
                Some(replacement),
            ),
        ];
        assert_eq!(
            hydrate_claim_history(&root, &history, Some(proof(context, &root))),
            Err(RehearsalClaimHydrationError::CompletionProofMismatch)
        );
    }
}

#[test]
fn restored_dispatched_handle_is_only_available_after_full_root_bound_hydration() {
    let context = rehearsal_context(1);
    let root = claim_root(context, claim(1), request(&frozen(), 1.0));
    let one = RehearsalClaimGeneration::first();
    let history = [
        event(
            &root,
            1,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::Prepared,
        ),
        event(
            &root,
            2,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::GradingDispatched,
        ),
    ];
    let handle = hydrate_claim_history(&root, &history, None)
        .unwrap()
        .into_dispatched_handle()
        .unwrap();
    assert!(validate_claim_completion(RehearsalLifecycle::Active, true, handle).is_ok());
}

#[test]
fn backward_observational_timestamps_do_not_weaken_sequence_or_phase_validity() {
    let context = rehearsal_context(1);
    let root = claim_root(context, claim(1), request(&frozen(), 1.0));
    let one = RehearsalClaimGeneration::first();
    let history = [
        root.restore_transition(
            1,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::Prepared,
            ActivityTimestamp::from_unix_millis(100),
            None,
            None,
        ),
        root.restore_transition(
            2,
            operation(11),
            one,
            RehearsalSubmissionClaimPhase::GradingDispatched,
            ActivityTimestamp::from_unix_millis(1),
            None,
            None,
        ),
    ];
    // Sequence, not a wall-clock observation, is the causal authority. Both
    // timestamps remain integrity material in their respective event records.
    assert!(
        hydrate_claim_history(&root, &history, None)
            .unwrap()
            .into_dispatched_handle()
            .is_ok()
    );
}
