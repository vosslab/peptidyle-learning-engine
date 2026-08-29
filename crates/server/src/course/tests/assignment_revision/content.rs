use super::super::*;
use super::fixture_setup::{build, create_assignment, request, response_json};
use axum::body::Body;
use axum::http::Request;
use axum::http::header::ETAG;
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn assignment_content_rejects_unavailable_question_and_exposes_qid() {
    let fixture = build().await;
    let draft = fixture
        .app
        .clone()
        .oneshot(request(
            "POST",
            format!("/api/courses/{}/assignments/drafts", fixture.course),
            &fixture.instructor_cookie,
            None,
            Some(serde_json::json!({"title": "Unavailable"})),
        ))
        .await
        .expect("unavailable draft response");
    assert_eq!(draft.status(), StatusCode::CREATED);
    let draft_etag = draft
        .headers()
        .get(ETAG)
        .expect("unavailable draft ETag")
        .to_str()
        .expect("unavailable draft ETag text")
        .to_owned();
    let draft = response_json(draft).await;
    let unavailable = fixture
        .app
        .clone()
        .oneshot(request(
            "PUT",
            format!(
                "/api/courses/{}/assignments/{}/content",
                fixture.course,
                draft["id"].as_str().expect("unavailable assignment ID")
            ),
            &fixture.instructor_cookie,
            Some(&draft_etag),
            Some(serde_json::json!({
                "title": "Unavailable",
                "entries": [{"kind": "fixed", "questionId": "000-0000", "position": 0, "pointsPossible": "1", "deliveryState": "active", "scoringMode": "normal"}]
            })),
        ))
        .await
        .expect("unavailable content response");
    assert_eq!(unavailable.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let state = create_assignment(fixture).await;
    assert_eq!(
        state.content["items"][0]["questionId"],
        state.fixture.question_id.to_string()
    );
    assert!(state.content["items"][0].get("reference").is_none());
    assert_eq!(
        state.content["disclosurePolicy"],
        serde_json::to_value(question_model::StudentDisclosurePolicy::default())
            .expect("default disclosure policy serializes")
    );
}

#[tokio::test]
async fn assignment_content_checks_revision_before_decoding_and_returns_no_store() {
    let state = create_assignment(build().await).await;
    let fixture = &state.fixture;
    let path = format!(
        "/api/courses/{}/assignments/{}/content",
        fixture.course, state.assignment
    );
    let missing = fixture
        .app
        .clone()
        .oneshot(
            Request::put(&path)
                .header("cookie", &fixture.instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from("not decoded without If-Match"))
                .expect("missing revision request"),
        )
        .await
        .expect("missing revision response");
    assert_eq!(missing.status(), StatusCode::PRECONDITION_REQUIRED);

    let malformed = fixture
        .app
        .clone()
        .oneshot(
            Request::put(&path)
                .header("cookie", &fixture.instructor_cookie)
                .header(IF_MATCH, "1")
                .header("content-type", "application/json")
                .body(Body::from("not decoded with malformed If-Match"))
                .expect("malformed revision request"),
        )
        .await
        .expect("malformed revision response");
    assert_eq!(malformed.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let stale = fixture
        .app
        .clone()
        .oneshot(
            Request::put(&path)
                .header("cookie", &fixture.instructor_cookie)
                .header(IF_MATCH, "\"999999\"")
                .body(Body::from("intentionally malformed body"))
                .expect("stale revision request"),
        )
        .await
        .expect("stale revision response");
    assert_eq!(stale.status(), StatusCode::PRECONDITION_FAILED);

    let entries = serde_json::json!([{
        "kind": "fixed",
        "questionId": state.content["items"][0]["questionId"],
        "position": state.content["items"][0]["position"],
        "pointsPossible": state.content["items"][0]["pointsPossible"],
        "deliveryState": state.content["items"][0]["deliveryState"],
        "scoringMode": state.content["items"][0]["scoringMode"]
    }]);
    let saved = fixture
        .app
        .clone()
        .oneshot(request(
            "PUT",
            path,
            &fixture.instructor_cookie,
            Some(&state.etag),
            Some(serde_json::json!({"title": "Peptide practice", "entries": entries})),
        ))
        .await
        .expect("content save response");
    assert_eq!(saved.status(), StatusCode::OK);
    assert_eq!(
        saved.headers().get("cache-control").expect("no-store"),
        "no-store"
    );
    assert_ne!(
        saved.headers().get(ETAG).expect("saved content ETag"),
        &state.etag
    );
    let saved_json: Value = response_json(saved).await;
    assert_eq!(
        saved_json["items"][0]["questionId"],
        fixture.question_id.to_string()
    );
}
