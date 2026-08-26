use crate::{
    AccountRecord, AuthenticationEmail, CurriculumAdoptionStore, ReplaceAlphaCourseCommand,
    ReplaceBlueprintCommand, ReusableCurriculumStore, SessionLifetime, SessionStore,
    SessionSubject,
};
use question_model::{
    AlphaCourseDefinitionInput, AlphaCourseModuleInput, AlphaInstantiationCommand,
    AlphaInstantiationPreviewRequest, AssignmentDeadlineBehavior, AssignmentDefinitionSourceView,
    AssignmentFastForwardCommand, AssignmentFastForwardPreviewRequest, AssignmentInstructions,
    AssignmentScoringMode, BlueprintDefinitionInput, BlueprintInstantiationCommand,
    BlueprintInstantiationPreviewRequest, CompletionRequirement, ContinuedPractice,
    CourseRolloverCommand, CourseRolloverPreviewRequest, CourseTerm, CourseTermShiftCommand,
    CourseTermShiftPreviewRequest, CreateSourceDerivedAssignmentCommand,
    CurriculumAdoptionIdempotencyKey, CurriculumAdoptionTitle, CurriculumPinReplacements,
    CurriculumReplayStatus, ForkAlphaCommand, ForkAlphaPreviewRequest, GradePolicy,
    LateSubmissionPolicy, LearnerDisclosurePolicy, ObservedAlphaAssignmentSource,
    ObservedAlphaSource, ObservedAssignmentRevision, ObservedBlueprintSource, PointValue,
    RelativeAssignmentSchedule, ReusableAssignmentDefaults, ReusableAssignmentDefinitionInput,
    ReusableAssignmentEntryInput, ReusableFixedQuestionInput, RunPolicies,
    SourceDerivedAssignmentPreviewRequest, UserRole, VariationPolicy,
};
use uuid::Uuid;

use super::*;

mod negative;

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
    CurriculumAdoptionIdempotencyKey::parse(value).expect("idempotency key")
}

