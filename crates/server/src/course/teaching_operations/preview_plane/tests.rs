use super::*;

use axum::body::{Body, to_bytes};
use axum::http::header::{CACHE_CONTROL, ETAG};
use axum::http::{Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AssignmentRecord, CourseRecord, CourseRosterStore, CreateCourseCommand,
    NavigationReferenceStore, SessionLifetime, SessionSubject, Store,
    TeachingAuthorityReferenceStore, TenantContext, UpsertCourseMember,
};
use question_model::{
    AssignmentAudience, AssignmentId, AssignmentItem, AssignmentItemId, AssignmentLifecycle,
    AssignmentScoringMode, AssignmentTeachingSettings, CourseId, CourseTerm, PointValue, TenantId,
    UserId, UserRole,
};
use tower::ServiceExt;
use uuid::Uuid;

use crate::course::tests::fixtures::{policies, publish_assignment, publish_fixture};

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

struct Fixture {
    store: Arc<MemoryStore>,
    app: axum::Router,
    instructor: UserId,
    instructor_cookie: String,
    student_cookie: String,
    outsider_cookie: String,
    course_ref: question_model::CourseReference,
    assignment_ref: AssignmentReference,
    member_ref: CourseMembershipReference,
    revision: TeachingOperationRevision,
}

async fn fixture() -> Fixture {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(question_model::ActivityTimestamp::from_unix_millis(
            1_788_000_000_000,
        ))
        .expect("clock");
    let tenant = TenantId::from_uuid(id(910));
    let course = CourseId::from_uuid(id(911));
    let assignment = AssignmentId::from_uuid(id(912));
    let instructor = UserId::from_uuid(id(913));
    let student = UserId::from_uuid(id(914));
    let outsider = UserId::from_uuid(id(915));
    let context = TenantContext::from_authenticated_session(tenant);
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "BIOC 301".to_owned(),
                    term: CourseTerm::from_parts("2026-01-01", "2026-12-31", "America/Chicago")
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
    for (user, name) in [(student, "Mary Student")] {
        store
            .upsert_course_member(
                context,
                instructor,
                UpsertCourseMember {
                    course,
                    user,
                    display_name: name.to_owned(),
                    roster_contact: None,
                },
            )
            .await
            .expect("student");
    }
    let problem = publish_fixture(store.as_ref(), context, tenant, instructor).await;
    let base_policy = question_model::BaseAssignmentPolicy {
        due_at: Some(question_model::ActivityTimestamp::from_unix_millis(
            1_790_000_000_000,
        )),
        ..question_model::BaseAssignmentPolicy::default()
    };
    store
        .create_assignment(
            context,
            learning_data_access::CreateAssignmentCommand {
                actor: instructor,
                assignment: AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    audience: AssignmentAudience::CourseWide,
                    title: "Peptide bonds".to_owned(),
                    lifecycle: AssignmentLifecycle::Draft,
                    instructions: question_model::AssignmentInstructions::default(),
                    items: vec![AssignmentItem {
                        id: AssignmentItemId::from_uuid(id(916)),
                        reference: problem,
                        position: 0,
                        points_possible: PointValue::from_whole(1),
                        delivery_state: question_model::AssignmentDeliveryState::Active,
                        scoring_mode: AssignmentScoringMode::Normal,
                    }],
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: policies(),
                },
                base_policy,
            },
        )
        .await
        .expect("assignment");
    publish_assignment(
        store.as_ref(),
        context,
        instructor,
        course,
        assignment,
        AssignmentTeachingSettings {
            lifecycle: AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::default(),
            base_policy,
        },
    )
    .await;
    let member = store
        .get_current_course_membership(context, course, student)
        .await
        .expect("membership")
        .expect("student membership");
    let course_ref = store
        .course_reference(context, instructor, course)
        .await
        .expect("course ref")
        .expect("course ref");
    let assignment_ref = store
        .assignment_reference(context, instructor, assignment)
        .await
        .expect("assignment ref")
        .expect("assignment ref");
    let member_ref = store
        .course_membership_reference(context, instructor, course, member.id)
        .await
        .expect("membership ref")
        .expect("membership ref");
    let instructor_cookie = cookie(store.as_ref(), tenant, instructor, UserRole::Instructor).await;
    let student_cookie = cookie(store.as_ref(), tenant, student, UserRole::Student).await;
    let outsider_cookie = cookie(store.as_ref(), tenant, outsider, UserRole::Student).await;
    let revision = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("assignment")
        .expect("assignment")
        .revision;
    Fixture {
        app: crate::course::router(Arc::clone(&store)),
        store,
        instructor,
        instructor_cookie,
        student_cookie,
        outsider_cookie,
        course_ref,
        assignment_ref,
        member_ref,
        revision: TeachingOperationRevision::new(revision.value()).expect("revision"),
    }
}

async fn cookie(store: &MemoryStore, tenant: TenantId, user: UserId, role: UserRole) -> String {
    let subject = SessionSubject::new(tenant, user, "Preview test", vec![role]).expect("session");
    crate::auth::issue_session(
        store,
        subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("session")
    .set_cookie
    .split(';')
    .next()
    .expect("cookie")
    .to_owned()
}

fn uri(fixture: &Fixture, suffix: &str) -> String {
    format!(
        "/api/courses/{}/assignments/{}/{}",
        fixture.course_ref, fixture.assignment_ref, suffix
    )
}

fn request(
    method: &str,
    uri: String,
    cookie: &str,
    revision: TeachingOperationRevision,
    body: Body,
) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("cookie", cookie)
        .header("if-match", format!("\"{revision}\""))
        .body(body)
        .expect("request");
    if request.method() == "POST" {
        request
            .headers_mut()
            .insert("content-type", "application/json".parse().expect("JSON"));
    }
    request
}

