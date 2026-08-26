use crate::{
    AccountRecord, AuthenticationEmail, CurriculumAdoptionStore, ReplaceAlphaCourseCommand,
    ReplaceBlueprintCommand, ReplaceUnissuedAssignmentDefinitionCommand, ReusableCurriculumStore,
    SessionLifetime, SessionStore, SessionSubject, Store,
};
use question_model::{
    AlphaCourseDefinitionInput, AlphaCourseModuleInput, AlphaInstantiationCommand,
    AlphaInstantiationCompleted, AlphaInstantiationPreviewRequest, AssignmentDeadlineBehavior,
    AssignmentDefinitionSourceView, AssignmentEnrollment, AssignmentFastForwardDecision,
    AssignmentFastForwardPreviewRequest, AssignmentInstructions, AssignmentRun,
    AssignmentScoringMode, BlueprintDefinitionInput, BlueprintInstantiationCommand,
    BlueprintInstantiationPreviewRequest, CompletionRequirement, ContinuedPractice, CourseTerm,
    CourseTermShiftCommand, CourseTermShiftPreviewRequest, CurriculumAdoptionIdempotencyKey,
    CurriculumAdoptionTitle, CurriculumPinReplacement, CurriculumPinReplacements, EnrollmentId,
    GradePolicy, LateSubmissionPolicy, LearnerDisclosurePolicy, ObservedAlphaAssignmentSource,
    ObservedAlphaSource, ObservedAssignmentRevision, ObservedBlueprintSource, PointValue,
    RelativeAssignmentSchedule, ReusableAssignmentDefaults, ReusableAssignmentDefinitionInput,
    ReusableAssignmentEntryInput, ReusableFixedQuestionInput, RunId, RunMode, RunPolicies,
    RunReference, SourceDerivedAssignmentPreviewRequest, StudentId, UserRole, VariationPolicy,
};
use uuid::Uuid;

use super::super::*;

struct Fixture {
    store: MemoryStore,
    tenant: TenantId,
    context: TenantContext,
    actor: UserId,
    session: SessionTokenHash,
    alpha: ObservedAlphaSource,
    blueprint: ObservedBlueprintSource,
    alpha_input: AlphaCourseDefinitionInput,
    term: CourseTerm,
    source_question: question_model::QuestionId,
    replacement_question: question_model::QuestionId,
}

impl Fixture {
    async fn new() -> Self {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(Uuid::from_u128(121_001));
        let context = TenantContext::from_authenticated_session(tenant);
        let actor = UserId::from_uuid(Uuid::from_u128(121_002));
        let session = SessionTokenHash::compute(b"curriculum-adoption-negative");
        let source_record = super::super::super::catalog_search_tests::record(121_003);
        let replacement_record = super::super::super::catalog_search_tests::record(121_004);
        let source_question = source_record.question_id.clone();
        let replacement_question = replacement_record.question_id.clone();
        {
            let mut state = store.write_state().expect("fixture state");
            state.published.insert(
                (source_record.problem, source_record.version),
                source_record,
            );
            state.published.insert(
                (replacement_record.problem, replacement_record.version),
                replacement_record,
            );
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
            state.accounts.insert(
                actor,
                AccountRecord {
                    user: actor,
                    email: AuthenticationEmail::parse("negative@example.edu").expect("email"),
                    display_name: "Negative Instructor".into(),
                    platform_roles: Vec::new(),
                    created_at: ActivityTimestamp::from_unix_millis(0),
                    updated_at: ActivityTimestamp::from_unix_millis(0),
                },
            );
        }
        store
            .create_session(
                session,
                SessionSubject::new(tenant, actor, "Negative", vec![UserRole::Instructor])
                    .expect("subject"),
                SessionLifetime::from_seconds(3_600).expect("lifetime"),
            )
            .await
            .expect("session");
        let alpha_input = AlphaCourseDefinitionInput {
            title: "Adoption source".into(),
            modules: vec![AlphaCourseModuleInput {
                label: "Exact module".into(),
                definitions: vec![definition(source_question.clone())],
            }],
        };
        let alpha = store
            .replace_alpha_course(
                context,
                session,
                ReplaceAlphaCourseCommand {
                    reference: None,
                    expected_revision: None,
                    definition: alpha_input.clone(),
                },
            )
            .await
            .expect("Alpha source");
        let blueprint = store
            .replace_blueprint(
                context,
                session,
                ReplaceBlueprintCommand {
                    reference: None,
                    expected_revision: None,
                    definition: BlueprintDefinitionInput {
                        definition: definition(source_question.clone()),
                    },
                },
            )
            .await
            .expect("Blueprint source");
        Self {
            store,
            tenant,
            context,
            actor,
            session,
            alpha: ObservedAlphaSource {
                reference: alpha.reference,
                revision: alpha.revision,
            },
            blueprint: ObservedBlueprintSource {
                reference: blueprint.reference,
                revision: blueprint.revision,
            },
            alpha_input,
            term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                .expect("term"),
            source_question,
            replacement_question,
        }
    }

