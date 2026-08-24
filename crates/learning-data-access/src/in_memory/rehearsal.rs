//! Memory implementation of the isolated WP-PROF-T4 rehearsal aggregate.

use async_trait::async_trait;
use domain::{
    DispatchedClaimHandle, PreparedClaimHandle, RehearsalClaimRoot, RehearsalClaimTransitionEvent,
    RehearsalEvidenceChainEntry, RehearsalEvidencePayload, RehearsalLifecycleSnapshot,
    RehearsalPersistedClaimRoot, RehearsalSubmissionClaimDecision, RehearsalSubmissionClaimPhase,
    RehearsalSubmissionClaimSnapshot, RehearsalTerminalTransition,
    RehearsalValidatedSubmissionRequest, abandon_rehearsal_submission_before_dispatch,
    decide_start, decide_submission_claim, evidence_entry_digest,
    fingerprint_resolved_preview_subject, hydrate_claim_history,
    mark_rehearsal_submission_dispatched, private_payload_digest,
    rehearsal_submission_request_fingerprint, validate_claim_completion,
    verify_rehearsal_claim_completion_proof,
};
use question_model::{
    AssignmentId, AssignmentReference, CourseId, CourseMembershipId, CourseMembershipReference,
    PreviewEvaluation, RehearsalEvidenceKind, RehearsalEvidenceRecord, RehearsalGradeOperationId,
    RehearsalLifecycle, RehearsalPublicOutcome, RehearsalReference, RehearsalRunId,
    RehearsalRunReceipt, RehearsalSubjectStart, RehearsalSubmissionClaimId,
    TeachingOperationRevision, TenantId, UserId,
};

pub(super) use super::rehearsal_integrity::invalidate_assignment_rehearsals;
use super::rehearsal_integrity::{
    genesis, transition_locked, verify_rehearsal_aggregate, verify_run,
};
use super::*;

#[derive(Debug, Clone)]
pub(super) struct StoredRehearsalRun {
    pub(super) id: RehearsalRunId,
    pub(super) reference: RehearsalReference,
    pub(super) course: CourseId,
    pub(super) assignment_id: AssignmentId,
    pub(super) assignment: AssignmentReference,
    pub(super) owner: CourseMembershipId,
    pub(super) actor: UserId,
    pub(super) revision: TeachingOperationRevision,
    pub(super) subject: question_model::PreviewSubject,
    pub(super) fingerprint: domain::RehearsalSubjectFingerprint,
    pub(super) lifecycle: RehearsalLifecycle,
    pub(super) started_at: question_model::ActivityTimestamp,
    pub(super) updated_at: question_model::ActivityTimestamp,
    /// Aggregate commitment advanced only by private evidence append.
    pub(super) evidence_head: domain::RehearsalEvidenceHead,
}

#[derive(Debug, Clone)]
pub(super) struct StoredRehearsalSubmissionReceipt {
    outcome: RehearsalPublicOutcome,
}

#[derive(Clone)]
pub(super) struct StoredRehearsalClaim {
    pub(super) root: RehearsalPersistedClaimRoot,
    pub(super) events: Vec<RehearsalClaimTransitionEvent>,
    pub(super) receipt: Option<StoredRehearsalSubmissionReceipt>,
}

impl std::fmt::Debug for StoredRehearsalClaim {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StoredRehearsalClaim")
            .field("claim", &self.root.claim())
            .field("fingerprint", &self.root.fingerprint().to_hex())
            .field("events", &self.events)
            .field("receipt", &self.receipt)
            .finish()
    }
}

#[derive(Clone, Default)]
pub(super) struct StoredRehearsalEvidence(pub(super) Vec<RehearsalEvidenceChainEntry>);

impl std::fmt::Debug for StoredRehearsalEvidence {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_list()
            .entries(self.0.iter().map(|entry| {
                (
                    entry.record.clone(),
                    private_payload_digest(&entry.payload).to_hex(),
                )
            }))
            .finish()
    }
}

