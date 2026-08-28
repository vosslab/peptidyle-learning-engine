use super::fixtures::{id, issued_cookie_for_tenant};
use super::*;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{CACHE_CONTROL, ETAG, IF_MATCH};
use axum::http::{Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    CourseRecord, CourseRosterStore, CreateCourseCommand, PageRequest, PageSize,
    RevokeCourseMember, Store, TenantContext, UpsertCourseMember,
};
use question_model::{
    AssignmentId, CourseId, GradingOperationReference, QuestionAttemptId, TenantId, UserId,
    UserRole,
};
use std::sync::Arc;
use tower::ServiceExt;

struct Fixture {
    store: Arc<MemoryStore>,
    app: Router,
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    instructor_cookie: String,
    sysadmin_cookie: String,
    student_cookie: String,
    outsider_cookie: String,
    foreign_cookie: String,
}

async fn fixture() -> Fixture {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(91_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(91_001));
    let student = UserId::from_uuid(id(91_002));
    let course = CourseId::from_uuid(id(91_003));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Grading operations fixture".to_owned(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("fixture term"),
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
        .expect("fixture course");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Fixture learner".to_owned(),
                roster_contact: None,
            },
        )
        .await
        .expect("fixture student");
    let instructor_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let sysadmin_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Sysadmin], instructor).await;
    let student_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Student], student).await;
    let outsider_cookie = issued_cookie_for_tenant(
        &store,
        tenant,
        vec![UserRole::Instructor],
        UserId::from_uuid(id(91_004)),
    )
    .await;
    let foreign_cookie = issued_cookie_for_tenant(
        &store,
        TenantId::from_uuid(id(91_005)),
        vec![UserRole::Instructor],
        UserId::from_uuid(id(91_006)),
    )
    .await;
    let app = router(Arc::clone(&store));
    let response = app
        .clone()
        .oneshot(
            Request::post(format!("/api/courses/{course}/assignments/drafts"))
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"Grading operation assignment"}"#))
                .expect("draft request"),
        )
        .await
        .expect("draft response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = to_bytes(response.into_body(), 16 * 1024)
        .await
        .expect("draft body");
    let body: serde_json::Value = serde_json::from_slice(&body).expect("draft JSON");
    let assignment = serde_json::from_value(body["id"].clone()).expect("assignment ID");
    Fixture {
        store,
        app,
        tenant,
        course,
        assignment,
        instructor_cookie,
        sysadmin_cookie,
        student_cookie,
        outsider_cookie,
        foreign_cookie,
    }
}

async fn revoke_instructor(fixture: &Fixture) {
    let request = Request::get("/api/auth/session")
        .header("cookie", &fixture.instructor_cookie)
        .body(Body::empty())
        .expect("session request");
    let authenticated =
        crate::auth::resolve_request_session(fixture.store.as_ref(), request.headers())
            .await
            .expect("fixture instructor session");
    let membership = fixture
        .store
        .get_current_course_membership(
            authenticated.tenant_context,
            fixture.course,
            authenticated.record.subject.user(),
        )
        .await
        .expect("current instructor membership")
        .expect("instructor membership");
    let roster = fixture
        .store
        .list_course_roster(
            authenticated.tenant_context,
            authenticated.session_hash,
            fixture.course,
            PageRequest::first(PageSize::new(1).expect("bounded roster page")),
        )
        .await
        .expect("roster policy");
    fixture
        .store
        .revoke_course_member(
            authenticated.tenant_context,
            authenticated.session_hash,
            RevokeCourseMember {
                course: fixture.course,
                member: learning_data_access::CourseMemberId::from_uuid(membership.id.as_uuid()),
                expected_revision: roster.policy.revision,
            },
        )
        .await
        .expect("revoke instructor membership");
}

