use question_model::curriculum_adoption::CurriculumSemanticPayload;
use question_model::{
    AssignmentDefinitionSourceView, AssignmentFastForwardCommand, AssignmentFastForwardCompleted,
    AssignmentFastForwardDecision, AssignmentFastForwardPreviewRequest,
    AssignmentFastForwardPreviewView, AssignmentId, CourseId, CourseReference,
    CourseScheduleRevision, CourseScheduleWitness, CourseTermShiftCommand,
    CourseTermShiftCompleted, CourseTermShiftIneligibility, CourseTermShiftPreviewOutcome,
    CourseTermShiftPreviewRequest, CourseTermShiftPreviewView, CourseTermShiftRecoveryAction,
    CreateSourceDerivedAssignmentCommand, CurriculumAssignmentImportSourceView,
    CurriculumCourseImportOriginView, CurriculumCourseImportView, RolloverAssignmentSourceView,
    RolloverCourseImportOriginView, SourceDerivedAssignmentCompleted,
    SourceDerivedAssignmentPreviewRequest, SourceDerivedAssignmentPreviewView, TenantId,
};

use super::dispatch::{
    CurriculumAdoptionOperation, MemoryCurriculumAdoptionOutcome, MemoryStore, SessionTokenHash,
    State, StoreError, StoredAssignmentAdoptionEvidence, StoredAssignmentImport,
    StoredAssignmentImportProvenance, StoredAssignmentImportSource, StoredCurriculumBaseline,
    StoredWholeCourseOrigin, TenantContext, UserId, advance_course_schedule_revision,
    assignment_has_run, assignment_source_snapshot_with_replacements, authorized_actor,
    course_assignment_ids, course_has_any_run, course_witness,
    current_with_projected_teaching_schedule, destination, fast_forward_completed,
    matching_receipt, next_import_revision, only_assignment, pin_correction,
    refuse_detached_whole_course_receipt, replacement_question_choices, request_digest,
    require_course_instructor, require_exact_witness, resolve_course, rollback,
    semantic_preview_error, source_derived_completed, store_import, store_receipt,
    term_shift_completed, unavailable_destination_pin, validate_current_assignment_import_evidence,
    validate_destination_pins, validate_whole_course_adoption,
};
use crate::curriculum_adoption::{
    CurrentTeachingImportInput, CurriculumImportInspectionInput, FastForwardProjectionInput,
    ObservedSemanticEnvelope, preview_assignment, project_current_teaching_import,
    project_curriculum_import_inspection, project_fast_forward_decision, same_source_locator,
};

pub(super) async fn preview_term_shift(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    request: CourseTermShiftPreviewRequest,
) -> Result<CourseTermShiftPreviewOutcome, StoreError> {
    let tenant = context.tenant_id();
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let course = require_exact_witness(&state, tenant, &request.witness)?;
    require_course_instructor(&state, tenant, course, actor)?;
    if course_has_any_run(&state, tenant, course) {
        return Ok(CourseTermShiftPreviewOutcome::Ineligible {
            course: request.witness.course,
            reason: CourseTermShiftIneligibility::IssuedWork,
            recovery: CourseTermShiftRecoveryAction::RolloverCourse,
        });
    }
    let mut assignments = Vec::new();
    let mut corrections = Vec::new();
    for assignment in course_assignment_ids(&state, tenant, course) {
        let semantic = current_with_projected_teaching_schedule(&state, tenant, assignment)?;
        let (prepared, mut row_corrections) =
            preview_assignment(&semantic, &request.target_term).map_err(semantic_preview_error)?;
        let reference = *state
            .assignment_references
            .get(&(tenant, assignment))
            .ok_or_else(|| destination::integrity("assignment reference"))?;
        let revision = *state
            .assignment_revisions
            .get(&(tenant, assignment))
            .ok_or_else(|| destination::integrity("assignment revision"))?;
        assignments.push(question_model::CurriculumAssignmentView {
            reference,
            title: prepared.title,
            revision,
            schedule: prepared.schedule,
        });
        corrections.append(&mut row_corrections);
    }
    Ok(CourseTermShiftPreviewOutcome::Eligible {
        preview: CourseTermShiftPreviewView {
            witness: request.witness,
            target_term: request.target_term,
            assignments,
            corrections,
        },
    })
}

