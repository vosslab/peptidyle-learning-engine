use super::*;
use axum::body::{Body, to_bytes};
use axum::http::header::{CONTENT_TYPE, ETAG, IF_MATCH};
use axum::http::{Method, Request, StatusCode};
use axum::response::Response;
use learning_data_access::Store;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    ClaimedJob, CourseRecord, CourseRosterStore, CreateCourseCommand, JobLeaseDuration, JobPayload,
    JobStore, RETENTION_ARCHIVE_NOTIFICATION_COPY, RetentionDispatchBatch, RetentionScheduleStore,
    RetentionWorkerCommand, RetentionWorkerStore, SessionLifetime, SessionSubject, TenantContext,
    UpsertCourseMember,
};
use question_model::{
    ActivityTimestamp, CourseId, CourseMembershipRole, TenantId, UserId, UserRole,
};
use tower::ServiceExt;
use uuid::Uuid;

use crate::auth::{CookieTransport, SessionConfig, issue_session};

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn course_route(course: CourseId, suffix: &str) -> String {
    format!("/api/courses/{course}/retention{suffix}")
}

fn assert_no_store(response: &Response) {
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .expect("cache-control header")
            .to_str()
            .expect("cache-control value"),
        "no-store"
    );
}

fn assert_private_projection_fields(value: &serde_json::Value) {
    let object = value.as_object().expect("object response");
    for key in [
        "policy",
        "deadline",
        "generation",
        "stage",
        "job",
        "lease",
        "recipient",
        "tenant",
        "user",
        "student",
        "object",
        "source",
        "provider",
        "answer",
        "key",
        "grading",
    ] {
        assert!(!object.contains_key(key), "field {key} must be excluded");
    }
    if let Some(notification) = object.get("notification") {
        let notification = notification
            .as_object()
            .expect("notification response should be an object");
        for key in [
            "policy",
            "deadline",
            "generation",
            "stage",
            "job",
            "lease",
            "recipient",
            "tenant",
            "user",
            "student",
            "object",
            "source",
            "provider",
            "answer",
            "key",
            "grading",
        ] {
            assert!(
                !notification.contains_key(key),
                "notification field {key} excluded"
            );
        }
    }
}

async fn response_json(response: Response) -> serde_json::Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), 128 * 1_024)
            .await
            .expect("response body"),
    )
    .expect("json response")
}

async fn issued_cookie(
    store: &MemoryStore,
    tenant: TenantId,
    roles: Vec<UserRole>,
    user: UserId,
) -> String {
    let issued = issue_session(
        store,
        SessionSubject::new(tenant, user, "Retention fixture", roles).expect("fixture identity"),
        SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
            CookieTransport::LocalHttp,
        ),
    )
    .await
    .expect("session issued");
    issued
        .set_cookie
        .split(';')
        .next()
        .expect("set-cookie")
        .to_string()
}

async fn create_course(
    store: &MemoryStore,
    tenant: TenantId,
    course: CourseId,
    members: Vec<(UserId, CourseMembershipRole)>,
) {
    let context = TenantContext::from_authenticated_session(tenant);
    let initial_instructor = members
        .iter()
        .find_map(|(user, role)| (*role == CourseMembershipRole::Instructor).then_some(*user))
        .expect("retention fixture needs an initial instructor");
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
                initial_instructor,
            },
        )
        .await
        .expect("course persisted");
    for (user, role) in members {
        if role == CourseMembershipRole::Student {
            store
                .upsert_course_member(
                    context,
                    UpsertCourseMember {
                        course,
                        user,
                        display_name: "Retention learner".to_string(),
                        roster_contact: None,
                    },
                )
                .await
                .expect("student roster membership");
        }
    }
}

async fn make_request(
    app: &axum::Router,
    method: Method,
    uri: String,
    cookie: Option<&str>,
    if_match: &[&str],
    content_type: Option<&str>,
    body: &str,
) -> Response {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    for header in if_match {
        request = request.header(IF_MATCH, *header);
    }
    if let Some(content_type) = content_type {
        request = request.header(CONTENT_TYPE, content_type);
    }
    let response = app
        .clone()
        .oneshot(
            request
                .body(Body::from(body.to_owned()))
                .expect("request body"),
        )
        .await
        .expect("router response");
    assert_no_store(&response);
    response
}

