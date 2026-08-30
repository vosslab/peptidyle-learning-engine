//! Current CourseInstance rollover and term-shift operations.

use question_model::{
    CourseInstanceEligibility, CourseInstanceWitness, RolloverCourseInstanceCommand,
    RolloverCourseInstanceManifest, RolloverCourseInstancePreview,
    RolloverCourseInstancePreviewRequest, RolloverReusableStateManifest,
    ShiftCourseInstanceTermCommand, ShiftCourseInstanceTermPreview,
    ShiftCourseInstanceTermPreviewRequest,
};

use super::super::{
    AssignmentAdoptionEvidenceDetail, StoredAssignmentAdoptionEvidence, StoredAssignmentImport,
    StoredWholeCourseAdoption, advance_course_schedule_revision, authorized_actor,
    course_has_any_run, course_instance_blueprint_application, course_witness,
    require_course_instructor, require_exact_witness, rollover_input,
};
use crate::in_memory::curriculum_adoption::destination;
use crate::in_memory::{MemoryStore, State};
use crate::{ActorContext, CourseRecord, SessionTokenHash, StoreError};

/// Post-state facts from a rollover core. The dispatcher retains the apply
/// record and builds immutable receipt evidence in its transaction envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AppliedRollover {
    pub(super) course: question_model::CourseReference,
    pub(super) outcome: CourseInstanceWitness,
}

/// Post-state facts from a term-shift core. The dispatcher owns receipt
/// construction from the original server-issued record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AppliedTermShift {
    pub(super) course: question_model::CourseReference,
    pub(super) outcome: CourseInstanceWitness,
}

/// Complete immutable facts for one assignment imported during a rollover.
///
/// Receipt authority stays at the transaction boundary; these facts bind the
/// exact stored assignment to its source and the course transition it applied.
struct RolloverAssignmentImportFacts {
    assignment: question_model::AssignmentId,
    source: question_model::AssignmentDefinitionSourceView,
    precondition: CourseInstanceWitness,
    outcome: CourseInstanceWitness,
    applied_assignment: question_model::ObservedCourseInstanceAssignment,
    import_revision: question_model::CurriculumImportRevision,
}

/// Projects rollover facts from the current source without creating apply authority.
pub(super) async fn preview_rollover_course_instance(
    store: &MemoryStore,
    context: ActorContext,
    session: SessionTokenHash,
    request: RolloverCourseInstancePreviewRequest,
) -> Result<RolloverCourseInstancePreview, StoreError> {
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let course = super::super::resolve_course(&state, request.source_course)?;
    require_course_instructor(&state, course, actor)?;
    let witness = course_witness(&state, course)?;
    let input = rollover_input(&state, course, &request.target_term)?;
    Ok(RolloverCourseInstancePreview {
        witness,
        target_term: request.target_term,
        manifest: manifest(&input)?,
        eligibility: CourseInstanceEligibility::Eligible,
    })
}

/// Projects a term shift, refusing a course whose delivery history has begun.
pub(super) async fn preview_shift_course_instance_term(
    store: &MemoryStore,
    context: ActorContext,
    session: SessionTokenHash,
    request: ShiftCourseInstanceTermPreviewRequest,
) -> Result<ShiftCourseInstanceTermPreview, StoreError> {
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let course = super::super::resolve_course(&state, request.course)?;
    require_course_instructor(&state, course, actor)?;
    course_instance_blueprint_application(&state, course)?;
    let witness = course_witness(&state, course)?;
    if course_has_any_run(&state, course) {
        return Ok(ShiftCourseInstanceTermPreview {
            witness: witness.clone(),
            target_term: request.target_term,
            schedules: question_model::BoundedResolvedScheduleSet::new(Vec::new())
                .map_err(|_| StoreError::Conflict)?,
            eligibility: CourseInstanceEligibility::Refused {
                refusal: question_model::CourseInstanceRefusal::IssuedWork {
                    course: witness.course,
                },
            },
        });
    }
    let mut schedules = Vec::new();
    let mut corrections = Vec::new();
    for assignment in witness.assignments() {
        let assignment_id = *state
            .assignments_by_reference
            .get(&assignment.assignment)
            .ok_or(StoreError::Conflict)?;
        let semantic =
            super::super::current_with_projected_teaching_schedule(&state, assignment_id)?;
        let (schedule, row_corrections) =
            crate::curriculum_adoption::preview_assignment(&semantic, &request.target_term)
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        schedules.push(schedule);
        corrections.extend(row_corrections);
    }
    let schedules = question_model::BoundedResolvedScheduleSet::new(schedules)
        .map_err(|_| StoreError::Conflict)?;
    let eligibility = if corrections.is_empty() {
        CourseInstanceEligibility::Eligible
    } else {
        CourseInstanceEligibility::Refused {
            refusal: question_model::CourseInstanceRefusal::ScheduleCorrectionsRequired {
                corrections,
            },
        }
    };
    Ok(ShiftCourseInstanceTermPreview {
        witness,
        target_term: request.target_term,
        schedules,
        eligibility,
    })
}

