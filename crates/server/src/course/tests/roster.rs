use std::sync::Mutex;

use super::fixtures::{id, issued_cookie_for_tenant, policies, publish_fixture};
use crate::course::{
    CourseInvitationDelivery, CourseInvitationDeliveryError, CourseInvitationIssuer,
    CourseInvitationSecret, LocalTeachingRosterDirectory, LocalTeachingRosterIdentity,
    UnavailableCourseInvitationDelivery, router_with_invitations,
    router_with_invitations_and_local_teaching,
};
use async_trait::async_trait;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use learning_data_access::{
    AssignmentRecord, AuthenticationEmail, ClaimCourseInvitation, CourseInvitationSecretHash,
    CourseRecord, CourseRosterStore, CourseRosterSupportAction, Store, TenantContext,
};
use question_model::{
    AssignmentId, CourseId, CourseMembership, CourseMembershipRole, TenantId, UserId, UserRole,
};
use tower::ServiceExt;

#[derive(Default)]
struct CapturingInvitationDelivery {
    deliveries: Mutex<Vec<(String, String)>>,
}

#[tokio::test]
async fn local_teaching_roster_uses_alias_resolution_and_canonical_member_upsert() {
    let store = std::sync::Arc::new(learning_data_access::in_memory::MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1_150));
    let instructor = UserId::from_uuid(id(1_151));
    let mary = UserId::from_uuid(id(1_152));
    let jack = UserId::from_uuid(id(1_153));
    let course = CourseId::from_uuid(id(1_154));
    let context = TenantContext::from_authenticated_session(tenant);
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Genetics local teaching".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("course fixture");
    let instructor_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let directory = LocalTeachingRosterDirectory::new([
        (
            "mary".to_string(),
            LocalTeachingRosterIdentity {
                tenant,
                user: mary,
                display_name: "Mary Fake Student".to_string(),
                roles: vec![UserRole::Student],
            },
        ),
        (
            "jack".to_string(),
            LocalTeachingRosterIdentity {
                tenant,
                user: jack,
                display_name: "Jack Fake Student".to_string(),
                roles: vec![UserRole::Student],
            },
        ),
        (
            "instructor".to_string(),
            LocalTeachingRosterIdentity {
                tenant,
                user: instructor,
                display_name: "Dr. Fake Professor".to_string(),
                roles: vec![UserRole::Instructor],
            },
        ),
    ])
    .expect("unique local student aliases");
    let app = router_with_invitations_and_local_teaching(
        std::sync::Arc::clone(&store),
        CourseInvitationIssuer::unavailable(),
        std::sync::Arc::new(UnavailableCourseInvitationDelivery),
        Some(std::sync::Arc::new(directory)),
    );

    let roster = app
        .clone()
        .oneshot(request(
            "GET",
            format!("/api/courses/{course}/roster"),
            &instructor_cookie,
            None,
        ))
        .await
        .expect("local roster response");
    let roster = json(roster).await;
    assert_eq!(roster["rosterMode"], "localTeaching");
    assert!(roster.get("pendingInvitations").is_none());
    assert!(roster.get("allowedEmailDomains").is_none());
    assert!(roster.get("signupPosture").is_none());
    assert_eq!(
        roster["localTeachingLearners"],
        serde_json::json!([
            {"alias": "jack", "displayName": "Jack Fake Student"},
            {"alias": "mary", "displayName": "Mary Fake Student"}
        ])
    );

    let rejected_instructor_alias = app
        .clone()
        .oneshot(request(
            "POST",
            format!("/api/courses/{course}/local-teaching-members"),
            &instructor_cookie,
            Some(serde_json::json!({"learnerAlias": "instructor"})),
        ))
        .await
        .expect("instructor alias response");
    assert_eq!(rejected_instructor_alias.status(), StatusCode::NOT_FOUND);

    let unknown_alias = app
        .clone()
        .oneshot(request(
            "POST",
            format!("/api/courses/{course}/local-teaching-members"),
            &instructor_cookie,
            Some(serde_json::json!({"learnerAlias": "unknown"})),
        ))
        .await
        .expect("unknown alias response");
    assert_eq!(unknown_alias.status(), StatusCode::NOT_FOUND);

    let malformed_request = app
        .clone()
        .oneshot(request(
            "POST",
            format!("/api/courses/{course}/local-teaching-members"),
            &instructor_cookie,
            Some(serde_json::json!({"learnerAlias": "mary", "userId": mary.as_uuid()})),
        ))
        .await
        .expect("malformed local teaching request response");
    assert_eq!(malformed_request.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let noninstructor_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Student], jack).await;
    let noninstructor = app
        .clone()
        .oneshot(request(
            "POST",
            format!("/api/courses/{course}/local-teaching-members"),
            &noninstructor_cookie,
            Some(serde_json::json!({"learnerAlias": "mary"})),
        ))
        .await
        .expect("noninstructor response");
    assert_eq!(noninstructor.status(), StatusCode::NOT_FOUND);

    let foreign_cookie = issued_cookie_for_tenant(
        &store,
        TenantId::from_uuid(id(1_155)),
        vec![UserRole::Instructor],
        UserId::from_uuid(id(1_156)),
    )
    .await;
    let foreign = app
        .clone()
        .oneshot(request(
            "POST",
            format!("/api/courses/{course}/local-teaching-members"),
            &foreign_cookie,
            Some(serde_json::json!({"learnerAlias": "mary"})),
        ))
        .await
        .expect("foreign tenant response");
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);

    let first = app
        .clone()
        .oneshot(request(
            "POST",
            format!("/api/courses/{course}/local-teaching-members"),
            &instructor_cookie,
            Some(serde_json::json!({"learnerAlias": "mary"})),
        ))
        .await
        .expect("Mary activation response");
    assert_eq!(first.status(), StatusCode::OK);
    let first = json(first).await;
    assert_eq!(first["member"]["displayName"], "Mary Fake Student");
    assert_eq!(first["member"]["status"], "active");
    assert!(first["member"].get("source").is_none());
    assert!(first["member"].get("userId").is_none());

    let repeated = app
        .clone()
        .oneshot(request(
            "POST",
            format!("/api/courses/{course}/local-teaching-members"),
            &instructor_cookie,
            Some(serde_json::json!({"learnerAlias": "mary"})),
        ))
        .await
        .expect("repeated Mary activation response");
    assert_eq!(repeated.status(), StatusCode::OK);
    let repeated = json(repeated).await;
    assert_eq!(repeated["member"], first["member"]);

    for unavailable_route in [
        format!("/api/courses/{course}/invitations"),
        format!("/api/courses/{course}/enrollment-policy"),
        format!("/api/courses/{course}/roster-imports/preview"),
    ] {
        let response = app
            .clone()
            .oneshot(request(
                "POST",
                unavailable_route,
                &instructor_cookie,
                Some(serde_json::json!({})),
            ))
            .await
            .expect("local unavailable route response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    let production_shape = router_with_invitations(
        store,
        CourseInvitationIssuer::unavailable(),
        std::sync::Arc::new(UnavailableCourseInvitationDelivery),
    )
    .oneshot(request(
        "POST",
        format!("/api/courses/{course}/local-teaching-members"),
        &instructor_cookie,
        Some(serde_json::json!({"learnerAlias": "mary"})),
    ))
    .await
    .expect("normal roster route response");
    assert_eq!(production_shape.status(), StatusCode::NOT_FOUND);
}

