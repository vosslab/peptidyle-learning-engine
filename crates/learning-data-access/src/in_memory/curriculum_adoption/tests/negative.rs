use crate::{
    AccountRecord, AuthenticationEmail, CurriculumAdoptionStore, ReplaceAlphaCourseCommand,
    ReplaceUnissuedAssignmentDefinitionCommand, ReusableCurriculumStore, SessionLifetime,
    SessionStore, SessionSubject, Store,
};
use question_model::{
    ActivityTimestamp, AlphaInstantiationCommand, AlphaInstantiationPreviewRequest,
    AssignmentDefinitionSourceView, AssignmentEnrollment, AssignmentFastForwardDecision,
    AssignmentFastForwardPreviewRequest, AssignmentId, AssignmentRun,
    BlueprintInstantiationCommand, BlueprintInstantiationPreviewRequest, CourseReference,
    CourseScheduleWitness, CourseTerm, CourseTermShiftCommand, CourseTermShiftPreviewRequest,
    CurriculumAdoptionTitle, CurriculumPinReplacement, CurriculumPinReplacements, EnrollmentId,
    ObservedAlphaAssignmentSource, ObservedAlphaSource, ObservedAssignmentRevision, RunId, RunMode,
    RunReference, SourceDerivedAssignmentPreviewRequest, StudentId, TenantId, UserId, UserRole,
    VariationPolicy,
};
use uuid::Uuid;

use super::super::{course_witness, resolve_course};
use crate::{SessionTokenHash, StoreError, TenantContext};

mod dst;
mod integrity;
use super::adoption_inputs::key;
use super::scenario::AdoptionScenario;
type Fixture = AdoptionScenario;

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
        (
            course,
            state.curriculum_adoption.whole_course_adoptions[&(fixture.tenant, course)]
                .destination_assignments
                .clone(),
        )
    };
    {
        let mut state = fixture.store.write_state().expect("state");
        let membership = state
            .active_course_membership_by_user
            .remove(&(fixture.tenant, course, fixture.actor))
            .expect("direct instructor membership");
        state
            .course_memberships
            .get_mut(&(fixture.tenant, membership))
            .expect("membership")
            .revoked_at = Some(ActivityTimestamp::from_unix_millis(1));
    }
    assert_eq!(
        fixture
            .store
            .apply_alpha_instantiation(fixture.context, fixture.session, command)
            .await,
        Err(StoreError::NotFound)
    );
    let state = fixture.store.read_state().expect("state");
    assert!(adoption_assignments.iter().all(|assignment| {
        state
            .curriculum_adoption
            .assignment_evidence
            .contains_key(&(fixture.tenant, key("receipt-authority"), *assignment))
    }));
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
    let before_stale = b2_snapshot(&fixture);
    assert_eq!(
        fixture
            .store
            .apply_course_term_shift(fixture.context, fixture.session, stale_command)
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(b2_snapshot(&fixture), before_stale);

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
    let before_issued = b2_snapshot(&fixture);
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
    assert_eq!(b2_snapshot(&fixture), before_issued);
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
        let baseline_title = state.curriculum_adoption.import_records
            [&(fixture.tenant, assignment_id)]
            .baseline
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
    let before_reauthorization = b2_snapshot(&fixture);
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
    assert_eq!(b2_snapshot(&fixture), before_reauthorization);

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
    let corrected_applied = fixture
        .store
        .apply_blueprint_instantiation(
            fixture.context,
            fixture.session,
            BlueprintInstantiationCommand::from_preview(&corrected_preview, key("replacement"))
                .expect("corrected command"),
        )
        .await
        .expect("corrected replacement applies");
    let state = fixture.store.read_state().expect("state");
    let assignment =
        state.assignments_by_reference[&(fixture.tenant, corrected_applied.assignment)];
    let replacement = state
        .published
        .values()
        .find(|record| record.question_id == fixture.replacement_question)
        .expect("replacement publication");
    assert!(
        state.assignments[&(fixture.tenant, assignment)]
            .items
            .iter()
            .any(|item| {
                item.reference.problem == replacement.problem
                    && item.reference.version == replacement.version
            })
    );
    let evidence = &state.curriculum_adoption.assignment_evidence
        [&(fixture.tenant, key("replacement"), assignment)];
    assert!(matches!(
        evidence.baseline.payload.entries().first(),
        Some(question_model::curriculum_adoption::CurriculumSemanticAssignmentEntry::Fixed { reference, .. })
            if reference.problem == replacement.problem && reference.version == replacement.version
    ));
}

pub(super) fn witness(fixture: &Fixture, course: CourseReference) -> CourseScheduleWitness {
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

/// The B2-owned durable records that must be unchanged after a refused command.
pub(super) fn b2_snapshot(fixture: &Fixture) -> impl PartialEq + std::fmt::Debug {
    let state = fixture.store.read_state().expect("state");
    (
        state.curriculum_adoption.clone(),
        state.course_schedule_revisions.clone(),
        state.course_memberships.clone(),
        state.assignments.clone(),
        state.assignment_revisions.clone(),
        state.enrollments.clone(),
        state.runs.clone(),
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
