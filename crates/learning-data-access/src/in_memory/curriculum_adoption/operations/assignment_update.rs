//! Controlled Blueprint assignment updates, selected copies, and derived-index repair.
//!
//! Each mutation is a lock-held core. The dispatcher owns the single Memory write transition
//! that issues the non-Serde record and consumes it, so no operation nests store locks.

use question_model::curriculum_adoption::{
    CurriculumSemanticComparison, CurriculumSemanticPayload,
};
use question_model::{
    AppliedAssignmentImportEvidence, ControlledUpdateBlueprintAssignmentCommand,
    ControlledUpdateBlueprintAssignmentPreview, ControlledUpdateBlueprintAssignmentPreviewRequest,
    ControlledUpdateEffect, CourseInstanceBlueprintInspectionView, CourseInstanceEligibility,
    CourseInstanceImportWitness, CourseInstanceReceiptTarget, CourseInstanceRefusal,
    CourseInstanceWitness, CreateSelectedBlueprintAssignmentCommand,
    CreateSelectedBlueprintAssignmentPreview, CreateSelectedBlueprintAssignmentPreviewRequest,
    CurriculumImportRevision, ReconcileCourseInstanceAdoptionCompleted,
};

use super::super::{
    AssignmentAdoptionEvidenceDetail, StoredAssignmentAdoptionEvidence, StoredAssignmentImport,
    assignment_has_run, assignment_source_snapshot_with_replacements, authorized_actor,
    course_instance_blueprint_application, course_witness, rebuild_current_projection,
    require_course_instructor, require_exact_witness, resolve_course,
    resolve_reconciliation_target, validate_destination_pins, validate_receipt_evidence,
};
use crate::curriculum_adoption::{
    CourseInstanceInspectionInput, preview_assignment, project_course_instance_blueprint_inspection,
};
use crate::in_memory::curriculum_adoption::destination;
use crate::in_memory::curriculum_adoption::receipt_evidence::validate_assignment_import_projection;
use crate::in_memory::{MemoryStore, State};
use crate::{ActorContext, SessionTokenHash, StoreError};

/// Resolves an answer-free controlled-update preview from current immutable import evidence.
pub(super) async fn preview_controlled_update_blueprint_assignment(
    store: &MemoryStore,
    context: ActorContext,
    session: SessionTokenHash,
    request: ControlledUpdateBlueprintAssignmentPreviewRequest,
) -> Result<ControlledUpdateBlueprintAssignmentPreview, StoreError> {
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let course = resolve_course(&state, request.course)?;
    require_course_instructor(&state, course, actor)?;
    let _blueprint_application = course_instance_blueprint_application(&state, course)?;
    let witness = course_witness(&state, course)?;
    let import = current_import_witness(&state, course, request.assignment)?;
    let eligibility =
        controlled_update_eligibility(&state, actor, &witness, &import, request.source)?;
    Ok(ControlledUpdateBlueprintAssignmentPreview {
        import,
        witness,
        eligibility,
    })
}

/// Resolves an answer-free selected-copy preview with the server-resolved target schedule.
pub(super) async fn preview_create_selected_blueprint_assignment(
    store: &MemoryStore,
    context: ActorContext,
    session: SessionTokenHash,
    request: CreateSelectedBlueprintAssignmentPreviewRequest,
) -> Result<CreateSelectedBlueprintAssignmentPreview, StoreError> {
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let course = resolve_course(&state, request.course)?;
    require_course_instructor(&state, course, actor)?;
    let _blueprint_application = course_instance_blueprint_application(&state, course)?;
    let witness = course_witness(&state, course)?;
    let source = assignment_source_snapshot_with_replacements(
        &state,
        store,
        actor,
        request.source,
        &request.replacements,
    )?;
    let semantic = only_assignment(&source.payload)?;
    let term = &state.courses.get(&course).ok_or(StoreError::NotFound)?.term;
    let (schedule, corrections) = preview_assignment(semantic, term).map_err(semantic_error)?;
    let eligibility = if validate_destination_pins(&state, &source.payload).is_err() {
        CourseInstanceEligibility::Refused {
            refusal: CourseInstanceRefusal::UnavailablePin {
                recovery: super::super::pin_correction(&state, request.source, &source.payload)?
                    .ok_or(StoreError::Conflict)?,
            },
        }
    } else if corrections.is_empty() {
        CourseInstanceEligibility::Eligible
    } else {
        CourseInstanceEligibility::Refused {
            refusal: CourseInstanceRefusal::ScheduleCorrectionsRequired { corrections },
        }
    };
    Ok(CreateSelectedBlueprintAssignmentPreview {
        source: request.source,
        witness,
        schedule,
        eligibility,
    })
}

