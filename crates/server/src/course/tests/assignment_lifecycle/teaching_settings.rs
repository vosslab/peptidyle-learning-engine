use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{Request, StatusCode};
use question_model::{AssignmentId, CourseId};
use tower::ServiceExt;

pub(super) async fn publish_and_assert(
    app: &Router,
    course: CourseId,
    assignment: AssignmentId,
    instructor_cookie: &str,
    student_cookie: &str,
    etag: &str,
) -> String {
    let draft_student_detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/assignments/{assignment}/learner"))
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("draft learner detail request"),
        )
        .await
        .expect("draft learner detail response");
    assert_eq!(draft_student_detail.status(), StatusCode::NOT_FOUND);

    let published = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/teaching-settings"
                ))
                .header("cookie", instructor_cookie)
                .header(IF_MATCH, etag)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "timeZone": "America/Chicago",
                        "lifecycle": "published",
                        "instructions": "Show your structural reasoning in complete sentences.",
                        "availableAt": null,
                        "dueAt": null,
                        "closesAt": null,
                        "timeLimitSeconds": null,
                        "attemptLimit": 1,
                        "lateSubmission": "accept",
                        "deadlineBehavior": "autoSubmit"
                    })
                    .to_string(),
                ))
                .expect("published teaching settings request"),
        )
        .await
        .expect("published teaching settings response");
    assert_eq!(published.status(), StatusCode::OK);
    let new_etag = published
        .headers()
        .get(ETAG)
        .expect("published teaching settings ETag")
        .to_str()
        .expect("ASCII published teaching settings ETag")
        .to_string();
    let body = to_bytes(published.into_body(), 128 * 1024)
        .await
        .expect("published teaching settings body");
    let published: serde_json::Value =
        serde_json::from_slice(&body).expect("published teaching settings JSON");
    assert_eq!(published["teachingSettings"]["lifecycle"], "published");
    assert_eq!(
        published["teachingSettings"]["instructions"],
        "Show your structural reasoning in complete sentences."
    );
    assert!(published["teachingSettings"]["availableAt"].is_null());
    assert!(published["teachingSettings"]["dueAt"].is_null());
    assert!(published["teachingSettings"]["closesAt"].is_null());
    assert_eq!(published["teachingSettings"]["attemptLimit"], 1);
    assert_eq!(
        published["currentState"],
        serde_json::json!({ "state": "open" })
    );

    let student_detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/assignments/{assignment}/learner"))
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("published learner detail request"),
        )
        .await
        .expect("published learner detail response");
    assert_eq!(student_detail.status(), StatusCode::OK);
    let body = to_bytes(student_detail.into_body(), 128 * 1024)
        .await
        .expect("learner detail body");
    let student_detail: serde_json::Value =
        serde_json::from_slice(&body).expect("learner detail JSON");
    assert_eq!(
        student_detail["instructions"],
        "Show your structural reasoning in complete sentences."
    );
    assert_eq!(student_detail["timeZone"], "America/Chicago");
    assert_eq!(
        student_detail["delivery"],
        serde_json::json!({
            "availableAt": null,
            "dueAt": null,
            "closesAt": null,
            "timeLimitSeconds": null,
            "attemptLimit": 1,
            "lateSubmission": "accept",
            "deadlineBehavior": "autoSubmit",
            "lateStatus": "onTime"
        })
    );
    for forbidden in [
        "tenant",
        "courseId",
        "basePolicy",
        "policy",
        "provenance",
        "clock",
        "lifecycle",
        "teachingSettings",
    ] {
        assert!(
            student_detail.get(forbidden).is_none(),
            "learner detail leaked {forbidden}: {student_detail}"
        );
    }

    new_etag
}
