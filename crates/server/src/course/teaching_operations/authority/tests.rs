use super::*;

use axum::body::{Body, to_bytes};
use axum::http::header::{CACHE_CONTROL, ETAG, LOCATION};
use axum::http::{Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AccountIdentityStore, AuthenticationEmail, AuthenticationRateLimitKey,
    BeginEmailAuthentication, BrowserBindingHash, CompleteEmailAuthentication, CourseRecord,
    CreateCourseCommand, EmailAuthenticationPurpose, EmailChallengeId, EmailChallengeLifetime,
    EmailChallengeSecretHash, Store, TenantContext,
};
use question_model::{ActivityTimestamp, CourseId, TenantId, UserId, UserRole};
use tower::ServiceExt;
use uuid::Uuid;

#[path = "sysadmin_candidate_tests.rs"]
mod sysadmin_candidate_tests;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

struct Fixture {
    app: axum::Router,
    store: Arc<MemoryStore>,
    course: CourseId,
    admin: String,
    instructor: String,
    target: String,
    other: String,
    ordinary: String,
    foreign: String,
    expired: String,
    target_account: String,
    other_account: String,
}

async fn issued_cookie(
    store: &MemoryStore,
    tenant: TenantId,
    user: UserId,
    role: UserRole,
) -> (String, learning_data_access::SessionTokenHash) {
    let subject =
        learning_data_access::SessionSubject::new(tenant, user, "Authority test", vec![role])
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
    let cookie = issued
        .set_cookie
        .split(';')
        .next()
        .expect("cookie")
        .to_owned();
    (cookie, issued.record.token_hash)
}

async fn cookie(store: &MemoryStore, tenant: TenantId, user: UserId, role: UserRole) -> String {
    issued_cookie(store, tenant, user, role).await.0
}

async fn create_account(store: &MemoryStore, user: UserId, suffix: u128, display: &str) {
    let token = EmailChallengeSecretHash::compute(format!("authority-token-{suffix}").as_bytes());
    let binding = BrowserBindingHash::compute(format!("authority-binding-{suffix}").as_bytes());
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(id(900 + suffix)),
            token_hash: token,
            browser_binding: binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(
                format!("authority-rate-{suffix}").as_bytes(),
            ),
            email: AuthenticationEmail::parse(&format!("authority-{suffix}@example.edu"))
                .expect("fixture email"),
            purpose: EmailAuthenticationPurpose::SignInOrRegister,
            lifetime: EmailChallengeLifetime::from_seconds(600).expect("lifetime"),
        })
        .await
        .expect("account challenge");
    store
        .complete_email_authentication(CompleteEmailAuthentication {
            token_hash: token,
            browser_binding: binding,
            proposed_user: user,
            proposed_display_name: display.to_owned(),
        })
        .await
        .expect("account");
}

async fn fixture() -> Fixture {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let course = CourseId::from_uuid(id(2));
    let admin_user = UserId::from_uuid(id(3));
    let instructor_user = UserId::from_uuid(id(4));
    let target_user = UserId::from_uuid(id(5));
    let other_user = UserId::from_uuid(id(6));
    let ordinary_user = UserId::from_uuid(id(7));
    for (user, suffix, display) in [
        (admin_user, 1, "Admin"),
        (instructor_user, 2, "Instructor"),
        (target_user, 3, "Target"),
        (other_user, 4, "Other target"),
        (ordinary_user, 5, "Ordinary"),
    ] {
        create_account(&store, user, suffix, display).await;
    }
    let context = TenantContext::from_authenticated_session(tenant);
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
                authority: crate::test_fixtures::sysadmin_course_creation_authority(
                    store.as_ref(),
                    tenant,
                    course,
                    instructor_user,
                )
                .await,
            },
        )
        .await
        .expect("course");
    let expired = cookie(&store, tenant, admin_user, UserRole::Sysadmin).await;
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000_000))
        .expect("advance clock");
    let (target, target_hash) =
        issued_cookie(&store, tenant, target_user, UserRole::Instructor).await;
    let (other, other_hash) = issued_cookie(&store, tenant, other_user, UserRole::Instructor).await;
    let target_account = store
        .own_account_reference(context, target_hash)
        .await
        .expect("target account reference")
        .reference
        .to_string();
    let other_account = store
        .own_account_reference(context, other_hash)
        .await
        .expect("other account reference")
        .reference
        .to_string();
    let admin = cookie(&store, tenant, admin_user, UserRole::Sysadmin).await;
    let instructor = cookie(&store, tenant, instructor_user, UserRole::Instructor).await;
    let ordinary = cookie(&store, tenant, ordinary_user, UserRole::Student).await;
    let foreign = cookie(
        &store,
        TenantId::from_uuid(id(8)),
        admin_user,
        UserRole::Sysadmin,
    )
    .await;
    Fixture {
        app: crate::course::router(Arc::clone(&store)),
        store,
        course,
        admin,
        instructor,
        target,
        other,
        ordinary,
        foreign,
        expired,
        target_account,
        other_account,
    }
}

