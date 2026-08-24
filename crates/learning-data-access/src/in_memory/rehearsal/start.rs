use super::*;

pub(in crate::in_memory) fn start_locked(
    state: &mut State,
    tenant: TenantId,
    command: &crate::StartRehearsalRouteCommand,
) -> Result<RehearsalRunReceipt, StoreError> {
    let (assignment_id, owner) = authorize_assignment(
        state,
        tenant,
        command.actor,
        command.course,
        command.assignment,
        command.expected_revision,
    )?;
    let subject = resolve_subject_locked(state, tenant, command, assignment_id)?;
    let fingerprint = fingerprint_resolved_preview_subject(
        command.assignment,
        command.expected_revision,
        &subject,
    )
    .map_err(|error| StoreError::InvalidRecord(format!("invalid rehearsal subject: {error:?}")))?;
    let key = (tenant, command.course, assignment_id, owner);
    let existing = state
        .rehearsal_active_by_owner
        .get(&key)
        .and_then(|id| state.rehearsal_runs.get(&(tenant, *id)))
        .cloned();
    if let Some(existing) = &existing {
        verify_rehearsal_aggregate(state, tenant, existing)?;
    }
    match decide_start(
        existing.as_ref().map(|run| RehearsalLifecycleSnapshot {
            lifecycle: run.lifecycle,
            revision: run.revision,
            subject_fingerprint: run.fingerprint,
        }),
        command.expected_revision,
        fingerprint,
        command.start_new_after_completion,
    ) {
        domain::RehearsalStartDecision::Resume => return receipt(existing.as_ref().expect("run")),
        domain::RehearsalStartDecision::RequireExplicitRestart => return Err(StoreError::Conflict),
        domain::RehearsalStartDecision::DiscardByNewSubjectThenCreate => transition_locked(
            state,
            tenant,
            existing.as_ref().expect("run").id,
            RehearsalTerminalTransition::DiscardByNewSubject,
        )?,
        domain::RehearsalStartDecision::DiscardStaleRevision => return Err(StoreError::Conflict),
        domain::RehearsalStartDecision::Create => {}
    }
    let id = RehearsalRunId::from_uuid(fresh_uuid()?);
    let reference = next_reference(state)?;
    let now = state.authoritative_time;
    let run = StoredRehearsalRun {
        id,
        reference,
        course: command.course,
        assignment_id,
        assignment: command.assignment,
        owner,
        actor: command.actor,
        revision: command.expected_revision,
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
            revision: command.expected_revision,
            subject_fingerprint: fingerprint,
        }),
    };
    state.rehearsal_by_reference.insert((tenant, reference), id);
    state.rehearsal_active_by_owner.insert(key, id);
    state.rehearsal_runs.insert((tenant, id), run.clone());
    state
        .rehearsal_evidence
        .insert((tenant, id), StoredRehearsalEvidence::default());
    receipt(&run)
}

pub(in crate::in_memory) fn frozen_material_from_published(
    state: &State,
    tenant: TenantId,
    frozen: &question_model::RehearsalFrozenItemEvidence,
) -> Result<
    (
        crate::IssuedQuestionSnapshotV1,
        crate::PrefetchedPrivateExecutionV1,
    ),
    StoreError,
> {
    let published = state
        .published
        .get(&(frozen.problem.problem, frozen.problem.version))
        .ok_or_else(missing_rehearsal_material)?;
    if published.problem != frozen.problem.problem
        || published.version != frozen.problem.version
        || published.question.response != frozen.response_definition
    {
        return Err(StoreError::Unavailable(
            "published question does not match frozen rehearsal commitment".into(),
        ));
    }
    let witness = match &published.question.source {
        question_model::QuestionSource::Native { .. } => {
            crate::IssuedQuestionFamilyWitnessV1::Native {
                physical_asset_bindings: Vec::new(),
            }
        }
        question_model::QuestionSource::Webwork { .. } => {
            crate::IssuedQuestionFamilyWitnessV1::Webwork {}
        }
        // A QTI/external contract needs its specialized immutable private
        // authority.  Memory does not invent one from mutable source maps.
        question_model::QuestionSource::Qti { .. }
        | question_model::QuestionSource::Imathas { .. }
        | question_model::QuestionSource::H5p { .. } => {
            return Err(StoreError::Unavailable(
                "rehearsal source family has no immutable execution contract".into(),
            ));
        }
    };
    // A rehearsal start is tenant-scoped even if publication visibility later
    // changes.  At freeze time it must still belong to the active tenant.
    if state.problem_owner_tenants.get(&published.problem) != Some(&tenant) {
        return Err(StoreError::NotFound);
    }
    let snapshot = crate::IssuedQuestionSnapshotV1::new(published.question.clone(), witness)?;
    Ok((
        snapshot,
        crate::PrefetchedPrivateExecutionV1 {
            flat_grading: None,
            webwork_replay: None,
            webwork_grading: None,
            qti_grading: None,
        },
    ))
}

