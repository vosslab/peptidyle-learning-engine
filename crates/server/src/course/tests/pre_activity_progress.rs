//! Read-only learner progress before the first educational receipt.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{Store, TenantContext};
use question_model::{AssignmentId, UserId};
use tower::ServiceExt;

pub(super) async fn assert_read_only_no_activity(
    app: &Router,
    store: &MemoryStore,
    context: TenantContext,
    student: UserId,
    assignment: AssignmentId,
    student_cookie: &str,
    outsider_cookie: &str,
) {
    assert!(
        store
            .learner_get_enrollment_for_assignment(context, student, assignment)
            .await
            .expect("pre-activity enrollment lookup")
            .is_none(),
        "publishing an assignment must not pre-materialize learner receipts"
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/assignments/{assignment}/summary"))
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("pre-activity summary request"),
        )
        .await
        .expect("pre-activity summary response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 128 * 1024)
        .await
        .expect("pre-activity summary body");
    let body: serde_json::Value = serde_json::from_slice(&body).expect("summary JSON");
    assert_eq!(
        body,
        serde_json::json!({
            "scoreState": "noActivity",
            "scoringStatus": "current",
            "currentScore": null,
            "bestScore": null,
            "latestScore": null,
            "completedRunCount": 0,
            "totalQuestionAttempts": 0,
            "lastActivityAt": null,
        })
    );
    assert!(
        store
            .learner_get_enrollment_for_assignment(context, student, assignment)
            .await
            .expect("post-summary enrollment lookup")
            .is_none(),
        "reading pre-activity progress must not create an enrollment"
    );
    for (label, assignment, cookie) in [
        ("denied", assignment, outsider_cookie),
        (
            "unknown",
            AssignmentId::from_uuid(uuid::Uuid::from_u128(u128::MAX)),
            student_cookie,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/assignments/{assignment}/summary"))
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .expect("concealed summary request"),
            )
            .await
            .expect("concealed summary response");
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{label} assignment summary must remain concealed"
        );
    }
}
