use super::*;

use axum::body::{Body, to_bytes};
use axum::http::header::{CACHE_CONTROL, ETAG, LOCATION};
use axum::http::{Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AssignmentRecord, CourseGroupManagementStore, CourseRecord, CourseRosterStore,
    CreateCourseCommand, Store, TeachingAuthorityReferenceStore, TenantContext, UpsertCourseMember,
};
use question_model::{
    AssignmentAudience, AssignmentDeliveryState, AssignmentId, AssignmentInstructions,
    AssignmentItem, AssignmentItemId, AssignmentLifecycle, AssignmentScoringMode,
    AssignmentTeachingSettings, BaseAssignmentPolicy, CourseId, LearnerDisclosurePolicy,
    PointValue, TenantId, UserId, UserRole,
};
use tower::ServiceExt;
use uuid::Uuid;

use crate::course::tests::fixtures::{policies, publish_assignment, publish_fixture};

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

async fn cookie(store: &MemoryStore, tenant: TenantId, role: UserRole, user: UserId) -> String {
    let subject = learning_data_access::SessionSubject::new(tenant, user, "Group test", vec![role])
        .expect("fixture session");
    crate::auth::issue_session(
        store,
        subject,
        crate::auth::SessionConfig::new(
            learning_data_access::SessionLifetime::from_seconds(3_600).expect("lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("issue session")
    .set_cookie
    .split(';')
    .next()
    .expect("cookie")
    .to_string()
}

async fn fixture() -> (Arc<MemoryStore>, String, String, String, CourseId, String) {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(30));
    let course = CourseId::from_uuid(id(31));
    let instructor = UserId::from_uuid(id(32));
    let student = UserId::from_uuid(id(33));
    let outsider = UserId::from_uuid(id(34));
    let context = TenantContext::from_authenticated_session(tenant);
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
                    .expect("term"),
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
        .expect("course");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Student One".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("student membership");
    let membership = store
        .get_current_course_membership(context, course, student)
        .await
        .expect("student membership")
        .expect("student membership exists");
    let member = store
        .course_membership_reference(context, instructor, course, membership.id)
        .await
        .expect("student reference")
        .expect("student reference exists")
        .to_string();
    let instructor_cookie = cookie(&store, tenant, UserRole::Instructor, instructor).await;
    let student_cookie = cookie(&store, tenant, UserRole::Student, student).await;
    let outsider_cookie = cookie(&store, tenant, UserRole::Student, outsider).await;
    (
        store,
        instructor_cookie,
        student_cookie,
        outsider_cookie,
        course,
        member,
    )
}

async fn json(response: axum::response::Response) -> serde_json::Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), 128 * 1024)
            .await
            .expect("bounded response"),
    )
    .expect("JSON response")
}

fn create(course: CourseId, cookie: &str, title: &str, member: &str) -> Request<Body> {
    create_with_purpose(course, cookie, title, member, "section")
}

fn create_with_purpose(
    course: CourseId,
    cookie: &str,
    title: &str,
    member: &str,
    purpose: &str,
) -> Request<Body> {
    Request::post(format!("/api/courses/{course}/groups"))
        .header("cookie", cookie)
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"title": title, "purpose": purpose, "members": [member]})
                .to_string(),
        ))
        .expect("group request")
}

async fn create_group(
    app: &Router,
    course: CourseId,
    cookie: &str,
    title: &str,
    member: &str,
) -> (String, String) {
    let response = app
        .clone()
        .oneshot(create(course, cookie, title, member))
        .await
        .expect("create response");
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
    let etag = response.headers()[ETAG].to_str().expect("etag").to_string();
    let location = response.headers()[LOCATION]
        .to_str()
        .expect("location")
        .to_string();
    let body = json(response).await;
    let reference = body["reference"]
        .as_str()
        .expect("group reference")
        .to_string();
    assert_eq!(
        location,
        format!("/api/courses/{course}/groups/{reference}")
    );
    (reference, etag)
}