#[async_trait]
impl crate::RehearsalStore for MemoryStore {
    async fn start_rehearsal(
        &self,
        context: TenantContext,
        command: crate::StartRehearsalCommand,
    ) -> Result<RehearsalRunReceipt, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let (assignment_id, owner) = authorize_assignment(
            &state,
            tenant,
            command.actor,
            command.course,
            command.assignment,
            command.revision,
        )?;
        let subject = resolve_subject_locked(&state, tenant, &command, assignment_id)?;
        let fingerprint =
            fingerprint_resolved_preview_subject(command.assignment, command.revision, &subject)
                .map_err(|error| {
                    StoreError::InvalidRecord(format!("invalid rehearsal subject: {error:?}"))
                })?;
        let key = (tenant, command.course, assignment_id, owner);
        let existing = state
            .rehearsal_active_by_owner
            .get(&key)
            .and_then(|id| state.rehearsal_runs.get(&(tenant, *id)))
            .cloned();
        if let Some(existing) = &existing {
            verify_rehearsal_aggregate(&state, tenant, existing)?;
        }
        match decide_start(
            existing.as_ref().map(|run| RehearsalLifecycleSnapshot {
                lifecycle: run.lifecycle,
                revision: run.revision,
                subject_fingerprint: run.fingerprint,
            }),
            command.revision,
            fingerprint,
            command.start_new_after_completion,
        ) {
            domain::RehearsalStartDecision::Resume => {
                return receipt(existing.as_ref().expect("run"));
            }
            domain::RehearsalStartDecision::RequireExplicitRestart => {
                return Err(StoreError::Conflict);
            }
            domain::RehearsalStartDecision::DiscardByNewSubjectThenCreate => transition_locked(
                &mut state,
                tenant,
                existing.as_ref().expect("run").id,
                RehearsalTerminalTransition::DiscardByNewSubject,
            )?,
            domain::RehearsalStartDecision::DiscardStaleRevision => {
                return Err(StoreError::Conflict);
            }
            domain::RehearsalStartDecision::Create => {}
        }
        let id = RehearsalRunId::from_uuid(fresh_uuid()?);
        let reference = next_reference(&mut state)?;
        let now = state.authoritative_time;
        let run = StoredRehearsalRun {
            id,
            reference,
            course: command.course,
            assignment_id,
            assignment: command.assignment,
            owner,
            actor: command.actor,
            revision: command.revision,
            subject,
            fingerprint,
            lifecycle: RehearsalLifecycle::Active,
            started_at: now,
            updated_at: now,
            evidence_head: domain::evidence_genesis_head(domain::RehearsalGenesisContext {
                rehearsal: id,
                tenant,
                course: command.course,
                assignment: command.assignment,
                direct_instructor_membership: owner,
                revision: command.revision,
                subject_fingerprint: fingerprint,
            }),
        };
        state.rehearsal_by_reference.insert((tenant, reference), id);
        state.rehearsal_active_by_owner.insert(key, id);
        state.rehearsal_runs.insert((tenant, id), run.clone());
        state
            .rehearsal_evidence
            .insert((tenant, id), StoredRehearsalEvidence::default());
        Ok(receipt(&run)?)
    }

    async fn read_rehearsal(
        &self,
        context: TenantContext,
        locator: crate::RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError> {
        let state = self.read_state()?;
        let tenant = context.tenant_id();
        let (assignment, owner) = authorize_locator(&state, tenant, locator)?;
        let run = authorized_run(&state, tenant, locator, assignment, owner)?;
        verify_rehearsal_aggregate(&state, tenant, run)?;
        receipt(run)
    }

    async fn append_rehearsal_frozen_item(
        &self,
        context: TenantContext,
        command: crate::AppendRehearsalFrozenItemCommand,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let (assignment, owner) = authorize_locator(&state, tenant, command.locator)?;
        let run = authorized_run(&state, tenant, command.locator, assignment, owner)?.clone();
        active_current(&state, tenant, &run)?;
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        crate::ensure_rehearsal_delivery_supported(&command.frozen.response_definition)?;
        let key = (tenant, run.id, command.frozen.attempt);
        if state.rehearsal_frozen_items.contains_key(&key) {
            return Err(StoreError::Conflict);
        }
        append_evidence(
            &mut state,
            tenant,
            &run,
            RehearsalEvidencePayload::FrozenItem(command.frozen.clone()),
        )?;
        state.rehearsal_frozen_items.insert(key, command.frozen);
        Ok(())
    }

    async fn claim_rehearsal_submission(
        &self,
        context: TenantContext,
        command: crate::ClaimRehearsalSubmissionCommand,
    ) -> Result<crate::RehearsalSubmissionClaimResult, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let (assignment, owner) = authorize_locator(&state, tenant, command.locator)?;
        let run = authorized_run(&state, tenant, command.locator, assignment, owner)?.clone();
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        let frozen = state
            .rehearsal_frozen_items
            .get(&(tenant, run.id, command.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let request = RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
            &frozen,
            command.attempt,
            command.response,
        )
        .map_err(|error| {
            StoreError::InvalidRecord(format!("invalid rehearsal submission: {error:?}"))
        })?;
        let fingerprint =
            rehearsal_submission_request_fingerprint(genesis(&run, tenant), &frozen, &request)
                .map_err(|error| {
                    StoreError::InvalidRecord(format!("invalid rehearsal request: {error:?}"))
                })?;
        let key = (tenant, run.id, command.idempotency_key);
        let existing = state
            .rehearsal_submission_claims
            .get(&key)
            .map(|claim| hydrate_claim(&state, tenant, &run, claim))
            .transpose()?;
        let root = RehearsalClaimRoot::verify_persisted(
            genesis(&run, tenant),
            &frozen,
            RehearsalPersistedClaimRoot::from_persisted(
                run.id,
                RehearsalSubmissionClaimId::from_uuid(fresh_uuid()?),
                fingerprint,
                request.clone(),
            ),
        )
        .map_err(invalid_claim_root)?;
        let decision = decide_submission_claim(
            run.lifecycle,
            revision_is_current(&state, tenant, &run)?,
            existing.as_ref().map(|(_, snapshot)| snapshot),
            fingerprint,
            &root,
            RehearsalGradeOperationId::from_uuid(fresh_uuid()?),
        );
        match decision {
            RehearsalSubmissionClaimDecision::New { handle } => {
                let event = root.restore_transition(
                    1,
                    handle.operation(),
                    handle.generation(),
                    RehearsalSubmissionClaimPhase::Prepared,
                    state.authoritative_time,
                    None,
                    None,
                );
                hydrate_claim_history(&root, &[event], None).map_err(invalid_claim_history)?;
                state.rehearsal_submission_claims.insert(
                    key,
                    StoredRehearsalClaim {
                        root: root.into_persisted(),
                        events: vec![event],
                        receipt: None,
                    },
                );
                Ok(crate::RehearsalSubmissionClaimResult::Claimed(
                    crate::ClaimedRehearsalSubmission {
                        handle,
                        prepared: request,
                    },
                ))
            }
            RehearsalSubmissionClaimDecision::Reclaimed { handle } => {
                let (existing_root, _) = existing.ok_or(StoreError::NotFound)?;
                let now = state.authoritative_time;
                let claim = state
                    .rehearsal_submission_claims
                    .get_mut(&key)
                    .ok_or(StoreError::NotFound)?;
                let event = existing_root.restore_transition(
                    next_claim_sequence(&claim.events)?,
                    handle.operation(),
                    handle.generation(),
                    RehearsalSubmissionClaimPhase::Prepared,
                    now,
                    None,
                    None,
                );
                let mut events = claim.events.clone();
                events.push(event);
                hydrate_claim_history(&existing_root, &events, None)
                    .map_err(invalid_claim_history)?;
                claim.events.push(event);
                Ok(crate::RehearsalSubmissionClaimResult::Claimed(
                    crate::ClaimedRehearsalSubmission {
                        handle,
                        prepared: request,
                    },
                ))
            }
            RehearsalSubmissionClaimDecision::Replay { receipt } => {
                let stored = state
                    .rehearsal_submission_claims
                    .get(&key)
                    .and_then(|claim| claim.receipt.as_ref())
                    .ok_or_else(|| {
                        StoreError::InvalidRecord(
                            "completed rehearsal claim has no immutable receipt".into(),
                        )
                    })?;
                if stored.outcome != receipt {
                    return Err(StoreError::InvalidRecord(
                        "rehearsal completion receipt integrity failure".into(),
                    ));
                }
                Ok(crate::RehearsalSubmissionClaimResult::Replay(
                    crate::RehearsalSubmissionReceipt {
                        outcome: receipt,
                        replayed: true,
                    },
                ))
            }
            RehearsalSubmissionClaimDecision::Pending => {
                Ok(crate::RehearsalSubmissionClaimResult::Pending)
            }
            RehearsalSubmissionClaimDecision::Conflict
            | RehearsalSubmissionClaimDecision::ReclaimRefused(_) => {
                Ok(crate::RehearsalSubmissionClaimResult::Conflict)
            }
            RehearsalSubmissionClaimDecision::StaleRevision => Err(StoreError::Conflict),
            RehearsalSubmissionClaimDecision::TerminalLifecycle => Err(StoreError::Conflict),
        }
    }

    async fn complete_rehearsal_submission(
        &self,
        context: TenantContext,
        command: crate::CompleteRehearsalSubmissionCommand,
    ) -> Result<crate::RehearsalSubmissionReceipt, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let (assignment, owner) = authorize_locator(&state, tenant, command.locator)?;
        let run = authorized_run(&state, tenant, command.locator, assignment, owner)?.clone();
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        let claim_key = claim_key_for_handle(&state, tenant, run.id, command.handle.claim())?;
        let claim = state
            .rehearsal_submission_claims
            .get(&claim_key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let (root, snapshot) = hydrate_claim(&state, tenant, &run, &claim)?;
        let claimed_operation = command.handle.operation();
        let claimed_generation = command.handle.generation();
        let hydrated_handle = snapshot
            .into_dispatched_handle()
            .map_err(|_| StoreError::Conflict)?;
        if !same_dispatched_handle(&hydrated_handle, &command.handle) {
            return Err(StoreError::Conflict);
        }
        validate_claim_completion(
            run.lifecycle,
            revision_is_current(&state, tenant, &run)?,
            command.handle,
        )
        .map_err(|_| StoreError::Conflict)?;
        let frozen = state
            .rehearsal_frozen_items
            .get(&(tenant, run.id, root.sealed_request().attempt()))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let evidence =
            domain::RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
                &root,
                root.sealed_request().clone(),
                &frozen,
                command.grading,
                state.authoritative_time,
            )
            .map_err(|error| {
                StoreError::InvalidRecord(format!("invalid rehearsal completion: {error:?}"))
            })?;
        let entries = state
            .rehearsal_evidence
            .get(&(tenant, run.id))
            .ok_or(StoreError::NotFound)?;
        let entry = next_evidence_entry(
            &run,
            RehearsalEvidencePayload::AcceptedSubmission(evidence),
            state.authoritative_time,
        )?;
        let mut staged_entries = entries.0.clone();
        staged_entries.push(entry.clone());
        let staged_head = run.evidence_head.advance(&entry.record).map_err(|error| {
            StoreError::InvalidRecord(format!(
                "rehearsal evidence-head advancement failure: {error:?}"
            ))
        })?;
        let proof = verify_rehearsal_claim_completion_proof(
            genesis(&run, tenant),
            staged_head,
            &root,
            &staged_entries,
        )
        .map_err(|error| {
            StoreError::InvalidRecord(format!("rehearsal completion proof failure: {error:?}"))
        })?;
        let outcome = proof.replay_receipt();
        let event = root.restore_transition(
            next_claim_sequence(&claim.events)?,
            claimed_operation,
            claimed_generation,
            RehearsalSubmissionClaimPhase::Completed,
            state.authoritative_time,
            None,
            Some(proof.completion_material()),
        );
        let mut staged_events = claim.events.clone();
        staged_events.push(event);
        hydrate_claim_history(&root, &staged_events, Some(proof)).map_err(invalid_claim_history)?;
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
            .evidence_head = staged_head;
        let claim = state
            .rehearsal_submission_claims
            .get_mut(&claim_key)
            .ok_or(StoreError::NotFound)?;
        claim.events.push(event);
        claim.receipt = Some(StoredRehearsalSubmissionReceipt {
            outcome: outcome.clone(),
        });
        Ok(crate::RehearsalSubmissionReceipt {
            outcome,
            replayed: false,
        })
    }

    async fn mark_rehearsal_submission_dispatched(
        &self,
        context: TenantContext,
        command: crate::MarkRehearsalSubmissionDispatchedCommand,
    ) -> Result<DispatchedClaimHandle, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let (assignment, owner) = authorize_locator(&state, tenant, command.locator)?;
        let run = authorized_run(&state, tenant, command.locator, assignment, owner)?.clone();
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        let key = claim_key_for_handle(&state, tenant, run.id, command.handle.claim())?;
        let claim = state
            .rehearsal_submission_claims
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let (root, snapshot) = hydrate_claim(&state, tenant, &run, &claim)?;
        let hydrated = snapshot
            .into_prepared_handle()
            .map_err(|_| StoreError::Conflict)?;
        if !same_prepared_handle(&hydrated, &command.handle) {
            return Err(StoreError::Conflict);
        }
        let operation = command.handle.operation();
        let generation = command.handle.generation();
        let dispatched = mark_rehearsal_submission_dispatched(command.handle);
        let event = root.restore_transition(
            next_claim_sequence(&claim.events)?,
            operation,
            generation,
            RehearsalSubmissionClaimPhase::GradingDispatched,
            state.authoritative_time,
            None,
            None,
        );
        let mut events = claim.events.clone();
        events.push(event);
        hydrate_claim_history(&root, &events, None).map_err(invalid_claim_history)?;
        state
            .rehearsal_submission_claims
            .get_mut(&key)
            .ok_or(StoreError::NotFound)?
            .events
            .push(event);
        Ok(dispatched)
    }

    async fn discard_rehearsal(
        &self,
        context: TenantContext,
        locator: crate::RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError> {
        transition_by_locator(
            self,
            context,
            locator,
            RehearsalTerminalTransition::DiscardByInstructor,
        )
        .await
    }
    async fn complete_rehearsal(
        &self,
        context: TenantContext,
        locator: crate::RehearsalLocator,
    ) -> Result<RehearsalRunReceipt, StoreError> {
        transition_by_locator(
            self,
            context,
            locator,
            RehearsalTerminalTransition::Complete,
        )
        .await
    }
}

