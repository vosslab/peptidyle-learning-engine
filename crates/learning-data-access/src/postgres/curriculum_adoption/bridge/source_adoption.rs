//! Source-backed curriculum-adoption projections.

use question_model::curriculum_adoption::CurriculumSemanticPayload;
use question_model::{
    AlphaInstantiationCommand, AlphaInstantiationPreviewRequest, AlphaInstantiationPreviewView,
    BlueprintInstantiationCommand, BlueprintInstantiationPreviewRequest,
    BlueprintInstantiationPreviewView, CreateSourceDerivedAssignmentCommand, ForkAlphaCommand,
    ForkAlphaPreviewRequest, ForkAlphaPreviewView, SourceDerivedAssignmentPreviewRequest,
    SourceDerivedAssignmentPreviewView,
};
use serde::Serialize;
use uuid::Uuid;

use crate::StoreError;
use crate::curriculum_adoption::{
    plan_assignment_materialization, preview_assignment, preview_course,
};

use super::{
    DestinationFactsV1, PinAvailabilityV1, PreparedCourseAssignmentV1, PreparedSemanticV1,
    SourceBindingV1, SourceFactsV1, prepare_course_assignments, prepare_semantic,
};

/// Canonical fork meaning and the exact Alpha revision it forks.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::postgres::curriculum_adoption) struct PreparedForkPlanV1 {
    pub(super) semantic: PreparedSemanticV1,
    pub(super) source: question_model::ObservedAlphaSource,
}

/// Canonical assignment meaning plus the exact live-course witness required
/// before SQL can mint a destination assignment.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::postgres::curriculum_adoption) struct PreparedSourceAssignmentPlanV1 {
    pub(super) semantic: PreparedSemanticV1,
    pub(super) source: SourceBindingV1,
    pub(super) destination_witness: question_model::CourseScheduleWitness,
    pub(super) target_term: question_model::CourseTerm,
    pub(super) preview: question_model::PreparedCurriculumAssignmentView,
    pub(super) corrections: Vec<question_model::CurriculumScheduleCorrection>,
    pub(super) materialization: crate::curriculum_adoption::AssignmentMaterializationPlan,
}

/// Canonical course meaning and Alpha source revision for a new teaching course.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::postgres::curriculum_adoption) struct PreparedSourceCoursePlanV1 {
    pub(super) semantic: PreparedSemanticV1,
    pub(super) source: question_model::ObservedAlphaSource,
    pub(super) target_term: question_model::CourseTerm,
    pub(super) preview: question_model::PreparedCurriculumCourseView,
    pub(super) corrections: Vec<question_model::CurriculumScheduleCorrection>,
    pub(super) assignments: Vec<PreparedCourseAssignmentV1>,
}

pub(in crate::postgres::curriculum_adoption) fn project_fork(
    request: &ForkAlphaPreviewRequest,
    facts: &SourceFactsV1,
) -> Result<ForkAlphaPreviewView, StoreError> {
    require_alpha_source(request.source, facts)?;
    require_replacements(&request.replacements, facts)?;
    let (payload, _) = prepare_source_semantic(facts)?;
    let CurriculumSemanticPayload::Course(course) = payload else {
        return Err(invalid_facts("fork source is not course-sized"));
    };
    Ok(ForkAlphaPreviewView {
        source: request.source,
        resulting_alpha_title: question_model::CurriculumAdoptionTitle::parse(course.title())
            .map_err(invalid_facts)?,
        replacements: request.replacements.clone(),
        pin_correction: pin_recovery(facts),
    })
}

pub(in crate::postgres::curriculum_adoption) fn prepare_fork(
    _preparation_id: Uuid,
    command: &ForkAlphaCommand,
    facts: &SourceFactsV1,
) -> Result<PreparedForkPlanV1, StoreError> {
    require_alpha_source(command.source(), facts)?;
    require_replacements(command.replacements(), facts)?;
    require_ready_to_apply(facts)?;
    let (payload, semantic) = prepare_source_semantic(facts)?;
    matches!(payload, CurriculumSemanticPayload::Course(_))
        .then_some(PreparedForkPlanV1 {
            semantic,
            source: command.source(),
        })
        .ok_or_else(|| invalid_facts("fork source is not course-sized"))
}

pub(in crate::postgres::curriculum_adoption) fn project_blueprint(
    request: &BlueprintInstantiationPreviewRequest,
    facts: &SourceFactsV1,
    destination: &DestinationFactsV1,
) -> Result<BlueprintInstantiationPreviewView, StoreError> {
    require_blueprint_source(request.source, facts)?;
    require_replacements(&request.replacements, facts)?;
    let target_term = require_target_term(facts, &request.target_term)?;
    let (assignment, corrections) = preview_assignment(
        &assignment_payload(prepare_source_semantic(facts)?.0)?,
        target_term,
    )
    .map_err(invalid_facts)?;
    Ok(BlueprintInstantiationPreviewView {
        source: request.source,
        course: request.course,
        target_term: request.target_term.clone(),
        witness: destination.witness.clone(),
        assignment,
        replacements: request.replacements.clone(),
        corrections,
        pin_correction: pin_recovery(facts),
    })
}

