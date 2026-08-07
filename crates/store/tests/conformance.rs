//! Reusable Store conformance suite, first run against memory in WP-C4.

use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::StudentResponse;
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::{License, Tag};
use question_model::{
    ActivityTimestamp, AssignmentEnrollment, AssignmentId, AssignmentRun, AttemptProvenance,
    AttemptResult, AttemptTimerRecord, CompletionRequirement, ContinuedPractice, EnrollmentId,
    GeneratorReference, GradePolicy, GradingDefinition, ImplementationVersion, ObjectId, ProblemId,
    QuestionAttempt, QuestionAttemptId, QuestionDefinition, QuestionMetadata, QuestionSource,
    ResponseDefinition, RunId, RunMode, RunPolicies, StudentId, TenantId, VariationPolicy,
    VersionId, WorkspaceId,
};
use store::memory::MemoryStore;
use store::{
    ActivityTransition, AssignmentRecord, DraftRecord, PageRequest, PageSize,
    PublishedProblemRecord, PublishedVersionRef, Store, StoreError, TenantContext,
};
use uuid::Uuid;

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn question(
    problem: Option<ProblemId>,
    version: VersionId,
    workspace: WorkspaceId,
) -> QuestionDefinition {
    QuestionDefinition {
        version,
        problem,
        workspace,
        source: QuestionSource::Native {
            family: "molar_mass".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "What is the molar mass?".to_string(),
        }],
        response: ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Relative { fraction: 0.01 },
            unit: Some("g/mol".to_string()),
        },
        attempt_policy: AttemptPolicy {
            max_attempts: None,
            feedback: FeedbackDisclosure::ImmediateFull,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Molar mass".to_string(),
            tags: vec![Tag::new("biochemistry")],
            taxonomy: Vec::new(),
            license: License::CcBySa,
            language: "en-US".to_string(),
        },
    }
}

fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AllCorrect,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

fn implementation(id: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: id.to_string(),
        version: "1".to_string(),
    }
}

fn generator(id: &str) -> GeneratorReference {
    GeneratorReference {
        id: id.to_string(),
        version: "1".to_string(),
    }
}