#[async_trait]
impl crate::RehearsalPreDispatchCompensationStore for MemoryStore {
    async fn abandon_rehearsal_submission_before_dispatch(
        &self,
        context: TenantContext,
        command: crate::AbandonRehearsalSubmissionBeforeDispatchCommand,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let (assignment, owner) = authorize_locator(&state, tenant, command.locator)?;
        let run = authorized_run(&state, tenant, command.locator, assignment, owner)?.clone();
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        let key = claim_key_for_handle(&state, tenant, run.id, command.handle.claim())?;
        let claim = state
            .rehearsal_submission_claims
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let (root, snapshot) = hydrate_claim(&state, tenant, &run, &claim)?;
        let hydrated = snapshot
            .into_prepared_handle()
            .map_err(|_| StoreError::Conflict)?;
        if !same_prepared_handle(&hydrated, &command.handle) {
            return Err(StoreError::Conflict);
        }
        let operation = command.handle.operation();
        let generation = command.handle.generation();
        let abandoned =
            abandon_rehearsal_submission_before_dispatch(command.handle, command.reason);
        let event = root.restore_transition(
            next_claim_sequence(&claim.events)?,
            operation,
            generation,
            RehearsalSubmissionClaimPhase::AbandonedBeforeDispatch,
            state.authoritative_time,
            Some(abandoned.reason()),
            None,
        );
        let mut events = claim.events.clone();
        events.push(event);
        hydrate_claim_history(&root, &events, None).map_err(invalid_claim_history)?;
        state
            .rehearsal_submission_claims
            .get_mut(&key)
            .ok_or(StoreError::NotFound)?
            .events
            .push(event);
        Ok(())
    }
}

