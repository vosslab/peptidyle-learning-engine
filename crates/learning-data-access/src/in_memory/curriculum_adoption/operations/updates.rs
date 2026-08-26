use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticPayload,
};
use question_model::{
    AssignmentDefinitionSourceView, AssignmentFastForwardCommand, AssignmentFastForwardCompleted,
    AssignmentFastForwardDecision, AssignmentFastForwardPreviewRequest,
    AssignmentFastForwardPreviewView, CourseTermShiftCommand, CourseTermShiftCompleted,
    CourseTermShiftPreviewRequest, CourseTermShiftPreviewView,
    CreateSourceDerivedAssignmentCommand, CurriculumCourseImportView, CurriculumImportView,
    PreservedAssignmentRecoveryAction, SourceDerivedAssignmentCompleted,
    SourceDerivedAssignmentPreviewRequest, SourceDerivedAssignmentPreviewView,
};

use super::*;

pub(super) async fn preview_term_shift(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    request: CourseTermShiftPreviewRequest,
) -> Result<CourseTermShiftPreviewView, StoreError> {
    let tenant = context.tenant_id();
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let course = require_exact_witness(&state, tenant, &request.witness)?;
    require_course_instructor(&state, tenant, course, actor)?;
    let mut assignments = Vec::new();
    let mut corrections = Vec::new();
    for assignment in course_assignment_ids(&state, tenant, course) {
        let semantic = current_with_retained_schedule(&state, tenant, assignment)?;
        let (prepared, mut row_corrections) = preview_assignment(&semantic, &request.target_term)?;
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
    Ok(CourseTermShiftPreviewView {
        witness: request.witness,
        target_term: request.target_term,
        assignments,
        corrections,
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
        "shift-course-term",
        actor,
        (command.preview_witness(), command.target_term()),
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
                let semantic = current_with_retained_schedule(&state, tenant, *assignment)?;
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
    let baseline = state
        .curriculum_import_baselines
        .get(&(tenant, assignment))
        .ok_or_else(|| destination::integrity("import baseline"))?;
    if baseline.revision != request.import_revision {
        return Err(StoreError::Conflict);
    }
    let envelope = state
        .curriculum_import_envelopes
        .get(&(tenant, assignment))
        .ok_or_else(|| destination::integrity("import envelope"))?;
    let decision = fast_forward_decision(
        &state,
        tenant,
        actor,
        assignment,
        baseline,
        envelope,
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
        "fast-forward-assignment",
        actor,
        (
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
        let baseline = state
            .curriculum_import_baselines
            .get(&(tenant, assignment))
            .cloned()
            .ok_or_else(|| destination::integrity("import baseline"))?;
        if baseline.revision != command.import_revision() {
            return Err(StoreError::Conflict);
        }
        let envelope = state
            .curriculum_import_envelopes
            .get(&(tenant, assignment))
            .cloned()
            .ok_or_else(|| destination::integrity("import envelope"))?;
        if fast_forward_decision(
            &state,
            tenant,
            actor,
            assignment,
            &baseline,
            &envelope,
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
        let import_revision = next_import_revision(baseline.revision)?;
        let semantic_digest = CurriculumSemanticPayload::assignment(semantic.clone()).digest();
        let next_baseline = StoredCurriculumBaseline {
            payload: semantic,
            digest: semantic_digest,
            revision: import_revision,
        };
        state
            .curriculum_import_baselines
            .insert((tenant, assignment), next_baseline.clone());
        let occurred_at = state.authoritative_time;
        let next_envelope = StoredCurriculumEnvelope {
            source: StoredCurriculumSource::Assignment(command.source()),
            actor,
            occurred_at,
            receipt: command.idempotency_key().clone(),
        };
        state
            .curriculum_import_envelopes
            .insert((tenant, assignment), next_envelope.clone());
        state.curriculum_assignment_adoption_evidence.insert(
            (tenant, command.idempotency_key().clone(), assignment),
            StoredAssignmentAdoptionEvidence {
                baseline: next_baseline,
                envelope: next_envelope,
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
    let (assignment, corrections) = preview_assignment(only_assignment(&source.payload)?, term)?;
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
        "create-source-derived-assignment",
        actor,
        (
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
    let Some(import) = state.curriculum_course_imports.get(&(tenant, course)) else {
        return Ok(None);
    };
    if !state
        .curriculum_adoption_receipts
        .contains_key(&(tenant, import.receipt.clone()))
    {
        return Err(destination::integrity("course import receipt"));
    }
    let mut assignments = Vec::new();
    for assignment in &import.assignments {
        let baseline = state
            .curriculum_import_baselines
            .get(&(tenant, *assignment))
            .ok_or_else(|| destination::integrity("import baseline"))?;
        let envelope = state
            .curriculum_import_envelopes
            .get(&(tenant, *assignment))
            .ok_or_else(|| destination::integrity("import envelope"))?;
        validate_current_assignment_import_evidence(
            &state, tenant, *assignment, baseline, envelope,
        )?;
        let current = destination::current_semantic_assignment(
            &state,
            tenant,
            *assignment,
            baseline.payload.schedule().clone(),
        )?;
        let source = match envelope.source {
            StoredCurriculumSource::Assignment(source) => source,
            StoredCurriculumSource::WholeReusable(_)
            | StoredCurriculumSource::Rollover { .. }
            | StoredCurriculumSource::RolloverAssignment { .. } => {
                return Err(destination::integrity("reusable import source"));
            }
        };
        assignments.push(CurriculumImportView {
            course: course_reference,
            assignment: *state
                .assignment_references
                .get(&(tenant, *assignment))
                .ok_or_else(|| destination::integrity("assignment reference"))?,
            source,
            revision: baseline.revision,
            reusable_meaning_matches_baseline: current == baseline.payload,
        });
    }
    Ok(Some(CurriculumCourseImportView {
        course: course_reference,
        source: import.source,
        term: state
            .courses
            .get(&(tenant, course))
            .ok_or(StoreError::NotFound)?
            .term
            .clone(),
        schedule_revision: *state
            .course_schedule_revisions
            .get(&(tenant, course))
            .ok_or_else(|| destination::integrity("course schedule revision"))?,
        assignments,
    }))
}

fn fast_forward_decision(
    state: &State,
    tenant: TenantId,
    actor: UserId,
    assignment: AssignmentId,
    baseline: &StoredCurriculumBaseline,
    envelope: &StoredCurriculumEnvelope,
    requested_source: AssignmentDefinitionSourceView,
) -> Result<AssignmentFastForwardDecision, StoreError> {
    let StoredCurriculumSource::Assignment(imported_source) = envelope.source else {
        return Ok(AssignmentFastForwardDecision::SourceRevisionDrift {
            source: requested_source,
        });
    };
    if !same_source_locator(imported_source, requested_source) {
        return Ok(AssignmentFastForwardDecision::SourceRevisionDrift {
            source: requested_source,
        });
    }
    let current_source = super::super::super::reusable_curriculum::current_assignment_source(
        state,
        tenant,
        actor,
        requested_source,
    )?;
    if current_source != requested_source || !source_is_newer(imported_source, requested_source) {
        return Ok(AssignmentFastForwardDecision::SourceRevisionDrift {
            source: current_source,
        });
    }
    let current = destination::current_semantic_assignment(
        state,
        tenant,
        assignment,
        baseline.payload.schedule().clone(),
    )?;
    if current != baseline.payload {
        return Ok(AssignmentFastForwardDecision::Divergent {
            recovery: PreservedAssignmentRecoveryAction::CreateSourceDerivedAssignment,
        });
    }
    if assignment_has_run(state, tenant, assignment) {
        return Ok(AssignmentFastForwardDecision::IssuedWork {
            recovery: PreservedAssignmentRecoveryAction::CreateSourceDerivedAssignment,
        });
    }
    let source = super::super::super::reusable_curriculum::curriculum_assignment_source_snapshot(
        state,
        tenant,
        actor,
        requested_source,
    )?;
    let source_assignment = only_assignment(&source.payload)?;
    let assignment_payload = CurriculumSemanticPayload::assignment(source_assignment.clone());
    if let Some(recovery) = pin_correction(state, tenant, &assignment_payload)? {
        return Ok(AssignmentFastForwardDecision::UnavailablePin { recovery });
    }
    Ok(AssignmentFastForwardDecision::Eligible)
}

fn same_source_locator(
    left: AssignmentDefinitionSourceView,
    right: AssignmentDefinitionSourceView,
) -> bool {
    match (left, right) {
        (
            AssignmentDefinitionSourceView::Blueprint(left),
            AssignmentDefinitionSourceView::Blueprint(right),
        ) => left.reference == right.reference,
        (
            AssignmentDefinitionSourceView::Alpha(left),
            AssignmentDefinitionSourceView::Alpha(right),
        ) => {
            left.source().reference == right.source().reference
                && left.module_index() == right.module_index()
                && left.assignment_index() == right.assignment_index()
        }
        _ => false,
    }
}

fn source_is_newer(
    old: AssignmentDefinitionSourceView,
    new: AssignmentDefinitionSourceView,
) -> bool {
    match (old, new) {
        (
            AssignmentDefinitionSourceView::Blueprint(old),
            AssignmentDefinitionSourceView::Blueprint(new),
        ) => new.revision.value() > old.revision.value(),
        (
            AssignmentDefinitionSourceView::Alpha(old),
            AssignmentDefinitionSourceView::Alpha(new),
        ) => new.source().revision.value() > old.source().revision.value(),
        _ => false,
    }
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

fn course_assignment_ids(state: &State, tenant: TenantId, course: CourseId) -> Vec<AssignmentId> {
    let mut ids = state
        .assignments
        .iter()
        .filter_map(|((record_tenant, assignment), record)| {
            (*record_tenant == tenant && record.course_id == course).then_some(*assignment)
        })
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

pub(super) fn current_with_retained_schedule(
    state: &State,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<CurriculumSemanticAssignment, StoreError> {
    let schedule = state
        .curriculum_import_baselines
        .get(&(tenant, assignment))
        .map(|baseline| baseline.payload.schedule().clone())
        .map(Ok)
        .unwrap_or_else(|| {
            let record = state
                .assignments
                .get(&(tenant, assignment))
                .ok_or(StoreError::NotFound)?;
            let policy = state
                .assignment_base_policy
                .get(&(tenant, assignment))
                .ok_or_else(|| destination::integrity("assignment base policy"))?;
            let term = &state
                .courses
                .get(&(tenant, record.course_id))
                .ok_or(StoreError::NotFound)?
                .term;
            question_model::RelativeAssignmentSchedule::from_base_policy(&policy.policy, term)
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))
        })?;
    destination::current_semantic_assignment(state, tenant, assignment, schedule)
}
