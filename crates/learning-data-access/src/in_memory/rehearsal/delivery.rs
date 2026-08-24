use super::*;

fn timing_witness_for_dispatch(
    state: &State,
    tenant: TenantId,
    run: &StoredRehearsalRun,
    descriptor: &crate::RehearsalDeliveryExecutionDescriptorV1,
    issued_at: question_model::ActivityTimestamp,
) -> Result<domain::RehearsalTimingDispatchDecisionV1, StoreError> {
    let source = state
        .rehearsal_frozen_source_snapshots
        .get(&(tenant, run.id, descriptor.attempt()))
        .ok_or_else(missing_rehearsal_material)?;
    let (_, digest) = source.snapshot.canonical_payload_bytes()?;
    let frozen_snapshot_digest =
        question_model::RehearsalEvidenceDigest::from_bytes(*digest.as_bytes());
    domain::decide_rehearsal_timing_dispatch(domain::RehearsalTimingInputsV1 {
        subject_fingerprint: run.fingerprint,
        frozen_snapshot_digest,
        timing_policy: source.snapshot.question().timing_policy,
        subject_time_limit_seconds: run.subject.policy.time_limit_seconds().value,
        run_started_at: run.started_at,
        issued_at,
    })
    .map_err(|error| {
        StoreError::Unavailable(format!("rehearsal timing cannot be derived: {error}"))
    })
}

/// Reconciles the immutable availability witness while the caller owns the
/// aggregate write lock. The function is shared by explicit expiry and
/// submission selection so those competing operations have one outcome.
pub(in crate::in_memory) fn reconcile_delivery_expiry_locked(
    state: &mut State,
    tenant: TenantId,
    run: &StoredRehearsalRun,
    operation: crate::RehearsalOperationId,
) -> Result<bool, StoreError> {
    let now = state.authoritative_time;
    let generation = state
        .rehearsal_delivery_operations
        .values()
        .flat_map(|entry| entry.generations.iter())
        .find(|entry| entry.operation == operation)
        .ok_or(StoreError::NotFound)?;
    let phase = generation.phase()?;
    let witness = generation.timing_witness;
    let attempt = generation.descriptor.attempt();
    match phase {
        StoredRehearsalDeliveryPhase::Expired => return Ok(true),
        StoredRehearsalDeliveryPhase::Dispatched | StoredRehearsalDeliveryPhase::Completed => {}
        StoredRehearsalDeliveryPhase::Prepared
        | StoredRehearsalDeliveryPhase::AbandonedBeforeDispatch
        | StoredRehearsalDeliveryPhase::RunTimeExhaustedBeforeDispatch => {
            return Ok(false);
        }
    }
    let witness = witness.ok_or_else(|| {
        StoreError::Unavailable("dispatched rehearsal delivery lacks timing witness".into())
    })?;
    let source = state
        .rehearsal_frozen_source_snapshots
        .get(&(tenant, run.id, attempt))
        .ok_or_else(missing_rehearsal_material)?;
    let (_, digest) = source.snapshot.canonical_payload_bytes()?;
    let digest = question_model::RehearsalEvidenceDigest::from_bytes(*digest.as_bytes());
    domain::verify_rehearsal_timing_witness(
        domain::RehearsalTimingInputsV1 {
            subject_fingerprint: run.fingerprint,
            frozen_snapshot_digest: digest,
            timing_policy: source.snapshot.question().timing_policy,
            subject_time_limit_seconds: run.subject.policy.time_limit_seconds().value,
            run_started_at: run.started_at,
            issued_at: witness.issued_at(),
        },
        witness,
    )
    .map_err(|error| {
        StoreError::Unavailable(format!("rehearsal timing witness is invalid: {error}"))
    })?;
    if domain::rehearsal_timing_verdict(witness, now).map_err(|error| {
        StoreError::Unavailable(format!("rehearsal timing cannot be evaluated: {error}"))
    })? == domain::RehearsalTimingVerdictV1::Expired
    {
        let generation = state
            .rehearsal_delivery_operations
            .values_mut()
            .flat_map(|entry| entry.generations.iter_mut())
            .find(|entry| entry.operation == operation)
            .ok_or(StoreError::NotFound)?;
        generation.append_phase(StoredRehearsalDeliveryPhase::Expired, now)?;
        return Ok(true);
    }
    Ok(false)
}

#[async_trait]
impl crate::RehearsalOperationStore for MemoryStore {
    async fn discard_rehearsal_idempotent(
        &self,
        _context: TenantContext,
        _command: crate::RehearsalDiscardOperationCommand,
    ) -> Result<crate::RehearsalIdempotentProjectionResult, StoreError> {
        Err(StoreError::Unavailable(
            "memory discard operation protocol is not installed".into(),
        ))
    }