async fn end_retention(
    app: &axum::Router,
    cookie: Option<&str>,
    course: CourseId,
    body: &str,
) -> Response {
    make_request(
        app,
        Method::POST,
        course_route(course, "/end"),
        cookie,
        &[],
        None,
        body,
    )
    .await
}

async fn get_retention(app: &axum::Router, cookie: Option<&str>, course: CourseId) -> Response {
    make_request(
        app,
        Method::GET,
        course_route(course, ""),
        cookie,
        &[],
        None,
        "",
    )
    .await
}

async fn archive_retention(
    app: &axum::Router,
    cookie: Option<&str>,
    course: CourseId,
    if_match: &[&str],
    content_type: Option<&str>,
    body: &str,
) -> Response {
    make_request(
        app,
        Method::POST,
        course_route(course, "/archive"),
        cookie,
        if_match,
        content_type,
        body,
    )
    .await
}

async fn delete_retention(
    app: &axum::Router,
    cookie: Option<&str>,
    course: CourseId,
    if_match: &[&str],
    body: &str,
) -> Response {
    make_request(
        app,
        Method::POST,
        course_route(course, "/delete"),
        cookie,
        if_match,
        Some("application/json"),
        body,
    )
    .await
}

async fn extend_retention(
    app: &axum::Router,
    cookie: Option<&str>,
    course: CourseId,
    if_match: &[&str],
    content_type: Option<&str>,
    body: &str,
) -> Response {
    make_request(
        app,
        Method::PATCH,
        course_route(course, "/extend"),
        cookie,
        if_match,
        content_type,
        body,
    )
    .await
}

fn worker_command_from_claim(claim: ClaimedJob) -> RetentionWorkerCommand {
    let (command_course, stage, generation) = match claim.payload {
        JobPayload::Retention {
            course,
            stage,
            generation,
        } => (course, stage, generation),
        _ => panic!("worker job is not retention payload"),
    };
    RetentionWorkerCommand {
        tenant: claim.tenant,
        course: command_course,
        stage,
        generation,
        job: claim.id,
        lease: claim.lease_token,
    }
}

#[tokio::test]
async fn retention_routes_require_session_before_body_and_if_match() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let course = CourseId::from_uuid(id(2));
    let instructor = UserId::from_uuid(id(3));
    create_course(
        &store,
        tenant,
        course,
        vec![(instructor, CourseMembershipRole::Instructor)],
    )
    .await;
    let app = router(Arc::clone(&store));

    let no_session_archive = archive_retention(
        &app,
        None,
        course,
        &["\"1\""],
        Some("application/json"),
        r#"{"assignmentDefinitions":"retain"}"#,
    )
    .await;
    assert_eq!(no_session_archive.status(), StatusCode::UNAUTHORIZED);
    assert_no_store(&no_session_archive);

    let no_session_delete = delete_retention(&app, None, course, &[], r#"{}"#).await;
    assert_eq!(no_session_delete.status(), StatusCode::UNAUTHORIZED);
    assert_no_store(&no_session_delete);

    let no_session_extend = extend_retention(
        &app,
        None,
        course,
        &[],
        Some("application/json"),
        r#"{"additionalDays":7}"#,
    )
    .await;
    assert_eq!(no_session_extend.status(), StatusCode::UNAUTHORIZED);
    assert_no_store(&no_session_extend);

    let no_session_end = end_retention(&app, None, course, "{}").await;
    assert_eq!(no_session_end.status(), StatusCode::UNAUTHORIZED);
    assert_no_store(&no_session_end);

    let no_session_get = get_retention(&app, None, course).await;
    assert_eq!(no_session_get.status(), StatusCode::UNAUTHORIZED);
    assert_no_store(&no_session_get);
}

