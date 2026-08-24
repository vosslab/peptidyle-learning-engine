use super::super::*;

use helpers::{
    latest_delivery_generation_mut, mutate_frozen_item_for_test, rehash_rehearsal_evidence,
};

mod helpers;

impl MemoryStore {
    /// Deliberately corrupts one private rehearsal record for conformance
    /// tests. This is not part of any Store trait or route composition.
    #[doc(hidden)]
    #[cfg(feature = "test-support")]
    pub fn rehearsal_test_snapshot(
        &self,
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
    ) -> Result<MemoryRehearsalTestSnapshot, StoreError> {
        let state = self.read_state()?;
        let rehearsal_id = state
            .rehearsal_by_reference
            .get(&(tenant, rehearsal))
            .copied()
            .ok_or(StoreError::NotFound)?;
        let run = state
            .rehearsal_runs
            .get(&(tenant, rehearsal_id))
            .ok_or(StoreError::NotFound)?;
        let claims = state
            .rehearsal_submission_claims
            .iter()
            .filter(|((record_tenant, record_rehearsal, _), _)| {
                *record_tenant == tenant && *record_rehearsal == rehearsal_id
            })
            .filter_map(|(_, claim)| {
                claim
                    .events
                    .last()
                    .map(|event| MemoryRehearsalClaimTestSnapshot {
                        phase: event.phase(),
                        generation: event.generation().value(),
                    })
            })
            .collect();
        Ok(MemoryRehearsalTestSnapshot {
            lifecycle: run.lifecycle,
            revision: run.revision,
            claims,
        })
    }

    /// Verifies a retained rehearsal archive without consulting live source
    /// authorization or returning any archived material. This is a private
    /// `test-support` seam for proving that source-context removal preserves
    /// an independently verifiable, tenant-owned aggregate; production code
    /// and Store/browser traits cannot call it.
    #[doc(hidden)]
    #[cfg(feature = "test-support")]
    pub fn verify_rehearsal_archive_for_test(
        &self,
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
    ) -> Result<(), StoreError> {
        let state = self.read_state()?;
        let rehearsal_id = state
            .rehearsal_by_reference
            .get(&(tenant, rehearsal))
            .copied()
            .ok_or(StoreError::NotFound)?;
        let run = state
            .rehearsal_runs
            .get(&(tenant, rehearsal_id))
            .ok_or(StoreError::NotFound)?;
        super::super::rehearsal_integrity::verify_rehearsal_aggregate(&state, tenant, run)
    }

