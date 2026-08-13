use super::*;

use axum::body::{Body, to_bytes};
use axum::http::Request;
use axum::response::Response;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{PageRequest, PageSize, SessionLifetime, SessionSubject, TenantContext};
use question_model::{ActivityTimestamp, TenantId, UserId, UserRole};
use tower::ServiceExt;
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

async fn issued_cookie(store: &MemoryStore, roles: Vec<UserRole>, user: UserId) -> String {
    let tenant = TenantId::from_uuid(id(1));
    let subject =
        SessionSubject::new(tenant, user, "Course Fixture", roles).expect("fixture identity");
    let issued = crate::auth::issue_session(
        store,
        subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("positive lifetime"),
            crate::auth::CookieTransport::LocalHttp,
        ),
    )
    .await
    .expect("fixture session");
    issued
        .set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string()
}

async fn response_json(response: Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), 128 * 1_024)
        .await
        .expect("response body");
    serde_json::from_slice(&body).expect("JSON response")
}

#[tokio::test]
async fn course_creation_rejects_invalid_requests_and_student_callers_without_persistence() {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(2));
    let student = UserId::from_uuid(id(3));
    let sysadmin = UserId::from_uuid(id(4));
    let instructor_cookie = issued_cookie(&store, vec![UserRole::Instructor], instructor).await;
    let student_cookie = issued_cookie(&store, vec![UserRole::Student], student).await;
    let sysadmin_cookie = issued_cookie(&store, vec![UserRole::Sysadmin], sysadmin).await;
    let app = router(Arc::clone(&store));

    let student_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/courses")
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"Student cannot create this course"}"#,
                ))
                .expect("student course request"),
        )
        .await
        .expect("student course response");
    assert_eq!(student_create.status(), StatusCode::FORBIDDEN);

    for invalid_body in [
        r#"{"title":"   "}"#,
        r#"{"title":"BIOC 301","role":"sysadmin"}"#,
    ] {
        let rejected_course = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/courses")
                    .header("cookie", &instructor_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(invalid_body))
                    .expect("invalid course request"),
            )
            .await
            .expect("invalid course response");
        assert_eq!(rejected_course.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    for (cookie, expected_message) in [
        (
            &student_cookie,
            "rejected student creation must not persist a course",
        ),
        (
            &instructor_cookie,
            "invalid course requests must not persist a course",
        ),
    ] {
        let courses = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/courses")
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .expect("course list request"),
            )
            .await
            .expect("course list response");
        assert_eq!(courses.status(), StatusCode::OK);
        assert_eq!(
            response_json(courses).await["items"],
            serde_json::json!([]),
            "{expected_message}",
        );
    }

    let sysadmin_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/courses")
                .header("cookie", &sysadmin_cookie)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"BIOC 301: Biochemistry"}"#))
                .expect("sysadmin course request"),
        )
        .await
        .expect("sysadmin course response");
    assert_eq!(sysadmin_create.status(), StatusCode::CREATED);
    let created = response_json(sysadmin_create).await;
    assert_eq!(created["role"], "instructor");

    let sysadmin_courses = store
        .list_courses(
            context,
            learning_data_access::CourseListScope::Member(sysadmin),
            PageRequest::first(PageSize::new(50).expect("valid page size")),
        )
        .await
        .expect("sysadmin-created direct course list");
    assert_eq!(sysadmin_courses.items.len(), 1);
}
