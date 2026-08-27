use super::fixture_setup::{self, AssignmentFixture};
use axum::body::Body;
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderValue, Request, StatusCode};
use question_model::QuestionId;
use tower::ServiceExt;

fn assignment_request(question_id: &QuestionId) -> serde_json::Value {
    serde_json::json!({
        "title": "Peptide practice",
        "entries": [{
            "kind": "fixed",
            "questionId": question_id,
            "position": 0,
            "pointsPossible": "1",
            "deliveryState": "active",
            "scoringMode": "normal"
        }],
    })
}

async fn create_assignment(fixture: &AssignmentFixture) -> (String, String) {
    let draft = fixture
        .app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/courses/{}/assignments/drafts",
                fixture.course
            ))
            .header("cookie", &fixture.instructor_cookie)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Peptide practice"}"#))
            .expect("create draft request"),
        )
        .await
        .expect("create draft response");
    assert_eq!(draft.status(), StatusCode::CREATED);
    let draft_revision = draft
        .headers()
        .get(ETAG)
        .expect("draft revision")
        .to_str()
        .expect("ASCII draft revision")
        .to_owned();
    let bytes = axum::body::to_bytes(draft.into_body(), 128 * 1_024)
        .await
        .expect("draft response body");
    let assignment = serde_json::from_slice::<serde_json::Value>(&bytes)
        .expect("draft response JSON")["id"]
        .as_str()
        .expect("assignment ID")
        .to_owned();
    let response = fixture
        .app
        .clone()
        .oneshot(
            Request::put(format!(
                "/api/courses/{}/assignments/{assignment}/content",
                fixture.course
            ))
            .header("cookie", &fixture.instructor_cookie)
            .header(IF_MATCH, draft_revision)
            .header("content-type", "application/json")
            .body(Body::from(
                assignment_request(&fixture.question_id).to_string(),
            ))
            .expect("save assignment content request"),
        )
        .await
        .expect("save assignment content response");
    assert_eq!(response.status(), StatusCode::OK);
    let revision = response
        .headers()
        .get(ETAG)
        .expect("assignment revision")
        .to_str()
        .expect("ASCII assignment revision")
        .to_owned();
    (assignment, revision)
}

async fn create_second_course(fixture: &AssignmentFixture) -> String {
    let response = fixture
        .app
        .clone()
        .oneshot(
            Request::post("/api/courses")
                .header("cookie", &fixture.instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "BIOC 302: Protein structure",
                        "term": {
                            "startDate": "2026-08-24",
                            "endDate": "2026-12-18",
                            "timeZone": "America/Chicago"
                        }
                    })
                    .to_string(),
                ))
                .expect("second course request"),
        )
        .await
        .expect("second course response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(response.into_body(), 128 * 1_024)
        .await
        .expect("second course response body");
    serde_json::from_slice::<serde_json::Value>(&bytes).expect("second course response JSON")["id"]
        .as_str()
        .expect("second course ID")
        .to_owned()
}

async fn assert_refusal(response: axum::response::Response, expected: StatusCode) {
    assert_eq!(response.status(), expected);
    assert_eq!(
        response.headers().get("cache-control"),
        Some(&HeaderValue::from_static("no-store"))
    );
    let bytes = axum::body::to_bytes(response.into_body(), 128 * 1_024)
        .await
        .expect("refusal body");
    let body = String::from_utf8(bytes.to_vec()).expect("UTF-8 refusal body");
    assert!(
        !body.contains("complete valid assignment definition"),
        "unauthorized callers must not reach assignment JSON decoding: {body}"
    );
}

