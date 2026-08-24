#[cfg(feature = "test-support")]
use super::start::frozen_material_from_published;
use super::*;

/// Deliberately impossible evidence is useful only to integrity conformance
/// tests. Production route starts freeze the complete ordinary assignment
/// inventory atomically; this fixture models historic corruption only.
#[cfg(feature = "test-support")]
impl MemoryStore {
    /// Returns ordinary-assignment material frozen by a canonical route start.
    /// Test support can inspect the immutable result but cannot create or
    /// replace it.
    pub fn frozen_rehearsal_item_for_test(
        &self,
        context: TenantContext,
        rehearsal: question_model::RehearsalReference,
    ) -> Result<question_model::RehearsalFrozenItemEvidence, StoreError> {
        let tenant = context.tenant_id();
        let state = self.read_state()?;
        let run = state
            .rehearsal_by_reference
            .get(&(tenant, rehearsal))
            .copied()
            .ok_or(StoreError::NotFound)?;
        state
            .rehearsal_frozen_items
            .iter()
            .find_map(|((entry_tenant, entry_run, _), item)| {
                (*entry_tenant == tenant && *entry_run == run).then(|| item.clone())
            })
            .ok_or_else(missing_rehearsal_material)
    }

    pub fn inject_rehearsal_frozen_item_for_test(
        &self,
        context: TenantContext,
        locator: crate::RehearsalLocator,
        frozen: question_model::RehearsalFrozenItemEvidence,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let (assignment, owner) = authorize_locator(&state, tenant, locator)?;
        let run = authorized_run(&state, tenant, locator, assignment, owner)?.clone();
        active_current(&state, tenant, &run)?;
        verify_rehearsal_aggregate(&state, tenant, &run)?;
        crate::ensure_rehearsal_delivery_supported(&frozen.response_definition)?;
        let key = (tenant, run.id, frozen.attempt);
        if state.rehearsal_frozen_items.contains_key(&key) {
            return Err(StoreError::Conflict);
        }
        let (snapshot, private_execution) =
            test_frozen_material_from_fixture(&state, tenant, &frozen)?;
        // Test-only historic-material injection still models the same durable
        // source-to-frozen commitment as production freezing.  In particular,
        // response-shape fixtures may synthesize a snapshot representation
        // from an older frozen row, so commit the canonical content actually
        // persisted beside that row rather than trusting the caller's digest.
        let mut frozen = frozen;
        frozen.canonical_content_digest =
            canonical_rehearsal_question_content_digest(snapshot.question())?;
        let snapshot_checksum = snapshot.canonical_payload_sha256()?;
        let private_checksum = private_execution_checksum(&private_execution)?;
        append_evidence(
            &mut state,
            tenant,
            &run,
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
        Ok(())
    }
}

#[cfg(feature = "test-support")]
fn test_frozen_material_from_fixture(
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
    if let Ok(material) = frozen_material_from_published(state, tenant, frozen) {
        return Ok(material);
    }
    let published = state
        .published
        .get(&(frozen.problem.problem, frozen.problem.version))
        .ok_or_else(missing_rehearsal_material)?;
    if state.problem_owner_tenants.get(&published.problem) != Some(&tenant) {
        return Err(StoreError::NotFound);
    }
    crate::ensure_rehearsal_delivery_supported(&frozen.response_definition)?;
    let mut question = published.question.clone();
    question.response = frozen.response_definition.clone();
    let snapshot = crate::IssuedQuestionSnapshotV1::new(
        question,
        crate::IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )?;
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

#[async_trait]
impl crate::RehearsalDeliveryMaterialStore for MemoryStore {
    async fn verify_rehearsal_delivery_material_from_route(
        &self,
        context: TenantContext,
        command: crate::VerifyRehearsalDeliveryMaterialRouteCommand,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let state = self.read_state()?;
        let (assignment, owner) = authorize_assignment(
            &state,
            tenant,
            command.route.actor,
            command.route.course,
            command.route.assignment,
            command.route.expected_revision,
        )?;
        let run = authorized_run(
            &state,
            tenant,
            crate::RehearsalLocator {
                actor: command.route.actor,
                course: command.route.course,
                assignment: command.route.assignment,
                revision: command.route.expected_revision,
                rehearsal: command.route.rehearsal,
            },
            assignment,
            owner,
        )?;
        active_current(&state, tenant, run)?;
        verify_rehearsal_aggregate(&state, tenant, run)?;
        Ok(())
    }
}
