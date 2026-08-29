//! Fast contract checks for reusable-curriculum Memory behavior.

use super::*;
use crate::{
    AccountRecord, AuthenticationEmail, CatalogStore, CatalogTransition, PageRequest, PageSize,
    ReplaceAlphaCourseCommand, ReplaceBlueprintCommand, ReusableCurriculumStore, SessionLifetime,
    SessionStore, SessionSubject, SessionTokenHash, TenantContext,
};
use question_model::{
    AlphaCourseAccess, AlphaCourseDefinitionInput, AlphaCourseModuleInput,
    AssignmentDeadlineBehavior, AssignmentInstructions, AssignmentScoringMode,
    BlueprintDefinitionInput, CompletionRequirement, ContinuedPractice, GradePolicy,
    LateSubmissionPolicy, PointValue, PublicationScope, RelativeAssignmentSchedule,
    ReusableAssignmentDefaults, ReusableAssignmentDefinitionInput, ReusableAssignmentEntryInput,
    ReusableFixedQuestionInput, RunPolicies, StudentDisclosurePolicy, UserRole, VariationPolicy,
};
use uuid::Uuid;

fn definition(question_id: question_model::QuestionId) -> BlueprintDefinitionInput {
    BlueprintDefinitionInput {
        definition: ReusableAssignmentDefinitionInput {
            title: "Protein structure practice".to_string(),
            instructions: AssignmentInstructions::try_new("Explain each choice.".to_string())
                .expect("valid instructions"),
            entries: vec![ReusableAssignmentEntryInput::Fixed(
                ReusableFixedQuestionInput {
                    question_id,
                    points_possible: PointValue::from_whole(3),
                    scoring_mode: AssignmentScoringMode::Normal,
                },
            )],
            defaults: ReusableAssignmentDefaults {
                time_limit_seconds: None,
                attempt_limit: None,
                late_submission: LateSubmissionPolicy::Accept,
                deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
                run_policies: RunPolicies {
                    completion: CompletionRequirement::AnswerAll,
                    grade: GradePolicy::Highest,
                    continued_practice: ContinuedPractice::Unlimited,
                    variation: VariationPolicy::NewSeeds,
                },
                student_disclosure: StudentDisclosurePolicy::default(),
            },
            schedule: RelativeAssignmentSchedule::default(),
        },
    }
}

fn alpha_definition(question_id: question_model::QuestionId) -> AlphaCourseDefinitionInput {
    AlphaCourseDefinitionInput {
        title: "Biochemistry alpha".to_string(),
        modules: vec![AlphaCourseModuleInput {
            label: "Week one".to_string(),
            definitions: vec![definition(question_id).definition],
        }],
    }
}

fn approve(state: &mut State, actor: UserId) {
    state.instructor_approvals.insert(
        actor,
        crate::StoredInstructorApproval {
            approval: question_model::InstructorApproval {
                user: actor,
                approved_by: actor,
                approved_at: ActivityTimestamp::from_unix_millis(0),
                revoked_at: None,
            },
            revision: crate::InstructorApprovalRevision::INITIAL,
        },
    );
}

async fn session(
    store: &MemoryStore,
    tenant: TenantId,
    actor: UserId,
    roles: Vec<UserRole>,
    token: &'static [u8],
) -> SessionTokenHash {
    let session = SessionTokenHash::compute(token);
    store
        .create_session(
            session,
            SessionSubject::new(tenant, actor, "Instructor", roles).expect("valid session"),
            SessionLifetime::from_seconds(60).expect("session lifetime"),
        )
        .await
        .expect("session stored");
    session
}

