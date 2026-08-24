use super::*;
use crate::contracts::RehearsalInternalStore;
use crate::in_memory::rehearsal::types::StoredRehearsalClaimDeliveryBinding;

pub(in crate::in_memory) fn route_locator(
    state: &State,
    tenant: TenantId,
    route: crate::RehearsalRouteIdentity,
) -> Result<crate::RehearsalLocator, StoreError> {
    let (assignment, owner) = authorize_assignment(
        state,
        tenant,
        route.actor,
        route.course,
        route.assignment,
        route.expected_revision,
    )?;
    let locator = crate::RehearsalLocator {
        actor: route.actor,
        course: route.course,
        assignment: route.assignment,
        revision: route.expected_revision,
        rehearsal: route.rehearsal,
    };
    let _ = authorized_run(state, tenant, locator, assignment, owner)?;
    Ok(locator)
}

#[derive(Clone, Copy)]
pub(in crate::in_memory) struct RouteClaimDeliveryBindingInput {
    pub(in crate::in_memory) attempt: RehearsalAttemptId,
    pub(in crate::in_memory) operation: crate::RehearsalOperationId,
    pub(in crate::in_memory) screen_digest: question_model::RehearsalPresentationDigestV1,
}

pub(in crate::in_memory) fn route_claim_delivery_binding(
    root: &RehearsalClaimRoot,
    input: RouteClaimDeliveryBindingInput,
) -> StoredRehearsalClaimDeliveryBinding {
    let mut bytes = Vec::with_capacity(32 * 4 + 16 * 3);
    bytes.extend_from_slice(b"ple.rehearsal.claim-delivery-binding.v1\0");
    bytes.extend_from_slice(root.rehearsal().as_uuid().as_bytes());
    bytes.extend_from_slice(root.claim().as_uuid().as_bytes());
    bytes.extend_from_slice(&root.fingerprint().as_bytes());
    bytes.extend_from_slice(input.attempt.as_uuid().as_bytes());
    bytes.extend_from_slice(input.operation.as_uuid().as_bytes());
    bytes.extend_from_slice(&input.screen_digest.as_bytes());
    StoredRehearsalClaimDeliveryBinding {
        operation: input.operation,
        screen_digest: input.screen_digest,
        binding_digest: crate::RehearsalOperationDigest::from_bytes(
            *objects::Sha256Digest::compute(&bytes).as_bytes(),
        ),
    }
}

/// Derives the first route-claim grade operation from the complete, private
/// binding digest. The digest already commits the run, claim, request
/// fingerprint, frozen attempt, issued delivery operation, and full screen
/// commitment. SHA-256 output is normalized to an RFC 4122 variant, v5 UUID
/// solely because the domain operation identity is UUID-shaped.
pub(in crate::in_memory) fn route_claim_initial_grade_operation(
    binding: StoredRehearsalClaimDeliveryBinding,
) -> RehearsalGradeOperationId {
    let mut bytes = Vec::with_capacity(80);
    bytes.extend_from_slice(b"ple.rehearsal.initial-route-claim-operation.v1\0");
    bytes.extend_from_slice(&binding.binding_digest.as_bytes());
    let mut uuid_bytes = [0_u8; 16];
    uuid_bytes.copy_from_slice(&objects::Sha256Digest::compute(&bytes).as_bytes()[..16]);
    uuid_bytes[6] = (uuid_bytes[6] & 0x0F) | 0x50;
    uuid_bytes[8] = (uuid_bytes[8] & 0x3F) | 0x80;
    RehearsalGradeOperationId::from_uuid(uuid::Uuid::from_bytes(uuid_bytes))
}