    async fn claim_rehearsal_delivery(
        &self,
        context: TenantContext,
        request: crate::RehearsalDeliveryRequest,
    ) -> Result<crate::RehearsalDeliveryClaimResult, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let (assignment, owner) = authorize_locator(&state, tenant, request.locator)?;
        let run = authorized_run(&state, tenant, request.locator, assignment, owner)?.clone();
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        let key = (tenant, run.id, request.idempotency_key.clone());
        if let Some(existing) = state.rehearsal_delivery_operations.get(&key) {
            if existing.fingerprint != request.request_fingerprint {
                return Ok(crate::RehearsalDeliveryClaimResult::Conflict);
            }
            let latest = existing.generations.last().cloned().ok_or_else(|| {
                StoreError::InvalidRecord("rehearsal delivery root has no generation".into())
            })?;
            if latest.phase()? == StoredRehearsalDeliveryPhase::Expired {
                return Ok(crate::RehearsalDeliveryClaimResult::Expired);
            }
            if let Some(screen) = &latest.screen {
                return Ok(crate::RehearsalDeliveryClaimResult::Replay(screen.clone()));
            }
            return match latest.phase()? {
                StoredRehearsalDeliveryPhase::Prepared => {
                    Ok(crate::RehearsalDeliveryClaimResult::Prepared {
                        prepared: crate::PreparedRehearsalDelivery::mint(
                            request.locator,
                            latest.operation,
                            latest.descriptor.clone(),
                        ),
                    })
                }
                StoredRehearsalDeliveryPhase::Dispatched => {
                    Ok(crate::RehearsalDeliveryClaimResult::Pending {
                        dispatched: crate::DispatchedRehearsalDelivery::mint(
                            request.locator,
                            latest.operation,
                        ),
                    })
                }
                StoredRehearsalDeliveryPhase::Completed => Err(StoreError::InvalidRecord(
                    "completed delivery lacks screen projection".into(),
                )),
                StoredRehearsalDeliveryPhase::Expired => {
                    Ok(crate::RehearsalDeliveryClaimResult::Expired)
                }
                StoredRehearsalDeliveryPhase::RunTimeExhaustedBeforeDispatch => {
                    Ok(crate::RehearsalDeliveryClaimResult::RunTimeExhausted {
                        deadline: latest.run_time_exhausted_deadline.ok_or_else(|| {
                            StoreError::Unavailable("run-time exhaustion lacks deadline".into())
                        })?,
                    })
                }
                StoredRehearsalDeliveryPhase::AbandonedBeforeDispatch => {
                    // An abandonment proves no external effect. Preserve the sealed
                    // selection exactly, but mint a new backend operation identity.
                    let operation = crate::RehearsalOperationId::from_uuid(fresh_uuid()?);
                    let now = state.authoritative_time;
                    let entry = state
                        .rehearsal_delivery_operations
                        .get_mut(&key)
                        .ok_or(StoreError::NotFound)?;
                    let prior = entry.generations.last().cloned().ok_or_else(|| {
                        StoreError::InvalidRecord(
                            "rehearsal delivery root has no generation".into(),
                        )
                    })?;
                    entry
                        .generations
                        .push(StoredRehearsalDeliveryGeneration::new(
                            operation,
                            prior.descriptor.clone(),
                            now,
                            None,
                        ));
                    Ok(crate::RehearsalDeliveryClaimResult::Prepared {
                        prepared: crate::PreparedRehearsalDelivery::mint(
                            request.locator,
                            operation,
                            prior.descriptor.clone(),
                        ),
                    })
                }
            };
        }
        // A Continue key identifies a retry request, never a second issue
        // cycle. Before a fresh root exists, the Store resumes the one
        // unresolved generation for this run regardless of its original key.
        // The write lock makes this check and any later root insertion one
        // atomic decision in Memory, mirroring the PostgreSQL broker.
        let mut accepted = std::collections::BTreeSet::new();
        for claim in state
            .rehearsal_submission_claims
            .iter()
            .filter(|(entry_key, _)| entry_key.0 == tenant && entry_key.1 == run.id)
            .map(|(_, claim)| claim)
        {
            let (_, snapshot) = hydrate_claim(&state, tenant, &run, claim)?;
            if snapshot.state() == domain::RehearsalSubmissionClaimState::Completed {
                accepted.insert(claim.attempt);
            }
        }
        let mut open = Vec::new();
        for generation in state
            .rehearsal_delivery_operations
            .iter()
            .filter(|(entry_key, _)| entry_key.0 == tenant && entry_key.1 == run.id)
            .filter_map(|(_, entry)| entry.generations.last())
        {
            if !accepted.contains(&generation.descriptor.attempt())
                && matches!(
                    generation.phase()?,
                    StoredRehearsalDeliveryPhase::Prepared
                        | StoredRehearsalDeliveryPhase::Dispatched
                )
            {
                open.push(generation.clone());
            }
        }
        if open.len() > 1 {
            return Err(StoreError::InvalidRecord(
                "rehearsal has more than one unresolved delivery".into(),
            ));
        }
        if let Some(generation) = open.into_iter().next() {
            if let Some(screen) = generation.screen {
                return Ok(crate::RehearsalDeliveryClaimResult::Replay(screen));
            }
            return match generation.phase()? {
                StoredRehearsalDeliveryPhase::Prepared => {
                    Ok(crate::RehearsalDeliveryClaimResult::Prepared {
                        prepared: crate::PreparedRehearsalDelivery::mint(
                            request.locator,
                            generation.operation,
                            generation.descriptor,
                        ),
                    })
                }
                StoredRehearsalDeliveryPhase::Dispatched => {
                    Ok(crate::RehearsalDeliveryClaimResult::Pending {
                        dispatched: crate::DispatchedRehearsalDelivery::mint(
                            request.locator,
                            generation.operation,
                        ),
                    })
                }
                StoredRehearsalDeliveryPhase::Completed => Err(StoreError::InvalidRecord(
                    "completed delivery lacks screen projection".into(),
                )),
                StoredRehearsalDeliveryPhase::Expired => {
                    Ok(crate::RehearsalDeliveryClaimResult::Expired)
                }
                StoredRehearsalDeliveryPhase::RunTimeExhaustedBeforeDispatch => {
                    Ok(crate::RehearsalDeliveryClaimResult::RunTimeExhausted {
                        deadline: generation.run_time_exhausted_deadline.ok_or_else(|| {
                            StoreError::Unavailable("run-time exhaustion lacks deadline".into())
                        })?,
                    })
                }
                StoredRehearsalDeliveryPhase::AbandonedBeforeDispatch => unreachable!(),
            };
        }
        // A prepared retry can become terminal between retry preparation and
        // dispatch when the run-wide clock reaches its cap.  That terminal
        // result is stronger than the predecessor's expiry: any new Continue
        // key must replay it rather than exposing the earlier retryable state
        // (ASVS 2.3.1, 2.3.3).
        let mut run_time_exhausted_deadline = None;
        for generation in state
            .rehearsal_delivery_operations
            .iter()
            .filter(|(entry_key, _)| entry_key.0 == tenant && entry_key.1 == run.id)
            .filter_map(|(_, entry)| entry.generations.last())
        {
            if generation.phase()? == StoredRehearsalDeliveryPhase::RunTimeExhaustedBeforeDispatch {
                run_time_exhausted_deadline = generation.run_time_exhausted_deadline;
                break;
            }
        }
        if let Some(deadline) = run_time_exhausted_deadline {
            return Ok(crate::RehearsalDeliveryClaimResult::RunTimeExhausted { deadline });
        }
        let mut has_expired = false;
        for generation in state
            .rehearsal_delivery_operations
            .iter()
            .filter(|(entry_key, _)| entry_key.0 == tenant && entry_key.1 == run.id)
            .filter_map(|(_, entry)| entry.generations.last())
        {
            if generation.phase()? == StoredRehearsalDeliveryPhase::Expired {
                has_expired = true;
                break;
            }
        }
        if has_expired {
            return Ok(crate::RehearsalDeliveryClaimResult::Expired);
        }
        active_current(&state, tenant, &run)?;
        let operation = crate::RehearsalOperationId::from_uuid(fresh_uuid()?);
        let frozen_inventory = state
            .rehearsal_frozen_items
            .iter()
            .filter_map(|((entry_tenant, entry_run, _), frozen)| {
                (*entry_tenant == tenant && *entry_run == run.id).then_some(frozen.clone())
            })
            .collect::<Vec<_>>();
        if frozen_inventory.iter().any(|item| {
            !state
                .rehearsal_frozen_source_snapshots
                .contains_key(&(tenant, run.id, item.attempt))
        }) {
            return Err(StoreError::Unavailable(
                "rehearsal source snapshot is absent".into(),
            ));
        }
        let frozen = frozen_inventory
            .into_iter()
            .filter(|item| !accepted.contains(&item.attempt))
            .min_by_key(|item| {
                state
                    .rehearsal_frozen_source_snapshots
                    .get(&(tenant, run.id, item.attempt))
                    .map(|source| source.ordinal)
                    .unwrap_or(usize::MAX)
            })
            .ok_or(StoreError::Conflict)?;
        let descriptor = crate::RehearsalDeliveryExecutionDescriptorV1::from_frozen(
            frozen.clone(),
            deterministic_rehearsal_seed(&run, &frozen),
            1,
        );
        let now = state.authoritative_time;
        state.rehearsal_delivery_operations.insert(
            key,
            StoredRehearsalDeliveryOperation {
                fingerprint: request.request_fingerprint,
                generations: vec![StoredRehearsalDeliveryGeneration::new(
                    operation,
                    descriptor.clone(),
                    now,
                    None,
                )],
            },
        );
        Ok(crate::RehearsalDeliveryClaimResult::Prepared {
            prepared: crate::PreparedRehearsalDelivery::mint(
                request.locator,
                operation,
                descriptor,
            ),
        })
    }

    async fn mark_rehearsal_delivery_dispatched(
        &self,
        context: TenantContext,
        prepared: crate::PreparedRehearsalDelivery,
    ) -> Result<crate::RehearsalDeliveryDispatchResult, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let locator = prepared.locator();
        let operation = prepared.operation();
        let (assignment, owner) = authorize_locator(&state, tenant, locator)?;
        let run = authorized_run(&state, tenant, locator, assignment, owner)?.clone();
        active_current(&state, tenant, &run)?;
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        let descriptor = state
            .rehearsal_delivery_operations
            .values()
            .flat_map(|entry| entry.generations.iter())
            .find(|entry| entry.operation == operation)
            .map(|entry| entry.descriptor.clone())
            .ok_or(StoreError::NotFound)?;
        let timing = timing_witness_for_dispatch(
            &state,
            tenant,
            &run,
            &descriptor,
            state.authoritative_time,
        )?;
        let frozen_binding = state
            .rehearsal_frozen_items
            .get(&(tenant, run.id, descriptor.attempt()))
            .cloned()
            .ok_or_else(missing_rehearsal_material)?;
        let now = state.authoritative_time;
        let generation = state
            .rehearsal_delivery_operations
            .values_mut()
            .flat_map(|entry| entry.generations.iter_mut())
            .find(|entry| entry.operation == operation)
            .ok_or(StoreError::NotFound)?;
        if generation.phase()? != StoredRehearsalDeliveryPhase::Prepared
            || generation.screen.is_some()
        {
            return Err(StoreError::Conflict);
        }
        let dispatched = crate::DispatchedRehearsalDelivery::mint(locator, operation);
        match timing {
            domain::RehearsalTimingDispatchDecisionV1::Witness(witness) => {
                generation.append_phase(StoredRehearsalDeliveryPhase::Dispatched, now)?;
                if generation
                    .frozen_binding
                    .as_ref()
                    .is_some_and(|bound| bound != &frozen_binding)
                {
                    return Err(StoreError::InvalidRecord(
                        "delivery dispatch disagrees with frozen binding".into(),
                    ));
                }
                generation.frozen_binding = Some(frozen_binding);
                generation.timing_witness = Some(witness);
                Ok(crate::RehearsalDeliveryDispatchResult::Dispatched { dispatched })
            }
            domain::RehearsalTimingDispatchDecisionV1::RunTimeExhausted { deadline } => {
                generation.append_phase(
                    StoredRehearsalDeliveryPhase::RunTimeExhaustedBeforeDispatch,
                    now,
                )?;
                generation.run_time_exhausted_deadline = Some(deadline);
                Ok(crate::RehearsalDeliveryDispatchResult::RunTimeExhausted { deadline })
            }
        }
    }

    async fn complete_rehearsal_delivery(
        &self,
        context: TenantContext,
        command: crate::RehearsalDeliveryCompletionCommand,
    ) -> Result<question_model::RehearsalActiveScreenV1, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let locator = command.dispatched.locator();
        let operation = command.dispatched.operation();
        let (assignment, owner) = authorize_locator(&state, tenant, locator)?;
        let run = authorized_run(&state, tenant, locator, assignment, owner)?.clone();
        active_current(&state, tenant, &run)?;
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        let issued_frozen = {
            let planned = state
                .rehearsal_delivery_operations
                .values()
                .flat_map(|entry| entry.generations.iter())
                .find(|entry| entry.operation == operation)
                .ok_or(StoreError::NotFound)?;
            let attempt = planned.descriptor.attempt();
            let frozen = state
                .rehearsal_frozen_items
                .get(&(tenant, run.id, attempt))
                .cloned()
                .ok_or(StoreError::Conflict)?;
            let descriptor = planned.descriptor.clone();
            if frozen.problem != descriptor.problem()
                || frozen.response_definition != *descriptor.response_definition()
                || frozen.canonical_content_digest != descriptor.frozen_content_digest()
            {
                return Err(StoreError::InvalidRecord(
                    "frozen rehearsal item does not match selected descriptor".into(),
                ));
            }
            if state
                .rehearsal_delivery_operations
                .values()
                .flat_map(|entry| entry.generations.iter())
                .filter_map(|other| other.frozen_binding.as_ref())
                .any(|bound| bound.attempt == frozen.attempt && bound != &frozen)
            {
                return Err(StoreError::InvalidRecord(
                    "rehearsal delivery generations disagree about frozen binding".into(),
                ));
            }
            Some(frozen)
        };
        let issued_artifact = state
            .rehearsal_delivery_operations
            .values()
            .flat_map(|entry| entry.generations.iter())
            .find(|entry| entry.operation == operation)
            .and_then(|entry| entry.issued_execution.clone())
            .ok_or(StoreError::Conflict)?;
        let generation_for_artifact = rehearsal_generation(&state, tenant, run.id, operation)?;
        let issue_work = rehearsal_issue_work(&state, tenant, run.id, generation_for_artifact)?;
        // A typed browser screen is not execution authority.  Require a fully
        // hydrated committed artifact before its first visible completion.
        let _execution = issued_artifact.decode_for_work(&issue_work)?;
        let expected_screen = issued_artifact.active_screen()?;
        if command.screen != expected_screen {
            return Err(StoreError::Conflict);
        }
        let now = state.authoritative_time;
        let generation = state
            .rehearsal_delivery_operations
            .values_mut()
            .flat_map(|entry| entry.generations.iter_mut())
            .find(|entry| entry.operation == operation)
            .ok_or(StoreError::NotFound)?;
        if generation.phase()? != StoredRehearsalDeliveryPhase::Dispatched
            || generation.screen.is_some()
        {
            return Err(StoreError::Conflict);
        }
        if let Some(frozen) = issued_frozen {
            if generation
                .frozen_binding
                .as_ref()
                .is_some_and(|bound| bound != &frozen)
            {
                return Err(StoreError::InvalidRecord(
                    "rehearsal delivery generation has a mismatched frozen binding".into(),
                ));
            }
            generation.frozen_binding = Some(frozen);
        }
        let screen_digest = command.screen.commitment().map_err(|error| {
            StoreError::InvalidRecord(format!("invalid rehearsal active screen: {error:?}"))
        })?;
        generation.append_phase(StoredRehearsalDeliveryPhase::Completed, now)?;
        generation.screen = Some(command.screen.clone());
        generation.screen_digest = Some(screen_digest);
        Ok(command.screen)
    }
}