pub(in crate::in_memory) fn freeze_route_start_material(
    state: &mut State,
    tenant: TenantId,
    receipt: &RehearsalRunReceipt,
) -> Result<(), StoreError> {
    let run_id = state
        .rehearsal_by_reference
        .get(&(tenant, receipt.rehearsal))
        .copied()
        .ok_or(StoreError::NotFound)?;
    let run = state
        .rehearsal_runs
        .get(&(tenant, run_id))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let assignment = state
        .assignments
        .get(&(tenant, run.assignment_id))
        .cloned()
        .ok_or(StoreError::NotFound)?;
    let selected = assignment
        .items
        .iter()
        .filter(|item| item.delivery_state == question_model::AssignmentDeliveryState::Active)
        .cloned()
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(StoreError::Unavailable(
            "rehearsal assignment has no selected automated items".into(),
        ));
    }
    for item in selected {
        let published = state
            .published
            .get(&(item.reference.problem, item.reference.version))
            .ok_or_else(missing_rehearsal_material)?;
        let frozen = question_model::RehearsalFrozenItemEvidence {
            attempt: RehearsalAttemptId::from_uuid(fresh_uuid()?),
            problem: item.reference,
            response_definition: published.question.response.clone(),
            canonical_content_digest: canonical_rehearsal_question_content_digest(
                &published.question,
            )?,
            frozen_at: state.authoritative_time,
        };
        crate::ensure_rehearsal_delivery_supported(&frozen.response_definition)?;
        let (snapshot, private_execution) = frozen_material_from_published(state, tenant, &frozen)?;
        let snapshot_checksum = snapshot.canonical_payload_sha256()?;
        let private_checksum = private_execution_checksum(&private_execution)?;
        let key = (tenant, run.id, frozen.attempt);
        if state.rehearsal_frozen_items.contains_key(&key) {
            return Err(StoreError::Conflict);
        }
        let current_run = state
            .rehearsal_runs
            .get(&(tenant, run.id))
            .cloned()
            .ok_or(StoreError::NotFound)?;
        append_evidence(
            state,
            tenant,
            &current_run,
            RehearsalEvidencePayload::FrozenItem(frozen.clone()),
        )?;
        state.rehearsal_frozen_items.insert(key, frozen);
        let ordinal = state
            .rehearsal_frozen_source_snapshots
            .iter()
            .filter(|((stored_tenant, stored_run, _), _)| {
                *stored_tenant == tenant && *stored_run == run.id
            })
            .count();
        state.rehearsal_frozen_source_snapshots.insert(
            key,
            StoredRehearsalFrozenSourceSnapshot {
                ordinal,
                snapshot,
                checksum: snapshot_checksum,
            },
        );
        state.rehearsal_frozen_private_execution.insert(
            key,
            StoredRehearsalFrozenPrivateExecution {
                execution: private_execution,
                checksum: private_checksum,
            },
        );
    }
    verify_rehearsal_aggregate(
        state,
        tenant,
        state
            .rehearsal_runs
            .get(&(tenant, run.id))
            .ok_or(StoreError::NotFound)?,
    )?;
    Ok(())
}

/// One canonical full-question commitment shared by start freeze and every
/// later immutable-material integrity verifier.
pub(in crate::in_memory) fn canonical_rehearsal_question_content_digest(
    question: &question_model::QuestionDefinition,
) -> Result<question_model::RehearsalEvidenceDigest, StoreError> {
    let content = serde_json::to_vec(question).map_err(|_| {
        StoreError::InvalidRecord("cannot canonicalize rehearsal question content".into())
    })?;
    Ok(question_model::RehearsalEvidenceDigest::from_bytes(
        *objects::Sha256Digest::compute(&content).as_bytes(),
    ))
}

pub(in crate::in_memory) fn private_execution_checksum(
    execution: &crate::PrefetchedPrivateExecutionV1,
) -> Result<String, StoreError> {
    // The closed private contract intentionally lacks serde.  Its safe digest
    // is a capability-presence commitment: family-private values remain behind
    // the grader facade while a changed family shape cannot be mistaken for the
    // all-native empty contract supported by this Memory implementation.
    if execution.flat_grading.is_some()
        || execution.webwork_replay.is_some()
        || execution.webwork_grading.is_some()
        || execution.qti_grading.is_some()
    {
        return Err(StoreError::Unavailable(
            "memory rehearsal private material requires a sealed family codec".into(),
        ));
    }
    Ok(objects::Sha256Digest::compute(b"rehearsal-private-execution:v1:none").to_string())
}

pub(in crate::in_memory) fn deterministic_rehearsal_seed(
    run: &StoredRehearsalRun,
    frozen: &question_model::RehearsalFrozenItemEvidence,
) -> u64 {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(run.id.as_uuid().as_bytes());
    bytes.extend_from_slice(frozen.attempt.as_uuid().as_bytes());
    let digest = objects::Sha256Digest::compute(&bytes);
    u64::from_be_bytes(digest.as_bytes()[..8].try_into().expect("digest prefix"))
}

pub(in crate::in_memory) fn missing_rehearsal_material() -> StoreError {
    StoreError::Unavailable("immutable rehearsal delivery material is absent".into())
}
