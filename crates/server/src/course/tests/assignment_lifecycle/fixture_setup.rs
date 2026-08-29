use super::super::fixtures::{id, issued_cookie_for_tenant, publish_fixture};
use super::*;
use axum::Router;
use axum::body::Body;
use axum::http::Request;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AccountIdentityStore, ApproveInstructorAccount, AuthenticationEmail,
    AuthenticationRateLimitKey, BeginEmailAuthentication, BrowserBindingHash, CatalogStore,
    CompleteEmailAuthentication, CourseRosterStore, EmailAuthenticationPurpose, EmailChallengeId,
    EmailChallengeLifetime, EmailChallengeSecretHash, SessionLifetime, SessionSubject,
    TeachingAuthorityStore, TenantContext, UpsertCourseMember,
};
use question_model::{ActivityTimestamp, CourseId, QuestionId, TenantId, UserId, UserRole};
use std::sync::Arc;

pub(super) struct AssignmentFixture {
    pub(super) store: Arc<MemoryStore>,
    pub(super) tenant: TenantId,
    pub(super) context: TenantContext,
    pub(super) instructor: UserId,
    pub(super) student: UserId,
    pub(super) instructor_cookie: String,
    pub(super) student_cookie: String,
    pub(super) outsider_cookie: String,
    pub(super) sysadmin_cookie: String,
    pub(super) foreign_cookie: String,
    pub(super) app: Router,
    pub(super) course: CourseId,
    pub(super) question_id: QuestionId,
}

async fn create_account(store: &MemoryStore, user: UserId, suffix: u128) {
    let token = EmailChallengeSecretHash::compute(
        format!("assignment-lifecycle-account-token-{suffix}").as_bytes(),
    );
    let binding = BrowserBindingHash::compute(
        format!("assignment-lifecycle-account-binding-{suffix}").as_bytes(),
    );
    store
        .begin_email_authentication(BeginEmailAuthentication {
            id: EmailChallengeId::from_uuid(id(100 + suffix)),
            token_hash: token,
            browser_binding: binding,
            email_rate_limit_key: AuthenticationRateLimitKey::compute(
                format!("assignment-lifecycle-account-rate-{suffix}").as_bytes(),
            ),
            email: AuthenticationEmail::parse(&format!(
                "assignment-lifecycle-{suffix}@example.edu"
            ))
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
            proposed_display_name: "Assignment lifecycle fixture".to_owned(),
        })
        .await
        .expect("fixture account");
}

pub(super) async fn build() -> AssignmentFixture {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(2));
    let student = UserId::from_uuid(id(3));
    let outsider = UserId::from_uuid(id(4));
    let sysadmin = UserId::from_uuid(id(5));
    let foreign_tenant = TenantId::from_uuid(id(6));
    let foreign_user = UserId::from_uuid(id(7));
    create_account(&store, instructor, 2).await;
    create_account(&store, sysadmin, 5).await;
    let instructor_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let student_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Student], student).await;
    let outsider_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Instructor], outsider).await;
    let sysadmin_issued = crate::auth::issue_session(
        store.as_ref(),
        SessionSubject::new(
            tenant,
            sysadmin,
            "Assignment lifecycle Sysadmin",
            vec![UserRole::Sysadmin],
        )
        .expect("fixture Sysadmin session subject"),
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("fixture session lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("fixture Sysadmin session");
    store
        .approve_instructor_account(
            context,
            ApproveInstructorAccount {
                session: sysadmin_issued.record.token_hash,
                target: instructor,
                expected_revision: None,
            },
        )
        .await
        .expect("authenticated Sysadmin instructor approval");
    let sysadmin_cookie = sysadmin_issued
        .set_cookie
        .split(';')
        .next()
        .expect("fixture Sysadmin cookie")
        .to_owned();
    let foreign_cookie = issued_cookie_for_tenant(
        &store,
        foreign_tenant,
        vec![UserRole::Instructor],
        foreign_user,
    )
    .await;
    let app = router(Arc::clone(&store));

    let created_course = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/courses")
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"BIOC 301: Biochemistry","term":{"startDate":"2026-08-24","endDate":"2026-12-18","timeZone":"America/Chicago"}}"#,
                ))
                .expect("course request"),
        )
        .await
        .expect("course response");
    assert_eq!(created_course.status(), StatusCode::CREATED);
    let created_course = super::response_json(created_course).await;
    let course: CourseId =
        serde_json::from_value(created_course["id"].clone()).expect("course ID response");
    assert_eq!(created_course["role"], "instructor");

    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Biochemistry Student".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("student membership save");
    let reference = publish_fixture(&store, context, tenant, instructor).await;
    let question_id = store
        .get_catalog_problem(context, reference)
        .await
        .expect("catalog fixture lookup")
        .expect("published fixture")
        .question_id;

    AssignmentFixture {
        store,
        tenant,
        context,
        instructor,
        student,
        instructor_cookie,
        student_cookie,
        outsider_cookie,
        sysadmin_cookie,
        foreign_cookie,
        app,
        course,
        question_id,
    }
}