#[async_trait]
impl CourseInvitationDelivery for CapturingInvitationDelivery {
    fn is_configured(&self) -> bool {
        true
    }

    async fn send_course_invitation(
        &self,
        email: &AuthenticationEmail,
        invitation_secret: &CourseInvitationSecret,
    ) -> Result<(), CourseInvitationDeliveryError> {
        self.deliveries
            .lock()
            .expect("delivery fixture lock")
            .push((email.delivery().to_string(), invitation_secret.encoded()));
        Ok(())
    }
}

async fn json(response: axum::response::Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), 128 * 1_024)
        .await
        .expect("bounded response body");
    serde_json::from_slice(&body).expect("JSON response")
}

fn request(
    method: &str,
    uri: String,
    cookie: &str,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("cookie", cookie);
    let body = match body {
        Some(body) => {
            request = request.header("content-type", "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    request.body(body).expect("fixture request")
}

#[tokio::test]
async fn sysadmin_roster_support_is_audited_without_granting_grade_export() {
    let store = std::sync::Arc::new(learning_data_access::in_memory::MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1_080));
    let instructor = UserId::from_uuid(id(1_081));
    let sysadmin = UserId::from_uuid(id(1_082));
    let course = CourseId::from_uuid(id(1_083));
    let context = TenantContext::from_authenticated_session(tenant);
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Roster support".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("course fixture");
    let sysadmin_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Sysadmin], sysadmin).await;
    let app = router_with_invitations(
        std::sync::Arc::clone(&store),
        CourseInvitationIssuer::from_server_secret([0x61; 32]),
        std::sync::Arc::new(CapturingInvitationDelivery::default()),
    );

    let roster = app
        .clone()
        .oneshot(request(
            "GET",
            format!("/api/courses/{course}/roster"),
            &sysadmin_cookie,
            None,
        ))
        .await
        .expect("sysadmin support roster response");
    assert_eq!(roster.status(), StatusCode::OK);

    let mut invite = request(
        "POST",
        format!("/api/courses/{course}/invitations"),
        &sysadmin_cookie,
        Some(serde_json::json!({
            "email": "supported@example.edu",
            "rosterId": "900108000",
        })),
    );
    invite.headers_mut().insert(
        "idempotency-key",
        "sysadmin-support".parse().expect("header"),
    );
    let invite = app
        .oneshot(invite)
        .await
        .expect("support invitation response");
    assert_eq!(invite.status(), StatusCode::ACCEPTED);

    assert_eq!(
        store
            .roster_support_audits()
            .expect("support audit events")
            .iter()
            .map(|event| event.action)
            .collect::<Vec<_>>(),
        vec![
            CourseRosterSupportAction::ListRoster,
            CourseRosterSupportAction::CreateInvitation,
        ]
    );

    let export = router_with_invitations(
        std::sync::Arc::clone(&store),
        CourseInvitationIssuer::from_server_secret([0x61; 32]),
        std::sync::Arc::new(CapturingInvitationDelivery::default()),
    )
    .oneshot(request(
        "POST",
        format!(
            "/api/courses/{course}/assignments/{}/grade-export.csv",
            AssignmentId::from_uuid(id(1_084))
        ),
        &sysadmin_cookie,
        None,
    ))
    .await
    .expect("grade export response");
    assert_eq!(export.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn roster_http_is_instructor_scoped_secret_free_and_idempotent() {
    let store = std::sync::Arc::new(learning_data_access::in_memory::MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1_100));
    let instructor = UserId::from_uuid(id(1_101));
    let student = UserId::from_uuid(id(1_102));
    let outsider = UserId::from_uuid(id(1_103));
    let course = CourseId::from_uuid(id(1_104));
    let context = TenantContext::from_authenticated_session(tenant);
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "BIOC 301".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("course fixture");
    let instructor_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let outsider_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Instructor], outsider).await;
    let delivery = std::sync::Arc::new(CapturingInvitationDelivery::default());
    let app = router_with_invitations(
        std::sync::Arc::clone(&store),
        CourseInvitationIssuer::from_server_secret([0x51; 32]),
        std::sync::Arc::clone(&delivery) as std::sync::Arc<dyn CourseInvitationDelivery>,
    );

    let hidden = app
        .clone()
        .oneshot(request(
            "GET",
            format!("/api/courses/{course}/roster"),
            &outsider_cookie,
            None,
        ))
        .await
        .expect("outsider response");
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);

    let initial = app
        .clone()
        .oneshot(request(
            "GET",
            format!("/api/courses/{course}/roster"),
            &instructor_cookie,
            None,
        ))
        .await
        .expect("initial roster response");
    assert_eq!(initial.status(), StatusCode::OK);
    assert_eq!(initial.headers()["etag"], "\"1\"");
    let initial = json(initial).await;
    assert_eq!(initial["rosterMode"], "emailEnrollment");
    assert_eq!(initial["members"], serde_json::json!([]));
    assert_eq!(initial["pendingInvitations"], serde_json::json!([]));

    let invite_body = serde_json::json!({
        "email": "NetID@mail.roosevelt.edu",
        "rosterId": "900123456",
    });
    let mut redemption_paths = Vec::new();
    for _ in 0..2 {
        let mut invite = request(
            "POST",
            format!("/api/courses/{course}/invitations"),
            &instructor_cookie,
            Some(invite_body.clone()),
        );
        invite
            .headers_mut()
            .insert("idempotency-key", "invite-netid".parse().expect("header"));
        let response = app.clone().oneshot(invite).await.expect("invite response");
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let body = json(response).await;
        assert_eq!(body["invitation"]["email"], "NetID@mail.roosevelt.edu");
        assert_eq!(body["emailDelivery"], "sent");
        redemption_paths.push(
            body["redemptionPath"]
                .as_str()
                .expect("one-time redemption path")
                .to_string(),
        );
        assert!(body.to_string().find("userId").is_none());
    }
    assert_eq!(redemption_paths[0], redemption_paths[1]);
    assert!(redemption_paths[0].starts_with("/course-invitations/redeem#token="));
    let encoded_secret = {
        let deliveries = delivery.deliveries.lock().expect("delivery fixture lock");
        assert_eq!(deliveries.len(), 2, "idempotent retry may resend safely");
        assert_eq!(deliveries[0], deliveries[1]);
        deliveries[0].1.clone()
    };

    let roster = app
        .clone()
        .oneshot(request(
            "GET",
            format!("/api/courses/{course}/roster"),
            &instructor_cookie,
            None,
        ))
        .await
        .expect("roster response");
    assert_eq!(roster.headers()["etag"], "\"2\"");
    let roster = json(roster).await;
    assert_eq!(roster["pendingInvitations"].as_array().unwrap().len(), 1);
    assert!(roster.to_string().find("invitedBy").is_none());
    assert!(roster.to_string().find("claimedBy").is_none());
    assert!(roster.to_string().find("redemptionPath").is_none());

    let mut correction = request(
        "POST",
        format!("/api/courses/{course}/invitations"),
        &instructor_cookie,
        Some(serde_json::json!({
            "email": "mistyped@example.edu",
            "rosterId": "900654321",
        })),
    );
    correction.headers_mut().insert(
        "idempotency-key",
        "mistyped-address".parse().expect("header"),
    );
    let correction = app
        .clone()
        .oneshot(correction)
        .await
        .expect("second invitation response");
    assert_eq!(correction.status(), StatusCode::ACCEPTED);
    let correction = json(correction).await;
    let correction_id = correction["invitation"]["invitationId"]
        .as_str()
        .expect("invitation ID");
    let mut cancel = request(
        "DELETE",
        format!("/api/courses/{course}/invitations/{correction_id}"),
        &instructor_cookie,
        None,
    );
    cancel
        .headers_mut()
        .insert("if-match", "\"3\"".parse().expect("header"));
    let cancelled = app
        .clone()
        .oneshot(cancel)
        .await
        .expect("cancel invitation response");
    assert_eq!(cancelled.status(), StatusCode::OK);
    assert_eq!(cancelled.headers()["etag"], "\"4\"");

    let secret = URL_SAFE_NO_PAD
        .decode(encoded_secret)
        .expect("canonical captured secret");
    let claimed = store
        .claim_course_invitation(ClaimCourseInvitation {
            token_hash: CourseInvitationSecretHash::compute(&secret),
            user: student,
            verified_email: AuthenticationEmail::parse("netid@mail.roosevelt.edu")
                .expect("verified email"),
            display_name: "Biochemistry Student".to_string(),
        })
        .await
        .expect("invitation claim");
    assert_eq!(claimed.course, course);

    let roster = app
        .clone()
        .oneshot(request(
            "GET",
            format!("/api/courses/{course}/roster"),
            &instructor_cookie,
            None,
        ))
        .await
        .expect("claimed roster response");
    assert_eq!(roster.headers()["etag"], "\"5\"");
    let roster_body = json(roster).await;
    assert_eq!(
        roster_body["pendingInvitations"],
        serde_json::json!([]),
        "claimed and revoked invitations are not presented as pending"
    );
    let member = &roster_body["members"][0];
    assert_eq!(member["rosterId"], "900123456");
    assert_eq!(member["role"], "student");
    assert!(member.get("userId").is_none());
    assert!(member.get("studentId").is_none());

    let student_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Student], student).await;
    let student_roster = app
        .clone()
        .oneshot(request(
            "GET",
            format!("/api/courses/{course}/roster"),
            &student_cookie,
            None,
        ))
        .await
        .expect("student roster response");
    assert_eq!(student_roster.status(), StatusCode::FORBIDDEN);

    let mut policy = request(
        "PUT",
        format!("/api/courses/{course}/enrollment-policy"),
        &instructor_cookie,
        Some(serde_json::json!({
            "allowedEmailDomains": [{
                "domain": "mail.roosevelt.edu",
                "includeSubdomains": false,
            }],
            "signupPosture": "permittedDomains",
        })),
    );
    policy
        .headers_mut()
        .insert("if-match", "\"5\"".parse().expect("header"));
    let updated_policy = app.clone().oneshot(policy).await.expect("policy response");
    assert_eq!(updated_policy.status(), StatusCode::OK);
    assert_eq!(updated_policy.headers()["etag"], "\"6\"");

    let delivery_count = delivery
        .deliveries
        .lock()
        .expect("delivery fixture lock")
        .len();
    let mut suffix_confusion = request(
        "POST",
        format!("/api/courses/{course}/invitations"),
        &instructor_cookie,
        Some(serde_json::json!({
            "email": "student@mail.roosevelt.edu.attacker.example",
            "rosterId": "900999999",
        })),
    );
    suffix_confusion.headers_mut().insert(
        "idempotency-key",
        "suffix-confusion".parse().expect("header"),
    );
    let suffix_confusion = app
        .clone()
        .oneshot(suffix_confusion)
        .await
        .expect("suffix-confusion response");
    assert_eq!(suffix_confusion.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        delivery
            .deliveries
            .lock()
            .expect("delivery fixture lock")
            .len(),
        delivery_count
    );

    let member_id = member["memberId"].as_str().expect("member ID");
    let mut revoke = request(
        "DELETE",
        format!("/api/courses/{course}/members/{member_id}"),
        &instructor_cookie,
        None,
    );
    revoke
        .headers_mut()
        .insert("if-match", "\"6\"".parse().expect("header"));
    let revoked = app.oneshot(revoke).await.expect("revoke response");
    assert_eq!(revoked.status(), StatusCode::OK);
    assert_eq!(revoked.headers()["etag"], "\"7\"");

    let copy_only_app = router_with_invitations(
        std::sync::Arc::clone(&store),
        CourseInvitationIssuer::from_server_secret([0x51; 32]),
        std::sync::Arc::new(UnavailableCourseInvitationDelivery),
    );
    let mut copy_only = request(
        "POST",
        format!("/api/courses/{course}/invitations"),
        &instructor_cookie,
        Some(serde_json::json!({
            "email": "copy-link@mail.roosevelt.edu",
            "rosterId": "900123499",
        })),
    );
    copy_only
        .headers_mut()
        .insert("idempotency-key", "copy-link-only".parse().expect("header"));
    let copy_only = copy_only_app
        .oneshot(copy_only)
        .await
        .expect("copy-only invitation response");
    assert_eq!(copy_only.status(), StatusCode::ACCEPTED);
    let copy_only = json(copy_only).await;
    assert_eq!(copy_only["emailDelivery"], "notSent");
    assert!(
        copy_only["redemptionPath"]
            .as_str()
            .expect("copy-only redemption path")
            .starts_with("/course-invitations/redeem#token=")
    );
}