async fn transition_by_locator(
    store: &MemoryStore,
    context: TenantContext,
    locator: crate::RehearsalLocator,
    transition: RehearsalTerminalTransition,
) -> Result<RehearsalRunReceipt, StoreError> {
    let tenant = context.tenant_id();
    let mut state = store.write_state()?;
    let (assignment, owner) = authorize_locator(&state, tenant, locator)?;
    let id = authorized_run(&state, tenant, locator, assignment, owner)?.id;
    transition_locked(&mut state, tenant, id, transition)?;
    receipt(
        state
            .rehearsal_runs
            .get(&(tenant, id))
            .ok_or(StoreError::NotFound)?,
    )
}

fn resolve_subject_locked(
    state: &State,
    tenant: TenantId,
    command: &crate::StartRehearsalCommand,
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
                    revision: command.revision,
                    selected_moment: request.selected_moment.clone(),
                    groups: request.groups.clone(),
                    modifiers: request.modifiers.clone(),
                },
            )?
        }
        RehearsalSubjectStart::Derived { candidate } => {
            domain::validate_subject_binding(command.assignment, command.revision, candidate)
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
                    command.revision,
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

fn authorize_assignment(
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
fn authorize_locator(
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
fn authorized_run(
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
fn revision_is_current(
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
fn active_current(
    state: &State,
    tenant: TenantId,
    run: &StoredRehearsalRun,
) -> Result<(), StoreError> {
    (run.lifecycle.is_active() && revision_is_current(state, tenant, run)?)
        .then_some(())
        .ok_or(StoreError::Conflict)
}
fn next_reference(state: &mut State) -> Result<RehearsalReference, StoreError> {
    state.next_rehearsal_reference = state
        .next_rehearsal_reference
        .checked_add(1)
        .ok_or_else(|| StoreError::Unavailable("rehearsal reference sequence exhausted".into()))?;
    RehearsalReference::new(u64::from(state.next_rehearsal_reference))
        .ok_or_else(|| StoreError::Unavailable("rehearsal reference limit reached".into()))
}
fn fresh_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("rehearsal ID randomness failed: {error}"))
    })
}
fn receipt(run: &StoredRehearsalRun) -> Result<RehearsalRunReceipt, StoreError> {
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
pub(super) fn hydrate_claim(
    state: &State,
    tenant: TenantId,
    run: &StoredRehearsalRun,
    claim: &StoredRehearsalClaim,
) -> Result<(RehearsalClaimRoot, RehearsalSubmissionClaimSnapshot), StoreError> {
    verify_run(state, tenant, run)?;
    let frozen = state
        .rehearsal_frozen_items
        .get(&(tenant, run.id, claim.root.sealed_request().attempt()))
        .ok_or(StoreError::NotFound)?;
    let root =
        RehearsalClaimRoot::verify_persisted(genesis(run, tenant), frozen, claim.root.clone())
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

fn invalid_claim_root(error: domain::RehearsalClaimRootVerificationError) -> StoreError {
    StoreError::InvalidRecord(format!("invalid rehearsal claim root: {error:?}"))
}
pub(super) fn invalid_claim_history(error: domain::RehearsalClaimHydrationError) -> StoreError {
    StoreError::InvalidRecord(format!("invalid rehearsal claim history: {error:?}"))
}
pub(super) fn next_claim_sequence(
    events: &[RehearsalClaimTransitionEvent],
) -> Result<u64, StoreError> {
    u64::try_from(events.len())
        .ok()
        .and_then(|length| length.checked_add(1))
        .ok_or_else(|| {
            StoreError::Unavailable("rehearsal claim transition sequence exhausted".into())
        })
}
fn claim_key_for_handle(
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
            (key.0 == tenant && key.1 == run && claim.root.claim() == claim_id).then(|| key.clone())
        })
        .ok_or(StoreError::NotFound)
}
fn same_prepared_handle(left: &PreparedClaimHandle, right: &PreparedClaimHandle) -> bool {
    left.rehearsal() == right.rehearsal()
        && left.claim() == right.claim()
        && left.operation() == right.operation()
        && left.generation() == right.generation()
        && left.fingerprint() == right.fingerprint()
}
fn same_dispatched_handle(left: &DispatchedClaimHandle, right: &DispatchedClaimHandle) -> bool {
    left.rehearsal() == right.rehearsal()
        && left.claim() == right.claim()
        && left.operation() == right.operation()
        && left.generation() == right.generation()
        && left.fingerprint() == right.fingerprint()
}
fn append_evidence(
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

fn next_evidence_entry(
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