#[tokio::test]
async fn blueprint_replacement_is_revision_checked_atomic_and_answer_free() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(96_001));
    let context = TenantContext::from_authenticated_session(tenant);
    let actor = UserId::from_uuid(Uuid::from_u128(96_002));
    let session = SessionTokenHash::compute(b"reusable-curriculum-elena");
    let record = super::catalog_search_tests::record(96_003);
    let question_id = record.question_id.clone();
    {
        let mut state = store.write_state().expect("fixture state");
        state
            .published
            .insert((record.problem, record.version), record);
        state.instructor_approvals.insert(
            actor,
            crate::StoredInstructorApproval {
                approval: question_model::InstructorApproval {
                    user: actor,
                    approved_by: actor,
                    approved_at: ActivityTimestamp::from_unix_millis(0),
                    revoked_at: None,
                },
                revision: crate::InstructorApprovalRevision::INITIAL,
            },
        );
    }
    store
        .create_session(
            session,
            SessionSubject::new(tenant, actor, "Elena", vec![UserRole::Instructor])
                .expect("instructor session"),
            SessionLifetime::from_seconds(60).expect("session lifetime"),
        )
        .await
        .expect("session stored");
    let blueprint_definition = definition(question_id);
    let created = store
        .replace_blueprint(
            context,
            session,
            ReplaceBlueprintCommand {
                reference: None,
                expected_revision: None,
                definition: blueprint_definition.clone(),
            },
        )
        .await
        .expect("blueprint created");
    let second = store
        .replace_blueprint(
            context,
            session,
            ReplaceBlueprintCommand {
                reference: None,
                expected_revision: None,
                definition: blueprint_definition.clone(),
            },
        )
        .await
        .expect("second blueprint created");
    assert_eq!(created.revision.value(), 1);
    let stale = store
        .replace_blueprint(
            context,
            session,
            ReplaceBlueprintCommand {
                reference: Some(created.reference),
                expected_revision: Some(
                    question_model::BlueprintRevision::INITIAL
                        .checked_next()
                        .expect("revision increments"),
                ),
                definition: blueprint_definition,
            },
        )
        .await;
    assert!(matches!(stale, Err(StoreError::Conflict)));
    let reread = store
        .get_blueprint(context, session, created.reference)
        .await
        .expect("read result")
        .expect("owned blueprint");
    assert_eq!(reread, created);
    let page = store
        .list_blueprints(
            context,
            session,
            PageRequest::first(PageSize::new(1).expect("page size")),
        )
        .await
        .expect("private list");
    assert_eq!(
        page.items.first().map(|item| item.reference),
        Some(created.reference)
    );
    let cursor = page.next_cursor.expect("first page continues");
    let other = UserId::from_uuid(Uuid::from_u128(96_004));
    let other_tenant = TenantId::from_uuid(Uuid::from_u128(96_005));
    let other_session = SessionTokenHash::compute(b"reusable-curriculum-other");
    let other_tenant_session = SessionTokenHash::compute(b"reusable-curriculum-other-tenant");
    {
        let mut state = store.write_state().expect("authority state");
        approve(&mut state, other);
    }
    for (context, token) in [
        (context, other_session),
        (
            TenantContext::from_authenticated_session(other_tenant),
            other_tenant_session,
        ),
    ] {
        store
            .create_session(
                token,
                SessionSubject::new(
                    context.tenant_id(),
                    other,
                    "Other instructor",
                    vec![UserRole::Instructor],
                )
                .expect("valid session"),
                SessionLifetime::from_seconds(60).expect("session lifetime"),
            )
            .await
            .expect("session stored");
    }
    for (foreign_context, foreign_session) in [
        (context, other_session),
        (
            TenantContext::from_authenticated_session(other_tenant),
            other_tenant_session,
        ),
    ] {
        assert!(matches!(
            store
                .list_blueprints(
                    foreign_context,
                    foreign_session,
                    PageRequest::after(cursor.clone(), PageSize::new(1).expect("page size")),
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
    }
    assert!(matches!(
        store
            .list_alpha_courses(
                context,
                session,
                PageRequest::after(cursor.clone(), PageSize::new(1).expect("page size")),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let resumed = store
        .list_blueprints(
            context,
            session,
            PageRequest::after(cursor, PageSize::new(1).expect("page size")),
        )
        .await
        .expect("authorized continuation");
    assert_eq!(
        resumed.items.first().map(|item| item.reference),
        Some(second.reference)
    );
    assert!(resumed.next_cursor.is_none());
}

#[tokio::test]
async fn alpha_is_public_only_creator_owned_and_cross_tenant_readable() {
    let store = MemoryStore::default();
    let creator_tenant = TenantId::from_uuid(Uuid::from_u128(97_001));
    let reader_tenant = TenantId::from_uuid(Uuid::from_u128(97_002));
    let creator = UserId::from_uuid(Uuid::from_u128(97_003));
    let reader = UserId::from_uuid(Uuid::from_u128(97_004));
    let student = UserId::from_uuid(Uuid::from_u128(97_005));
    let unapproved = UserId::from_uuid(Uuid::from_u128(97_008));
    let mut public_record = super::catalog_search_tests::record(97_006);
    public_record.author_ids = vec![creator];
    let public_pin = question_model::ProblemVersionRef {
        problem: public_record.problem,
        version: public_record.version,
    };
    let public_id = public_record.question_id.clone();
    let mut institution_record = super::catalog_search_tests::record(97_007);
    institution_record.scope = PublicationScope::Institution;
    let institution_id = institution_record.question_id.clone();
    {
        let mut state = store.write_state().expect("fixture state");
        state.published.insert(
            (public_record.problem, public_record.version),
            public_record,
        );
        state.published.insert(
            (institution_record.problem, institution_record.version),
            institution_record,
        );
        state
            .problem_owner_tenants
            .insert(public_pin.problem, creator_tenant);
        approve(&mut state, creator);
        approve(&mut state, reader);
        state.accounts.insert(
            creator,
            AccountRecord {
                user: creator,
                email: AuthenticationEmail::parse("elena@example.edu").expect("email"),
                display_name: "Elena Instructor".to_string(),
                platform_roles: Vec::new(),
                created_at: ActivityTimestamp::from_unix_millis(0),
                updated_at: ActivityTimestamp::from_unix_millis(0),
            },
        );
    }
    let creator_session = session(
        &store,
        creator_tenant,
        creator,
        vec![UserRole::Instructor],
        b"alpha-creator",
    )
    .await;
    let reader_session = session(
        &store,
        reader_tenant,
        reader,
        vec![UserRole::Instructor],
        b"alpha-reader",
    )
    .await;
    let student_session = session(
        &store,
        creator_tenant,
        student,
        vec![UserRole::Student],
        b"alpha-student",
    )
    .await;
    let unapproved_session = session(
        &store,
        reader_tenant,
        unapproved,
        vec![UserRole::Instructor],
        b"alpha-unapproved",
    )
    .await;
    let input = alpha_definition(public_id.clone());
    let created = store
        .replace_alpha_course(
            TenantContext::from_authenticated_session(creator_tenant),
            creator_session,
            ReplaceAlphaCourseCommand {
                reference: None,
                expected_revision: None,
                definition: input.clone(),
            },
        )
        .await
        .expect("public Alpha created");
    assert_eq!(created.creator_byline.names[0].as_str(), "Elena Instructor");
    let unchanged = store
        .replace_alpha_course(
            TenantContext::from_authenticated_session(creator_tenant),
            creator_session,
            ReplaceAlphaCourseCommand {
                reference: Some(created.reference),
                expected_revision: Some(created.revision),
                definition: input.clone(),
            },
        )
        .await
        .expect("no-op accepted");
    assert_eq!(unchanged.revision, created.revision);
    let mut updated_input = input.clone();
    updated_input.title = "Biochemistry alpha revised".to_string();
    let updated = store
        .replace_alpha_course(
            TenantContext::from_authenticated_session(creator_tenant),
            creator_session,
            ReplaceAlphaCourseCommand {
                reference: Some(created.reference),
                expected_revision: Some(created.revision),
                definition: updated_input.clone(),
            },
        )
        .await
        .expect("revision advances");
    assert_eq!(updated.revision.value(), 2);
    assert!(matches!(
        store
            .replace_alpha_course(
                TenantContext::from_authenticated_session(creator_tenant),
                creator_session,
                ReplaceAlphaCourseCommand {
                    reference: Some(created.reference),
                    expected_revision: Some(created.revision),
                    definition: updated_input.clone(),
                },
            )
            .await,
        Err(StoreError::Conflict)
    ));
    let reader_context = TenantContext::from_authenticated_session(reader_tenant);
    let reader_view = store
        .get_alpha_course(reader_context, reader_session, created.reference)
        .await
        .expect("reader request")
        .expect("public Alpha visible");
    assert_eq!(reader_view.access, AlphaCourseAccess::ApprovedInstructor);
    assert_eq!(reader_view.creator_byline, created.creator_byline);
    let reader_summary = store
        .list_alpha_courses(
            reader_context,
            reader_session,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("shared list")
        .items
        .into_iter()
        .find(|summary| summary.reference == created.reference)
        .expect("public Alpha has its stable shared reference");
    assert_eq!(reader_summary.access, AlphaCourseAccess::ApprovedInstructor);
    assert_eq!(reader_summary.creator_byline, created.creator_byline);
    assert!(matches!(
        store
            .replace_alpha_course(
                reader_context,
                reader_session,
                ReplaceAlphaCourseCommand {
                    reference: Some(created.reference),
                    expected_revision: Some(updated.revision),
                    definition: updated_input.clone(),
                },
            )
            .await,
        Err(StoreError::Forbidden)
    ));
    assert!(matches!(
        store
            .list_alpha_courses(
                reader_context,
                unapproved_session,
                PageRequest::first(PageSize::new(10).expect("page size")),
            )
            .await,
        Err(StoreError::Forbidden)
    ));
    assert!(matches!(
        store
            .get_alpha_course(
                TenantContext::from_authenticated_session(creator_tenant),
                student_session,
                created.reference,
            )
            .await,
        Err(StoreError::Forbidden)
    ));
    assert!(matches!(
        store
            .replace_alpha_course(
                TenantContext::from_authenticated_session(creator_tenant),
                creator_session,
                ReplaceAlphaCourseCommand {
                    reference: None,
                    expected_revision: None,
                    definition: alpha_definition(institution_id),
                },
            )
            .await,
        Err(StoreError::NotFound)
    ));
    let after_rejected_create = store
        .get_alpha_course(
            TenantContext::from_authenticated_session(creator_tenant),
            creator_session,
            created.reference,
        )
        .await
        .expect("reread after failed create")
        .expect("known Alpha remains visible");
    assert_eq!(after_rejected_create, updated);
    store
        .transition_catalog_problem(
            TenantContext::from_authenticated_session(creator_tenant),
            creator,
            public_pin,
            CatalogTransition::Deprecate {
                reason: "Superseded by a corrected question".to_string(),
            },
        )
        .await
        .expect("deprecate the exact retained publication");
    assert!(matches!(
        store
            .replace_alpha_course(
                TenantContext::from_authenticated_session(creator_tenant),
                creator_session,
                ReplaceAlphaCourseCommand {
                    reference: None,
                    expected_revision: None,
                    definition: alpha_definition(public_id),
                },
            )
            .await,
        Err(StoreError::NotFound)
    ));
    let retained = store
        .get_alpha_course(
            TenantContext::from_authenticated_session(creator_tenant),
            creator_session,
            created.reference,
        )
        .await
        .expect("retained Alpha read")
        .expect("known Alpha remains inspectable");
    let question = match &retained.modules[0].definitions[0].entries[0] {
        question_model::ReusableAssignmentEntryView::Fixed { question, .. } => question,
        question_model::ReusableAssignmentEntryView::Pool(_) => panic!("fixture has a fixed item"),
    };
    assert_eq!(
        question.selection_availability,
        question_model::ReusableSelectionAvailability::Retained,
        "the exact pinned member stays inspectable after deprecation"
    );
}