pub(in crate::postgres::curriculum_adoption) fn prepare_blueprint(
    _preparation_id: Uuid,
    command: &BlueprintInstantiationCommand,
    facts: &SourceFactsV1,
    destination: &DestinationFactsV1,
) -> Result<PreparedSourceAssignmentPlanV1, StoreError> {
    require_blueprint_source(command.source(), facts)?;
    require_replacements(command.replacements(), facts)?;
    require_target_term(facts, command.target_term())?;
    require_witness(command.preview_witness(), destination)?;
    require_ready_to_apply(facts)?;
    assignment_plan(facts, destination)
}

pub(in crate::postgres::curriculum_adoption) fn project_alpha(
    request: &AlphaInstantiationPreviewRequest,
    facts: &SourceFactsV1,
) -> Result<AlphaInstantiationPreviewView, StoreError> {
    require_alpha_source(request.source, facts)?;
    require_replacements(&request.replacements, facts)?;
    let target_term = require_target_term(facts, &request.target_term)?;
    let (course, corrections) = preview_course(
        &request.title,
        &course_payload(prepare_source_semantic(facts)?.0)?,
        target_term,
    )
    .map_err(invalid_facts)?;
    Ok(AlphaInstantiationPreviewView {
        source: request.source,
        target_term: request.target_term.clone(),
        course,
        replacements: request.replacements.clone(),
        corrections,
        pin_correction: pin_recovery(facts),
    })
}

pub(in crate::postgres::curriculum_adoption) fn prepare_alpha(
    _preparation_id: Uuid,
    command: &AlphaInstantiationCommand,
    facts: &SourceFactsV1,
) -> Result<PreparedSourceCoursePlanV1, StoreError> {
    require_alpha_source(command.source(), facts)?;
    require_replacements(command.replacements(), facts)?;
    require_target_term(facts, command.target_term())?;
    require_ready_to_apply(facts)?;
    course_plan(facts, command.title())
}

pub(in crate::postgres::curriculum_adoption) fn project_source_derived(
    request: &SourceDerivedAssignmentPreviewRequest,
    facts: &SourceFactsV1,
    destination: &DestinationFactsV1,
) -> Result<SourceDerivedAssignmentPreviewView, StoreError> {
    require_assignment_source(request.source, facts)?;
    require_replacements(&request.replacements, facts)?;
    let target_term = require_existing_destination_term(facts)?;
    let (assignment, corrections) = preview_assignment(
        &assignment_payload(prepare_source_semantic(facts)?.0)?,
        target_term,
    )
    .map_err(invalid_facts)?;
    Ok(SourceDerivedAssignmentPreviewView {
        course: request.course,
        source: request.source,
        witness: destination.witness.clone(),
        assignment,
        corrections,
        replacements: request.replacements.clone(),
        pin_correction: pin_recovery(facts),
    })
}

pub(in crate::postgres::curriculum_adoption) fn prepare_source_derived(
    _preparation_id: Uuid,
    command: &CreateSourceDerivedAssignmentCommand,
    facts: &SourceFactsV1,
    destination: &DestinationFactsV1,
) -> Result<PreparedSourceAssignmentPlanV1, StoreError> {
    require_assignment_source(command.source(), facts)?;
    require_replacements(command.replacements(), facts)?;
    require_existing_destination_term(facts)?;
    require_witness(command.preview_witness(), destination)?;
    require_ready_to_apply(facts)?;
    assignment_plan(facts, destination)
}

fn assignment_plan(
    facts: &SourceFactsV1,
    destination: &DestinationFactsV1,
) -> Result<PreparedSourceAssignmentPlanV1, StoreError> {
    let target_term = require_existing_destination_term(facts)?.clone();
    let (payload, semantic) = prepare_source_semantic(facts)?;
    let assignment = assignment_payload(payload)?;
    let materialization =
        plan_assignment_materialization(&assignment, &target_term).map_err(invalid_facts)?;
    let (preview, corrections) =
        preview_assignment(&assignment, &target_term).map_err(invalid_facts)?;
    Ok(PreparedSourceAssignmentPlanV1 {
        semantic,
        source: facts.current_source,
        destination_witness: destination.witness.clone(),
        target_term,
        preview,
        corrections,
        materialization,
    })
}