async fn create_group_with_purpose(
    app: &Router,
    course: CourseId,
    cookie: &str,
    title: &str,
    member: &str,
    purpose: &str,
) -> (String, String) {
    let response = app
        .clone()
        .oneshot(create_with_purpose(course, cookie, title, member, purpose))
        .await
        .expect("create response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let etag = response.headers()[ETAG].to_str().expect("etag").to_string();
    let body = json(response).await;
    (
        body["reference"]
            .as_str()
            .expect("group reference")
            .to_string(),
        etag,
    )
}

async fn assignment_referencing_group(
    store: &MemoryStore,
    context: TenantContext,
    tenant: TenantId,
    instructor: UserId,
    course: CourseId,
    group: &str,
) -> AssignmentId {
    let assignment = AssignmentId::from_uuid(id(35));
    let group = store
        .get_course_group_by_reference(context, instructor, course, group.parse().expect("group"))
        .await
        .expect("group read")
        .expect("group exists")
        .group
        .record
        .id;
    let reference = publish_fixture(store, context, tenant, instructor).await;
    let base_policy = BaseAssignmentPolicy::default();
    store
        .create_assignment(
            context,
            learning_data_access::CreateAssignmentCommand {
                actor: instructor,
                assignment: AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    audience: AssignmentAudience::any_of_groups(vec![group]).expect("one group"),
                    title: "Group guard fixture".to_string(),
                    lifecycle: AssignmentLifecycle::Draft,
                    instructions: AssignmentInstructions::default(),
                    items: vec![AssignmentItem {
                        id: AssignmentItemId::from_uuid(id(36)),
                        reference,
                        position: 0,
                        points_possible: PointValue::from_whole(1),
                        delivery_state: AssignmentDeliveryState::Active,
                        scoring_mode: AssignmentScoringMode::Normal,
                    }],
                    selection_groups: Vec::new(),
                    disclosure_policy: LearnerDisclosurePolicy::default(),
                    policies: policies(),
                },
                base_policy,
            },
        )
        .await
        .expect("assignment");
    publish_assignment(
        store,
        context,
        instructor,
        course,
        assignment,
        AssignmentTeachingSettings {
            lifecycle: AssignmentLifecycle::Published,
            instructions: AssignmentInstructions::default(),
            base_policy,
        },
    )
    .await;
    assignment
}

#[tokio::test]
async fn memory_group_list_detail_create_update_and_delete_are_safe_and_paginated() {
    let (store, instructor, _, _, course, member) = fixture().await;
    let app = crate::course::router(Arc::clone(&store));
    let (first, first_etag) = create_group(&app, course, &instructor, "Section A", &member).await;
    let (second, _) = create_group(&app, course, &instructor, "Section B", &member).await;
    let (third, _) = create_group(&app, course, &instructor, "Section C", &member).await;
    let page = app
        .clone()
        .oneshot(
            Request::get(format!("/api/courses/{course}/groups?size=2"))
                .header("cookie", &instructor)
                .body(Body::empty())
                .expect("list request"),
        )
        .await
        .expect("list response");
    assert_eq!(page.status(), StatusCode::OK);
    let page = json(page).await;
    assert_eq!(page["groups"][0]["title"], "Section A");
    assert_eq!(page["groups"][1]["title"], "Section B");
    let cursor = page["nextCursor"].as_str().expect("next cursor");
    let next = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/courses/{course}/groups?size=2&after={cursor}"
            ))
            .header("cookie", &instructor)
            .body(Body::empty())
            .expect("next request"),
        )
        .await
        .expect("next response");
    assert_eq!(json(next).await["groups"][0]["reference"], third);
    let detail = app
        .clone()
        .oneshot(
            Request::get(format!("/api/courses/{course}/groups/{first}"))
                .header("cookie", &instructor)
                .body(Body::empty())
                .expect("detail request"),
        )
        .await
        .expect("detail response");
    assert_eq!(detail.headers()[ETAG], first_etag);
    let detail = json(detail).await;
    assert_eq!(detail["members"][0]["display"], "Student One");
    assert!(detail.get("tenant").is_none());
    assert!(!detail.to_string().contains("00000000-0000"));
    let update_request = Request::put(format!("/api/courses/{course}/groups/{first}"))
        .header("cookie", &instructor)
        .header("if-match", &first_etag)
        .header("content-type", "application/json; charset=utf-8")
        .body(Body::from(
            serde_json::json!({
                "title": "Section A revised",
                "purpose": "section",
                "members": [member],
            })
            .to_string(),
        ))
        .expect("update request");
    let update = app
        .clone()
        .oneshot(update_request)
        .await
        .expect("update response");
    assert_eq!(update.status(), StatusCode::OK);
    let update_etag = update.headers()[ETAG].to_str().expect("etag").to_string();
    assert_eq!(json(update).await["title"], "Section A revised");
    let deleted = app
        .clone()
        .oneshot(
            Request::delete(format!("/api/courses/{course}/groups/{first}"))
                .header("cookie", &instructor)
                .header("if-match", update_etag)
                .body(Body::empty())
                .expect("delete request"),
        )
        .await
        .expect("delete response");
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    assert_eq!(deleted.headers()[CACHE_CONTROL], "no-store");
    assert_ne!(second, third);
}

