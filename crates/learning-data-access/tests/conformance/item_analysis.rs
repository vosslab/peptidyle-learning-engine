//! Memory-only current item-analysis contract checks.

use super::*;

struct AnalysisFixture {
    context: TenantContext,
    foreign_context: TenantContext,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    instructor: UserId,
    student: UserId,
    enrollment: EnrollmentId,
    primary: ProblemVersionRef,
    secondary: ProblemVersionRef,
    secondary_item: AssignmentItemId,
    instructor_session: SessionTokenHash,
    sysadmin_session: SessionTokenHash,
    student_session: SessionTokenHash,
    outsider_session: SessionTokenHash,
}

async fn analysis_fixture(store: &MemoryStore) -> AnalysisFixture {
    let tenant = TenantId::from_uuid(uuid(80_001));
    let foreign_tenant = TenantId::from_uuid(uuid(80_002));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let instructor = UserId::from_uuid(uuid(80_003));
    let sysadmin = UserId::from_uuid(uuid(80_004));
    let student = UserId::from_uuid(uuid(80_005));
    let outsider = UserId::from_uuid(uuid(80_006));
    let workspace = WorkspaceId::from_uuid(uuid(80_007));
    let course = CourseId::from_uuid(uuid(80_008));
    let assignment = AssignmentId::from_uuid(uuid(80_009));
    let course_creation_authority =
        sysadmin_course_creation_authority(store, tenant, course, instructor).await;
    let primary = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(80_011)),
        version: VersionId::from_uuid(uuid(80_012)),
    };
    let secondary = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(80_013)),
        version: VersionId::from_uuid(uuid(80_014)),
    };
    for reference in [primary, secondary] {
        let draft = DraftRecord {
            tenant,
            question: draft_question(workspace),
            derived_from: None,
        };
        let saved = store
            .upsert_draft(context, instructor, None, draft.clone())
            .await
            .expect("analysis fixture draft");
        store
            .publish_draft(
                context,
                instructor,
                PublishDraftCommand {
                    expected_draft: draft,
                    expected_revision: saved.revision,
                    publication: reference,
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher: instructor,
                    scope: PublicationScope::Public,
                    byline: reviewed_byline(),
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await
            .expect("analysis fixture publication");
    }
    store
        .create_course(
            context,
            learning_data_access::CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Item analysis course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: course_creation_authority,
            },
        )
        .await
        .expect("analysis fixture course");
    store
        .upsert_course_member(
            context,
            instructor,
            learning_data_access::UpsertCourseMember {
                course,
                user: student,
                display_name: "Analysis Student".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("analysis Student membership");
    let mut policy = policies();
    policy.completion = CompletionRequirement::AnswerAll;
    let items = fixed_items(vec![primary, secondary]);
    let secondary_item = items[1].id;
    store
        .create_assignment_with_default_policy(
            context,
            instructor,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Item analysis assignment".to_string(),
                lifecycle: question_model::AssignmentLifecycle::Published,
                instructions: question_model::AssignmentInstructions::default(),
                audience: question_model::AssignmentAudience::CourseWide,
                items,
                selection_groups: Vec::new(),
                disclosure_policy: question_model::StudentDisclosurePolicy::default(),
                policies: policy,
            },
        )
        .await
        .expect("analysis fixture assignment");
    let enrollment = match store
        .issue_assignment_entitlement(
            context,
            learning_data_access::MaterializeAssignmentEntitlementCommand::for_instructor_action(
                student,
                course,
                assignment,
                instructor,
                question_model::EntitlementPurpose::InstructorIssue,
            )
            .expect("valid explicit instructor issue"),
        )
        .await
        .expect("analysis fixture instructor issue")
    {
        learning_data_access::AssignmentEntitlementMaterialization::Granted(receipt) => {
            receipt.enrollment.id
        }
        learning_data_access::AssignmentEntitlementMaterialization::Denied(reason) => {
            panic!("fixture instructor issue denied: {reason:?}")
        }
    };

    async fn session(
        store: &MemoryStore,
        tenant: TenantId,
        user: UserId,
        roles: Vec<UserRole>,
        key: &'static [u8],
    ) -> SessionTokenHash {
        let token = SessionTokenHash::compute(key);
        store
            .create_session(
                token,
                SessionSubject::new(tenant, user, "Item analysis fixture", roles)
                    .expect("fixture session subject"),
                SessionLifetime::from_seconds(60).expect("fixture session lifetime"),
            )
            .await
            .expect("fixture session");
        token
    }

    AnalysisFixture {
        context,
        foreign_context,
        tenant,
        course,
        assignment,
        instructor,
        student,
        enrollment,
        primary,
        secondary,
        secondary_item,
        instructor_session: session(
            store,
            tenant,
            instructor,
            vec![UserRole::Instructor],
            b"analysis-instructor",
        )
        .await,
        sysadmin_session: session(
            store,
            tenant,
            sysadmin,
            vec![UserRole::Sysadmin],
            b"analysis-sysadmin",
        )
        .await,
        student_session: session(
            store,
            tenant,
            student,
            vec![UserRole::Student],
            b"analysis-student",
        )
        .await,
        outsider_session: session(
            store,
            tenant,
            outsider,
            vec![UserRole::Instructor],
            b"analysis-outsider",
        )
        .await,
    }
}

fn provenance(label: &str) -> AttemptProvenance {
    AttemptProvenance {
        adapter: implementation("native"),
        renderer: None,
        generator: None,
        source_artifact: None,
        asset_objects: Vec::new(),
        grading: implementation("numeric"),
        rendered_question_sha256: format!("item-analysis-rendered-{label}"),
    }
}

async fn issue(
    store: &MemoryStore,
    fixture: &AnalysisFixture,
    run: RunId,
    position: u32,
    reference: ProblemVersionRef,
    id: u128,
) -> QuestionAttempt {
    let question = store
        .get_catalog_problem(fixture.context, reference)
        .await
        .expect("analysis fixture catalog question")
        .expect("analysis fixture publication")
        .question;
    let issued_question_snapshot = learning_data_access::IssuedQuestionSnapshotV1::new(
        question,
        learning_data_access::IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("analysis fixture issued question snapshot");
    store
        .issue_or_resume_question_attempt(
            fixture.context,
            IssueQuestionAttemptCommand {
                actor: fixture.student,
                binding: StudentWorkRoutingBinding::new(fixture.course, fixture.assignment),
                attempt: QuestionAttemptId::from_uuid(uuid(id)),
                run,
                assignment_position: position,
                problem: reference.problem,
                question_version: reference.version,
                issued_question_snapshot,
                seed: u64::try_from(id).expect("fixture seed"),
                presentation_capability: PresentationCapability::NotApplicable,
                presentation: None,
                presentation_snapshot: None,
                grading_envelope: None,
                native_execution_envelope_capability:
                    learning_data_access::NativeExecutionEnvelopeCapability::Required,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability: WebworkGradingCapability::NotApplicable,
                qti_grading: None,
                qti_grading_capability: learning_data_access::QtiGradingCapability::NotApplicable,
                parameter_hash: format!("item-analysis-parameters-{id}"),
                provenance: provenance(&id.to_string()),
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("analysis fixture issue")
}

async fn submit_auto(
    store: &MemoryStore,
    fixture: &AnalysisFixture,
    attempt: QuestionAttemptId,
    key: &str,
) {
    let accepted = store
        .accept_automated_submission(
            fixture.context,
            AcceptedSubmissionCommand {
                actor: fixture.student,
                course: fixture.course,
                assignment: fixture.assignment,
                attempt,
                idempotency_key: SubmissionIdempotencyKey::parse(key).expect("fixture key"),
                response: StudentResponse::Numeric { value: 42.0 },
                execution_job: JobId::from_uuid(uuid(90_000 + attempt.as_uuid().as_u128())),
            },
        )
        .await
        .expect("accepted automated submission");
    let claim = store
        .claim_next_accepted_submission_execution(
            WorkerId::from_uuid(uuid(91_000 + attempt.as_uuid().as_u128())),
            JobLeaseDuration::from_seconds(30).expect("analysis worker lease"),
        )
        .await
        .expect("analysis worker claim")
        .expect("accepted submission is claimable");
    assert_eq!(claim.submission, accepted.submission);
    assert_eq!(
        store
            .commit_or_fail_accepted_submission_execution(
                fixture.context,
                claim,
                AcceptedSubmissionExecutionOutcome::Evaluated {
                    grade: AcceptedSubmissionGrade {
                        evidence: canonical_attempt_result_json(AttemptResult {
                            correct: true,
                            points_earned: 1.0,
                            points_possible: 1.0,
                        })
                        .expect("canonical automated result"),
                        feedback: FeedbackContent::default(),
                    },
                },
            )
            .await
            .expect("automated worker completion"),
        AcceptedSubmissionExecutionDisposition::Committed
    );
}

async fn submit_pending_auto(
    store: &MemoryStore,
    fixture: &AnalysisFixture,
    attempt: QuestionAttemptId,
    key: &str,
) {
    store
        .accept_automated_submission(
            fixture.context,
            AcceptedSubmissionCommand {
                actor: fixture.student,
                course: fixture.course,
                assignment: fixture.assignment,
                attempt,
                idempotency_key: SubmissionIdempotencyKey::parse(key).expect("fixture key"),
                response: StudentResponse::Numeric { value: 42.0 },
                execution_job: JobId::from_uuid(uuid(92_000 + attempt.as_uuid().as_u128())),
            },
        )
        .await
        .expect("accepted pending automated submission");
}

async fn submit_auto_exception(
    store: &MemoryStore,
    fixture: &AnalysisFixture,
    attempt: QuestionAttemptId,
    key: &str,
) {
    submit_pending_auto(store, fixture, attempt, key).await;
    let claim = store
        .claim_next_accepted_submission_execution(
            WorkerId::from_uuid(uuid(93_000 + attempt.as_uuid().as_u128())),
            JobLeaseDuration::from_seconds(30).expect("analysis worker lease"),
        )
        .await
        .expect("analysis worker claim")
        .expect("accepted submission is claimable");
    assert_eq!(
        store
            .commit_or_fail_accepted_submission_execution(
                fixture.context,
                claim,
                AcceptedSubmissionExecutionOutcome::TerminalFailure,
            )
            .await
            .expect("automated worker exception"),
        AcceptedSubmissionExecutionDisposition::Terminal
    );
}

async fn run_analysis_job(
    store: &MemoryStore,
    fixture: &AnalysisFixture,
) -> CourseItemAnalysisCommitOutcome {
    for _ in 0..4 {
        let claim = store
            .claim_next_job(
                &JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(30).expect("analysis lease"),
            )
            .await
            .expect("analysis job claim")
            .expect("analysis job available");
        match claim.payload {
            JobPayload::RecalculateAssignment {
                assignment,
                generation,
            } => {
                let command = AssignmentScoringWorkerCommand {
                    job: claim.id,
                    lease: claim.lease_token,
                    assignment,
                    generation,
                };
                store
                    .prepare_assignment_scoring(fixture.context, command)
                    .await
                    .expect("scoring staging");
                store
                    .commit_assignment_scoring(fixture.context, command)
                    .await
                    .expect("scoring publication");
            }
            JobPayload::RecalculateCourseItemAnalysis {
                assignment,
                generation,
            } => {
                let command = CourseItemAnalysisWorkerCommand {
                    job: claim.id,
                    lease: claim.lease_token,
                    assignment,
                    generation,
                };
                store
                    .prepare_course_item_analysis(fixture.context, command)
                    .await
                    .expect("analysis staging");
                let outcome = store
                    .commit_course_item_analysis(fixture.context, command)
                    .await
                    .expect("analysis publication");
                if outcome == CourseItemAnalysisCommitOutcome::Committed {
                    return outcome;
                }
            }
            other => panic!("unexpected item-analysis dependency job: {other:?}"),
        }
    }
    panic!("item-analysis job was not enqueued after its scoring dependency")
}

async fn run_scoring_job(
    store: &MemoryStore,
    fixture: &AnalysisFixture,
) -> AssignmentScoringCommitOutcome {
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("scoring lease"),
        )
        .await
        .expect("scoring job claim")
        .expect("scoring job available");
    let JobPayload::RecalculateAssignment {
        assignment,
        generation,
    } = claim.payload
    else {
        panic!("expected scoring job")
    };
    let command = AssignmentScoringWorkerCommand {
        job: claim.id,
        lease: claim.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_assignment_scoring(fixture.context, command)
        .await
        .expect("scoring staging");
    store
        .commit_assignment_scoring(fixture.context, command)
        .await
        .expect("scoring publication")
}

async fn current_report(
    store: &MemoryStore,
    fixture: &AnalysisFixture,
) -> domain::item_analysis::CourseItemAnalysisReport {
    store
        .course_item_analysis(
            fixture.context,
            fixture.instructor_session,
            fixture.course,
            fixture.assignment,
        )
        .await
        .expect("analysis read")
        .expect("analysis is current")
}

#[tokio::test]
async fn memory_student_class_statistics_requires_current_s5_and_never_leaks_absent_evidence() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let fixture = analysis_fixture(&store).await;

    assert_eq!(
        store
            .student_class_statistics(
                fixture.context,
                fixture.student,
                fixture.course,
                fixture.assignment,
            )
            .await
            .expect("currently entitled Student"),
        question_model::StudentClassStatistics::InsufficientEvidence
    );
    assert_eq!(
        store
            .student_class_statistics(
                fixture.context,
                UserId::from_uuid(uuid(80_099)),
                fixture.course,
                fixture.assignment,
            )
            .await,
        Err(learning_data_access::StoreError::NotFound)
    );
}

#[tokio::test]
async fn memory_item_analysis_excludes_a_cleared_attempt_without_fabricating_a_bucket() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let fixture = analysis_fixture(&store).await;
    let run = store
        .start_or_resume_run(
            fixture.context,
            fixture.student,
            StudentWorkRoutingBinding::new(fixture.course, fixture.assignment),
            RunId::from_uuid(uuid(80_034)),
        )
        .await
        .expect("analysis run");
    let primary_attempt = issue(&store, &fixture, run.id, 0, fixture.primary, 80_035).await;
    submit_auto(
        &store,
        &fixture,
        primary_attempt.id,
        "analysis-cleared-primary",
    )
    .await;
    let cleared_attempt = issue(&store, &fixture, run.id, 1, fixture.secondary, 80_036).await;
    store
        .clear_attempt(
            fixture.context,
            ClearAttemptCommand {
                action: AttemptSupportActionId::from_uuid(uuid(80_037)),
                actor: fixture.instructor,
                attempt: cleared_attempt.id,
            },
        )
        .await
        .expect("clear second current attempt");
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    let report = current_report(&store, &fixture).await;
    let cleared_row = report
        .items
        .iter()
        .find(|row| row.assignment_item == fixture.secondary_item)
        .expect("cleared row");
    assert_eq!(cleared_row.unscored_attempt_count, 0);
    assert_eq!(cleared_row.response_distribution.unanswered, 0);
}

#[tokio::test]
async fn memory_item_analysis_marks_pending_automated_scoring_unscored() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let fixture = analysis_fixture(&store).await;
    let run = store
        .start_or_resume_run(
            fixture.context,
            fixture.student,
            StudentWorkRoutingBinding::new(fixture.course, fixture.assignment),
            RunId::from_uuid(uuid(80_038)),
        )
        .await
        .expect("analysis run");
    let primary_attempt = issue(&store, &fixture, run.id, 0, fixture.primary, 80_039).await;
    submit_auto(
        &store,
        &fixture,
        primary_attempt.id,
        "analysis-pending-primary",
    )
    .await;
    let pending_attempt = issue(&store, &fixture, run.id, 1, fixture.secondary, 80_040).await;
    submit_pending_auto(
        &store,
        &fixture,
        pending_attempt.id,
        "analysis-pending-secondary",
    )
    .await;
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    let report = current_report(&store, &fixture).await;
    let pending_row = report
        .items
        .iter()
        .find(|row| row.assignment_item == fixture.secondary_item)
        .expect("pending row");
    assert!(report.incomplete_scoring);
    assert_eq!(pending_row.unscored_attempt_count, 1);
    assert_eq!(pending_row.response_distribution.unanswered, 0);
    assert_eq!(report.assignment_average_score, None);
}

#[tokio::test]
async fn memory_item_analysis_marks_automated_exception_unscored() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let fixture = analysis_fixture(&store).await;
    let run = store
        .start_or_resume_run(
            fixture.context,
            fixture.student,
            StudentWorkRoutingBinding::new(fixture.course, fixture.assignment),
            RunId::from_uuid(uuid(80_041)),
        )
        .await
        .expect("analysis run");
    let primary_attempt = issue(&store, &fixture, run.id, 0, fixture.primary, 80_042).await;
    submit_auto(
        &store,
        &fixture,
        primary_attempt.id,
        "analysis-exception-primary",
    )
    .await;
    let exception_attempt = issue(&store, &fixture, run.id, 1, fixture.secondary, 80_043).await;
    submit_auto_exception(
        &store,
        &fixture,
        exception_attempt.id,
        "analysis-exception-secondary",
    )
    .await;
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    let report = current_report(&store, &fixture).await;
    let exception_row = report
        .items
        .iter()
        .find(|row| row.assignment_item == fixture.secondary_item)
        .expect("exception row");
    assert!(report.incomplete_scoring);
    assert_eq!(exception_row.unscored_attempt_count, 1);
    assert_eq!(exception_row.response_distribution.unanswered, 0);
    assert_eq!(report.assignment_average_score, None);
}

