//! Aggregate verification and all-or-nothing lifecycle staging for rehearsal.

use domain::{
    RehearsalFrozenInventoryEntry, RehearsalGenesisContext, RehearsalSubmissionClaimPhase,
    RehearsalTerminalTransition, apply_terminal_transition, hydrate_claim_history,
    rehearsal_accepted_evidence_owner, verify_evidence_chain, verify_rehearsal_inventory,
};
use question_model::{AssignmentId, RehearsalRunId, TenantId};

use std::collections::BTreeSet;

use super::rehearsal::{StoredRehearsalRun, hydrate_claim, next_claim_sequence};
use super::*;

pub(super) fn verify_run(
    state: &State,
    tenant: TenantId,
    run: &StoredRehearsalRun,
) -> Result<(), StoreError> {
    let entries = state
        .rehearsal_evidence
        .get(&(tenant, run.id))
        .ok_or(StoreError::NotFound)?;
    verify_evidence_chain(genesis(run, tenant), run.evidence_head, &entries.0).map_err(|error| {
        StoreError::InvalidRecord(format!("rehearsal evidence integrity failure: {error:?}"))
    })
}

pub(super) fn verify_rehearsal_aggregate(
    state: &State,
    tenant: TenantId,
    run: &StoredRehearsalRun,
) -> Result<(), StoreError> {
    verify_run(state, tenant, run)?;
    let entries = state
        .rehearsal_evidence
        .get(&(tenant, run.id))
        .ok_or(StoreError::NotFound)?;
    let frozen_rows = state
        .rehearsal_frozen_items
        .iter()
        .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
        .map(|(key, frozen)| RehearsalFrozenInventoryEntry::new(key.2, frozen));
    let completed_owners = completed_claim_owners(state, tenant, run, &entries.0)?;
    verify_rehearsal_inventory(frozen_rows, &entries.0, completed_owners).map_err(|error| {
        StoreError::InvalidRecord(format!("rehearsal inventory integrity failure: {error:?}"))
    })?;
    verify_frozen_material_and_delivery_bindings(state, tenant, run, &entries.0)?;
    for delivery in state
        .rehearsal_delivery_operations
        .iter()
        .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
        .map(|(_, delivery)| delivery)
    {
        if delivery.generations.is_empty() {
            return Err(StoreError::InvalidRecord(
                "rehearsal delivery root has no generations".into(),
            ));
        }
        for generation in &delivery.generations {
            let phase = generation.phase()?;
            if let Some(screen) = &generation.screen {
                let commitment = screen.commitment().map_err(|error| {
                    StoreError::InvalidRecord(format!("invalid rehearsal active screen: {error:?}"))
                })?;
                if generation.screen_digest != Some(commitment) {
                    return Err(StoreError::InvalidRecord(
                        "rehearsal active screen commitment mismatch".into(),
                    ));
                }
            }
            if let Some(witness) = generation.timing_witness {
                let source = state
                    .rehearsal_frozen_source_snapshots
                    .get(&(tenant, run.id, generation.descriptor.attempt()))
                    .ok_or_else(|| {
                        StoreError::InvalidRecord(
                            "timed delivery lacks frozen source snapshot".into(),
                        )
                    })?;
                let (_, snapshot_digest) = source.snapshot.canonical_payload_bytes()?;
                domain::verify_rehearsal_timing_witness(
                    domain::RehearsalTimingInputsV1 {
                        subject_fingerprint: run.fingerprint,
                        frozen_snapshot_digest: question_model::RehearsalEvidenceDigest::from_bytes(
                            *snapshot_digest.as_bytes(),
                        ),
                        timing_policy: source.snapshot.question().timing_policy,
                        subject_time_limit_seconds: run.subject.policy.time_limit_seconds().value,
                        run_started_at: run.started_at,
                        issued_at: witness.issued_at(),
                    },
                    witness,
                )
                .map_err(|error| {
                    StoreError::InvalidRecord(format!(
                        "rehearsal delivery timing witness integrity failure: {error}"
                    ))
                })?;
            }
            if matches!(
                phase,
                super::rehearsal::StoredRehearsalDeliveryPhase::Dispatched
                    | super::rehearsal::StoredRehearsalDeliveryPhase::Completed
                    | super::rehearsal::StoredRehearsalDeliveryPhase::Expired
            ) && generation.timing_witness.is_none()
            {
                return Err(StoreError::InvalidRecord(
                    "dispatched delivery lacks timing witness".into(),
                ));
            }
            if phase
                == super::rehearsal::StoredRehearsalDeliveryPhase::RunTimeExhaustedBeforeDispatch
            {
                let source = state
                    .rehearsal_frozen_source_snapshots
                    .get(&(tenant, run.id, generation.descriptor.attempt()))
                    .ok_or_else(|| {
                        StoreError::InvalidRecord("run-time terminal lacks source snapshot".into())
                    })?;
                let (_, snapshot_digest) = source.snapshot.canonical_payload_bytes()?;
                let observed_at = generation
                    .events
                    .last()
                    .ok_or_else(|| {
                        StoreError::InvalidRecord("run-time terminal lacks event".into())
                    })?
                    .recorded_at;
                let decision =
                    domain::decide_rehearsal_timing_dispatch(domain::RehearsalTimingInputsV1 {
                        subject_fingerprint: run.fingerprint,
                        frozen_snapshot_digest: question_model::RehearsalEvidenceDigest::from_bytes(
                            *snapshot_digest.as_bytes(),
                        ),
                        timing_policy: source.snapshot.question().timing_policy,
                        subject_time_limit_seconds: run.subject.policy.time_limit_seconds().value,
                        run_started_at: run.started_at,
                        issued_at: observed_at,
                    })
                    .map_err(|error| {
                        StoreError::InvalidRecord(format!(
                            "run-time terminal timing derivation failed: {error}"
                        ))
                    })?;
                let domain::RehearsalTimingDispatchDecisionV1::RunTimeExhausted { deadline } =
                    decision
                else {
                    return Err(StoreError::InvalidRecord("run-time terminal event is not justified by its committed observation time".into()));
                };
                if generation.run_time_exhausted_deadline != Some(deadline) {
                    return Err(StoreError::InvalidRecord(
                        "run-time terminal deadline does not match committed timing decision"
                            .into(),
                    ));
                }
            }
        }
    }
    verify_delivery_retry_index(state, tenant, run)?;
    Ok(())
}

