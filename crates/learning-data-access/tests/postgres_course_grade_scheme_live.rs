#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for WP-PROF-S6 course-grade persistence.
//!
//! This creates no runs: course totals use summaries, never activity history.

#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;

#[path = "fixtures/published_assignment.rs"]
mod published_assignment;
use published_assignment::create_published_assignment;

use learning_data_access::postgres::PostgresStore;
use learning_data_access::{
    AssignmentRecord, CatalogStore, CourseRecord, CreateCourseCommand, DraftRecord,
    PublishDraftCommand, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, Store,
    TenantContext,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, TimingPolicy,
    VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    AssignmentAudience, AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentScoringMode, BackendCapabilities, Capability, CourseId, DraftQuestionDefinition,
    DraftQuestionSource, GradingDefinition, PointValue, ProblemId, ProblemVersionRef,
    PublicationScope, QuestionMetadata, QuestionSource, RunPolicies, TenantId, UserId, UserRole,
    VersionId, WorkspaceId,
};
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

async fn session(store: &PostgresStore, tenant: TenantId, user: UserId) -> SessionTokenHash {
    let token = SessionTokenHash::compute(id().as_bytes());
    store
        .create_session(
            token,
            SessionSubject::new(tenant, user, "S6 live fixture", vec![UserRole::Instructor])
                .expect("valid fixture session"),
            SessionLifetime::from_seconds(3600).expect("valid fixture lifetime"),
        )
        .await
        .expect("fixture session persists");
    token
}

async fn create_fixture_course(
    store: &PostgresStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
) -> CourseId {
    let course = CourseId::from_uuid(id());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "S6 live course".into(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("fixture term"),
                },
                authority: sysadmin_course_creation_authority(store, tenant, course, instructor)
                    .await,
            },
        )
        .await
        .expect("fixture course persists");
    course
}

#[rustfmt::skip]
async fn publish_fixture_question(store: &PostgresStore, context: TenantContext, tenant: TenantId, instructor: UserId) -> ProblemVersionRef {
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id()),
        version: VersionId::from_uuid(id()),
    };
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(id()),
            source: DraftQuestionSource::Native { family: "molar_mass".into() }, prompt: vec![ContentBlock::Text { markdown: "S6 numeric fixture".into() }],
            response: question_model::ResponseDefinition::Numeric { tolerance: NumericTolerance::Absolute { epsilon: 0.01 }, unit: None },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata { title: "S6 numeric fixture".into(), tags: Vec::new(), taxonomy: Vec::new(), license: License::CcBy, language: "en-US".into() },
        },
        derived_from: None,
    };
    let saved = store.upsert_draft(context, instructor, None, draft.clone()).await.expect("save numeric draft");
    store.publish_draft(context, instructor, PublishDraftCommand { expected_draft: draft, expected_revision: saved.revision, publication: reference, published_source: QuestionSource::Native { family: "molar_mass".into() }, publisher: instructor, scope: PublicationScope::Public, byline: question_model::PublicByline::new(vec![question_model::PublicAuthorName::new("PLE fixture".to_string()).expect("byline")]).expect("byline"), source_artifact: None, qti_promotion: None, flat_question_promotion: None, capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]) }).await.expect("publish numeric question");
    reference
}

#[rustfmt::skip]
fn numeric_assignment(tenant: TenantId, course: CourseId, reference: ProblemVersionRef, title: &str, points: u32) -> AssignmentRecord {
    AssignmentRecord { id: AssignmentId::from_uuid(id()), tenant, course_id: course, title: title.into(), lifecycle: question_model::AssignmentLifecycle::Published, instructions: question_model::AssignmentInstructions::default(), audience: AssignmentAudience::CourseWide, items: vec![AssignmentItem { id: AssignmentItemId::from_uuid(id()), reference, position: 0, points_possible: PointValue::from_whole(points), delivery_state: AssignmentDeliveryState::Active, scoring_mode: AssignmentScoringMode::Normal }], selection_groups: Vec::new(), disclosure_policy: question_model::LearnerDisclosurePolicy::default(), policies: RunPolicies { completion: CompletionRequirement::AnswerAll, grade: GradePolicy::Highest, continued_practice: ContinuedPractice::Unlimited, variation: VariationPolicy::NewSeeds } }
}

#[rustfmt::skip]
async fn set_summary_scores(pool: &sqlx::PgPool, tenant: TenantId, student: UserId, scores: &[(AssignmentId, Option<f64>)], status: Option<&str>) {
    let mut tx = pool.begin().await.expect("summary fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_app").execute(&mut *tx).await.expect("app role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)").bind(tenant.to_string()).execute(&mut *tx).await.expect("tenant context");
    for (assignment, score) in scores {
        let updated = sqlx::query("UPDATE student_assignment_summary sas SET current_score=$4,best_score=$4,latest_score=$4 FROM enrollment e WHERE sas.tenant_id=$1 AND sas.enrollment_id=e.enrollment_id AND e.user_id=$2 AND e.assignment_id=$3")
            .bind(tenant.as_uuid()).bind(student.as_uuid()).bind(assignment.as_uuid()).bind(score)
            .execute(&mut *tx).await.expect("summary projection update");
        assert_eq!(updated.rows_affected(), 1, "one materialized learner summary is updated");
        if let Some(status) = status {
            sqlx::query("RESET ROLE").execute(&mut *tx).await.expect("schema owner fixture role");
            sqlx::query("UPDATE assignment SET scoring_status=$3 WHERE tenant_id=$1 AND assignment_id=$2").bind(tenant.as_uuid()).bind(assignment.as_uuid()).bind(status).execute(&mut *tx).await.expect("assignment scoring state update");
            sqlx::query("SET LOCAL ROLE ple_app").execute(&mut *tx).await.expect("restore app role");
        }
    }
    tx.commit().await.expect("summary fixture commit");
}

#[path = "postgres_course_grade_scheme_live/course_grade_cases.rs"]
mod course_grade_cases;

#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
use acceptance_runtime::load as load_acceptance_runtime;