#[async_trait]
impl crate::RehearsalDeliveryPreDispatchCompensationStore for MemoryStore {
    async fn abandon_rehearsal_delivery_before_dispatch(
        &self,
        context: TenantContext,
        prepared: crate::PreparedRehearsalDelivery,
        _reason: crate::RehearsalDeliveryPreDispatchAbandonReason,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let locator = prepared.locator();
        let operation = prepared.operation();
        let (assignment, owner) = authorize_locator(&state, tenant, locator)?;
        let run = authorized_run(&state, tenant, locator, assignment, owner)?.clone();
        active_current(&state, tenant, &run)?;
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        let key = state
            .rehearsal_delivery_operations
            .iter()
            .find_map(|(key, entry)| {
                (key.0 == tenant
                    && key.1 == run.id
                    && entry
                        .generations
                        .iter()
                        .any(|generation| generation.operation == operation))
                .then_some(key.clone())
            })
            .ok_or(StoreError::NotFound)?;
        let now = state.authoritative_time;
        let generation = state
            .rehearsal_delivery_operations
            .get_mut(&key)
            .and_then(|entry| {
                entry
                    .generations
                    .iter_mut()
                    .find(|generation| generation.operation == operation)
            })
            .ok_or(StoreError::NotFound)?;
        if generation.phase()? != StoredRehearsalDeliveryPhase::Prepared
            || generation.screen.is_some()
        {
            return Err(StoreError::Conflict);
        }
        generation.append_phase(StoredRehearsalDeliveryPhase::AbandonedBeforeDispatch, now)?;
        Ok(())
    }
}