fn course_plan(
    facts: &SourceFactsV1,
    title: &question_model::CurriculumAdoptionTitle,
) -> Result<PreparedSourceCoursePlanV1, StoreError> {
    let target_term = require_existing_destination_term(facts)?.clone();
    let (payload, semantic) = prepare_source_semantic(facts)?;
    let course = course_payload(payload)?;
    let assignments = prepare_course_assignments(&course, &target_term)?;
    let (preview, corrections) =
        preview_course(title, &course, &target_term).map_err(invalid_facts)?;
    let SourceBindingV1::Alpha { source } = facts.current_source else {
        return Err(StoreError::Conflict);
    };
    Ok(PreparedSourceCoursePlanV1 {
        semantic,
        source,
        target_term,
        preview,
        corrections,
        assignments,
    })
}

fn prepare_source_semantic(
    facts: &SourceFactsV1,
) -> Result<(CurriculumSemanticPayload, PreparedSemanticV1), StoreError> {
    prepare_semantic(&facts.raw_semantic, &facts.resolved_replacements)
}
fn assignment_payload(
    payload: CurriculumSemanticPayload,
) -> Result<question_model::curriculum_adoption::CurriculumSemanticAssignment, StoreError> {
    let CurriculumSemanticPayload::Assignment(assignment) = payload else {
        return Err(invalid_facts("source is not assignment-sized"));
    };
    Ok(assignment)
}
fn course_payload(
    payload: CurriculumSemanticPayload,
) -> Result<question_model::curriculum_adoption::CurriculumSemanticCourse, StoreError> {
    let CurriculumSemanticPayload::Course(course) = payload else {
        return Err(invalid_facts("source is not course-sized"));
    };
    Ok(course)
}
fn require_alpha_source(
    expected: question_model::ObservedAlphaSource,
    facts: &SourceFactsV1,
) -> Result<(), StoreError> {
    let (SourceBindingV1::Alpha { source: requested }, SourceBindingV1::Alpha { source: current }) =
        (facts.requested_source, facts.current_source)
    else {
        return Err(StoreError::Conflict);
    };
    (requested == expected && current == expected)
        .then_some(())
        .ok_or(StoreError::Conflict)
}
fn require_blueprint_source(
    expected: question_model::ObservedBlueprintSource,
    facts: &SourceFactsV1,
) -> Result<(), StoreError> {
    let (
        SourceBindingV1::Blueprint { source: requested },
        SourceBindingV1::Blueprint { source: current },
    ) = (facts.requested_source, facts.current_source)
    else {
        return Err(StoreError::Conflict);
    };
    (requested == expected && current == expected)
        .then_some(())
        .ok_or(StoreError::Conflict)
}
fn require_assignment_source(
    expected: question_model::AssignmentDefinitionSourceView,
    facts: &SourceFactsV1,
) -> Result<(), StoreError> {
    let (
        SourceBindingV1::Assignment { source: requested },
        SourceBindingV1::Assignment { source: current },
    ) = (facts.requested_source, facts.current_source)
    else {
        return Err(StoreError::Conflict);
    };
    (requested == expected && current == expected)
        .then_some(())
        .ok_or(StoreError::Conflict)
}
fn require_replacements(
    expected: &question_model::CurriculumPinReplacements,
    facts: &SourceFactsV1,
) -> Result<(), StoreError> {
    (facts.requested_replacements == *expected)
        .then_some(())
        .ok_or(StoreError::Conflict)
}
fn require_target_term<'a>(
    facts: &'a SourceFactsV1,
    expected: &question_model::CourseTerm,
) -> Result<&'a question_model::CourseTerm, StoreError> {
    let target_term = facts
        .target_term
        .as_ref()
        .ok_or_else(|| invalid_facts("source fact is missing target term"))?;
    (target_term == expected)
        .then_some(target_term)
        .ok_or(StoreError::Conflict)
}
fn require_existing_destination_term(
    facts: &SourceFactsV1,
) -> Result<&question_model::CourseTerm, StoreError> {
    facts
        .target_term
        .as_ref()
        .ok_or_else(|| invalid_facts("destination fact is missing current term"))
}
fn require_witness(
    expected: &question_model::CourseScheduleWitness,
    destination: &DestinationFactsV1,
) -> Result<(), StoreError> {
    (destination.witness == *expected)
        .then_some(())
        .ok_or(StoreError::Conflict)
}
fn require_ready_to_apply(facts: &SourceFactsV1) -> Result<(), StoreError> {
    matches!(facts.pin_availability, PinAvailabilityV1::Available)
        .then_some(())
        .ok_or(StoreError::Conflict)
}
fn pin_recovery(facts: &SourceFactsV1) -> Option<question_model::UnavailablePinRecoveryAction> {
    match &facts.pin_availability {
        PinAvailabilityV1::Available => None,
        PinAvailabilityV1::Unavailable { pin, candidates } => Some(
            question_model::UnavailablePinRecoveryAction::SelectReplacementQuestion {
                position: pin.position(),
                candidates: candidates.clone(),
            },
        ),
    }
}
fn invalid_facts(error: impl std::fmt::Display) -> StoreError {
    StoreError::InvalidRecord(format!(
        "curriculum adoption source facts are invalid: {error}"
    ))
}
