//! Current public Store behavior for BlueprintCourse source operations.

use crate::{
    CurriculumAdoptionStore, ReplaceBlueprintCourseCommand, ReusableCurriculumStore, StoreError,
};
use question_model::{
    AdoptBlueprintAssignmentPreviewRequest, AssignmentDefinitionSourceView,
    BlueprintAssignmentEditHandle, BlueprintCourseAssignmentReplacementInput,
    BlueprintCourseModuleReplacementInput, BlueprintModuleEditHandle,
    ControlledUpdateBlueprintAssignmentPreviewRequest, ControlledUpdateEffect,
    CourseInstanceEligibility, CourseInstanceReceiptTarget,
    CreateSelectedBlueprintAssignmentPreviewRequest, CurriculumAdoptionApplyIntent,
    CurriculumAdoptionCompleted, CurriculumAdoptionPreview, CurriculumAdoptionPreviewRequest,
    CurriculumPinPosition, CurriculumPinReplacement, CurriculumPinReplacements,
    ForkBlueprintCoursePreviewRequest, ObservedBlueprintSource,
    ReplaceBlueprintCourseDefinitionInput,
};

use super::adoption_inputs::{definition, key};
use super::scenario::CurriculumAdoptionScenario;

#[tokio::test]
async fn fork_materializes_the_exact_blueprint_meaning() {
    let fixture = CurriculumAdoptionScenario::new().await;
    let request = CurriculumAdoptionPreviewRequest::ForkBlueprintCourse {
        request: ForkBlueprintCoursePreviewRequest {
            source: fixture.blueprint,
            replacements: CurriculumPinReplacements::default(),
        },
    };
    let preview = fixture
        .store
        .preview_curriculum_adoption(fixture.context, fixture.session, request.clone())
        .await
        .expect("fork preview");
    assert!(matches!(
        preview,
        CurriculumAdoptionPreview::ForkBlueprintCourse { .. }
    ));
    let completed = fixture
        .store
        .apply_curriculum_adoption(
            fixture.context,
            fixture.session,
            CurriculumAdoptionApplyIntent {
                request,
                idempotency_key: key("fork-current-blueprint"),
            },
        )
        .await
        .expect("fork apply");
    let CurriculumAdoptionCompleted::ForkBlueprintCourse { completed } = completed else {
        panic!("fork request must create a BlueprintCourse");
    };
    let fork = fixture
        .store
        .get_blueprint_course(fixture.context, fixture.session, completed.blueprint)
        .await
        .expect("fork lookup")
        .expect("forked BlueprintCourse exists");
    assert_eq!(
        fork.modules[0].definitions[0].definition.title,
        "Protein structure practice"
    );
}

#[tokio::test]
async fn adopt_materializes_the_exact_selected_blueprint_assignment() {
    let fixture = CurriculumAdoptionScenario::new().await;
    let source = fixture.assignment_source();
    let request = CurriculumAdoptionPreviewRequest::AdoptBlueprintAssignment {
        request: AdoptBlueprintAssignmentPreviewRequest {
            source,
            course: fixture.course,
            replacements: CurriculumPinReplacements::default(),
        },
    };
    let preview = fixture
        .store
        .preview_curriculum_adoption(fixture.context, fixture.session, request.clone())
        .await
        .expect("adoption preview");
    assert!(matches!(
        preview,
        CurriculumAdoptionPreview::AdoptBlueprintAssignment { .. }
    ));
    let completed = fixture
        .store
        .apply_curriculum_adoption(
            fixture.context,
            fixture.session,
            CurriculumAdoptionApplyIntent {
                request,
                idempotency_key: key("adopt-current-blueprint-assignment"),
            },
        )
        .await
        .expect("adoption apply");
    let CurriculumAdoptionCompleted::AdoptBlueprintAssignment { completed } = completed else {
        panic!("adoption request must create an assignment");
    };
    let inspection = fixture
        .store
        .inspect_course_instance_blueprint_adoption(
            fixture.context,
            fixture.session,
            completed.course,
        )
        .await
        .expect("adoption inspection")
        .expect("adopted CourseInstance inspection");
    assert_eq!(
        inspection
            .assignments
            .last()
            .expect("adopted source")
            .source,
        source
    );
}