#[async_trait]
impl crate::SealedRehearsalDeliveryExecutionStore for super::MemorySealedPrivateExecutionStore {
    async fn prepare_or_resume_issued_execution(
        &self,
        context: TenantContext,
        dispatched: &crate::DispatchedRehearsalDelivery,
    ) -> Result<crate::SealedRehearsalDeliveryIssuePreparation, StoreError> {
        let state = self.state.read().map_err(|_| {
            StoreError::Unavailable("sealed rehearsal execution state is unavailable".into())
        })?;
        let tenant = context.tenant_id();
        let locator = dispatched.locator();
        let (assignment, owner) = authorize_locator(&state, tenant, locator)?;
        let run = authorized_run(&state, tenant, locator, assignment, owner)?;
        verify_rehearsal_aggregate(&state, tenant, run)?;
        let generation = rehearsal_generation(&state, tenant, run.id, dispatched.operation())?;
        if !matches!(
            generation.phase()?,
            StoredRehearsalDeliveryPhase::Dispatched | StoredRehearsalDeliveryPhase::Completed
        ) {
            return Err(StoreError::Conflict);
        }
        let work = rehearsal_issue_work(&state, tenant, run.id, generation)?;
        if let Some(artifact) = &generation.issued_execution {
            return Ok(
                crate::SealedRehearsalDeliveryIssuePreparation::ExistingArtifact(Box::new(
                    artifact.decode_for_work(&work)?,
                )),
            );
        }
        Ok(crate::SealedRehearsalDeliveryIssuePreparation::IssueWork(
            Box::new(work),
        ))
    }

