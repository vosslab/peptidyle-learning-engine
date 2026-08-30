//! Exact, derived-projection-only CourseInstance reconciliation behavior.

use crate::{
    CurriculumAdoptionStore, ReplaceBlueprintCourseCommand, ReusableCurriculumStore, StoreError,
};
use question_model::{
    AssignmentDefinitionSourceView, AssignmentReference, BlueprintAssignmentEditHandle,
    BlueprintCourseAssignmentReplacementInput, BlueprintCourseModuleReplacementInput,
    BlueprintModuleEditHandle, ControlledUpdateBlueprintAssignmentPreviewRequest,
    CourseInstanceReceiptTarget, CreateSelectedBlueprintAssignmentPreviewRequest,
    CurriculumAdoptionApplyIntent, CurriculumAdoptionCompleted, CurriculumAdoptionPreviewRequest,
    CurriculumPinReplacements, CurriculumReplayStatus, ObservedBlueprintSource,
    ReconcileCourseInstanceAdoptionIntent, ReplaceBlueprintCourseDefinitionInput,
};

use super::adoption_inputs::{definition, key};
use super::scenario::CurriculumAdoptionScenario;

async fn create_selected_copy(
    scenario: &CurriculumAdoptionScenario,
    key_suffix: &str,
) -> (AssignmentReference, CourseInstanceReceiptTarget) {
    let operation_key = key(key_suffix);
    let request = CurriculumAdoptionPreviewRequest::CreateSelectedBlueprintAssignment {
        request: CreateSelectedBlueprintAssignmentPreviewRequest {
            course: scenario.course,
            source: scenario.assignment_source(),
            replacements: CurriculumPinReplacements::default(),
        },
    };
    scenario
        .store
        .preview_curriculum_adoption(scenario.context, scenario.session, request.clone())
        .await
        .expect("selected-copy preview");
    let completed = scenario
        .store
        .apply_curriculum_adoption(
            scenario.context,
            scenario.session,
            CurriculumAdoptionApplyIntent {
                request,
                idempotency_key: operation_key.clone(),
            },
        )
        .await
        .expect("selected-copy apply");
    let CurriculumAdoptionCompleted::CreateSelectedBlueprintAssignment { completed } = completed
    else {
        panic!("selected-copy operation must return its completion variant");
    };
    let target = scenario
        .store
        .read_state()
        .expect("fixture state")
        .curriculum_adoption
        .receipt_targets[&(scenario.actor, operation_key)]
        .clone();
    (completed.assignment, target)
}

async fn update_selected_copy(
    scenario: &CurriculumAdoptionScenario,
    assignment: AssignmentReference,
    key_suffix: &str,
) -> CourseInstanceReceiptTarget {
    let mut revised_definition = definition(scenario.source_question.clone());
    revised_definition.title = "Revised protein structure practice".into();
    let revised = scenario
        .store
        .replace_blueprint_course(
            scenario.context,
            scenario.session,
            ReplaceBlueprintCourseCommand {
                reference: scenario.blueprint.reference,
                expected_revision: scenario.blueprint.revision,
                definition: ReplaceBlueprintCourseDefinitionInput {
                    title: "Current curriculum source".into(),
                    modules: vec![BlueprintCourseModuleReplacementInput {
                        handle: BlueprintModuleEditHandle::Retained {
                            module_id: scenario.blueprint_module,
                        },
                        label: "Exact module".into(),
                        definitions: vec![BlueprintCourseAssignmentReplacementInput {
                            handle: BlueprintAssignmentEditHandle::Retained {
                                assignment_id: scenario.blueprint_assignment,
                            },
                            definition: revised_definition,
                        }],
                    }],
                },
            },
        )
        .await
        .expect("BlueprintCourse revision");
    let source = AssignmentDefinitionSourceView::new(
        ObservedBlueprintSource {
            reference: revised.reference,
            revision: revised.revision,
        },
        scenario.blueprint_assignment,
    );
    let operation_key = key(key_suffix);
    let request = CurriculumAdoptionPreviewRequest::ControlledUpdateBlueprintAssignment {
        request: ControlledUpdateBlueprintAssignmentPreviewRequest {
            course: scenario.course,
            source,
            assignment,
        },
    };
    scenario
        .store
        .preview_curriculum_adoption(scenario.context, scenario.session, request.clone())
        .await
        .expect("controlled-update preview");
    let completed = scenario
        .store
        .apply_curriculum_adoption(
            scenario.context,
            scenario.session,
            CurriculumAdoptionApplyIntent {
                request,
                idempotency_key: operation_key.clone(),
            },
        )
        .await
        .expect("controlled-update apply");
    assert!(matches!(
        completed,
        CurriculumAdoptionCompleted::ControlledUpdateBlueprintAssignment { .. }
    ));
    scenario
        .store
        .read_state()
        .expect("fixture state")
        .curriculum_adoption
        .receipt_targets[&(scenario.actor, operation_key)]
        .clone()
}

