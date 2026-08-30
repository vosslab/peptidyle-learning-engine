//! Daylight-saving correction remains explicit before CourseInstance creation.

use crate::{
    CreateBlueprintCourseCommand, CurriculumAdoptionStore, ReplaceBlueprintCourseCommand,
    ReusableCurriculumStore,
};
use question_model::{
    BlueprintAdoptionEligibility, BlueprintAdoptionRefusal, BlueprintAssignmentEditHandle,
    BlueprintCourseAssignmentReplacementInput, BlueprintCourseModuleReplacementInput,
    BlueprintModuleEditHandle, CourseInstanceScheduleReason, CourseTerm,
    CreateBlueprintCourseDefinitionInput, CreateBlueprintCourseModuleInput,
    CurriculumAdoptionApplyIntent, CurriculumAdoptionCompleted, CurriculumAdoptionPreview,
    CurriculumAdoptionPreviewRequest, CurriculumPinReplacements,
    InstantiateBlueprintCoursePreviewRequest, LocalTimeOfDay, ObservedBlueprintSource,
    RelativeAssignmentSchedule, RelativeScheduleMoment, ReplaceBlueprintCourseDefinitionInput,
};

use super::super::adoption_inputs::{definition, key};
use super::super::scenario::CurriculumAdoptionScenario;

#[tokio::test]
async fn preview_returns_dst_correction_and_corrected_blueprint_instantiates() {
    let scenario = CurriculumAdoptionScenario::new().await;
    let mut assignment = definition(scenario.source_question.clone());
    assignment.schedule = RelativeAssignmentSchedule {
        available_at: Some(RelativeScheduleMoment {
            day_offset: 0,
            local_time: LocalTimeOfDay::parse("02:30:00.000").expect("gap time"),
        }),
        due_at: None,
        closes_at: None,
    };
    let created = scenario
        .store
        .create_blueprint_course(
            scenario.context,
            scenario.session,
            CreateBlueprintCourseCommand {
                definition: CreateBlueprintCourseDefinitionInput {
                    title: "DST lifecycle source".into(),
                    modules: vec![CreateBlueprintCourseModuleInput {
                        label: "DST module".into(),
                        definitions: vec![assignment.clone()],
                    }],
                },
            },
        )
        .await
        .expect("gap BlueprintCourse");
    let target_term =
        CourseTerm::from_parts("2026-03-08", "2026-11-01", "America/Chicago").expect("DST term");
    let gap_source = ObservedBlueprintSource {
        reference: created.reference,
        revision: created.revision,
    };
    let gap_request = instantiate_request(gap_source, target_term.clone());
    let gap_preview = scenario
        .store
        .preview_curriculum_adoption(scenario.context, scenario.session, gap_request.clone())
        .await
        .expect("gap preview");
    let CurriculumAdoptionPreview::InstantiateBlueprintCourse {
        preview: gap_preview,
    } = gap_preview
    else {
        panic!("gap preview has the exact operation kind");
    };
    assert!(matches!(
        gap_preview.eligibility,
        BlueprintAdoptionEligibility::Refused {
            refusal: BlueprintAdoptionRefusal::ScheduleCorrectionsRequired { ref corrections }
        } if corrections.iter().any(|correction| {
            correction.reason == CourseInstanceScheduleReason::NonexistentLocalTime
        })
    ));
    assert_eq!(
        scenario
            .store
            .apply_curriculum_adoption(
                scenario.context,
                scenario.session,
                CurriculumAdoptionApplyIntent {
                    request: gap_request,
                    idempotency_key: key("dst-gap"),
                },
            )
            .await,
        Err(crate::StoreError::Conflict),
    );

    assignment.schedule.available_at = Some(RelativeScheduleMoment {
        day_offset: 0,
        local_time: LocalTimeOfDay::parse("03:30:00.000").expect("corrected time"),
    });
    let module = &created.modules[0];
    let corrected = scenario
        .store
        .replace_blueprint_course(
            scenario.context,
            scenario.session,
            ReplaceBlueprintCourseCommand {
                reference: created.reference,
                expected_revision: created.revision,
                definition: ReplaceBlueprintCourseDefinitionInput {
                    title: created.title,
                    modules: vec![BlueprintCourseModuleReplacementInput {
                        handle: BlueprintModuleEditHandle::Retained {
                            module_id: module.module_id,
                        },
                        label: module.label.clone(),
                        definitions: vec![BlueprintCourseAssignmentReplacementInput {
                            handle: BlueprintAssignmentEditHandle::Retained {
                                assignment_id: module.definitions[0].assignment_id,
                            },
                            definition: assignment,
                        }],
                    }],
                },
            },
        )
        .await
        .expect("corrected BlueprintCourse revision");
    let corrected_source = ObservedBlueprintSource {
        reference: corrected.reference,
        revision: corrected.revision,
    };
    let corrected_request = instantiate_request(corrected_source, target_term);
    let corrected_preview = scenario
        .store
        .preview_curriculum_adoption(
            scenario.context,
            scenario.session,
            corrected_request.clone(),
        )
        .await
        .expect("corrected preview");
    assert!(matches!(
        corrected_preview,
        CurriculumAdoptionPreview::InstantiateBlueprintCourse {
            preview: question_model::InstantiateBlueprintCoursePreviewView {
                eligibility: BlueprintAdoptionEligibility::Eligible,
                ..
            }
        }
    ));
    assert!(matches!(
        scenario
            .store
            .apply_curriculum_adoption(
                scenario.context,
                scenario.session,
                CurriculumAdoptionApplyIntent {
                    request: corrected_request,
                    idempotency_key: key("dst-corrected"),
                },
            )
            .await
            .expect("corrected apply"),
        CurriculumAdoptionCompleted::InstantiateBlueprintCourse { .. }
    ));
}

fn instantiate_request(
    source: ObservedBlueprintSource,
    target_term: CourseTerm,
) -> CurriculumAdoptionPreviewRequest {
    CurriculumAdoptionPreviewRequest::InstantiateBlueprintCourse {
        request: InstantiateBlueprintCoursePreviewRequest {
            source,
            target_term,
            replacements: CurriculumPinReplacements::default(),
        },
    }
}
