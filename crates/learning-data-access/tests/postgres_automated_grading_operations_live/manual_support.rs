//! Production-shaped manual-grade and learner-support invalidation evidence.

use learning_data_access::{
    AttemptSupportActionId, ClearAttemptCommand, CourseRosterStore, EvaluationRevision,
    FlatGradingCapability, IssueQuestionAttemptCommand, ManualCredit, ManualGradeActionId,
    ManualGradingStore, NativeExecutionEnvelopeCapability, PresentationCapability,
    QtiGradingCapability, SetManualGradeCommand, Store, SubmissionIdempotencyKey,
    SubmitPendingManualQuestionAttemptCommand, TenantContext, UpsertCourseMember,
    WebworkGradingCapability,
};
use question_model::{
    AssignmentId, AttemptProvenance, CourseId, ProblemVersionRef, QuestionAttemptId, RunId,
    StudentResponse, TenantId, UserId,
};
use sqlx::{PgPool, Row};

use super::{fresh_uuid, implementation};

pub(super) struct ManualSupportScenario<'a> {
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

pub(super) async fn prove_manual_grade_and_support_origins(scenario: ManualSupportScenario<'_>) {
    let ManualSupportScenario {
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
                display_name: "Manual/support connected learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("create dedicated manual/support learner");
    let run = store
        .start_or_resume_run(
            context,
            student,
            learning_data_access::LearnerWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(fresh_uuid()),
        )
        .await
        .expect("start dedicated manual/support run");
    let attempt = QuestionAttemptId::from_uuid(fresh_uuid());
    store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                binding: learning_data_access::LearnerWorkRoutingBinding::new(course, assignment),
                attempt,
                run: run.id,
                assignment_position: 0,
                problem: question.problem,
                question_version: question.version,
                issued_question_snapshot: snapshot,
                seed: 41,
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
                parameter_hash: "connected-manual-support".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("connected-manual-support-native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("connected-manual-support-grade"),
                    rendered_question_sha256: "connected-manual-support-render".to_string(),
                },
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("issue dedicated manual/support attempt");
    store
        .submit_pending_manual_question_attempt(
            context,
            SubmitPendingManualQuestionAttemptCommand {
                actor: student,
                binding: learning_data_access::LearnerWorkRoutingBinding::new(course, assignment),
                attempt,
                response: StudentResponse::Numeric { value: 7.0 },
                idempotency_key: SubmissionIdempotencyKey::parse("connected-manual-support")
                    .expect("manual/support idempotency key"),
            },
        )
        .await
        .expect("submit pending manual evaluation");
    let manual_action = ManualGradeActionId::from_uuid(fresh_uuid());
    let manual = store
        .set_manual_grade(
            context,
            SetManualGradeCommand {
                action: manual_action,
                actor: instructor,
                attempt,
                expected_revision: EvaluationRevision::INITIAL,
                credit: ManualCredit::parse("1.0").expect("manual credit"),
            },
        )
        .await
        .expect("set manual grade through production adapter");
    assert_eq!(
        store
            .set_manual_grade(
                context,
                SetManualGradeCommand {
                    action: manual_action,
                    actor: instructor,
                    attempt,
                    expected_revision: EvaluationRevision::INITIAL,
                    credit: ManualCredit::parse("1.0").expect("manual credit replay"),
                },
            )
            .await
            .expect("replay manual grade through production adapter"),
        manual,
        "the same manual-grade source action replays its original evidence"
    );
    assert_origin(
        pool,
        tenant,
        "manual_grade",
        manual_action.as_uuid(),
        instructor,
        manual.scoring_generation.value(),
    )
    .await;
    let support_action = AttemptSupportActionId::from_uuid(fresh_uuid());
    let support = store
        .clear_attempt(
            context,
            ClearAttemptCommand {
                action: support_action,
                actor: instructor,
                attempt,
            },
        )
        .await
        .expect("clear evaluated attempt through production adapter");
    assert_eq!(
        support.action, support_action,
        "support receipt retains its action"
    );
    let support_generation: i64 = sqlx::query_scalar(
        "SELECT scoring_generation FROM public.scoring_invalidation_origin \
         WHERE tenant_id=$1 AND origin_kind='learner_support' AND origin_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(support_action.as_uuid())
    .fetch_one(pool)
    .await
    .expect("read learner-support generation");
    assert_origin(
        pool,
        tenant,
        "learner_support",
        support_action.as_uuid(),
        instructor,
        u64::try_from(support_generation).expect("positive support generation"),
    )
    .await;
}

async fn assert_origin(
    pool: &PgPool,
    tenant: TenantId,
    kind: &str,
    action: uuid::Uuid,
    actor: UserId,
    generation: u64,
) {
    let row = sqlx::query(
        "SELECT actor_id, scoring_generation, recalculation_job_id, grading_operation_id \
         FROM public.scoring_invalidation_origin \
         WHERE tenant_id=$1 AND origin_kind=$2 AND origin_id=$3",
    )
    .bind(tenant.as_uuid())
    .bind(kind)
    .bind(action)
    .fetch_one(pool)
    .await
    .expect("read immutable source origin");
    assert_eq!(
        row.try_get::<uuid::Uuid, _>("actor_id").unwrap(),
        actor.as_uuid()
    );
    assert_eq!(
        row.try_get::<i64, _>("scoring_generation").unwrap(),
        i64::try_from(generation).expect("generation fits PostgreSQL bigint")
    );
    assert_eq!(
        row.try_get::<uuid::Uuid, _>("recalculation_job_id")
            .unwrap(),
        action,
        "source action deterministically owns its recalculation job"
    );
    assert!(
        row.try_get::<i64, _>("grading_operation_id").unwrap() > 0,
        "every scoring origin creates an Instructor-visible operation"
    );
}
