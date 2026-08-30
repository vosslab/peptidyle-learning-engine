//! Current BlueprintCourse source-adoption operations.
//!
//! These operations own only reusable-source materialization.  CourseInstance
//! lifecycle, controlled updates, reconciliation, and Store dispatch remain in
//! their dedicated package modules.

use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticPayload,
};
use question_model::{
    AdoptBlueprintAssignmentCommand, AdoptBlueprintAssignmentPreviewRequest,
    AdoptBlueprintAssignmentPreviewView, AssignmentDefinitionSourceView,
    BlueprintAdoptionEligibility, BlueprintAdoptionRefusal, CourseInstanceImportWitness,
    CourseInstanceWitness, ForkBlueprintCourseCommand, ForkBlueprintCoursePreviewRequest,
    ForkBlueprintCoursePreviewView, InstantiateBlueprintCourseCommand,
    InstantiateBlueprintCoursePreviewRequest, InstantiateBlueprintCoursePreviewView,
    ObservedBlueprintSource,
};

use super::super::{
    AssignmentAdoptionEvidenceDetail, MemoryCurriculumAdoptionEvidence,
    MemoryCurriculumAdoptionOutcome, StoredAssignmentAdoptionEvidence, StoredAssignmentImport,
    StoredWholeCourseAdoption, assignment_source_snapshot_with_replacements, authorized_actor,
    course_instance_blueprint_application, course_witness, pin_correction,
    require_course_instructor, require_exact_witness, source_snapshot_with_replacements,
    validate_destination_pins,
};
use crate::curriculum_adoption::{preview_assignment, preview_course};
use crate::in_memory::curriculum_adoption::destination;
use crate::in_memory::{MemoryStore, State};
use crate::{CourseRecord, SessionTokenHash, StoreError, TenantContext};

use super::AppliedCurriculumAdoption;

/// Resolves one fork request without granting mutation authority.
pub(super) async fn preview_fork_blueprint_course(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    request: ForkBlueprintCoursePreviewRequest,
) -> Result<ForkBlueprintCoursePreviewView, StoreError> {
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let source = source_snapshot_with_replacements(
        &state,
        store,
        context.tenant_id(),
        actor,
        request.source,
        &request.replacements,
    )?;
    Ok(ForkBlueprintCoursePreviewView {
        source: request.source,
        replacements: request.replacements,
        eligibility: blueprint_eligibility(
            &state,
            context.tenant_id(),
            request.source,
            &source.payload,
        )?,
    })
}

/// Resolves one existing-CourseInstance assignment adoption without mutation.
pub(super) async fn preview_adopt_blueprint_assignment(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    request: AdoptBlueprintAssignmentPreviewRequest,
) -> Result<AdoptBlueprintAssignmentPreviewView, StoreError> {
    let tenant = context.tenant_id();
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let course = super::super::resolve_course(&state, tenant, request.course)?;
    require_course_instructor(&state, tenant, course, actor)?;
    course_instance_blueprint_application(&state, tenant, course)?;
    let destination = course_witness(&state, tenant, course)?;
    let source = assignment_source_snapshot_with_replacements(
        &state,
        store,
        tenant,
        actor,
        request.source,
        &request.replacements,
    )?;
    let eligibility = assignment_eligibility(
        &state,
        tenant,
        request.source,
        &source.payload,
        &state
            .courses
            .get(&(tenant, course))
            .ok_or(StoreError::NotFound)?
            .term,
    )?;
    Ok(AdoptBlueprintAssignmentPreviewView {
        source: request.source,
        destination,
        replacements: request.replacements,
        eligibility,
    })
}

