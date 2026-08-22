use super::fixtures::{id, issued_cookie_for_tenant, publish_fixture};
use super::*;
use axum::body::{Body, to_bytes};
use axum::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, ETAG};
use axum::http::{Request, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AuthenticationEmail, CatalogStore, ClaimCourseInvitation, CourseInvitationSecretHash,
    CourseRecord, CourseRosterStore, CreateCourseCommand, Store, TenantContext,
};
use question_model::{CourseId, TenantId, UserId, UserRole};
use std::sync::Arc;
use tower::ServiceExt;

async fn fixture() -> (Arc<MemoryStore>, String, TenantId, UserId, CourseId) {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(9_100));
    let course = CourseId::from_uuid(id(9_101));
    let instructor = UserId::from_uuid(id(9_102));
    store
        .create_course(
            TenantContext::from_authenticated_session(tenant),
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
                initial_instructor: instructor,
            },
        )
        .await
        .expect("course");
    let cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Instructor], instructor).await;
    (store, cookie, tenant, instructor, course)
}

#[tokio::test]
async fn course_grade_scheme_uses_no_store_strong_etag_and_strict_put_body() {
    let (store, cookie, tenant, instructor, course) = fixture().await;
    let app = router(Arc::clone(&store));
    let initial = app
        .clone()
        .oneshot(
            Request::get(format!("/api/courses/{course}/grade-scheme"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("initial get"),
        )
        .await
        .expect("initial response");
    assert_eq!(initial.status(), StatusCode::OK);
    assert_eq!(initial.headers().get(ETAG).expect("initial etag"), "\"1\"");
    let reference = publish_fixture(
        &store,
        TenantContext::from_authenticated_session(tenant),
        tenant,
        instructor,
    )
    .await;
    let question_id = store
        .get_catalog_problem(TenantContext::from_authenticated_session(tenant), reference)
        .await
        .expect("catalog lookup")
        .expect("fixture publication")
        .question_id;
    let created = app
        .clone()
        .oneshot(
            Request::post(format!("/api/courses/{course}/assignments"))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "Read-only title",
                        "questionIds": [question_id],
                        "disclosurePolicy": question_model::LearnerDisclosurePolicy::default(),
                        "policies": super::fixtures::policies()
                    })
                    .to_string(),
                ))
                .expect("assignment create"),
        )
        .await
        .expect("assignment create response");
    assert_eq!(created.status(), StatusCode::CREATED);
    let get = app
        .clone()
        .oneshot(
            Request::get(format!("/api/courses/{course}/grade-scheme"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("get"),
        )
        .await
        .expect("response");
    assert_eq!(get.status(), StatusCode::OK);
    assert_eq!(
        get.headers().get(CACHE_CONTROL).expect("no store"),
        "no-store"
    );
    assert_eq!(get.headers().get(ETAG).expect("etag"), "\"2\"");
    let body = to_bytes(get.into_body(), 64 * 1024).await.expect("body");
    let view: serde_json::Value = serde_json::from_slice(&body).expect("view");
    assert!(view.get("tenant").is_none());
    assert!(view.get("course").is_none());
    assert!(view.get("revision").is_none());
    assert_eq!(view["assignments"][0]["title"], "Read-only title");

    let invalid = app
        .clone()
        .oneshot(
            Request::put(format!("/api/courses/{course}/grade-scheme"))
                .header("cookie", &cookie)
                .header("if-match", "\"2\"")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"scheme":{"mode":"totalPoints","rounding":"fourDecimalPlacesHalfAwayFromZero","categories":[],"letterBands":[]},"assignments":[{"assignment":"00000000-0000-0000-0000-000000000001","title":"no","included":true,"category":null,"position":null}]}"#))
                .expect("put"),
        )
        .await
        .expect("response");
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let assignment = view["assignments"][0]["assignment"].clone();
    let reordered = serde_json::json!({"assignments": [{"position": null, "category": null, "included": true, "assignment": assignment}], "scheme": {"letterBands": [], "categories": [], "rounding": "fourDecimalPlacesHalfAwayFromZero", "mode": "totalPoints"}});
    let saved = app
        .clone()
        .oneshot(
            Request::put(format!("/api/courses/{course}/grade-scheme"))
                .header("cookie", &cookie)
                .header("if-match", "\"2\"")
                .header("content-type", "application/json")
                .body(Body::from(format!(" \n {} \t", reordered)))
                .expect("put"),
        )
        .await
        .expect("response");
    assert_eq!(saved.status(), StatusCode::OK);
    assert_eq!(saved.headers().get(ETAG).expect("new etag"), "\"3\"");
}

