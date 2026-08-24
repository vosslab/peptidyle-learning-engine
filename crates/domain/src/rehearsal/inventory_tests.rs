use super::*;
use question_model::{
    ActivityTimestamp, AssignmentReference, CourseId, CourseMembershipId, ProblemId,
    ProblemVersionRef, RehearsalAttemptId, RehearsalEvidenceDigest, RehearsalEvidenceRecord,
    RehearsalRunId, RehearsalSubmissionClaimId, TenantId, VersionId,
};
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
fn context() -> RehearsalGenesisContext {
    RehearsalGenesisContext {
        rehearsal: RehearsalRunId::from_uuid(id(101)),
        tenant: TenantId::from_uuid(id(102)),
        course: CourseId::from_uuid(id(103)),
        assignment: AssignmentReference::new(1).unwrap(),
        direct_instructor_membership: CourseMembershipId::from_uuid(id(104)),
        revision: TeachingOperationRevision::new(1).unwrap(),
        subject_fingerprint: RehearsalSubjectFingerprint([105; 32]),
    }
}
fn frozen(attempt: u128) -> RehearsalFrozenItemEvidence {
    RehearsalFrozenItemEvidence {
        attempt: RehearsalAttemptId::from_uuid(id(attempt)),
        problem: ProblemVersionRef {
            problem: ProblemId::from_uuid(id(attempt + 1000)),
            version: VersionId::from_uuid(id(attempt + 2000)),
        },
        response_definition: question_model::ResponseDefinition::Numeric {
            tolerance: question_model::answer::NumericTolerance::Exact,
            unit: None,
        },
        canonical_content_digest: RehearsalEvidenceDigest::from_bytes([attempt as u8; 32]),
        frozen_at: ActivityTimestamp::from_unix_millis(1),
    }
}
fn root(
    context: RehearsalGenesisContext,
    frozen: &RehearsalFrozenItemEvidence,
    claim: u128,
    response: f64,
) -> RehearsalClaimRoot {
    let request = RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
        frozen,
        frozen.attempt,
        question_model::StudentResponse::Numeric { value: response },
    )
    .unwrap();
    let fingerprint = rehearsal_submission_request_fingerprint(context, frozen, &request).unwrap();
    RehearsalClaimRoot::verify_persisted(
        context,
        frozen,
        RehearsalPersistedClaimRoot::from_persisted(
            context.rehearsal,
            RehearsalSubmissionClaimId::from_uuid(id(claim)),
            fingerprint,
            request,
        ),
    )
    .unwrap()
}
fn rehash(
    context: RehearsalGenesisContext,
    payloads: Vec<RehearsalEvidencePayload>,
) -> Vec<RehearsalEvidenceChainEntry> {
    let mut previous = evidence_genesis_digest(context);
    payloads
        .into_iter()
        .enumerate()
        .map(|(index, payload)| {
            let sequence = u32::try_from(index + 1).unwrap();
            let recorded_at = ActivityTimestamp::from_unix_millis(i64::from(sequence));
            let entry = RehearsalEvidenceChainEntry {
                record: RehearsalEvidenceRecord {
                    sequence,
                    kind: payload.kind(),
                    previous_digest: Some(previous),
                    digest: evidence_entry_digest(
                        sequence,
                        payload.kind(),
                        previous,
                        private_payload_digest(&payload),
                        recorded_at,
                    ),
                    recorded_at,
                },
                payload,
            };
            previous = entry.record.digest;
            entry
        })
        .collect()
}
fn head(entries: &[RehearsalEvidenceChainEntry]) -> RehearsalEvidenceHead {
    let last = entries.last().unwrap();
    RehearsalEvidenceHead::from_persisted(last.record.digest, last.record.sequence)
}
fn accepted(
    _context: RehearsalGenesisContext,
    frozen: &RehearsalFrozenItemEvidence,
    root: &RehearsalClaimRoot,
) -> RehearsalEvidencePayload {
    let evidence = RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
        root,
        root.submission_input().durable_request().unwrap().clone(),
        frozen,
        question_model::RehearsalPrivateGradingResult::Graded {
            result: question_model::AttemptResult {
                correct: true,
                points_earned: 1.0,
                points_possible: 1.0,
            },
            feedback: question_model::DisclosedFeedback::empty(),
            backend_receipt_reference: question_model::RehearsalBackendReceiptReference::new(
                "native:inventory".into(),
            )
            .unwrap(),
        },
        ActivityTimestamp::from_unix_millis(2),
    )
    .unwrap();
    RehearsalEvidencePayload::AcceptedSubmission(evidence)
}
fn owner(
    context: RehearsalGenesisContext,
    entries: &[RehearsalEvidenceChainEntry],
    root: &RehearsalClaimRoot,
) -> VerifiedRehearsalAcceptedEvidenceOwner {
    rehearsal_accepted_evidence_owner(
        verify_rehearsal_claim_completion_proof(context, head(entries), root, entries).unwrap(),
    )
    .unwrap()
}
fn rows<'a>(items: &'a [RehearsalFrozenItemEvidence]) -> Vec<RehearsalFrozenInventoryEntry<'a>> {
    items
        .iter()
        .map(|item| RehearsalFrozenInventoryEntry::new(item.attempt, item))
        .collect()
}