    async fn commit_issued_execution(
        &self,
        context: TenantContext,
        work: crate::SealedRehearsalDeliveryIssueWork,
        artifact: crate::RehearsalIssuedExecutionArtifactV1,
    ) -> Result<crate::SealedRehearsalDeliveryExecution, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.state.write().map_err(|_| {
            StoreError::Unavailable("sealed rehearsal execution state is unavailable".into())
        })?;
        let found = state
            .rehearsal_delivery_operations
            .iter()
            .find_map(|((entry_tenant, run, _), entry)| {
                (*entry_tenant == tenant
                    && entry
                        .generations
                        .iter()
                        .any(|g| g.operation == work.operation()))
                .then_some(*run)
            })
            .ok_or(StoreError::NotFound)?;
        let generation = rehearsal_generation(&state, tenant, found, work.operation())?.clone();
        if !matches!(
            generation.phase()?,
            StoredRehearsalDeliveryPhase::Dispatched | StoredRehearsalDeliveryPhase::Completed
        ) {
            return Err(StoreError::Conflict);
        }
        let expected = rehearsal_issue_work(&state, tenant, found, &generation)?;
        let execution = artifact.decode_for_work(&expected)?;
        let stored = state
            .rehearsal_delivery_operations
            .values_mut()
            .flat_map(|entry| entry.generations.iter_mut())
            .find(|generation| generation.operation == work.operation())
            .ok_or(StoreError::NotFound)?;
        match &stored.issued_execution {
            Some(existing) if existing.bytes() == artifact.bytes() => Ok(execution),
            Some(_) => Err(StoreError::Conflict),
            None => {
                stored.issued_execution = Some(artifact);
                Ok(execution)
            }
        }
    }

    async fn prepare_sealed_rehearsal_delivery_execution(
        &self,
        context: TenantContext,
        dispatched: &crate::DispatchedRehearsalDelivery,
    ) -> Result<crate::SealedRehearsalDeliveryExecution, StoreError> {
        match self
            .prepare_or_resume_issued_execution(context, dispatched)
            .await?
        {
            crate::SealedRehearsalDeliveryIssuePreparation::ExistingArtifact(execution) => {
                Ok(*execution)
            }
            crate::SealedRehearsalDeliveryIssuePreparation::IssueWork(_) => {
                Err(StoreError::Conflict)
            }
        }
    }
}