pub(super) async fn apply_term_shift(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    command: CourseTermShiftCommand,
) -> Result<CourseTermShiftCompleted, StoreError> {
    let tenant = context.tenant_id();
    let mut state = store.write_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let digest = request_digest(
        CurriculumAdoptionOperation::ShiftCourseTerm,
        actor,
        &(command.preview_witness(), command.target_term()),
    )?;
    if let Some(outcome) = matching_receipt(
        &state,
        tenant,
        command.idempotency_key(),
        CurriculumAdoptionOperation::ShiftCourseTerm,
        actor,
        digest,
    )? {
        return term_shift_completed(outcome, command.idempotency_key(), true);
    }
    let before = state.clone();
    let result = (|| {
        let course = require_exact_witness(&state, tenant, command.preview_witness())?;
        require_course_instructor(&state, tenant, course, actor)?;
        if course_has_any_run(&state, tenant, course) {
            return Err(StoreError::Conflict);
        }
        let ids = course_assignment_ids(&state, tenant, course);
        let resolved = ids
            .iter()
            .map(|assignment| {
                let semantic =
                    current_with_projected_teaching_schedule(&state, tenant, *assignment)?;
                let schedule = semantic
                    .schedule()
                    .resolve_for_target_term(command.target_term())
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
                Ok((*assignment, schedule))
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        state
            .courses
            .get_mut(&(tenant, course))
            .ok_or(StoreError::NotFound)?
            .term = command.target_term().clone();
        for (assignment, schedule) in resolved {
            let key = (tenant, assignment);
            let revision = *state
                .assignment_revisions
                .get(&key)
                .ok_or_else(|| destination::integrity("assignment revision"))?;
            let next = crate::assignment_revision_checked_next(revision)?;
            let mut stored = state
                .assignment_base_policy
                .get(&key)
                .copied()
                .ok_or_else(|| destination::integrity("assignment base policy"))?;
            stored.policy.available_at = schedule.available_at.map(|value| value.timestamp);
            stored.policy.due_at = schedule.due_at.map(|value| value.timestamp);
            stored.policy.closes_at = schedule.closes_at.map(|value| value.timestamp);
            stored.revision = next;
            state.assignment_base_policy.insert(key, stored);
            state.assignment_revisions.insert(key, next);
        }
        advance_course_schedule_revision(&mut state, tenant, course)?;
        let outcome = MemoryCurriculumAdoptionOutcome::ShiftCourseTerm {
            course: command.preview_witness().course,
            term: command.target_term().clone(),
        };
        store_receipt(
            &mut state,
            tenant,
            command.idempotency_key().clone(),
            CurriculumAdoptionOperation::ShiftCourseTerm,
            actor,
            digest,
            outcome.clone(),
        );
        term_shift_completed(outcome, command.idempotency_key(), false)
    })();
    rollback(&mut state, before, result)
}

pub(super) async fn preview_fast_forward(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    request: AssignmentFastForwardPreviewRequest,
) -> Result<AssignmentFastForwardPreviewView, StoreError> {
    let tenant = context.tenant_id();
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let course = resolve_course(&state, tenant, request.course)?;
    require_course_instructor(&state, tenant, course, actor)?;
    let assignment = resolve_assignment(&state, tenant, course, request.assignment.assignment)?;
    if state
        .assignment_revisions
        .get(&(tenant, assignment))
        .copied()
        != Some(request.assignment.revision)
    {
        return Err(StoreError::Conflict);
    }
    let import = state
        .curriculum_adoption
        .import_records
        .get(&(tenant, assignment))
        .ok_or_else(|| destination::integrity("current import record"))?;
    if import.baseline.revision != request.import_revision {
        return Err(StoreError::Conflict);
    }
    let decision = fast_forward_decision(
        &state,
        tenant,
        actor,
        assignment,
        &import.baseline,
        &import.provenance,
        request.source,
    )?;
    Ok(AssignmentFastForwardPreviewView {
        course: request.course,
        assignment: request.assignment,
        import_revision: request.import_revision,
        source: request.source,
        witness: course_witness(&state, tenant, course)?,
        decision,
    })
}

pub(super) async fn apply_fast_forward(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    command: AssignmentFastForwardCommand,
) -> Result<AssignmentFastForwardCompleted, StoreError> {
    let tenant = context.tenant_id();
    let mut state = store.write_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let digest = request_digest(
        CurriculumAdoptionOperation::FastForwardAssignment,
        actor,
        &(
            command.course(),
            command.assignment(),
            command.import_revision(),
            command.source(),
            command.preview_witness(),
        ),
    )?;
    if let Some(outcome) = matching_receipt(
        &state,
        tenant,
        command.idempotency_key(),
        CurriculumAdoptionOperation::FastForwardAssignment,
        actor,
        digest,
    )? {
        return fast_forward_completed(outcome, command.idempotency_key(), true);
    }
    let before = state.clone();
    let result = (|| {
        let course = require_exact_witness(&state, tenant, command.preview_witness())?;
        require_course_instructor(&state, tenant, course, actor)?;
        if command.course() != command.preview_witness().course {
            return Err(StoreError::Conflict);
        }
        let assignment =
            resolve_assignment(&state, tenant, course, command.assignment().assignment)?;
        if state
            .assignment_revisions
            .get(&(tenant, assignment))
            .copied()
            != Some(command.assignment().revision)
        {
            return Err(StoreError::Conflict);
        }
        let import = state
            .curriculum_adoption
            .import_records
            .get(&(tenant, assignment))
            .cloned()
            .ok_or_else(|| destination::integrity("current import record"))?;
        if import.baseline.revision != command.import_revision() {
            return Err(StoreError::Conflict);
        }
        if fast_forward_decision(
            &state,
            tenant,
            actor,
            assignment,
            &import.baseline,
            &import.provenance,
            command.source(),
        )? != AssignmentFastForwardDecision::Eligible
        {
            return Err(StoreError::Conflict);
        }
        let source =
            super::super::super::reusable_curriculum::curriculum_assignment_source_snapshot(
                &state,
                tenant,
                actor,
                command.source(),
            )?;
        let semantic = only_assignment(&source.payload)?.clone();
        destination::replace_reusable_meaning(&mut state, tenant, assignment, &semantic)?;
        advance_course_schedule_revision(&mut state, tenant, course)?;
        let import_revision = next_import_revision(import.baseline.revision)?;
        let semantic_digest = CurriculumSemanticPayload::assignment(semantic.clone()).digest();
        let next_baseline = StoredCurriculumBaseline {
            payload: semantic,
            digest: semantic_digest,
            revision: import_revision,
        };
        let occurred_at = state.authoritative_time;
        let next_provenance = StoredAssignmentImportProvenance {
            source: StoredAssignmentImportSource::Reusable(command.source()),
            actor,
            occurred_at,
            receipt: command.idempotency_key().clone(),
        };
        state.curriculum_adoption.import_records.insert(
            (tenant, assignment),
            StoredAssignmentImport {
                baseline: next_baseline.clone(),
                provenance: next_provenance.clone(),
            },
        );
        state.curriculum_adoption.assignment_evidence.insert(
            (tenant, command.idempotency_key().clone(), assignment),
            StoredAssignmentAdoptionEvidence {
                baseline: next_baseline,
                provenance: next_provenance,
            },
        );
        let outcome = MemoryCurriculumAdoptionOutcome::FastForwardAssignment {
            course: command.course(),
            assignment: command.assignment().assignment,
            import_revision,
        };
        store_receipt(
            &mut state,
            tenant,
            command.idempotency_key().clone(),
            CurriculumAdoptionOperation::FastForwardAssignment,
            actor,
            digest,
            outcome.clone(),
        );
        fast_forward_completed(outcome, command.idempotency_key(), false)
    })();
    rollback(&mut state, before, result)
}

pub(super) async fn preview_source_derived(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    request: SourceDerivedAssignmentPreviewRequest,
) -> Result<SourceDerivedAssignmentPreviewView, StoreError> {
    let tenant = context.tenant_id();
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let course = resolve_course(&state, tenant, request.course)?;
    require_course_instructor(&state, tenant, course, actor)?;
    let source = assignment_source_snapshot_with_replacements(
        &state,
        store,
        tenant,
        actor,
        request.source,
        &request.replacements,
    )?;
    let term = &state
        .courses
        .get(&(tenant, course))
        .ok_or(StoreError::NotFound)?
        .term;
    let (assignment, corrections) = preview_assignment(only_assignment(&source.payload)?, term)
        .map_err(semantic_preview_error)?;
    Ok(SourceDerivedAssignmentPreviewView {
        course: request.course,
        source: request.source,
        witness: course_witness(&state, tenant, course)?,
        assignment,
        corrections,
        replacements: request.replacements,
        pin_correction: pin_correction(&state, tenant, &source.payload)?,
    })
}

pub(super) async fn apply_source_derived(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    command: CreateSourceDerivedAssignmentCommand,
) -> Result<SourceDerivedAssignmentCompleted, StoreError> {
    let tenant = context.tenant_id();
    let mut state = store.write_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let digest = request_digest(
        CurriculumAdoptionOperation::CreateSourceDerivedAssignment,
        actor,
        &(
            command.course(),
            command.source(),
            command.preview_witness(),
            command.replacements(),
        ),
    )?;
    if let Some(outcome) = matching_receipt(
        &state,
        tenant,
        command.idempotency_key(),
        CurriculumAdoptionOperation::CreateSourceDerivedAssignment,
        actor,
        digest,
    )? {
        return source_derived_completed(outcome, command.idempotency_key(), true);
    }
    let before = state.clone();
    let result = (|| {
        let course = require_exact_witness(&state, tenant, command.preview_witness())?;
        require_course_instructor(&state, tenant, course, actor)?;
        if command.course() != command.preview_witness().course {
            return Err(StoreError::Conflict);
        }
        let source = assignment_source_snapshot_with_replacements(
            &state,
            store,
            tenant,
            actor,
            command.source(),
            command.replacements(),
        )?;
        validate_destination_pins(&state, tenant, &source.payload)
            .map_err(|_| StoreError::Conflict)?;
        let semantic = only_assignment(&source.payload)?.clone();
        let term = &state
            .courses
            .get(&(tenant, course))
            .ok_or(StoreError::NotFound)?
            .term;
        semantic
            .schedule()
            .resolve_for_target_term(term)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let (assignment, reference) =
            destination::materialize_semantic_assignment(&mut state, context, course, &semantic)?;
        store_import(
            &mut state,
            tenant,
            assignment,
            semantic,
            command.source(),
            actor,
            command.idempotency_key(),
        );
        let outcome = MemoryCurriculumAdoptionOutcome::CreateSourceDerivedAssignment {
            course: command.course(),
            assignment: reference,
        };
        store_receipt(
            &mut state,
            tenant,
            command.idempotency_key().clone(),
            CurriculumAdoptionOperation::CreateSourceDerivedAssignment,
            actor,
            digest,
            outcome.clone(),
        );
        source_derived_completed(outcome, command.idempotency_key(), false)
    })();
    rollback(&mut state, before, result)
}

pub(super) async fn inspect_imports(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    course_reference: CourseReference,
) -> Result<Option<CurriculumCourseImportView>, StoreError> {
    let tenant = context.tenant_id();
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let course = resolve_course(&state, tenant, course_reference)?;
    require_course_instructor(&state, tenant, course, actor)?;
    let witness = course_witness(&state, tenant, course)?;
    let adoption = state
        .curriculum_adoption
        .whole_course_adoptions
        .get(&(tenant, course));
    if let Some(adoption) = adoption {
        validate_whole_course_adoption(&state, tenant, course)?;
        if adoption.destination_assignments.iter().any(|assignment| {
            !state
                .curriculum_adoption
                .import_records
                .contains_key(&(tenant, *assignment))
        }) {
            return Err(destination::integrity(
                "whole-course current import projection",
            ));
        }
    } else {
        refuse_detached_whole_course_receipt(&state, tenant, course_reference)?;
    }
    let assignment_ids = course_assignment_ids(&state, tenant, course)
        .into_iter()
        .filter(|assignment| {
            state
                .curriculum_adoption
                .import_records
                .contains_key(&(tenant, *assignment))
        })
        .collect::<Vec<_>>();
    if assignment_ids.is_empty() {
        return Ok(None);
    }
    let origin = inspection_origin(&state, tenant, course, adoption)?;
    let rollover_source = match &origin {
        CurriculumCourseImportOriginView::Rollover { source } => Some(&source.source_schedule),
        CurriculumCourseImportOriginView::Ordinary
        | CurriculumCourseImportOriginView::Alpha { .. } => None,
    };
    let mut assignments = Vec::new();
    for assignment in &assignment_ids {
        let import = state
            .curriculum_adoption
            .import_records
            .get(&(tenant, *assignment))
            .ok_or_else(|| destination::integrity("current import record"))?;
        validate_current_assignment_import_evidence(&state, tenant, *assignment, import)?;
        let current = destination::current_semantic_assignment(
            &state,
            tenant,
            *assignment,
            import.baseline.payload.schedule().clone(),
        )?;
        let source = match &import.provenance.source {
            StoredAssignmentImportSource::Reusable(source) => {
                CurriculumAssignmentImportSourceView::Reusable {
                    definition: *source,
                }
            }
            StoredAssignmentImportSource::Rollover(provenance) => {
                CurriculumAssignmentImportSourceView::Rollover {
                    source: RolloverAssignmentSourceView::new(
                        rollover_source
                            .ok_or_else(|| destination::integrity("rollover course origin"))?,
                        provenance.source_assignment,
                    )
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
                }
            }
        };
        let baseline_payload =
            CurriculumSemanticPayload::assignment(import.baseline.payload.clone());
        let baseline_envelope = baseline_payload.canonical_envelope();
        let current_payload = CurriculumSemanticPayload::assignment(current);
        assignments.push(
            project_current_teaching_import(CurrentTeachingImportInput {
                assignment: *state
                    .assignment_references
                    .get(&(tenant, *assignment))
                    .ok_or_else(|| destination::integrity("assignment reference"))?,
                source,
                revision: import.baseline.revision,
                baseline: &baseline_payload,
                baseline_evidence: ObservedSemanticEnvelope {
                    canonical_version: baseline_envelope.version(),
                    canonical_bytes: baseline_envelope.canonical_bytes(),
                    digest: import.baseline.digest.as_bytes(),
                },
                current: &current_payload,
            })
            .map_err(semantic_preview_error)?,
        );
    }
    project_curriculum_import_inspection(CurriculumImportInspectionInput {
        witness,
        origin,
        term: state
            .courses
            .get(&(tenant, course))
            .ok_or(StoreError::NotFound)?
            .term
            .clone(),
        assignments,
    })
    .map(Some)
    .map_err(semantic_preview_error)
}

fn inspection_origin(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    adoption: Option<&super::dispatch::StoredWholeCourseAdoption>,
) -> Result<CurriculumCourseImportOriginView, StoreError> {
    let Some(_) = adoption else {
        return Ok(CurriculumCourseImportOriginView::Ordinary);
    };
    let adoption = validate_whole_course_adoption(state, tenant, course)?;
    match adoption.origin {
        StoredWholeCourseOrigin::Alpha { source } => {
            Ok(CurriculumCourseImportOriginView::Alpha { source })
        }
        StoredWholeCourseOrigin::Rollover {
            source_course,
            source_schedule_revision,
        } => Ok(CurriculumCourseImportOriginView::Rollover {
            source: RolloverCourseImportOriginView {
                source_schedule: rollover_source_witness(
                    state,
                    tenant,
                    source_course,
                    source_schedule_revision,
                    adoption,
                )?,
            },
        }),
    }
}

fn rollover_source_witness(
    state: &State,
    tenant: TenantId,
    source_course: CourseReference,
    source_schedule_revision: CourseScheduleRevision,
    adoption: &super::dispatch::StoredWholeCourseAdoption,
) -> Result<CourseScheduleWitness, StoreError> {
    let assignments = adoption
        .destination_assignments
        .iter()
        .map(|assignment| {
            let evidence = state
                .curriculum_adoption
                .assignment_evidence
                .get(&(tenant, adoption.receipt.clone(), *assignment))
                .ok_or_else(|| destination::integrity("rollover assignment evidence"))?;
            let StoredAssignmentImportSource::Rollover(provenance) = &evidence.provenance.source
            else {
                return Err(destination::integrity("rollover assignment source"));
            };
            if provenance.source_course != source_course
                || provenance.source_schedule_revision != source_schedule_revision
            {
                return Err(destination::integrity(
                    "rollover assignment witness binding",
                ));
            }
            Ok(provenance.source_assignment)
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    CourseScheduleWitness::new(source_course, source_schedule_revision, assignments)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))
}

fn fast_forward_decision(
    state: &State,
    tenant: TenantId,
    actor: UserId,
    assignment: AssignmentId,
    baseline: &StoredCurriculumBaseline,
    provenance: &StoredAssignmentImportProvenance,
    requested_source: AssignmentDefinitionSourceView,
) -> Result<AssignmentFastForwardDecision, StoreError> {
    let imported_source = match &provenance.source {
        StoredAssignmentImportSource::Reusable(source) => Some(*source),
        StoredAssignmentImportSource::Rollover(_) => None,
    };
    let current_source =
        if imported_source.is_some_and(|source| same_source_locator(source, requested_source)) {
            super::super::super::reusable_curriculum::current_assignment_source(
                state,
                tenant,
                actor,
                requested_source,
            )?
        } else {
            requested_source
        };
    let current_assignment = destination::current_semantic_assignment(
        state,
        tenant,
        assignment,
        baseline.payload.schedule().clone(),
    )?;
    let baseline_payload = CurriculumSemanticPayload::assignment(baseline.payload.clone());
    let current_payload = CurriculumSemanticPayload::assignment(current_assignment);
    let initial = project_fast_forward_decision(FastForwardProjectionInput {
        imported_source,
        requested_source,
        current_source,
        baseline: &baseline_payload,
        current: &current_payload,
        issued_work: assignment_has_run(state, tenant, assignment),
        unavailable_pin: None,
        replacement_choices: None,
    })
    .map_err(semantic_preview_error)?;
    if initial != AssignmentFastForwardDecision::Eligible {
        return Ok(initial);
    }
    let source = super::super::super::reusable_curriculum::curriculum_assignment_source_snapshot(
        state,
        tenant,
        actor,
        requested_source,
    )?;
    let source_assignment = only_assignment(&source.payload)?;
    let assignment_payload = CurriculumSemanticPayload::assignment(source_assignment.clone());
    let unavailable_pin = unavailable_destination_pin(state, tenant, &assignment_payload)?;
    let replacement_choices = unavailable_pin
        .map(|_| replacement_question_choices(state, tenant))
        .transpose()?;
    project_fast_forward_decision(FastForwardProjectionInput {
        imported_source,
        requested_source,
        current_source,
        baseline: &baseline_payload,
        current: &current_payload,
        issued_work: false,
        unavailable_pin,
        replacement_choices,
    })
    .map_err(semantic_preview_error)
}

fn resolve_assignment(
    state: &State,
    tenant: TenantId,
    course: CourseId,
    reference: question_model::AssignmentReference,
) -> Result<AssignmentId, StoreError> {
    let assignment = *state
        .assignments_by_reference
        .get(&(tenant, reference))
        .ok_or(StoreError::NotFound)?;
    state
        .assignments
        .get(&(tenant, assignment))
        .filter(|record| record.course_id == course)
        .map(|_| assignment)
        .ok_or(StoreError::NotFound)
}