#[tokio::test]
async fn course_grade_export_is_separate_noncacheable_csv_with_empty_body_gate() {
    let (store, cookie, _tenant, _instructor, course) = fixture().await;
    let app = router(Arc::clone(&store));
    let rejected = app
        .clone()
        .oneshot(
            Request::post(format!("/api/courses/{course}/grade-export.csv"))
                .header("cookie", &cookie)
                .body(Body::from("not empty"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let exported = app
        .oneshot(
            Request::post(format!("/api/courses/{course}/grade-export.csv"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let export_status = exported.status();
    let export_headers = exported.headers().clone();
    let bytes = to_bytes(exported.into_body(), 64 * 1024)
        .await
        .expect("csv body");
    assert_eq!(
        export_status,
        StatusCode::OK,
        "{}",
        std::str::from_utf8(&bytes).unwrap_or("non UTF-8 response")
    );
    assert_eq!(
        export_headers.get(CACHE_CONTROL).expect("no store"),
        "no-store"
    );
    assert!(export_headers.get("x-ple-course-grade-export-id").is_some());
    assert_eq!(
        export_headers
            .get(CONTENT_DISPOSITION)
            .expect("download disposition"),
        "attachment; filename=ple-course-grades.csv"
    );
    let text = std::str::from_utf8(&bytes).expect("UTF-8");
    let records = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(text.as_bytes())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("rectangular RFC4180 CSV");
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|record| record.len() == 9));
    assert_eq!(
        records[0].iter().collect::<Vec<_>>(),
        [
            "record_type",
            "aggregation_mode",
            "rounding_rule",
            "roster_id",
            "email",
            "display_name",
            "course_total",
            "letter",
            "unavailable_status",
        ]
    );
    assert_eq!(
        records[1].iter().collect::<Vec<_>>(),
        [
            "metadata",
            "totalPoints",
            "fourDecimalPlacesHalfAwayFromZero",
            "",
            "",
            "",
            "",
            "",
            "",
        ]
    );
}

#[tokio::test]
async fn students_and_nonmember_sysadmins_are_denied_every_course_grade_operation_before_body_parse()
 {
    let (store, instructor_cookie, tenant, instructor, course) = fixture().await;
    let student = UserId::from_uuid(id(9_103));
    let student_cookie = enrolled_student(
        &store,
        tenant,
        instructor,
        course,
        EnrolledStudent {
            user: student,
            email: "student-grade-security@example.edu",
            roster_id: "910010001",
            display_name: "Student Grade Security",
        },
    )
    .await;
    let sysadmin = UserId::from_uuid(id(9_104));
    let sysadmin_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Sysadmin], sysadmin).await;
    let app = crate::course::router_with_invitations(
        Arc::clone(&store),
        crate::course::CourseInvitationIssuer::from_server_secret([0x81; 32]),
    );
    let grade_scheme = format!("/api/courses/{course}/grade-scheme");
    let totals = format!("/api/courses/{course}/gradebook-totals");
    let export = format!("/api/courses/{course}/grade-export.csv");
    let denied_needles = [
        tenant.to_string(),
        course.to_string(),
        student.to_string(),
        "910010001".to_string(),
        "student-grade-security@example.edu".to_string(),
    ];

    for (cookie, expected) in [
        (&student_cookie, StatusCode::FORBIDDEN),
        (&sysadmin_cookie, StatusCode::NOT_FOUND),
    ] {
        for uri in [&grade_scheme, &totals] {
            let response = app
                .clone()
                .oneshot(
                    Request::get(uri)
                        .header("cookie", cookie)
                        .body(Body::empty())
                        .expect("denied read request"),
                )
                .await
                .expect("denied read response");
            assert_denied_grade_response(response, expected, &denied_needles).await;
        }

        let response = app
            .clone()
            .oneshot(
                Request::put(&grade_scheme)
                    .header("cookie", cookie)
                    .header("if-match", "\"1\"")
                    .header("content-type", "application/json")
                    .body(Body::from("[".repeat(64 * 1024 + 1)))
                    .expect("denied hostile scheme request"),
            )
            .await
            .expect("denied hostile scheme response");
        assert_denied_grade_response(response, expected, &denied_needles).await;

        let response = app
            .clone()
            .oneshot(
                Request::post(&export)
                    .header("cookie", cookie)
                    .body(Body::from("x".repeat(128)))
                    .expect("denied hostile export request"),
            )
            .await
            .expect("denied hostile export response");
        assert_denied_grade_response(response, expected, &denied_needles).await;
    }

    let unchanged = app
        .clone()
        .oneshot(
            Request::get(&grade_scheme)
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("instructor scheme request"),
        )
        .await
        .expect("instructor scheme response");
    assert_eq!(unchanged.status(), StatusCode::OK);
    assert_eq!(unchanged.headers()[ETAG], "\"1\"");

    let authorized = app
        .oneshot(
            Request::post(&export)
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("authorized export request"),
        )
        .await
        .expect("authorized export response");
    assert_eq!(authorized.status(), StatusCode::OK);
    assert!(
        authorized
            .headers()
            .contains_key("x-ple-course-grade-export-id")
    );
}

#[tokio::test]
async fn course_grade_export_emits_rectangular_inert_csv_rows_from_the_real_http_route() {
    let (store, instructor_cookie, tenant, instructor, course) = fixture().await;
    let app = crate::course::router_with_invitations(
        Arc::clone(&store),
        crate::course::CourseInvitationIssuer::from_server_secret([0x82; 32]),
    );
    let reference = publish_fixture(
        &store,
        TenantContext::from_authenticated_session(tenant),
        tenant,
        instructor,
    )
    .await;
    let question_id = store
        .get_catalog_problem(TenantContext::from_authenticated_session(tenant), reference)
        .await
        .expect("catalog lookup")
        .expect("fixture publication")
        .question_id;
    let created = app
        .clone()
        .oneshot(
            Request::post(format!("/api/courses/{course}/assignments"))
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "CSV proof assignment",
                        "questionIds": [question_id],
                        "disclosurePolicy": question_model::LearnerDisclosurePolicy::default(),
                        "policies": super::fixtures::policies()
                    })
                    .to_string(),
                ))
                .expect("assignment create"),
        )
        .await
        .expect("assignment create response");
    assert_eq!(created.status(), StatusCode::CREATED);

    let student = UserId::from_uuid(id(9_105));
    let display_name = "=SUM(1,1), \"quoted\"\nStudent";
    let student_cookie = enrolled_student(
        &store,
        tenant,
        instructor,
        course,
        EnrolledStudent {
            user: student,
            email: "ordinary.student@example.edu",
            roster_id: "910010002",
            display_name,
        },
    )
    .await;
    let scheme = app
        .clone()
        .oneshot(
            Request::put(format!("/api/courses/{course}/grade-scheme"))
                .header("cookie", &instructor_cookie)
                .header("if-match", "\"2\"")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "scheme": {
                            "mode": "totalPoints",
                            "rounding": "fourDecimalPlacesHalfAwayFromZero",
                            "categories": [],
                            "letterBands": [{"label": "=A", "minimumBasisPoints": 0}]
                        },
                        "assignments": [{
                            "assignment": created_assignment_id(created).await,
                            "included": true,
                            "category": null,
                            "position": null
                        }]
                    })
                    .to_string(),
                ))
                .expect("scheme save"),
        )
        .await
        .expect("scheme save response");
    assert_eq!(scheme.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::post(format!("/api/courses/{course}/grade-export.csv"))
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("course export request"),
        )
        .await
        .expect("course export response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        "text/csv; charset=utf-8"
    );
    assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
    let export_id = response.headers()["x-ple-course-grade-export-id"]
        .to_str()
        .expect("export ID is text")
        .to_string();
    assert!(uuid::Uuid::parse_str(&export_id).is_ok());
    let csv = String::from_utf8(
        to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("bounded export")
            .to_vec(),
    )
    .expect("UTF-8 CSV");
    assert!(csv.ends_with("\r\n"));
    assert!(!csv.contains("\n") || csv.contains("\r\n"));
    let records = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(csv.as_bytes())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("RFC4180 CSV");
    assert_eq!(records.len(), 3);
    assert!(records.iter().all(|record| record.len() == 9));
    assert_eq!(records[2].get(0), Some("student"));
    assert_eq!(records[2].get(3), Some("910010002"));
    assert_eq!(records[2].get(4), Some("ordinary.student@example.edu"));
    assert_eq!(records[2].get(5), Some(format!("'{display_name}").as_str()));
    assert_eq!(
        records[2]
            .get(6)
            .expect("course score")
            .parse::<f64>()
            .expect("numeric score"),
        0.0
    );
    assert_eq!(records[2].get(7), Some("'=A"));
    assert_eq!(records[2].get(8), Some(""));
    assert!(student_cookie.starts_with("__Host-ple_session="));
}

