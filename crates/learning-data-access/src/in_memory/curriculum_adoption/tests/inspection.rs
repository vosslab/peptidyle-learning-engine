//! Answer-free CourseInstance Blueprint inspection behavior.

use crate::{CurriculumAdoptionStore, StoreError};

use super::adoption_inputs::key;
use super::scenario::CurriculumAdoptionScenario;

#[tokio::test]
async fn inspection_reports_exact_blueprint_application_and_assignment_provenance() {
    let scenario = CurriculumAdoptionScenario::new().await;

    let inspection = scenario
        .store
        .inspect_course_instance_blueprint_adoption(
            scenario.context,
            scenario.session,
            scenario.course,
        )
        .await
        .expect("inspection succeeds")
        .expect("CourseInstance has Blueprint evidence");

    assert_eq!(
        inspection.initial_blueprint_application.source,
        scenario.blueprint
    );
    assert!(matches!(
        (
            inspection.witness.assignments(),
            inspection.assignments.as_slice()
        ),
        ([_observed], [provenance])
            if provenance.source == scenario.assignment_source()
                && provenance.import_revision.value() == 1
    ));
}

#[tokio::test]
async fn inspection_refuses_missing_immutable_assignment_evidence() {
    let scenario = CurriculumAdoptionScenario::new().await;
    {
        let mut state = scenario.store.write_state().expect("fixture state");
        let assignment_reference = state
            .curriculum_adoption
            .whole_course_adoptions
            .values()
            .find(|adoption| adoption.destination.course == scenario.course)
            .expect("fixture whole-course adoption")
            .destination
            .assignments()[0]
            .assignment;
        let assignment = state.assignments_by_reference[&(scenario.tenant, assignment_reference)];
        let import_revision = state.curriculum_adoption.import_records[&assignment].import_revision;
        state
            .curriculum_adoption
            .assignment_evidence
            .remove(&(assignment, import_revision));
    }

    let result = scenario
        .store
        .inspect_course_instance_blueprint_adoption(
            scenario.context,
            scenario.session,
            scenario.course,
        )
        .await;

    assert!(matches!(result, Err(StoreError::Unavailable(_))));
}

#[tokio::test]
async fn inspection_refuses_missing_completed_receipt() {
    let scenario = CurriculumAdoptionScenario::new().await;
    scenario
        .store
        .write_state()
        .expect("fixture state")
        .curriculum_adoption
        .receipts
        .remove(&(scenario.actor, key("fixture-course")));

    let result = scenario
        .store
        .inspect_course_instance_blueprint_adoption(
            scenario.context,
            scenario.session,
            scenario.course,
        )
        .await;

    assert!(matches!(result, Err(StoreError::Unavailable(_))));
}
