use super::super::fixtures::policies;
use super::super::*;
use super::fixture_setup::{build, create_assignment, request, response_json};
use tower::ServiceExt;

#[tokio::test]
async fn published_assignment_keeps_editor_private_and_learner_projection_safe() {
    let state = create_assignment(build().await).await;
    let fixture = &state.fixture;
    let path = format!(
        "/api/courses/{}/assignments/{}/policies",
        fixture.course, state.assignment
    );
    let published = fixture
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &path,
            &fixture.instructor_cookie,
            Some(&state.etag),
            Some(serde_json::json!({
                "audience": {"kind": "courseWide"},
                "disclosurePolicy": question_model::LearnerDisclosurePolicy::default(),
                "policies": policies(),
                "teachingSettings": {
                    "timeZone": "America/Chicago",
                    "lifecycle": "published",
                    "instructions": "Use complete sentences when explaining your reasoning.",
                    "availableAt": null,
                    "dueAt": null,
                    "closesAt": null,
                    "timeLimitSeconds": null,
                    "attemptLimit": null,
                    "lateSubmission": "accept",
                    "deadlineBehavior": "autoSubmit"
                }
            })),
        ))
        .await
        .expect("publish teaching settings response");
    assert_eq!(published.status(), StatusCode::OK);
    assert_eq!(
        published.headers().get("cache-control").expect("no-store"),
        "no-store"
    );

    let student_editor = fixture
        .app
        .clone()
        .oneshot(request(
            "GET",
            format!(
                "/api/courses/{}/assignments/{}",
                fixture.course, state.assignment
            ),
            &fixture.student_cookie,
            None,
            None,
        ))
        .await
        .expect("student editor response");
    assert_eq!(student_editor.status(), StatusCode::FORBIDDEN);

    let learner = fixture
        .app
        .clone()
        .oneshot(request(
            "GET",
            format!("/api/assignments/{}/learner", state.assignment),
            &fixture.student_cookie,
            None,
            None,
        ))
        .await
        .expect("learner projection response");
    assert_eq!(learner.status(), StatusCode::OK);
    let learner_json = response_json(learner).await;
    for forbidden in [
        "tenant",
        "courseId",
        "disclosurePolicy",
        "policies",
        "assignmentTiming",
    ] {
        assert!(
            learner_json.get(forbidden).is_none(),
            "learner projection leaked {forbidden}: {learner_json}"
        );
    }
}