/// Retry idempotency is an aggregate index, not an independent authority.
/// Every entry must point into this run's append-only delivery history and
/// describe precisely either the expired predecessor it terminally replays or
/// the immediately following retry generation it replays.
fn verify_delivery_retry_index(
    state: &State,
    tenant: TenantId,
    run: &StoredRehearsalRun,
) -> Result<(), StoreError> {
    for ((entry_tenant, entry_run, _key), retry) in &state.rehearsal_delivery_retries {
        if *entry_tenant != tenant || *entry_run != run.id {
            continue;
        }
        let root = state
            .rehearsal_delivery_operations
            .get(&(tenant, run.id, retry.root_key.clone()))
            .ok_or_else(|| {
                StoreError::InvalidRecord("rehearsal retry points to absent delivery root".into())
            })?;
        let index = root
            .generations
            .iter()
            .position(|generation| generation.operation == retry.operation)
            .ok_or_else(|| {
                StoreError::InvalidRecord(
                    "rehearsal retry points to absent delivery generation".into(),
                )
            })?;
        let generation = &root.generations[index];
        if retry.terminal_predecessor {
            if index + 1 != root.generations.len()
                || generation.phase()? != super::rehearsal::StoredRehearsalDeliveryPhase::Expired
            {
                return Err(StoreError::InvalidRecord(
                    "terminal rehearsal retry does not point to the current expired predecessor"
                        .into(),
                ));
            }
        } else {
            let predecessor = index
                .checked_sub(1)
                .and_then(|previous| root.generations.get(previous))
                .ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "rehearsal retry successor lacks expired predecessor".into(),
                    )
                })?;
            if predecessor.phase()? != super::rehearsal::StoredRehearsalDeliveryPhase::Expired {
                return Err(StoreError::InvalidRecord(
                    "rehearsal retry successor is not linked to an expired predecessor".into(),
                ));
            }
        }
    }
    Ok(())
}