fn rehearsal_generation(
    state: &State,
    tenant: TenantId,
    run: question_model::RehearsalRunId,
    operation: crate::RehearsalOperationId,
) -> Result<&StoredRehearsalDeliveryGeneration, StoreError> {
    state
        .rehearsal_delivery_operations
        .iter()
        .filter(|((entry_tenant, entry_run, _), _)| *entry_tenant == tenant && *entry_run == run)
        .flat_map(|(_, entry)| entry.generations.iter())
        .find(|generation| generation.operation == operation)
        .ok_or(StoreError::NotFound)
}

pub(in crate::in_memory) fn rehearsal_issue_work(
    state: &State,
    tenant: TenantId,
    run: question_model::RehearsalRunId,
    generation: &StoredRehearsalDeliveryGeneration,
) -> Result<crate::SealedRehearsalDeliveryIssueWork, StoreError> {
    let key = (tenant, run, generation.descriptor.attempt());
    let source = state
        .rehearsal_frozen_source_snapshots
        .get(&key)
        .ok_or_else(missing_rehearsal_material)?;
    let private = state
        .rehearsal_frozen_private_execution
        .get(&key)
        .ok_or_else(missing_rehearsal_material)?;
    let bytes = decode_rehearsal_private_checksum(&private.checksum)?;
    Ok(crate::SealedRehearsalDeliveryIssueWork::new(
        generation.operation,
        generation.descriptor.clone(),
        source.snapshot.clone(),
        private.execution.clone(),
        crate::RehearsalOperationDigest::from_bytes(bytes),
    ))
}

