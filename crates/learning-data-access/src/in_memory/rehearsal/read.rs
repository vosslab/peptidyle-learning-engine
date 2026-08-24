use super::*;

pub(in crate::in_memory) fn resolve_subject_locked(
    state: &State,
    tenant: TenantId,
    command: &crate::StartRehearsalRouteCommand,
    _assignment: AssignmentId,
) -> Result<question_model::PreviewSubject, StoreError> {
    let evaluation = match &command.subject {
        RehearsalSubjectStart::Synthetic { request } => {
            super::preview_plane::resolve_synthetic_preview_locked(
                state,
                tenant,
                command.course,
                question_model::SyntheticPreviewSubjectRequest {
                    assignment: command.assignment,
                    revision: command.expected_revision,
                    selected_moment: request.selected_moment.clone(),
                    groups: request.groups.clone(),
                    modifiers: request.modifiers.clone(),
                },
            )?
        }
        RehearsalSubjectStart::Derived { candidate } => {
            domain::validate_subject_binding(
                command.assignment,
                command.expected_revision,
                candidate,
            )
            .map_err(|_| StoreError::NotFound)?;
            let audit_memberships: Vec<CourseMembershipReference> = state
                .preview_subject_audits
                .iter()
                .filter_map(|audit| {
                    (audit.tenant == tenant
                        && audit.actor == command.actor
                        && audit.course == command.course
                        && audit.assignment == command.assignment
                        && audit.action == "preview.subject.derived")
                        .then(|| {
                            state
                                .course_membership_references
                                .get(&(tenant, audit.target_membership))
                                .copied()
                        })
                        .flatten()
                })
                .collect();
            let mut matched = None;
            for membership in audit_memberships {
                let resolved = super::preview_plane::resolve_derived_preview_locked(
                    state,
                    tenant,
                    command.course,
                    command.assignment,
                    command.expected_revision,
                    membership,
                    candidate.selected_moment.clone(),
                )?;
                if let PreviewEvaluation::Allowed { subject, .. } = resolved.evaluation
                    && subject == *candidate
                {
                    matched = Some(subject);
                    break;
                }
            }
            return matched.ok_or(StoreError::NotFound);
        }
    };
    match evaluation.evaluation {
        PreviewEvaluation::Allowed { subject, .. } => Ok(subject),
        _ => Err(StoreError::NotFound),
    }
}

pub(in crate::in_memory) fn authorize_assignment(
    state: &State,
    tenant: TenantId,
    actor: UserId,
    course: CourseId,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
) -> Result<(AssignmentId, CourseMembershipId), StoreError> {
    let owner = super::teaching_authority::require_direct_instructor(state, tenant, course, actor)?;
    let assignment_id = state
        .assignments_by_reference
        .get(&(tenant, assignment))
        .copied()
        .ok_or(StoreError::NotFound)?;
    let _record = state
        .assignments
        .get(&(tenant, assignment_id))
        .filter(|record| record.course_id == course)
        .ok_or(StoreError::NotFound)?;
    let current = state
        .assignment_revisions
        .get(&(tenant, assignment_id))
        .copied()
        .ok_or(StoreError::NotFound)?;
    (current.value() == revision.value())
        .then_some((assignment_id, owner))
        .ok_or(StoreError::Conflict)
}

