//! Connected calculated-Gradebook service oracle.

use super::*;
use learning_data_access::{
    CalculatedGradebookRequest, CalculatedGradebookResult, CatalogStore, FlatGradingCapability,
    GradebookFilter, GradebookReloadReason, IssueQuestionAttemptCommand,
    IssuedQuestionFamilyWitnessV1, IssuedQuestionSnapshotV1, NativeExecutionEnvelopeCapability,
    PageRequest, PageSize, PresentationCapability, QtiGradingCapability, StudentWorkRoutingBinding,
    SubmissionIdempotencyKey, SubmitQuestionAttemptCommand, WebworkGradingCapability,
};
use question_model::{
    AssignmentId, AttemptProvenance, AttemptResult, FeedbackContent, GradePolicy,
    ImplementationVersion, ProblemVersionRef, QuestionAttemptId, RunId, StudentResponse,
};

pub(super) async fn postgres_calculated_gradebook_page_uses_roster_structure_and_live_witnesses() {
    let runtime = load_acceptance_runtime();
    let pool = lazy_pool(runtime.admin_url().expose()).expect("PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("migrated schema");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x67; 32]);
    let tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id());
    let token = session(&store, tenant, instructor).await;
    let course = create_fixture_course(&store, context, tenant, instructor).await;
    let question = publish_fixture_question(&store, context, tenant, instructor).await;
    let mut assignments = [
        numeric_assignment(tenant, course, question, "Calculated one", 10),
        numeric_assignment(tenant, course, question, "Calculated two", 10),
    ];
    assignments[1].policies.grade = GradePolicy::InstructorSelected;
    let assignment_ids = assignments.each_ref().map(|assignment| assignment.id);
    for assignment in assignments {
        create_published_assignment(
            &store,
            context,
            instructor,
            assignment,
            question_model::BaseAssignmentPolicy::default(),
        )
        .await
        .expect("published calculated assignment");
    }
    let students = [UserId::from_uuid(id()), UserId::from_uuid(id())];
    for (index, student) in students.iter().copied().enumerate() {
        store
            .upsert_course_member(
                context,
                instructor,
                UpsertCourseMember {
                    course,
                    user: student,
                    display_name: format!("Calculated Student {index}"),
                    roster_contact: Some(CourseRosterContact {
                        email: learning_data_access::AuthenticationEmail::parse(&format!(
                            "calculated-{index}@roosevelt.edu"
                        ))
                        .expect("roster email"),
                        roster_id: CourseRosterId::parse(&format!("9100000{index}"))
                            .expect("roster ID"),
                    }),
                },
            )
            .await
            .expect("active calculated Student");
        for assignment in assignment_ids {
            let materialization = store
                .issue_assignment_entitlement(
                    context,
                    MaterializeAssignmentEntitlementCommand::for_instructor_action(
                        student,
                        course,
                        assignment,
                        instructor,
                        EntitlementPurpose::InstructorIssue,
                    )
                    .expect("typed instructor issue"),
                )
                .await
                .expect("calculated enrollment");
            let AssignmentEntitlementMaterialization::Granted(_) = materialization else {
                panic!("active calculated Student receives an enrollment");
            };
        }
        set_summary_scores(
            &pool,
            tenant,
            student,
            &[(assignment_ids[0], Some(0.8)), (assignment_ids[1], None)],
        )
        .await;
    }
    let selected_run = complete_run_choice(
        &store,
        context,
        students[0],
        course,
        assignment_ids[0],
        question,
        "gradebook-selected-run",
    )
    .await;
    complete_run_choice(
        &store,
        context,
        students[0],
        course,
        assignment_ids[1],
        question,
        "gradebook-choice-one",
    )
    .await;
    complete_run_choice(
        &store,
        context,
        students[0],
        course,
        assignment_ids[1],
        question,
        "gradebook-choice-two",
    )
    .await;
    set_summary_scores(
        &pool,
        tenant,
        students[0],
        &[(assignment_ids[0], Some(0.8)), (assignment_ids[1], None)],
    )
    .await;
    let page_size = PageSize::new(1).expect("bounded page size");
    let request = |filter, page| CalculatedGradebookRequest { filter, page };
    let CalculatedGradebookResult::Page(first) = store
        .calculated_gradebook_page(
            context,
            token,
            course,
            request(GradebookFilter::All, PageRequest::first(page_size)),
        )
        .await
        .expect("first calculated Gradebook page")
    else {
        panic!("first calculated page is not a reload");
    };
    assert_eq!(first.rows.len(), 1);
    assert_eq!(first.scoring_witnesses.len(), 2);
    assert_eq!(first.rows[0].outcome.rounded_score, Some(0.4));
    assert!(first.scoring_witnesses.iter().all(|witness| {
        first.rows[0]
            .assignment_cells
            .iter()
            .any(|cell| cell.assignment == witness.assignment)
            && witness.status == question_model::ScoringStatus::Current
            && witness.generation == question_model::ScoringGeneration::INITIAL
    }));
    assert!(first.rows[0].assignment_cells.iter().any(|cell| {
        cell.title == "Calculated one"
            && matches!(
                cell.inspection_choice,
                learning_data_access::AssignmentInspectionChoice::SelectedRun { run, .. }
                    if run == selected_run
            )
    }));
    assert!(first.rows[0].assignment_cells.iter().any(|cell| {
        cell.title == "Calculated two"
            && matches!(
                cell.inspection_choice,
                learning_data_access::AssignmentInspectionChoice::ChooseRun {
                    completed_run_count: 2
                }
            )
    }));
    let continuation = first.next_cursor.clone().expect("second roster page");
    let CalculatedGradebookResult::Page(second) = store
        .calculated_gradebook_page(
            context,
            token,
            course,
            request(
                GradebookFilter::All,
                PageRequest::after(continuation.clone(), page_size),
            ),
        )
        .await
        .expect("second calculated Gradebook page")
    else {
        panic!("continuation is structurally valid");
    };
    assert_ne!(first.rows[0].membership, second.rows[0].membership);
    assert!(second.rows[0].assignment_cells.iter().all(|cell| {
        matches!(
            cell.inspection_choice,
            learning_data_access::AssignmentInspectionChoice::NoSubmittedRun
        )
    }));
    let assignment = first.rows[0].assignment_cells[0].assignment;
    let CalculatedGradebookResult::Page(assignment_page) = store
        .calculated_gradebook_page(
            context,
            token,
            course,
            request(
                GradebookFilter::Assignment(assignment),
                PageRequest::first(page_size),
            ),
        )
        .await
        .expect("assignment-scoped roster page")
    else {
        panic!("assignment page is not a reload");
    };
    assert_eq!(assignment_page.rows[0].assignment_cells.len(), 1);
    assert_eq!(assignment_page.rows[0].outcome, first.rows[0].outcome);
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: UserId::from_uuid(id()),
                display_name: "Calculated structural change".into(),
                roster_contact: Some(CourseRosterContact {
                    email: learning_data_access::AuthenticationEmail::parse(
                        "calculated-structural@roosevelt.edu",
                    )
                    .expect("roster email"),
                    roster_id: CourseRosterId::parse("91000009").expect("roster ID"),
                }),
            },
        )
        .await
        .expect("roster structure changes");
    assert_eq!(
        store
            .calculated_gradebook_page(
                context,
                token,
                course,
                request(
                    GradebookFilter::All,
                    PageRequest::after(continuation, page_size),
                ),
            )
            .await,
        Ok(CalculatedGradebookResult::ReloadRequired {
            reason: GradebookReloadReason::RosterChanged,
        })
    );
    set_summary_scores(
        &pool,
        tenant,
        students[0],
        &[(assignment_ids[0], Some(0.8))],
    )
    .await;
    let assignment = store
        .get_assignment_for_edit(context, assignment_ids[0])
        .await
        .expect("recalculation assignment query")
        .expect("recalculation assignment");
    store
        .recalculate_instructor_assignment(
            context,
            RecalculateAssignmentCommand {
                tenant,
                session: token,
                course,
                assignment: assignment_ids[0],
                action: GradingOperationActionId::from_uuid(id()),
                expected_assignment_revision: assignment.revision,
            },
        )
        .await
        .expect("production recalculation capability");
    let CalculatedGradebookResult::Page(recalculating) = store
        .calculated_gradebook_page(
            context,
            token,
            course,
            request(GradebookFilter::All, PageRequest::first(page_size)),
        )
        .await
        .expect("recalculating page")
    else {
        panic!("recalculating page is not a reload");
    };
    assert!(
        recalculating
            .scoring_witnesses
            .iter()
            .any(|witness| witness.status == question_model::ScoringStatus::Recalculating)
    );
    assert_eq!(
        recalculating.rows[0].assignment_cells[0].selected_score,
        None
    );
    let export = store
        .create_course_grade_export(context, token, course)
        .await
        .expect("export remains separate");
    assert_eq!(export.audit.row_count, 3);
    fail_recalculation_job(&store, tenant, assignment_ids[0]).await;
}

