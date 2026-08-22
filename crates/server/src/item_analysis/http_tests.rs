use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::Request;
use axum::response::Response;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AssignmentRecord, CatalogStore, CourseItemAnalysisCommitOutcome,
    CourseItemAnalysisWorkerCommand, CourseItemAnalysisWorkerStore, CourseRecord,
    CourseRosterStore, CreateCourseCommand, DraftRecord, EnqueueJob, JobLeaseDuration, JobPayload,
    JobStore, PublishDraftCommand, SessionLifetime, SessionSubject, Store, TenantContext,
    UpsertCourseMember,
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
    ActivityTimestamp, AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentScoringMode, BackendCapabilities, Capability, CourseId, DraftQuestionDefinition,
    DraftQuestionSource, GradingDefinition, PointValue, ProblemId, ProblemVersionRef,
    PublicationScope, QuestionMetadata, QuestionSource, ScoringGeneration, TenantId, UserId,
    UserRole, VersionId, WorkspaceId,
};
use tower::ServiceExt;
use uuid::Uuid;

use super::router;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

async fn issued_cookie(
    store: &MemoryStore,
    tenant: TenantId,
    user: UserId,
    roles: Vec<UserRole>,
) -> String {
    let issued = crate::auth::issue_session(
        store,
        SessionSubject::new(tenant, user, "Item analysis fixture", roles).expect("fixture subject"),
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("positive lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("issue session");
    issued
        .set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string()
}

async fn publish_fixture(
    store: &MemoryStore,
    context: TenantContext,
    tenant: TenantId,
    publisher: UserId,
) -> ProblemVersionRef {
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id(20)),
        version: VersionId::from_uuid(id(21)),
    };
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(id(22)),
            source: DraftQuestionSource::Native {
                family: "item-analysis-fixture".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Fixture question".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Absolute { epsilon: 0.0 },
                unit: None,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Fixture question".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        },
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("save draft");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "item-analysis-fixture".to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish fixture");
    reference
}

fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AllCorrect,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

async fn get(app: &axum::Router, cookie: &str, uri: String) -> Response {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

#[tokio::test]
async fn current_item_analysis_route_authorizes_without_leaking_private_analysis_inputs() {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(2));
    let sysadmin = UserId::from_uuid(id(3));
    let student = UserId::from_uuid(id(4));
    let outsider = UserId::from_uuid(id(5));
    let foreign_tenant = TenantId::from_uuid(id(6));
    let foreign_user = UserId::from_uuid(id(7));
    let instructor_cookie =
        issued_cookie(&store, tenant, instructor, vec![UserRole::Instructor]).await;
    let sysadmin_cookie = issued_cookie(&store, tenant, sysadmin, vec![UserRole::Sysadmin]).await;
    let student_cookie = issued_cookie(&store, tenant, student, vec![UserRole::Student]).await;
    let outsider_cookie = issued_cookie(&store, tenant, outsider, vec![UserRole::Instructor]).await;
    let foreign_cookie = issued_cookie(
        &store,
        foreign_tenant,
        foreign_user,
        vec![UserRole::Sysadmin],
    )
    .await;

    let course = CourseId::from_uuid(id(8));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "BIOC 301".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("create course");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Item analysis learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("student roster membership");
    let reference = publish_fixture(&store, context, tenant, instructor).await;
    let assignment = AssignmentId::from_uuid(id(9));
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                audience: question_model::AssignmentAudience::CourseWide,
                title: "Item analysis fixture".to_string(),
                lifecycle: question_model::AssignmentLifecycle::Draft,
                instructions: question_model::AssignmentInstructions::default(),
                items: vec![AssignmentItem {
                    id: AssignmentItemId::from_uuid(id(10)),
                    reference,
                    position: 0,
                    points_possible: PointValue::from_whole(1),
                    delivery_state: AssignmentDeliveryState::Active,
                    scoring_mode: AssignmentScoringMode::Normal,
                }],
                selection_groups: Vec::new(),
                disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                policies: policies(),
            },
            question_model::BaseAssignmentPolicy::default(),
        )
        .await
        .expect("create assignment");
    crate::course::tests::fixtures::publish_assignment(
        store.as_ref(),
        context,
        instructor,
        course,
        assignment,
        question_model::AssignmentTeachingSettings {
            lifecycle: question_model::AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::default(),
            base_policy: question_model::BaseAssignmentPolicy::default(),
        },
    )
    .await;
    store
        .enqueue_job(
            context,
            EnqueueJob {
                tenant,
                payload: JobPayload::RecalculateCourseItemAnalysis {
                    assignment,
                    generation: ScoringGeneration::INITIAL,
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("enqueue analysis");
    let claim = store
        .claim_next_job(
            &learning_data_access::JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease"),
        )
        .await
        .expect("claim analysis")
        .expect("analysis job");
    let JobPayload::RecalculateCourseItemAnalysis {
        assignment: queued_assignment,
        generation,
    } = claim.payload
    else {
        panic!("claimed wrong worker family");
    };
    let command = CourseItemAnalysisWorkerCommand {
        job: claim.id,
        lease: claim.lease_token,
        assignment: queued_assignment,
        generation,
    };
    store
        .prepare_course_item_analysis(context, command)
        .await
        .expect("stage analysis");
    assert_eq!(
        store
            .commit_course_item_analysis(context, command)
            .await
            .expect("publish analysis"),
        CourseItemAnalysisCommitOutcome::Committed
    );

    let app = router(Arc::clone(&store));
    let uri = format!("/api/courses/{course}/assignments/{assignment}/item-analysis");
    for cookie in [&instructor_cookie] {
        let response = get(&app, cookie, uri.clone()).await;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let body = to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("response body");
        let body = std::str::from_utf8(&body).expect("JSON UTF-8");
        for forbidden in [
            "\"tenant\"",
            "\"course\"",
            "\"assignment\"",
            "\"reference\"",
            "\"problem\"",
            "\"version\"",
            "\"learner\"",
            "\"attempt\"",
            "\"response\"",
            "\"answer\"",
            "\"key\"",
            "\"object\"",
        ] {
            assert!(
                !body.contains(forbidden),
                "response leaked {forbidden}: {body}"
            );
        }
    }
    for cookie in [
        &sysadmin_cookie,
        &student_cookie,
        &outsider_cookie,
        &foreign_cookie,
    ] {
        let response = get(&app, cookie, uri.clone()).await;
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
}