/// Atomically admits either a generic durable test-support request or an
/// already screen-validated live rendered request. Both return only a
/// pre-grader handle: sealed preparation is the sole translation boundary.
#[allow(clippy::too_many_arguments)]
fn claim_submission_input_locked(
    state: &mut State,
    tenant: TenantId,
    locator: crate::RehearsalLocator,
    attempt: RehearsalAttemptId,
    input: RehearsalClaimSubmissionInput,
    idempotency_key: crate::RehearsalIdempotencyKey,
    route_delivery: Option<RouteClaimDeliveryBindingInput>,
) -> Result<crate::RehearsalSubmissionClaimResult, StoreError> {
    let (assignment, owner) = authorize_locator(state, tenant, locator)?;
    let run = authorized_run(state, tenant, locator, assignment, owner)?.clone();
    verify_rehearsal_aggregate(state, tenant, &run)?;
    let frozen = state
        .rehearsal_frozen_items
        .get(&(tenant, run.id, attempt))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let fingerprint =
        rehearsal_claim_submission_input_fingerprint(genesis(&run, tenant), &frozen, &input)
            .map_err(|error| {
                StoreError::InvalidRecord(format!("invalid rehearsal request: {error:?}"))
            })?;
    let key = (tenant, run.id, idempotency_key);
    let existing = state
        .rehearsal_submission_claims
        .get(&key)
        .map(|claim| hydrate_claim(state, tenant, &run, claim))
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
    let route_delivery = route_delivery.map(|input| {
        route_claim_delivery_binding(
            existing
                .as_ref()
                .map(|(existing_root, _)| existing_root)
                .unwrap_or(&root),
            input,
        )
    });
    if let Some((_, snapshot)) = &existing
        && matches!(
            snapshot.state(),
            domain::RehearsalSubmissionClaimState::Prepared
                | domain::RehearsalSubmissionClaimState::GradingDispatched
                | domain::RehearsalSubmissionClaimState::Completed
        )
    {
        let stored = state
            .rehearsal_submission_claims
            .get(&key)
            .ok_or(StoreError::NotFound)?;
        if stored.route_delivery != route_delivery {
            return Err(StoreError::Conflict);
        }
    }
    let new_operation = match route_delivery {
        Some(binding) => route_claim_initial_grade_operation(binding),
        None => RehearsalGradeOperationId::from_uuid(fresh_uuid()?),
    };
    match decide_submission_claim(
        run.lifecycle,
        revision_is_current(state, tenant, &run)?,
        existing.as_ref().map(|(_, snapshot)| snapshot),
        fingerprint,
        &root,
        new_operation,
    ) {
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
                    attempt,
                    submission_input: domain::rehearsal::persistence::encode_claim_submission_input(
                        root.submission_input(),
                    ),
                    events: vec![event],
                    receipt: None,
                    route_delivery,
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
            if claim.route_delivery != route_delivery {
                return Err(StoreError::Conflict);
            }
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
            hydrate_claim_history(&existing_root, &events, None).map_err(invalid_claim_history)?;
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
        RehearsalSubmissionClaimDecision::StaleRevision
        | RehearsalSubmissionClaimDecision::TerminalLifecycle => Err(StoreError::Conflict),
    }
}