/// Reconstructs the one durable completion request from an authenticated
/// rendered claim.  This is deliberately Store-private: the coordinator sees
/// grading parts only, while completion independently replays the sealed
/// artifact translation before recording original rendered evidence.
pub(in crate::in_memory) fn durable_request_for_claim_completion(
    state: &State,
    tenant: TenantId,
    run: question_model::RehearsalRunId,
    claim: &StoredRehearsalClaim,
    root: &RehearsalClaimRoot,
) -> Result<RehearsalValidatedSubmissionRequest, StoreError> {
    if let Some(request) = root.submission_input().durable_request() {
        return Ok(request.clone());
    }
    let binding = claim.route_delivery.ok_or(StoreError::Conflict)?;
    let generation = rehearsal_generation(state, tenant, run, binding.operation)?;
    if generation.descriptor.attempt() != claim.attempt
        || generation.screen_digest != Some(binding.screen_digest)
    {
        return Err(StoreError::Conflict);
    }
    let work = rehearsal_issue_work(state, tenant, run, generation)?;
    let artifact = generation.issued_execution.as_ref().ok_or_else(|| {
        StoreError::Unavailable("rendered claim has no committed issued artifact".into())
    })?;
    let execution = artifact.decode_for_work(&work)?;
    let screen = execution.active_screen()?;
    let commitment = screen
        .commitment()
        .map_err(|_| StoreError::InvalidRecord("issued execution screen is invalid".into()))?;
    if commitment != binding.screen_digest
        || root.submission_input().presentation_commitment() != Some(commitment)
    {
        return Err(StoreError::Conflict);
    }
    let grading = execution.into_grading_parts(root.submission_input().original_response())?;
    let frozen = state
        .rehearsal_frozen_items
        .get(&(tenant, run, claim.attempt))
        .ok_or(StoreError::NotFound)?;
    RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
        frozen,
        claim.attempt,
        grading.response().clone(),
    )
    .map_err(|_| StoreError::Conflict)
}