/// Verifies the one frozen-material authority shared by route material checks,
/// expiry/retry, and sealed execution.  No path may trust a delivery descriptor
/// or cached binding without re-establishing this proof.
pub(super) fn verify_frozen_material_and_delivery_bindings(
    state: &State,
    tenant: TenantId,
    run: &StoredRehearsalRun,
    entries: &[domain::RehearsalEvidenceChainEntry],
) -> Result<(), StoreError> {
    let frozen_items = state
        .rehearsal_frozen_items
        .iter()
        .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
        .collect::<Vec<_>>();
    if frozen_items.is_empty() {
        return Err(StoreError::InvalidRecord(
            "rehearsal has no immutable delivery material".into(),
        ));
    }
    let frozen_keys = frozen_items
        .iter()
        .map(|(key, _)| **key)
        .collect::<std::collections::BTreeSet<_>>();
    let source_keys = state
        .rehearsal_frozen_source_snapshots
        .keys()
        .filter(|key| key.0 == tenant && key.1 == run.id)
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let private_keys = state
        .rehearsal_frozen_private_execution
        .keys()
        .filter(|key| key.0 == tenant && key.1 == run.id)
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if source_keys != frozen_keys || private_keys != frozen_keys {
        return Err(StoreError::InvalidRecord(
            "frozen material sibling key sets do not match".into(),
        ));
    }
    let evidence_attempts = entries
        .iter()
        .filter_map(|entry| match &entry.payload {
            domain::RehearsalEvidencePayload::FrozenItem(frozen) => Some(frozen.attempt),
            _ => None,
        })
        .collect::<Vec<_>>();
    if evidence_attempts.len() != frozen_keys.len() {
        return Err(StoreError::InvalidRecord(
            "frozen material evidence cardinality does not match".into(),
        ));
    }
    for (ordinal, attempt) in evidence_attempts.into_iter().enumerate() {
        let key = (tenant, run.id, attempt);
        let source = state
            .rehearsal_frozen_source_snapshots
            .get(&key)
            .ok_or_else(|| {
                StoreError::InvalidRecord("frozen evidence lacks source snapshot".into())
            })?;
        if source.ordinal != ordinal {
            return Err(StoreError::InvalidRecord(
                "frozen source snapshot ordinal does not match evidence order".into(),
            ));
        }
    }
    for (key, frozen) in &frozen_items {
        let source = state
            .rehearsal_frozen_source_snapshots
            .get(key)
            .ok_or_else(|| {
                StoreError::InvalidRecord("rehearsal source snapshot is absent".into())
            })?;
        let private = state
            .rehearsal_frozen_private_execution
            .get(key)
            .ok_or_else(|| {
                StoreError::InvalidRecord("rehearsal private execution is absent".into())
            })?;
        let (_, snapshot_checksum) = source.snapshot.canonical_payload()?;
        if source.checksum != snapshot_checksum {
            return Err(StoreError::InvalidRecord(
                "rehearsal source snapshot checksum mismatch".into(),
            ));
        }
        if private.checksum != super::rehearsal::private_execution_checksum(&private.execution)? {
            return Err(StoreError::InvalidRecord(
                "rehearsal private execution checksum mismatch".into(),
            ));
        }
        let question = source.snapshot.question();
        let source_content_digest =
            super::rehearsal::canonical_rehearsal_question_content_digest(question)?;
        if question.problem != frozen.problem.problem
            || question.version != frozen.problem.version
            || question.response != frozen.response_definition
            || source_content_digest != frozen.canonical_content_digest
        {
            return Err(StoreError::InvalidRecord(
                "rehearsal source snapshot does not match frozen commitment".into(),
            ));
        }
    }
    for generation in state
        .rehearsal_delivery_operations
        .iter()
        .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
        .flat_map(|(_, delivery)| delivery.generations.iter())
    {
        let phase = generation.phase()?;
        let frozen = state
            .rehearsal_frozen_items
            .get(&(tenant, run.id, generation.descriptor.attempt()))
            .ok_or_else(|| {
                StoreError::InvalidRecord("delivery selected absent frozen item".into())
            })?;
        if generation.descriptor.problem() != frozen.problem
            || generation.descriptor.response_definition() != &frozen.response_definition
            || generation.descriptor.frozen_content_digest() != frozen.canonical_content_digest
            || generation
                .frozen_binding
                .as_ref()
                .is_some_and(|bound| bound != frozen)
        {
            return Err(StoreError::InvalidRecord(
                "delivery generation does not match immutable frozen material".into(),
            ));
        }
        if matches!(
            phase,
            super::rehearsal::StoredRehearsalDeliveryPhase::Dispatched
                | super::rehearsal::StoredRehearsalDeliveryPhase::Completed
                | super::rehearsal::StoredRehearsalDeliveryPhase::Expired
        ) && generation.frozen_binding.is_none()
        {
            return Err(StoreError::InvalidRecord(
                "dispatched delivery lacks immutable frozen binding".into(),
            ));
        }
        if let Some(artifact) = &generation.issued_execution {
            let source = state
                .rehearsal_frozen_source_snapshots
                .get(&(tenant, run.id, generation.descriptor.attempt()))
                .ok_or_else(|| {
                    StoreError::InvalidRecord("issued execution lacks frozen source".into())
                })?;
            let private = state
                .rehearsal_frozen_private_execution
                .get(&(tenant, run.id, generation.descriptor.attempt()))
                .ok_or_else(|| {
                    StoreError::InvalidRecord("issued execution lacks private material".into())
                })?;
            let private_digest =
                super::rehearsal::decode_rehearsal_private_checksum(&private.checksum)?;
            let work = crate::SealedRehearsalDeliveryIssueWork::new(
                generation.operation,
                generation.descriptor.clone(),
                source.snapshot.clone(),
                private.execution.clone(),
                crate::RehearsalOperationDigest::from_bytes(private_digest),
            );
            artifact.decode_for_work(&work)?;
        }
    }
    for claim in state
        .rehearsal_submission_claims
        .iter()
        .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
        .map(|(_, claim)| claim)
    {
        if let Some(binding) = claim.route_delivery {
            let generation = state
                .rehearsal_delivery_operations
                .iter()
                .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
                .flat_map(|(_, delivery)| delivery.generations.iter())
                .find(|generation| generation.operation == binding.operation)
                .ok_or_else(|| {
                    StoreError::InvalidRecord("route claim delivery binding is absent".into())
                })?;
            if generation.screen_digest != Some(binding.screen_digest)
                || generation.screen.is_none()
                || generation.issued_execution.is_none()
            {
                return Err(StoreError::InvalidRecord(
                    "route claim delivery binding is not an issued screen".into(),
                ));
            }
            let (root, _) = super::rehearsal::hydrate_claim(state, tenant, run, claim)?;
            let expected_binding = super::rehearsal::route_claim_delivery_binding(
                &root,
                super::rehearsal::RouteClaimDeliveryBindingInput {
                    attempt: claim.attempt,
                    operation: binding.operation,
                    screen_digest: binding.screen_digest,
                },
            );
            if binding != expected_binding {
                return Err(StoreError::InvalidRecord(
                    "route claim delivery binding digest is invalid".into(),
                ));
            }
            let first = claim.events.first().ok_or_else(|| {
                StoreError::InvalidRecord("route claim has no initial prepared event".into())
            })?;
            if first.sequence() != 1
                || first.phase() != domain::RehearsalSubmissionClaimPhase::Prepared
                || first.generation() != domain::RehearsalClaimGeneration::first()
                || first.operation()
                    != super::rehearsal::route_claim_initial_grade_operation(binding)
            {
                return Err(StoreError::InvalidRecord(
                    "route claim initial operation is not binding-derived".into(),
                ));
            }
            if generation.descriptor.attempt() != claim.attempt
                || root.submission_input().presentation_commitment() != Some(binding.screen_digest)
            {
                return Err(StoreError::InvalidRecord(
                    "route claim delivery does not match sealed attempt".into(),
                ));
            }
        }
    }
    Ok(())
}

