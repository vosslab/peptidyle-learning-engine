#![cfg(feature = "postgres")]

#[path = "fixtures/published_assignment.rs"]
mod published_assignment;
use published_assignment::create_published_assignment;

#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, AssignmentScoringCommitOutcome, AssignmentScoringWorkerCommand,
    AssignmentScoringWorkerStore, CatalogStore, CourseRecord, CourseRosterStore,
    CreateCourseCommand, DraftRecord, EvaluationRevision, FlatGradingCapability,
    IssueQuestionAttemptCommand, IssuedQuestionFamilyWitnessV1, IssuedQuestionSnapshotV1,
    JobClaimFilter, JobLeaseDuration, JobPayload, JobStore, LearnerWorkRoutingBinding,
    ManualCredit, ManualGradeActionId, ManualGradingStore, NativeExecutionEnvelopeCapability,
    PresentationCapability, PublishDraftCommand, QtiGradingCapability, SetManualGradeCommand,
    Store, StoreError, SubmissionIdempotencyKey, SubmitPendingManualQuestionAttemptCommand,
    SubmitQuestionAttemptCommand, TenantContext, UpsertCourseMember,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::ResponseDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId, AssignmentScoringMode,
    AttemptProvenance, AttemptResult, AttemptStatus, BackendCapabilities, Capability, CourseId,
    DraftQuestionDefinition, DraftQuestionSource, FeedbackContent, GradingDefinition,
    ImplementationVersion, PointValue, ProblemId, ProblemVersionRef, PublicationScope,
    QuestionAttemptId, QuestionMetadata, QuestionSource, RunId, ScoringStatus, StudentResponse,
    TenantId, UserId, VersionId, WorkspaceId,
};
use uuid::Uuid;

fn fresh_uuid() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn implementation(name: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: name.to_string(),
        version: "1".to_string(),
    }
}

fn provenance(name: &str) -> AttemptProvenance {
    AttemptProvenance {
        adapter: implementation("live-native"),
        renderer: None,
        generator: None,
        source_artifact: None,
        asset_objects: Vec::new(),
        grading: implementation(name),
        rendered_question_sha256: format!("live-rendered-{name}"),
    }
}

fn draft_question(
    workspace: WorkspaceId,
    family: &str,
    title: &str,
    response: ResponseDefinition,
) -> DraftQuestionDefinition {
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Native {
            family: family.to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: format!("Live PostgreSQL fixture: {title}"),
        }],
        response,
        attempt_policy: AttemptPolicy { max_attempts: None },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: title.to_string(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    }
}