pub(in crate::in_memory) fn authorize_locator(
    state: &State,
    tenant: TenantId,
    locator: crate::RehearsalLocator,
) -> Result<(AssignmentId, CourseMembershipId), StoreError> {
    authorize_assignment(
        state,
        tenant,
        locator.actor,
        locator.course,
        locator.assignment,
        locator.revision,
    )
}
pub(in crate::in_memory) fn authorized_run(
    state: &State,
    tenant: TenantId,
    locator: crate::RehearsalLocator,
    assignment: AssignmentId,
    owner: CourseMembershipId,
) -> Result<&StoredRehearsalRun, StoreError> {
    let id = state
        .rehearsal_by_reference
        .get(&(tenant, locator.rehearsal))
        .copied()
        .ok_or(StoreError::NotFound)?;
    state
        .rehearsal_runs
        .get(&(tenant, id))
        .filter(|run| {
            run.course == locator.course
                && run.assignment_id == assignment
                && run.owner == owner
                && run.actor == locator.actor
                && run.assignment == locator.assignment
                && run.revision == locator.revision
        })
        .ok_or(StoreError::NotFound)
}
pub(in crate::in_memory) fn revision_is_current(
    state: &State,
    tenant: TenantId,
    run: &StoredRehearsalRun,
) -> Result<bool, StoreError> {
    Ok(state
        .assignment_revisions
        .get(&(tenant, run.assignment_id))
        .copied()
        .ok_or(StoreError::NotFound)?
        .value()
        == run.revision.value())
}
pub(in crate::in_memory) fn active_current(
    state: &State,
    tenant: TenantId,
    run: &StoredRehearsalRun,
) -> Result<(), StoreError> {
    (run.lifecycle.is_active() && revision_is_current(state, tenant, run)?)
        .then_some(())
        .ok_or(StoreError::Conflict)
}
pub(in crate::in_memory) fn next_reference(
    state: &mut State,
) -> Result<RehearsalReference, StoreError> {
    state.next_rehearsal_reference = state
        .next_rehearsal_reference
        .checked_add(1)
        .ok_or_else(|| StoreError::Unavailable("rehearsal reference sequence exhausted".into()))?;
    RehearsalReference::new(u64::from(state.next_rehearsal_reference))
        .ok_or_else(|| StoreError::Unavailable("rehearsal reference limit reached".into()))
}
pub(in crate::in_memory) fn fresh_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("rehearsal ID randomness failed: {error}"))
    })
}
pub(in crate::in_memory) fn receipt(
    run: &StoredRehearsalRun,
) -> Result<RehearsalRunReceipt, StoreError> {
    Ok(RehearsalRunReceipt {
        rehearsal: run.reference,
        assignment: run.assignment,
        revision: run.revision,
        lifecycle: run.lifecycle,
        subject: run.subject.clone(),
        started_at: run.started_at,
        updated_at: run.updated_at,
    })
}
pub(in crate::in_memory) fn hydrate_claim(
    state: &State,
    tenant: TenantId,
    run: &StoredRehearsalRun,
    claim: &StoredRehearsalClaim,
) -> Result<(RehearsalClaimRoot, RehearsalSubmissionClaimSnapshot), StoreError> {
    verify_run(state, tenant, run)?;
    let screen = match claim.route_delivery {
        Some(binding) => {
            let generation = state
                .rehearsal_delivery_operations
                .iter()
                .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
                .flat_map(|(_, delivery)| delivery.generations.iter())
                .find(|generation| generation.operation == binding.operation)
                .ok_or_else(|| {
                    StoreError::InvalidRecord("route claim delivery binding is absent".into())
                })?;
            if generation.descriptor.attempt() != claim.attempt {
                return Err(StoreError::InvalidRecord(
                    "route claim attempt disagrees with delivery binding".into(),
                ));
            }
            Some(generation.screen.as_ref().ok_or_else(|| {
                StoreError::InvalidRecord("route claim delivery lacks active screen".into())
            })?)
        }
        None => None,
    };
    let frozen = state
        .rehearsal_frozen_items
        .get(&(tenant, run.id, claim.attempt))
        .ok_or(StoreError::NotFound)?;
    let persisted = match screen {
        Some(screen) => domain::rehearsal::persistence::decode_persisted_claim_root_with_screen(
            run.id,
            claim.claim,
            &claim.fingerprint.as_bytes(),
            &claim.submission_input,
            frozen,
            claim.attempt,
            Some(screen),
        ),
        None => domain::rehearsal::persistence::decode_persisted_claim_root(
            run.id,
            claim.claim,
            &claim.fingerprint.as_bytes(),
            &claim.submission_input,
            frozen,
            claim.attempt,
        ),
    }
    .map_err(|_| StoreError::InvalidRecord("rehearsal claim root is invalid".into()))?;
    let root = RehearsalClaimRoot::verify_persisted(genesis(run, tenant), frozen, persisted)
        .map_err(invalid_claim_root)?;
    let proof = if claim
        .events
        .last()
        .is_some_and(|event| event.phase() == RehearsalSubmissionClaimPhase::Completed)
    {
        let entries = state
            .rehearsal_evidence
            .get(&(tenant, run.id))
            .ok_or(StoreError::NotFound)?;
        let proof = verify_rehearsal_claim_completion_proof(
            genesis(run, tenant),
            run.evidence_head,
            &root,
            &entries.0,
        )
        .map_err(|error| {
            StoreError::InvalidRecord(format!("rehearsal completion proof failure: {error:?}"))
        })?;
        let receipt = claim.receipt.as_ref().ok_or_else(|| {
            StoreError::InvalidRecord("completed rehearsal claim has no immutable receipt".into())
        })?;
        if receipt.outcome != proof.replay_receipt() {
            return Err(StoreError::InvalidRecord(
                "rehearsal completion receipt integrity failure".into(),
            ));
        }
        Some(proof)
    } else {
        if claim.receipt.is_some() {
            return Err(StoreError::InvalidRecord(
                "non-completed rehearsal claim has receipt".into(),
            ));
        }
        None
    };
    let snapshot =
        hydrate_claim_history(&root, &claim.events, proof).map_err(invalid_claim_history)?;
    Ok((root, snapshot))
}