/// Hydrates and verifies each completed claim before emitting the only
/// accepted-evidence ownership witness that the shared inventory verifier
/// accepts.  The Store never supplies arbitrary sequence/digest tuples.
fn completed_claim_owners(
    state: &State,
    tenant: TenantId,
    run: &StoredRehearsalRun,
    entries: &[domain::RehearsalEvidenceChainEntry],
) -> Result<Vec<domain::VerifiedRehearsalAcceptedEvidenceOwner>, StoreError> {
    let mut owners = Vec::new();
    for claim in state
        .rehearsal_submission_claims
        .iter()
        .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
        .map(|(_, claim)| claim)
    {
        let (root, snapshot) = hydrate_claim(state, tenant, run, claim)?;
        if snapshot.state() != domain::RehearsalSubmissionClaimState::Completed {
            continue;
        }
        let proof = domain::verify_rehearsal_claim_completion_proof(
            genesis(run, tenant),
            run.evidence_head,
            &root,
            entries,
        )
        .map_err(|error| {
            StoreError::InvalidRecord(format!(
                "rehearsal completed claim proof failure: {error:?}"
            ))
        })?;
        owners.push(rehearsal_accepted_evidence_owner(proof).map_err(|error| {
            StoreError::InvalidRecord(format!(
                "rehearsal completed claim ownership witness failure: {error:?}"
            ))
        })?);
    }
    Ok(owners)
}