#[tokio::test]
async fn memory_item_analysis_is_instructor_only_and_report_is_identity_free() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let fixture = analysis_fixture(&store).await;
    let run = store
        .start_or_resume_run(
            fixture.context,
            fixture.student,
            StudentWorkRoutingBinding::new(fixture.course, fixture.assignment),
            RunId::from_uuid(uuid(80_030)),
        )
        .await
        .expect("analysis run");
    let primary_attempt = issue(&store, &fixture, run.id, 0, fixture.primary, 80_031).await;
    submit_auto(&store, &fixture, primary_attempt.id, "analysis-auth-auto").await;
    let unanswered_attempt = issue(&store, &fixture, run.id, 1, fixture.secondary, 80_032).await;
    store
        .force_submit_attempt(
            fixture.context,
            ForceSubmitAttemptCommand {
                action: AttemptSupportActionId::from_uuid(uuid(80_033)),
                actor: fixture.instructor,
                attempt: unanswered_attempt.id,
            },
        )
        .await
        .expect("force submit produces unanswered evidence");
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    let report = current_report(&store, &fixture).await;
    let unanswered_row = report
        .items
        .iter()
        .find(|row| row.assignment_item == fixture.secondary_item)
        .expect("unanswered row");
    assert_eq!(unanswered_row.unanswered_attempt_count, 1);
    assert_eq!(unanswered_row.unscored_attempt_count, 0);
    assert_eq!(unanswered_row.response_distribution.unanswered, 1);
    assert!(!report.incomplete_scoring);
    for (context, session, label) in [
        (fixture.context, fixture.student_session, "student"),
        (
            fixture.context,
            fixture.sysadmin_session,
            "sysadmin without direct instructor membership",
        ),
        (fixture.context, fixture.outsider_session, "outsider"),
        (
            fixture.foreign_context,
            fixture.instructor_session,
            "foreign tenant",
        ),
    ] {
        assert_eq!(
            store
                .course_item_analysis(context, session, fixture.course, fixture.assignment)
                .await,
            Ok(None),
            "{label} cannot enumerate course analysis"
        );
    }
    let serialized = serde_json::to_string(&report).expect("report serialization");
    for private_value in [
        fixture.student.to_string(),
        fixture.enrollment.to_string(),
        run.id.to_string(),
        primary_attempt.id.to_string(),
        unanswered_attempt.id.to_string(),
        "analysis-auth-auto".to_string(),
        "feedback".to_string(),
    ] {
        assert!(
            !serialized.contains(&private_value),
            "course analysis must not serialize private Student, attempt, response, or feedback data: {private_value}"
        );
    }
}

