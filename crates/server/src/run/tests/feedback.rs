use super::*;
#[path = "feedback_support.rs"]
mod feedback_support;
use feedback_support::*;

#[tokio::test]
async fn non_current_scoring_redacts_every_learner_item_http_surface() {
    let (store, backend, app, student_cookie, _outsider_cookie, assignment) =
        native_feedback_fixture().await;
    let first = active_attempt_for(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        &student_cookie,
    )
    .await;
    let choice = presented_choice_id(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        first.id,
        &student_cookie,
        1,
    )
    .await;
    let submission_request = || {
        Request::builder()
            .method("POST")
            .uri(submission_path(
                CourseId::from_uuid(id(205)),
                assignment,
                first.id,
            ))
            .header("cookie", &student_cookie)
            .header("content-type", "application/json")
            .header("idempotency-key", "t1-scoring-redaction")
            .body(Body::from(
                serde_json::json!({
                    "response": { "kind": "multipleChoice", "selected": [choice] }
                })
                .to_string(),
            ))
            .expect("submission request")
    };
    let accepted = app
        .clone()
        .oneshot(submission_request())
        .await
        .expect("initial submission");
    assert_eq!(accepted.status(), StatusCode::ACCEPTED);
    drain_one_accepted_submission(&store, backend).await;

    for (expected_status, points) in [("recalculating", 3), ("failed", 4)] {
        set_assignment_item_points(store.as_ref(), assignment, points).await;
        if expected_status == "failed" {
            fail_assignment_scoring_job(store.as_ref(), assignment).await;
        }

        let receipt = json(
            app.clone()
                .oneshot(submission_request())
                .await
                .expect("idempotent receipt"),
        )
        .await;
        assert_redacted_item_surface(&receipt, expected_status, "receipt");

        let list = json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/runs/{}/attempts", first.run))
                        .header("cookie", &student_cookie)
                        .body(Body::empty())
                        .expect("attempt-list request"),
                )
                .await
                .expect("attempt-list response"),
        )
        .await;
        assert_redacted_item_surface(&list["items"][0], expected_status, "attempt list");

        let detail = json(
            app.clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/attempts/{}", first.id))
                        .header("cookie", &student_cookie)
                        .body(Body::empty())
                        .expect("attempt-detail request"),
                )
                .await
                .expect("attempt-detail response"),
        )
        .await;
        assert_redacted_item_surface(&detail, expected_status, "attempt detail");

        let summary = run_summary(&app, first.run, &student_cookie).await;
        assert_eq!(summary["summary"]["scoringStatus"], expected_status);
        assert!(summary["run"]["score"].is_null());
        assert!(summary["summary"]["currentScore"].is_null());
        let outcome = &summary["outcomes"]["items"][0];
        assert_eq!(outcome["scoringStatus"], expected_status);
        assert!(outcome["feedback"]["pointsEarned"].is_null());
        assert!(outcome["feedback"]["pointsPossible"].is_null());
    }
}