#[tokio::test]
async fn adoption_operations_materialize_meaning_and_keep_rollover_learner_state_empty() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(120_001));
    let context = TenantContext::from_authenticated_session(tenant);
    let actor = UserId::from_uuid(Uuid::from_u128(120_002));
    let session = SessionTokenHash::compute(b"curriculum-adoption-instructor");
    let record = super::super::catalog_search_tests::record(120_003);
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
        state.accounts.insert(
            actor,
            AccountRecord {
                user: actor,
                email: AuthenticationEmail::parse("adoption@example.edu").expect("email"),
                display_name: "Elena Instructor".into(),
                platform_roles: Vec::new(),
                created_at: ActivityTimestamp::from_unix_millis(0),
                updated_at: ActivityTimestamp::from_unix_millis(0),
            },
        );
    }
    store
        .create_session(
            session,
            SessionSubject::new(tenant, actor, "Elena", vec![UserRole::Instructor])
                .expect("subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("session");
    let alpha_input = AlphaCourseDefinitionInput {
        title: "Biochemistry alpha".into(),
        modules: vec![AlphaCourseModuleInput {
            label: "Week one".into(),
            definitions: vec![definition(question_id.clone())],
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
                    definition: definition(question_id),
                },
            },
        )
        .await
        .expect("Blueprint source");
    let term = CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago").expect("term");
    let alpha_preview = store
        .preview_alpha_instantiation(
            context,
            session,
            AlphaInstantiationPreviewRequest {
                source: ObservedAlphaSource {
                    reference: alpha.reference,
                    revision: alpha.revision,
                },
                title: CurriculumAdoptionTitle::parse("Fall biochemistry").expect("title"),
                target_term: term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("Alpha preview");
    let alpha_command = AlphaInstantiationCommand::from_preview(&alpha_preview, key("alpha"))
        .expect("corrected preview");
    let applied = store
        .apply_alpha_instantiation(context, session, alpha_command.clone())
        .await
        .expect("Alpha apply");
    {
        let state = store.read_state().expect("state");
        let course = resolve_course(&state, tenant, applied.course).expect("course");
        let adoption = &state.curriculum_course_adoptions[&(tenant, course)];
        assert_eq!(adoption.payload.title(), "Fall biochemistry");
        assert!(state.course_memberships.values().any(|membership| {
            membership.course == course && membership.user == actor
        }));
    }
    let replayed = store
        .apply_alpha_instantiation(context, session, alpha_command)
        .await
        .expect("Alpha replay");
    assert_eq!(replayed.replay, CurriculumReplayStatus::Replayed);

    let imports = store
        .inspect_curriculum_imports(context, session, applied.course)
        .await
        .expect("import inspection")
        .expect("course import");
    let blueprint_preview = store
        .preview_blueprint_instantiation(
            context,
            session,
            BlueprintInstantiationPreviewRequest {
                source: ObservedBlueprintSource {
                    reference: blueprint.reference,
                    revision: blueprint.revision,
                },
                course: applied.course,
                target_term: term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("Blueprint preview");
    let blueprint_applied = store
        .apply_blueprint_instantiation(
            context,
            session,
            BlueprintInstantiationCommand::from_preview(&blueprint_preview, key("blueprint"))
                .expect("corrected preview"),
        )
        .await
        .expect("Blueprint apply");
    {
        let state = store.read_state().expect("state");
        let assignment = state.assignments_by_reference[&(tenant, blueprint_applied.assignment)];
        assert_eq!(state.assignments[&(tenant, assignment)].course_id,
            resolve_course(&state, tenant, applied.course).expect("course"));
        assert_eq!(state.curriculum_import_baselines[&(tenant, assignment)].payload.title(),
            "Protein structure practice");
    }

    let mut revised_alpha = alpha_input;
    revised_alpha.modules[0].definitions[0].title = "Revised protein structure".into();
    let alpha_v2 = store
        .replace_alpha_course(
            context,
            session,
            ReplaceAlphaCourseCommand {
                reference: Some(alpha.reference),
                expected_revision: Some(alpha.revision),
                definition: revised_alpha,
            },
        )
        .await
        .expect("new source revision");
    let imported = &imports.assignments[0];
    let assignment_revision = {
        let state = store.read_state().expect("state");
        let assignment = state.assignments_by_reference[&(tenant, imported.assignment)];
        state.assignment_revisions[&(tenant, assignment)]
    };
    let fast_preview = store
        .preview_assignment_fast_forward(
            context,
            session,
            AssignmentFastForwardPreviewRequest {
                course: applied.course,
                assignment: ObservedAssignmentRevision {
                    assignment: imported.assignment,
                    revision: assignment_revision,
                },
                import_revision: imported.revision,
                source: AssignmentDefinitionSourceView::Alpha(
                    ObservedAlphaAssignmentSource::new(
                        ObservedAlphaSource {
                            reference: alpha_v2.reference,
                            revision: alpha_v2.revision,
                        },
                        0,
                        0,
                    )
                    .expect("Alpha assignment source"),
                ),
            },
        )
        .await
        .expect("fast-forward preview");
    let gradebook_revision_before_fast_forward = {
        let state = store.read_state().expect("state");
        let course = resolve_course(&state, tenant, applied.course).expect("course");
        state.course_grade_schemes[&(tenant, course)].revision
    };
    let fast_forward = store
        .apply_assignment_fast_forward(
            context,
            session,
            AssignmentFastForwardCommand::from_preview(&fast_preview, key("fast"))
                .expect("eligible fast-forward"),
        )
        .await
        .expect("fast-forward apply");
    {
        let state = store.read_state().expect("state");
        let assignment = state.assignments_by_reference[&(tenant, fast_forward.assignment)];
        let course = resolve_course(&state, tenant, applied.course).expect("course");
        assert_eq!(state.assignments[&(tenant, assignment)].title, "Revised protein structure");
        assert!(fast_forward.import_revision.value() > imported.revision.value());
        assert!(state.course_grade_schemes[&(tenant, course)].revision > gradebook_revision_before_fast_forward);
    }

    let source_preview = store
        .preview_source_derived_assignment(
            context,
            session,
            SourceDerivedAssignmentPreviewRequest {
                course: applied.course,
                source: AssignmentDefinitionSourceView::Blueprint(ObservedBlueprintSource {
                    reference: blueprint.reference,
                    revision: blueprint.revision,
                }),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("source-derived preview");
    let derived = store
        .create_source_derived_assignment(
            context,
            session,
            CreateSourceDerivedAssignmentCommand::from_preview(&source_preview, key("derived"))
                .expect("corrected preview"),
        )
        .await
        .expect("source-derived apply");
    {
        let state = store.read_state().expect("state");
        let assignment = state.assignments_by_reference[&(tenant, derived.assignment)];
        assert_eq!(state.assignments[&(tenant, assignment)].title, "Protein structure practice");
    }

    let witness = {
        let state = store.read_state().expect("state");
        let course = resolve_course(&state, tenant, applied.course).expect("course");
        course_witness(&state, tenant, course).expect("witness")
    };
    let shifted_term = CourseTerm::from_parts("2027-01-11", "2027-05-08", "America/Chicago")
        .expect("shifted term");
    let shift_preview = store
        .preview_course_term_shift(
            context,
            session,
            CourseTermShiftPreviewRequest {
                witness,
                target_term: shifted_term.clone(),
            },
        )
        .await
        .expect("shift preview");
    let shifted = store
        .apply_course_term_shift(
            context,
            session,
            CourseTermShiftCommand::from_preview(&shift_preview, key("shift"))
                .expect("corrected preview"),
        )
        .await
        .expect("shift apply");
    assert_eq!(shifted.term, shifted_term);

    let shifted_witness = {
        let state = store.read_state().expect("state");
        let course = resolve_course(&state, tenant, applied.course).expect("course");
        course_witness(&state, tenant, course).expect("witness")
    };
    let rollover_preview = store
        .preview_course_rollover(
            context,
            session,
            CourseRolloverPreviewRequest {
                witness: shifted_witness,
                title: CurriculumAdoptionTitle::parse("Spring biochemistry").expect("title"),
                target_term: shifted_term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("rollover preview");
    let rollover_command = CourseRolloverCommand::from_preview(&rollover_preview, key("rollover"))
        .expect("corrected preview");
    let rollover = store
        .apply_course_rollover(context, session, rollover_command.clone())
        .await
        .expect("rollover apply");
    {
        let state = store.read_state().expect("state");
        let destination = resolve_course(&state, tenant, rollover.course).expect("destination course");
        let copied = &state.curriculum_course_adoptions[&(tenant, destination)];
        assert_eq!(copied.payload.title(), "Spring biochemistry");
        assert!(copied.assignments.iter().all(|assignment| matches!(
            &state.curriculum_import_envelopes[&(tenant, *assignment)].source,
            super::state::StoredCurriculumSource::RolloverAssignment { source_course, .. }
                if *source_course == applied.course
        )));
        assert!(!state.course_memberships.values().any(|membership| membership.course == destination && membership.student.is_some()));
    }

    let later_assignment = store
        .preview_blueprint_instantiation(
            context,
            session,
            BlueprintInstantiationPreviewRequest {
                source: ObservedBlueprintSource {
                    reference: blueprint.reference,
                    revision: blueprint.revision,
                },
                course: rollover.course,
                target_term: shifted_term,
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("later ordinary assignment preview");
    store
        .apply_blueprint_instantiation(
            context,
            session,
            BlueprintInstantiationCommand::from_preview(&later_assignment, key("rollover-later"))
                .expect("corrected preview"),
        )
        .await
        .expect("later ordinary assignment");
    assert_eq!(
        store
            .apply_course_rollover(context, session, rollover_command.clone())
            .await
            .expect("rollover replay ignores later assignments")
            .replay,
        CurriculumReplayStatus::Replayed
    );

    let fork_preview = store
        .preview_fork_alpha(
            context,
            session,
            ForkAlphaPreviewRequest {
                source: ObservedAlphaSource {
                    reference: alpha_v2.reference,
                    revision: alpha_v2.revision,
                },
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("fork preview");
    let fork = store
        .apply_fork_alpha(
            context,
            session,
            ForkAlphaCommand::from_preview(&fork_preview, key("fork")).expect("corrected preview"),
        )
        .await
        .expect("fork apply");
    {
        let state = store.read_state().expect("state");
        assert_ne!(fork.alpha, alpha.reference);
        assert_eq!(state.curriculum_alpha_fork_lineage[&fork.alpha].source.reference, alpha.reference);
    }

    let missing_evidence = {
        let mut state = store.write_state().expect("state");
        let rollover_id = resolve_course(&state, tenant, rollover.course).expect("rollover course");
        let adopted_assignment =
            state.curriculum_course_adoptions[&(tenant, rollover_id)].assignments[0];
        state.curriculum_assignment_adoption_evidence.remove(&(
            tenant,
            key("rollover"),
            adopted_assignment,
        ));
        rollover_id
    };
    assert!(matches!(
        store.inspect_curriculum_imports(context, session, rollover.course).await,
        Err(StoreError::Unavailable(_))
    ));
    assert!(matches!(
        store
            .apply_course_rollover(context, session, rollover_command)
            .await,
        Err(StoreError::Unavailable(_))
    ));

    let state = store.read_state().expect("state");
    let rollover_id = missing_evidence;
    assert!(!state.enrollments.values().any(|enrollment| {
        state
            .assignments
            .get(&(tenant, enrollment.assignment))
            .is_some_and(|assignment| assignment.course_id == rollover_id)
    }));
    assert!(!state.runs.values().any(|run| run.tenant == tenant));
}
