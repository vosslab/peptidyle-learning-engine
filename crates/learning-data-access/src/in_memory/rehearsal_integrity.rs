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
    let ids = state
        .rehearsal_runs
        .iter()
        .filter_map(|((record_tenant, id), run)| {
            (*record_tenant == tenant
                && run.assignment_id == assignment
                && run.lifecycle.is_active())
            .then_some(*id)
        })
        .collect::<Vec<_>>();
    for id in &ids {
        let run = state
            .rehearsal_runs
            .get(&(tenant, *id))
            .ok_or(StoreError::NotFound)?;
        verify_rehearsal_aggregate(state, tenant, run)?;
    }
    let mut staged = state.clone();
    for id in ids {
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
    let ids = state
        .rehearsal_runs
        .iter()
        .filter_map(|((record_tenant, id), run)| {
            (*record_tenant == tenant
                && assignments.contains(&run.assignment_id)
                && run.lifecycle.is_active())
            .then_some(*id)
        })
        .collect::<Vec<_>>();
    for id in &ids {
        let run = state
            .rehearsal_runs
            .get(&(tenant, *id))
            .ok_or(StoreError::NotFound)?;
        verify_rehearsal_aggregate(state, tenant, run)?;
    }
    let mut staged = state.clone();
    for id in ids {
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