pub(in crate::in_memory) fn invalid_claim_root(
    error: domain::RehearsalClaimRootVerificationError,
) -> StoreError {
    StoreError::InvalidRecord(format!("invalid rehearsal claim root: {error:?}"))
}
pub(in crate::in_memory) fn invalid_claim_history(
    error: domain::RehearsalClaimHydrationError,
) -> StoreError {
    StoreError::InvalidRecord(format!("invalid rehearsal claim history: {error:?}"))
}
pub(in crate::in_memory) fn next_claim_sequence(
    events: &[RehearsalClaimTransitionEvent],
) -> Result<u64, StoreError> {
    u64::try_from(events.len())
        .ok()
        .and_then(|length| length.checked_add(1))
        .ok_or_else(|| {
            StoreError::Unavailable("rehearsal claim transition sequence exhausted".into())
        })
}
pub(in crate::in_memory) fn claim_key_for_handle(
    state: &State,
    tenant: TenantId,
    run: RehearsalRunId,
    claim_id: RehearsalSubmissionClaimId,
) -> Result<
    (
        TenantId,
        RehearsalRunId,
        crate::RehearsalSubmissionIdempotencyKey,
    ),
    StoreError,
> {
    state
        .rehearsal_submission_claims
        .iter()
        .find_map(|(key, claim)| {
            (key.0 == tenant && key.1 == run && claim.claim == claim_id).then(|| key.clone())
        })
        .ok_or(StoreError::NotFound)
}
pub(in crate::in_memory) fn same_prepared_handle(
    left: &PreparedClaimHandle,
    right: &PreparedClaimHandle,
) -> bool {
    left.rehearsal() == right.rehearsal()
        && left.claim() == right.claim()
        && left.operation() == right.operation()
        && left.generation() == right.generation()
        && left.fingerprint() == right.fingerprint()
}
pub(in crate::in_memory) fn same_dispatched_handle(
    left: &DispatchedClaimHandle,
    right: &DispatchedClaimHandle,
) -> bool {
    left.rehearsal() == right.rehearsal()
        && left.claim() == right.claim()
        && left.operation() == right.operation()
        && left.generation() == right.generation()
        && left.fingerprint() == right.fingerprint()
}
pub(in crate::in_memory) fn append_evidence(
    state: &mut State,
    tenant: TenantId,
    run: &StoredRehearsalRun,
    payload: RehearsalEvidencePayload,
) -> Result<(), StoreError> {
    state
        .rehearsal_evidence
        .get(&(tenant, run.id))
        .ok_or(StoreError::NotFound)?;
    let entry = next_evidence_entry(run, payload, state.authoritative_time)?;
    let head = run.evidence_head.advance(&entry.record).map_err(|error| {
        StoreError::InvalidRecord(format!(
            "rehearsal evidence-head advancement failure: {error:?}"
        ))
    })?;
    state
        .rehearsal_evidence
        .get_mut(&(tenant, run.id))
        .ok_or(StoreError::NotFound)?
        .0
        .push(entry);
    state
        .rehearsal_runs
        .get_mut(&(tenant, run.id))
        .ok_or(StoreError::NotFound)?
        .evidence_head = head;
    Ok(())
}

pub(in crate::in_memory) fn next_evidence_entry(
    run: &StoredRehearsalRun,
    payload: RehearsalEvidencePayload,
    recorded_at: question_model::ActivityTimestamp,
) -> Result<RehearsalEvidenceChainEntry, StoreError> {
    let sequence =
        run.evidence_head.length().checked_add(1).ok_or_else(|| {
            StoreError::Unavailable("rehearsal evidence sequence exhausted".into())
        })?;
    let previous = run.evidence_head.digest();
    let kind = match &payload {
        RehearsalEvidencePayload::FrozenItem(_) => RehearsalEvidenceKind::FrozenItem,
        RehearsalEvidencePayload::AcceptedSubmission(_) => {
            RehearsalEvidenceKind::AcceptedSubmission
        }
    };
    let record = RehearsalEvidenceRecord {
        sequence,
        kind,
        previous_digest: Some(previous),
        digest: evidence_entry_digest(
            sequence,
            kind,
            previous,
            private_payload_digest(&payload),
            recorded_at,
        ),
        recorded_at,
    };
    Ok(RehearsalEvidenceChainEntry { record, payload })
}
