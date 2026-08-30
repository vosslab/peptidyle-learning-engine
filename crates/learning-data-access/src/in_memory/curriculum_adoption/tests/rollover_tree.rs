//! Current CourseInstance rollover and term-shift behavior.

use crate::{CreateBlueprintCourseCommand, CurriculumAdoptionStore, ReusableCurriculumStore};
use question_model::{
    AssignmentDefinitionSourceView, BlueprintAdoptionEligibility,
    CourseInstanceBlueprintApplication, CourseInstanceEligibility, CourseInstanceReceiptTarget,
    CourseReference, CourseTerm, CreateBlueprintCourseDefinitionInput,
    CreateBlueprintCourseModuleInput, CurriculumAdoptionApplyIntent, CurriculumAdoptionCompleted,
    CurriculumAdoptionPreview, CurriculumAdoptionPreviewRequest, CurriculumPinReplacements,
    InstantiateBlueprintCoursePreviewRequest, ObservedBlueprintSource,
    RolloverCourseInstancePreviewRequest, ShiftCourseInstanceTermPreviewRequest,
};

use super::super::{course_witness, resolve_course};
use super::adoption_inputs::{definition, key};
use super::scenario::CurriculumAdoptionScenario;

struct TwoModuleCourse {
    course: CourseReference,
    source: ObservedBlueprintSource,
    assignments: Vec<AssignmentDefinitionSourceView>,
}

#[tokio::test]
async fn rollover_retains_ordered_source_parent_and_target_schedule_evidence() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let source = two_module_course(&scenario, "rollover-source").await;
    let target_term = spring_term();
    let request = CurriculumAdoptionPreviewRequest::RolloverCourseInstance {
        request: RolloverCourseInstancePreviewRequest {
            source_course: source.course,
            target_term: target_term.clone(),
        },
    };
    let preview = scenario
        .store
        .preview_curriculum_adoption(scenario.context, scenario.session, request.clone())
        .await
        .expect("rollover preview");
    let CurriculumAdoptionPreview::RolloverCourseInstance { preview } = preview else {
        panic!("rollover preview has the exact operation kind");
    };
    assert_eq!(preview.eligibility, CourseInstanceEligibility::Eligible);
    let preview_manifest = preview.manifest;

    let receipt_key = key("rollover-two-module");
    let completed = apply(&scenario, request, receipt_key.clone()).await;
    let CurriculumAdoptionCompleted::RolloverCourseInstance { completed } = completed else {
        panic!("rollover operation returned another completion variant");
    };
    let state = scenario.store.read_state().expect("state");
    let destination_id = resolve_course(&state, scenario.tenant, completed.course)
        .expect("rollover destination course");
    let current = course_witness(&state, scenario.tenant, destination_id)
        .expect("rollover destination witness");
    let CourseInstanceReceiptTarget::Rollover(receipt) =
        &state.curriculum_adoption.receipt_targets[&(scenario.actor, receipt_key)]
    else {
        panic!("rollover receipt target has the exact operation kind");
    };
    let application = CourseInstanceBlueprintApplication {
        source: source.source,
    };

    assert_eq!(
        (
            receipt.source_course_instance().course,
            receipt.source_blueprint_application(),
            receipt.created_blueprint_application(),
            receipt.created_course_instance(),
            receipt.target_term(),
            receipt.manifest(),
            receipt.manifest().copied.assignments(),
        ),
        (
            source.course,
            application,
            application,
            &current,
            &target_term,
            &preview_manifest,
            source.assignments.as_slice(),
        ),
        "the receipt retains the exact ordered Blueprint meaning and resolved target schedule",
    );
}

