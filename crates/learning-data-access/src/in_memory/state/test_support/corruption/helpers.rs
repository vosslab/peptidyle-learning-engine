use super::super::super::*;

#[cfg(feature = "test-support")]
pub(super) fn latest_delivery_generation_mut(
    state: &mut State,
    tenant: TenantId,
    rehearsal: question_model::RehearsalRunId,
    idempotency_key: crate::RehearsalIdempotencyKey,
) -> Result<&mut super::rehearsal::StoredRehearsalDeliveryGeneration, StoreError> {
    state
        .rehearsal_delivery_operations
        .get_mut(&(tenant, rehearsal, idempotency_key))
        .and_then(|operation| operation.generations.last_mut())
        .ok_or(StoreError::NotFound)
}

#[cfg(feature = "test-support")]
pub(super) fn mutate_frozen_item_for_test(
    state: &mut State,
    tenant: TenantId,
    rehearsal: question_model::RehearsalRunId,
    attempt: question_model::RehearsalAttemptId,
    mutate: impl Fn(&mut question_model::RehearsalFrozenItemEvidence),
) -> Result<(), StoreError> {
    let persisted = state
        .rehearsal_frozen_items
        .get_mut(&(tenant, rehearsal, attempt))
        .ok_or(StoreError::NotFound)?;
    mutate(persisted);
    let evidence = state
        .rehearsal_evidence
        .get_mut(&(tenant, rehearsal))
        .ok_or(StoreError::NotFound)?;
    let entry = evidence
        .0
        .iter_mut()
        .find(|entry| {
            matches!(
                &entry.payload,
                domain::RehearsalEvidencePayload::FrozenItem(frozen) if frozen.attempt == attempt
            )
        })
        .ok_or(StoreError::NotFound)?;
    let domain::RehearsalEvidencePayload::FrozenItem(frozen) = &mut entry.payload else {
        return Err(StoreError::NotFound);
    };
    mutate(frozen);
    Ok(())
}

#[cfg(feature = "test-support")]
pub(super) fn rehash_rehearsal_evidence(
    state: &mut State,
    tenant: TenantId,
    rehearsal: question_model::RehearsalRunId,
) -> Result<(), StoreError> {
    let run = state
        .rehearsal_runs
        .get(&(tenant, rehearsal))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let entries = state
        .rehearsal_evidence
        .get_mut(&(tenant, rehearsal))
        .ok_or(StoreError::NotFound)?;
    let mut previous = domain::evidence_genesis_digest(
        super::super::super::rehearsal_integrity::genesis(&run, tenant),
    );
    for (index, entry) in entries.0.iter_mut().enumerate() {
        let sequence = u32::try_from(index + 1).map_err(|_| {
            StoreError::Unavailable("rehearsal test evidence sequence exhausted".into())
        })?;
        let kind = match &entry.payload {
            domain::RehearsalEvidencePayload::FrozenItem(_) => {
                question_model::RehearsalEvidenceKind::FrozenItem
            }
            domain::RehearsalEvidencePayload::AcceptedSubmission(_) => {
                question_model::RehearsalEvidenceKind::AcceptedSubmission
            }
        };
        entry.record.sequence = sequence;
        entry.record.kind = kind;
        entry.record.previous_digest = Some(previous);
        entry.record.digest = domain::evidence_entry_digest(
            sequence,
            kind,
            previous,
            domain::private_payload_digest(&entry.payload),
            entry.record.recorded_at,
        );
        previous = entry.record.digest;
    }
    Ok(())
}