#[async_trait]
impl crate::RehearsalRouteMutationStore for MemoryStore {
    async fn claim_rehearsal_delivery_from_route(
        &self,
        context: TenantContext,
        command: crate::ClaimRehearsalDeliveryRouteCommand,
    ) -> Result<crate::RehearsalDeliveryClaimResult, StoreError> {
        let locator = {
            let state = self.read_state()?;
            route_locator(&state, context.tenant_id(), command.route)?
        };
        self.claim_rehearsal_delivery(
            context,
            crate::RehearsalDeliveryRequest {
                locator,
                idempotency_key: command.idempotency_key,
                request_fingerprint: command.request_fingerprint,
            },
        )
        .await
    }
    async fn reconcile_rehearsal_delivery_expiry_from_route(
        &self,
        context: TenantContext,
        command: crate::ReconcileRehearsalDeliveryExpiryRouteCommand,
    ) -> Result<crate::RehearsalDeliveryTimingResult, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let locator = route_locator(&state, tenant, command.route)?;
        let (assignment, owner) = authorize_locator(&state, tenant, locator)?;
        let run = authorized_run(&state, tenant, locator, assignment, owner)?.clone();
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        active_current(&state, tenant, &run)?;
        let mut accepted = std::collections::BTreeSet::new();
        for claim in state
            .rehearsal_submission_claims
            .iter()
            .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
            .map(|(_, claim)| claim)
        {
            let (_, snapshot) = hydrate_claim(&state, tenant, &run, claim)?;
            if snapshot.state() == domain::RehearsalSubmissionClaimState::Completed {
                accepted.insert(claim.attempt);
            }
        }
        let candidates = state
            .rehearsal_delivery_operations
            .iter()
            .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
            .filter_map(|(_, delivery)| delivery.generations.last())
            .filter(|generation| {
                matches!(
                    generation.phase(),
                    Ok(StoredRehearsalDeliveryPhase::Dispatched
                        | StoredRehearsalDeliveryPhase::Completed
                        | StoredRehearsalDeliveryPhase::Expired)
                ) && !accepted.contains(&generation.descriptor.attempt())
            })
            .map(|generation| generation.operation)
            .collect::<Vec<_>>();
        let [operation] = candidates.as_slice() else {
            return Err(if candidates.is_empty() {
                StoreError::Conflict
            } else {
                StoreError::InvalidRecord("rehearsal has more than one issued delivery".into())
            });
        };
        let expired = reconcile_delivery_expiry_locked(&mut state, tenant, &run, *operation)?;
        let witness = state
            .rehearsal_delivery_operations
            .values()
            .flat_map(|delivery| delivery.generations.iter())
            .find(|generation| generation.operation == *operation)
            .and_then(|generation| generation.timing_witness)
            .ok_or_else(|| {
                StoreError::Unavailable("dispatched rehearsal delivery lacks timing witness".into())
            })?;
        let verdict = if expired {
            domain::RehearsalTimingVerdictV1::Expired
        } else {
            domain::rehearsal_timing_verdict(witness, state.authoritative_time).map_err(
                |error| {
                    StoreError::Unavailable(format!(
                        "rehearsal timing cannot be evaluated: {error}"
                    ))
                },
            )?
        };
        let expires_at = match (witness.deadline(), witness.grace_millis()) {
            (Some(deadline), Some(grace)) => Some(
                deadline
                    .as_unix_millis()
                    .checked_add(grace)
                    .map(question_model::ActivityTimestamp::from_unix_millis)
                    .ok_or_else(|| {
                        StoreError::Unavailable("rehearsal expiry overflows timestamp".into())
                    })?,
            ),
            _ => None,
        };
        let retry_disposition = if verdict == domain::RehearsalTimingVerdictV1::Expired {
            if domain::rehearsal_retry_is_available(witness, state.authoritative_time).map_err(
                |error| {
                    StoreError::Unavailable(format!("rehearsal retry cannot be evaluated: {error}"))
                },
            )? {
                crate::RehearsalDeliveryRetryDisposition::Available
            } else {
                crate::RehearsalDeliveryRetryDisposition::RunTimeExhausted
            }
        } else {
            crate::RehearsalDeliveryRetryDisposition::NotApplicable
        };
        Ok(crate::RehearsalDeliveryTimingResult {
            verdict,
            deadline: witness.deadline(),
            expires_at,
            retry_disposition,
        })
    }
    async fn retry_rehearsal_delivery_from_route(
        &self,
        context: TenantContext,
        command: crate::RetryRehearsalDeliveryRouteCommand,
    ) -> Result<crate::RetryRehearsalDeliveryResult, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let locator = route_locator(&state, tenant, command.route)?;
        let (assignment, owner) = authorize_locator(&state, tenant, locator)?;
        let run = authorized_run(&state, tenant, locator, assignment, owner)?.clone();
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        active_current(&state, tenant, &run)?;
        let retry_key = (tenant, run.id, command.idempotency_key.clone());
        if let Some(retry) = state.rehearsal_delivery_retries.get(&retry_key) {
            if retry.fingerprint != command.request_fingerprint {
                return Ok(crate::RetryRehearsalDeliveryResult::Conflict);
            }
            let generation = state
                .rehearsal_delivery_operations
                .get(&(tenant, run.id, retry.root_key.clone()))
                .and_then(|root| {
                    root.generations
                        .iter()
                        .find(|entry| entry.operation == retry.operation)
                })
                .ok_or_else(|| {
                    StoreError::Unavailable(
                        "rehearsal retry points to absent delivery generation".into(),
                    )
                })?;
            // Terminal results take precedence over a predecessor screen. An
            // expired generation has an issued screen by definition, but its
            // retry key must replay the terminal decision, not that stale
            // browser projection (ASVS 2.3.1).
            match generation.phase()? {
                StoredRehearsalDeliveryPhase::Expired => {
                    if retry.terminal_predecessor {
                        return Ok(crate::RetryRehearsalDeliveryResult::RunTimeExhausted {
                            deadline: generation
                                .timing_witness
                                .and_then(|witness| witness.deadline())
                                .ok_or_else(|| {
                                    StoreError::Unavailable("expired retry lacks deadline".into())
                                })?,
                        });
                    }
                    return Ok(crate::RetryRehearsalDeliveryResult::Conflict);
                }
                StoredRehearsalDeliveryPhase::RunTimeExhaustedBeforeDispatch => {
                    return Ok(crate::RetryRehearsalDeliveryResult::RunTimeExhausted {
                        deadline: generation.run_time_exhausted_deadline.ok_or_else(|| {
                            StoreError::Unavailable("run-time exhaustion lacks deadline".into())
                        })?,
                    });
                }
                StoredRehearsalDeliveryPhase::Prepared
                | StoredRehearsalDeliveryPhase::Dispatched
                | StoredRehearsalDeliveryPhase::Completed
                | StoredRehearsalDeliveryPhase::AbandonedBeforeDispatch => {}
            }
            if let Some(screen) = &generation.screen {
                return Ok(crate::RetryRehearsalDeliveryResult::Replay(screen.clone()));
            }
            return match generation.phase()? {
                StoredRehearsalDeliveryPhase::Prepared => {
                    Ok(crate::RetryRehearsalDeliveryResult::Prepared {
                        prepared: crate::PreparedRehearsalDelivery::mint(
                            locator,
                            generation.operation,
                            generation.descriptor.clone(),
                        ),
                    })
                }
                StoredRehearsalDeliveryPhase::Dispatched => {
                    Ok(crate::RetryRehearsalDeliveryResult::Pending {
                        dispatched: crate::DispatchedRehearsalDelivery::mint(
                            locator,
                            generation.operation,
                        ),
                    })
                }
                StoredRehearsalDeliveryPhase::Completed => Err(StoreError::InvalidRecord(
                    "completed retry generation lacks screen projection".into(),
                )),
                StoredRehearsalDeliveryPhase::Expired
                | StoredRehearsalDeliveryPhase::RunTimeExhaustedBeforeDispatch => unreachable!(),
                StoredRehearsalDeliveryPhase::AbandonedBeforeDispatch => {
                    Err(StoreError::Unavailable(
                        "rehearsal retry points to abandoned delivery generation".into(),
                    ))
                }
            };
        }
        let mut candidates = Vec::new();
        for (key, delivery) in state
            .rehearsal_delivery_operations
            .iter()
            .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
        {
            let generation = delivery.generations.last().ok_or_else(|| {
                StoreError::InvalidRecord("rehearsal delivery root has no generations".into())
            })?;
            if generation.phase()? == StoredRehearsalDeliveryPhase::Expired {
                candidates.push((
                    key.2.clone(),
                    generation.operation,
                    generation.descriptor.clone(),
                    generation.frozen_binding.clone(),
                ));
            }
        }
        let [(root_key, expired_operation, descriptor, frozen_binding)] = candidates.as_slice()
        else {
            return Err(if candidates.is_empty() {
                StoreError::Conflict
            } else {
                StoreError::InvalidRecord("rehearsal has more than one expired delivery".into())
            });
        };
        let witness = state
            .rehearsal_delivery_operations
            .get(&(tenant, run.id, root_key.clone()))
            .and_then(|root| root.generations.last())
            .and_then(|generation| generation.timing_witness)
            .ok_or_else(|| {
                StoreError::Unavailable("expired rehearsal delivery lacks timing witness".into())
            })?;
        if !domain::rehearsal_retry_is_available(witness, state.authoritative_time).map_err(
            |error| {
                StoreError::Unavailable(format!("rehearsal retry cannot be evaluated: {error}"))
            },
        )? {
            let deadline = witness.deadline().ok_or_else(|| {
                StoreError::Unavailable("run-time exhaustion lacks deadline".into())
            })?;
            // Persist the terminal result under the retry key, even though it
            // creates no successor generation.  A same-key request therefore
            // replays immutable prior evidence instead of re-evaluating a
            // clock-dependent workflow branch (ASVS 2.3.1, 2.3.3).
            state.rehearsal_delivery_retries.insert(
                retry_key,
                StoredRehearsalDeliveryRetry {
                    fingerprint: command.request_fingerprint,
                    root_key: root_key.clone(),
                    operation: *expired_operation,
                    terminal_predecessor: true,
                },
            );
            return Ok(crate::RetryRehearsalDeliveryResult::RunTimeExhausted { deadline });
        }
        let frozen_binding = frozen_binding.clone().ok_or_else(|| {
            StoreError::Unavailable("expired rehearsal delivery lacks frozen binding".into())
        })?;
        let operation = crate::RehearsalOperationId::from_uuid(fresh_uuid()?);
        let now = state.authoritative_time;
        let root = state
            .rehearsal_delivery_operations
            .get_mut(&(tenant, run.id, root_key.clone()))
            .ok_or(StoreError::NotFound)?;
        root.generations
            .push(StoredRehearsalDeliveryGeneration::new(
                operation,
                descriptor.clone(),
                now,
                Some(frozen_binding),
            ));
        state.rehearsal_delivery_retries.insert(
            retry_key,
            StoredRehearsalDeliveryRetry {
                fingerprint: command.request_fingerprint,
                root_key: root_key.clone(),
                operation,
                terminal_predecessor: false,
            },
        );
        Ok(crate::RetryRehearsalDeliveryResult::Prepared {
            prepared: crate::PreparedRehearsalDelivery::mint(
                locator,
                operation,
                descriptor.clone(),
            ),
        })
    }
    async fn mark_rehearsal_delivery_dispatched_from_route(
        &self,
        context: TenantContext,
        route: crate::RehearsalRouteIdentity,
        prepared: crate::PreparedRehearsalDelivery,
    ) -> Result<crate::RehearsalDeliveryDispatchResult, StoreError> {
        let locator = route_locator(&*self.read_state()?, context.tenant_id(), route)?;
        if prepared.locator() != locator {
            return Err(StoreError::NotFound);
        }
        self.mark_rehearsal_delivery_dispatched(context, prepared)
            .await
    }
    async fn complete_rehearsal_delivery_from_route(
        &self,
        context: TenantContext,
        command: crate::CompleteRehearsalDeliveryRouteCommand,
    ) -> Result<question_model::RehearsalActiveScreenV1, StoreError> {
        let locator = route_locator(&*self.read_state()?, context.tenant_id(), command.route)?;
        if command.dispatched.locator() != locator {
            return Err(StoreError::NotFound);
        }
        self.complete_rehearsal_delivery(
            context,
            crate::RehearsalDeliveryCompletionCommand {
                dispatched: command.dispatched,
                screen: command.screen,
            },
        )
        .await
    }
    async fn abandon_rehearsal_delivery_before_dispatch_from_route(
        &self,
        context: TenantContext,
        route: crate::RehearsalRouteIdentity,
        prepared: crate::PreparedRehearsalDelivery,
        reason: crate::RehearsalDeliveryPreDispatchAbandonReason,
    ) -> Result<(), StoreError> {
        let locator = route_locator(&*self.read_state()?, context.tenant_id(), route)?;
        if prepared.locator() != locator {
            return Err(StoreError::NotFound);
        }
        self.abandon_rehearsal_delivery_before_dispatch(context, prepared, reason)
            .await
    }
    async fn claim_rehearsal_submission_from_route(
        &self,
        context: TenantContext,
        command: crate::ClaimRehearsalSubmissionRouteCommand,
    ) -> Result<crate::RehearsalSubmissionClaimResult, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let locator = route_locator(&state, tenant, command.route)?;
        let (assignment, owner) = authorize_locator(&state, tenant, locator)?;
        let run = authorized_run(&state, tenant, locator, assignment, owner)?.clone();
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        // Idempotency is claim-root scoped. Resolve and authenticate an exact
        // prior route claim before searching for a still-unresolved delivery:
        // a completed item has no open candidate, but an exact browser retry
        // must still return its immutable receipt (ASVS V2.3.1, V2.3.3).
        if let Some(existing) = state
            .rehearsal_submission_claims
            .get(&(tenant, run.id, command.idempotency_key.clone()))
            .cloned()
        {
            let binding = existing.route_delivery.ok_or(StoreError::Conflict)?;
            let generation = state
                .rehearsal_delivery_operations
                .iter()
                .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
                .flat_map(|(_, delivery)| delivery.generations.iter())
                .find(|generation| generation.operation == binding.operation)
                .ok_or(StoreError::Conflict)?;
            let screen = generation.screen.as_ref().ok_or(StoreError::Conflict)?;
            let rendered =
                question_model::ValidatedRehearsalRenderedSubmissionV1::try_from_active_screen(
                    question_model::RehearsalSubmissionRequestV1 {
                        presentation_digest: command.presentation_digest,
                        response: command.response,
                    },
                    screen,
                )
                .map_err(|_| StoreError::Conflict)?;
            return claim_submission_input_locked(
                &mut state,
                tenant,
                locator,
                existing.attempt,
                RehearsalClaimSubmissionInput::rendered(rendered),
                command.idempotency_key,
                Some(RouteClaimDeliveryBindingInput {
                    attempt: existing.attempt,
                    operation: binding.operation,
                    screen_digest: binding.screen_digest,
                }),
            );
        }
        let mut accepted = std::collections::BTreeSet::new();
        for claim in state
            .rehearsal_submission_claims
            .iter()
            .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
            .map(|(_, claim)| claim)
        {
            let (_, snapshot) = hydrate_claim(&state, tenant, &run, claim)?;
            if snapshot.state() == domain::RehearsalSubmissionClaimState::Completed {
                accepted.insert(claim.attempt);
            }
        }
        let mut candidates = Vec::new();
        for generation in state
            .rehearsal_delivery_operations
            .iter()
            .filter(|(key, _)| key.0 == tenant && key.1 == run.id)
            .filter_map(|(_, delivery)| delivery.generations.last())
        {
            if generation.screen.is_some()
                && !accepted.contains(&generation.descriptor.attempt())
                && matches!(
                    generation.phase()?,
                    StoredRehearsalDeliveryPhase::Dispatched
                        | StoredRehearsalDeliveryPhase::Completed
                        | StoredRehearsalDeliveryPhase::Expired
                )
            {
                candidates.push(generation.operation);
            }
        }
        let [operation] = candidates.as_slice() else {
            return Err(if candidates.is_empty() {
                StoreError::Conflict
            } else {
                StoreError::InvalidRecord(
                    "rehearsal has more than one issued unresolved delivery".into(),
                )
            });
        };
        if reconcile_delivery_expiry_locked(&mut state, tenant, &run, *operation)? {
            return Err(StoreError::Conflict);
        }
        let attempt = state
            .rehearsal_delivery_operations
            .values()
            .flat_map(|delivery| delivery.generations.iter())
            .find(|generation| generation.operation == *operation)
            .ok_or(StoreError::NotFound)?;
        let screen = attempt
            .screen
            .as_ref()
            .ok_or_else(|| StoreError::InvalidRecord("issued delivery lacks active screen".into()))?
            .clone();
        let presentation_digest = screen.commitment().map_err(|error| {
            StoreError::InvalidRecord(format!("invalid active screen: {error:?}"))
        })?;
        if attempt.screen_digest != Some(presentation_digest)
            || presentation_digest.public_token() != command.presentation_digest
        {
            return Err(StoreError::Conflict);
        }
        let screen_digest = attempt.screen_digest.ok_or_else(|| {
            StoreError::InvalidRecord("issued delivery lacks screen digest".into())
        })?;
        let attempt = attempt.descriptor.attempt();
        let rendered =
            question_model::ValidatedRehearsalRenderedSubmissionV1::try_from_active_screen(
                question_model::RehearsalSubmissionRequestV1 {
                    presentation_digest: command.presentation_digest,
                    response: command.response,
                },
                &screen,
            )
            .map_err(|error| {
                StoreError::InvalidRecord(format!(
                    "invalid rendered rehearsal submission: {error:?}"
                ))
            })?;
        claim_submission_input_locked(
            &mut state,
            tenant,
            locator,
            attempt,
            RehearsalClaimSubmissionInput::rendered(rendered),
            command.idempotency_key,
            Some(RouteClaimDeliveryBindingInput {
                attempt,
                operation: *operation,
                screen_digest,
            }),
        )
    }
    async fn mark_rehearsal_submission_dispatched_from_route(
        &self,
        context: TenantContext,
        route: crate::RehearsalRouteIdentity,
        handle: PreparedClaimHandle,
    ) -> Result<DispatchedClaimHandle, StoreError> {
        let locator = route_locator(&*self.read_state()?, context.tenant_id(), route)?;
        self.mark_rehearsal_submission_dispatched(
            context,
            crate::MarkRehearsalSubmissionDispatchedCommand { locator, handle },
        )
        .await
    }
    async fn dispatch_rehearsal_submission_from_route(
        &self,
        context: TenantContext,
        route: crate::RehearsalRouteIdentity,
        idempotency_key: crate::RehearsalIdempotencyKey,
    ) -> Result<DispatchedClaimHandle, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let locator = route_locator(&state, tenant, route)?;
        let (assignment, owner) = authorize_locator(&state, tenant, locator)?;
        let run = authorized_run(&state, tenant, locator, assignment, owner)?.clone();
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        let key = (tenant, run.id, idempotency_key);
        let claim = state
            .rehearsal_submission_claims
            .get(&key)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let (root, snapshot) = hydrate_claim(&state, tenant, &run, &claim)?;
        match snapshot.state() {
            domain::RehearsalSubmissionClaimState::GradingDispatched => snapshot
                .into_dispatched_handle()
                .map_err(|_| StoreError::Conflict),
            domain::RehearsalSubmissionClaimState::Prepared => {
                let prepared = snapshot
                    .into_prepared_handle()
                    .map_err(|_| StoreError::Conflict)?;
                let operation = prepared.operation();
                let generation = prepared.generation();
                let dispatched = mark_rehearsal_submission_dispatched(prepared);
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
            domain::RehearsalSubmissionClaimState::Completed
            | domain::RehearsalSubmissionClaimState::AbandonedBeforeDispatch
            | domain::RehearsalSubmissionClaimState::RevokedStaleRevision
            | domain::RehearsalSubmissionClaimState::RevokedTerminalLifecycle
            | domain::RehearsalSubmissionClaimState::RevokedSourceContextRemoved => {
                Err(StoreError::Conflict)
            }
        }
    }
    async fn abandon_rehearsal_submission_before_dispatch_from_route(
        &self,
        context: TenantContext,
        route: crate::RehearsalRouteIdentity,
        handle: PreparedClaimHandle,
        reason: domain::RehearsalPreDispatchAbandonReason,
    ) -> Result<(), StoreError> {
        let locator = route_locator(&*self.read_state()?, context.tenant_id(), route)?;
        self.abandon_rehearsal_submission_before_dispatch(
            context,
            crate::AbandonRehearsalSubmissionBeforeDispatchCommand {
                locator,
                handle,
                reason,
            },
        )
        .await
    }
    async fn discard_rehearsal_from_route(
        &self,
        context: TenantContext,
        command: crate::DiscardRehearsalRouteCommand,
    ) -> Result<crate::RehearsalIdempotentProjectionResult, StoreError> {
        let locator = route_locator(&*self.read_state()?, context.tenant_id(), command.route)?;
        self.discard_rehearsal_idempotent(
            context,
            crate::RehearsalDiscardOperationCommand {
                locator,
                idempotency_key: command.idempotency_key,
                request_fingerprint: command.request_fingerprint,
                response: command.response,
                response_digest: command.response_digest,
            },
        )
        .await
    }
}

#[cfg(feature = "test-support")]
pub(in crate::in_memory) async fn transition_by_locator(
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
