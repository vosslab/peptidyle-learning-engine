//! Refusal behavior at the current public Memory adoption boundary.

use crate::{CurriculumAdoptionStore, StoreError, TenantContext};
use question_model::{
    ActivityTimestamp, AssignmentEnrollment, AssignmentId, AssignmentRun,
    CourseInstanceEligibility, CourseReference, CourseTerm, CurriculumAdoptionApplyIntent,
    CurriculumAdoptionPreview, CurriculumAdoptionPreviewRequest, EnrollmentId, RunId, RunMode,
    RunReference, ShiftCourseInstanceTermPreviewRequest, StudentId, TenantId, VariationPolicy,
};
use uuid::Uuid;

use super::super::resolve_course;
use super::adoption_inputs::key;
use super::scenario::CurriculumAdoptionScenario;

mod dst;
mod integrity;

#[tokio::test]
async fn same_key_with_changed_intent_conflicts_without_mutation() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let first_request = shift_request(&scenario, spring_term());
    scenario
        .store
        .apply_curriculum_adoption(
            scenario.context,
            scenario.session,
            CurriculumAdoptionApplyIntent {
                request: first_request,
                idempotency_key: key("shift-changed-intent"),
            },
        )
        .await
        .expect("first shift applies");
    let changed_request = shift_request(
        &scenario,
        CourseTerm::from_parts("2027-08-23", "2027-12-17", "America/Chicago")
            .expect("changed term"),
    );
    let before = lifecycle_snapshot(&scenario);

    assert_eq!(
        scenario
            .store
            .apply_curriculum_adoption(
                scenario.context,
                scenario.session,
                CurriculumAdoptionApplyIntent {
                    request: changed_request,
                    idempotency_key: key("shift-changed-intent"),
                },
            )
            .await,
        Err(StoreError::Conflict),
    );
    assert_eq!(lifecycle_snapshot(&scenario), before);
}

#[tokio::test]
async fn foreign_tenant_and_revoked_course_authority_refuse_without_mutation() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let foreign =
        TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(121_099)));
    assert_eq!(
        scenario
            .store
            .preflight_curriculum_adoption(foreign, scenario.session)
            .await,
        Err(StoreError::NotFound),
    );

    let request = shift_request(&scenario, spring_term());
    let preview = scenario
        .store
        .preview_curriculum_adoption(scenario.context, scenario.session, request.clone())
        .await
        .expect("authorized preview");
    assert!(matches!(
        preview,
        CurriculumAdoptionPreview::ShiftCourseInstanceTerm {
            preview: question_model::ShiftCourseInstanceTermPreview {
                eligibility: CourseInstanceEligibility::Eligible,
                ..
            }
        }
    ));
    {
        let mut state = scenario.store.write_state().expect("state");
        let course = resolve_course(&state, scenario.tenant, scenario.course).expect("course");
        state
            .active_course_membership_by_user
            .remove(&(course, scenario.actor))
            .expect("active Instructor membership");
    }
    let before = lifecycle_snapshot(&scenario);

    assert_eq!(
        scenario
            .store
            .apply_curriculum_adoption(
                scenario.context,
                scenario.session,
                CurriculumAdoptionApplyIntent {
                    request,
                    idempotency_key: key("revoked-course-authority"),
                },
            )
            .await,
        Err(StoreError::NotFound),
    );
    assert_eq!(lifecycle_snapshot(&scenario), before);
}

#[tokio::test]
async fn issued_work_between_preview_and_apply_fences_term_shift() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let request = shift_request(&scenario, spring_term());
    let preview = scenario
        .store
        .preview_curriculum_adoption(scenario.context, scenario.session, request.clone())
        .await
        .expect("term-shift preview");
    assert!(matches!(
        preview,
        CurriculumAdoptionPreview::ShiftCourseInstanceTerm {
            preview: question_model::ShiftCourseInstanceTermPreview {
                eligibility: CourseInstanceEligibility::Eligible,
                ..
            }
        }
    ));
    issue_run(&scenario, scenario.course, 121_090);
    let before = lifecycle_snapshot(&scenario);

    assert_eq!(
        scenario
            .store
            .apply_curriculum_adoption(
                scenario.context,
                scenario.session,
                CurriculumAdoptionApplyIntent {
                    request,
                    idempotency_key: key("issued-shift"),
                },
            )
            .await,
        Err(StoreError::Conflict),
    );
    assert_eq!(lifecycle_snapshot(&scenario), before);
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

fn lifecycle_snapshot(scenario: &CurriculumAdoptionScenario) -> impl PartialEq + std::fmt::Debug {
    let state = scenario.store.read_state().expect("state");
    (
        state.curriculum_adoption.clone(),
        state.courses.clone(),
        state.course_schedule_revisions.clone(),
        state.course_memberships.clone(),
        state.assignments.clone(),
        state.assignment_revisions.clone(),
        state.assignment_base_policy.clone(),
        state.enrollments.clone(),
        state.runs.clone(),
    )
}

fn issue_run(scenario: &CurriculumAdoptionScenario, course: CourseReference, number: u128) {
    let assignment = {
        let state = scenario.store.read_state().expect("state");
        let course = resolve_course(&state, scenario.tenant, course).expect("course");
        state
            .assignments
            .iter()
            .find_map(|((tenant, assignment), row)| {
                (*tenant == scenario.tenant && row.course_id == course).then_some(*assignment)
            })
            .expect("course assignment")
    };
    issue_run_for_assignment(scenario, assignment, number);
}

fn issue_run_for_assignment(
    scenario: &CurriculumAdoptionScenario,
    assignment: AssignmentId,
    number: u128,
) {
    let mut state = scenario.store.write_state().expect("state");
    let enrollment = EnrollmentId::from_uuid(Uuid::from_u128(number));
    let run = RunId::from_uuid(Uuid::from_u128(number + 1));
    state.enrollments.insert(
        (scenario.tenant, enrollment),
        AssignmentEnrollment {
            id: enrollment,
            assignment,
            user: scenario.actor,
            student: StudentId::from_uuid(Uuid::from_u128(number + 2)),
            first_completed_at: None,
            current_grade_run: None,
            best_grade_run: None,
        },
    );
    state.runs.insert(
        (scenario.tenant, run),
        AssignmentRun {
            id: run,
            reference: RunReference::new(1).expect("run reference"),
            enrollment,
            run_number: 1,
            started_at: ActivityTimestamp::from_unix_millis(0),
            completed_at: None,
            score: None,
            mode: RunMode::Assigned,
            variation: VariationPolicy::NewSeeds,
        },
    );
}