/// Resolves a whole BlueprintCourse instantiation without mutation.
pub(super) async fn preview_instantiate_blueprint_course(
    store: &MemoryStore,
    context: TenantContext,
    session: SessionTokenHash,
    request: InstantiateBlueprintCoursePreviewRequest,
) -> Result<InstantiateBlueprintCoursePreviewView, StoreError> {
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let source = source_snapshot_with_replacements(
        &state,
        store,
        context.tenant_id(),
        actor,
        request.source,
        &request.replacements,
    )?;
    let eligibility =
        blueprint_eligibility(&state, context.tenant_id(), request.source, &source.payload)?;
    let eligibility = match eligibility {
        BlueprintAdoptionEligibility::Eligible => {
            let CurriculumSemanticPayload::Course(course) = source.payload else {
                return Err(StoreError::Conflict);
            };
            let (_, corrections) =
                preview_course(&course, &request.target_term).map_err(semantic_preview_error)?;
            if corrections.is_empty() {
                BlueprintAdoptionEligibility::Eligible
            } else {
                BlueprintAdoptionEligibility::Refused {
                    refusal: BlueprintAdoptionRefusal::ScheduleCorrectionsRequired { corrections },
                }
            }
        }
        refusal => refusal,
    };
    Ok(InstantiateBlueprintCoursePreviewView {
        source: request.source,
        target_term: request.target_term,
        replacements: request.replacements,
        eligibility,
    })
}

/// Applies one exact fork inside an existing Memory write transition.
///
/// The dispatcher owns the enclosing authorization, canonical-intent, replay,
/// receipt, and rollback boundary (ASVS 2.3.1, 2.3.3).
pub(super) fn apply_fork_blueprint_course_locked(
    state: &mut State,
    store: &MemoryStore,
    context: TenantContext,
    actor: question_model::UserId,
    command: &ForkBlueprintCourseCommand,
) -> Result<AppliedCurriculumAdoption, StoreError> {
    let tenant = context.tenant_id();
    let creation = command.creation();
    if actor != creation.authorized_actor()
        || command.idempotency_key() != creation.idempotency_key()
    {
        return Err(StoreError::Conflict);
    }
    let source = source_snapshot_with_replacements(
        state,
        store,
        tenant,
        actor,
        *command.source(),
        command.replacements(),
    )?;
    validate_destination_pins(state, tenant, &source.payload).map_err(|_| StoreError::Conflict)?;
    let CurriculumSemanticPayload::Course(course) = source.payload else {
        return Err(StoreError::Conflict);
    };
    let created_reference =
        super::super::super::reusable_curriculum::create_blueprint_course_from_semantic_locked(
            state, tenant, actor, &course,
        )?;
    if created_reference != creation.reserved_blueprint() {
        return Err(StoreError::Conflict);
    }
    let created = ObservedBlueprintSource {
        reference: created_reference,
        revision: question_model::BlueprintRevision::INITIAL,
    };
    Ok(AppliedCurriculumAdoption {
        outcome: MemoryCurriculumAdoptionOutcome::ForkBlueprintCourse {
            source: *command.source(),
            created,
        },
        evidence: MemoryCurriculumAdoptionEvidence::ForkBlueprintCourse {
            source: *command.source(),
            created,
        },
    })
}

/// Applies one bounded assignment adoption inside an existing write transition.
pub(super) fn apply_adopt_blueprint_assignment_locked(
    state: &mut State,
    store: &MemoryStore,
    context: TenantContext,
    actor: question_model::UserId,
    command: &AdoptBlueprintAssignmentCommand,
) -> Result<AppliedCurriculumAdoption, StoreError> {
    let tenant = context.tenant_id();
    if actor != command.authorized_actor() {
        return Err(StoreError::Conflict);
    }
    let course = require_exact_witness(state, tenant, command.destination())?;
    require_course_instructor(state, tenant, course, actor)?;
    if course_instance_blueprint_application(state, tenant, course)?
        != command.blueprint_application()
    {
        return Err(StoreError::Conflict);
    }
    let source = assignment_source_snapshot_with_replacements(
        state,
        store,
        tenant,
        actor,
        *command.source(),
        command.replacements(),
    )?;
    validate_destination_pins(state, tenant, &source.payload).map_err(|_| StoreError::Conflict)?;
    let assignment = only_assignment(&source.payload)?.clone();
    let (assignment_id, reference) =
        destination::materialize_semantic_assignment(state, context, course, &assignment)?;
    let import_revision = question_model::CurriculumImportRevision::new(1)
        .expect("initial import revision is bounded");
    let import = CourseInstanceImportWitness {
        source: *command.source(),
        destination: question_model::ObservedCourseInstanceAssignment {
            assignment: reference,
            revision: *state
                .assignment_revisions
                .get(&(tenant, assignment_id))
                .ok_or(StoreError::Conflict)?,
        },
        import_revision,
    };
    let outcome = course_witness(state, tenant, course)?;
    let applied_assignment = import.destination;
    store_assignment_import(
        state,
        assignment_id,
        import,
        command.destination().clone(),
        outcome.clone(),
        actor,
        command.idempotency_key(),
    );
    Ok(AppliedCurriculumAdoption {
        outcome: MemoryCurriculumAdoptionOutcome::AdoptBlueprintAssignment {
            course: command.destination().course,
            assignment: reference,
        },
        evidence: MemoryCurriculumAdoptionEvidence::AdoptBlueprintAssignment {
            source: *command.source(),
            precondition: command.destination().clone(),
            applied_assignment,
            outcome,
            import_revision,
        },
    })
}