fn assignment_id(
    scenario: &CurriculumAdoptionScenario,
    assignment_reference: AssignmentReference,
) -> question_model::AssignmentId {
    scenario
        .store
        .read_state()
        .expect("fixture state")
        .assignments_by_reference
        .iter()
        .find_map(|((_, stored_reference), assignment)| {
            (*stored_reference == assignment_reference).then_some(*assignment)
        })
        .expect("selected-copy assignment reference")
}

#[tokio::test]
async fn already_consistent_reconciliation_is_audited_without_changing_the_projection() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let (assignment_reference, target) =
        create_selected_copy(&scenario, "reconcile-consistent-source").await;
    let assignment = assignment_id(&scenario, assignment_reference);
    let before = scenario
        .store
        .read_state()
        .expect("fixture state")
        .curriculum_adoption
        .import_records[&assignment]
        .clone();
    let intent = ReconcileCourseInstanceAdoptionIntent {
        target,
        idempotency_key: key("reconcile-consistent-action"),
    };

    let first = scenario
        .store
        .reconcile_course_instance_adoption(scenario.context, scenario.session, intent.clone())
        .await
        .expect("consistent reconciliation");
    let second = scenario
        .store
        .reconcile_course_instance_adoption(scenario.context, scenario.session, intent)
        .await
        .expect("exact reconciliation replay");

    assert_eq!(
        scenario
            .store
            .read_state()
            .expect("fixture state")
            .curriculum_adoption
            .import_records[&assignment],
        before
    );
    assert!(
        first.replay == CurriculumReplayStatus::Applied
            && second.replay == CurriculumReplayStatus::Replayed
    );
}

#[tokio::test]
async fn reconciliation_restores_the_missing_exact_derived_projection() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let (assignment_reference, target) =
        create_selected_copy(&scenario, "reconcile-restore-source").await;
    let assignment = assignment_id(&scenario, assignment_reference);
    let expected = scenario
        .store
        .write_state()
        .expect("fixture state")
        .curriculum_adoption
        .import_records
        .remove(&assignment)
        .expect("derived import projection");

    scenario
        .store
        .reconcile_course_instance_adoption(
            scenario.context,
            scenario.session,
            ReconcileCourseInstanceAdoptionIntent {
                target,
                idempotency_key: key("reconcile-restore-action"),
            },
        )
        .await
        .expect("missing projection repair");

    assert_eq!(
        scenario
            .store
            .read_state()
            .expect("fixture state")
            .curriculum_adoption
            .import_records[&assignment],
        expected
    );
}

#[tokio::test]
async fn reconciliation_preserves_a_newer_superseding_projection() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let (assignment_reference, original_target) =
        create_selected_copy(&scenario, "reconcile-superseded-source").await;
    update_selected_copy(
        &scenario,
        assignment_reference,
        "reconcile-superseding-update",
    )
    .await;
    let assignment = assignment_id(&scenario, assignment_reference);
    let newer = scenario
        .store
        .read_state()
        .expect("fixture state")
        .curriculum_adoption
        .import_records[&assignment]
        .clone();

    scenario
        .store
        .reconcile_course_instance_adoption(
            scenario.context,
            scenario.session,
            ReconcileCourseInstanceAdoptionIntent {
                target: original_target,
                idempotency_key: key("reconcile-superseded-action"),
            },
        )
        .await
        .expect("superseded receipt reconciliation");

    assert_eq!(
        scenario
            .store
            .read_state()
            .expect("fixture state")
            .curriculum_adoption
            .import_records[&assignment],
        newer
    );
}

#[tokio::test]
async fn reconciliation_refuses_a_contradictory_older_projection_atomically() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let (assignment_reference, original_target) =
        create_selected_copy(&scenario, "reconcile-older-source").await;
    let updated_target =
        update_selected_copy(&scenario, assignment_reference, "reconcile-older-update").await;
    let assignment = assignment_id(&scenario, assignment_reference);
    let original_revision = original_target
        .assignment_import_target()
        .expect("selected-copy import target")
        .import_revision();
    let older = {
        let mut state = scenario.store.write_state().expect("fixture state");
        let mut older = state.curriculum_adoption.import_records[&assignment].clone();
        older.import_revision = original_revision;
        state
            .curriculum_adoption
            .import_records
            .insert(assignment, older.clone());
        older
    };

    let result = scenario
        .store
        .reconcile_course_instance_adoption(
            scenario.context,
            scenario.session,
            ReconcileCourseInstanceAdoptionIntent {
                target: updated_target,
                idempotency_key: key("reconcile-older-action"),
            },
        )
        .await;

    assert!(matches!(result, Err(StoreError::Unavailable(_))));
    assert_eq!(
        scenario
            .store
            .read_state()
            .expect("fixture state")
            .curriculum_adoption
            .import_records[&assignment],
        older
    );
}