#[tokio::test]
async fn memory_group_purpose_policy_changes_warning_semantics_without_rejecting_memberships() {
    let (store, instructor, _, _, course, member) = fixture().await;
    let app = crate::course::router(Arc::clone(&store));
    create_group(&app, course, &instructor, "Section A", &member).await;
    create_group(&app, course, &instructor, "Section B", &member).await;
    let warnings = app
        .clone()
        .oneshot(
            Request::get(format!("/api/courses/{course}/group-membership-warnings"))
                .header("cookie", &instructor)
                .body(Body::empty())
                .expect("warning request"),
        )
        .await
        .expect("warning response");
    assert_eq!(
        json(warnings).await,
        serde_json::json!({"disposition":"allowedWithWarning","warningCount":1})
    );
    let policy = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/courses/{course}/group-purpose-policies/section"
            ))
            .header("cookie", &instructor)
            .body(Body::empty())
            .expect("policy request"),
        )
        .await
        .expect("policy response");
    assert_eq!(policy.status(), StatusCode::OK);
    let etag = policy.headers()[ETAG].to_str().expect("etag").to_string();
    assert_eq!(json(policy).await["multipleMembership"], "warn");
    let updated = app
        .clone()
        .oneshot(
            Request::put(format!(
                "/api/courses/{course}/group-purpose-policies/section"
            ))
            .header("cookie", &instructor)
            .header("if-match", etag)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"multipleMembership":"allow"}"#))
            .expect("policy update"),
        )
        .await
        .expect("policy update response");
    assert_eq!(updated.status(), StatusCode::OK);
    assert_eq!(updated.headers()[ETAG], "\"2\"");
    let warnings = app
        .clone()
        .oneshot(
            Request::get(format!("/api/courses/{course}/group-membership-warnings"))
                .header("cookie", &instructor)
                .body(Body::empty())
                .expect("warning request"),
        )
        .await
        .expect("warning response");
    assert_eq!(
        json(warnings).await,
        serde_json::json!({"disposition":"allowed","warningCount":0})
    );
}

#[tokio::test]
async fn memory_group_delete_refuses_http_references_without_partial_state_change() {
    let (store, instructor, _, _, course, member) = fixture().await;
    let app = crate::course::router(Arc::clone(&store));
    let (audience, audience_etag) =
        create_group(&app, course, &instructor, "Audience", &member).await;
    let (schedule, schedule_etag) =
        create_group(&app, course, &instructor, "Schedule", &member).await;
    let (accommodation, accommodation_etag) = create_group_with_purpose(
        &app,
        course,
        &instructor,
        "Accommodation",
        &member,
        "accommodation",
    )
    .await;
    let tenant = TenantId::from_uuid(id(30));
    let context = TenantContext::from_authenticated_session(tenant);
    let assignment = assignment_referencing_group(
        &store,
        context,
        tenant,
        UserId::from_uuid(id(32)),
        course,
        &audience,
    )
    .await;
    let revision = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("assignment")
        .expect("assignment exists")
        .revision
        .value();
    let mut modifier_revision = revision;
    for (path, body) in [
        (
            format!("group-schedule-offsets/{schedule}"),
            r#"{"offsetSeconds":3600}"#,
        ),
        (
            format!("group-accommodations/{accommodation}"),
            r#"{"mode":"extendOnly","patch":{"availableAt":{"kind":"inherit"},"dueAt":{"kind":"unrestricted"},"closesAt":{"kind":"inherit"},"timeLimitSeconds":{"kind":"inherit"},"attemptLimit":{"kind":"inherit"}}}"#,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::put(format!(
                    "/api/courses/{course}/assignments/{assignment}/{path}"
                ))
                .header("cookie", &instructor)
                .header("if-match", format!("\"{modifier_revision}\""))
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("modifier request"),
            )
            .await
            .expect("modifier response");
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        modifier_revision += 1;
        assert_eq!(response.headers()[ETAG], format!("\"{modifier_revision}\""));
    }
    let expected_revision = modifier_revision;
    for (group, etag) in [
        (audience, audience_etag),
        (schedule, schedule_etag),
        (accommodation, accommodation_etag),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::delete(format!("/api/courses/{course}/groups/{group}"))
                    .header("cookie", &instructor)
                    .header("if-match", etag.clone())
                    .body(Body::empty())
                    .expect("delete request"),
            )
            .await
            .expect("delete response");
        assert_eq!(
            response.status(),
            StatusCode::PRECONDITION_FAILED,
            "{group}"
        );
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store", "{group}");
        let preserved = app
            .clone()
            .oneshot(
                Request::get(format!("/api/courses/{course}/groups/{group}"))
                    .header("cookie", &instructor)
                    .body(Body::empty())
                    .expect("group read"),
            )
            .await
            .expect("group response");
        assert_eq!(preserved.status(), StatusCode::OK, "{group}");
        assert_eq!(preserved.headers()[ETAG], etag, "{group}");
    }
    assert_eq!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("assignment")
            .expect("assignment exists")
            .revision
            .value(),
        expected_revision
    );
}

