use super::*;
use crate::contracts::RehearsalInternalStore;

#[async_trait]
impl crate::contracts::RehearsalInternalStore for MemoryStore {
    async fn start_rehearsal_from_route(
        &self,
        context: TenantContext,
        command: crate::StartRehearsalRouteCommand,
    ) -> Result<crate::StartRehearsalRouteResult, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let (assignment_id, owner) = authorize_assignment(
            &state,
            tenant,
            command.actor,
            command.course,
            command.assignment,
            command.expected_revision,
        )?;
        let key = (
            tenant,
            command.course,
            assignment_id,
            owner,
            command.actor,
            command.idempotency_key.clone(),
        );
        if let Some(existing) = state.rehearsal_start_operations.get(&key) {
            return if existing.fingerprint == command.request_fingerprint {
                Ok(crate::StartRehearsalRouteResult {
                    receipt: existing.receipt.clone(),
                    replayed: true,
                })
            } else {
                Err(StoreError::Conflict)
            };
        }
        // Route start is the canonical 1822 admission boundary.  Stage the
        // aggregate, every frozen item, and both material siblings together;
        // any unsupported or malformed selected item leaves no new run or
        // durable idempotency receipt behind.
        let mut staged = state.clone();
        let receipt = start_locked(&mut staged, tenant, &command)?;
        freeze_route_start_material(&mut staged, tenant, &receipt)?;
        staged.rehearsal_start_operations.insert(
            key,
            StoredRehearsalStartOperation {
                fingerprint: command.request_fingerprint,
                receipt: receipt.clone(),
            },
        );
        *state = staged;
        Ok(crate::StartRehearsalRouteResult {
            receipt,
            replayed: false,
        })
    }

    async fn read_rehearsal_from_route(
        &self,
        context: TenantContext,
        command: crate::ReadRehearsalRouteCommand,
    ) -> Result<RehearsalRunReceipt, StoreError> {
        let tenant = context.tenant_id();
        let locator = {
            let state = self.read_state()?;
            let run_id = state
                .rehearsal_by_reference
                .get(&(tenant, command.rehearsal))
                .copied()
                .ok_or(StoreError::NotFound)?;
            let run = state
                .rehearsal_runs
                .get(&(tenant, run_id))
                .ok_or(StoreError::NotFound)?;
            if run.course != command.course || run.assignment != command.assignment {
                return Err(StoreError::NotFound);
            }
            crate::RehearsalLocator {
                actor: command.actor,
                course: command.course,
                assignment: command.assignment,
                revision: run.revision,
                rehearsal: command.rehearsal,
            }
        };
        RehearsalInternalStore::read_rehearsal(self, context, locator).await
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

    #[cfg(feature = "test-support")]
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
        // This happens under the same aggregate write lock that mints a
        // grading claim. A server-clock expiry and a submission therefore
        // serialize to one durable outcome rather than racing through a
        // browser-selected timer state.
        if let Some(operation) = state
            .rehearsal_delivery_operations
            .iter()
            .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
            .flat_map(|(_, delivery)| delivery.generations.iter())
            .find(|generation| generation.descriptor.attempt() == command.attempt)
            .map(|generation| generation.operation)
            && super::reconcile_delivery_expiry_locked(&mut state, tenant, &run, operation)?
        {
            return Err(StoreError::Conflict);
        }
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
        let input = RehearsalClaimSubmissionInput::durable(request.clone());
        let fingerprint =
            rehearsal_claim_submission_input_fingerprint(genesis(&run, tenant), &frozen, &input)
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
                input,
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
                        claim: root.claim(),
                        fingerprint: root.fingerprint(),
                        attempt: command.attempt,
                        submission_input:
                            domain::rehearsal::persistence::encode_claim_submission_input(
                                root.submission_input(),
                            ),
                        events: vec![event],
                        receipt: None,
                        route_delivery: None,
                    },
                );
                Ok(crate::RehearsalSubmissionClaimResult::Claimed(
                    crate::ClaimedRehearsalSubmission { handle },
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
                    crate::ClaimedRehearsalSubmission { handle },
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

    #[cfg(feature = "test-support")]
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
            .get(&(tenant, run.id, claim.attempt))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let durable_request =
            super::durable_request_for_claim_completion(&state, tenant, run.id, &claim, &root)?;
        let evidence = domain::RehearsalValidatedSubmissionEvidence::try_complete_with_claim_input(
            &root,
            durable_request,
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
        let mut accepted_after_completion = std::collections::BTreeSet::new();
        for stored in state
            .rehearsal_submission_claims
            .iter()
            .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
            .map(|(_, stored)| stored)
        {
            let (_, stored_snapshot) = hydrate_claim(&state, tenant, &run, stored)?;
            if stored_snapshot.state() == domain::RehearsalSubmissionClaimState::Completed {
                accepted_after_completion.insert(stored.attempt);
            }
        }
        let frozen_count = state
            .rehearsal_frozen_items
            .keys()
            .filter(|(stored_tenant, stored_run, _)| {
                *stored_tenant == tenant && *stored_run == run.id
            })
            .count();
        let completes_run = accepted_after_completion.len().saturating_add(1) == frozen_count;
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
        if completes_run {
            transition_locked(
                &mut state,
                tenant,
                run.id,
                domain::RehearsalTerminalTransition::Complete,
            )?;
        }
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

    #[cfg(feature = "test-support")]
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
    #[cfg(feature = "test-support")]
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

/// Consumes a sealed completion capability under the aggregate write lock.
/// Reauthentication prevents a stale capability, wrong tenant, or binding
/// substitution from appending evidence after preparation.
pub(in crate::in_memory) fn complete_sealed_submission_locked(
    state: &std::sync::RwLock<State>,
    context: TenantContext,
    completion: crate::SealedRehearsalSubmissionCompletion,
    grading: question_model::RehearsalPrivateGradingResult,
) -> Result<crate::RehearsalSubmissionReceipt, StoreError> {
    let expected_head = completion.expected_evidence_head();
    let (
        capability_context,
        route,
        handle,
        sealed_root,
        attempt,
        sealed_frozen,
        capability_head,
        presentation_commitment,
        durable_request,
    ) = completion.into_internal_parts();
    if capability_context != context {
        return Err(StoreError::Conflict);
    }
    let tenant = context.tenant_id();
    let mut state = state.write().map_err(|_| {
        StoreError::Unavailable("sealed rehearsal execution state is unavailable".into())
    })?;
    let locator = super::mutations::route_locator(&state, tenant, route)?;
    let (assignment, owner) = authorize_locator(&state, tenant, locator)?;
    let run = authorized_run(&state, tenant, locator, assignment, owner)?.clone();
    verify_rehearsal_aggregate(&state, tenant, &run)?;
    if run.evidence_head != expected_head || capability_head != expected_head {
        return Err(StoreError::Conflict);
    }
    let claim_key = claim_key_for_handle(&state, tenant, run.id, handle.claim())?;
    let claim = state
        .rehearsal_submission_claims
        .get(&claim_key)
        .cloned()
        .ok_or(StoreError::NotFound)?;
    if claim.attempt != attempt {
        return Err(StoreError::Conflict);
    }
    let (root, snapshot) = hydrate_claim(&state, tenant, &run, &claim)?;
    if root != sealed_root {
        return Err(StoreError::Conflict);
    }
    let hydrated_handle = snapshot
        .into_dispatched_handle()
        .map_err(|_| StoreError::Conflict)?;
    if !same_dispatched_handle(&hydrated_handle, &handle) {
        return Err(StoreError::Conflict);
    }
    let binding = claim.route_delivery.ok_or(StoreError::Conflict)?;
    if binding.screen_digest != presentation_commitment
        || root.submission_input().presentation_commitment() != Some(presentation_commitment)
    {
        return Err(StoreError::Conflict);
    }
    let claimed_operation = handle.operation();
    let claimed_generation = handle.generation();
    validate_claim_completion(
        run.lifecycle,
        revision_is_current(&state, tenant, &run)?,
        handle,
    )
    .map_err(|_| StoreError::Conflict)?;
    let frozen = state
        .rehearsal_frozen_items
        .get(&(tenant, run.id, attempt))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    if frozen != sealed_frozen {
        return Err(StoreError::Conflict);
    }
    let expected_request =
        super::durable_request_for_claim_completion(&state, tenant, run.id, &claim, &root)?;
    if expected_request != durable_request {
        return Err(StoreError::Conflict);
    }
    let evidence = domain::RehearsalValidatedSubmissionEvidence::try_complete_with_claim_input(
        &root,
        durable_request,
        &frozen,
        grading,
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
    let mut completed_attempts = std::collections::BTreeSet::new();
    for stored in state
        .rehearsal_submission_claims
        .iter()
        .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
        .map(|(_, stored)| stored)
    {
        let (_, stored_snapshot) = hydrate_claim(&state, tenant, &run, stored)?;
        if stored_snapshot.state() == domain::RehearsalSubmissionClaimState::Completed {
            completed_attempts.insert(stored.attempt);
        }
    }
    let frozen_count = state
        .rehearsal_frozen_items
        .keys()
        .filter(|(stored_tenant, stored_run, _)| *stored_tenant == tenant && *stored_run == run.id)
        .count();
    let completes_run = completed_attempts.len().saturating_add(1) == frozen_count;
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
    if completes_run {
        transition_locked(
            &mut state,
            tenant,
            run.id,
            domain::RehearsalTerminalTransition::Complete,
        )?;
    }
    Ok(crate::RehearsalSubmissionReceipt {
        outcome,
        replayed: false,
    })
}
