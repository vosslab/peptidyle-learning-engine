use question_model::curriculum_adoption::{
    CurriculumSemanticCourse, CurriculumSemanticModule, CurriculumSemanticPayload,
};
use question_model::{
    AlphaInstantiationCommand, AlphaInstantiationCompleted, AssignmentDefinitionSourceView,
    CourseRolloverCommand, CourseRolloverCompleted, CourseRolloverPreviewRequest,
    CourseRolloverPreviewView, CurriculumSourceView, ObservedAlphaAssignmentSource,
};

use super::{
    CurriculumAdoptionOperation, MemoryCurriculumAdoptionOutcome, MemoryStore, SessionTokenHash,
    State, StoreError, StoredCourseAdoptionRecord, StoredCourseImportEnvelope,
    StoredCurriculumEnvelope, StoredCurriculumSource, TenantContext, TenantId,
    adopted_course_semantic, apply_pin_replacements, authorized_actor, current_with_retained_schedule,
    destination, matching_receipt, pin_correction, preview_course, require_course_instructor,
    require_exact_witness, request_digest, rollback, source_snapshot_with_replacements,
    store_import, store_receipt, store_rollover_import, validate_destination_pins,
};
use crate::CourseRecord;
use crate::in_memory::MemoryStore;

pub(super) async fn apply_new_alpha_course(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    command: AlphaInstantiationCommand,
) -> Result<AlphaInstantiationCompleted, StoreError> {
    let tenant = context.tenant_id();
    let mut state = store.write_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let digest = request_digest(
        "instantiate-alpha",
        actor,
        (
            command.source(),
            command.title(),
            command.target_term(),
            command.replacements(),
        ),
    )?;
    if let Some(outcome) = matching_receipt(
        &state,
        tenant,
        command.idempotency_key(),
        CurriculumAdoptionOperation::InstantiateAlpha,
        actor,
        digest,
    )? {
        return alpha_completed(outcome, command.idempotency_key(), true);
    }
    let before = state.clone();
    let result = (|| {
        let source = source_snapshot_with_replacements(
            &state,
            store,
            tenant,
            actor,
            CurriculumSourceView::Alpha(command.source()),
            command.replacements(),
        )?;
        validate_destination_pins(&state, tenant, &source.payload)
            .map_err(|_| StoreError::Conflict)?;
        let CurriculumSemanticPayload::Course(course_semantic) = source.payload else {
            return Err(StoreError::InvalidRecord(
                "Alpha source is not course-sized".into(),
            ));
        };
        for assignment in course_semantic
            .modules()
            .iter()
            .flat_map(|module| module.assignments())
        {
            assignment
                .schedule()
                .resolve_for_target_term(command.target_term())
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        }
        let course = random_course_id()?;
        let reference = super::super::super::courses::provision_course_locked(
            &mut state,
            CourseRecord {
                id: course,
                tenant,
                title: command.title().as_str().to_owned(),
                term: command.target_term().clone(),
            },
            actor,
        )?;
        let mut assignment_ids = Vec::new();
        for (module_index, module) in course_semantic.modules().iter().enumerate() {
            for (assignment_index, semantic) in module.assignments().iter().enumerate() {
                let (assignment, _) = destination::materialize_semantic_assignment(
                    &mut state, context, course, semantic,
                )?;
                store_import(
                    &mut state,
                    tenant,
                    assignment,
                    semantic.clone(),
                    AssignmentDefinitionSourceView::Alpha(
                        ObservedAlphaAssignmentSource::new(
                            command.source(),
                            u16::try_from(module_index)
                                .expect("bounded Alpha module index fits u16"),
                            u16::try_from(assignment_index)
                                .expect("bounded Alpha assignment index fits u16"),
                        )
                        .expect("validated Alpha positions remain bounded"),
                    ),
                    actor,
                    command.idempotency_key(),
                );
                assignment_ids.push(assignment);
            }
        }
        let occurred_at = state.authoritative_time;
        let adopted_payload = adopted_course_semantic(command.title(), &course_semantic)?;
        let adopted_digest = CurriculumSemanticPayload::course(adopted_payload.clone()).digest();
        state.curriculum_course_imports.insert(
            (tenant, course),
            StoredCourseImportEnvelope {
                source: command.source(),
                assignments: assignment_ids.clone(),
                actor,
                occurred_at,
                receipt: command.idempotency_key().clone(),
            },
        );
        state.curriculum_course_adoptions.insert(
            (tenant, course),
            StoredCourseAdoptionRecord {
                assignments: assignment_ids,
                payload: adopted_payload,
                digest: adopted_digest,
                receipt: command.idempotency_key().clone(),
            },
        );
        state.curriculum_course_envelopes.insert(
            (tenant, course),
            StoredCurriculumEnvelope {
                source: StoredCurriculumSource::WholeReusable(CurriculumSourceView::Alpha(
                    command.source(),
                )),
                actor,
                occurred_at,
                receipt: command.idempotency_key().clone(),
            },
        );
        let outcome = MemoryCurriculumAdoptionOutcome::InstantiateAlpha {
            source: command.source(),
            course: reference,
        };
        store_receipt(
            &mut state,
            tenant,
            command.idempotency_key().clone(),
            CurriculumAdoptionOperation::InstantiateAlpha,
            actor,
            digest,
            outcome.clone(),
        );
        alpha_completed(outcome, command.idempotency_key(), false)
    })();
    rollback(&mut state, before, result)
}