#[tokio::test]
async fn instantiation_materializes_parent_application_and_replays_exact_intent() {
    let fixture = CurriculumAdoptionScenario::new().await;
    let request = fixture.instantiate_request();
    let preview = fixture
        .store
        .preview_curriculum_adoption(fixture.context, fixture.session, request.clone())
        .await
        .expect("instantiation preview");
    assert!(matches!(
        preview,
        CurriculumAdoptionPreview::InstantiateBlueprintCourse { .. }
    ));
    let intent = CurriculumAdoptionApplyIntent {
        request,
        idempotency_key: key("instantiate-current-blueprint"),
    };
    let first = fixture
        .store
        .apply_curriculum_adoption(fixture.context, fixture.session, intent.clone())
        .await
        .expect("instantiation apply");
    let replay = fixture
        .store
        .apply_curriculum_adoption(fixture.context, fixture.session, intent)
        .await
        .expect("instantiation replay");
    let (
        CurriculumAdoptionCompleted::InstantiateBlueprintCourse { completed: first },
        CurriculumAdoptionCompleted::InstantiateBlueprintCourse { completed: replay },
    ) = (first, replay)
    else {
        panic!("instantiation must keep its completion shape");
    };
    let inspection = fixture
        .store
        .inspect_course_instance_blueprint_adoption(fixture.context, fixture.session, first.course)
        .await
        .expect("instantiation inspection")
        .expect("instantiated CourseInstance inspection");
    assert_eq!(
        inspection.initial_blueprint_application.source,
        fixture.blueprint
    );
    assert_eq!(replay.course, first.course);
    assert_eq!(
        replay.replay,
        question_model::CurriculumReplayStatus::Replayed
    );
}

#[tokio::test]
async fn same_actor_key_with_changed_source_operation_conflicts() {
    let fixture = CurriculumAdoptionScenario::new().await;
    let key = key("source-operation-conflict");
    let fork = CurriculumAdoptionPreviewRequest::ForkBlueprintCourse {
        request: ForkBlueprintCoursePreviewRequest {
            source: fixture.blueprint,
            replacements: CurriculumPinReplacements::default(),
        },
    };
    fixture
        .store
        .preview_curriculum_adoption(fixture.context, fixture.session, fork.clone())
        .await
        .expect("first source-operation preview");
    fixture
        .store
        .apply_curriculum_adoption(
            fixture.context,
            fixture.session,
            CurriculumAdoptionApplyIntent {
                request: fork,
                idempotency_key: key.clone(),
            },
        )
        .await
        .expect("first source operation");
    let changed = fixture.instantiate_request();
    fixture
        .store
        .preview_curriculum_adoption(fixture.context, fixture.session, changed.clone())
        .await
        .expect("changed source-operation preview");
    let result = fixture
        .store
        .apply_curriculum_adoption(
            fixture.context,
            fixture.session,
            CurriculumAdoptionApplyIntent {
                request: changed,
                idempotency_key: key,
            },
        )
        .await;
    assert!(matches!(result, Err(StoreError::Conflict)));
}

#[tokio::test]
async fn controlled_update_records_source_revision_only_when_assignment_meaning_is_unchanged() {
    let fixture = CurriculumAdoptionScenario::new().await;
    let source = replace_source(
        &fixture,
        "Renamed BlueprintCourse",
        "Protein structure practice",
    )
    .await;
    let completed = apply_controlled_update(&fixture, source, "controlled-source-only").await;
    let CurriculumAdoptionCompleted::ControlledUpdateBlueprintAssignment { completed } = completed
    else {
        panic!("controlled update must keep its completion shape");
    };
    let target = receipt_target(&fixture, "controlled-source-only");
    let CourseInstanceReceiptTarget::ControlledUpdate(receipt) = target else {
        panic!("controlled update must retain its exact receipt target");
    };
    assert_eq!(
        completed.assignment,
        receipt.applied().assignment().assignment
    );
    assert_eq!(receipt.effect(), ControlledUpdateEffect::SourceRevisionOnly);
}

#[tokio::test]
async fn controlled_update_records_meaning_change_when_assignment_meaning_changes() {
    let fixture = CurriculumAdoptionScenario::new().await;
    let source = replace_source(
        &fixture,
        "Updated BlueprintCourse",
        "Revised protein structure",
    )
    .await;
    let completed = apply_controlled_update(&fixture, source, "controlled-meaning-change").await;
    let CurriculumAdoptionCompleted::ControlledUpdateBlueprintAssignment { completed } = completed
    else {
        panic!("controlled update must keep its completion shape");
    };
    let target = receipt_target(&fixture, "controlled-meaning-change");
    let CourseInstanceReceiptTarget::ControlledUpdate(receipt) = target else {
        panic!("controlled update must retain its exact receipt target");
    };
    assert_eq!(
        completed.assignment,
        receipt.applied().assignment().assignment
    );
    assert_eq!(receipt.effect(), ControlledUpdateEffect::MeaningChanged);
}

