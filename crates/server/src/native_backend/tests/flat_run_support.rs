//! Shared setup for the native flat-question run lifecycle.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderValue, Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AssignmentRecord, CourseRecord, CourseRosterStore, CreateCourseCommand, SessionLifetime,
    SessionSubject, Store, UpsertCourseMember,
};
use question_model::response::ChoiceId;
use question_model::run_policy::{
    CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies, VariationPolicy,
};
use question_model::{
    AssignmentId, AssignmentItem, AssignmentItemId, CourseId, PointValue, UserId, UserRole,
};
use tower::ServiceExt;

use super::{published_flat_fixture, uuid};
use crate::native_backend::NativeBackend;

pub(super) async fn flat_run_fixture() -> (
    Router,
    Arc<MemoryStore>,
    Arc<NativeBackend<MemoryStore>>,
    String,
    CourseId,
    AssignmentId,
) {
    let (backend, store, context, reference, _question, _attempt, _correct, _incorrect) =
        published_flat_fixture().await;
    let tenant = context.tenant_id();
    let instructor = UserId::from_uuid(uuid(102));
    let student = UserId::from_uuid(uuid(120));
    let course = CourseId::from_uuid(uuid(121));
    let assignment = AssignmentId::from_uuid(uuid(122));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Retry semantics".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: crate::test_fixtures::sysadmin_course_creation_authority(
                    store.as_ref(),
                    tenant,
                    course,
                    instructor,
                )
                .await,
            },
        )
        .await
        .expect("retry fixture course saves");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Retry learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("retry fixture student membership");
    store
        .create_assignment(
            context,
            learning_data_access::CreateAssignmentCommand {
                actor: instructor,
                assignment: AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    audience: question_model::AssignmentAudience::CourseWide,
                    title: "Retry semantics".to_string(),
                    lifecycle: question_model::AssignmentLifecycle::Draft,
                    instructions: question_model::AssignmentInstructions::default(),
                    items: vec![AssignmentItem {
                        id: AssignmentItemId::from_uuid(uuid(123)),
                        reference,
                        position: 0,
                        points_possible: PointValue::from_whole(1),
                        delivery_state: question_model::AssignmentDeliveryState::Active,
                        scoring_mode: question_model::AssignmentScoringMode::Normal,
                    }],
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: RunPolicies {
                        completion: CompletionRequirement::AllCorrect,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: VariationPolicy::NewSeeds,
                    },
                },
                base_policy: question_model::BaseAssignmentPolicy::default(),
            },
        )
        .await
        .expect("retry fixture assignment saves");
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
    let subject = SessionSubject::new(tenant, student, "Retry student", vec![UserRole::Student])
        .expect("retry fixture session subject");
    let issued = crate::auth::issue_session(
        store.as_ref(),
        subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("retry fixture session lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("retry fixture session issues");
    let cookie = issued
        .set_cookie
        .split(';')
        .next()
        .expect("retry fixture cookie pair")
        .to_string();
    let backend = Arc::new(backend);
    let app = crate::run::router(
        Arc::clone(&store),
        Arc::clone(&backend),
        Arc::new(
            learning_data_access::in_memory::MemorySealedPrivateExecutionStore::new(Arc::clone(
                &store,
            )),
        ),
        Arc::clone(&store) as Arc<dyn learning_data_access::LearnerSubmissionStatusStore>,
        Arc::clone(&store) as Arc<dyn learning_data_access::AutomatedGradingStore>,
    );
    (app, store, backend, cookie, course, assignment)
}

fn post_json(path: &str, cookie: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("cookie", cookie)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("retry fixture request")
}

pub(super) fn submission_json(
    path: &str,
    cookie: &str,
    idempotency_key: &'static str,
    body: serde_json::Value,
) -> Request<Body> {
    let mut request = post_json(path, cookie, body);
    request
        .headers_mut()
        .insert("idempotency-key", HeaderValue::from_static(idempotency_key));
    request
}

pub(super) async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), 256 * 1_024)
        .await
        .expect("retry fixture response body");
    serde_json::from_slice(&bytes).expect("retry fixture response JSON")
}

pub(super) async fn active_attempt_id(app: &Router, run: &str, cookie: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{run}/attempts"))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("retry fixture attempts request"),
        )
        .await
        .expect("retry fixture attempts response");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "attempt list remains available"
    );
    response_json(response)
        .await
        .get("items")
        .and_then(serde_json::Value::as_array)
        .and_then(|attempts| {
            attempts.iter().find_map(|attempt| {
                attempt
                    .get("status")
                    .filter(|status| *status == "in_progress")
                    .and_then(|_| attempt.get("id"))
                    .and_then(serde_json::Value::as_str)
            })
        })
        .map(str::to_string)
        .expect("an active retry attempt is issued")
}

pub(super) async fn rendered_choice_id(
    app: &Router,
    course: CourseId,
    assignment: AssignmentId,
    attempt: &str,
    cookie: &str,
    label: &str,
) -> ChoiceId {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/question"
                ))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("rendered choice request"),
        )
        .await
        .expect("rendered choice response");
    assert_eq!(response.status(), StatusCode::OK);
    let envelope = response_json(response).await;
    let expected_body = serde_json::json!([{
        "kind": "text",
        "markdown": label,
    }]);
    let identifier = envelope
        .pointer("/response/choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| {
            choices.iter().find_map(|choice| {
                (choice.get("body") == Some(&expected_body))
                    .then(|| choice.get("id").and_then(serde_json::Value::as_str))
                    .flatten()
            })
        })
        .expect("visible choice has a rendered ID");
    ChoiceId::new(identifier)
}