#[tokio::test]
async fn retention_route_authority_hides_courses_before_payload_inspection() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(10));
    let foreign_tenant = TenantId::from_uuid(id(20));
    let course = CourseId::from_uuid(id(11));
    let missing_course = CourseId::from_uuid(id(12));
    let instructor = UserId::from_uuid(id(13));
    let student = UserId::from_uuid(id(14));
    let outsider = UserId::from_uuid(id(15));
    let foreign_instructor = UserId::from_uuid(id(16));

    create_course(
        &store,
        tenant,
        course,
        vec![
            (instructor, CourseMembershipRole::Instructor),
            (student, CourseMembershipRole::Student),
        ],
    )
    .await;

    let instructor_cookie =
        issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let student_cookie = issued_cookie(&store, tenant, vec![UserRole::Student], student).await;
    let outsider_cookie = issued_cookie(&store, tenant, vec![UserRole::Instructor], outsider).await;
    let foreign_cookie = issued_cookie(
        &store,
        foreign_tenant,
        vec![UserRole::Instructor],
        foreign_instructor,
    )
    .await;

    let app = router(Arc::clone(&store));
    let ended = end_retention(&app, Some(&instructor_cookie), course, "").await;
    assert_eq!(ended.status(), StatusCode::OK);

    let student_archive = archive_retention(
        &app,
        Some(&student_cookie),
        course,
        &[],
        Some("text/plain"),
        r#"{"assignmentDefinitions":"retain"}"#,
    )
    .await;
    assert_eq!(student_archive.status(), StatusCode::NOT_FOUND);
    assert_no_store(&student_archive);

    let outsider_delete = delete_retention(&app, Some(&outsider_cookie), course, &[], "").await;
    assert_eq!(outsider_delete.status(), StatusCode::NOT_FOUND);
    assert_no_store(&outsider_delete);

    let foreign_extend = extend_retention(
        &app,
        Some(&foreign_cookie),
        course,
        &[],
        None,
        r#"{"additionalDays":7}"#,
    )
    .await;
    assert_eq!(foreign_extend.status(), StatusCode::NOT_FOUND);
    assert_no_store(&foreign_extend);

    let missing_course_archive = archive_retention(
        &app,
        Some(&instructor_cookie),
        missing_course,
        &[],
        Some("text/plain"),
        r#"{"assignmentDefinitions":"retain"}"#,
    )
    .await;
    assert_eq!(missing_course_archive.status(), StatusCode::NOT_FOUND);
    assert_no_store(&missing_course_archive);
}

#[tokio::test]
async fn retention_end_route_is_replayable_and_requires_exact_empty_body() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(21));
    let course = CourseId::from_uuid(id(22));
    let instructor = UserId::from_uuid(id(23));
    create_course(
        &store,
        tenant,
        course,
        vec![(instructor, CourseMembershipRole::Instructor)],
    )
    .await;
    let instructor_cookie =
        issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let app = router(Arc::clone(&store));

    let non_empty = end_retention(&app, Some(&instructor_cookie), course, "{}").await;
    assert_eq!(non_empty.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_no_store(&non_empty);

    let first = end_retention(&app, Some(&instructor_cookie), course, "").await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_payload = response_json(first).await;
    let revision = first_payload["revision"].as_u64().expect("revision");

    let replay = end_retention(&app, Some(&instructor_cookie), course, "").await;
    assert_eq!(replay.status(), StatusCode::OK);
    let replay_payload = response_json(replay).await;
    assert_eq!(
        replay_payload["revision"].as_u64().expect("revision"),
        revision
    );
    assert_private_projection_fields(&replay_payload);
}