/// Applies a complete BlueprintCourse instantiation inside an existing write transition.
pub(super) fn apply_instantiate_blueprint_course_locked(
    state: &mut State,
    store: &MemoryStore,
    context: TenantContext,
    actor: question_model::UserId,
    command: &InstantiateBlueprintCourseCommand,
) -> Result<AppliedCurriculumAdoption, StoreError> {
    let tenant = context.tenant_id();
    let creation = command.creation();
    if actor != creation.authorized_actor()
        || !creation.matches_blueprint_source(command.source())
        || creation.target_term() != command.target_term()
        || command.idempotency_key() != creation.idempotency_key()
    {
        return Err(StoreError::Conflict);
    }
    let source = source_snapshot_with_replacements(
        state,
        store,
        tenant,
        actor,
        *command.source(),
        command.replacements(),
    )?;
    validate_destination_pins(state, tenant, &source.payload).map_err(|_| StoreError::Conflict)?;
    let CurriculumSemanticPayload::Course(course_semantic) = source.payload else {
        return Err(StoreError::Conflict);
    };
    let course_id = random_course_id()?;
    let course_reference = super::super::super::courses::provision_course_locked(
        state,
        CourseRecord {
            id: course_id,
            tenant,
            title: course_semantic.title().to_owned(),
            term: command.target_term().clone(),
        },
        actor,
    )?;
    if course_reference != creation.reserved_course() {
        return Err(StoreError::Conflict);
    }
    let blueprint_application = question_model::CourseInstanceBlueprintApplication {
        source: *command.source(),
    };
    if state
        .curriculum_adoption
        .course_instance_blueprint_applications
        .insert((tenant, course_id), blueprint_application)
        .is_some()
    {
        return Err(StoreError::Conflict);
    }
    let precondition = course_witness(state, tenant, course_id)?;
    let source_assignments = super::super::super::reusable_curriculum::course_assignment_sources(
        state,
        tenant,
        *command.source(),
    )?;
    let assignments = course_semantic
        .modules()
        .iter()
        .flat_map(|module| module.assignments().iter())
        .collect::<Vec<_>>();
    if assignments.len() != source_assignments.len() {
        return Err(StoreError::Conflict);
    }
    let mut imports = Vec::new();
    for (assignment, source_view) in assignments.into_iter().zip(source_assignments) {
        let (assignment_id, reference) =
            destination::materialize_semantic_assignment(state, context, course_id, assignment)?;
        let import_revision = question_model::CurriculumImportRevision::new(1)
            .expect("initial import revision is bounded");
        imports.push((
            assignment_id,
            CourseInstanceImportWitness {
                source: source_view,
                destination: question_model::ObservedCourseInstanceAssignment {
                    assignment: reference,
                    revision: *state
                        .assignment_revisions
                        .get(&(tenant, assignment_id))
                        .ok_or(StoreError::Conflict)?,
                },
                import_revision,
            },
        ));
    }
    let destination = course_witness(state, tenant, course_id)?;
    for (assignment_id, import) in imports {
        store_assignment_import(
            state,
            assignment_id,
            import,
            precondition.clone(),
            destination.clone(),
            actor,
            command.idempotency_key(),
        );
    }
    state.curriculum_adoption.whole_course_adoptions.insert(
        course_id,
        StoredWholeCourseAdoption {
            receipt_actor: actor,
            receipt_key: command.idempotency_key().clone(),
            destination: destination.clone(),
            blueprint_application,
        },
    );
    Ok(AppliedCurriculumAdoption {
        outcome: MemoryCurriculumAdoptionOutcome::InstantiateBlueprintCourse {
            course: course_reference,
        },
        evidence: MemoryCurriculumAdoptionEvidence::InstantiateBlueprintCourse {
            source: *command.source(),
            destination,
            blueprint_application,
        },
    })
}