    async fn instantiate(&self, suffix: &str) -> AlphaInstantiationCompleted {
        let title = format!("Course {suffix}");
        let preview = self
            .store
            .preview_alpha_instantiation(
                self.context,
                self.session,
                AlphaInstantiationPreviewRequest {
                    source: self.alpha,
                    title: CurriculumAdoptionTitle::parse(&title).expect("title"),
                    target_term: self.term.clone(),
                    replacements: CurriculumPinReplacements::default(),
                },
            )
            .await
            .expect("preview");
        self.store
            .apply_alpha_instantiation(
                self.context,
                self.session,
                AlphaInstantiationCommand::from_preview(&preview, key(suffix))
                    .expect("corrected preview"),
            )
            .await
            .expect("apply")
    }
}

fn definition(question_id: question_model::QuestionId) -> ReusableAssignmentDefinitionInput {
    ReusableAssignmentDefinitionInput {
        title: "Protein structure practice".into(),
        instructions: AssignmentInstructions::try_new("Explain each choice.".into())
            .expect("instructions"),
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
            learner_disclosure: LearnerDisclosurePolicy::default(),
        },
        schedule: RelativeAssignmentSchedule::default(),
    }
}

fn key(value: &str) -> CurriculumAdoptionIdempotencyKey {
    CurriculumAdoptionIdempotencyKey::parse(value).expect("key")
}