pub(super) fn genesis(run: &StoredRehearsalRun, tenant: TenantId) -> RehearsalGenesisContext {
    RehearsalGenesisContext {
        rehearsal: run.id,
        tenant,
        course: run.course,
        assignment: run.assignment,
        direct_instructor_membership: run.owner,
        revision: run.revision,
        subject_fingerprint: run.fingerprint,
    }
}

pub(super) fn invalidate_assignment_rehearsals(
    state: &mut State,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<(), StoreError> {
    let matching_ids = state
        .rehearsal_runs
        .iter()
        .filter_map(|((record_tenant, id), run)| {
            (*record_tenant == tenant && run.assignment_id == assignment).then_some(*id)
        })
        .collect::<Vec<_>>();
    for id in &matching_ids {
        let run = state
            .rehearsal_runs
            .get(&(tenant, *id))
            .ok_or(StoreError::NotFound)?;
        verify_rehearsal_aggregate(state, tenant, run)?;
    }
    let active_ids = matching_ids
        .iter()
        .copied()
        .filter(|id| {
            state
                .rehearsal_runs
                .get(&(tenant, *id))
                .is_some_and(|run| run.lifecycle.is_active())
        })
        .collect::<Vec<_>>();
    let mut staged = state.clone();
    for id in active_ids {
        transition_locked_in_place(
            &mut staged,
            tenant,
            id,
            RehearsalTerminalTransition::DiscardStaleRevision,
        )?;
    }
    *state = staged;
    Ok(())
}

/// Fences active rehearsal aggregates before their historical assignment
/// source is removed.  This is deliberately private to the retention owner:
/// it is not a general route capability and it never deletes the retained
/// evidence archive.
///
/// All matching aggregates are verified before a clone is changed.  The
/// caller may therefore compose this with a larger clone-and-publish source
/// deletion transaction without exposing a partly-fenced archive.
pub(super) fn fence_assignment_rehearsals_for_source_removal(
    state: &mut State,
    tenant: TenantId,
    assignments: &BTreeSet<AssignmentId>,
) -> Result<(), StoreError> {
    let matching_ids = state
        .rehearsal_runs
        .iter()
        .filter_map(|((record_tenant, id), run)| {
            (*record_tenant == tenant && assignments.contains(&run.assignment_id)).then_some(*id)
        })
        .collect::<Vec<_>>();
    for id in &matching_ids {
        let run = state
            .rehearsal_runs
            .get(&(tenant, *id))
            .ok_or(StoreError::NotFound)?;
        verify_rehearsal_aggregate(state, tenant, run)?;
    }
    let active_ids = matching_ids
        .iter()
        .copied()
        .filter(|id| {
            state
                .rehearsal_runs
                .get(&(tenant, *id))
                .is_some_and(|run| run.lifecycle.is_active())
        })
        .collect::<Vec<_>>();
    let mut staged = state.clone();
    for id in active_ids {
        transition_locked_in_place(
            &mut staged,
            tenant,
            id,
            RehearsalTerminalTransition::DiscardSourceContextRemoved,
        )?;
    }
    *state = staged;
    Ok(())
}

pub(super) fn transition_locked(
    state: &mut State,
    tenant: TenantId,
    id: RehearsalRunId,
    transition: RehearsalTerminalTransition,
) -> Result<(), StoreError> {
    let mut staged = state.clone();
    transition_locked_in_place(&mut staged, tenant, id, transition)?;
    *state = staged;
    Ok(())
}

fn transition_locked_in_place(
    state: &mut State,
    tenant: TenantId,
    id: RehearsalRunId,
    transition: RehearsalTerminalTransition,
) -> Result<(), StoreError> {
    let run = state
        .rehearsal_runs
        .get(&(tenant, id))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    verify_rehearsal_aggregate(state, tenant, &run)?;
    let lifecycle = apply_terminal_transition(run.lifecycle, transition).map_err(|error| {
        StoreError::InvalidRecord(format!("invalid rehearsal transition: {error:?}"))
    })?;
    let clears_active_uniqueness =
        transition == RehearsalTerminalTransition::DiscardSourceContextRemoved;
    let now = state.authoritative_time;
    let keys = state
        .rehearsal_submission_claims
        .iter()
        .filter(|(key, _)| key.0 == tenant && key.1 == id)
        .map(|(key, claim)| (key.clone(), claim.clone()))
        .collect::<Vec<_>>();
    let mut staged_events = Vec::new();
    for (key, claim) in keys {
        let (root, snapshot) = hydrate_claim(state, tenant, &run, &claim)?;
        let phase = match snapshot.state() {
            domain::RehearsalSubmissionClaimState::Prepared
            | domain::RehearsalSubmissionClaimState::GradingDispatched => match transition {
                RehearsalTerminalTransition::DiscardStaleRevision => {
                    RehearsalSubmissionClaimPhase::RevokedStaleRevision
                }
                RehearsalTerminalTransition::DiscardSourceContextRemoved => {
                    RehearsalSubmissionClaimPhase::RevokedSourceContextRemoved
                }
                _ => RehearsalSubmissionClaimPhase::RevokedTerminalLifecycle,
            },
            _ => continue,
        };
        let event = root.restore_transition(
            next_claim_sequence(&claim.events)?,
            snapshot.operation(),
            snapshot.generation(),
            phase,
            now,
            None,
            None,
        );
        let mut events = claim.events.clone();
        events.push(event);
        hydrate_claim_history(&root, &events, None)
            .map_err(super::rehearsal::invalid_claim_history)?;
        staged_events.push((key, event));
    }
    let stored = state
        .rehearsal_runs
        .get_mut(&(tenant, id))
        .ok_or(StoreError::NotFound)?;
    stored.lifecycle = lifecycle;
    stored.updated_at = now;
    if clears_active_uniqueness {
        state
            .rehearsal_active_by_owner
            .retain(|_, active_id| *active_id != id);
    }
    for (key, event) in staged_events {
        state
            .rehearsal_submission_claims
            .get_mut(&key)
            .ok_or(StoreError::NotFound)?
            .events
            .push(event);
    }
    Ok(())
}
