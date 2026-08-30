//! Integrity failures preserve the exact Memory transaction boundary.

use crate::{CurriculumAdoptionStore, StoreError};
use question_model::{
    CourseTerm, CurriculumAdoptionApplyIntent, CurriculumAdoptionPreviewRequest,
    ShiftCourseInstanceTermPreviewRequest,
};

use super::super::adoption_inputs::key;
use super::super::scenario::CurriculumAdoptionScenario;

#[tokio::test]
async fn missing_course_schedule_witness_fails_closed_without_repairing_state() {
    let scenario = CurriculumAdoptionScenario::new().await;
    {
        let mut state = scenario.store.write_state().expect("state");
        let course = state
            .courses
            .keys()
            .find_map(|(_, course)| (*course == scenario.course).then_some(*course))
            .expect("course");
        state
            .course_schedule_revisions
            .remove(&course)
            .expect("course schedule witness");
    }
    let before = lifecycle_state(&scenario);

    assert!(matches!(
        scenario
            .store
            .preview_curriculum_adoption(
                scenario.context,
                scenario.session,
                shift_request(&scenario, spring_term()),
            )
            .await,
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(lifecycle_state(&scenario), before);
}

#[tokio::test]
async fn late_receipt_identity_collision_restores_the_complete_memory_state() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let seed_key = key("rollback-seed");
    scenario
        .store
        .apply_curriculum_adoption(
            scenario.context,
            scenario.session,
            CurriculumAdoptionApplyIntent {
                request: shift_request(&scenario, spring_term()),
                idempotency_key: seed_key.clone(),
            },
        )
        .await
        .expect("seed shift receipt");
    let collision_key = key("rollback-collision");
    {
        let mut state = scenario.store.write_state().expect("state");
        let retained =
            state.curriculum_adoption.receipt_targets[&(scenario.actor, seed_key)].clone();
        state
            .curriculum_adoption
            .receipt_targets
            .insert((scenario.actor, collision_key.clone()), retained);
    }
    let before = debug_state(&scenario);
    let later_term =
        CourseTerm::from_parts("2027-08-23", "2027-12-17", "America/Chicago").expect("later term");

    assert_eq!(
        scenario
            .store
            .apply_curriculum_adoption(
                scenario.context,
                scenario.session,
                CurriculumAdoptionApplyIntent {
                    request: shift_request(&scenario, later_term),
                    idempotency_key: collision_key,
                },
            )
            .await,
        Err(StoreError::Conflict),
    );
    assert_eq!(
        debug_state(&scenario),
        before,
        "a failure after lifecycle mutation restores every private Memory collection",
    );
}

fn shift_request(
    scenario: &CurriculumAdoptionScenario,
    target_term: CourseTerm,
) -> CurriculumAdoptionPreviewRequest {
    CurriculumAdoptionPreviewRequest::ShiftCourseInstanceTerm {
        request: ShiftCourseInstanceTermPreviewRequest {
            course: scenario.course,
            target_term,
        },
    }
}

fn spring_term() -> CourseTerm {
    CourseTerm::from_parts("2027-01-11", "2027-05-08", "America/Chicago").expect("spring term")
}

fn debug_state(scenario: &CurriculumAdoptionScenario) -> String {
    format!(
        "{:#?}",
        scenario
            .store
            .read_state()
            .expect("Memory state is available")
    )
}

fn lifecycle_state(scenario: &CurriculumAdoptionScenario) -> impl PartialEq + std::fmt::Debug {
    let state = scenario
        .store
        .read_state()
        .expect("Memory state is available");
    (
        state.curriculum_adoption.clone(),
        state.courses.clone(),
        state.course_schedule_revisions.clone(),
        state.assignments.clone(),
        state.assignment_revisions.clone(),
        state.assignment_base_policy.clone(),
    )
}