async fn exercise_store(store: &dyn Store) {
    let tenant = TenantId::from_uuid(uuid(1));
    let foreign_tenant = TenantId::from_uuid(uuid(2));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let workspace = WorkspaceId::from_uuid(uuid(3));
    let problem = ProblemId::from_uuid(uuid(4));
    let version = VersionId::from_uuid(uuid(5));
    let second_problem = ProblemId::from_uuid(uuid(6));
    let second_version = VersionId::from_uuid(uuid(7));
    let assignment_id = AssignmentId::from_uuid(uuid(8));
    let enrollment_id = EnrollmentId::from_uuid(uuid(9));
    let run_id = RunId::from_uuid(uuid(10));
    let practice_run_id = RunId::from_uuid(uuid(14));
    let draft = DraftRecord {
        tenant,
        question: question(None, version, workspace),
    };
    let published = PublishedProblemRecord {
        problem,
        version,
        question: question(Some(problem), version, workspace),
    };
    let assignment = AssignmentRecord {
        id: assignment_id,
        tenant,
        problems: vec![PublishedVersionRef { problem, version }],
        policies: policies(),
    };

    store
        .upsert_draft(context, draft.clone())
        .await
        .expect("conforming draft write should succeed");
    store
        .publish_problem(published.clone())
        .await
        .expect("conforming publish should succeed");
    store
        .publish_problem(PublishedProblemRecord {
            problem: second_problem,
            version: second_version,
            question: question(Some(second_problem), second_version, workspace),
        })
        .await
        .expect("second publish should succeed");

    let first_page = store
        .list_published_problems(PageRequest::first(
            PageSize::new(1).expect("one is a valid page size"),
        ))
        .await
        .expect("first catalog page should load");
    let second_page = store
        .list_published_problems(PageRequest::after(
            first_page
                .next_cursor
                .clone()
                .expect("first page should carry a cursor"),
            PageSize::new(1).expect("one is a valid page size"),
        ))
        .await
        .expect("second catalog page should load");

    store
        .upsert_assignment(context, assignment.clone())
        .await
        .expect("conforming assignment write should succeed");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: enrollment_id,
                tenant,
                assignment: assignment_id,
                student: StudentId::from_uuid(uuid(11)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("conforming enrollment creation should succeed");
    store
        .apply_activity_transition(
            context,
            ActivityTransition::StartRun {
                run: AssignmentRun {
                    id: run_id,
                    tenant,
                    enrollment: enrollment_id,
                    run_number: 1,
                    started_at: ActivityTimestamp::from_unix_millis(100),
                    completed_at: None,
                    score: None,
                    mode: RunMode::Assigned,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .expect("conforming run start should succeed");
    store
        .apply_activity_transition(
            context,
            ActivityTransition::RecordQuestionAttempt {
                attempt: Box::new(QuestionAttempt {
                    id: QuestionAttemptId::from_uuid(uuid(12)),
                    tenant,
                    run: run_id,
                    problem,
                    question_version: version,
                    seed: 42,
                    parameter_hash: "parameters-sha256".to_string(),
                    response: Some(StudentResponse::Numeric { value: 18.0 }),
                    result: Some(AttemptResult {
                        correct: true,
                        points_earned: 1.0,
                        points_possible: 1.0,
                    }),
                    timer: AttemptTimerRecord {
                        issued_at: ActivityTimestamp::from_unix_millis(110),
                        deadline: None,
                        submitted_at: Some(ActivityTimestamp::from_unix_millis(120)),
                    },
                    provenance: AttemptProvenance {
                        adapter: implementation("native"),
                        renderer: None,
                        generator: Some(generator("molar-mass")),
                        source_artifact: None,
                        asset_objects: vec![ObjectId::from_uuid(uuid(13))],
                        grading: implementation("numeric"),
                        rendered_question_sha256: "render-sha256".to_string(),
                    },
                }),
            },
        )
        .await
        .expect("conforming attempt write should succeed");
    let summary = store
        .apply_activity_transition(
            context,
            ActivityTransition::CompleteRun {
                run: run_id,
                score: 1.0,
                at: ActivityTimestamp::from_unix_millis(130),
            },
        )
        .await
        .expect("conforming completion should succeed");
    let completed_run = store
        .get_run(context, run_id)
        .await
        .expect("run read should succeed")
        .expect("completed run should exist");
    let attempt = store
        .get_question_attempt(context, QuestionAttemptId::from_uuid(uuid(12)))
        .await
        .expect("attempt read should succeed")
        .expect("question attempt should exist");

    store
        .apply_activity_transition(
            context,
            ActivityTransition::StartRun {
                run: AssignmentRun {
                    id: practice_run_id,
                    tenant,
                    enrollment: enrollment_id,
                    run_number: 2,
                    started_at: ActivityTimestamp::from_unix_millis(140),
                    completed_at: None,
                    score: None,
                    mode: RunMode::Practice,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .expect("continued practice should remain available after completion");
    let practice_summary = store
        .apply_activity_transition(
            context,
            ActivityTransition::CompleteRun {
                run: practice_run_id,
                score: 0.8,
                at: ActivityTimestamp::from_unix_millis(150),
            },
        )
        .await
        .expect("continued-practice completion should succeed");
    let enrollment = store
        .get_enrollment(context, enrollment_id)
        .await
        .expect("enrollment read should succeed")
        .expect("enrollment should exist");
    let persisted_summary = store
        .get_summary(context, enrollment_id)
        .await
        .expect("summary read should succeed")
        .expect("summary should exist");

    let tenant_mismatch = store
        .upsert_draft(
            foreign_context,
            DraftRecord {
                tenant,
                question: question(None, VersionId::from_uuid(uuid(15)), workspace),
            },
        )
        .await;
    let tenant_assignments = store
        .list_assignments(
            context,
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("assignment list should load");
    let run_page = store
        .list_runs(
            context,
            enrollment_id,
            PageRequest::first(PageSize::new(10).expect("ten is a valid page size")),
        )
        .await
        .expect("run list should load");

    assert_eq!((first_page.items.len(), second_page.items.len()), (1, 1));
    assert_eq!(store.get_draft(context, workspace).await, Ok(Some(draft)));
    assert_eq!(
        store.get_published_problem(problem, version).await,
        Ok(Some(published))
    );
    assert_eq!(
        store.get_assignment(context, assignment_id).await,
        Ok(Some(assignment))
    );
    assert_eq!(tenant_assignments.items.len(), 1);
    assert_eq!(
        (
            summary.current_score,
            summary.completed_run_count,
            summary.total_question_attempts,
        ),
        (Some(1.0), 1, 1)
    );
    assert_eq!(practice_summary, persisted_summary);
    assert_eq!(
        (
            persisted_summary.current_score,
            persisted_summary.best_score,
            persisted_summary.latest_score,
            persisted_summary.completed_run_count,
        ),
        (Some(1.0), Some(1.0), Some(0.8), 2)
    );
    assert_eq!(
        (
            enrollment.first_completed_at,
            enrollment.current_grade_run,
            enrollment.best_grade_run,
        ),
        (
            Some(ActivityTimestamp::from_unix_millis(130)),
            Some(run_id),
            Some(run_id),
        )
    );
    assert_eq!(
        (
            completed_run.completed_at,
            attempt.problem,
            run_page.items.len()
        ),
        (Some(ActivityTimestamp::from_unix_millis(130)), problem, 2,)
    );
    assert_eq!(tenant_mismatch, Err(StoreError::TenantMismatch));
    assert_eq!(store.get_draft(foreign_context, workspace).await, Ok(None));
    assert_eq!(
        store.get_assignment(foreign_context, assignment_id).await,
        Ok(None)
    );
    assert_eq!(
        store.get_enrollment(foreign_context, enrollment_id).await,
        Ok(None)
    );
    assert_eq!(store.get_run(foreign_context, run_id).await, Ok(None));
    assert_eq!(
        store
            .get_question_attempt(foreign_context, QuestionAttemptId::from_uuid(uuid(12)))
            .await,
        Ok(None)
    );
    assert_eq!(
        store.get_summary(foreign_context, enrollment_id).await,
        Ok(None)
    );
}

#[tokio::test]
async fn memory_store_conforms() {
    exercise_store(&MemoryStore::default()).await;
}