    /// Deliberately corrupts one private rehearsal record for conformance
    #[cfg(feature = "test-support")]
    pub fn corrupt_rehearsal_integrity_for_test(
        &self,
        corruption: MemoryRehearsalIntegrityTestCorruption,
    ) -> Result<(), StoreError> {
        let mut state = self.write_state()?;
        let (tenant, reference) = corruption.binding();
        let rehearsal = state
            .rehearsal_by_reference
            .get(&(tenant, reference))
            .copied()
            .ok_or(StoreError::NotFound)?;
        match corruption {
            MemoryRehearsalIntegrityTestCorruption::RemoveFrozenItem { attempt, .. } => {
                state
                    .rehearsal_frozen_items
                    .remove(&(tenant, rehearsal, attempt))
                    .ok_or(StoreError::NotFound)?;
            }
            MemoryRehearsalIntegrityTestCorruption::DropLatestClaimEvent {
                idempotency_key,
                ..
            } => {
                let claim = state
                    .rehearsal_submission_claims
                    .get_mut(&(tenant, rehearsal, idempotency_key))
                    .ok_or(StoreError::NotFound)?;
                claim.events.pop().ok_or(StoreError::NotFound)?;
            }
            MemoryRehearsalIntegrityTestCorruption::SubstituteRouteClaimDeliveryWithIssuedGeneration {
                claim_idempotency_key,
                replacement_delivery_idempotency_key,
                ..
            } => {
                let claim = state
                    .rehearsal_submission_claims
                    .get(&(tenant, rehearsal, claim_idempotency_key.clone()))
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let run = state
                    .rehearsal_runs
                    .get(&(tenant, rehearsal))
                    .ok_or(StoreError::NotFound)?;
                let (root, _) = super::super::rehearsal::hydrate_claim(&state, tenant, run, &claim)?;
                let bound = claim.route_delivery.ok_or(StoreError::NotFound)?;
                let replacement = state
                    .rehearsal_delivery_operations
                    .get(&(tenant, rehearsal, replacement_delivery_idempotency_key))
                    .and_then(|delivery| {
                        delivery.generations.iter().find(|generation| {
                            generation.operation != bound.operation
                                && generation.descriptor.attempt() == claim.attempt
                                && generation.screen.is_some()
                                && generation.issued_execution.is_some()
                        })
                    })
                    .ok_or(StoreError::NotFound)?;
                let replacement_binding = super::super::rehearsal::route_claim_delivery_binding(
                    &root,
                    super::super::rehearsal::RouteClaimDeliveryBindingInput {
                        attempt: claim.attempt,
                        operation: replacement.operation,
                        screen_digest: replacement.screen_digest.ok_or(StoreError::NotFound)?,
                    },
                );
                state
                    .rehearsal_submission_claims
                    .get_mut(&(tenant, rehearsal, claim_idempotency_key))
                    .ok_or(StoreError::NotFound)?
                    .route_delivery = Some(replacement_binding);
            }
            MemoryRehearsalIntegrityTestCorruption::DropLatestDeliveryEvent {
                idempotency_key,
                ..
            } => {
                let generation =
                    latest_delivery_generation_mut(&mut state, tenant, rehearsal, idempotency_key)?;
                generation.events.pop().ok_or(StoreError::NotFound)?;
            }
            MemoryRehearsalIntegrityTestCorruption::DuplicateLatestDeliveryEvent {
                idempotency_key,
                ..
            } => {
                let generation =
                    latest_delivery_generation_mut(&mut state, tenant, rehearsal, idempotency_key)?;
                let event = generation
                    .events
                    .last()
                    .copied()
                    .ok_or(StoreError::NotFound)?;
                generation.events.push(event);
            }
            MemoryRehearsalIntegrityTestCorruption::AppendIllegalDeliveryEvent {
                idempotency_key,
                ..
            } => {
                let generation =
                    latest_delivery_generation_mut(&mut state, tenant, rehearsal, idempotency_key)?;
                generation
                    .events
                    .last_mut()
                    .ok_or(StoreError::NotFound)?
                    .phase = super::super::rehearsal::StoredRehearsalDeliveryPhase::Prepared;
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceDeliveryJournalHead {
                idempotency_key,
                ..
            } => {
                latest_delivery_generation_mut(&mut state, tenant, rehearsal, idempotency_key)?
                    .journal_head =
                    objects::Sha256Digest::compute(b"corrupt delivery journal head");
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceDeliveryJournalCount {
                idempotency_key,
                ..
            } => {
                latest_delivery_generation_mut(&mut state, tenant, rehearsal, idempotency_key)?
                    .journal_count = 0;
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceDeliveryJournalPhase {
                idempotency_key,
                ..
            } => {
                latest_delivery_generation_mut(&mut state, tenant, rehearsal, idempotency_key)?
                    .journal_phase = super::super::rehearsal::StoredRehearsalDeliveryPhase::Prepared;
            }
            MemoryRehearsalIntegrityTestCorruption::ClearDeliveryGenerations {
                idempotency_key,
                ..
            } => {
                state
                    .rehearsal_delivery_operations
                    .get_mut(&(tenant, rehearsal, idempotency_key))
                    .ok_or(StoreError::NotFound)?
                    .generations
                    .clear();
            }
            MemoryRehearsalIntegrityTestCorruption::DropDeliveryTimingWitness {
                idempotency_key,
                ..
            } => {
                latest_delivery_generation_mut(&mut state, tenant, rehearsal, idempotency_key)?
                    .timing_witness = None;
            }
            MemoryRehearsalIntegrityTestCorruption::TamperIssuedExecutionArtifact {
                idempotency_key,
                tampering,
                ..
            } => {
                let generation =
                    latest_delivery_generation_mut(&mut state, tenant, rehearsal, idempotency_key)?;
                let artifact = generation
                    .issued_execution
                    .as_ref()
                    .ok_or(StoreError::NotFound)?;
                generation.issued_execution = Some(artifact.canonical_test_tamper(tampering)?);
            }
            MemoryRehearsalIntegrityTestCorruption::MutateActiveScreenTitle {
                idempotency_key,
                ..
            } => {
                let screen = latest_delivery_generation_mut(&mut state, tenant, rehearsal, idempotency_key)?
                    .screen
                    .as_mut()
                    .ok_or(StoreError::NotFound)?;
                screen.presentation.title = "corrupt persisted active screen".into();
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceActiveScreenCommitment {
                idempotency_key,
                ..
            } => {
                latest_delivery_generation_mut(&mut state, tenant, rehearsal, idempotency_key)?
                    .screen_digest = Some(question_model::RehearsalPresentationDigestV1::from_bytes([0xA5; 32]));
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceDeliveryRunTimeExhaustedDeadline {
                idempotency_key,
                deadline,
                ..
            } => {
                latest_delivery_generation_mut(&mut state, tenant, rehearsal, idempotency_key)?
                    .run_time_exhausted_deadline = Some(deadline);
            }
            MemoryRehearsalIntegrityTestCorruption::CorruptFrozenSourceChecksum { .. } => {
                state
                    .rehearsal_frozen_source_snapshots
                    .iter_mut()
                    .find(|(key, _)| key.0 == tenant && key.1 == rehearsal)
                    .ok_or(StoreError::NotFound)?
                    .1
                    .checksum = "corrupt-source-checksum".into();
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceFrozenSourceContentWithRehashedChecksum {
                attempt,
                ..
            } => {
                let source = state
                    .rehearsal_frozen_source_snapshots
                    .get_mut(&(tenant, rehearsal, attempt))
                    .ok_or(StoreError::NotFound)?;
                let mut question = source.snapshot.question().clone();
                question
                    .prompt
                    .push(question_model::envelope::ContentBlock::Text {
                    markdown: "corrupt immutable source content".into(),
                    });
                source.snapshot = crate::IssuedQuestionSnapshotV1::new(
                    question,
                    source.snapshot.family_witness().clone(),
                )?;
                source.checksum = source.snapshot.canonical_payload()?.1;
            }
            MemoryRehearsalIntegrityTestCorruption::CorruptFrozenPrivateChecksum { .. } => {
                state
                    .rehearsal_frozen_private_execution
                    .iter_mut()
                    .find(|(key, _)| key.0 == tenant && key.1 == rehearsal)
                    .ok_or(StoreError::NotFound)?
                    .1
                    .checksum = "corrupt-private-checksum".into();
            }
            MemoryRehearsalIntegrityTestCorruption::DropDeliveryFrozenBinding {
                idempotency_key,
                ..
            } => {
                latest_delivery_generation_mut(&mut state, tenant, rehearsal, idempotency_key)?
                    .frozen_binding = None;
            }
            MemoryRehearsalIntegrityTestCorruption::InsertOrphanFrozenSourceSibling { .. } => {
                let ((_, _, attempt), source) = state
                    .rehearsal_frozen_source_snapshots
                    .iter()
                    .find(|(key, _)| key.0 == tenant && key.1 == rehearsal)
                    .map(|(key, value)| (*key, value.clone()))
                    .ok_or(StoreError::NotFound)?;
                state.rehearsal_frozen_source_snapshots.insert(
                    (
                        tenant,
                        rehearsal,
                        question_model::RehearsalAttemptId::from_uuid(uuid::Uuid::from_u128(
                            0xF001,
                        )),
                    ),
                    source,
                );
                let _ = attempt;
            }
            MemoryRehearsalIntegrityTestCorruption::InsertOrphanFrozenPrivateSibling { .. } => {
                let ((_, _, attempt), private) = state
                    .rehearsal_frozen_private_execution
                    .iter()
                    .find(|(key, _)| key.0 == tenant && key.1 == rehearsal)
                    .map(|(key, value)| (*key, value.clone()))
                    .ok_or(StoreError::NotFound)?;
                state.rehearsal_frozen_private_execution.insert(
                    (
                        tenant,
                        rehearsal,
                        question_model::RehearsalAttemptId::from_uuid(uuid::Uuid::from_u128(
                            0xF002,
                        )),
                    ),
                    private,
                );
                let _ = attempt;
            }
            MemoryRehearsalIntegrityTestCorruption::SetFirstFrozenSourceOrdinal {
                ordinal, ..
            } => {
                state
                    .rehearsal_frozen_source_snapshots
                    .iter_mut()
                    .find(|(key, _)| key.0 == tenant && key.1 == rehearsal)
                    .ok_or(StoreError::NotFound)?
                    .1
                    .ordinal = ordinal;
            }
            MemoryRehearsalIntegrityTestCorruption::RedirectDeliveryRetryToPredecessor {
                idempotency_key,
                ..
            } => {
                let retry = state
                    .rehearsal_delivery_retries
                    .get(&(tenant, rehearsal, idempotency_key.clone()))
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let root = state
                    .rehearsal_delivery_operations
                    .get(&(tenant, rehearsal, retry.root_key.clone()))
                    .ok_or(StoreError::NotFound)?;
                let successor_index = root
                    .generations
                    .iter()
                    .position(|generation| generation.operation == retry.operation)
                    .ok_or(StoreError::NotFound)?;
                let predecessor = root
                    .generations
                    .get(successor_index.checked_sub(1).ok_or(StoreError::NotFound)?)
                    .ok_or(StoreError::NotFound)?
                    .operation;
                state
                    .rehearsal_delivery_retries
                    .get_mut(&(tenant, rehearsal, idempotency_key))
                    .ok_or(StoreError::NotFound)?
                    .operation = predecessor;
            }
            MemoryRehearsalIntegrityTestCorruption::DuplicateFrozenEvidence { attempt, .. } => {
                let entries = state
                    .rehearsal_evidence
                    .get_mut(&(tenant, rehearsal))
                    .ok_or(StoreError::NotFound)?;
                let duplicate = entries
                    .0
                    .iter()
                    .find(|entry| {
                        matches!(
                            &entry.payload,
                            domain::RehearsalEvidencePayload::FrozenItem(frozen)
                                if frozen.attempt == attempt
                        )
                    })
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                entries.0.push(duplicate);
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::DuplicateAcceptedEvidence {
                sequence, ..
            } => {
                let entries = state
                    .rehearsal_evidence
                    .get_mut(&(tenant, rehearsal))
                    .ok_or(StoreError::NotFound)?;
                let duplicate = entries
                    .0
                    .iter()
                    .find(|entry| {
                        entry.record.sequence == sequence
                            && matches!(
                                &entry.payload,
                                domain::RehearsalEvidencePayload::AcceptedSubmission(_)
                            )
                    })
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                entries.0.push(duplicate);
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::CopyAcceptedEvidenceFromRehearsal {
                source_rehearsal,
                ..
            } => {
                let source = state
                    .rehearsal_by_reference
                    .get(&(tenant, source_rehearsal))
                    .copied()
                    .ok_or(StoreError::NotFound)?;
                let copied = state
                    .rehearsal_evidence
                    .get(&(tenant, source))
                    .and_then(|entries| {
                        entries.0.iter().find(|entry| {
                            matches!(
                                &entry.payload,
                                domain::RehearsalEvidencePayload::AcceptedSubmission(_)
                            )
                        })
                    })
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                state
                    .rehearsal_evidence
                    .get_mut(&(tenant, rehearsal))
                    .ok_or(StoreError::NotFound)?
                    .0
                    .push(copied);
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::RemoveAllSubmissionClaims { .. } => {
                state.rehearsal_submission_claims.retain(
                    |(record_tenant, record_rehearsal, _), _| {
                        *record_tenant != tenant || *record_rehearsal != rehearsal
                    },
                );
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceAcceptedEvidence {
                source_sequence,
                target_sequence,
                ..
            } => {
                let entries = state
                    .rehearsal_evidence
                    .get_mut(&(tenant, rehearsal))
                    .ok_or(StoreError::NotFound)?;
                let source = entries
                    .0
                    .iter()
                    .find(|entry| {
                        entry.record.sequence == source_sequence
                            && matches!(
                                &entry.payload,
                                domain::RehearsalEvidencePayload::AcceptedSubmission(_)
                            )
                    })
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let target = entries
                    .0
                    .iter_mut()
                    .find(|entry| {
                        entry.record.sequence == target_sequence
                            && matches!(
                                &entry.payload,
                                domain::RehearsalEvidencePayload::AcceptedSubmission(_)
                            )
                    })
                    .ok_or(StoreError::NotFound)?;
                target.payload = source.payload;
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceClaimRequestWithoutFingerprint {
                idempotency_key,
                response,
                ..
            } => {
                let existing = state
                    .rehearsal_submission_claims
                    .get(&(tenant, rehearsal, idempotency_key.clone()))
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let frozen = state
                    .rehearsal_frozen_items
                    .get(&(tenant, rehearsal, existing.attempt))
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let request = domain::RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
                    &frozen,
                    frozen.attempt,
                    response,
                )
                .map_err(|error| StoreError::InvalidRecord(format!("test request: {error:?}")))?;
                let claim = state
                    .rehearsal_submission_claims
                    .get_mut(&(tenant, rehearsal, idempotency_key))
                    .ok_or(StoreError::NotFound)?;
                claim.submission_input = domain::rehearsal::persistence::encode_claim_submission_input(
                    &domain::RehearsalClaimSubmissionInput::durable(request),
                );
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceClaimFingerprintWithoutRequest {
                idempotency_key,
                response,
                ..
            } => {
                let run = state
                    .rehearsal_runs
                    .get(&(tenant, rehearsal))
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let existing = state
                    .rehearsal_submission_claims
                    .get(&(tenant, rehearsal, idempotency_key.clone()))
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let frozen = state
                    .rehearsal_frozen_items
                    .get(&(tenant, rehearsal, existing.attempt))
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let alternate =
                    domain::RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
                        &frozen,
                        frozen.attempt,
                        response,
                    )
                    .map_err(|error| {
                        StoreError::InvalidRecord(format!("test request: {error:?}"))
                    })?;
                let fingerprint = domain::rehearsal_claim_submission_input_fingerprint(
                    super::super::rehearsal_integrity::genesis(&run, tenant),
                    &frozen,
                    &domain::RehearsalClaimSubmissionInput::durable(alternate),
                )
                .map_err(|error| {
                    StoreError::InvalidRecord(format!("test fingerprint: {error:?}"))
                })?;
                let claim = state
                    .rehearsal_submission_claims
                    .get_mut(&(tenant, rehearsal, idempotency_key))
                    .ok_or(StoreError::NotFound)?;
                claim.fingerprint = fingerprint;
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceFrozenContentDigest {
                attempt,
                digest,
                ..
            } => {
                mutate_frozen_item_for_test(&mut state, tenant, rehearsal, attempt, |frozen| {
                    frozen.canonical_content_digest = digest;
                })?;
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceFrozenTimestamp {
                attempt,
                frozen_at,
                ..
            } => {
                mutate_frozen_item_for_test(&mut state, tenant, rehearsal, attempt, |frozen| {
                    frozen.frozen_at = frozen_at;
                })?;
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceFrozenResponseDefinition {
                attempt,
                response_definition,
                ..
            } => {
                mutate_frozen_item_for_test(&mut state, tenant, rehearsal, attempt, |frozen| {
                    frozen.response_definition = response_definition.clone();
                })?;
                rehash_rehearsal_evidence(&mut state, tenant, rehearsal)?;
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceEvidenceHeadDigest {
                digest, ..
            } => {
                let run = state
                    .rehearsal_runs
                    .get_mut(&(tenant, rehearsal))
                    .ok_or(StoreError::NotFound)?;
                run.evidence_head = domain::RehearsalEvidenceHead::from_persisted(
                    digest,
                    run.evidence_head.length(),
                );
            }
            MemoryRehearsalIntegrityTestCorruption::ReplaceEvidenceHeadLength {
                length, ..
            } => {
                let run = state
                    .rehearsal_runs
                    .get_mut(&(tenant, rehearsal))
                    .ok_or(StoreError::NotFound)?;
                run.evidence_head = domain::RehearsalEvidenceHead::from_persisted(
                    run.evidence_head.digest(),
                    length,
                );
            }
        }
        Ok(())
    }
}

/// Non-sensitive lifecycle projection for feature-gated Memory conformance.
#[doc(hidden)]
#[cfg(feature = "test-support")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRehearsalTestSnapshot {
    pub lifecycle: question_model::RehearsalLifecycle,
    pub revision: question_model::TeachingOperationRevision,
    pub claims: Vec<MemoryRehearsalClaimTestSnapshot>,
}

/// One claim's terminal event state, without identity, response, or evidence.
#[doc(hidden)]
#[cfg(feature = "test-support")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRehearsalClaimTestSnapshot {
    pub phase: domain::RehearsalSubmissionClaimPhase,
    pub generation: u32,
}

/// Narrow corrupt-data selector used only by Memory conformance tests.
#[doc(hidden)]
#[cfg(feature = "test-support")]
#[derive(Debug, Clone)]
pub enum MemoryRehearsalIntegrityTestCorruption {
    RemoveFrozenItem {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        attempt: question_model::RehearsalAttemptId,
    },
    DropLatestClaimEvent {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalSubmissionIdempotencyKey,
    },
    /// Replaces a route claim's otherwise valid authenticated binding with a
    /// different issued generation for the same frozen attempt. This proves
    /// the aggregate rejects root-to-generation substitution, not malformed
    /// fields or a missing artifact.
    SubstituteRouteClaimDeliveryWithIssuedGeneration {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        claim_idempotency_key: crate::RehearsalSubmissionIdempotencyKey,
        replacement_delivery_idempotency_key: crate::RehearsalIdempotencyKey,
    },
    DropLatestDeliveryEvent {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
    },
    DuplicateLatestDeliveryEvent {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
    },
    AppendIllegalDeliveryEvent {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
    },
    ReplaceDeliveryJournalHead {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
    },
    ReplaceDeliveryJournalCount {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
    },
    ReplaceDeliveryJournalPhase {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
    },
    ClearDeliveryGenerations {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
    },
    DropDeliveryTimingWitness {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
    },
    TamperIssuedExecutionArtifact {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
        tampering: crate::RehearsalIssuedExecutionTestTampering,
    },
    MutateActiveScreenTitle {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
    },
    ReplaceActiveScreenCommitment {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
    },
    ReplaceDeliveryRunTimeExhaustedDeadline {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
        deadline: question_model::ActivityTimestamp,
    },
    CorruptFrozenSourceChecksum {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
    },
    ReplaceFrozenSourceContentWithRehashedChecksum {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        attempt: question_model::RehearsalAttemptId,
    },
    CorruptFrozenPrivateChecksum {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
    },
    DropDeliveryFrozenBinding {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
    },
    InsertOrphanFrozenSourceSibling {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
    },
    InsertOrphanFrozenPrivateSibling {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
    },
    SetFirstFrozenSourceOrdinal {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        ordinal: usize,
    },
    RedirectDeliveryRetryToPredecessor {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalIdempotencyKey,
    },
    DuplicateFrozenEvidence {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        attempt: question_model::RehearsalAttemptId,
    },
    DuplicateAcceptedEvidence {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        sequence: u32,
    },
    CopyAcceptedEvidenceFromRehearsal {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        source_rehearsal: question_model::RehearsalReference,
    },
    RemoveAllSubmissionClaims {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
    },
    ReplaceAcceptedEvidence {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        source_sequence: u32,
        target_sequence: u32,
    },
    ReplaceClaimRequestWithoutFingerprint {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalSubmissionIdempotencyKey,
        response: question_model::StudentResponse,
    },
    ReplaceClaimFingerprintWithoutRequest {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        idempotency_key: crate::RehearsalSubmissionIdempotencyKey,
        response: question_model::StudentResponse,
    },
    ReplaceFrozenContentDigest {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        attempt: question_model::RehearsalAttemptId,
        digest: question_model::RehearsalEvidenceDigest,
    },
    ReplaceFrozenTimestamp {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        attempt: question_model::RehearsalAttemptId,
        frozen_at: question_model::ActivityTimestamp,
    },
    ReplaceFrozenResponseDefinition {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        attempt: question_model::RehearsalAttemptId,
        response_definition: question_model::ResponseDefinition,
    },
    ReplaceEvidenceHeadDigest {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        digest: question_model::RehearsalEvidenceDigest,
    },
    ReplaceEvidenceHeadLength {
        tenant: TenantId,
        rehearsal: question_model::RehearsalReference,
        length: u32,
    },
}

#[cfg(feature = "test-support")]
impl MemoryRehearsalIntegrityTestCorruption {
    fn binding(&self) -> (TenantId, question_model::RehearsalReference) {
        match self {
            Self::RemoveFrozenItem {
                tenant, rehearsal, ..
            }
            | Self::DropLatestClaimEvent {
                tenant, rehearsal, ..
            }
            | Self::SubstituteRouteClaimDeliveryWithIssuedGeneration {
                tenant, rehearsal, ..
            }
            | Self::DropLatestDeliveryEvent {
                tenant, rehearsal, ..
            }
            | Self::DuplicateLatestDeliveryEvent {
                tenant, rehearsal, ..
            }
            | Self::AppendIllegalDeliveryEvent {
                tenant, rehearsal, ..
            }
            | Self::ReplaceDeliveryJournalHead {
                tenant, rehearsal, ..
            }
            | Self::ReplaceDeliveryJournalCount {
                tenant, rehearsal, ..
            }
            | Self::ReplaceDeliveryJournalPhase {
                tenant, rehearsal, ..
            }
            | Self::ClearDeliveryGenerations {
                tenant, rehearsal, ..
            }
            | Self::DropDeliveryTimingWitness {
                tenant, rehearsal, ..
            }
            | Self::TamperIssuedExecutionArtifact {
                tenant, rehearsal, ..
            }
            | Self::MutateActiveScreenTitle {
                tenant, rehearsal, ..
            }
            | Self::ReplaceActiveScreenCommitment {
                tenant, rehearsal, ..
            }
            | Self::ReplaceDeliveryRunTimeExhaustedDeadline {
                tenant, rehearsal, ..
            }
            | Self::CorruptFrozenSourceChecksum { tenant, rehearsal }
            | Self::ReplaceFrozenSourceContentWithRehashedChecksum {
                tenant, rehearsal, ..
            }
            | Self::CorruptFrozenPrivateChecksum { tenant, rehearsal }
            | Self::DropDeliveryFrozenBinding {
                tenant, rehearsal, ..
            }
            | Self::InsertOrphanFrozenSourceSibling { tenant, rehearsal }
            | Self::InsertOrphanFrozenPrivateSibling { tenant, rehearsal }
            | Self::SetFirstFrozenSourceOrdinal {
                tenant, rehearsal, ..
            }
            | Self::RedirectDeliveryRetryToPredecessor {
                tenant, rehearsal, ..
            }
            | Self::DuplicateFrozenEvidence {
                tenant, rehearsal, ..
            }
            | Self::DuplicateAcceptedEvidence {
                tenant, rehearsal, ..
            }
            | Self::CopyAcceptedEvidenceFromRehearsal {
                tenant, rehearsal, ..
            }
            | Self::RemoveAllSubmissionClaims { tenant, rehearsal }
            | Self::ReplaceAcceptedEvidence {
                tenant, rehearsal, ..
            }
            | Self::ReplaceClaimRequestWithoutFingerprint {
                tenant, rehearsal, ..
            }
            | Self::ReplaceClaimFingerprintWithoutRequest {
                tenant, rehearsal, ..
            }
            | Self::ReplaceFrozenContentDigest {
                tenant, rehearsal, ..
            }
            | Self::ReplaceFrozenTimestamp {
                tenant, rehearsal, ..
            }
            | Self::ReplaceFrozenResponseDefinition {
                tenant, rehearsal, ..
            }
            | Self::ReplaceEvidenceHeadDigest {
                tenant, rehearsal, ..
            }
            | Self::ReplaceEvidenceHeadLength {
                tenant, rehearsal, ..
            } => (*tenant, *rehearsal),
        }
    }
}