#[tokio::test]
async fn retention_get_route_hides_private_fields_and_emits_etag_and_notification() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(31));
    let course = CourseId::from_uuid(id(32));
    let instructor = UserId::from_uuid(id(33));
    create_course(
        &store,
        tenant,
        course,
        vec![(instructor, CourseMembershipRole::Instructor)],
    )
    .await;
    let instructor_cookie =
        issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;

    let app = router(Arc::clone(&store));
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("clock set");
    let ended = end_retention(&app, Some(&instructor_cookie), course, "").await;
    assert_eq!(ended.status(), StatusCode::OK);

    let notification_due = ActivityTimestamp::from_unix_millis(30 * 86_400_000 + 2_000);
    store
        .set_authoritative_time(notification_due)
        .expect("clock set");
    let dispatched = store
        .dispatch_due_retention_stages(RetentionDispatchBatch::new(4).expect("dispatch batch"))
        .await
        .expect("due dispatch");
    assert_eq!(dispatched, 1);
    let claim = store
        .claim_next_job(
            &learning_data_access::JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease"),
        )
        .await
        .expect("claimed job")
        .expect("job claim");
    let command = worker_command_from_claim(claim);
    store
        .prepare_retention_work(command)
        .await
        .expect("prepare notify job");
    store
        .commit_retention_work(command)
        .await
        .expect("commit notify job");

    let instructor_cookie =
        issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;

    let viewed = get_retention(&app, Some(&instructor_cookie), course).await;
    assert_eq!(viewed.status(), StatusCode::OK);
    let etag = viewed
        .headers()
        .get(ETAG)
        .expect("etag")
        .to_str()
        .expect("etag")
        .to_string();
    let viewed = response_json(viewed).await;
    assert_eq!(viewed["state"], serde_json::json!("active"));
    assert_eq!(viewed["assignmentDefinitions"], serde_json::json!("retain"));
    if let Some(notification) = viewed.get("notification") {
        assert_eq!(
            notification["copy"],
            serde_json::json!(RETENTION_ARCHIVE_NOTIFICATION_COPY)
        );
        assert_eq!(notification["intent"], serde_json::json!("archive"));
        assert!(notification["createdAt"].is_number());
    }
    assert_private_projection_fields(&viewed);
    let revision = viewed["revision"].as_u64().expect("revision");
    assert_eq!(etag, format!("\"{}\"", revision));
}

#[tokio::test]
async fn retention_archive_route_validates_if_match_and_body_grammar() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(41));
    let course = CourseId::from_uuid(id(42));
    let instructor = UserId::from_uuid(id(43));
    create_course(
        &store,
        tenant,
        course,
        vec![(instructor, CourseMembershipRole::Instructor)],
    )
    .await;
    let instructor_cookie =
        issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let app = router(Arc::clone(&store));
    let ended = end_retention(&app, Some(&instructor_cookie), course, "").await;
    assert_eq!(ended.status(), StatusCode::OK);

    let missing_if_match = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &[],
        Some("text/plain"),
        r#"{"assignmentDefinitions":"retain"}"#, // should still be 428
    )
    .await;
    assert_eq!(missing_if_match.status(), StatusCode::PRECONDITION_REQUIRED);

    for header in ["W/\"1\"", "0", "bad", "\"9223372036854775808\""] {
        let malformed = archive_retention(
            &app,
            Some(&instructor_cookie),
            course,
            &[header],
            Some("application/json"),
            r#"{"assignmentDefinitions":"retain"}"#, // malformed header only
        )
        .await;
        assert_eq!(
            malformed.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            header
        );
        assert_no_store(&malformed);
    }

    let multiple = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &["\"1\"", "\"1\""],
        Some("application/json"),
        r#"{"assignmentDefinitions":"retain"}"#,
    )
    .await;
    assert_eq!(multiple.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let non_json = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &["\"1\""],
        Some("text/plain"),
        r#"{"assignmentDefinitions":"retain"}"#,
    )
    .await;
    assert_eq!(non_json.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let unknown = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &["\"1\""],
        Some("application/json"),
        r#"{"assignmentDefinitions":"retain","extra":"oops"}"#,
    )
    .await;
    assert_eq!(unknown.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let duplicate = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &["\"1\""],
        Some("application/json"),
        r#"{\"assignmentDefinitions\":\"retain\",\"assignmentDefinitions\":\"delete\"}"#,
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let enum_value = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &["\"1\""],
        Some("application/json"),
        r#"{"assignmentDefinitions":"invalid"}"#,
    )
    .await;
    assert_eq!(enum_value.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let oversized = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &["\"1\""],
        Some("application/json"),
        &format!(
            "{{\"assignmentDefinitions\":\"retain\",\"padding\":\"{}\"}}",
            "a".repeat(MAX_RETENTION_BODY_BYTES + 10),
        ),
    )
    .await;
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let valid = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &["\"1\""],
        Some("application/json"),
        r#"{"assignmentDefinitions":"delete"}"#,
    )
    .await;
    assert_eq!(valid.status(), StatusCode::ACCEPTED);
    let valid_json = response_json(valid).await;
    assert_eq!(valid_json["outcome"], serde_json::json!("scheduled"));
    assert_private_projection_fields(&valid_json);
}

