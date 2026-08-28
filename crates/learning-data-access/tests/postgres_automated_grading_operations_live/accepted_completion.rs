//! Accepted-submission completion evidence through the sealed fast-path pool.

use acceptance_runtime::AcceptanceRuntime;
use learning_data_access::postgres::{
    PostgresAcceptedSubmissionFastPathStore, local_accepted_submission_fast_path_pool,
};
use learning_data_access::{
    AcceptedSubmissionCommand, AcceptedSubmissionExecutionDisposition,
    AcceptedSubmissionExecutionFastPathClaimStore, AcceptedSubmissionExecutionOutcome,
    AcceptedSubmissionExecutionStore, AcceptedSubmissionExecutionTarget, AcceptedSubmissionGrade,
    AutomatedGradingStore, CourseRosterStore, FlatGradingCapability, IssueQuestionAttemptCommand,
    JobId, JobLeaseDuration, LearnerWorkRoutingBinding, NativeExecutionEnvelopeCapability,
    PresentationCapability, QtiGradingCapability, Store, SubmissionIdempotencyKey, TenantContext,
    UpsertCourseMember, WebworkGradingCapability, WorkerId, canonical_attempt_result_json,
};
use question_model::{
    AssignmentId, AttemptProvenance, AttemptResult, CourseId, EnrollmentId, FeedbackContent,
    ProblemVersionRef, QuestionAttemptId, RunId, StudentResponse, TenantId, UserId,
};
use sqlx::{PgPool, Row};

use super::{fresh_uuid, implementation};

pub(super) struct AcceptedCompletionScenario<'a> {
    pub(super) runtime: &'a AcceptanceRuntime,
    pub(super) store: &'a learning_data_access::postgres::PostgresStore,
    pub(super) pool: &'a PgPool,
    pub(super) context: TenantContext,
    pub(super) tenant: TenantId,
    pub(super) instructor: UserId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) question: ProblemVersionRef,
    pub(super) snapshot: learning_data_access::IssuedQuestionSnapshotV1,
}

pub(super) struct AcceptedCompletionOrigin {
    pub(super) recalculation_job: uuid::Uuid,
    pub(super) scoring_generation: i64,
    pub(super) student: UserId,
    pub(super) enrollment: EnrollmentId,
}

pub(super) async fn prove_accepted_completion_origin(
    scenario: AcceptedCompletionScenario<'_>,
) -> AcceptedCompletionOrigin {
    let AcceptedCompletionScenario {
        runtime,
        store,
        pool,
        context,
        tenant,
        instructor,
        course,
        assignment,
        question,
        snapshot,
    } = scenario;
    let student = UserId::from_uuid(fresh_uuid());
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Accepted-completion connected learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("create accepted-completion learner");
    let run = store
        .start_or_resume_run(
            context,
            student,
            LearnerWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(fresh_uuid()),
        )
        .await
        .expect("start accepted-completion run");
    let attempt = QuestionAttemptId::from_uuid(fresh_uuid());
    store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                binding: LearnerWorkRoutingBinding::new(course, assignment),
                attempt,
                run: run.id,
                assignment_position: 0,
                problem: question.problem,
                question_version: question.version,
                issued_question_snapshot: snapshot,
                seed: 53,
                presentation_capability: PresentationCapability::NotApplicable,
                presentation: None,
                presentation_snapshot: None,
                grading_envelope: None,
                native_execution_envelope_capability:
                    NativeExecutionEnvelopeCapability::NotApplicable,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability: WebworkGradingCapability::NotApplicable,
                qti_grading: None,
                qti_grading_capability: QtiGradingCapability::NotApplicable,
                parameter_hash: "connected-accepted-completion".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("connected-completion-native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("connected-completion-grade"),
                    rendered_question_sha256: "connected-completion-render".to_string(),
                },
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("issue accepted-completion attempt");
    let execution_job = JobId::from_uuid(fresh_uuid());
    let accepted = store
        .accept_automated_submission(
            context,
            AcceptedSubmissionCommand {
                actor: student,
                course,
                assignment,
                attempt,
                idempotency_key: SubmissionIdempotencyKey::parse("connected-accepted-completion")
                    .expect("accepted-completion key"),
                response: StudentResponse::Numeric { value: 5.0 },
                execution_job,
            },
        )
        .await
        .expect("accept exact completion response");
    let fast_pool = local_accepted_submission_fast_path_pool(runtime.fast_path_url().expose())
        .await
        .expect("attest disposable fast-path pool");
    let fast_path = PostgresAcceptedSubmissionFastPathStore::from_fast_path_pool(fast_pool);
    let claim = fast_path
        .claim_exact_accepted_submission_execution(
            AcceptedSubmissionExecutionTarget {
                tenant,
                attempt,
                submission: accepted.submission,
                job: execution_job,
            },
            WorkerId::from_uuid(fresh_uuid()),
            JobLeaseDuration::from_seconds(30).expect("bounded accepted-completion lease"),
        )
        .await
        .expect("claim exact accepted completion")
        .expect("new accepted submission is claimable");
    let grade = AcceptedSubmissionGrade {
        evidence: canonical_attempt_result_json(AttemptResult {
            correct: true,
            points_earned: 1.0,
            points_possible: 1.0,
        })
        .expect("canonical evaluated result"),
        feedback: FeedbackContent::default(),
    };
    assert_eq!(
        fast_path
            .commit_or_fail_accepted_submission_execution(
                context,
                claim,
                AcceptedSubmissionExecutionOutcome::Evaluated { grade },
            )
            .await
            .expect("commit accepted completion through sealed worker adapter"),
        AcceptedSubmissionExecutionDisposition::Committed
    );
    let row = sqlx::query(
        "SELECT origin_id, actor_id, recalculation_job_id, scoring_generation, grading_operation_id \
         FROM public.scoring_invalidation_origin \
         WHERE tenant_id=$1 AND origin_kind='accepted_submission_completion' AND origin_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(accepted.submission.as_uuid())
    .fetch_one(pool)
    .await
    .expect("read accepted-completion origin");
    assert_eq!(
        row.try_get::<uuid::Uuid, _>("origin_id").unwrap(),
        accepted.submission.as_uuid()
    );
    assert!(
        row.try_get::<Option<uuid::Uuid>, _>("actor_id")
            .unwrap()
            .is_none(),
        "accepted completion is system-origin evidence"
    );
    assert_ne!(
        row.try_get::<uuid::Uuid, _>("recalculation_job_id")
            .unwrap(),
        execution_job.as_uuid(),
        "execution and recalculation jobs remain distinct"
    );
    assert!(
        row.try_get::<i64, _>("grading_operation_id").unwrap() > 0,
        "completion creates an Instructor-visible scoring thread"
    );
    AcceptedCompletionOrigin {
        recalculation_job: row.try_get("recalculation_job_id").unwrap(),
        scoring_generation: row.try_get("scoring_generation").unwrap(),
        student,
        enrollment: run.enrollment,
    }
}