fn run_provenance(label: &str) -> AttemptProvenance {
    AttemptProvenance {
        adapter: ImplementationVersion {
            id: "calculated-gradebook-live".to_string(),
            version: "1".to_string(),
        },
        renderer: None,
        generator: None,
        source_artifact: None,
        asset_objects: Vec::new(),
        grading: ImplementationVersion {
            id: "calculated-gradebook-live-grading".to_string(),
            version: "1".to_string(),
        },
        rendered_question_sha256: format!("calculated-gradebook-{label}"),
    }
}

async fn complete_run_choice(
    store: &PostgresStore,
    context: TenantContext,
    student: UserId,
    course: CourseId,
    assignment: AssignmentId,
    reference: ProblemVersionRef,
    key: &str,
) -> question_model::RunReference {
    let binding = StudentWorkRoutingBinding::new(course, assignment);
    let run = store
        .start_or_resume_run(context, student, binding, RunId::from_uuid(id()))
        .await
        .expect("start ordinary calculated-Gradebook Student run");
    let question = store
        .get_catalog_problem(context, reference)
        .await
        .expect("read calculated-Gradebook question")
        .expect("published calculated-Gradebook question")
        .question;
    let issued_question_snapshot = IssuedQuestionSnapshotV1::new(
        question,
        IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("construct calculated-Gradebook issue snapshot");
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                binding,
                attempt: QuestionAttemptId::from_uuid(id()),
                run: run.id,
                assignment_position: 0,
                problem: reference.problem,
                question_version: reference.version,
                issued_question_snapshot,
                seed: 18,
                presentation_capability: PresentationCapability::NotApplicable,
                presentation: None,
                presentation_snapshot: None,
                grading_envelope: None,
                native_execution_envelope_capability: NativeExecutionEnvelopeCapability::Required,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability: WebworkGradingCapability::NotApplicable,
                qti_grading: None,
                qti_grading_capability: QtiGradingCapability::NotApplicable,
                parameter_hash: format!("calculated-gradebook-{key}"),
                provenance: run_provenance(key),
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("issue ordinary calculated-Gradebook attempt");
    let idempotency = SubmissionIdempotencyKey::parse(key).expect("valid fixture idempotency key");
    let submission = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                binding,
                attempt: attempt.id,
                response: StudentResponse::Numeric { value: 18.0 },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: idempotency,
            },
        )
        .await
        .expect("complete ordinary calculated-Gradebook run");
    assert!(
        submission.run.completed_at.is_some(),
        "one submitted item completes the answer-all run"
    );
    submission.run.reference
}