async fn publish_question(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
    family: &str,
    title: &str,
    response: ResponseDefinition,
) -> (ProblemVersionRef, IssuedQuestionSnapshotV1) {
    let workspace = WorkspaceId::from_uuid(fresh_uuid());
    let publication = ProblemVersionRef {
        problem: ProblemId::from_uuid(fresh_uuid()),
        version: VersionId::from_uuid(fresh_uuid()),
    };
    let draft = DraftRecord {
        tenant,
        question: draft_question(workspace, family, title, response),
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, instructor, None, draft.clone())
        .await
        .expect("save live manual-grading draft");
    store
        .publish_draft(
            context,
            instructor,
            PublishDraftCommand {
                expected_draft: draft.clone(),
                expected_revision: saved.revision,
                publication,
                published_source: QuestionSource::Native {
                    family: family.to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: instructor,
                scope: PublicationScope::Institution,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish live manual-grading question");
    let snapshot = IssuedQuestionSnapshotV1::new(
        question_model::QuestionDefinition::from_draft(
            draft.question,
            publication.problem,
            publication.version,
            QuestionSource::Native {
                family: family.to_string(),
            },
        ),
        IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("construct live native question snapshot");
    (publication, snapshot)
}

fn assignment_item(reference: ProblemVersionRef, position: u32) -> AssignmentItem {
    AssignmentItem {
        id: AssignmentItemId::from_uuid(fresh_uuid()),
        reference,
        position,
        points_possible: PointValue::from_whole(1),
        delivery_state: AssignmentDeliveryState::Active,
        scoring_mode: AssignmentScoringMode::Normal,
    }
}

struct ManualGradingAttemptFixture<'a> {
    store: &'a PostgresStore,
    context: TenantContext,
    binding: LearnerWorkRoutingBinding,
    student: UserId,
    run: RunId,
}

async fn issue_attempt(
    fixture: &ManualGradingAttemptFixture<'_>,
    reference: ProblemVersionRef,
    issued_question_snapshot: IssuedQuestionSnapshotV1,
    position: u32,
    predecessor: Option<QuestionAttemptId>,
) -> question_model::QuestionAttempt {
    fixture
        .store
        .issue_or_resume_question_attempt(
            fixture.context,
            IssueQuestionAttemptCommand {
                actor: fixture.student,
                binding: fixture.binding,
                attempt: QuestionAttemptId::from_uuid(fresh_uuid()),
                run: fixture.run,
                assignment_position: position,
                problem: reference.problem,
                question_version: reference.version,
                issued_question_snapshot,
                seed: u64::from(position) + 1,
                presentation_capability: PresentationCapability::NotApplicable,
                presentation: None,
                presentation_snapshot: None,
                grading_envelope: None,
                native_execution_envelope_capability:
                    NativeExecutionEnvelopeCapability::NotApplicable,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                qti_grading: None,
                qti_grading_capability: QtiGradingCapability::NotApplicable,
                parameter_hash: format!("live-parameters-{position}"),
                provenance: provenance(if position == 0 { "automatic" } else { "manual" }),
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: predecessor,
            },
        )
        .await
        .unwrap_or_else(|error| panic!("issue live mixed-grading attempt {position}: {error:?}"))
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_mixed_automatic_and_manual_grading_is_generation_fenced() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x42; 32]);

    let tenant = TenantId::from_uuid(fresh_uuid());
    let foreign_context =
        TenantContext::from_authenticated_session(TenantId::from_uuid(fresh_uuid()));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(fresh_uuid());
    let student = UserId::from_uuid(fresh_uuid());
    let other_student = UserId::from_uuid(fresh_uuid());
    let course = CourseId::from_uuid(fresh_uuid());
    let assignment = AssignmentId::from_uuid(fresh_uuid());

    let (automatic_reference, automatic_snapshot) = publish_question(
        &store,
        context,
        tenant,
        instructor,
        "live_automatic",
        "Automatic item",
        ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Absolute { epsilon: 0.01 },
            unit: None,
        },
    )
    .await;
    let (manual_reference, manual_snapshot) = publish_question(
        &store,
        context,
        tenant,
        instructor,
        "live_file_upload",
        "Manual file item",
        ResponseDefinition::FileUpload {
            max_bytes: 1_000_000,
            accepted_extensions: vec!["pdf".to_string()],
        },
    )
    .await;
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Live mixed grading course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: sysadmin_course_creation_authority(&store, tenant, course, instructor)
                    .await,
            },
        )
        .await
        .expect("create live mixed-grading course");
    create_published_assignment(
        &store,
        context,
        instructor,
        AssignmentRecord {
            id: assignment,
            tenant,
            course_id: course,
            title: "Live mixed grading assignment".to_string(),
            lifecycle: question_model::AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::default(),
            audience: question_model::AssignmentAudience::CourseWide,
            items: vec![
                assignment_item(automatic_reference, 0),
                assignment_item(manual_reference, 1),
            ],
            selection_groups: Vec::new(),
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
            policies: RunPolicies {
                completion: CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: ContinuedPractice::Unlimited,
                variation: VariationPolicy::NewSeeds,
            },
        },
        question_model::BaseAssignmentPolicy::default(),
    )
    .await
    .expect("create live mixed-grading assignment");
    for (user, display_name) in [
        (student, "Live grading student"),
        (other_student, "Other live grading student"),
    ] {
        store
            .upsert_course_member(
                context,
                instructor,
                UpsertCourseMember {
                    course,
                    user,
                    display_name: display_name.to_string(),
                    roster_contact: None,
                },
            )
            .await
            .expect("canonical roster upsert derives mixed-grading enrollment");
    }
    let run = store
        .start_or_resume_run(
            context,
            student,
            LearnerWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(fresh_uuid()),
        )
        .await
        .expect("start live mixed-grading run");
    let enrollment = run.enrollment;
    assert_eq!(
        store
            .get_enrollment(context, enrollment)
            .await
            .expect("read live fixture enrollment before issue")
            .expect("live fixture enrollment exists before issue")
            .user,
        student
    );

    let attempt_fixture = ManualGradingAttemptFixture {
        store: &store,
        context,
        binding: LearnerWorkRoutingBinding::new(course, assignment),
        student,
        run: run.id,
    };
    let automatic = issue_attempt(
        &attempt_fixture,
        automatic_reference,
        automatic_snapshot,
        0,
        None,
    )
    .await;
    let automatic_response = StudentResponse::Numeric { value: 18.0 };
    let automatic_submission = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                binding: LearnerWorkRoutingBinding::new(course, assignment),
                attempt: automatic.id,
                response: automatic_response,
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("live-automatic")
                    .expect("valid automatic key"),
            },
        )
        .await
        .expect("submit live automatic item");
    assert_eq!(
        automatic_submission.attempt.status,
        AttemptStatus::Submitted
    );
    assert!(automatic_submission.attempt.result.is_some());
    assert_eq!(automatic_submission.run.completed_at, None);

    let manual = issue_attempt(
        &attempt_fixture,
        manual_reference,
        manual_snapshot,
        1,
        Some(automatic.id),
    )
    .await;
    let manual_response = StudentResponse::FileUpload {
        object_key: "student-records/live-manual-response.pdf".to_string(),
    };
    let pending = store
        .submit_pending_manual_question_attempt(
            context,
            SubmitPendingManualQuestionAttemptCommand {
                actor: student,
                binding: LearnerWorkRoutingBinding::new(course, assignment),
                attempt: manual.id,
                response: manual_response.clone(),
                idempotency_key: SubmissionIdempotencyKey::parse("live-manual-pending")
                    .expect("valid pending key"),
            },
        )
        .await
        .expect("submit live pending manual item");
    assert_eq!(pending.attempt.status, AttemptStatus::NeedsManualGrading);
    assert_eq!(pending.attempt.response, Some(manual_response));
    assert_eq!(pending.attempt.result, None);
    assert_eq!(pending.run.completed_at, None);
    assert_eq!(pending.run.score, None);
    let initial_summary = store
        .get_summary(context, enrollment)
        .await
        .expect("read live pending summary")
        .expect("live pending summary exists");
    assert_eq!(initial_summary.current_score, None);
    assert_eq!(initial_summary.completed_run_count, 0);
    let initial_enrollment = store
        .get_enrollment(context, enrollment)
        .await
        .expect("read live pending enrollment")
        .expect("live pending enrollment exists");
    assert_eq!(initial_enrollment.first_completed_at, None);
    assert_eq!(initial_enrollment.current_grade_run, None);

    assert_eq!(
        store
            .get_manual_evaluation_for_edit(context, student, manual.id)
            .await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        store
            .get_manual_evaluation_for_edit(context, other_student, manual.id)
            .await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        store
            .get_manual_evaluation_for_edit(foreign_context, instructor, manual.id)
            .await,
        Err(StoreError::NotFound)
    );
    let initial_evaluation = store
        .get_manual_evaluation_for_edit(context, instructor, manual.id)
        .await
        .expect("instructor reads pending evaluation")
        .expect("pending evaluation exists");
    assert_eq!(initial_evaluation.revision, EvaluationRevision::INITIAL);
    assert_eq!(initial_evaluation.credit, None);

    let first_command = SetManualGradeCommand {
        action: ManualGradeActionId::from_uuid(fresh_uuid()),
        actor: instructor,
        attempt: manual.id,
        expected_revision: EvaluationRevision::INITIAL,
        credit: ManualCredit::parse("0.25").expect("valid first manual credit"),
    };
    let first = store
        .set_manual_grade(context, first_command.clone())
        .await
        .expect("set first live manual grade");
    assert_eq!(
        store.set_manual_grade(context, first_command.clone()).await,
        Ok(first)
    );
    let first_run = store
        .get_run(context, run.id)
        .await
        .expect("read first manually completed run")
        .expect("first manually completed run exists");
    let first_completed_at = first_run
        .completed_at
        .expect("first manual grade completes the mixed run");
    assert!((first_run.score.expect("first current run score") - 0.625).abs() < 1e-12);
    assert_eq!(
        store
            .get_summary(context, enrollment)
            .await
            .expect("read unpublished first summary")
            .expect("unpublished first summary exists")
            .current_score,
        None
    );
    assert_eq!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("read recalculating assignment")
            .expect("recalculating assignment exists")
            .scoring_status,
        ScoringStatus::Recalculating
    );

    let first_job = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("bounded scoring lease"),
        )
        .await
        .expect("claim first manual scoring generation")
        .expect("first manual scoring generation is queued");
    let JobPayload::RecalculateAssignment {
        assignment: first_assignment,
        generation: first_generation,
    } = first_job.payload
    else {
        panic!("manual grade must queue assignment recalculation");
    };
    assert_eq!(first_assignment, assignment);
    assert_eq!(first_generation, first.scoring_generation);
    let first_worker = AssignmentScoringWorkerCommand {
        job: first_job.id,
        lease: first_job.lease_token,
        assignment,
        generation: first_generation,
    };
    store
        .prepare_assignment_scoring(context, first_worker)
        .await
        .expect("prepare first manual scoring generation");

    let correction = store
        .set_manual_grade(
            context,
            SetManualGradeCommand {
                action: ManualGradeActionId::from_uuid(fresh_uuid()),
                actor: instructor,
                attempt: manual.id,
                expected_revision: first.resulting_revision,
                credit: ManualCredit::parse("0.5").expect("valid corrected manual credit"),
            },
        )
        .await
        .expect("correct live manual grade");
    assert_eq!(
        store.set_manual_grade(context, first_command).await,
        Ok(first),
        "an earlier action keeps its original minimal receipt after correction"
    );
    let current_evaluation = store
        .get_manual_evaluation_for_edit(context, instructor, manual.id)
        .await
        .expect("read corrected current evaluation")
        .expect("corrected current evaluation exists");
    assert_eq!(
        current_evaluation
            .credit
            .as_ref()
            .expect("corrected current credit")
            .as_canonical_decimal(),
        "0.5"
    );
    let corrected_attempt = store
        .get_question_attempt(context, manual.id)
        .await
        .expect("read corrected current attempt")
        .expect("corrected current attempt exists");
    assert_eq!(corrected_attempt.status, AttemptStatus::Submitted);
    assert_eq!(
        corrected_attempt
            .result
            .expect("corrected current result")
            .points_earned,
        0.5
    );
    let corrected_run = store
        .get_run(context, run.id)
        .await
        .expect("read corrected current run")
        .expect("corrected current run exists");
    assert_eq!(corrected_run.completed_at, Some(first_completed_at));
    assert!((corrected_run.score.expect("corrected current run score") - 0.75).abs() < 1e-12);
    assert_eq!(
        store
            .get_summary(context, enrollment)
            .await
            .expect("read still-unpublished corrected summary")
            .expect("still-unpublished corrected summary exists")
            .current_score,
        None
    );

    let database_evaluation: (Option<String>, Option<bool>, String, i64) = sqlx::query_as(
        "SELECT credit_fraction::text, correct, grading_status, evaluation_revision \
         FROM submission_evaluation WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(manual.id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("inspect exact live NUMERIC evaluation");
    assert_eq!(database_evaluation.0.as_deref(), Some("0.500000000000"));
    assert_eq!(database_evaluation.1, Some(false));
    assert_eq!(database_evaluation.2, "graded");
    assert_eq!(database_evaluation.3, 3);
    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM manual_grade_receipt WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(manual.id.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("count minimal live manual receipts");
    assert_eq!(receipt_count, 2);

    assert_eq!(
        store.commit_assignment_scoring(context, first_worker).await,
        Ok(AssignmentScoringCommitOutcome::Superseded)
    );
    let correction_job = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("corrected scoring lease"),
        )
        .await
        .expect("claim corrected manual scoring generation")
        .expect("corrected manual scoring generation is queued");
    let JobPayload::RecalculateAssignment {
        assignment: corrected_assignment,
        generation: corrected_generation,
    } = correction_job.payload
    else {
        panic!("manual correction must queue assignment recalculation");
    };
    assert_eq!(corrected_assignment, assignment);
    assert_eq!(corrected_generation, correction.scoring_generation);
    let correction_worker = AssignmentScoringWorkerCommand {
        job: correction_job.id,
        lease: correction_job.lease_token,
        assignment,
        generation: corrected_generation,
    };
    store
        .prepare_assignment_scoring(context, correction_worker)
        .await
        .expect("prepare corrected manual scoring generation");
    assert_eq!(
        store
            .commit_assignment_scoring(context, correction_worker)
            .await,
        Ok(AssignmentScoringCommitOutcome::Committed)
    );

    let published_summary = store
        .get_summary(context, enrollment)
        .await
        .expect("read published mixed summary")
        .expect("published mixed summary exists");
    assert_eq!(published_summary.current_score, Some(0.75));
    assert_eq!(published_summary.best_score, Some(0.75));
    assert_eq!(published_summary.latest_score, Some(0.75));
    assert_eq!(published_summary.completed_run_count, 1);
    let published_enrollment = store
        .get_enrollment(context, enrollment)
        .await
        .expect("read published mixed enrollment")
        .expect("published mixed enrollment exists");
    assert_eq!(
        published_enrollment.first_completed_at,
        Some(first_completed_at)
    );
    assert_eq!(published_enrollment.current_grade_run, Some(run.id));
    assert_eq!(published_enrollment.best_grade_run, Some(run.id));
    assert_eq!(
        store.get_question_attempt(foreign_context, manual.id).await,
        Ok(None)
    );
    assert_eq!(
        store
            .set_manual_grade(
                foreign_context,
                SetManualGradeCommand {
                    action: ManualGradeActionId::from_uuid(fresh_uuid()),
                    actor: instructor,
                    attempt: manual.id,
                    expected_revision: correction.resulting_revision,
                    credit: ManualCredit::parse("1").expect("valid foreign probe credit"),
                },
            )
            .await,
        Err(StoreError::NotFound)
    );
}
