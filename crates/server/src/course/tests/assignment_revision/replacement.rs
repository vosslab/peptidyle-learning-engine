use super::super::*;
use super::fixture_setup::{build, create_assignment, request, response_json};
use axum::body::Body;
use axum::http::Request;
use question_model::CourseId;
use tower::ServiceExt;

fn replacement_path(state: &super::fixture_setup::AssignmentState) -> String {
    let item = state.content["items"][0]["id"]
        .as_str()
        .expect("assignment-owned fixed item ID");
    format!(
        "/api/courses/{}/assignments/{}/fixed-items/{item}",
        state.fixture.course, state.assignment
    )
}

#[tokio::test]
async fn fixed_item_replacement_checks_revision_before_decoding_and_closes_its_body() {
    let state = create_assignment(build().await).await;
    let fixture = &state.fixture;
    let path = replacement_path(&state);
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

    let unknown = fixture
        .app
        .clone()
        .oneshot(request(
            "PUT",
            path,
            &fixture.instructor_cookie,
            Some(&state.etag),
            Some(serde_json::json!({
                "questionId": fixture.question_id,
                "unexpected": true,
            })),
        ))
        .await
        .expect("unknown-field replacement response");
    assert_eq!(unknown.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn fixed_item_replacement_is_course_bound_and_returns_the_next_editor_revision() {
    let state = create_assignment(build().await).await;
    let fixture = &state.fixture;
    let item = state.content["items"][0]["id"]
        .as_str()
        .expect("assignment-owned fixed item ID");
    let wrong_course = CourseId::from_uuid(super::super::fixtures::id(8_299));
    let unavailable = fixture
        .app
        .clone()
        .oneshot(request(
            "PUT",
            format!(
                "/api/courses/{wrong_course}/assignments/{}/fixed-items/{item}",
                state.assignment
            ),
            &fixture.instructor_cookie,
            Some(&state.etag),
            Some(serde_json::json!({"questionId": fixture.question_id})),
        ))
        .await
        .expect("cross-course replacement response");
    assert_eq!(unavailable.status(), StatusCode::NOT_FOUND);

    let replaced = fixture
        .app
        .clone()
        .oneshot(request(
            "PUT",
            replacement_path(&state),
            &fixture.instructor_cookie,
            Some(&state.etag),
            Some(serde_json::json!({"questionId": fixture.replacement_question_id})),
        ))
        .await
        .expect("replacement response");
    assert_eq!(replaced.status(), StatusCode::OK);
    assert_eq!(
        replaced.headers().get("cache-control").expect("no-store"),
        "no-store"
    );
    let next_revision = replaced
        .headers()
        .get("etag")
        .expect("replacement ETag")
        .to_str()
        .expect("replacement ETag text");
    assert_ne!(next_revision, state.etag);
    let replacement = response_json(replaced).await;
    assert_eq!(replacement["id"], serde_json::json!(state.assignment));
    assert_eq!(replacement["items"][0]["id"], serde_json::json!(item));
    assert_eq!(
        replacement["items"][0]["questionId"],
        serde_json::json!(fixture.replacement_question_id)
    );
    assert_ne!(
        replacement["items"][0]["questionId"],
        serde_json::json!(fixture.question_id)
    );
}

#[tokio::test]
async fn fixed_item_replacement_requires_instructor_authority_and_a_current_revision() {
    let state = create_assignment(build().await).await;
    let fixture = &state.fixture;
    let path = replacement_path(&state);

    for (cookie, expected) in [
        (&fixture.student_cookie, StatusCode::FORBIDDEN),
        (&fixture.outsider_cookie, StatusCode::NOT_FOUND),
    ] {
        let denied = fixture
            .app
            .clone()
            .oneshot(request(
                "PUT",
                &path,
                cookie,
                Some(&state.etag),
                Some(serde_json::json!({"questionId": fixture.replacement_question_id})),
            ))
            .await
            .expect("replacement authority response");
        assert_eq!(denied.status(), expected);
    }

    let stale = fixture
        .app
        .clone()
        .oneshot(request(
            "PUT",
            path,
            &fixture.instructor_cookie,
            Some("\"999999\""),
            Some(serde_json::json!({"questionId": fixture.replacement_question_id})),
        ))
        .await
        .expect("stale replacement response");
    assert_eq!(stale.status(), StatusCode::PRECONDITION_FAILED);
}