async fn json(response: axum::response::Response) -> serde_json::Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("body"),
    )
    .expect("JSON")
}

async fn request(app: &axum::Router, request: Request<Body>) -> axum::response::Response {
    app.clone().oneshot(request).await.expect("response")
}

fn assert_safe(response: &axum::response::Response) {
    assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
}

#[tokio::test]
async fn memory_authority_approval_requires_operator_before_reference_and_tracks_revisions() {
    let fixture = fixture().await;
    let url = format!(
        "/api/teaching/instructor-approvals/{}",
        fixture.target_account
    );
    for (kind, cookie) in [
        ("ordinary", &fixture.ordinary),
        ("expired", &fixture.expired),
    ] {
        let response = request(
            &fixture.app,
            Request::put(&url)
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("denial"),
        )
        .await;
        assert!(
            response.status().is_client_error(),
            "{kind}: {}",
            response.status()
        );
        assert_safe(&response);
    }
    let initial = request(
        &fixture.app,
        Request::put(&url)
            .header("cookie", &fixture.admin)
            .body(Body::empty())
            .expect("approve"),
    )
    .await;
    assert_eq!(initial.status(), StatusCode::OK);
    assert_safe(&initial);
    let etag = initial.headers()[ETAG].to_str().expect("etag").to_owned();
    assert_eq!(
        json(initial).await,
        serde_json::json!({"state":"approved","revision":"1"})
    );
    let reapprove = request(
        &fixture.app,
        Request::put(&url)
            .header("cookie", &fixture.admin)
            .header("if-match", &etag)
            .body(Body::empty())
            .expect("reapprove"),
    )
    .await;
    assert_eq!(reapprove.status(), StatusCode::OK);
    let reapprove_etag = reapprove.headers()[ETAG].to_str().expect("etag").to_owned();
    let no_ambient_course_authority = request(
        &fixture.app,
        Request::get(format!(
            "/api/courses/{}/co-instructor-invitations",
            fixture.course
        ))
        .header("cookie", &fixture.foreign)
        .body(Body::empty())
        .expect("foreign course request"),
    )
    .await;
    assert_eq!(no_ambient_course_authority.status(), StatusCode::NOT_FOUND);
    assert_safe(&no_ambient_course_authority);
    let revoked = request(
        &fixture.app,
        Request::delete(&url)
            .header("cookie", &fixture.admin)
            .header("if-match", reapprove_etag)
            .body(Body::empty())
            .expect("revoke"),
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::OK);
    assert_safe(&revoked);
    assert_eq!(json(revoked).await["state"], "revoked");
}

async fn approve(fixture: &Fixture, account: &str) {
    let response = request(
        &fixture.app,
        Request::put(format!("/api/teaching/instructor-approvals/{account}"))
            .header("cookie", &fixture.admin)
            .body(Body::empty())
            .expect("approval"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}

async fn invite(fixture: &Fixture, account: &str) -> (String, String) {
    invite_for_course(fixture, fixture.course, account).await
}

async fn invite_for_course(fixture: &Fixture, course: CourseId, account: &str) -> (String, String) {
    let response = request(
        &fixture.app,
        Request::post(format!("/api/courses/{}/co-instructor-invitations", course))
            .header("cookie", &fixture.instructor)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"target": account}).to_string(),
            ))
            .expect("invite"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_safe(&response);
    let location = response.headers()[LOCATION]
        .to_str()
        .expect("location")
        .to_owned();
    let etag = response.headers()[ETAG].to_str().expect("etag").to_owned();
    assert!(
        to_bytes(response.into_body(), 64)
            .await
            .expect("empty")
            .is_empty()
    );
    (
        location.rsplit('/').next().expect("reference").to_owned(),
        etag,
    )
}