#[tokio::test]
async fn memory_item_analysis_stale_generation_cannot_replace_current_report() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let fixture = analysis_fixture(&store).await;
    store
        .enqueue_job(
            fixture.context,
            EnqueueJob {
                tenant: fixture.tenant,
                payload: JobPayload::RecalculateCourseItemAnalysis {
                    assignment: fixture.assignment,
                    generation: question_model::ScoringGeneration::INITIAL,
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("stale analysis job");
    let stale = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease"),
        )
        .await
        .expect("claim")
        .expect("job");
    let JobPayload::RecalculateCourseItemAnalysis {
        assignment,
        generation,
    } = stale.payload
    else {
        panic!("analysis job")
    };
    let stale_command = CourseItemAnalysisWorkerCommand {
        job: stale.id,
        lease: stale.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_course_item_analysis(fixture.context, stale_command)
        .await
        .expect("stale staging");

    let run = store
        .start_or_resume_run(
            fixture.context,
            fixture.student,
            StudentWorkRoutingBinding::new(fixture.course, fixture.assignment),
            RunId::from_uuid(uuid(80_040)),
        )
        .await
        .expect("run");
    let primary_attempt = issue(&store, &fixture, run.id, 0, fixture.primary, 80_041).await;
    submit_auto(&store, &fixture, primary_attempt.id, "analysis-stale-auto").await;
    let secondary_attempt = issue(&store, &fixture, run.id, 1, fixture.secondary, 80_042).await;
    submit_auto(
        &store,
        &fixture,
        secondary_attempt.id,
        "analysis-stale-second-auto",
    )
    .await;
    assert_eq!(
        store
            .commit_course_item_analysis(fixture.context, stale_command)
            .await,
        Ok(CourseItemAnalysisCommitOutcome::Superseded),
        "prepared analysis from an older scoring generation cannot publish"
    );
    assert_eq!(
        run_scoring_job(&store, &fixture).await,
        AssignmentScoringCommitOutcome::Committed
    );
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    assert_eq!(
        current_report(&store, &fixture)
            .await
            .source_scoring_generation
            .value(),
        3
    );
}

#[tokio::test]
async fn memory_item_analysis_uses_only_each_students_latest_run_when_it_is_active() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let fixture = analysis_fixture(&store).await;
    let completed_run = store
        .start_or_resume_run(
            fixture.context,
            fixture.student,
            StudentWorkRoutingBinding::new(fixture.course, fixture.assignment),
            RunId::from_uuid(uuid(80_050)),
        )
        .await
        .expect("completed fixture run");
    let primary_attempt = issue(
        &store,
        &fixture,
        completed_run.id,
        0,
        fixture.primary,
        80_051,
    )
    .await;
    submit_auto(&store, &fixture, primary_attempt.id, "analysis-old-auto").await;
    let secondary_attempt = issue(
        &store,
        &fixture,
        completed_run.id,
        1,
        fixture.secondary,
        80_052,
    )
    .await;
    submit_auto(
        &store,
        &fixture,
        secondary_attempt.id,
        "analysis-old-second-auto",
    )
    .await;
    assert_eq!(
        run_scoring_job(&store, &fixture).await,
        AssignmentScoringCommitOutcome::Committed
    );
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    assert_eq!(
        current_report(&store, &fixture).await.completed_run_count,
        1
    );

    let latest_run = store
        .start_or_resume_run(
            fixture.context,
            fixture.student,
            StudentWorkRoutingBinding::new(fixture.course, fixture.assignment),
            RunId::from_uuid(uuid(80_054)),
        )
        .await
        .expect("newer run starts after the completed run");
    let _active = issue(&store, &fixture, latest_run.id, 0, fixture.primary, 80_055).await;
    store
        .enqueue_job(
            fixture.context,
            EnqueueJob {
                tenant: fixture.tenant,
                payload: JobPayload::RecalculateCourseItemAnalysis {
                    assignment: fixture.assignment,
                    generation: question_model::ScoringGeneration::new(3)
                        .expect("scoring generation"),
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("active-latest analysis job");
    assert_eq!(
        run_analysis_job(&store, &fixture).await,
        CourseItemAnalysisCommitOutcome::Committed
    );
    let report = current_report(&store, &fixture).await;
    assert_eq!(report.completed_run_count, 0);
    assert_eq!(report.in_progress_run_count, 1);
    assert!(
        report.items.is_empty(),
        "an active newer run suppresses the Student's older completed observations"
    );
}