pub(super) async fn preview_rollover(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    request: CourseRolloverPreviewRequest,
) -> Result<CourseRolloverPreviewView, StoreError> {
    let tenant = context.tenant_id();
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let course = require_exact_witness(&state, tenant, &request.witness)?;
    require_course_instructor(&state, tenant, course, actor)?;
    let payload = rollover_payload(&state, tenant, course)?;
    let payload = apply_pin_replacements(&state, store, tenant, &payload, &request.replacements)?;
    let CurriculumSemanticPayload::Course(semantic) = &payload else {
        unreachable!("rollover payload is course-sized")
    };
    let (course_view, corrections) =
        preview_course(&request.title, semantic, &request.target_term)?;
    Ok(CourseRolloverPreviewView {
        witness: request.witness,
        target_term: request.target_term,
        course: course_view,
        corrections,
        replacements: request.replacements,
        pin_correction: pin_correction(&state, tenant, &payload)?,
    })
}

pub(super) async fn apply_rollover(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    command: CourseRolloverCommand,
) -> Result<CourseRolloverCompleted, StoreError> {
    let tenant = context.tenant_id();
    let mut state = store.write_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let digest = request_digest(
        "rollover-course",
        actor,
        (
            command.preview_witness(),
            command.title(),
            command.target_term(),
            command.replacements(),
        ),
    )?;
    if let Some(outcome) = matching_receipt(
        &state,
        tenant,
        command.idempotency_key(),
        CurriculumAdoptionOperation::RolloverCourse,
        actor,
        digest,
    )? {
        return rollover_completed(outcome, command.idempotency_key(), true);
    }
    let before = state.clone();
    let result = (|| {
        let source_course = require_exact_witness(&state, tenant, command.preview_witness())?;
        require_course_instructor(&state, tenant, source_course, actor)?;
        let source_reference = command.preview_witness().course;
        let source_assignments = rollover_source_assignments(&state, tenant, source_course)?;
        let payload = rollover_payload(&state, tenant, source_course)?;
        let payload =
            apply_pin_replacements(&state, store, tenant, &payload, command.replacements())?;
        validate_destination_pins(&state, tenant, &payload).map_err(|_| StoreError::Conflict)?;
        let CurriculumSemanticPayload::Course(semantic) = payload else {
            unreachable!("rollover payload is course-sized")
        };
        for assignment in semantic
            .modules()
            .iter()
            .flat_map(|module| module.assignments())
        {
            assignment
                .schedule()
                .resolve_for_target_term(command.target_term())
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        }
        let course = random_course_id()?;
        let reference = super::super::super::courses::provision_course_locked(
            &mut state,
            CourseRecord {
                id: course,
                tenant,
                title: command.title().as_str().to_owned(),
                term: command.target_term().clone(),
            },
            actor,
        )?;
        let semantic_assignments = semantic
            .modules()
            .iter()
            .flat_map(|module| module.assignments())
            .collect::<Vec<_>>();
        if semantic_assignments.len() != source_assignments.len() {
            return Err(destination::integrity("rollover source assignment pairing"));
        }
        let mut assignment_ids = Vec::with_capacity(semantic_assignments.len());
        for (assignment, source_assignment) in semantic_assignments.iter().zip(source_assignments) {
            let (assignment_id, _) = destination::materialize_semantic_assignment(
                &mut state, context, course, assignment,
            )?;
            store_rollover_import(
                &mut state,
                tenant,
                assignment_id,
                (*assignment).clone(),
                StoredCurriculumSource::RolloverAssignment {
                    source_course: source_reference,
                    source_schedule_revision: command.preview_witness().schedule_revision,
                    source_assignment,
                },
                actor,
                command.idempotency_key(),
            );
            assignment_ids.push(assignment_id);
        }
        let occurred_at = state.authoritative_time;
        let adopted_payload = adopted_course_semantic(command.title(), &semantic)?;
        let adopted_digest = CurriculumSemanticPayload::course(adopted_payload.clone()).digest();
        state.curriculum_course_adoptions.insert(
            (tenant, course),
            StoredCourseAdoptionRecord {
                assignments: assignment_ids,
                payload: adopted_payload,
                digest: adopted_digest,
                receipt: command.idempotency_key().clone(),
            },
        );
        state.curriculum_course_envelopes.insert(
            (tenant, course),
            StoredCurriculumEnvelope {
                source: StoredCurriculumSource::Rollover {
                    source_course: source_reference,
                    source_schedule_revision: command.preview_witness().schedule_revision,
                },
                actor,
                occurred_at,
                receipt: command.idempotency_key().clone(),
            },
        );
        let outcome = MemoryCurriculumAdoptionOutcome::RolloverCourse {
            source_course: source_reference,
            course: reference,
        };
        store_receipt(
            &mut state,
            tenant,
            command.idempotency_key().clone(),
            CurriculumAdoptionOperation::RolloverCourse,
            actor,
            digest,
            outcome.clone(),
        );
        rollover_completed(outcome, command.idempotency_key(), false)
    })();
    rollback(&mut state, before, result)
}

