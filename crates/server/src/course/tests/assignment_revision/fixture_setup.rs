use super::super::fixtures::{
    id, issued_cookie_for_tenant, publish_fixture, publish_fixture_with_identity,
};
use super::super::*;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use axum::http::header::{ETAG, IF_MATCH};
use axum::response::Response;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    CatalogStore, CourseRecord, CourseRosterStore, CreateCourseCommand, Store, TenantContext,
    UpsertCourseMember,
};
use question_model::{AssignmentId, CourseId, QuestionId, TenantId, UserId, UserRole};
use std::sync::Arc;
use tower::ServiceExt;

pub(crate) struct AssignmentFixture {
    pub(crate) store: Arc<MemoryStore>,
    pub(crate) context: TenantContext,
    pub(crate) instructor: UserId,
    pub(crate) course: CourseId,
    pub(crate) student: UserId,
    pub(crate) question_id: QuestionId,
    pub(crate) replacement_question_id: QuestionId,
    pub(crate) instructor_cookie: String,
    pub(crate) student_cookie: String,
    pub(crate) outsider_cookie: String,
    pub(crate) app: axum::Router,
}

pub(crate) struct AssignmentState {
    pub(crate) fixture: AssignmentFixture,
    pub(crate) assignment: AssignmentId,
    pub(crate) etag: String,
    pub(crate) content: serde_json::Value,
}

pub(crate) async fn build() -> AssignmentFixture {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(8_200));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(8_201));
    let course = CourseId::from_uuid(id(8_202));
    let student = UserId::from_uuid(id(8_203));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Biochemistry".to_string(),
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
        .expect("course save");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Biochemistry learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("student membership save");
    let reference = publish_fixture(&store, context, tenant, instructor).await;
    let question_id = store
        .get_catalog_problem(context, reference)
        .await
        .expect("catalog lookup")
        .expect("published fixture")
        .question_id;
    let replacement_reference =
        publish_fixture_with_identity(&store, context, tenant, instructor, 8_220).await;
    let replacement_question_id = store
        .get_catalog_problem(context, replacement_reference)
        .await
        .expect("replacement catalog lookup")
        .expect("published replacement fixture")
        .question_id;
    let instructor_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let student_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Student], student).await;
    let outsider_cookie = issued_cookie_for_tenant(
        &store,
        tenant,
        vec![UserRole::Instructor],
        UserId::from_uuid(id(8_204)),
    )
    .await;
    AssignmentFixture {
        app: router(Arc::clone(&store)),
        store,
        context,
        instructor,
        course,
        student,
        question_id,
        replacement_question_id,
        instructor_cookie,
        student_cookie,
        outsider_cookie,
    }
}

pub(crate) fn request(
    method: &str,
    uri: impl Into<String>,
    cookie: &str,
    etag: Option<&str>,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri.into())
        .header("cookie", cookie);
    if let Some(etag) = etag {
        builder = builder.header(IF_MATCH, etag);
    }
    let body = match body {
        Some(body) => {
            builder = builder.header("content-type", "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    builder.body(body).expect("fixture request")
}

pub(crate) async fn response_json(response: Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), 128 * 1024)
        .await
        .expect("bounded response body");
    serde_json::from_slice(&body).expect("JSON response")
}

pub(crate) async fn create_assignment(fixture: AssignmentFixture) -> AssignmentState {
    let draft = fixture
        .app
        .clone()
        .oneshot(request(
            "POST",
            format!("/api/courses/{}/assignments/drafts", fixture.course),
            &fixture.instructor_cookie,
            None,
            Some(serde_json::json!({"title": "Peptide practice"})),
        ))
        .await
        .expect("draft response");
    assert_eq!(draft.status(), StatusCode::CREATED);
    let etag = draft
        .headers()
        .get(ETAG)
        .expect("draft ETag")
        .to_str()
        .expect("draft ETag text")
        .to_owned();
    let draft = response_json(draft).await;
    let assignment = serde_json::from_value(draft["id"].clone()).expect("assignment ID");
    let entries = serde_json::json!([{
        "kind": "fixed",
        "questionId": fixture.question_id,
        "position": 0,
        "pointsPossible": "1",
        "deliveryState": "active",
        "scoringMode": "normal"
    }]);
    let content_response = fixture
        .app
        .clone()
        .oneshot(request(
            "PUT",
            format!(
                "/api/courses/{}/assignments/{assignment}/content",
                fixture.course
            ),
            &fixture.instructor_cookie,
            Some(&etag),
            Some(serde_json::json!({"title": "Peptide practice", "entries": entries})),
        ))
        .await
        .expect("content response");
    assert_eq!(content_response.status(), StatusCode::OK);
    let etag = content_response
        .headers()
        .get(ETAG)
        .expect("content ETag")
        .to_str()
        .expect("content ETag text")
        .to_owned();
    let content = response_json(content_response).await;
    AssignmentState {
        fixture,
        assignment,
        etag,
        content,
    }
}
