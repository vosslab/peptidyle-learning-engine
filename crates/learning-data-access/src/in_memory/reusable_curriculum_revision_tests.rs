//! Durable Memory behavior for immutable BlueprintCourse history and stable child handles.

use super::*;
use crate::{
    CreateBlueprintCourseCommand, InstructorApprovalRevision, ReplaceBlueprintCourseCommand,
    ReusableCurriculumStore, SessionLifetime, SessionStore, SessionSubject,
    StoredInstructorApproval,
};
use question_model::{
    AssignmentDefinitionSourceView, AssignmentInstructions, AssignmentScoringMode,
    BlueprintAssignmentEditHandle, BlueprintCourseAssignmentReplacementInput,
    BlueprintCourseModuleReplacementInput, BlueprintModuleEditHandle, CompletionRequirement,
    ContinuedPractice, CreateBlueprintCourseDefinitionInput, CreateBlueprintCourseModuleInput,
    GradePolicy, LateSubmissionPolicy, ObservedBlueprintSource, PointValue,
    ReplaceBlueprintCourseDefinitionInput, ReusableAssignmentDefaults,
    ReusableAssignmentDefinitionInput, ReusableAssignmentEntryInput, ReusableFixedQuestionInput,
    RunPolicies, StudentDisclosurePolicy, UserRole, VariationPolicy,
};
use uuid::Uuid;

fn assignment(
    question: question_model::QuestionId,
    title: &str,
) -> ReusableAssignmentDefinitionInput {
    ReusableAssignmentDefinitionInput {
        title: title.to_owned(),
        instructions: AssignmentInstructions::try_new("Choose the best answer.".to_owned())
            .expect("fixture instructions"),
        entries: vec![ReusableAssignmentEntryInput::Fixed(
            ReusableFixedQuestionInput {
                question_id: question,
                points_possible: PointValue::from_whole(2),
                scoring_mode: AssignmentScoringMode::Normal,
            },
        )],
        defaults: ReusableAssignmentDefaults {
            time_limit_seconds: None,
            attempt_limit: None,
            late_submission: LateSubmissionPolicy::Accept,
            deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
            run_policies: RunPolicies {
                completion: CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: ContinuedPractice::Unlimited,
                variation: VariationPolicy::NewSeeds,
            },
            student_disclosure: StudentDisclosurePolicy::default(),
        },
        schedule: question_model::RelativeAssignmentSchedule::default(),
    }
}

fn create_definition(
    question: question_model::QuestionId,
) -> (
    CreateBlueprintCourseDefinitionInput,
    ReusableAssignmentDefinitionInput,
    ReusableAssignmentDefinitionInput,
) {
    let first = assignment(question.clone(), "First retained assignment");
    let second = assignment(question, "Second retained assignment");
    (
        CreateBlueprintCourseDefinitionInput {
            title: "Blueprint history".to_owned(),
            modules: vec![CreateBlueprintCourseModuleInput {
                label: "Core module".to_owned(),
                definitions: vec![first.clone(), second.clone()],
            }],
        },
        first,
        second,
    )
}

async fn fixture() -> (
    MemoryStore,
    TenantContext,
    SessionTokenHash,
    SessionTokenHash,
    question_model::QuestionId,
) {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(97_001));
    let owner = UserId::from_uuid(Uuid::from_u128(97_002));
    let reader = UserId::from_uuid(Uuid::from_u128(97_003));
    let record = super::catalog_search_tests::record(97_004);
    let question = record.question_id.clone();
    {
        let mut state = store.write_state().expect("fixture state");
        state
            .published
            .insert((record.problem, record.version), record);
        for actor in [owner, reader] {
            state.instructor_approvals.insert(
                actor,
                StoredInstructorApproval {
                    approval: question_model::InstructorApproval {
                        user: actor,
                        approved_by: owner,
                        approved_at: ActivityTimestamp::from_unix_millis(0),
                        revoked_at: None,
                    },
                    revision: InstructorApprovalRevision::INITIAL,
                },
            );
        }
    }
    let owner_session = SessionTokenHash::compute(b"blueprint-history-owner");
    let reader_session = SessionTokenHash::compute(b"blueprint-history-reader");
    for (actor, label, session) in [
        (owner, "Owner", owner_session),
        (reader, "Reader", reader_session),
    ] {
        store
            .create_session(
                session,
                SessionSubject::new(tenant, actor, label, vec![UserRole::Instructor])
                    .expect("instructor session"),
                SessionLifetime::from_seconds(60).expect("session lifetime"),
            )
            .await
            .expect("session stored");
    }
    (
        store,
        TenantContext::from_authenticated_session(tenant),
        owner_session,
        reader_session,
        question,
    )
}

