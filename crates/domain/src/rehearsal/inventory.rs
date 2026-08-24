//! Backend-neutral verification of rehearsal aggregate inventory ownership.

use std::collections::BTreeMap;

use super::*;

/// One persistence-row view of a frozen attempt.  A vector of these entries,
/// rather than a map, deliberately preserves duplicate storage rows so that a
/// backend cannot erase corruption during hydration.
#[derive(Clone, Copy)]
pub struct RehearsalFrozenInventoryEntry<'a> {
    attempt: question_model::RehearsalAttemptId,
    frozen: &'a RehearsalFrozenItemEvidence,
}

impl<'a> RehearsalFrozenInventoryEntry<'a> {
    pub const fn new(
        attempt: question_model::RehearsalAttemptId,
        frozen: &'a RehearsalFrozenItemEvidence,
    ) -> Self {
        Self { attempt, frozen }
    }
}

/// Opaque ownership witness derived only from a verified completion proof.
/// It intentionally exposes neither a response nor a constructor accepting
/// caller-controlled sequence/digest material.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VerifiedRehearsalAcceptedEvidenceOwner {
    sequence: u32,
    digest: RehearsalEvidenceDigest,
}

/// Converts a completion proof into its accepted-evidence ownership witness.
pub fn rehearsal_accepted_evidence_owner(
    proof: VerifiedRehearsalClaimCompletionProof,
) -> Result<VerifiedRehearsalAcceptedEvidenceOwner, RehearsalInventoryError> {
    let material = proof.completion_material();
    let sequence = u32::try_from(material.accepted_evidence_sequence())
        .map_err(|_| RehearsalInventoryError::AcceptedEvidenceSequenceOutOfRange)?;
    Ok(VerifiedRehearsalAcceptedEvidenceOwner {
        sequence,
        digest: material.accepted_evidence_digest(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalInventoryError {
    FrozenEvidenceMissingStoredAttempt,
    FrozenEvidenceNotOneToOne,
    FrozenEvidencePayloadMismatch,
    StoredFrozenAttemptMissingEvidence,
    AcceptedEvidenceSequenceDuplicated,
    AcceptedEvidenceSequenceOutOfRange,
    CompletedClaimMissingAcceptedEvidence,
    CompletedClaimAcceptedEvidenceDigestMismatch,
    MultipleCompletedClaimsOwnAcceptedEvidence,
    AcceptedEvidenceMissingCompletedClaimOwner,
}

/// Proves the bidirectional aggregate inventory contract after callers have
/// verified the evidence chain and each completed claim proof.  Frozen row
/// entries and evidence retain multiplicity; accepted ownership is accepted
/// only through an opaque witness made from a verified domain proof.
pub fn verify_rehearsal_inventory<'a>(
    frozen_rows: impl IntoIterator<Item = RehearsalFrozenInventoryEntry<'a>>,
    evidence: &[RehearsalEvidenceChainEntry],
    completed_owners: impl IntoIterator<Item = VerifiedRehearsalAcceptedEvidenceOwner>,
) -> Result<(), RehearsalInventoryError> {
    let mut evidence_by_attempt = BTreeMap::new();
    for entry in evidence {
        let RehearsalEvidencePayload::FrozenItem(frozen) = &entry.payload else {
            continue;
        };
        evidence_by_attempt
            .entry(frozen.attempt)
            .or_insert_with(Vec::new)
            .push(frozen);
    }
    let mut rows_by_attempt = BTreeMap::new();
    for row in frozen_rows {
        rows_by_attempt
            .entry(row.attempt)
            .or_insert_with(Vec::new)
            .push(row.frozen);
    }
    for (attempt, frozen_entries) in &evidence_by_attempt {
        let Some(frozen_rows) = rows_by_attempt.get(attempt) else {
            return Err(RehearsalInventoryError::FrozenEvidenceMissingStoredAttempt);
        };
        if frozen_entries.len() != 1 || frozen_rows.len() != 1 {
            return Err(RehearsalInventoryError::FrozenEvidenceNotOneToOne);
        }
        if frozen_entries[0] != frozen_rows[0] {
            return Err(RehearsalInventoryError::FrozenEvidencePayloadMismatch);
        }
    }
    if rows_by_attempt
        .keys()
        .any(|attempt| !evidence_by_attempt.contains_key(attempt))
    {
        return Err(RehearsalInventoryError::StoredFrozenAttemptMissingEvidence);
    }

    let mut accepted_ownership = BTreeMap::new();
    for entry in evidence {
        if matches!(
            entry.payload,
            RehearsalEvidencePayload::AcceptedSubmission(_)
        ) && accepted_ownership
            .insert(entry.record.sequence, 0_usize)
            .is_some()
        {
            return Err(RehearsalInventoryError::AcceptedEvidenceSequenceDuplicated);
        }
    }
    for owner in completed_owners {
        let Some(owners) = accepted_ownership.get_mut(&owner.sequence) else {
            return Err(RehearsalInventoryError::CompletedClaimMissingAcceptedEvidence);
        };
        let entry = evidence
            .iter()
            .find(|entry| entry.record.sequence == owner.sequence)
            .expect("accepted evidence ownership map was built from this evidence");
        if entry.record.digest != owner.digest {
            return Err(RehearsalInventoryError::CompletedClaimAcceptedEvidenceDigestMismatch);
        }
        *owners = owners
            .checked_add(1)
            .ok_or(RehearsalInventoryError::MultipleCompletedClaimsOwnAcceptedEvidence)?;
        if *owners != 1 {
            return Err(RehearsalInventoryError::MultipleCompletedClaimsOwnAcceptedEvidence);
        }
    }
    if accepted_ownership.values().any(|owners| *owners != 1) {
        return Err(RehearsalInventoryError::AcceptedEvidenceMissingCompletedClaimOwner);
    }
    Ok(())
}