/// Loads one answer-free CourseInstance provenance projection from immutable evidence.
pub(super) async fn inspect_course_instance_blueprint_adoption(
    store: &MemoryStore,
    context: ActorContext,
    session: SessionTokenHash,
    course_reference: question_model::CourseReference,
) -> Result<Option<CourseInstanceBlueprintInspectionView>, StoreError> {
    let state = store.read_state()?;
    let actor = authorized_actor(&state, context, session)?;
    let course = resolve_course(&state, course_reference)?;
    require_course_instructor(&state, course, actor)?;
    let initial_blueprint_application = course_instance_blueprint_application(&state, course)?;
    let witness = course_witness(&state, course)?;
    let assignments = witness
        .assignments()
        .iter()
        .map(|observed| {
            let assignment = assignment_id_for_reference(&state, course, observed.assignment)?;
            let import = state
                .curriculum_adoption
                .import_records
                .get(&assignment)
                .ok_or_else(|| {
                    destination::integrity("CourseInstance inspection import projection")
                })?;
            let evidence = validate_assignment_import_projection(
                &state,
                course_reference,
                assignment,
                observed.assignment,
                import,
            )?;
            Ok(question_model::BlueprintAssignmentProvenance {
                source: evidence.source,
                import_revision: evidence.import_revision(),
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    project_course_instance_blueprint_inspection(CourseInstanceInspectionInput {
        initial_blueprint_application,
        witness,
        assignments,
    })
    .map(Some)
    .map_err(semantic_error)
}

/// Values returned from a locked mutation core for the outer M5 receipt writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AppliedControlledUpdate {
    pub(super) course: question_model::CourseReference,
    pub(super) assignment: question_model::AssignmentReference,
    pub(super) import_revision: CurriculumImportRevision,
    pub(super) outcome: CourseInstanceWitness,
    pub(super) applied: AppliedAssignmentImportEvidence,
    pub(super) effect: ControlledUpdateEffect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AppliedSelectedCopy {
    pub(super) course: question_model::CourseReference,
    pub(super) assignment: question_model::AssignmentReference,
    pub(super) import_revision: CurriculumImportRevision,
    pub(super) outcome: CourseInstanceWitness,
    pub(super) applied: AppliedAssignmentImportEvidence,
}

/// Consumes the server-issued controlled-update record inside an existing write transition.
///
/// M5 owns replay lookup, receipt creation, receipt insertion, and rollback around this core.
pub(super) fn apply_controlled_update_blueprint_assignment_locked(
    state: &mut State,
    context: ActorContext,
    actor: question_model::UserId,
    command: &ControlledUpdateBlueprintAssignmentCommand,
) -> Result<AppliedControlledUpdate, StoreError> {
    if actor != command.authorized_actor() {
        return Err(StoreError::Conflict);
    }
    let course = require_exact_witness(state, command.destination())?;
    require_course_instructor(state, course, actor)?;
    if course_instance_blueprint_application(state, course)? != command.blueprint_application() {
        return Err(StoreError::Conflict);
    }
    let eligibility = controlled_update_eligibility(
        state,
        actor,
        command.destination(),
        command.import(),
        command.source(),
    )?;
    if eligibility != CourseInstanceEligibility::Eligible {
        return Err(StoreError::Conflict);
    }
    let assignment =
        assignment_id_for_reference(state, course, command.import().destination.assignment)?;
    let source = super::super::super::reusable_curriculum::curriculum_assignment_source_snapshot(
        state,
        command.source(),
    )?;
    validate_destination_pins(state, &source.payload).map_err(|_| StoreError::Conflict)?;
    let semantic = only_assignment(&source.payload)?;
    let imported = super::super::super::reusable_curriculum::curriculum_assignment_source_snapshot(
        state,
        command.import().source,
    )?;
    let original = only_assignment(&imported.payload)?;
    let current =
        destination::current_semantic_assignment(state, assignment, original.schedule().clone())?;
    let current_payload = CurriculumSemanticPayload::assignment(current);
    let proposed_payload = CurriculumSemanticPayload::assignment(semantic.clone());
    let (effect, semantic_digest) = match current_payload.compare(&proposed_payload) {
        CurriculumSemanticComparison::Changed { actual, .. } => {
            destination::replace_reusable_meaning(state, assignment, semantic)?;
            super::super::advance_course_schedule_revision(state, course)?;
            (ControlledUpdateEffect::MeaningChanged, actual)
        }
        CurriculumSemanticComparison::Equivalent { digest } => {
            (ControlledUpdateEffect::SourceRevisionOnly, digest)
        }
    };
    let import_revision = next_import_revision(command.import().import_revision)?;
    let outcome = course_witness(state, course)?;
    let assignment_reference = command.import().destination.assignment;
    let observed = outcome
        .assignments()
        .iter()
        .copied()
        .find(|value| value.assignment == assignment_reference)
        .ok_or_else(|| destination::integrity("controlled update outcome assignment"))?;
    let applied = AppliedAssignmentImportEvidence::new(
        command.source(),
        question_model::CurriculumPinReplacements::default(),
        semantic_digest,
        observed,
        import_revision,
    );
    state.curriculum_adoption.import_records.insert(
        assignment,
        StoredAssignmentImport {
            receipt_actor: actor,
            receipt_key: command.idempotency_key().clone(),
            import_revision,
        },
    );
    state.curriculum_adoption.assignment_evidence.insert(
        (assignment, import_revision),
        StoredAssignmentAdoptionEvidence {
            receipt_actor: actor,
            receipt_key: command.idempotency_key().clone(),
            source: command.source(),
            detail: AssignmentAdoptionEvidenceDetail::ControlledUpdate {
                precondition: command.destination().clone(),
                outcome: outcome.clone(),
                effect,
                replacements: question_model::CurriculumPinReplacements::default(),
                semantic_digest,
                applied_assignment: observed,
                import_revision,
            },
        },
    );
    Ok(AppliedControlledUpdate {
        course: command.destination().course,
        assignment: assignment_reference,
        import_revision,
        outcome,
        applied,
        effect,
    })
}

/// Consumes the server-issued selected-copy record inside an existing write transition.
///
/// M5 owns replay lookup, receipt creation, receipt insertion, and rollback around this core.
pub(super) fn apply_create_selected_blueprint_assignment_locked(
    state: &mut State,
    store: &MemoryStore,
    context: ActorContext,
    actor: question_model::UserId,
    command: &CreateSelectedBlueprintAssignmentCommand,
) -> Result<AppliedSelectedCopy, StoreError> {
    if actor != command.authorized_actor() {
        return Err(StoreError::Conflict);
    }
    let course = require_exact_witness(state, command.destination())?;
    require_course_instructor(state, course, actor)?;
    if course_instance_blueprint_application(state, course)? != command.blueprint_application() {
        return Err(StoreError::Conflict);
    }
    let source = assignment_source_snapshot_with_replacements(
        state,
        store,
        actor,
        command.source(),
        command.replacements(),
    )?;
    validate_destination_pins(state, &source.payload).map_err(|_| StoreError::Conflict)?;
    let semantic = only_assignment(&source.payload)?;
    let semantic_digest = source.payload.digest();
    let term = &state.courses.get(&course).ok_or(StoreError::NotFound)?.term;
    let (schedule, corrections) = preview_assignment(semantic, term).map_err(semantic_error)?;
    if !corrections.is_empty() || schedule != *command.schedule() {
        return Err(StoreError::Conflict);
    }
    let (assignment, reference) =
        destination::materialize_semantic_assignment(state, course, semantic)?;
    let import_revision =
        CurriculumImportRevision::new(1).expect("initial import revision is bounded");
    let outcome = course_witness(state, course)?;
    let observed = outcome
        .assignments()
        .iter()
        .copied()
        .find(|value| value.assignment == reference)
        .ok_or_else(|| destination::integrity("selected copy outcome assignment"))?;
    let applied = AppliedAssignmentImportEvidence::new(
        command.source(),
        command.replacements().clone(),
        semantic_digest,
        observed,
        import_revision,
    );
    state.curriculum_adoption.import_records.insert(
        assignment,
        StoredAssignmentImport {
            receipt_actor: actor,
            receipt_key: command.idempotency_key().clone(),
            import_revision,
        },
    );
    state.curriculum_adoption.assignment_evidence.insert(
        (assignment, import_revision),
        StoredAssignmentAdoptionEvidence {
            receipt_actor: actor,
            receipt_key: command.idempotency_key().clone(),
            source: command.source(),
            detail: AssignmentAdoptionEvidenceDetail::SelectedCopy {
                precondition: command.destination().clone(),
                outcome: outcome.clone(),
                replacements: command.replacements().clone(),
                semantic_digest,
                applied_assignment: observed,
                import_revision,
            },
        },
    );
    Ok(AppliedSelectedCopy {
        course: command.destination().course,
        assignment: reference,
        import_revision,
        outcome,
        applied,
    })
}

/// Rebuilds B2-owned import indexes from exact immutable receipt evidence only.
pub(super) fn reconcile_course_instance_adoption_locked(
    state: &mut State,
    actor: question_model::UserId,
    target: &CourseInstanceReceiptTarget,
) -> Result<ReconcileCourseInstanceAdoptionCompleted, StoreError> {
    let target = resolve_reconciliation_target(state, target)?;
    let course = resolve_course(state, target.destination().course)?;
    require_course_instructor(state, course, actor)?;
    if course_instance_blueprint_application(state, course)? != target.blueprint_application() {
        return Err(StoreError::Conflict);
    }
    let receipt_identity = (target.authorized_actor(), target.idempotency_key().clone());
    let receipt = state
        .curriculum_adoption
        .receipts
        .get(&receipt_identity)
        .ok_or(StoreError::Conflict)?;
    validate_receipt_evidence(state, receipt)?;
    let Some(locator) = target.assignment_import_target() else {
        return Err(StoreError::Conflict);
    };
    let assignment = assignment_id_for_reference(state, course, locator.assignment())?;
    let evidence = state
        .curriculum_adoption
        .assignment_evidence
        .get(&(assignment, locator.import_revision()))
        .ok_or_else(|| destination::integrity("reconciliation assignment evidence"))?;
    if evidence.receipt_actor != locator.receipt_actor()
        || evidence.receipt_key != *locator.receipt_key()
        || evidence.outcome().course != locator.course()
        || evidence.assignment() != locator.assignment()
        || evidence.import_revision() != locator.import_revision()
    {
        return Err(StoreError::Conflict);
    }
    match state.curriculum_adoption.import_records.get(&assignment) {
        None => rebuild_current_projection(
            &mut state.curriculum_adoption,
            assignment,
            locator.import_revision(),
        )?,
        Some(current) if current.import_revision == locator.import_revision() => {}
        Some(current) if current.import_revision.value() > locator.import_revision().value() => {}
        Some(_) => return Err(destination::integrity("reconciliation import projection")),
    }
    Ok(ReconcileCourseInstanceAdoptionCompleted {
        course: target.destination().course,
        replay: question_model::CurriculumReplayStatus::Applied,
    })
}

pub(super) fn controlled_update_eligibility(
    state: &State,
    actor: question_model::UserId,
    destination: &CourseInstanceWitness,
    import: &CourseInstanceImportWitness,
    source: question_model::AssignmentDefinitionSourceView,
) -> Result<CourseInstanceEligibility, StoreError> {
    let course = resolve_course(state, destination.course)?;
    let assignment = assignment_id_for_reference(state, course, import.destination.assignment)?;
    let current_import = state
        .curriculum_adoption
        .import_records
        .get(&assignment)
        .ok_or_else(|| destination::integrity("controlled update import projection"))?;
    let evidence = state
        .curriculum_adoption
        .assignment_evidence
        .get(&(assignment, current_import.import_revision))
        .ok_or_else(|| destination::integrity("controlled update immutable import evidence"))?;
    // ASVS 2.3.1/2.3.3: an update advances exactly one retained assignment lineage.
    let current_source =
        super::super::super::reusable_curriculum::current_assignment_source(state, import.source);
    if current_import.import_revision != import.import_revision
        || evidence.source != import.source
        || evidence.assignment() != import.destination.assignment
        || !source.is_strictly_newer_revision_of(import.source)
        || !matches!(current_source, Ok(current) if current == source)
    {
        return Ok(CourseInstanceEligibility::Refused {
            refusal: CourseInstanceRefusal::SourceRevisionDrift {
                source: source.source(),
            },
        });
    }
    let revision = *state
        .assignment_revisions
        .get(&assignment)
        .ok_or_else(|| destination::integrity("controlled update assignment revision"))?;
    if revision != import.destination.revision {
        return Ok(CourseInstanceEligibility::Refused {
            refusal: CourseInstanceRefusal::Divergent {
                assignment: import.destination.assignment,
            },
        });
    }
    if assignment_has_run(state, assignment) {
        return Ok(CourseInstanceEligibility::Refused {
            refusal: CourseInstanceRefusal::IssuedWork {
                course: destination.course,
            },
        });
    }
    let imported = super::super::super::reusable_curriculum::curriculum_assignment_source_snapshot(
        state,
        import.source,
    )?;
    let original = only_assignment(&imported.payload)?;
    let current =
        destination::current_semantic_assignment(state, assignment, original.schedule().clone())?;
    if &current != original {
        return Ok(CourseInstanceEligibility::Refused {
            refusal: CourseInstanceRefusal::Divergent {
                assignment: import.destination.assignment,
            },
        });
    }
    let proposed = super::super::super::reusable_curriculum::curriculum_assignment_source_snapshot(
        state, source,
    )?;
    if validate_destination_pins(state, &proposed.payload).is_err() {
        return Ok(CourseInstanceEligibility::Refused {
            refusal: CourseInstanceRefusal::UnavailablePin {
                recovery: super::super::pin_correction(state, source, &proposed.payload)?
                    .ok_or(StoreError::Conflict)?,
            },
        });
    }
    Ok(CourseInstanceEligibility::Eligible)
}

pub(super) fn current_import_witness(
    state: &State,
    course: question_model::CourseId,
    assignment_reference: question_model::AssignmentReference,
) -> Result<CourseInstanceImportWitness, StoreError> {
    let assignment = assignment_id_for_reference(state, course, assignment_reference)?;
    let import = state
        .curriculum_adoption
        .import_records
        .get(&assignment)
        .ok_or_else(|| destination::integrity("controlled update import projection"))?;
    let evidence = state
        .curriculum_adoption
        .assignment_evidence
        .get(&(assignment, import.import_revision))
        .ok_or_else(|| destination::integrity("controlled update immutable import evidence"))?;
    let revision = *state
        .assignment_revisions
        .get(&assignment)
        .ok_or_else(|| destination::integrity("controlled update assignment revision"))?;
    Ok(CourseInstanceImportWitness {
        source: evidence.source,
        destination: question_model::ObservedCourseInstanceAssignment {
            assignment: assignment_reference,
            revision,
        },
        import_revision: import.import_revision,
    })
}

fn assignment_id_for_reference(
    state: &State,
    course: question_model::CourseId,
    reference: question_model::AssignmentReference,
) -> Result<question_model::AssignmentId, StoreError> {
    let assignment = *state
        .assignments_by_reference
        .get(&reference)
        .ok_or(StoreError::NotFound)?;
    state
        .assignments
        .get(&assignment)
        .filter(|record| record.course_id == course)
        .map(|_| assignment)
        .ok_or(StoreError::NotFound)
}

fn next_import_revision(
    revision: CurriculumImportRevision,
) -> Result<CurriculumImportRevision, StoreError> {
    revision
        .value()
        .checked_add(1)
        .and_then(CurriculumImportRevision::new)
        .ok_or_else(|| StoreError::Unavailable("curriculum import revision exhausted".into()))
}

fn only_assignment(
    payload: &CurriculumSemanticPayload,
) -> Result<&question_model::curriculum_adoption::CurriculumSemanticAssignment, StoreError> {
    let CurriculumSemanticPayload::Assignment(assignment) = payload else {
        return Err(StoreError::Conflict);
    };
    Ok(assignment)
}

fn semantic_error(error: crate::curriculum_adoption::SemanticPlannerError) -> StoreError {
    StoreError::InvalidRecord(error.to_string())
}