#[tokio::test]
async fn term_shift_advances_every_assignment_and_the_course_schedule_witness() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let source = two_module_course(&scenario, "shift-source").await;
    let before = witness(&scenario, source.course);
    let target_term = spring_term();
    let request = CurriculumAdoptionPreviewRequest::ShiftCourseInstanceTerm {
        request: ShiftCourseInstanceTermPreviewRequest {
            course: source.course,
            target_term: target_term.clone(),
        },
    };
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

    let receipt_key = key("shift-two-module");
    apply(&scenario, request, receipt_key.clone()).await;
    let after = witness(&scenario, source.course);
    let state = scenario.store.read_state().expect("state");
    let CourseInstanceReceiptTarget::ShiftTerm(receipt) =
        &state.curriculum_adoption.receipt_targets[&(scenario.actor, receipt_key)]
    else {
        panic!("term-shift receipt target has the exact operation kind");
    };

    assert!(
        before.schedule_revision < after.schedule_revision
            && receipt.binding().precondition() == &before
            && receipt.binding().outcome() == &after
            && receipt.target_term() == &target_term
            && receipt.schedules().len() == before.assignments().len()
            && before
                .assignments()
                .iter()
                .zip(after.assignments())
                .all(|(old, new)| {
                    old.assignment == new.assignment && old.revision < new.revision
                }),
        "one shift advances every assignment and the enclosing course schedule witness",
    );
}

async fn two_module_course(
    scenario: &CurriculumAdoptionScenario,
    key_suffix: &str,
) -> TwoModuleCourse {
    let created = scenario
        .store
        .create_blueprint_course(
            scenario.context,
            scenario.session,
            CreateBlueprintCourseCommand {
                definition: CreateBlueprintCourseDefinitionInput {
                    title: "Two-module lifecycle source".into(),
                    modules: vec![
                        CreateBlueprintCourseModuleInput {
                            label: "Protein structure".into(),
                            definitions: vec![definition(scenario.source_question.clone())],
                        },
                        CreateBlueprintCourseModuleInput {
                            label: "Molecular recognition".into(),
                            definitions: vec![definition(scenario.replacement_question.clone())],
                        },
                    ],
                },
            },
        )
        .await
        .expect("two-module BlueprintCourse");
    let source = ObservedBlueprintSource {
        reference: created.reference,
        revision: created.revision,
    };
    let assignments = created
        .modules
        .iter()
        .flat_map(|module| module.definitions.iter())
        .map(|assignment| AssignmentDefinitionSourceView::new(source, assignment.assignment_id))
        .collect::<Vec<_>>();
    let request = CurriculumAdoptionPreviewRequest::InstantiateBlueprintCourse {
        request: InstantiateBlueprintCoursePreviewRequest {
            source,
            target_term: scenario.term.clone(),
            replacements: CurriculumPinReplacements::default(),
        },
    };
    let preview = scenario
        .store
        .preview_curriculum_adoption(scenario.context, scenario.session, request.clone())
        .await
        .expect("two-module instantiation preview");
    let CurriculumAdoptionPreview::InstantiateBlueprintCourse { preview } = preview else {
        panic!("instantiation preview has the exact operation kind");
    };
    assert_eq!(preview.eligibility, BlueprintAdoptionEligibility::Eligible);
    let completed = apply(scenario, request, key(key_suffix)).await;
    let CurriculumAdoptionCompleted::InstantiateBlueprintCourse { completed } = completed else {
        panic!("instantiation returned another completion variant");
    };
    TwoModuleCourse {
        course: completed.course,
        source,
        assignments,
    }
}

async fn apply(
    scenario: &CurriculumAdoptionScenario,
    request: CurriculumAdoptionPreviewRequest,
    idempotency_key: question_model::CurriculumAdoptionIdempotencyKey,
) -> CurriculumAdoptionCompleted {
    scenario
        .store
        .apply_curriculum_adoption(
            scenario.context,
            scenario.session,
            CurriculumAdoptionApplyIntent {
                request,
                idempotency_key,
            },
        )
        .await
        .expect("curriculum operation applies")
}

fn witness(
    scenario: &CurriculumAdoptionScenario,
    course: CourseReference,
) -> question_model::CourseInstanceWitness {
    let state = scenario.store.read_state().expect("state");
    let course = resolve_course(&state, scenario.tenant, course).expect("course");
    course_witness(&state, scenario.tenant, course).expect("course witness")
}

fn spring_term() -> CourseTerm {
    CourseTerm::from_parts("2027-01-11", "2027-05-08", "America/Chicago").expect("spring term")
}