#[tokio::test]
async fn retention_archive_route_replays_scheduled_with_no_duplicate_jobs_and_complete_via_worker()
{
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(51));
    let course = CourseId::from_uuid(id(52));
    let instructor = UserId::from_uuid(id(53));
    let sysadmin = UserId::from_uuid(id(54));
    create_course(
        &store,
        tenant,
        course,
        vec![(instructor, CourseMembershipRole::Instructor)],
    )
    .await;
    let instructor_cookie =
        issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let sysadmin_cookie = issued_cookie(&store, tenant, vec![UserRole::Sysadmin], sysadmin).await;
    let app = router(Arc::clone(&store));
    let ended = end_retention(&app, Some(&instructor_cookie), course, "").await;
    assert_eq!(ended.status(), StatusCode::OK);

    let first = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &["\"1\""],
        Some("application/json"),
        r#"{"assignmentDefinitions":"delete"}"#,
    )
    .await;
    assert_eq!(first.status(), StatusCode::ACCEPTED);
    let first = response_json(first).await;
    assert_eq!(first["outcome"], serde_json::json!("scheduled"));
    let revision = first["revision"].as_u64().expect("revision");

    let stale_replay = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &["\"1\""],
        Some("application/json"),
        r#"{"assignmentDefinitions":"delete"}"#,
    )
    .await;
    assert_eq!(stale_replay.status(), StatusCode::ACCEPTED);
    assert_eq!(
        response_json(stale_replay).await["revision"],
        serde_json::json!(revision)
    );

    let first_job = store
        .claim_next_job(
            &learning_data_access::JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).unwrap(),
        )
        .await
        .expect("next job")
        .expect("archive job");
    assert!(
        store
            .claim_next_job(
                &learning_data_access::JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(30).unwrap()
            )
            .await
            .expect("next job")
            .is_none(),
        "no duplicate job on replay"
    );

    let command = worker_command_from_claim(first_job);
    store
        .prepare_retention_work(command)
        .await
        .expect("prepare archive");
    let current_header = format!("\"{}\"", revision);
    let in_progress = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &[&current_header],
        Some("application/json"),
        r#"{"assignmentDefinitions":"delete"}"#,
    )
    .await;
    assert_eq!(in_progress.status(), StatusCode::ACCEPTED);
    assert_eq!(
        response_json(in_progress).await["outcome"],
        serde_json::json!("inProgress")
    );

    store
        .commit_retention_work(command)
        .await
        .expect("commit archive");
    let completed = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &[&current_header],
        Some("application/json"),
        r#"{"assignmentDefinitions":"delete"}"#,
    )
    .await;
    assert_eq!(completed.status(), StatusCode::OK);
    assert_eq!(
        response_json(completed).await["outcome"],
        serde_json::json!("completed")
    );

    let original_completed_replay = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &["\"1\""],
        Some("application/json"),
        r#"{"assignmentDefinitions":"delete"}"#,
    )
    .await;
    assert_eq!(original_completed_replay.status(), StatusCode::OK);
    assert_eq!(
        response_json(original_completed_replay).await["outcome"],
        serde_json::json!("completed")
    );

    let mismatched_actor = archive_retention(
        &app,
        Some(&sysadmin_cookie),
        course,
        &[&current_header],
        Some("application/json"),
        r#"{"assignmentDefinitions":"delete"}"#,
    )
    .await;
    assert_eq!(mismatched_actor.status(), StatusCode::CONFLICT);

    let mismatched_disposition = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &[&current_header],
        Some("application/json"),
        r#"{"assignmentDefinitions":"retain"}"#,
    )
    .await;
    assert_eq!(mismatched_disposition.status(), StatusCode::CONFLICT);

    let mismatched_action = delete_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &[&current_header],
        "",
    )
    .await;
    assert_eq!(mismatched_action.status(), StatusCode::CONFLICT);

    let stale = archive_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &["\"999\""],
        Some("application/json"),
        r#"{"assignmentDefinitions":"delete"}"#,
    )
    .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn retention_delete_route_requires_exact_empty_body_and_stale_if_match() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(61));
    let course = CourseId::from_uuid(id(62));
    let instructor = UserId::from_uuid(id(63));
    create_course(
        &store,
        tenant,
        course,
        vec![(instructor, CourseMembershipRole::Instructor)],
    )
    .await;
    let instructor_cookie =
        issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let app = router(Arc::clone(&store));
    let ended = end_retention(&app, Some(&instructor_cookie), course, "").await;
    assert_eq!(ended.status(), StatusCode::OK);

    let missing_if_match = delete_retention(&app, Some(&instructor_cookie), course, &[], "").await;
    assert_eq!(missing_if_match.status(), StatusCode::PRECONDITION_REQUIRED);

    let non_empty = delete_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &["\"1\""],
        r#"{"junk":true}"#,
    )
    .await;
    assert_eq!(non_empty.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let first = delete_retention(&app, Some(&instructor_cookie), course, &["\"1\""], "").await;
    assert_eq!(first.status(), StatusCode::ACCEPTED);

    let stale = delete_retention(&app, Some(&instructor_cookie), course, &["\"999\""], "").await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn retention_extend_route_is_sysadmin_only_and_rejects_stale_requests() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(71));
    let course = CourseId::from_uuid(id(72));
    let instructor = UserId::from_uuid(id(73));
    let sysadmin = UserId::from_uuid(id(74));
    create_course(
        &store,
        tenant,
        course,
        vec![(instructor, CourseMembershipRole::Instructor)],
    )
    .await;
    let instructor_cookie =
        issued_cookie(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let sysadmin_cookie = issued_cookie(&store, tenant, vec![UserRole::Sysadmin], sysadmin).await;
    let app = router(Arc::clone(&store));
    let ended = end_retention(&app, Some(&instructor_cookie), course, "").await;
    assert_eq!(ended.status(), StatusCode::OK);

    let instructor_forbidden = extend_retention(
        &app,
        Some(&instructor_cookie),
        course,
        &[],
        Some("text/plain"),
        r#"{"additionalDays":3}"#,
    )
    .await;
    assert_eq!(instructor_forbidden.status(), StatusCode::FORBIDDEN);

    let sysadmin_requires_if_match = extend_retention(
        &app,
        Some(&sysadmin_cookie),
        course,
        &[],
        Some("text/plain"),
        r#"{"additionalDays":3}"#,
    )
    .await;
    assert_eq!(
        sysadmin_requires_if_match.status(),
        StatusCode::PRECONDITION_REQUIRED
    );

    let sysadmin_success = extend_retention(
        &app,
        Some(&sysadmin_cookie),
        course,
        &["\"1\""],
        Some("application/json"),
        r#"{"additionalDays":3}"#,
    )
    .await;
    assert_eq!(sysadmin_success.status(), StatusCode::OK);
    let sysadmin_success = response_json(sysadmin_success).await;
    assert_eq!(sysadmin_success["state"], serde_json::json!("active"));
    assert_private_projection_fields(&sysadmin_success);

    let stale = extend_retention(
        &app,
        Some(&sysadmin_cookie),
        course,
        &["\"1\""],
        Some("application/json"),
        r#"{"additionalDays":3}"#,
    )
    .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
}
