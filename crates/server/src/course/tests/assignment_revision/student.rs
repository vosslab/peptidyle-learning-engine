use super::super::fixtures::policies;
use super::super::*;
use super::fixture_setup::{build, create_assignment, request, response_json};
use tower::ServiceExt;

#[tokio::test]
async fn published_assignment_keeps_editor_private_and_student_projection_safe() {
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
                "disclosurePolicy": question_model::StudentDisclosurePolicy::default(),
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

    let student = fixture
        .app
        .clone()
        .oneshot(request(
            "GET",
            format!("/api/assignments/{}/student", state.assignment),
            &fixture.student_cookie,
            None,
            None,
        ))
        .await
        .expect("Student projection response");
    assert_eq!(student.status(), StatusCode::OK);
    let student_json = response_json(student).await;
    for forbidden in [
        "privateScope",
        "courseId",
        "disclosurePolicy",
        "policies",
        "assignmentTiming",
    ] {
        assert!(
            student_json.get(forbidden).is_none(),
            "Student projection leaked {forbidden}: {student_json}"
        );
    }
}
