//! Native flat-question HTTP lifecycle through accepted automated grading.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use question_model::StudentResponse;
use question_model::response::ChoiceId;
use tower::ServiceExt;

use super::flat_run_support::{
    active_attempt_id, flat_run_fixture, rendered_choice_id, response_json, submission_json,
};

#[tokio::test]
async fn flat_run_route_retries_wrong_first_source_choice_then_completes_correct_second_choice() {
    let (app, store, backend, cookie, course, assignment) = flat_run_fixture().await;
    let start = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/runs"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("start run request"),
        )
        .await
        .expect("run starts");
    assert_eq!(
        start.status(),
        StatusCode::CREATED,
        "run route starts assigned work"
    );
    let run = response_json(start).await;
    let run_id = run
        .get("id")
        .and_then(serde_json::Value::as_str)
        .expect("run has a public id")
        .to_string();
    let first_attempt = active_attempt_id(&app, &run_id, &cookie).await;
    let first_wrong =
        rendered_choice_id(&app, course, assignment, &first_attempt, &cookie, "Red").await;
    assert_ne!(first_wrong, ChoiceId::new("red"));

    let wrong_submission = app
        .clone()
        .oneshot(submission_json(
            &format!(
                "/api/courses/{course}/assignments/{assignment}/attempts/{first_attempt}/submissions"
            ),
            &cookie,
            "flat-route-wrong-first",
            serde_json::json!({
                "response": StudentResponse::MultipleChoice {
                    selected: vec![first_wrong],
                }
            }),
        ))
        .await
        .expect("wrong first source choice submits");
    assert_eq!(
        wrong_submission.status(),
        StatusCode::ACCEPTED,
        "wrong source choice enters durable automated grading"
    );
    assert_eq!(
        response_json(wrong_submission).await,
        serde_json::json!({
            "kind": "accepted_pending",
            "accepted": true,
            "attemptId": first_attempt,
            "automatedGradingStatus": "pending",
            "nextAction": "check_status",
        })
    );
    crate::test_fixtures::drain_one_accepted_submission(&store, Arc::clone(&backend)).await;
    let wrong_status = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/attempts/{first_attempt}/submission-status"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("wrong grading status request"),
        )
        .await
        .expect("wrong grading status response");
    assert_eq!(wrong_status.status(), StatusCode::OK);
    let wrong_receipt = response_json(wrong_status).await;
    assert_eq!(
        wrong_receipt
            .pointer("/feedback/correctness")
            .and_then(serde_json::Value::as_bool),
        Some(false),
        "first source position remains incorrect while score recalculation is pending"
    );
    let resumed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/runs"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("resume after wrong grading request"),
        )
        .await
        .expect("resume after wrong grading response");
    assert_eq!(resumed.status(), StatusCode::CREATED);
    let second_attempt = active_attempt_id(&app, &run_id, &cookie).await;
    assert_ne!(
        second_attempt, first_attempt,
        "retry receives a distinct attempt"
    );
    assert_eq!(
        wrong_receipt["nextPending"], true,
        "wrong attempt records that server-owned successor delivery remains"
    );
    let second_correct =
        rendered_choice_id(&app, course, assignment, &second_attempt, &cookie, "Blue").await;
    assert_ne!(second_correct, ChoiceId::new("blue"));

    let correct_submission = app
        .clone()
        .oneshot(submission_json(
            &format!(
                "/api/courses/{course}/assignments/{assignment}/attempts/{second_attempt}/submissions"
            ),
            &cookie,
            "flat-route-correct-second",
            serde_json::json!({
                "response": StudentResponse::MultipleChoice {
                    selected: vec![second_correct],
                }
            }),
        ))
        .await
        .expect("correct second source choice submits");
    assert_eq!(
        correct_submission.status(),
        StatusCode::ACCEPTED,
        "correct source choice enters durable automated grading"
    );
    assert_eq!(
        response_json(correct_submission).await["kind"],
        "accepted_pending"
    );
    crate::test_fixtures::drain_one_accepted_submission(&store, Arc::clone(&backend)).await;
    let correct_status = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/attempts/{second_attempt}/submission-status"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("correct grading status request"),
        )
        .await
        .expect("correct grading status response");
    assert_eq!(correct_status.status(), StatusCode::OK);
    let correct_receipt = response_json(correct_status).await;
    assert_eq!(
        correct_receipt
            .pointer("/feedback/correctness")
            .and_then(serde_json::Value::as_bool),
        Some(true),
        "second source position remains correct while score recalculation is pending"
    );
    assert!(
        correct_receipt
            .get("nextIssued")
            .is_some_and(serde_json::Value::is_null),
        "completion does not issue a third assigned attempt"
    );
}
