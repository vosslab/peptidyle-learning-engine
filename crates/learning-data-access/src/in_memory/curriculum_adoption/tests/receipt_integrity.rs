//! Immutable receipt and reconciliation-target integrity behavior.

use crate::{CurriculumAdoptionStore, StoreError};
use question_model::{
    AssignmentReference, CourseInstanceReceiptTarget,
    CreateSelectedBlueprintAssignmentPreviewRequest, CurriculumAdoptionApplyIntent,
    CurriculumAdoptionCompleted, CurriculumAdoptionPreviewRequest, CurriculumPinReplacements,
    ReconcileCourseInstanceAdoptionIntent,
};

use super::adoption_inputs::key;
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

#[tokio::test]
async fn reconciliation_refuses_a_target_whose_completed_receipt_is_missing() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let (_assignment, target) = create_selected_copy(&scenario, "integrity-missing").await;
    scenario
        .store
        .write_state()
        .expect("fixture state")
        .curriculum_adoption
        .receipts
        .remove(&(scenario.actor, key("integrity-missing")));

    let result = scenario
        .store
        .reconcile_course_instance_adoption(
            scenario.context,
            scenario.session,
            ReconcileCourseInstanceAdoptionIntent {
                target,
                idempotency_key: key("repair-missing-receipt"),
            },
        )
        .await;

    assert!(matches!(result, Err(StoreError::Conflict)));
}

#[tokio::test]
async fn reconciliation_refuses_an_unrelated_target_substituted_for_the_receipt_identity() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let (_first_assignment, first_target) =
        create_selected_copy(&scenario, "integrity-first").await;
    let (_second_assignment, second_target) =
        create_selected_copy(&scenario, "integrity-second").await;
    scenario
        .store
        .write_state()
        .expect("fixture state")
        .curriculum_adoption
        .receipt_targets
        .insert((scenario.actor, key("integrity-first")), second_target);

    let result = scenario
        .store
        .reconcile_course_instance_adoption(
            scenario.context,
            scenario.session,
            ReconcileCourseInstanceAdoptionIntent {
                target: first_target,
                idempotency_key: key("repair-unrelated-target"),
            },
        )
        .await;

    assert!(matches!(result, Err(StoreError::Conflict)));
}

#[tokio::test]
async fn reconciliation_refuses_assignment_evidence_bound_to_another_receipt() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let (assignment_reference, target) =
        create_selected_copy(&scenario, "integrity-evidence").await;
    create_selected_copy(&scenario, "integrity-other").await;
    {
        let mut state = scenario.store.write_state().expect("fixture state");
        let assignment = state
            .assignments_by_reference
            .iter()
            .find_map(|((_, stored_reference), assignment)| {
                (*stored_reference == assignment_reference).then_some(*assignment)
            })
            .expect("fixture assignment reference");
        let revision = state.curriculum_adoption.import_records[&assignment].import_revision;
        state
            .curriculum_adoption
            .assignment_evidence
            .get_mut(&(assignment, revision))
            .expect("immutable assignment evidence")
            .receipt_key = key("integrity-other");
    }

    let result = scenario
        .store
        .reconcile_course_instance_adoption(
            scenario.context,
            scenario.session,
            ReconcileCourseInstanceAdoptionIntent {
                target,
                idempotency_key: key("repair-contradictory-evidence"),
            },
        )
        .await;

    assert!(matches!(result, Err(StoreError::Conflict)));
}
