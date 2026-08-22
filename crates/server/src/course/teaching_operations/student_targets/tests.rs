use super::*;

use axum::body::{Body, to_bytes};
use axum::http::header::CACHE_CONTROL;
use axum::http::{Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AuthenticationEmail, CourseMemberId, CourseRecord, CourseRosterContact, CourseRosterId,
    CourseRosterStore, CreateCourseCommand, PageRequest, PageSize, RevokeCourseMember, Store,
    TenantContext, UpsertCourseMember,
};
use question_model::{CourseId, TenantId, UserId, UserRole};
use tower::ServiceExt;
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

struct Fixture {
    app: Router,
    store: Arc<MemoryStore>,
    context: TenantContext,
    course: CourseId,
    instructor: String,
    instructor_session: learning_data_access::SessionTokenHash,
    student: String,
    outsider: String,
}

async fn session(
    store: &MemoryStore,
    tenant: TenantId,
    user: UserId,
    role: UserRole,
) -> (String, learning_data_access::SessionTokenHash) {
    let subject =
        learning_data_access::SessionSubject::new(tenant, user, "Picker test", vec![role])
            .expect("fixture session");
    let issued = crate::auth::issue_session(
        store,
        subject,
        crate::auth::SessionConfig::new(
            learning_data_access::SessionLifetime::from_seconds(3_600).expect("lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("issue session");
    (
        issued
            .set_cookie
            .split(';')
            .next()
            .expect("cookie")
            .to_owned(),
        issued.record.token_hash,
    )
}

async fn fixture() -> Fixture {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(80));
    let context = TenantContext::from_authenticated_session(tenant);
    let course = CourseId::from_uuid(id(81));
    let instructor_user = UserId::from_uuid(id(82));
    let student_user = UserId::from_uuid(id(83));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "BIOC 301".to_owned(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("term"),
                },
                initial_instructor: instructor_user,
            },
        )
        .await
        .expect("course");
    for (offset, display) in [
        (83, "Student One"),
        (84, "Student Two"),
        (85, "Student Three"),
    ] {
        store
            .upsert_course_member(
                context,
                UpsertCourseMember {
                    course,
                    user: UserId::from_uuid(id(offset)),
                    display_name: display.to_owned(),
                    roster_contact: Some(CourseRosterContact {
                        email: AuthenticationEmail::parse(&format!("private-{offset}@example.edu"))
                            .expect("fixture email"),
                        roster_id: CourseRosterId::parse(&format!("9000000{offset}"))
                            .expect("fixture roster ID"),
                    }),
                },
            )
            .await
            .expect("student membership");
    }
    let (instructor, instructor_session) =
        session(&store, tenant, instructor_user, UserRole::Instructor).await;
    let (student, _) = session(&store, tenant, student_user, UserRole::Student).await;
    let (outsider, _) = session(&store, tenant, UserId::from_uuid(id(86)), UserRole::Student).await;
    Fixture {
        app: crate::course::router(Arc::clone(&store)),
        store,
        context,
        course,
        instructor,
        instructor_session,
        student,
        outsider,
    }
}

async fn response(app: &Router, request: Request<Body>) -> axum::response::Response {
    app.clone().oneshot(request).await.expect("response")
}

async fn json(response: axum::response::Response) -> serde_json::Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("bounded response"),
    )
    .expect("JSON")
}

#[tokio::test]
async fn student_targets_authorize_before_parsing_a_malformed_page() {
    let fixture = fixture().await;
    for cookie in [&fixture.student, &fixture.outsider] {
        let response = response(
            &fixture.app,
            Request::get(format!(
                "/api/courses/{}/student-targets?size=zero&extra=private",
                fixture.course
            ))
            .header("cookie", cookie)
            .body(Body::empty())
            .expect("request"),
        )
        .await;
        assert!(matches!(
            response.status(),
            StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
        ));
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
    }
}

#[tokio::test]
async fn student_targets_return_only_active_students_in_stable_safe_pages() {
    let fixture = fixture().await;
    let revoked = fixture
        .store
        .get_current_course_membership(fixture.context, fixture.course, UserId::from_uuid(id(85)))
        .await
        .expect("membership lookup")
        .expect("membership exists");
    let roster_revision = fixture
        .store
        .list_course_roster(
            fixture.context,
            fixture.instructor_session,
            fixture.course,
            PageRequest::first(PageSize::new(1).expect("bounded page")),
        )
        .await
        .expect("roster revision")
        .policy
        .revision;
    fixture
        .store
        .revoke_course_member(
            fixture.context,
            fixture.instructor_session,
            RevokeCourseMember {
                course: fixture.course,
                member: CourseMemberId::from_uuid(revoked.id.as_uuid()),
                expected_revision: roster_revision,
            },
        )
        .await
        .expect("revoke student");
    let first = response(
        &fixture.app,
        Request::get(format!(
            "/api/courses/{}/student-targets?size=1",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .body(Body::empty())
        .expect("first page"),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(first.headers()[CACHE_CONTROL], "no-store");
    let first = json(first).await;
    let cursor = first["nextCursor"].as_str().expect("next cursor");
    let second = response(
        &fixture.app,
        Request::get(format!(
            "/api/courses/{}/student-targets?size=1&after={cursor}",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .body(Body::empty())
        .expect("second page"),
    )
    .await;
    assert_eq!(second.status(), StatusCode::OK);
    let second = json(second).await;
    assert_ne!(
        first["students"][0]["reference"],
        second["students"][0]["reference"]
    );
    let beyond = response(
        &fixture.app,
        Request::get(format!(
            "/api/courses/{}/student-targets?size=1&after=9999999999",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .body(Body::empty())
        .expect("beyond page"),
    )
    .await;
    assert_eq!(
        json(beyond).await,
        serde_json::json!({"students": [], "nextCursor": null})
    );
    let combined = format!("{first}{second}");
    assert!(combined.contains("Student One") && combined.contains("Student Two"));
    assert!(!combined.contains("Student Three") && !combined.contains("Instructor"));
    for private in ["email", "uuid", "rosterId", "private-83@example.edu"] {
        assert!(!combined.contains(private), "response leaked {private}");
    }
}