#[tokio::test]
async fn memory_group_mutations_require_well_formed_current_preconditions_before_bodies() {
    let (store, instructor, _, _, course, member) = fixture().await;
    let app = crate::course::router(Arc::clone(&store));
    let (group, etag) = create_group(&app, course, &instructor, "Section A", &member).await;
    for (if_match, expected) in [
        (None, StatusCode::PRECONDITION_REQUIRED),
        (Some("bad"), StatusCode::BAD_REQUEST),
        (Some("\"999\""), StatusCode::PRECONDITION_FAILED),
    ] {
        let mut request = Request::put(format!("/api/courses/{course}/groups/{group}"))
            .header("cookie", &instructor);
        if let Some(value) = if_match {
            request = request.header("if-match", value);
        }
        let response = app
            .clone()
            .oneshot(
                request
                    .body(Body::from("not JSON"))
                    .expect("precondition request"),
            )
            .await
            .expect("precondition response");
        assert_eq!(response.status(), expected);
    }
    let unsupported = app
        .clone()
        .oneshot(
            Request::put(format!("/api/courses/{course}/groups/{group}"))
                .header("cookie", &instructor)
                .header("if-match", &etag)
                .body(Body::from("{}"))
                .expect("content type request"),
        )
        .await
        .expect("content type response");
    assert_eq!(unsupported.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let malformed = app
        .clone()
        .oneshot(
            Request::put(format!("/api/courses/{course}/groups/{group}"))
                .header("cookie", &instructor)
                .header("if-match", &etag)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"x","purpose":"section","members":[],"extra":true}"#,
                ))
                .expect("unknown field request"),
        )
        .await
        .expect("unknown field response");
    assert_eq!(malformed.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let oversized = app
        .clone()
        .oneshot(
            Request::put(format!("/api/courses/{course}/groups/{group}"))
                .header("cookie", &instructor)
                .header("if-match", &etag)
                .header("content-type", "application/json")
                .body(Body::from(vec![b'x'; MAX_GROUP_JSON_BYTES + 1]))
                .expect("oversized request"),
        )
        .await
        .expect("oversized response");
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let unchanged = app
        .clone()
        .oneshot(
            Request::get(format!("/api/courses/{course}/groups/{group}"))
                .header("cookie", &instructor)
                .body(Body::empty())
                .expect("unchanged request"),
        )
        .await
        .expect("unchanged response");
    assert_eq!(unchanged.headers()[ETAG], etag);
    assert_eq!(json(unchanged).await["group"]["title"], "Section A");
}

#[tokio::test]
async fn memory_group_authorizes_students_and_outsiders_before_reading_private_bodies() {
    let (store, instructor, student, outsider, course, member) = fixture().await;
    let app = crate::course::router(Arc::clone(&store));
    let (group, _) = create_group(&app, course, &instructor, "Section A", &member).await;
    for cookie in [&student, &outsider] {
        for request in [
            Request::post(format!("/api/courses/{course}/groups"))
                .header("cookie", cookie)
                .body(Body::from("private not JSON"))
                .expect("create denial"),
            Request::put(format!("/api/courses/{course}/groups/{group}"))
                .header("cookie", cookie)
                .header("if-match", "\"1\"")
                .body(Body::from("private not JSON"))
                .expect("update denial"),
        ] {
            let response = app.clone().oneshot(request).await.expect("denial response");
            assert!(matches!(
                response.status(),
                StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
            ));
            assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
            assert!(
                !json(response)
                    .await
                    .to_string()
                    .contains("private not JSON")
            );
        }
    }
}