#[tokio::test]
async fn grading_operation_list_is_instructor_only_bounded_and_no_store() {
    let fixture = fixture().await;
    let recalculate = fixture
        .app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/courses/{}/assignments/{}/grading-operations/recalculate",
                fixture.course, fixture.assignment
            ))
            .header("cookie", &fixture.instructor_cookie)
            .header(IF_MATCH, "\"1\"")
            .header("idempotency-key", "019c0000-0000-7000-8000-000000000005")
            .body(Body::empty())
            .expect("recalculation request"),
        )
        .await
        .expect("recalculation response");
    assert_eq!(recalculate.status(), StatusCode::OK);
    let path = format!(
        "/api/courses/{}/assignments/{}/grading-operations?groupBy=question&pageSize=1",
        fixture.course, fixture.assignment
    );
    let response = fixture
        .app
        .clone()
        .oneshot(
            Request::get(&path)
                .header("cookie", &fixture.instructor_cookie)
                .body(Body::empty())
                .expect("list request"),
        )
        .await
        .expect("list response");
    let response_status = response.status();
    let response_cache_control = response.headers().get(CACHE_CONTROL).cloned();
    let response_body = to_bytes(response.into_body(), 16 * 1024)
        .await
        .expect("list body");
    assert_eq!(response_status, StatusCode::OK);
    assert_eq!(response_cache_control, Some("no-store".parse().unwrap()));
    let body: serde_json::Value = serde_json::from_slice(&response_body).expect("list JSON");
    assert_eq!(body["items"].as_array().map(Vec::len), Some(1));
    assert!(body["nextCursor"].is_null());
    let item = &body["items"][0];
    assert_eq!(item["operation"]["state"], "action_in_progress");
    for forbidden in [
        "answer",
        "response",
        "feedback",
        "score",
        "submission",
        "attempt",
    ] {
        assert!(!item.to_string().contains(forbidden));
    }

    let invalid = fixture
        .app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/courses/{}/assignments/{}/grading-operations?groupBy=question&groupBy=learner",
                fixture.course, fixture.assignment
            ))
            .header("cookie", &fixture.instructor_cookie)
            .body(Body::empty())
            .expect("invalid list request"),
        )
        .await
        .expect("invalid list response");
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    for cookie in [
        &fixture.student_cookie,
        &fixture.sysadmin_cookie,
        &fixture.outsider_cookie,
        &fixture.foreign_cookie,
    ] {
        let response = fixture
            .app
            .clone()
            .oneshot(
                Request::get(&path)
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .expect("concealed request"),
            )
            .await
            .expect("concealed response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(CACHE_CONTROL),
            Some(&"no-store".parse().unwrap())
        );
    }

    revoke_instructor(&fixture).await;
    let revoked = fixture
        .app
        .clone()
        .oneshot(
            Request::get(&path)
                .header("cookie", &fixture.instructor_cookie)
                .body(Body::empty())
                .expect("revoked instructor request"),
        )
        .await
        .expect("revoked instructor response");
    assert_eq!(revoked.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        revoked.headers().get(CACHE_CONTROL),
        Some(&"no-store".parse().unwrap())
    );
}

#[tokio::test]
async fn recalculation_requires_closed_headers_and_replays_one_metadata_only_receipt() {
    let fixture = fixture().await;
    let path = format!(
        "/api/courses/{}/assignments/{}/grading-operations/recalculate",
        fixture.course, fixture.assignment
    );
    let missing = fixture
        .app
        .clone()
        .oneshot(
            Request::post(&path)
                .header("cookie", &fixture.instructor_cookie)
                .body(Body::empty())
                .expect("missing header request"),
        )
        .await
        .expect("missing header response");
    assert_eq!(missing.status(), StatusCode::PRECONDITION_REQUIRED);

    let nonempty = fixture
        .app
        .clone()
        .oneshot(
            Request::post(&path)
                .header("cookie", &fixture.instructor_cookie)
                .header(IF_MATCH, "\"1\"")
                .header("idempotency-key", "019c0000-0000-7000-8000-000000000001")
                .body(Body::from("x"))
                .expect("nonempty request"),
        )
        .await
        .expect("nonempty response");
    assert_eq!(nonempty.status(), StatusCode::BAD_REQUEST);

    let action = "019c0000-0000-7000-8000-000000000002";
    let request = || {
        Request::post(&path)
            .header("cookie", &fixture.instructor_cookie)
            .header(IF_MATCH, "\"1\"")
            .header("idempotency-key", action)
            .body(Body::empty())
            .expect("recalculation request")
    };
    let first = fixture
        .app
        .clone()
        .oneshot(request())
        .await
        .expect("first recalculation response");
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(
        first.headers().get(CACHE_CONTROL),
        Some(&"no-store".parse().unwrap())
    );
    assert_eq!(first.headers().get(ETAG), Some(&"\"1\"".parse().unwrap()));
    let first = to_bytes(first.into_body(), 16 * 1024)
        .await
        .expect("first body");
    let first: serde_json::Value = serde_json::from_slice(&first).expect("first JSON");
    assert_eq!(first["kind"], "recalculation");
    for forbidden in [
        "answer",
        "response",
        "feedback",
        "score",
        "submission",
        "attempt",
        "job",
    ] {
        assert!(
            !first
                .as_object()
                .expect("receipt object")
                .contains_key(forbidden)
        );
    }

    let replay = fixture
        .app
        .clone()
        .oneshot(request())
        .await
        .expect("replay response");
    assert_eq!(replay.status(), StatusCode::OK);
    let replay = to_bytes(replay.into_body(), 16 * 1024)
        .await
        .expect("replay body");
    let replay: serde_json::Value = serde_json::from_slice(&replay).expect("replay JSON");
    assert_eq!(replay, first);
}