#[tokio::test]
async fn memory_authority_pages_traverse_course_pending_and_instructor_cursors() {
    let fixture = fixture().await;
    approve(&fixture, &fixture.target_account).await;
    approve(&fixture, &fixture.other_account).await;
    let (target_invitation, target_etag) = invite(&fixture, &fixture.target_account).await;
    let (other_invitation, _) = invite(&fixture, &fixture.other_account).await;
    let course_page = request(
        &fixture.app,
        Request::get(format!(
            "/api/courses/{}/co-instructor-invitations?size=1",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .body(Body::empty())
        .expect("course invitation page"),
    )
    .await;
    assert_eq!(course_page.status(), StatusCode::OK);
    assert_safe(&course_page);
    let course_page = json(course_page).await;
    let course_cursor = course_page["nextCursor"].as_str().expect("course cursor");
    let course_next = request(
        &fixture.app,
        Request::get(format!(
            "/api/courses/{}/co-instructor-invitations?size=1&after={course_cursor}",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .body(Body::empty())
        .expect("next course invitation page"),
    )
    .await;
    let course_next = json(course_next).await;
    assert_ne!(
        course_page["invitations"][0]["reference"],
        course_next["invitations"][0]["reference"]
    );

    let second_course = CourseId::from_uuid(id(9));
    let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id(1)));
    fixture
        .store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: second_course,
                    tenant: TenantId::from_uuid(id(1)),
                    title: "BIOC 302".to_owned(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("term"),
                },
                authority: crate::test_fixtures::sysadmin_course_creation_authority(
                    fixture.store.as_ref(),
                    TenantId::from_uuid(id(1)),
                    second_course,
                    UserId::from_uuid(id(4)),
                )
                .await,
            },
        )
        .await
        .expect("second course");
    let (second_invitation, _) =
        invite_for_course(&fixture, second_course, &fixture.target_account).await;
    let pending_page = request(
        &fixture.app,
        Request::get("/api/account/co-instructor-invitations?size=1")
            .header("cookie", &fixture.target)
            .body(Body::empty())
            .expect("pending page"),
    )
    .await;
    assert_eq!(pending_page.status(), StatusCode::OK);
    let pending_page = json(pending_page).await;
    let pending_cursor = pending_page["nextCursor"].as_str().expect("pending cursor");
    let pending_next = request(
        &fixture.app,
        Request::get(format!(
            "/api/account/co-instructor-invitations?size=1&after={pending_cursor}"
        ))
        .header("cookie", &fixture.target)
        .body(Body::empty())
        .expect("next pending page"),
    )
    .await;
    let pending_next = json(pending_next).await;
    assert_ne!(
        pending_page["invitations"][0]["reference"],
        pending_next["invitations"][0]["reference"]
    );
    let accepted = request(
        &fixture.app,
        Request::post(format!(
            "/api/account/co-instructor-invitations/{target_invitation}"
        ))
        .header("cookie", &fixture.target)
        .header("if-match", target_etag)
        .header("content-type", "application/json")
        .body(Body::from(r#"{"action":"accept"}"#))
        .expect("accept"),
    )
    .await;
    assert_eq!(accepted.status(), StatusCode::NO_CONTENT);
    let instructors = request(
        &fixture.app,
        Request::get(format!(
            "/api/courses/{}/instructors?size=1",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .body(Body::empty())
        .expect("instructor page"),
    )
    .await;
    let instructors = json(instructors).await;
    let instructor_cursor = instructors["nextCursor"]
        .as_str()
        .expect("instructor cursor");
    let instructors_next = request(
        &fixture.app,
        Request::get(format!(
            "/api/courses/{}/instructors?size=1&after={instructor_cursor}",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .body(Body::empty())
        .expect("next instructor page"),
    )
    .await;
    let instructors_next = json(instructors_next).await;
    assert_ne!(
        instructors["instructors"][0]["membership"],
        instructors_next["instructors"][0]["membership"]
    );
    assert_ne!(target_invitation, second_invitation);
    assert_ne!(target_invitation, other_invitation);
}

#[tokio::test]
async fn memory_authority_target_search_is_exact_course_authorized_and_pii_free() {
    let fixture = fixture().await;
    let targets = format!("/api/courses/{}/co-instructor-targets", fixture.course);

    for (kind, cookie) in [
        ("student", &fixture.ordinary),
        ("outsider", &fixture.foreign),
    ] {
        let denied = request(
            &fixture.app,
            Request::get(format!("{targets}?query=target"))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("target-search denial"),
        )
        .await;
        assert_eq!(denied.status(), StatusCode::NOT_FOUND, "{kind}");
        assert_safe(&denied);
    }

    for invalid_url in [
        targets.clone(),
        format!("{targets}?query=t"),
        format!("{targets}?query=target&size=0"),
        format!("{targets}?query=target&unexpected=value"),
    ] {
        let invalid = request(
            &fixture.app,
            Request::get(invalid_url)
                .header("cookie", &fixture.instructor)
                .body(Body::empty())
                .expect("invalid target search"),
        )
        .await;
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        assert_safe(&invalid);
    }

    approve(&fixture, &fixture.target_account).await;
    approve(&fixture, &fixture.other_account).await;
    let first = request(
        &fixture.app,
        Request::get(format!("{targets}?query=target&size=1"))
            .header("cookie", &fixture.instructor)
            .body(Body::empty())
            .expect("first target page"),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    assert_safe(&first);
    let first = json(first).await;
    assert_eq!(first["targets"][0]["account"]["display"], "Target");
    let cursor = first["nextCursor"].as_str().expect("target cursor");
    let second = request(
        &fixture.app,
        Request::get(format!("{targets}?query=target&size=1&after={cursor}"))
            .header("cookie", &fixture.instructor)
            .body(Body::empty())
            .expect("second target page"),
    )
    .await;
    assert_eq!(second.status(), StatusCode::OK);
    assert_safe(&second);
    let second = json(second).await;
    assert_eq!(second["targets"][0]["account"]["display"], "Other target");
    let serialized = serde_json::to_string(&first).expect("serialized target page");
    assert!(!serialized.contains('@'));
    assert!(!serialized.contains("00000000-0000-0000-0000"));
    assert!(
        first["targets"]
            .as_array()
            .expect("targets")
            .iter()
            .all(|target| {
                target["account"].get("reference").is_some()
                    && target["account"].get("display").is_some()
                    && target["account"].get("email").is_none()
                    && target["account"].get("userId").is_none()
            })
    );

    let (invitation, etag) = invite(&fixture, &fixture.target_account).await;
    let accepted = request(
        &fixture.app,
        Request::post(format!(
            "/api/account/co-instructor-invitations/{invitation}"
        ))
        .header("cookie", &fixture.target)
        .header("if-match", etag)
        .header("content-type", "application/json")
        .body(Body::from(r#"{"action":"accept"}"#))
        .expect("accept target"),
    )
    .await;
    assert_eq!(accepted.status(), StatusCode::NO_CONTENT);
    let active_excluded = request(
        &fixture.app,
        Request::get(format!("{targets}?query=target"))
            .header("cookie", &fixture.instructor)
            .body(Body::empty())
            .expect("active instructor exclusion"),
    )
    .await;
    let active_excluded = json(active_excluded).await;
    assert_eq!(
        active_excluded["targets"]
            .as_array()
            .expect("targets")
            .len(),
        1
    );
    assert_eq!(
        active_excluded["targets"][0]["account"]["display"],
        "Other target"
    );

    let _ = invite(&fixture, &fixture.other_account).await;
    let pending_excluded = request(
        &fixture.app,
        Request::get(format!("{targets}?query=target"))
            .header("cookie", &fixture.instructor)
            .body(Body::empty())
            .expect("pending invitation exclusion"),
    )
    .await;
    assert_eq!(pending_excluded.status(), StatusCode::OK);
    assert_safe(&pending_excluded);
    assert_eq!(
        json(pending_excluded).await["targets"],
        serde_json::json!([])
    );
}

#[tokio::test]
async fn memory_authority_invitations_are_typed_replayable_and_cas_revocable() {
    let fixture = fixture().await;
    approve(&fixture, &fixture.target_account).await;
    let (invitation, etag) = invite(&fixture, &fixture.target_account).await;
    let replay = invite(&fixture, &fixture.target_account).await;
    assert_eq!(replay, (invitation.clone(), etag.clone()));
    let listed = request(
        &fixture.app,
        Request::get(format!(
            "/api/courses/{}/co-instructor-invitations",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .body(Body::empty())
        .expect("list"),
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    assert_safe(&listed);
    let listed = json(listed).await;
    assert_eq!(listed["invitations"][0]["reference"], invitation);
    assert_eq!(
        listed["invitations"][0]["target"]["account"]["reference"],
        fixture.target_account
    );
    assert_eq!(
        listed["invitations"][0]["target"]["approval"]["state"],
        "approved"
    );
    let stale = request(
        &fixture.app,
        Request::delete(format!(
            "/api/courses/{}/co-instructor-invitations/{invitation}",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .header("if-match", "\"99\"")
        .body(Body::empty())
        .expect("stale revoke"),
    )
    .await;
    assert_eq!(stale.status(), StatusCode::PRECONDITION_FAILED);
    let revoked = request(
        &fixture.app,
        Request::delete(format!(
            "/api/courses/{}/co-instructor-invitations/{invitation}",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .header("if-match", etag)
        .body(Body::empty())
        .expect("revoke"),
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::NO_CONTENT);
    assert_safe(&revoked);
}

#[tokio::test]
async fn memory_authority_pending_target_actions_are_private_and_expire() {
    let fixture = fixture().await;
    approve(&fixture, &fixture.target_account).await;
    approve(&fixture, &fixture.other_account).await;
    let (invitation, etag) = invite(&fixture, &fixture.target_account).await;
    let target_list = request(
        &fixture.app,
        Request::get("/api/account/co-instructor-invitations")
            .header("cookie", &fixture.target)
            .body(Body::empty())
            .expect("target list"),
    )
    .await;
    assert_eq!(target_list.status(), StatusCode::OK);
    assert_eq!(
        json(target_list).await["invitations"][0]["reference"],
        invitation
    );
    let wrong_target = request(
        &fixture.app,
        Request::post(format!(
            "/api/account/co-instructor-invitations/{invitation}"
        ))
        .header("cookie", &fixture.other)
        .header("if-match", &etag)
        .body(Body::from("private non-json"))
        .expect("wrong target"),
    )
    .await;
    assert_eq!(wrong_target.status(), StatusCode::NOT_FOUND);
    assert_safe(&wrong_target);
    let malformed = request(
        &fixture.app,
        Request::post(format!(
            "/api/account/co-instructor-invitations/{invitation}"
        ))
        .header("cookie", &fixture.target)
        .header("if-match", &etag)
        .body(Body::from("not json"))
        .expect("malformed"),
    )
    .await;
    assert_eq!(malformed.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let accepted = request(
        &fixture.app,
        Request::post(format!(
            "/api/account/co-instructor-invitations/{invitation}"
        ))
        .header("cookie", &fixture.target)
        .header("if-match", &etag)
        .header("content-type", "application/json")
        .body(Body::from(r#"{"action":"accept"}"#))
        .expect("accept"),
    )
    .await;
    assert_eq!(accepted.status(), StatusCode::NO_CONTENT);
    assert_safe(&accepted);
    let (decline, decline_etag) = invite(&fixture, &fixture.other_account).await;
    let declined = request(
        &fixture.app,
        Request::post(format!("/api/account/co-instructor-invitations/{decline}"))
            .header("cookie", &fixture.other)
            .header("if-match", decline_etag)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"action":"decline"}"#))
            .expect("decline"),
    )
    .await;
    assert_eq!(declined.status(), StatusCode::NO_CONTENT);
    let (expiry, _) = invite(&fixture, &fixture.other_account).await;
    fixture
        .store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(3_000_000_000))
        .expect("expire");
    let active_other = cookie(
        &fixture.store,
        TenantId::from_uuid(id(1)),
        UserId::from_uuid(id(6)),
        UserRole::Instructor,
    )
    .await;
    let expired = request(
        &fixture.app,
        Request::post(format!("/api/account/co-instructor-invitations/{expiry}"))
            .header("cookie", &active_other)
            .header("if-match", "\"1\"")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"action":"accept"}"#))
            .expect("expired"),
    )
    .await;
    assert_eq!(expired.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn memory_authority_instructor_page_and_removal_are_revisioned_and_safe() {
    let fixture = fixture().await;
    approve(&fixture, &fixture.target_account).await;
    let (invitation, etag) = invite(&fixture, &fixture.target_account).await;
    let accepted = request(
        &fixture.app,
        Request::post(format!(
            "/api/account/co-instructor-invitations/{invitation}"
        ))
        .header("cookie", &fixture.target)
        .header("if-match", etag)
        .header("content-type", "application/json")
        .body(Body::from(r#"{"action":"accept"}"#))
        .expect("accept"),
    )
    .await;
    assert_eq!(accepted.status(), StatusCode::NO_CONTENT);
    let page = request(
        &fixture.app,
        Request::get(format!("/api/courses/{}/instructors", fixture.course))
            .header("cookie", &fixture.instructor)
            .body(Body::empty())
            .expect("instructors"),
    )
    .await;
    assert_eq!(page.status(), StatusCode::OK);
    assert_safe(&page);
    let roster_etag = page.headers()[ETAG].to_str().expect("etag").to_owned();
    let page = json(page).await;
    assert_eq!(page["rosterRevision"], "2");
    assert!(
        page["instructors"]
            .as_array()
            .expect("instructors")
            .iter()
            .all(|value| {
                value["membership"]
                    .as_str()
                    .is_some_and(|v| v.starts_with("M-"))
                    && value["account"]["reference"]
                        .as_str()
                        .is_some_and(|v| v.starts_with("U-"))
                    && value["account"]["display"].is_string()
            })
    );
    let target_membership = page["instructors"]
        .as_array()
        .expect("instructors")
        .iter()
        .find(|value| value["account"]["reference"] == fixture.target_account)
        .and_then(|value| value["membership"].as_str())
        .expect("target membership")
        .to_owned();
    let stale = request(
        &fixture.app,
        Request::delete(format!(
            "/api/courses/{}/instructors/{target_membership}",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .header("if-match", "\"1\"")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("stale"),
    )
    .await;
    assert_eq!(stale.status(), StatusCode::PRECONDITION_FAILED);
    let removed = request(
        &fixture.app,
        Request::delete(format!(
            "/api/courses/{}/instructors/{target_membership}",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .header("if-match", &roster_etag)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("remove"),
    )
    .await;
    assert_eq!(removed.status(), StatusCode::NO_CONTENT);
    assert_safe(&removed);
    let after_removal = request(
        &fixture.app,
        Request::get(format!("/api/courses/{}/instructors", fixture.course))
            .header("cookie", &fixture.instructor)
            .body(Body::empty())
            .expect("after-removal page"),
    )
    .await;
    let after_removal_etag = after_removal.headers()[ETAG]
        .to_str()
        .expect("etag")
        .to_owned();
    let not_instructor = request(
        &fixture.app,
        Request::delete(format!(
            "/api/courses/{}/instructors/{target_membership}",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .header("if-match", after_removal_etag)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("repeat"),
    )
    .await;
    assert_eq!(not_instructor.status(), StatusCode::PRECONDITION_FAILED);
    let final_page = request(
        &fixture.app,
        Request::get(format!("/api/courses/{}/instructors", fixture.course))
            .header("cookie", &fixture.instructor)
            .body(Body::empty())
            .expect("final page"),
    )
    .await;
    let final_etag = final_page.headers()[ETAG]
        .to_str()
        .expect("etag")
        .to_owned();
    let final_membership = json(final_page).await["instructors"][0]["membership"]
        .as_str()
        .expect("final member")
        .to_owned();
    let final_refusal = request(
        &fixture.app,
        Request::delete(format!(
            "/api/courses/{}/instructors/{final_membership}",
            fixture.course
        ))
        .header("cookie", &fixture.instructor)
        .header("if-match", final_etag)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("final removal"),
    )
    .await;
    assert_eq!(final_refusal.status(), StatusCode::PRECONDITION_FAILED);
}