#[async_trait]
impl crate::SealedRehearsalSubmissionExecutionStore for super::MemorySealedPrivateExecutionStore {
    async fn prepare_or_resume_sealed_rehearsal_submission_execution(
        &self,
        context: TenantContext,
        route: crate::RehearsalRouteIdentity,
        idempotency_key: crate::RehearsalSubmissionIdempotencyKey,
    ) -> Result<crate::SealedRehearsalSubmissionExecutionPreparation, StoreError> {
        let state = self.state.read().map_err(|_| {
            StoreError::Unavailable("sealed rehearsal execution state is unavailable".into())
        })?;
        let tenant = context.tenant_id();
        let locator = super::mutations::route_locator(&state, tenant, route)?;
        let (assignment, owner) = authorize_locator(&state, tenant, locator)?;
        let run = authorized_run(&state, tenant, locator, assignment, owner)?;
        verify_rehearsal_aggregate(&state, tenant, run)?;
        let key = (tenant, run.id, idempotency_key);
        let claim = state
            .rehearsal_submission_claims
            .get(&key)
            .ok_or(StoreError::NotFound)?;
        let (root, snapshot) = hydrate_claim(&state, tenant, run, claim)?;
        // Live route recovery accepts only the authenticated rendered form.
        // Generic Durable test-support claims have no route delivery binding
        // and retain their direct frozen completion path instead.
        if !matches!(
            root.submission_input(),
            domain::RehearsalClaimSubmissionInput::Rendered(_)
        ) {
            return Err(StoreError::Conflict);
        }
        let binding = claim.route_delivery.ok_or(StoreError::Conflict)?;
        match snapshot.state() {
            domain::RehearsalSubmissionClaimState::Completed => {
                let receipt = claim.receipt.as_ref().ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "completed rehearsal claim has no immutable receipt".into(),
                    )
                })?;
                Ok(
                    crate::SealedRehearsalSubmissionExecutionPreparation::Receipt(
                        crate::RehearsalSubmissionReceipt {
                            outcome: receipt.outcome.clone(),
                            replayed: true,
                        },
                    ),
                )
            }
            domain::RehearsalSubmissionClaimState::Prepared => {
                if claim.receipt.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "prepared rehearsal claim has receipt".into(),
                    ));
                }
                Ok(crate::SealedRehearsalSubmissionExecutionPreparation::PendingPreparation)
            }
            domain::RehearsalSubmissionClaimState::GradingDispatched => {
                let handle = snapshot
                    .into_dispatched_handle()
                    .map_err(|_| StoreError::Conflict)?;
                let generation = rehearsal_generation(&state, tenant, run.id, binding.operation)?;
                if generation.descriptor.attempt() != claim.attempt
                    || generation.screen_digest != Some(binding.screen_digest)
                    || !matches!(
                        generation.phase()?,
                        StoredRehearsalDeliveryPhase::Dispatched
                            | StoredRehearsalDeliveryPhase::Completed
                    )
                {
                    return Err(StoreError::Conflict);
                }
                let work = rehearsal_issue_work(&state, tenant, run.id, generation)?;
                let artifact = generation.issued_execution.as_ref().ok_or_else(|| {
                    StoreError::Unavailable(
                        "dispatched rehearsal claim has no committed issued artifact".into(),
                    )
                })?;
                let execution = artifact.decode_for_work(&work)?;
                let screen = execution.active_screen()?;
                let actual_digest = screen.commitment().map_err(|_| {
                    StoreError::InvalidRecord("issued execution screen is invalid".into())
                })?;
                if actual_digest != binding.screen_digest {
                    return Err(StoreError::Conflict);
                }
                if root.submission_input().presentation_commitment() != Some(actual_digest) {
                    return Err(StoreError::Conflict);
                }
                let grading =
                    execution.into_grading_parts(root.submission_input().original_response())?;
                // The artifact is the only rendered-to-durable translator.
                // Revalidate that output against the aggregate-frozen durable
                // schema before a grader can receive it (ASVS V2.2.1, V2.3.1).
                let frozen = state
                    .rehearsal_frozen_items
                    .get(&(tenant, run.id, claim.attempt))
                    .cloned()
                    .ok_or(StoreError::NotFound)?;
                let durable_request = RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
                    &frozen,
                    claim.attempt,
                    grading.response().clone(),
                )
                .map_err(|_| StoreError::Conflict)?;
                let completion = crate::SealedRehearsalSubmissionCompletion::new(
                    crate::contracts::SealedRehearsalSubmissionCompletionParts {
                        context,
                        route,
                        handle,
                        root,
                        attempt: claim.attempt,
                        frozen,
                        expected_evidence_head: run.evidence_head,
                        presentation_commitment: actual_digest,
                        durable_request,
                    },
                );
                Ok(crate::SealedRehearsalSubmissionExecutionPreparation::Work(
                    Box::new(crate::SealedRehearsalSubmissionExecutionWork::new(
                        grading, completion,
                    )),
                ))
            }
            domain::RehearsalSubmissionClaimState::AbandonedBeforeDispatch
            | domain::RehearsalSubmissionClaimState::RevokedStaleRevision
            | domain::RehearsalSubmissionClaimState::RevokedTerminalLifecycle
            | domain::RehearsalSubmissionClaimState::RevokedSourceContextRemoved => {
                Err(StoreError::Conflict)
            }
        }
    }

    async fn complete_sealed_rehearsal_submission_execution(
        &self,
        context: TenantContext,
        completion: crate::SealedRehearsalSubmissionCompletion,
        grading: question_model::RehearsalPrivateGradingResult,
    ) -> Result<crate::RehearsalSubmissionReceipt, StoreError> {
        super::complete_sealed_submission_locked(&self.state, context, completion, grading)
    }
}

pub(in crate::in_memory) fn decode_rehearsal_private_checksum(
    value: &str,
) -> Result<[u8; 32], StoreError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(StoreError::InvalidRecord(
            "rehearsal private checksum is invalid".into(),
        ));
    }
    let mut result = [0; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(pair).map_err(|_| {
            StoreError::InvalidRecord("rehearsal private checksum is invalid".into())
        })?;
        result[index] = u8::from_str_radix(text, 16).map_err(|_| {
            StoreError::InvalidRecord("rehearsal private checksum is invalid".into())
        })?;
    }
    Ok(result)
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