#[tokio::test]
async fn retry_route_requires_a_strong_operation_revision_and_empty_idempotent_action() {
    let fixture = fixture().await;
    fixture
        .store
        .seed_retryable_grading_operation_for_test(
            fixture.tenant,
            fixture.course,
            fixture.assignment,
            GradingOperationReference::new(1).expect("operation reference"),
            QuestionAttemptId::from_uuid(id(91_007)),
            learning_data_access::AcceptedSubmissionId::from_uuid(id(91_008)),
        )
        .expect("retryable grading operation");
    let path = format!(
        "/api/courses/{}/assignments/{}/grading-operations/GO-1/retry",
        fixture.course, fixture.assignment
    );
    let malformed = fixture
        .app
        .clone()
        .oneshot(
            Request::post(&path)
                .header("cookie", &fixture.instructor_cookie)
                .header(IF_MATCH, "1")
                .header("idempotency-key", "019c0000-0000-7000-8000-000000000003")
                .body(Body::empty())
                .expect("malformed retry request"),
        )
        .await
        .expect("malformed retry response");
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);

    let nonempty = fixture
        .app
        .clone()
        .oneshot(
            Request::post(&path)
                .header("cookie", &fixture.instructor_cookie)
                .header(IF_MATCH, "\"1\"")
                .header("idempotency-key", "019c0000-0000-7000-8000-000000000004")
                .body(Body::from("x"))
                .expect("nonempty retry request"),
        )
        .await
        .expect("nonempty retry response");
    assert_eq!(nonempty.status(), StatusCode::BAD_REQUEST);

    let action = "019c0000-0000-7000-8000-000000000009";
    let request = || {
        Request::post(&path)
            .header("cookie", &fixture.instructor_cookie)
            .header(IF_MATCH, "\"1\"")
            .header("idempotency-key", action)
            .body(Body::empty())
            .expect("retry request")
    };
    let first = fixture
        .app
        .clone()
        .oneshot(request())
        .await
        .expect("first retry response");
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(
        first.headers().get(CACHE_CONTROL),
        Some(&"no-store".parse().unwrap())
    );
    assert_eq!(first.headers().get(ETAG), Some(&"\"2\"".parse().unwrap()));
    let first = to_bytes(first.into_body(), 16 * 1024)
        .await
        .expect("first retry body");
    let first: serde_json::Value = serde_json::from_slice(&first).expect("first retry JSON");
    assert_eq!(first["kind"], "retry");
    for forbidden in [
        "answer",
        "response",
        "feedback",
        "score",
        "submission",
        "attempt",
    ] {
        assert!(!first.to_string().contains(forbidden));
    }

    let replay = fixture
        .app
        .clone()
        .oneshot(request())
        .await
        .expect("retry replay response");
    assert_eq!(replay.status(), StatusCode::OK);
    let replay = to_bytes(replay.into_body(), 16 * 1024)
        .await
        .expect("retry replay body");
    let replay: serde_json::Value = serde_json::from_slice(&replay).expect("retry replay JSON");
    assert_eq!(replay, first);

    let duplicate = fixture
        .app
        .clone()
        .oneshot(
            Request::post(&path)
                .header("cookie", &fixture.instructor_cookie)
                .header(IF_MATCH, "\"1\"")
                .header("idempotency-key", "019c0000-0000-7000-8000-000000000010")
                .body(Body::empty())
                .expect("duplicate-effect probe request"),
        )
        .await
        .expect("duplicate-effect probe response");
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
}