#[tokio::test]
async fn authoring_routes_authorize_before_decoding_malformed_bodies() {
    let fixture = fixture_setup::build().await;
    let (assignment, revision) = create_assignment(&fixture).await;
    let second_course = create_second_course(&fixture).await;
    let malformed = "not JSON";

    let create_cases = [
        (None, StatusCode::UNAUTHORIZED),
        (Some(fixture.student_cookie.as_str()), StatusCode::FORBIDDEN),
        (
            Some(fixture.outsider_cookie.as_str()),
            StatusCode::NOT_FOUND,
        ),
    ];
    for (cookie, expected) in create_cases {
        let mut request = Request::post(format!(
            "/api/courses/{}/assignments/drafts",
            fixture.course
        ))
        .header("content-type", "application/json");
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        assert_refusal(
            fixture
                .app
                .clone()
                .oneshot(
                    request
                        .body(Body::from(malformed))
                        .expect("malformed create request"),
                )
                .await
                .expect("malformed create response"),
            expected,
        )
        .await;
    }

    let authorized_create = fixture
        .app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/courses/{}/assignments/drafts",
                fixture.course
            ))
            .header("cookie", &fixture.instructor_cookie)
            .header("content-type", "application/json")
            .body(Body::from(malformed))
            .expect("authorized malformed create request"),
        )
        .await
        .expect("authorized malformed create response");
    assert_eq!(authorized_create.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(authorized_create.headers()["cache-control"], "no-store");

    let update_cases = [
        (None, StatusCode::UNAUTHORIZED),
        (Some(fixture.student_cookie.as_str()), StatusCode::FORBIDDEN),
        (
            Some(fixture.outsider_cookie.as_str()),
            StatusCode::NOT_FOUND,
        ),
    ];
    for (cookie, expected) in update_cases {
        let mut request = Request::put(format!(
            "/api/courses/{}/assignments/{assignment}/content",
            fixture.course
        ))
        .header(IF_MATCH, &revision)
        .header("content-type", "application/json");
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        assert_refusal(
            fixture
                .app
                .clone()
                .oneshot(
                    request
                        .body(Body::from(malformed))
                        .expect("malformed update request"),
                )
                .await
                .expect("malformed update response"),
            expected,
        )
        .await;
    }

    assert_refusal(
        fixture
            .app
            .clone()
            .oneshot(
                Request::put(format!(
                    "/api/courses/{second_course}/assignments/{assignment}/content"
                ))
                .header("cookie", &fixture.instructor_cookie)
                .header(IF_MATCH, &revision)
                .header("content-type", "application/json")
                .body(Body::from(malformed))
                .expect("cross-course malformed update request"),
            )
            .await
            .expect("cross-course malformed update response"),
        StatusCode::NOT_FOUND,
    )
    .await;

    let authorized_update = fixture
        .app
        .clone()
        .oneshot(
            Request::put(format!(
                "/api/courses/{}/assignments/{assignment}/content",
                fixture.course
            ))
            .header("cookie", &fixture.instructor_cookie)
            .header(IF_MATCH, revision)
            .header("content-type", "application/json")
            .body(Body::from(malformed))
            .expect("authorized malformed update request"),
        )
        .await
        .expect("authorized malformed update response");
    assert_eq!(authorized_update.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(authorized_update.headers()["cache-control"], "no-store");
}

#[tokio::test]
async fn over_limit_pool_definition_performs_no_catalog_resolution() {
    let fixture = fixture_setup::build().await;
    assert_eq!(fixture.store.catalog_resolution_calls(), 0);
    let candidates = (0..=1_024)
        .map(|index| format!("{index:03X}-0000"))
        .collect::<Vec<_>>();
    let draft = fixture
        .app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/courses/{}/assignments/drafts",
                fixture.course
            ))
            .header("cookie", &fixture.instructor_cookie)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Too many pool candidates"}"#))
            .expect("over-limit draft request"),
        )
        .await
        .expect("over-limit draft response");
    assert_eq!(draft.status(), StatusCode::CREATED);
    let revision = draft
        .headers()
        .get(ETAG)
        .expect("draft ETag")
        .to_str()
        .expect("ASCII draft ETag")
        .to_owned();
    let body = axum::body::to_bytes(draft.into_body(), 128 * 1_024)
        .await
        .expect("draft response body");
    let assignment = serde_json::from_slice::<serde_json::Value>(&body).expect("draft JSON")["id"]
        .as_str()
        .expect("draft ID")
        .to_owned();
    let response = fixture
        .app
        .clone()
        .oneshot(
            Request::put(format!(
                "/api/courses/{}/assignments/{assignment}/content",
                fixture.course
            ))
            .header("cookie", &fixture.instructor_cookie)
            .header(IF_MATCH, revision)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "title": "Too many pool candidates",
                    "entries": [{
                        "kind": "selectionGroup",
                        "candidateQuestionIds": candidates,
                        "position": 0,
                        "drawCount": 1,
                        "pointsPerItem": "1",
                        "ordering": "candidateOrder"
                    }]
                })
                .to_string(),
            ))
            .expect("over-limit content request"),
        )
        .await
        .expect("over-limit content response");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(fixture.store.catalog_resolution_calls(), 0);
}