fn blueprint_eligibility(
    state: &State,
    tenant: question_model::TenantId,
    source: ObservedBlueprintSource,
    payload: &CurriculumSemanticPayload,
) -> Result<BlueprintAdoptionEligibility, StoreError> {
    match validate_destination_pins(state, tenant, payload) {
        Ok(()) => Ok(BlueprintAdoptionEligibility::Eligible),
        Err(_) => Ok(BlueprintAdoptionEligibility::Refused {
            refusal: BlueprintAdoptionRefusal::UnavailablePin {
                recovery: pin_correction_for_payload(state, tenant, source, payload)?,
            },
        }),
    }
}

fn assignment_eligibility(
    state: &State,
    tenant: question_model::TenantId,
    source: AssignmentDefinitionSourceView,
    payload: &CurriculumSemanticPayload,
    term: &question_model::CourseTerm,
) -> Result<BlueprintAdoptionEligibility, StoreError> {
    if validate_destination_pins(state, tenant, payload).is_err() {
        return Ok(BlueprintAdoptionEligibility::Refused {
            refusal: BlueprintAdoptionRefusal::UnavailablePin {
                recovery: pin_correction(state, tenant, source, payload)?
                    .ok_or(StoreError::Conflict)?,
            },
        });
    }
    let (_, corrections) =
        preview_assignment(only_assignment(payload)?, term).map_err(semantic_preview_error)?;
    Ok(if corrections.is_empty() {
        BlueprintAdoptionEligibility::Eligible
    } else {
        BlueprintAdoptionEligibility::Refused {
            refusal: BlueprintAdoptionRefusal::ScheduleCorrectionsRequired { corrections },
        }
    })
}

fn pin_correction_for_payload(
    state: &State,
    tenant: question_model::TenantId,
    source: ObservedBlueprintSource,
    payload: &CurriculumSemanticPayload,
) -> Result<question_model::UnavailableCurriculumPinRecovery, StoreError> {
    let position = super::super::unavailable_destination_pin(state, tenant, payload)?
        .ok_or(StoreError::Conflict)?
        .position();
    let module = position.module_index().ok_or(StoreError::Conflict)?;
    let assignment_source =
        super::super::super::reusable_curriculum::course_assignment_source_at_position(
            state,
            tenant,
            source,
            module,
            position.assignment_index(),
        )?;
    pin_correction(state, tenant, assignment_source, payload)?.ok_or(StoreError::Conflict)
}

fn only_assignment(
    payload: &CurriculumSemanticPayload,
) -> Result<&CurriculumSemanticAssignment, StoreError> {
    let CurriculumSemanticPayload::Assignment(assignment) = payload else {
        return Err(StoreError::Conflict);
    };
    Ok(assignment)
}

fn store_assignment_import(
    state: &mut State,
    assignment: question_model::AssignmentId,
    import: CourseInstanceImportWitness,
    precondition: CourseInstanceWitness,
    outcome: CourseInstanceWitness,
    actor: question_model::UserId,
    key: &question_model::CurriculumAdoptionIdempotencyKey,
) {
    state.curriculum_adoption.import_records.insert(
        assignment,
        StoredAssignmentImport {
            receipt_actor: actor,
            receipt_key: key.clone(),
            import_revision: import.import_revision,
        },
    );
    state.curriculum_adoption.assignment_evidence.insert(
        (assignment, import.import_revision),
        StoredAssignmentAdoptionEvidence {
            receipt_actor: actor,
            receipt_key: key.clone(),
            source: import.source,
            detail: AssignmentAdoptionEvidenceDetail::AdoptBlueprintAssignment {
                precondition,
                applied_assignment: import.destination,
                outcome,
                import_revision: import.import_revision,
            },
        },
    );
}

fn random_course_id() -> Result<question_model::CourseId, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("course ID randomness unavailable: {error}"))
    })
    .map(question_model::CourseId::from_uuid)
}

fn semantic_preview_error(error: crate::curriculum_adoption::SemanticPlannerError) -> StoreError {
    StoreError::InvalidRecord(error.to_string())
}