#[test]
fn inventory_rejects_duplicate_and_orphan_frozen_rows_and_evidence() {
    let context = context();
    let item = frozen(1);
    let frozen_payload = RehearsalEvidencePayload::FrozenItem(item.clone());
    let one = rehash(context, vec![frozen_payload.clone()]);
    assert_eq!(
        verify_rehearsal_inventory(rows(&[item.clone(), item.clone()]), &one, []),
        Err(RehearsalInventoryError::FrozenEvidenceNotOneToOne)
    );
    let duplicate_evidence = rehash(context, vec![frozen_payload.clone(), frozen_payload]);
    assert_eq!(
        verify_rehearsal_inventory(rows(std::slice::from_ref(&item)), &duplicate_evidence, []),
        Err(RehearsalInventoryError::FrozenEvidenceNotOneToOne)
    );
    assert_eq!(
        verify_rehearsal_inventory([], &one, []),
        Err(RehearsalInventoryError::FrozenEvidenceMissingStoredAttempt)
    );
    assert_eq!(
        verify_rehearsal_inventory(rows(std::slice::from_ref(&item)), &[], []),
        Err(RehearsalInventoryError::StoredFrozenAttemptMissingEvidence)
    );
}

#[test]
fn inventory_rejects_frozen_payload_mismatch() {
    let context = context();
    let item = frozen(1);
    let mut changed = item.clone();
    changed.canonical_content_digest = RehearsalEvidenceDigest::from_bytes([99; 32]);
    let evidence = rehash(context, vec![RehearsalEvidencePayload::FrozenItem(item)]);
    assert_eq!(
        verify_rehearsal_inventory(rows(&[changed]), &evidence, []),
        Err(RehearsalInventoryError::FrozenEvidencePayloadMismatch)
    );
}

#[test]
fn inventory_rejects_accepted_sequence_and_owner_corruption() {
    let context = context();
    let item = frozen(1);
    let root = root(context, &item, 11, 1.0);
    let valid = rehash(
        context,
        vec![
            RehearsalEvidencePayload::FrozenItem(item.clone()),
            accepted(context, &item, &root),
        ],
    );
    let valid_owner = owner(context, &valid, &root);
    assert!(
        verify_rehearsal_inventory(rows(std::slice::from_ref(&item)), &valid, [valid_owner])
            .is_ok()
    );

    let duplicate = [valid[0].clone(), valid[1].clone(), valid[1].clone()];
    assert_eq!(
        verify_rehearsal_inventory(rows(std::slice::from_ref(&item)), &duplicate, [valid_owner]),
        Err(RehearsalInventoryError::AcceptedEvidenceSequenceDuplicated)
    );
    assert_eq!(
        verify_rehearsal_inventory(rows(std::slice::from_ref(&item)), &valid, []),
        Err(RehearsalInventoryError::AcceptedEvidenceMissingCompletedClaimOwner)
    );
    assert_eq!(
        verify_rehearsal_inventory(
            rows(std::slice::from_ref(&item)),
            &valid,
            [valid_owner, valid_owner],
        ),
        Err(RehearsalInventoryError::MultipleCompletedClaimsOwnAcceptedEvidence)
    );
}

#[test]
fn inventory_rejects_missing_evidence_and_digest_mismatch_from_verified_owners() {
    let context = context();
    let item = frozen(1);
    let root = root(context, &item, 11, 1.0);
    let valid = rehash(
        context,
        vec![
            RehearsalEvidencePayload::FrozenItem(item.clone()),
            accepted(context, &item, &root),
        ],
    );
    let owner = owner(context, &valid, &root);
    let missing = [valid[0].clone()];
    assert_eq!(
        verify_rehearsal_inventory(rows(std::slice::from_ref(&item)), &missing, [owner]),
        Err(RehearsalInventoryError::CompletedClaimMissingAcceptedEvidence)
    );
    let mut digest_changed = valid.clone();
    digest_changed[1].record.digest = RehearsalEvidenceDigest::from_bytes([77; 32]);
    assert_eq!(
        verify_rehearsal_inventory(rows(std::slice::from_ref(&item)), &digest_changed, [owner]),
        Err(RehearsalInventoryError::CompletedClaimAcceptedEvidenceDigestMismatch)
    );
}

#[test]
fn inventory_accepts_multiple_verified_claims() {
    let context = context();
    let first = frozen(1);
    let second = frozen(2);
    let first_root = root(context, &first, 11, 1.0);
    let second_root = root(context, &second, 12, 2.0);
    let evidence = rehash(
        context,
        vec![
            RehearsalEvidencePayload::FrozenItem(first.clone()),
            accepted(context, &first, &first_root),
            RehearsalEvidencePayload::FrozenItem(second.clone()),
            accepted(context, &second, &second_root),
        ],
    );
    assert!(
        verify_rehearsal_inventory(
            rows(&[first, second]),
            &evidence,
            [
                owner(context, &evidence, &first_root),
                owner(context, &evidence, &second_root)
            ],
        )
        .is_ok()
    );
}