fn retained_module(
    module_id: question_model::BlueprintModuleId,
    label: &str,
    definitions: Vec<(
        question_model::BlueprintAssignmentId,
        ReusableAssignmentDefinitionInput,
    )>,
) -> BlueprintCourseModuleReplacementInput {
    BlueprintCourseModuleReplacementInput {
        handle: BlueprintModuleEditHandle::Retained { module_id },
        label: label.to_owned(),
        definitions: definitions
            .into_iter()
            .map(
                |(assignment_id, definition)| BlueprintCourseAssignmentReplacementInput {
                    handle: BlueprintAssignmentEditHandle::Retained { assignment_id },
                    definition,
                },
            )
            .collect(),
    }
}

#[tokio::test]
async fn immutable_history_preserves_retained_assignment_identity_across_reorder_and_insert() {
    let (store, context, owner, reader, question) = fixture().await;
    let (creation, _first, second) = create_definition(question.clone());
    let created = store
        .create_blueprint_course(
            context,
            owner,
            CreateBlueprintCourseCommand {
                definition: creation,
            },
        )
        .await
        .expect("creation succeeds");
    let module = &created.modules[0];
    let first_id = module.definitions[0].assignment_id;
    let second_id = module.definitions[1].assignment_id;
    let initial_source = AssignmentDefinitionSourceView::new(
        ObservedBlueprintSource {
            reference: created.reference,
            revision: created.revision,
        },
        first_id,
    );
    let reader_view = store
        .get_blueprint_course(context, reader, created.reference)
        .await
        .expect("reader request")
        .expect("published BlueprintCourse exists");
    assert_eq!(
        reader_view.access,
        question_model::BlueprintCourseAccess::ApprovedInstructor
    );
    let listed = store
        .list_blueprint_courses(
            context,
            reader,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("approved Instructor list");
    let listed_summary = listed
        .items
        .iter()
        .find(|summary| summary.reference == created.reference)
        .expect("created BlueprintCourse appears in the approved Instructor list");
    assert_eq!(
        listed_summary.access,
        question_model::BlueprintCourseAccess::ApprovedInstructor
    );
    let serialized = serde_json::to_string(&reader_view).expect("answer-free view serializes");
    assert!(!serialized.contains("\"answer\""));
    assert!(!serialized.contains("\"answers\""));

    let revised_first = assignment(question.clone(), "First retained assignment, revised");
    let inserted = assignment(question, "Inserted neighbor");
    let revised = store
        .replace_blueprint_course(
            context,
            owner,
            ReplaceBlueprintCourseCommand {
                reference: created.reference,
                expected_revision: created.revision,
                definition: ReplaceBlueprintCourseDefinitionInput {
                    title: created.title.clone(),
                    modules: vec![
                        BlueprintCourseModuleReplacementInput {
                            handle: BlueprintModuleEditHandle::New,
                            label: "Inserted module".to_owned(),
                            definitions: vec![BlueprintCourseAssignmentReplacementInput {
                                handle: BlueprintAssignmentEditHandle::New,
                                definition: inserted,
                            }],
                        },
                        retained_module(
                            module.module_id,
                            "Core module",
                            vec![(second_id, second), (first_id, revised_first)],
                        ),
                    ],
                },
            },
        )
        .await
        .expect("retained reorder succeeds");
    assert!(revised.revision > created.revision);
    let state = store.read_state().expect("state");
    let historical = super::reusable_curriculum::curriculum_assignment_source_snapshot(
        &state,
        context.tenant_id(),
        UserId::from_uuid(Uuid::from_u128(97_002)),
        initial_source,
    )
    .expect("old exact snapshot remains available");
    let current = super::reusable_curriculum::current_assignment_source(
        &state,
        context.tenant_id(),
        UserId::from_uuid(Uuid::from_u128(97_002)),
        initial_source,
    )
    .expect("retained assignment resolves at head");
    let current_snapshot = super::reusable_curriculum::curriculum_assignment_source_snapshot(
        &state,
        context.tenant_id(),
        UserId::from_uuid(Uuid::from_u128(97_002)),
        current,
    )
    .expect("current exact snapshot remains available");
    assert!(current.is_strictly_newer_revision_of(initial_source));
    assert_eq!(current.assignment_id(), first_id);
    let question_model::curriculum_adoption::CurriculumSemanticPayload::Assignment(historical) =
        historical.payload
    else {
        panic!("historical source must reconstruct one assignment");
    };
    let question_model::curriculum_adoption::CurriculumSemanticPayload::Assignment(current) =
        current_snapshot.payload
    else {
        panic!("current source must reconstruct one assignment");
    };
    let historical_pin = match historical.entries() {
        [
            question_model::curriculum_adoption::CurriculumSemanticAssignmentEntry::Fixed {
                reference,
                ..
            },
        ] => *reference,
        _ => panic!("historical source must retain its fixed question pin"),
    };
    let current_pin = match current.entries() {
        [
            question_model::curriculum_adoption::CurriculumSemanticAssignmentEntry::Fixed {
                reference,
                ..
            },
        ] => *reference,
        _ => panic!("current source must retain its fixed question pin"),
    };
    assert_eq!(historical.title(), "First retained assignment");
    assert_eq!(current.title(), "First retained assignment, revised");
    assert_eq!(historical_pin, current_pin);
}

#[tokio::test]
async fn replacement_refuses_foreign_stale_and_removed_handles_and_keeps_no_op_revision() {
    let (store, context, owner, _reader, question) = fixture().await;
    let (creation, first, second) = create_definition(question.clone());
    let created = store
        .create_blueprint_course(
            context,
            owner,
            CreateBlueprintCourseCommand {
                definition: creation,
            },
        )
        .await
        .expect("creation succeeds");
    let module = &created.modules[0];
    let first_id = module.definitions[0].assignment_id;
    let second_id = module.definitions[1].assignment_id;
    let no_op = ReplaceBlueprintCourseCommand {
        reference: created.reference,
        expected_revision: created.revision,
        definition: ReplaceBlueprintCourseDefinitionInput {
            title: created.title.clone(),
            modules: vec![retained_module(
                module.module_id,
                "Core module",
                vec![(first_id, first.clone()), (second_id, second.clone())],
            )],
        },
    };
    let unchanged = store
        .replace_blueprint_course(context, owner, no_op)
        .await
        .expect("identity-and-meaning no-op succeeds");
    assert_eq!(unchanged.revision, created.revision);

    let foreign = store
        .replace_blueprint_course(
            context,
            owner,
            ReplaceBlueprintCourseCommand {
                reference: created.reference,
                expected_revision: created.revision,
                definition: ReplaceBlueprintCourseDefinitionInput {
                    title: created.title.clone(),
                    modules: vec![BlueprintCourseModuleReplacementInput {
                        handle: BlueprintModuleEditHandle::Retained {
                            module_id: question_model::BlueprintModuleId::from_uuid(
                                Uuid::from_u128(97_099),
                            ),
                        },
                        label: "Core module".to_owned(),
                        definitions: vec![BlueprintCourseAssignmentReplacementInput {
                            handle: BlueprintAssignmentEditHandle::Retained {
                                assignment_id: first_id,
                            },
                            definition: first.clone(),
                        }],
                    }],
                },
            },
        )
        .await;
    assert!(matches!(
        foreign,
        Err(StoreError::InvalidRecord(_)) | Err(StoreError::Conflict)
    ));

    let removed = store
        .replace_blueprint_course(
            context,
            owner,
            ReplaceBlueprintCourseCommand {
                reference: created.reference,
                expected_revision: created.revision,
                definition: ReplaceBlueprintCourseDefinitionInput {
                    title: created.title.clone(),
                    modules: vec![retained_module(
                        module.module_id,
                        "Core module",
                        vec![(second_id, second)],
                    )],
                },
            },
        )
        .await
        .expect("omitting retained assignment removes it");
    let old_source = AssignmentDefinitionSourceView::new(
        ObservedBlueprintSource {
            reference: created.reference,
            revision: created.revision,
        },
        first_id,
    );
    {
        let state = store.read_state().expect("state");
        assert!(matches!(
            super::reusable_curriculum::current_assignment_source(
                &state,
                context.tenant_id(),
                UserId::from_uuid(Uuid::from_u128(97_002)),
                old_source,
            ),
            Err(StoreError::NotFound)
        ));
    }
    let stale = store
        .replace_blueprint_course(
            context,
            owner,
            ReplaceBlueprintCourseCommand {
                reference: created.reference,
                expected_revision: created.revision,
                definition: ReplaceBlueprintCourseDefinitionInput {
                    title: created.title.clone(),
                    modules: vec![retained_module(
                        module.module_id,
                        "Core module",
                        vec![
                            (first_id, first),
                            (
                                second_id,
                                assignment(question, "Second retained assignment"),
                            ),
                        ],
                    )],
                },
            },
        )
        .await;
    assert!(matches!(stale, Err(StoreError::Conflict)));
    assert!(removed.revision > created.revision);
}