fn rollover_source_assignments(
    state: &State,
    tenant: TenantId,
    course: CourseId,
) -> Result<Vec<ObservedAssignmentRevision>, StoreError> {
    let mut assignment_ids = state
        .assignments
        .iter()
        .filter_map(|((record_tenant, assignment), record)| {
            (*record_tenant == tenant && record.course_id == course).then_some(*assignment)
        })
        .collect::<Vec<_>>();
    assignment_ids.sort_unstable();
    assignment_ids
        .into_iter()
        .map(|assignment| {
            Ok(ObservedAssignmentRevision {
                assignment: *state
                    .assignment_references
                    .get(&(tenant, assignment))
                    .ok_or_else(|| destination::integrity("source assignment reference"))?,
                revision: *state
                    .assignment_revisions
                    .get(&(tenant, assignment))
                    .ok_or_else(|| destination::integrity("source assignment revision"))?,
            })
        })
        .collect()
}

fn rollover_payload(
    state: &State,
    tenant: TenantId,
    course: CourseId,
) -> Result<CurriculumSemanticPayload, StoreError> {
    let row = state
        .courses
        .get(&(tenant, course))
        .ok_or(StoreError::NotFound)?;
    let mut ids = state
        .assignments
        .iter()
        .filter_map(|((record_tenant, assignment), record)| {
            (*record_tenant == tenant && record.course_id == course).then_some(*assignment)
        })
        .collect::<Vec<_>>();
    ids.sort_unstable();
    let assignments = ids
        .into_iter()
        .map(|assignment| current_with_retained_schedule(state, tenant, assignment))
        .collect::<Result<Vec<_>, _>>()?;
    let module = CurriculumSemanticModule::new("Assignments".into(), assignments)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let course = CurriculumSemanticCourse::new(row.title.clone(), vec![module])
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    Ok(CurriculumSemanticPayload::course(course))
}

fn adopted_course_semantic(
    title: &question_model::CurriculumAdoptionTitle,
    source: &CurriculumSemanticCourse,
) -> Result<CurriculumSemanticCourse, StoreError> {
    CurriculumSemanticCourse::new(title.as_str().to_owned(), source.modules().to_vec())
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))
}

fn random_course_id() -> Result<CourseId, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("course ID randomness unavailable: {error}"))
    })
    .map(CourseId::from_uuid)
}