async fn json(response: axum::response::Response) -> serde_json::Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("body"),
    )
    .expect("JSON")
}

fn subject_body(membership: Option<CourseMembershipReference>) -> Body {
    let selected = serde_json::json!({
        "value":"2026-08-25T09:00:00.000", "timeZone":"America/Chicago"
    });
    Body::from(match membership {
        Some(membership) => serde_json::json!({"selectedMoment":selected,"membership":membership}).to_string(),
        None => serde_json::json!({
            "selectedMoment":selected,"groups":[],"modifiers":{"mode":"extendOnly","patch":{
                "availableAt":{"kind":"inherit"},"dueAt":{"kind":"inherit"},"closesAt":{"kind":"inherit"},
                "timeLimitSeconds":{"kind":"inherit"},"attemptLimit":{"kind":"inherit"}}}
        }).to_string(),
    })
}

#[tokio::test]
async fn memory_preview_http_schedule_synthetic_and_derived_are_closed_and_uncached() {
    let fixture = fixture().await;
    let schedule = fixture
        .app
        .clone()
        .oneshot(request(
            "GET",
            uri(&fixture, "preview-schedule?size=1"),
            &fixture.instructor_cookie,
            fixture.revision,
            Body::empty(),
        ))
        .await
        .expect("schedule");
    assert_eq!(schedule.status(), StatusCode::OK);
    assert_eq!(schedule.headers()[CACHE_CONTROL], "no-store");
    assert!(schedule.headers().get(ETAG).is_none());
    let page = json(schedule).await;
    assert_eq!(page["rows"].as_array().expect("rows").len(), 1);
    assert!(page.to_string().contains("M-"));
    for (suffix, body) in [
        ("preview-subjects/synthetic", subject_body(None)),
        (
            "preview-subjects/derived",
            subject_body(Some(fixture.member_ref)),
        ),
    ] {
        let response = fixture
            .app
            .clone()
            .oneshot(request(
                "POST",
                uri(&fixture, suffix),
                &fixture.instructor_cookie,
                fixture.revision,
                body,
            ))
            .await
            .expect("preview");
        assert_eq!(response.status(), StatusCode::OK, "{suffix}");
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        let value = json(response).await;
        assert_eq!(value["evaluation"]["kind"], "allowed");
        let wire = value.to_string();
        assert!(!wire.contains("Mary Student"));
        assert!(!wire.contains("score") || wire.contains("scoreShown"));
        assert!(!wire.contains("answer"));
        assert!(!wire.contains("audit"));
    }
    let audits = fixture.store.preview_subject_audits().expect("audit seam");
    assert_eq!(
        audits.len(),
        1,
        "only successful derivation records an audit"
    );
    assert_eq!(audits[0].actor, fixture.instructor);
}

#[tokio::test]
async fn memory_preview_http_refuses_stale_malformed_and_unauthorized_before_subject_decode() {
    let fixture = fixture().await;
    let before = fixture.store.preview_subject_audits().expect("audits");
    let cases = [
        (
            &fixture.student_cookie,
            fixture.revision,
            Body::from("not JSON"),
        ),
        (
            &fixture.outsider_cookie,
            fixture.revision,
            Body::from("not JSON"),
        ),
        (
            &fixture.instructor_cookie,
            TeachingOperationRevision::new(fixture.revision.value() + 1).expect("stale"),
            subject_body(Some(fixture.member_ref)),
        ),
    ];
    for (index, (cookie, revision, body)) in cases.into_iter().enumerate() {
        let response = fixture
            .app
            .clone()
            .oneshot(request(
                "POST",
                uri(&fixture, "preview-subjects/derived"),
                cookie,
                revision,
                body,
            ))
            .await
            .expect("denial");
        assert_eq!(
            response.status(),
            if index < 2 {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::PRECONDITION_FAILED
            }
        );
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        assert!(response.headers().get(ETAG).is_none());
    }
    assert_eq!(
        fixture.store.preview_subject_audits().expect("audits"),
        before
    );
}

#[tokio::test]
async fn memory_preview_http_rejects_foreign_and_bad_navigation_without_subject_metadata() {
    let fixture = fixture().await;
    for path in [
        format!(
            "/api/courses/C-999999/assignments/{}/preview-schedule",
            fixture.assignment_ref
        ),
        format!(
            "/api/courses/{}/assignments/A-999999/preview-schedule",
            fixture.course_ref
        ),
        uri(&fixture, "preview-schedule?size=0"),
    ] {
        let response = fixture
            .app
            .clone()
            .oneshot(request(
                "GET",
                path,
                &fixture.instructor_cookie,
                fixture.revision,
                Body::empty(),
            ))
            .await
            .expect("response");
        assert_ne!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        let body = json(response).await.to_string();
        assert!(!body.contains("\"rows\""));
        assert!(!body.contains("\"evaluation\""));
        assert!(!body.contains("\"disclosure\""));
    }
}
