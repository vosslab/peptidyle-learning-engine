//! Receipt-led repair of the B2 current import projection.

use question_model::{
    CurriculumAdoptionReconciliationResult, CurriculumAdoptionRepairedProjection,
    CurriculumAdoptionRepairedProjections, ReconcileCurriculumAdoptionCommand,
};

use super::dispatch::{
    MemoryCurriculumAdoptionOutcome, MemoryStore, SessionTokenHash, State, StoreError,
    StoredAssignmentImport, TenantContext, authorized_actor, completed_outcome_assignment_ids,
    destination, ensure_completed_outcome_binding, ensure_completed_outcome_contains_assignment,
    require_course_instructor, resolve_course, rollback,
};

pub(super) async fn reconcile(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    command: ReconcileCurriculumAdoptionCommand,
) -> Result<CurriculumAdoptionReconciliationResult, StoreError> {
    let tenant = context.tenant_id();
    let mut state = store.write_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let receipt = state
        .curriculum_adoption
        .receipts
        .get(&(tenant, command.receipt.idempotency_key.clone()))
        .cloned()
        .ok_or_else(|| destination::integrity("reconciliation receipt"))?;
    ensure_completed_outcome_binding(&state, tenant, &command.receipt.idempotency_key, &receipt)?;
    authorize_outcome(&state, tenant, actor, &receipt.completed)?;
    let before = state.clone();
    let result = (|| {
        let assignments = completed_outcome_assignment_ids(&state, tenant, &receipt.completed)?;
        let mut repaired = Vec::new();
        for assignment in assignments {
            let _evidence = state
                .curriculum_adoption
                .assignment_evidence
                .get(&(tenant, command.receipt.idempotency_key.clone(), assignment))
                .cloned()
                .ok_or_else(|| destination::integrity("reconciliation assignment evidence"))?;
            let newest = newest_evidence(&state, tenant, assignment)?;
            let expected = StoredAssignmentImport {
                baseline: newest.baseline,
                provenance: newest.provenance,
            };
            if state
                .curriculum_adoption
                .import_records
                .get(&(tenant, assignment))
                != Some(&expected)
            {
                state
                    .curriculum_adoption
                    .import_records
                    .insert((tenant, assignment), expected);
                let reference = *state
                    .assignment_references
                    .get(&(tenant, assignment))
                    .ok_or_else(|| destination::integrity("reconciliation assignment reference"))?;
                repaired.push(
                    CurriculumAdoptionRepairedProjection::AssignmentImportCurrent {
                        assignment: reference,
                    },
                );
            }
        }
        if repaired.is_empty() {
            Ok(CurriculumAdoptionReconciliationResult::AlreadyConsistent {
                receipt: command.receipt,
            })
        } else {
            let projections = CurriculumAdoptionRepairedProjections::new(repaired)
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
            Ok(CurriculumAdoptionReconciliationResult::Repaired {
                receipt: command.receipt,
                projections,
            })
        }
    })();
    rollback(&mut state, before, result)
}

fn authorize_outcome(
    state: &State,
    tenant: question_model::TenantId,
    actor: question_model::UserId,
    outcome: &MemoryCurriculumAdoptionOutcome,
) -> Result<(), StoreError> {
    let authorize_course = |reference| {
        require_course_instructor(
            state,
            tenant,
            resolve_course(state, tenant, reference)?,
            actor,
        )
    };
    match outcome {
        MemoryCurriculumAdoptionOutcome::ForkAlpha { alpha, .. } => {
            super::super::super::reusable_curriculum::require_alpha_creator(state, *alpha, actor)
        }
        MemoryCurriculumAdoptionOutcome::InstantiateBlueprint { course, .. }
        | MemoryCurriculumAdoptionOutcome::InstantiateAlpha { course, .. }
        | MemoryCurriculumAdoptionOutcome::ShiftCourseTerm { course, .. }
        | MemoryCurriculumAdoptionOutcome::FastForwardAssignment { course, .. }
        | MemoryCurriculumAdoptionOutcome::CreateSourceDerivedAssignment { course, .. } => {
            authorize_course(*course)
        }
        MemoryCurriculumAdoptionOutcome::RolloverCourse {
            source_course,
            course,
        } => {
            authorize_course(*source_course)?;
            authorize_course(*course)
        }
    }
}

fn newest_evidence(
    state: &State,
    tenant: question_model::TenantId,
    assignment: question_model::AssignmentId,
) -> Result<super::dispatch::StoredAssignmentAdoptionEvidence, StoreError> {
    let candidates = state
        .curriculum_adoption
        .assignment_evidence
        .iter()
        .filter(|((entry_tenant, _, entry_assignment), _)| {
            *entry_tenant == tenant && *entry_assignment == assignment
        })
        .collect::<Vec<_>>();
    let maximum = candidates
        .iter()
        .map(|(_, evidence)| evidence.baseline.revision.value())
        .max()
        .ok_or_else(|| destination::integrity("reconciliation latest immutable evidence"))?;
    let mut newest = candidates
        .into_iter()
        .filter(|(_, evidence)| evidence.baseline.revision.value() == maximum);
    let (key, evidence) = newest
        .next()
        .ok_or_else(|| destination::integrity("reconciliation latest immutable evidence"))?;
    if newest.next().is_some() {
        return Err(destination::integrity(
            "reconciliation duplicate latest immutable evidence",
        ));
    }
    let evidence = evidence.clone();
    let receipt = state
        .curriculum_adoption
        .receipts
        .get(&(tenant, evidence.provenance.receipt.clone()))
        .ok_or_else(|| destination::integrity("reconciliation latest evidence receipt"))?;
    if key.1 != evidence.provenance.receipt {
        return Err(destination::integrity(
            "reconciliation evidence receipt key",
        ));
    }
    ensure_completed_outcome_contains_assignment(
        state,
        tenant,
        &evidence.provenance.receipt,
        receipt,
        assignment,
    )?;
    if evidence.provenance.actor != receipt.actor
        || evidence.provenance.occurred_at != receipt.occurred_at
        || question_model::curriculum_adoption::CurriculumSemanticPayload::assignment(
            evidence.baseline.payload.clone(),
        )
        .digest()
            != evidence.baseline.digest
    {
        return Err(destination::integrity(
            "reconciliation latest evidence binding",
        ));
    }
    Ok(evidence)
}