#[tokio::test]
async fn selected_copy_materializes_the_explicit_replacement_pin() {
    let fixture = CurriculumAdoptionScenario::new().await;
    let replacements = CurriculumPinReplacements::new(vec![CurriculumPinReplacement {
        position: CurriculumPinPosition::new(None, 0, 0, None).expect("fixed-item position"),
        question: fixture.replacement_question.clone(),
    }])
    .expect("replacement pin");
    let request = CurriculumAdoptionPreviewRequest::CreateSelectedBlueprintAssignment {
        request: CreateSelectedBlueprintAssignmentPreviewRequest {
            course: fixture.course,
            source: fixture.assignment_source(),
            replacements: replacements.clone(),
        },
    };
    let preview = fixture
        .store
        .preview_curriculum_adoption(fixture.context, fixture.session, request.clone())
        .await
        .expect("selected-copy preview");
    assert!(matches!(
        preview,
        CurriculumAdoptionPreview::CreateSelectedBlueprintAssignment { ref preview }
            if preview.eligibility == CourseInstanceEligibility::Eligible
    ));
    let completed = fixture
        .store
        .apply_curriculum_adoption(
            fixture.context,
            fixture.session,
            CurriculumAdoptionApplyIntent {
                request,
                idempotency_key: key("selected-copy-replacement"),
            },
        )
        .await
        .expect("selected-copy apply");
    let CurriculumAdoptionCompleted::CreateSelectedBlueprintAssignment { completed } = completed
    else {
        panic!("selected copy must keep its completion shape");
    };
    let target = receipt_target(&fixture, "selected-copy-replacement");
    let CourseInstanceReceiptTarget::SelectedCopy(receipt) = target else {
        panic!("selected copy must retain its exact receipt target");
    };
    assert_eq!(receipt.applied().replacements(), &replacements);
    let state = fixture.store.read_state().expect("state");
    let assignment = state
        .assignments_by_reference
        .iter()
        .find_map(|((_, reference), assignment)| {
            (*reference == completed.assignment).then_some(*assignment)
        })
        .expect("selected-copy assignment");
    let replacement = state
        .published
        .values()
        .find(|record| record.question_id == fixture.replacement_question)
        .expect("replacement publication");
    assert!(
        state
            .assignments
            .iter()
            .find_map(|((_, stored_assignment), record)| {
                (*stored_assignment == assignment).then_some(record)
            })
            .expect("selected-copy assignment record")
            .items
            .iter()
            .any(|item| {
                item.reference.problem == replacement.problem
                    && item.reference.version == replacement.version
            })
    );
}

async fn replace_source(
    fixture: &CurriculumAdoptionScenario,
    blueprint_title: &str,
    assignment_title: &str,
) -> AssignmentDefinitionSourceView {
    let mut assignment = definition(fixture.source_question.clone());
    assignment.title = assignment_title.into();
    let revised = fixture
        .store
        .replace_blueprint_course(
            fixture.context,
            fixture.session,
            ReplaceBlueprintCourseCommand {
                reference: fixture.blueprint.reference,
                expected_revision: fixture.blueprint.revision,
                definition: ReplaceBlueprintCourseDefinitionInput {
                    title: blueprint_title.into(),
                    modules: vec![BlueprintCourseModuleReplacementInput {
                        handle: BlueprintModuleEditHandle::Retained {
                            module_id: fixture.blueprint_module,
                        },
                        label: "Exact module".into(),
                        definitions: vec![BlueprintCourseAssignmentReplacementInput {
                            handle: BlueprintAssignmentEditHandle::Retained {
                                assignment_id: fixture.blueprint_assignment,
                            },
                            definition: assignment,
                        }],
                    }],
                },
            },
        )
        .await
        .expect("source revision");
    AssignmentDefinitionSourceView::new(
        ObservedBlueprintSource {
            reference: revised.reference,
            revision: revised.revision,
        },
        fixture.blueprint_assignment,
    )
}

async fn apply_controlled_update(
    fixture: &CurriculumAdoptionScenario,
    source: AssignmentDefinitionSourceView,
    idempotency_key: &str,
) -> CurriculumAdoptionCompleted {
    let inspection = fixture
        .store
        .inspect_course_instance_blueprint_adoption(
            fixture.context,
            fixture.session,
            fixture.course,
        )
        .await
        .expect("update inspection")
        .expect("bound CourseInstance inspection");
    let assignment = inspection.witness.assignments()[0].assignment;
    let request = CurriculumAdoptionPreviewRequest::ControlledUpdateBlueprintAssignment {
        request: ControlledUpdateBlueprintAssignmentPreviewRequest {
            course: fixture.course,
            source,
            assignment,
        },
    };
    let preview = fixture
        .store
        .preview_curriculum_adoption(fixture.context, fixture.session, request.clone())
        .await
        .expect("controlled-update preview");
    assert!(matches!(
        preview,
        CurriculumAdoptionPreview::ControlledUpdateBlueprintAssignment { ref preview }
            if preview.eligibility == CourseInstanceEligibility::Eligible
    ));
    fixture
        .store
        .apply_curriculum_adoption(
            fixture.context,
            fixture.session,
            CurriculumAdoptionApplyIntent {
                request,
                idempotency_key: key(idempotency_key),
            },
        )
        .await
        .expect("controlled-update apply")
}

fn receipt_target(
    fixture: &CurriculumAdoptionScenario,
    idempotency_key: &str,
) -> CourseInstanceReceiptTarget {
    fixture
        .store
        .read_state()
        .expect("state")
        .curriculum_adoption
        .receipt_targets
        .get(&(fixture.actor, key(idempotency_key)))
        .expect("immutable receipt target")
        .clone()
}