/// Consumes a server-built rollover command inside an existing write transition.
pub(super) fn apply_rollover_course_instance_locked(
    state: &mut State,
    _context: ActorContext,
    actor: question_model::UserId,
    command: &RolloverCourseInstanceCommand,
) -> Result<AppliedRollover, StoreError> {
    validate_rollover_command(actor, command)?;
    let source_course = require_exact_witness(state, command.source_course_instance())?;
    require_course_instructor(state, source_course, actor)?;
    let blueprint_application = course_instance_blueprint_application(state, source_course)?;
    if blueprint_application != command.blueprint_application() {
        return Err(StoreError::Conflict);
    }
    let input = rollover_input(state, source_course, command.target_term())?;
    if manifest(&input)? != *command.manifest() {
        return Err(StoreError::Conflict);
    }
    let course_id = random_course_id()?;
    let course = crate::in_memory::courses::provision_course_locked(
        state,
        CourseRecord {
            id: course_id,
            title: input.title.clone(),
            term: command.target_term().clone(),
        },
        actor,
    )?;
    if course != command.creation().reserved_course() {
        return Err(StoreError::Conflict);
    }
    if state
        .curriculum_adoption
        .course_instance_blueprint_applications
        .insert(course_id, blueprint_application)
        .is_some()
    {
        return Err(StoreError::Conflict);
    }
    let precondition = course_witness(state, course_id)?;
    let mut imported = Vec::new();
    for assignment in input.assignments() {
        let (assignment_id, reference) =
            destination::materialize_semantic_assignment(state, course_id, &assignment.semantic)?;
        imported.push((assignment_id, reference, assignment.source));
    }
    let destination = course_witness(state, course_id)?;
    for (assignment_id, reference, source) in imported {
        let applied_assignment = destination
            .assignments()
            .iter()
            .copied()
            .find(|observed| observed.assignment == reference)
            .ok_or_else(|| destination::integrity("rollover outcome assignment"))?;
        let import_revision = question_model::CurriculumImportRevision::new(1)
            .expect("initial import revision is bounded");
        store_import(
            state,
            RolloverAssignmentImportFacts {
                assignment: assignment_id,
                source,
                precondition: precondition.clone(),
                outcome: destination.clone(),
                applied_assignment,
                import_revision,
            },
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
    Ok(AppliedRollover {
        course,
        outcome: destination,
    })
}

/// Consumes the exact server-resolved term-shift schedule set inside one write transition.
pub(super) fn apply_shift_course_instance_term_locked(
    state: &mut State,
    _context: ActorContext,
    actor: question_model::UserId,
    command: &ShiftCourseInstanceTermCommand,
) -> Result<AppliedTermShift, StoreError> {
    if actor != command.authorized_actor() {
        return Err(StoreError::Conflict);
    }
    let course = require_exact_witness(state, command.destination())?;
    require_course_instructor(state, course, actor)?;
    let blueprint_application = course_instance_blueprint_application(state, course)?;
    if blueprint_application != command.blueprint_application() || course_has_any_run(state, course)
    {
        return Err(StoreError::Conflict);
    }
    let current = course_witness(state, course)?;
    let input = shift_schedules(state, &current, command.target_term())?;
    if input.as_slice() != command.schedules() {
        return Err(StoreError::Conflict);
    }
    for (observed, schedule) in current.assignments().iter().zip(command.schedules()) {
        let assignment = *state
            .assignments_by_reference
            .get(&observed.assignment)
            .ok_or(StoreError::Conflict)?;
        apply_schedule(state, assignment, schedule)?;
    }
    state
        .courses
        .get_mut(&course)
        .ok_or(StoreError::NotFound)?
        .term = command.target_term().clone();
    advance_course_schedule_revision(state, course)?;
    let destination = course_witness(state, course)?;
    Ok(AppliedTermShift {
        course: destination.course,
        outcome: destination,
    })
}

pub(super) fn manifest(
    input: &super::super::course_structure::RolloverInput,
) -> Result<RolloverCourseInstanceManifest, StoreError> {
    RolloverReusableStateManifest::new(
        input.blueprint_application.source,
        input.assignments().iter().map(|row| row.source).collect(),
        input
            .assignments()
            .iter()
            .map(|row| row.schedule.clone())
            .collect(),
    )
    .map(RolloverCourseInstanceManifest::new)
    .map_err(|_| StoreError::Conflict)
}

fn validate_rollover_command(
    actor: question_model::UserId,
    command: &RolloverCourseInstanceCommand,
) -> Result<(), StoreError> {
    (actor == command.creation().authorized_actor()
        && command
            .creation()
            .matches_rollover_source(command.source_course_instance())
        && command.creation().target_term() == command.target_term()
        && command.creation().idempotency_key() == command.idempotency_key())
    .then_some(())
    .ok_or(StoreError::Conflict)
}

pub(super) fn shift_schedules(
    state: &State,
    witness: &CourseInstanceWitness,
    target_term: &question_model::CourseTerm,
) -> Result<question_model::BoundedResolvedScheduleSet, StoreError> {
    let mut schedules = Vec::new();
    for observed in witness.assignments() {
        let assignment = *state
            .assignments_by_reference
            .get(&observed.assignment)
            .ok_or(StoreError::Conflict)?;
        let semantic = super::super::current_with_projected_teaching_schedule(state, assignment)?;
        let (schedule, corrections) =
            crate::curriculum_adoption::preview_assignment(&semantic, target_term)
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        if !corrections.is_empty() {
            return Err(StoreError::Conflict);
        }
        schedules.push(schedule);
    }
    question_model::BoundedResolvedScheduleSet::new(schedules).map_err(|_| StoreError::Conflict)
}

fn apply_schedule(
    state: &mut State,
    assignment: question_model::AssignmentId,
    schedule: &question_model::ResolvedRelativeAssignmentSchedule,
) -> Result<(), StoreError> {
    let revision = *state
        .assignment_revisions
        .get(&assignment)
        .ok_or_else(|| destination::integrity("assignment revision"))?;
    let next = crate::assignment_revision_checked_next(revision)?;
    let mut stored = state
        .assignment_base_policy
        .get(&assignment)
        .copied()
        .ok_or_else(|| destination::integrity("assignment base policy"))?;
    stored.policy.available_at = schedule.available_at.as_ref().map(|value| value.timestamp);
    stored.policy.due_at = schedule.due_at.as_ref().map(|value| value.timestamp);
    stored.policy.closes_at = schedule.closes_at.as_ref().map(|value| value.timestamp);
    stored.revision = next;
    state.assignment_base_policy.insert(assignment, stored);
    state.assignment_revisions.insert(assignment, next);
    Ok(())
}

fn store_import(
    state: &mut State,
    facts: RolloverAssignmentImportFacts,
    actor: question_model::UserId,
    key: &question_model::CurriculumAdoptionIdempotencyKey,
) {
    state.curriculum_adoption.import_records.insert(
        facts.assignment,
        StoredAssignmentImport {
            receipt_actor: actor,
            receipt_key: key.clone(),
            import_revision: facts.import_revision,
        },
    );
    state.curriculum_adoption.assignment_evidence.insert(
        (facts.assignment, facts.import_revision),
        StoredAssignmentAdoptionEvidence {
            receipt_actor: actor,
            receipt_key: key.clone(),
            source: facts.source,
            detail: AssignmentAdoptionEvidenceDetail::AdoptBlueprintAssignment {
                precondition: facts.precondition,
                outcome: facts.outcome,
                applied_assignment: facts.applied_assignment,
                import_revision: facts.import_revision,
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