#[tokio::test]
async fn authority_revision_and_idempotency_conflicts_do_not_mutate() {
    let fixture = Fixture::new().await;
    let wrong_tenant =
        TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(121_099)));
    assert_eq!(
        fixture
            .store
            .preflight_curriculum_adoption(wrong_tenant, fixture.session)
            .await,
        Err(StoreError::NotFound)
    );

    let first = fixture.instantiate("same-key").await;
    let second_actor = UserId::from_uuid(Uuid::from_u128(121_020));
    let second_session = SessionTokenHash::compute(b"curriculum-adoption-second-instructor");
    {
        let mut state = fixture.store.write_state().expect("state");
        state.instructor_approvals.insert(
            second_actor,
            crate::StoredInstructorApproval {
                approval: question_model::InstructorApproval {
                    user: second_actor,
                    approved_by: fixture.actor,
                    approved_at: ActivityTimestamp::from_unix_millis(0),
                    revoked_at: None,
                },
                revision: crate::InstructorApprovalRevision::INITIAL,
            },
        );
        state.accounts.insert(
            second_actor,
            AccountRecord {
                user: second_actor,
                email: AuthenticationEmail::parse("second@example.edu").expect("email"),
                display_name: "Second Instructor".into(),
                platform_roles: Vec::new(),
                created_at: ActivityTimestamp::from_unix_millis(0),
                updated_at: ActivityTimestamp::from_unix_millis(0),
            },
        );
        let course = resolve_course(&state, fixture.tenant, first.course).expect("course");
        let membership = question_model::CourseMembershipId::from_uuid(Uuid::from_u128(121_021));
        state.course_memberships.insert(
            (fixture.tenant, membership),
            crate::CourseMembershipRecord {
                id: membership,
                tenant: fixture.tenant,
                course,
                user: second_actor,
                student: None,
                role: question_model::CourseMembershipRole::Instructor,
                roster_id: None,
                status: crate::CourseMemberStatus::Active,
                joined_at: ActivityTimestamp::from_unix_millis(0),
                revoked_at: None,
            },
        );
        state
            .active_course_membership_by_user
            .insert((fixture.tenant, course, second_actor), membership);
    }
    fixture
        .store
        .create_session(
            second_session,
            SessionSubject::new(
                fixture.tenant,
                second_actor,
                "Second",
                vec![UserRole::Instructor],
            )
            .expect("subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("second session");
    fixture
        .store
        .preview_alpha_instantiation(
            fixture.context,
            second_session,
            AlphaInstantiationPreviewRequest {
                source: fixture.alpha,
                title: CurriculumAdoptionTitle::parse("Public Alpha").expect("title"),
                target_term: fixture.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("public Alpha is readable by another approved Instructor");
    let actor_bound_preview = fixture
        .store
        .preview_alpha_instantiation(
            fixture.context,
            second_session,
            AlphaInstantiationPreviewRequest {
                source: fixture.alpha,
                title: CurriculumAdoptionTitle::parse("Course same-key").expect("title"),
                target_term: fixture.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("public source preview");
    assert_eq!(
        fixture
            .store
            .apply_alpha_instantiation(
                fixture.context,
                second_session,
                AlphaInstantiationCommand::from_preview(&actor_bound_preview, key("same-key"))
                    .expect("actor-bound command"),
            )
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(
        fixture
            .store
            .preview_blueprint_instantiation(
                fixture.context,
                second_session,
                BlueprintInstantiationPreviewRequest {
                    source: fixture.blueprint,
                    course: first.course,
                    target_term: fixture.term.clone(),
                    replacements: CurriculumPinReplacements::default(),
                },
            )
            .await,
        Err(StoreError::NotFound)
    );
    let other_preview = fixture
        .store
        .preview_alpha_instantiation(
            fixture.context,
            fixture.session,
            AlphaInstantiationPreviewRequest {
                source: fixture.alpha,
                title: CurriculumAdoptionTitle::parse("Different request").expect("title"),
                target_term: fixture.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("preview");
    let before = fixture.store.read_state().expect("state").courses.clone();
    assert_eq!(
        fixture
            .store
            .apply_alpha_instantiation(
                fixture.context,
                fixture.session,
                AlphaInstantiationCommand::from_preview(&other_preview, key("same-key"))
                    .expect("command"),
            )
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(fixture.store.read_state().expect("state").courses, before);
    assert!(
        fixture
            .store
            .inspect_curriculum_imports(fixture.context, fixture.session, first.course)
            .await
            .expect("inspection")
            .is_some()
    );

    let mut changed_alpha = fixture.alpha_input.clone();
    changed_alpha.title = "Changed source".into();
    let replacement = fixture
        .store
        .replace_alpha_course(
            fixture.context,
            fixture.session,
            ReplaceAlphaCourseCommand {
                reference: Some(fixture.alpha.reference),
                expected_revision: Some(fixture.alpha.revision),
                definition: changed_alpha,
            },
        )
        .await
        .expect("new revision");
    assert!(replacement.revision.value() > fixture.alpha.revision.value());
    assert_eq!(
        fixture
            .store
            .preview_alpha_instantiation(
                fixture.context,
                fixture.session,
                AlphaInstantiationPreviewRequest {
                    source: fixture.alpha,
                    title: CurriculumAdoptionTitle::parse("Stale source").expect("title"),
                    target_term: fixture.term.clone(),
                    replacements: CurriculumPinReplacements::default(),
                },
            )
            .await,
        Err(StoreError::Conflict)
    );
}

#[tokio::test]
async fn completed_receipt_requires_current_destination_membership_without_losing_evidence() {
    let fixture = Fixture::new().await;
    let preview = fixture
        .store
        .preview_alpha_instantiation(
            fixture.context,
            fixture.session,
            AlphaInstantiationPreviewRequest {
                source: fixture.alpha,
                title: CurriculumAdoptionTitle::parse("Receipt authority").expect("title"),
                target_term: fixture.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("preview");
    let command = AlphaInstantiationCommand::from_preview(&preview, key("receipt-authority"))
        .expect("command");
    let applied = fixture
        .store
        .apply_alpha_instantiation(fixture.context, fixture.session, command.clone())
        .await
        .expect("apply");
    let (course, adoption_assignments) = {
        let state = fixture.store.read_state().expect("state");
        let course = resolve_course(&state, fixture.tenant, applied.course).expect("course");
        (course, state.curriculum_course_adoptions[&(fixture.tenant, course)].assignments.clone())
    };
    {
        let mut state = fixture.store.write_state().expect("state");
        let membership = state.active_course_membership_by_user.remove(&(fixture.tenant, course, fixture.actor))
            .expect("direct instructor membership");
        state.course_memberships.get_mut(&(fixture.tenant, membership)).expect("membership").revoked_at =
            Some(ActivityTimestamp::from_unix_millis(1));
    }
    assert_eq!(
        fixture
            .store
            .apply_alpha_instantiation(fixture.context, fixture.session, command)
            .await,
        Err(StoreError::NotFound)
    );
    let state = fixture.store.read_state().expect("state");
    assert!(adoption_assignments.iter().all(|assignment| state
        .curriculum_assignment_adoption_evidence
        .contains_key(&(fixture.tenant, key("receipt-authority"), *assignment))));
}

#[tokio::test]
async fn stale_witness_and_first_issued_run_fence_roll_back_term_shift() {
    let fixture = Fixture::new().await;
    let applied = fixture.instantiate("shift-fence").await;
    let initial_witness = witness(&fixture, applied.course);
    let shifted = CourseTerm::from_parts("2027-01-11", "2027-05-08", "America/Chicago")
        .expect("shifted term");
    let preview = fixture
        .store
        .preview_course_term_shift(
            fixture.context,
            fixture.session,
            CourseTermShiftPreviewRequest {
                witness: initial_witness,
                target_term: shifted.clone(),
            },
        )
        .await
        .expect("shift preview");
    let stale_command =
        CourseTermShiftCommand::from_preview(&preview, key("stale-shift")).expect("command");

    let (course_id, assignment_id, revision, mut definition, base_policy) = {
        let state = fixture.store.read_state().expect("state");
        let course = resolve_course(&state, fixture.tenant, applied.course).expect("course");
        let assignment = state
            .assignments
            .iter()
            .find_map(|((tenant, id), row)| {
                (*tenant == fixture.tenant && row.course_id == course).then_some(*id)
            })
            .expect("assignment");
        (
            course,
            assignment,
            state.assignment_revisions[&(fixture.tenant, assignment)],
            state.assignments[&(fixture.tenant, assignment)].clone(),
            state.assignment_base_policy[&(fixture.tenant, assignment)].policy,
        )
    };
    definition.title = "Ordinary writer changed title".into();
    fixture
        .store
        .replace_unissued_assignment_definition(
            fixture.context,
            ReplaceUnissuedAssignmentDefinitionCommand {
                actor: fixture.actor,
                course: course_id,
                assignment: assignment_id,
                expected_revision: revision,
                definition,
                base_policy,
            },
        )
        .await
        .expect("ordinary writer");
    let before_stale = course_state(&fixture, applied.course);
    assert_eq!(
        fixture
            .store
            .apply_course_term_shift(fixture.context, fixture.session, stale_command)
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(course_state(&fixture, applied.course), before_stale);

    let fresh_preview = fixture
        .store
        .preview_course_term_shift(
            fixture.context,
            fixture.session,
            CourseTermShiftPreviewRequest {
                witness: witness(&fixture, applied.course),
                target_term: shifted,
            },
        )
        .await
        .expect("fresh preview");
    issue_run(&fixture, applied.course);
    let before_issued = course_state(&fixture, applied.course);
    assert_eq!(
        fixture
            .store
            .apply_course_term_shift(
                fixture.context,
                fixture.session,
                CourseTermShiftCommand::from_preview(&fresh_preview, key("issued-shift"))
                    .expect("command"),
            )
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(course_state(&fixture, applied.course), before_issued);
}

#[tokio::test]
async fn pin_reauthorization_and_fast_forward_recovery_preserve_existing_meaning() {
    let fixture = Fixture::new().await;
    let applied = fixture.instantiate("recovery").await;
    let imports = fixture
        .store
        .inspect_curriculum_imports(fixture.context, fixture.session, applied.course)
        .await
        .expect("inspection")
        .expect("course import");
    let imported = &imports.assignments[0];
    let (assignment_id, revision) = assignment(&fixture, applied.course, imported.assignment);
    {
        let mut state = fixture.store.write_state().expect("state");
        state
            .assignments
            .get_mut(&(fixture.tenant, assignment_id))
            .expect("assignment")
            .title = "Instructor divergence".into();
    }
    let mut revised = fixture.alpha_input.clone();
    revised.modules[0].definitions[0].title = "New source meaning".into();
    let alpha_v2 = fixture
        .store
        .replace_alpha_course(
            fixture.context,
            fixture.session,
            ReplaceAlphaCourseCommand {
                reference: Some(fixture.alpha.reference),
                expected_revision: Some(fixture.alpha.revision),
                definition: revised,
            },
        )
        .await
        .expect("source revision");
    let source = AssignmentDefinitionSourceView::Alpha(
        ObservedAlphaAssignmentSource::new(
            ObservedAlphaSource {
                reference: alpha_v2.reference,
                revision: alpha_v2.revision,
            },
            0,
            0,
        )
        .expect("exact source"),
    );
    let decision = fixture
        .store
        .preview_assignment_fast_forward(
            fixture.context,
            fixture.session,
            AssignmentFastForwardPreviewRequest {
                course: applied.course,
                assignment: ObservedAssignmentRevision {
                    assignment: imported.assignment,
                    revision,
                },
                import_revision: imported.revision,
                source,
            },
        )
        .await
        .expect("fast-forward preview")
        .decision;
    assert!(matches!(
        decision,
        AssignmentFastForwardDecision::Divergent { .. }
    ));

    let derived = fixture
        .store
        .preview_source_derived_assignment(
            fixture.context,
            fixture.session,
            SourceDerivedAssignmentPreviewRequest {
                course: applied.course,
                source,
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("source-derived preview");
    fixture
        .store
        .create_source_derived_assignment(
            fixture.context,
            fixture.session,
            question_model::CreateSourceDerivedAssignmentCommand::from_preview(
                &derived,
                key("preserve-divergence"),
            )
            .expect("command"),
        )
        .await
        .expect("source-derived apply");
    assert_eq!(
        fixture.store.read_state().expect("state").assignments[&(fixture.tenant, assignment_id)]
            .title,
        "Instructor divergence"
    );

    {
        let mut state = fixture.store.write_state().expect("state");
        let baseline_title = state.curriculum_import_baselines[&(fixture.tenant, assignment_id)]
            .payload
            .title()
            .to_owned();
        state
            .assignments
            .get_mut(&(fixture.tenant, assignment_id))
            .expect("assignment")
            .title = baseline_title;
    }
    issue_run_for_assignment(&fixture, assignment_id, 121_093);
    let issued_decision = fixture
        .store
        .preview_assignment_fast_forward(
            fixture.context,
            fixture.session,
            AssignmentFastForwardPreviewRequest {
                course: applied.course,
                assignment: ObservedAssignmentRevision {
                    assignment: imported.assignment,
                    revision,
                },
                import_revision: imported.revision,
                source,
            },
        )
        .await
        .expect("issued fast-forward preview")
        .decision;
    assert!(matches!(
        issued_decision,
        AssignmentFastForwardDecision::IssuedWork { .. }
    ));

    let wrong_replacements = CurriculumPinReplacements::new(vec![CurriculumPinReplacement {
        position: question_model::CurriculumPinPosition::new(None, 0, 1, None)
            .expect("bounded wrong position"),
        question: fixture.replacement_question.clone(),
    }])
    .expect("replacements");
    assert!(matches!(
        fixture
            .store
            .preview_blueprint_instantiation(
                fixture.context,
                fixture.session,
                BlueprintInstantiationPreviewRequest {
                    source: fixture.blueprint,
                    course: applied.course,
                    target_term: fixture.term.clone(),
                    replacements: wrong_replacements,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));

    {
        let mut state = fixture.store.write_state().expect("state");
        state
            .published
            .values_mut()
            .find(|record| record.question_id == fixture.source_question)
            .expect("source publication")
            .lifecycle = question_model::CatalogLifecycle::Deprecated {
            reason: "still assignable".into(),
        };
    }
    let authorized_preview = fixture
        .store
        .preview_blueprint_instantiation(
            fixture.context,
            fixture.session,
            BlueprintInstantiationPreviewRequest {
                source: fixture.blueprint,
                course: applied.course,
                target_term: fixture.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("authorized pin preview");
    {
        let mut state = fixture.store.write_state().expect("state");
        state
            .published
            .values_mut()
            .find(|record| record.question_id == fixture.source_question)
            .expect("source publication")
            .lifecycle = question_model::CatalogLifecycle::Archived {
            reason: "retired".into(),
        };
    }
    let before_reauthorization = fixture
        .store
        .read_state()
        .expect("state")
        .assignments
        .clone();
    assert_eq!(
        fixture
            .store
            .apply_blueprint_instantiation(
                fixture.context,
                fixture.session,
                BlueprintInstantiationCommand::from_preview(
                    &authorized_preview,
                    key("pin-reauthorize"),
                )
                .expect("command"),
            )
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(
        fixture.store.read_state().expect("state").assignments,
        before_reauthorization
    );

    let recovery = fixture
        .store
        .preview_blueprint_instantiation(
            fixture.context,
            fixture.session,
            BlueprintInstantiationPreviewRequest {
                source: fixture.blueprint,
                course: applied.course,
                target_term: fixture.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("pin recovery preview")
        .pin_correction
        .expect("typed pin correction");
    let question_model::UnavailablePinRecoveryAction::SelectReplacementQuestion {
        position, ..
    } = recovery;
    assert_eq!(
        position,
        question_model::CurriculumPinPosition::new(None, 0, 0, None).expect("exact position")
    );
    let corrected = CurriculumPinReplacements::new(vec![CurriculumPinReplacement {
        position,
        question: fixture.replacement_question.clone(),
    }])
    .expect("correct replacement");
    let corrected_preview = fixture
        .store
        .preview_blueprint_instantiation(
            fixture.context,
            fixture.session,
            BlueprintInstantiationPreviewRequest {
                source: fixture.blueprint,
                course: applied.course,
                target_term: fixture.term.clone(),
                replacements: corrected,
            },
        )
        .await
        .expect("corrected preview");
    assert!(corrected_preview.pin_correction.is_none());
}

fn witness(fixture: &Fixture, course: CourseReference) -> CourseScheduleWitness {
    let state = fixture.store.read_state().expect("state");
    let course = resolve_course(&state, fixture.tenant, course).expect("course");
    course_witness(&state, fixture.tenant, course).expect("witness")
}

fn assignment(
    fixture: &Fixture,
    course: CourseReference,
    reference: question_model::AssignmentReference,
) -> (AssignmentId, question_model::AssignmentRevision) {
    let state = fixture.store.read_state().expect("state");
    let course_id = resolve_course(&state, fixture.tenant, course).expect("course");
    let assignment = state.assignments_by_reference[&(fixture.tenant, reference)];
    assert_eq!(
        state.assignments[&(fixture.tenant, assignment)].course_id,
        course_id
    );
    (
        assignment,
        state.assignment_revisions[&(fixture.tenant, assignment)],
    )
}

fn course_state(fixture: &Fixture, course: CourseReference) -> (CourseTerm, CourseScheduleWitness) {
    let state = fixture.store.read_state().expect("state");
    let course_id = resolve_course(&state, fixture.tenant, course).expect("course");
    (
        state.courses[&(fixture.tenant, course_id)].term.clone(),
        course_witness(&state, fixture.tenant, course_id).expect("witness"),
    )
}

fn issue_run(fixture: &Fixture, course: CourseReference) {
    let assignment = {
        let state = fixture.store.read_state().expect("state");
        let course = resolve_course(&state, fixture.tenant, course).expect("course");
        state
            .assignments
            .iter()
            .find_map(|((tenant, id), row)| {
                (*tenant == fixture.tenant && row.course_id == course).then_some(*id)
            })
            .expect("assignment")
    };
    issue_run_for_assignment(fixture, assignment, 121_090);
}

fn issue_run_for_assignment(fixture: &Fixture, assignment: AssignmentId, number: u128) {
    let mut state = fixture.store.write_state().expect("state");
    let enrollment = EnrollmentId::from_uuid(Uuid::from_u128(number));
    let run = RunId::from_uuid(Uuid::from_u128(number + 1));
    state.enrollments.insert(
        (fixture.tenant, enrollment),
        AssignmentEnrollment {
            id: enrollment,
            tenant: fixture.tenant,
            assignment,
            user: fixture.actor,
            student: StudentId::from_uuid(Uuid::from_u128(number + 2)),
            first_completed_at: None,
            current_grade_run: None,
            best_grade_run: None,
        },
    );
    state.runs.insert(
        (fixture.tenant, run),
        AssignmentRun {
            id: run,
            reference: RunReference::new(1).expect("run reference"),
            tenant: fixture.tenant,
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
