//! Shared HTTP and authenticated-session helpers for run-route behavior tests.

use super::*;

pub(super) async fn issued_cookie(store: &MemoryStore, user: UserId, name: &str) -> String {
    issued_cookie_for(store, TenantId::from_uuid(id(1)), user, name).await
}

pub(super) async fn issued_cookie_for(
    store: &MemoryStore,
    tenant: TenantId,
    user: UserId,
    name: &str,
) -> String {
    let subject =
        SessionSubject::new(tenant, user, name, vec![UserRole::Student]).expect("session subject");
    let issued = crate::auth::issue_session(
        store,
        subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("session");
    issued
        .set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string()
}

pub(super) async fn json(response: Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), 256 * 1_024)
        .await
        .expect("response bytes");
    serde_json::from_slice(&bytes).expect("JSON response")
}

pub(super) fn post_json(path: &str, cookie: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("cookie", cookie)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

pub(super) async fn active_attempt_for(
    app: &Router,
    assignment: AssignmentId,
    cookie: &str,
) -> QuestionAttempt {
    let run_response = app
        .clone()
        .oneshot(post_json(
            "/api/runs",
            cookie,
            serde_json::json!({ "assignmentId": assignment }),
        ))
        .await
        .expect("start run response");
    assert_eq!(run_response.status(), StatusCode::CREATED);
    let run: AssignmentRun =
        serde_json::from_value(json(run_response).await).expect("run contract");
    let attempts_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{}/attempts", run.id))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("attempt request"),
        )
        .await
        .expect("attempt response");
    assert_eq!(attempts_response.status(), StatusCode::OK);
    let attempts: Page<QuestionAttempt> =
        serde_json::from_value(json(attempts_response).await).expect("attempt page");
    attempts.items.into_iter().next().expect("active attempt")
}

pub(super) async fn next_active_attempt(app: &Router, run: RunId, cookie: &str) -> QuestionAttempt {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{run}/attempts"))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("attempt list request"),
        )
        .await
        .expect("attempt list response");
    let attempts: Page<QuestionAttempt> =
        serde_json::from_value(json(response).await).expect("attempt page");
    attempts
        .items
        .into_iter()
        .find(|attempt| attempt.response.is_none())
        .expect("next active attempt")
}

/// Returns an ID from the answer-free schema actually issued to the browser.
/// Native durable IDs are deliberately not accepted by the submit route.
pub(super) async fn presented_choice_id(
    app: &Router,
    attempt: QuestionAttemptId,
    cookie: &str,
    position: usize,
) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/attempts/{attempt}/question"))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("issued question request"),
        )
        .await
        .expect("issued question response");
    assert_eq!(response.status(), StatusCode::OK);
    json(response).await["response"]["choices"]
        .as_array()
        .and_then(|choices| choices.get(position))
        .and_then(|choice| choice["id"].as_str())
        .expect("issued choice identifier")
        .to_string()
}