async fn created_assignment_id(response: axum::response::Response) -> String {
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("assignment response body");
    serde_json::from_slice::<serde_json::Value>(&body).expect("assignment response JSON")["id"]
        .as_str()
        .expect("assignment ID")
        .to_string()
}

struct EnrolledStudent<'a> {
    user: UserId,
    email: &'a str,
    roster_id: &'a str,
    display_name: &'a str,
}

async fn enrolled_student(
    store: &Arc<MemoryStore>,
    tenant: TenantId,
    instructor: UserId,
    course: CourseId,
    student: EnrolledStudent<'_>,
) -> String {
    let instructor_cookie =
        issued_cookie_for_tenant(store, tenant, vec![UserRole::Instructor], instructor).await;
    let app = crate::course::router_with_invitations(
        Arc::clone(store),
        crate::course::CourseInvitationIssuer::from_server_secret([0x82; 32]),
    );
    let response = app
        .oneshot(
            Request::post(format!("/api/courses/{course}/invitations"))
                .header("cookie", instructor_cookie)
                .header("content-type", "application/json")
                .header(
                    "idempotency-key",
                    format!("gradebook-student-{}", student.roster_id),
                )
                .body(Body::from(
                    serde_json::json!({"email": student.email, "rosterId": student.roster_id})
                        .to_string(),
                ))
                .expect("invitation request"),
        )
        .await
        .expect("invitation response");
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("invitation body");
    let value: serde_json::Value = serde_json::from_slice(&body).expect("invitation JSON");
    let encoded_secret = value["redemptionPath"]
        .as_str()
        .expect("redemption path")
        .strip_prefix("/course-invitations/redeem#token=")
        .expect("redemption secret");
    let secret = URL_SAFE_NO_PAD
        .decode(encoded_secret)
        .expect("secret encoding");
    store
        .claim_course_invitation(ClaimCourseInvitation {
            token_hash: CourseInvitationSecretHash::compute(&secret),
            user: student.user,
            verified_email: AuthenticationEmail::parse(student.email).expect("student email"),
            display_name: student.display_name.to_string(),
        })
        .await
        .expect("student enrollment");
    issued_cookie_for_tenant(store, tenant, vec![UserRole::Student], student.user).await
}

async fn assert_denied_grade_response(
    response: axum::response::Response,
    expected: StatusCode,
    sensitive_values: &[String],
) {
    assert_eq!(response.status(), expected);
    assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
    assert!(!response.headers().contains_key(ETAG));
    assert!(
        !response
            .headers()
            .contains_key("x-ple-course-grade-export-id")
    );
    assert!(!response.headers().contains_key(CONTENT_DISPOSITION));
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("bounded denied body");
    let text = std::str::from_utf8(&body).expect("denied body is UTF-8");
    for sensitive in sensitive_values {
        assert!(!text.contains(sensitive), "denied body leaked {sensitive}");
    }
}
