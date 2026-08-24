use super::*;

use axum::body::{Body, to_bytes};
use axum::http::Request;
use axum::response::Response;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AccountIdentityStore, ApproveInstructorAccount, AuthenticationEmail,
    AuthenticationRateLimitKey, BeginEmailAuthentication, BrowserBindingHash,
    CompleteEmailAuthentication, EmailAuthenticationPurpose, EmailChallengeId,
    EmailChallengeLifetime, EmailChallengeSecretHash, PageRequest, PageSize, SessionLifetime,
    SessionSubject, TeachingAuthorityStore, TenantContext,
};
use question_model::{ActivityTimestamp, TenantId, UserId, UserRole};
use tower::ServiceExt;
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

async fn issued_cookie(
    store: &MemoryStore,
    roles: Vec<UserRole>,
    user: UserId,
) -> (String, learning_data_access::SessionTokenHash) {
    let tenant = TenantId::from_uuid(id(1));
    let subject =
        SessionSubject::new(tenant, user, "Course Fixture", roles).expect("fixture identity");
    let issued = crate::auth::issue_session(
        store,
        subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("positive lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("fixture session");
    let cookie = issued
        .set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string();
    (cookie, issued.record.token_hash)
}

async fn create_account(store: &MemoryStore, user: UserId, suffix: u128) {
    let token =
        EmailChallengeSecretHash::compute(format!("course-create-token-{suffix}").as_bytes());
    let binding = BrowserBindingHash::compute(format!("course-create-binding-{suffix}").as_bytes());
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(id(100 + suffix)),
            token_hash: token,
            browser_binding: binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(
                format!("course-create-rate-{suffix}").as_bytes(),
            ),
            email: AuthenticationEmail::parse(&format!("course-create-{suffix}@example.edu"))
                .expect("fixture email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("fixture lifetime"),
        })
        .await
        .expect("account challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash: token,
            browser_binding: binding,
            proposed_user: user,
            proposed_display_name: "Course instructor".to_owned(),
        })
        .await
        .expect("fixture account");
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
    create_account(&store, instructor, 2).await;
    let (instructor_cookie, _) =
        issued_cookie(&store, vec![UserRole::Instructor], instructor).await;
    let (student_cookie, _) = issued_cookie(&store, vec![UserRole::Student], student).await;
    let (sysadmin_cookie, sysadmin_session) =
        issued_cookie(&store, vec![UserRole::Sysadmin], sysadmin).await;
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
        r#"{"title":"   ","term":{"startDate":"2026-08-24","endDate":"2026-12-18","timeZone":"America/Chicago"}}"#,
        r#"{"title":"BIOC 301","term":{"startDate":"2026-08-24","endDate":"2026-12-18","timeZone":"America/Chicago"},"role":"sysadmin"}"#,
        r#"{"title":"BIOC 301","term":{"startDate":"2026-08-24","endDate":"2026-12-18","timeZone":"America/Chicago"},"actor":"00000000-0000-0000-0000-000000000004"}"#,
        r#"{"title":"BIOC 301","term":{"startDate":"2026-08-24","endDate":"2026-12-18","timeZone":"America/Chicago","offset":"-06:00"}}"#,
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

    for (invalid_body, field, reason, message) in [
        (
            r#"{"title":"BIOC 301"}"#,
            "term",
            "required",
            "Enter the course term dates and time zone.",
        ),
        (
            r#"{"title":"BIOC 301","term":{"endDate":"2026-12-18","timeZone":"America/Chicago"}}"#,
            "startDate",
            "required",
            "Enter a course start date.",
        ),
        (
            r#"{"title":"BIOC 301","term":{"startDate":"2026-08-24","timeZone":"America/Chicago"}}"#,
            "endDate",
            "required",
            "Enter a course end date.",
        ),
        (
            r#"{"title":"BIOC 301","term":{"startDate":"2026-08-24","endDate":"2026-12-18"}}"#,
            "timeZone",
            "required",
            "Enter an IANA time zone.",
        ),
        (
            r#"{"title":"BIOC 301","term":{"startDate":"2026-02-30","endDate":"2026-12-18","timeZone":"America/Chicago"}}"#,
            "startDate",
            "invalidCalendarDate",
            "Enter a valid date in YYYY-MM-DD format.",
        ),
        (
            r#"{"title":"BIOC 301","term":{"startDate":"2026-12-19","endDate":"2026-12-18","timeZone":"America/Chicago"}}"#,
            "endDate",
            "endBeforeStart",
            "Choose an end date on or after the start date.",
        ),
        (
            r#"{"title":"BIOC 301","term":{"startDate":"2026-08-24","endDate":"2026-02-30","timeZone":"America/Chicago"}}"#,
            "endDate",
            "invalidCalendarDate",
            "Enter a valid date in YYYY-MM-DD format.",
        ),
        (
            r#"{"title":"BIOC 301","term":{"startDate":"2026-08-24","endDate":"2026-12-18","timeZone":"america/chicago"}}"#,
            "timeZone",
            "unknownIanaTimeZone",
            "Choose a valid IANA time zone such as America/Chicago.",
        ),
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
                    .expect("invalid course term request"),
            )
            .await
            .expect("invalid course term response");
        assert_eq!(rejected_course.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response_json(rejected_course).await,
            serde_json::json!({
                "error": "courseTermInvalid",
                "field": field,
                "reason": reason,
                "message": message,
            })
        );
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

    let unapproved_instructor_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/courses")
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"BIOC 302: Enzymes","term":{"startDate":"2027-01-11","endDate":"2027-05-07","timeZone":"Europe/Paris"}}"#,
                ))
                .expect("unapproved instructor course request"),
        )
        .await
        .expect("unapproved instructor course response");
    assert_eq!(unapproved_instructor_create.status(), StatusCode::FORBIDDEN);

    let instructor_courses = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/courses")
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("unapproved instructor course list request"),
        )
        .await
        .expect("unapproved instructor course list response");
    assert_eq!(instructor_courses.status(), StatusCode::OK);
    assert_eq!(
        response_json(instructor_courses).await["items"],
        serde_json::json!([])
    );

    let sysadmin_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/courses")
                .header("cookie", &sysadmin_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"BIOC 301: Biochemistry","term":{"startDate":"2026-08-24","endDate":"2026-12-18","timeZone":"America/Chicago"}}"#,
                ))
                .expect("sysadmin course request"),
        )
        .await
        .expect("sysadmin course response");
    assert_eq!(sysadmin_create.status(), StatusCode::CREATED);
    let created = response_json(sysadmin_create).await;
    assert_eq!(created["role"], "instructor");
    assert_eq!(
        created["term"],
        serde_json::json!({
            "startDate": "2026-08-24",
            "endDate": "2026-12-18",
            "timeZone": "America/Chicago",
        })
    );

    store
        .approve_instructor_account(
            context,
            ApproveInstructorAccount {
                session: sysadmin_session,
                target: instructor,
                expected_revision: None,
            },
        )
        .await
        .expect("authenticated sysadmin approval");

    let instructor_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/courses")
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"BIOC 302: Enzymes","term":{"startDate":"2027-01-11","endDate":"2027-05-07","timeZone":"Europe/Paris"}}"#,
                ))
                .expect("instructor course request"),
        )
        .await
        .expect("instructor course response");
    assert_eq!(instructor_create.status(), StatusCode::CREATED);
    assert_eq!(
        response_json(instructor_create).await["term"]["timeZone"],
        "Europe/Paris"
    );

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