#[tokio::test]
async fn roster_csv_preview_is_bounded_and_commit_invites_only_ready_rows() {
    let store = std::sync::Arc::new(learning_data_access::in_memory::MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1_200));
    let instructor = UserId::from_uuid(id(1_201));
    let course = CourseId::from_uuid(id(1_202));
    let context = TenantContext::from_authenticated_session(tenant);
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Bulk roster".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("course fixture");
    let instructor_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let delivery = std::sync::Arc::new(CapturingInvitationDelivery::default());
    let app = router_with_invitations(
        std::sync::Arc::clone(&store),
        CourseInvitationIssuer::from_server_secret([0x61; 32]),
        std::sync::Arc::clone(&delivery) as std::sync::Arc<dyn CourseInvitationDelivery>,
    );
    let csv = "email,roster_id\nready@example.edu,900120001\nduplicate@example.edu,900120002\nduplicate@example.edu,900120003\nnot-an-email,900120004\n";
    let preview = Request::builder()
        .method("POST")
        .uri(format!("/api/courses/{course}/roster-imports/preview"))
        .header("cookie", &instructor_cookie)
        .header("content-type", "text/csv; charset=utf-8")
        .header("if-match", "\"1\"")
        .header("idempotency-key", "bulk-preview-1200")
        .body(Body::from(csv))
        .expect("preview request");
    let preview = app
        .clone()
        .oneshot(preview)
        .await
        .expect("preview response");
    assert_eq!(preview.status(), StatusCode::OK);
    assert_eq!(preview.headers()["etag"], "\"1\"");
    let preview = json(preview).await;
    assert_eq!(
        preview["rows"]
            .as_array()
            .expect("preview rows")
            .iter()
            .map(|row| row["status"].as_str().expect("row status"))
            .collect::<Vec<_>>(),
        vec!["readyToInvite", "duplicate", "duplicate", "invalid"]
    );
    assert!(preview["rows"][3]["email"].is_null());
    assert!(preview["rows"][3]["rosterId"].is_null());
    assert!(!preview.to_string().contains("not-an-email"));

    let import_id = preview["importId"].as_str().expect("import ID");
    let commit = |rows: serde_json::Value| {
        let mut request = request(
            "POST",
            format!("/api/courses/{course}/roster-imports/{import_id}/commit"),
            &instructor_cookie,
            Some(serde_json::json!({ "rowNumbers": rows })),
        );
        request
            .headers_mut()
            .insert("if-match", "\"1\"".parse().expect("header"));
        request.headers_mut().insert(
            "idempotency-key",
            "bulk-commit-1200".parse().expect("header"),
        );
        request
    };
    let committed = app
        .clone()
        .oneshot(commit(serde_json::json!([2])))
        .await
        .expect("commit response");
    assert_eq!(committed.status(), StatusCode::OK);
    assert_eq!(committed.headers()["etag"], "\"2\"");
    let committed = json(committed).await;
    assert_eq!(committed["invitationsCreated"], 1);
    assert_eq!(committed["rosterRevision"], 2);
    assert!(committed.to_string().find("token").is_none());
    assert!(committed.to_string().find("email").is_none());

    let retry = app
        .clone()
        .oneshot(commit(serde_json::json!([2])))
        .await
        .expect("idempotent commit retry");
    assert_eq!(retry.status(), StatusCode::OK);
    assert_eq!(
        delivery
            .deliveries
            .lock()
            .expect("delivery fixture lock")
            .len(),
        2,
        "a safe retry may resend the same deterministic invitation"
    );

    let roster = app
        .oneshot(request(
            "GET",
            format!("/api/courses/{course}/roster"),
            &instructor_cookie,
            None,
        ))
        .await
        .expect("roster response");
    assert_eq!(roster.headers()["etag"], "\"2\"");
    let roster = json(roster).await;
    assert_eq!(roster["pendingInvitations"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn manual_grade_export_contains_only_course_roster_identity_and_selected_score() {
    let store = std::sync::Arc::new(learning_data_access::in_memory::MemoryStore::default());
    let tenant = TenantId::from_uuid(id(1_300));
    let instructor = UserId::from_uuid(id(1_301));
    let learner = UserId::from_uuid(id(1_302));
    let course = CourseId::from_uuid(id(1_303));
    let assignment = AssignmentId::from_uuid(id(1_304));
    let context = TenantContext::from_authenticated_session(tenant);
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Export course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("course fixture");
    let reference = publish_fixture(&store, context, tenant, instructor).await;
    store
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Manual export assignment".to_string(),
                items: vec![question_model::AssignmentItem {
                    id: question_model::AssignmentItemId::from_uuid(id(1_305)),
                    reference,
                    position: 0,
                    points_possible: question_model::PointValue::from_whole(1),
                    delivery_state: question_model::AssignmentDeliveryState::Active,
                    scoring_mode: question_model::AssignmentScoringMode::Normal,
                }],
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("assignment fixture");
    let instructor_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let delivery = std::sync::Arc::new(CapturingInvitationDelivery::default());
    let app = router_with_invitations(
        std::sync::Arc::clone(&store),
        CourseInvitationIssuer::from_server_secret([0x71; 32]),
        std::sync::Arc::clone(&delivery) as std::sync::Arc<dyn CourseInvitationDelivery>,
    );
    let mut invite = request(
        "POST",
        format!("/api/courses/{course}/invitations"),
        &instructor_cookie,
        Some(serde_json::json!({
            "email": "grade-export@example.edu",
            "rosterId": "900130001",
        })),
    );
    invite.headers_mut().insert(
        "idempotency-key",
        "grade-export-invite".parse().expect("header"),
    );
    assert_eq!(
        app.clone()
            .oneshot(invite)
            .await
            .expect("invitation response")
            .status(),
        StatusCode::ACCEPTED
    );
    let encoded_secret = delivery.deliveries.lock().expect("delivery lock")[0]
        .1
        .clone();
    let secret = URL_SAFE_NO_PAD
        .decode(encoded_secret)
        .expect("captured invitation secret");
    store
        .claim_course_invitation(ClaimCourseInvitation {
            token_hash: CourseInvitationSecretHash::compute(&secret),
            user: learner,
            verified_email: AuthenticationEmail::parse("grade-export@example.edu")
                .expect("verified email"),
            display_name: "Export Learner".to_string(),
        })
        .await
        .expect("claim invitation");

    let response = app
        .oneshot(request(
            "POST",
            format!("/api/courses/{course}/assignments/{assignment}/grade-export.csv"),
            &instructor_cookie,
            None,
        ))
        .await
        .expect("grade export response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        "text/csv; charset=utf-8"
    );
    assert!(response.headers().contains_key("x-ple-export-id"));
    assert_eq!(response.headers()["cache-control"], "no-store");
    let csv = String::from_utf8(
        to_bytes(response.into_body(), 64 * 1_024)
            .await
            .expect("bounded CSV")
            .to_vec(),
    )
    .expect("UTF-8 CSV");
    assert_eq!(
        csv,
        "roster_id,email,display_name,score\r\n900130001,grade-export@example.edu,Export Learner,\r\n"
    );
    assert!(!csv.contains(&learner.to_string()));
}
