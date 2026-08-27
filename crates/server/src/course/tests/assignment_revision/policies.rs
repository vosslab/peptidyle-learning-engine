use super::super::fixtures::policies;
use super::super::*;
use super::fixture_setup::{AssignmentState, build, create_assignment, request, response_json};
use axum::body::Body;
use axum::http::Request;
use axum::http::header::ETAG;
use learning_data_access::TeachingAuthorityReferenceStore;
use question_model::CourseId;
use serde_json::Value;
use tower::ServiceExt;

fn teaching_settings(overrides: Value) -> Value {
    let defaults = serde_json::json!({
        "timeZone": "America/Chicago",
        "lifecycle": "draft",
        "instructions": "",
        "availableAt": null,
        "dueAt": null,
        "closesAt": null,
        "timeLimitSeconds": null,
        "attemptLimit": null,
        "lateSubmission": "accept",
        "deadlineBehavior": "autoSubmit"
    });
    let mut object = defaults.as_object().expect("settings object").clone();
    object.extend(overrides.as_object().expect("settings overrides").clone());
    Value::Object(object)
}

fn policy_body(audience: Value, disclosure: Value, settings: Value) -> Value {
    serde_json::json!({
        "audience": audience,
        "disclosurePolicy": disclosure,
        "policies": policies(),
        "teachingSettings": settings,
    })
}

async fn create_groups(state: &AssignmentState) -> Vec<String> {
    let fixture = &state.fixture;
    let membership = fixture
        .store
        .get_current_course_membership(fixture.context, fixture.course, fixture.student)
        .await
        .expect("student membership read")
        .expect("student membership exists");
    let member_reference = fixture
        .store
        .course_membership_reference(
            fixture.context,
            fixture.instructor,
            fixture.course,
            membership.id,
        )
        .await
        .expect("student public reference")
        .expect("student public reference exists")
        .to_string();
    let mut references = Vec::new();
    for title in ["Section A", "Section B"] {
        let response = fixture
            .app
            .clone()
            .oneshot(request(
                "POST",
                format!("/api/courses/{}/groups", fixture.course),
                &fixture.instructor_cookie,
                None,
                Some(serde_json::json!({
                    "title": title,
                    "purpose": "section",
                    "members": [member_reference],
                })),
            ))
            .await
            .expect("course group response");
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response_json(response).await;
        references.push(
            body["reference"]
                .as_str()
                .expect("group reference")
                .to_string(),
        );
    }
    references
}

#[tokio::test]
async fn assignment_policies_persist_public_audience_and_revisioned_disclosure() {
    let state = create_assignment(build().await).await;
    let fixture = &state.fixture;
    let mut groups = create_groups(&state).await;
    let mut expected_groups = groups.clone();
    expected_groups.sort_unstable();
    groups.reverse();
    let path = format!(
        "/api/courses/{}/assignments/{}/policies",
        fixture.course, state.assignment
    );
    let saved = fixture
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &path,
            &fixture.instructor_cookie,
            Some(&state.etag),
            Some(policy_body(
                serde_json::json!({"kind": "anyOfGroups", "groups": groups}),
                serde_json::to_value(question_model::LearnerDisclosurePolicy::default())
                    .expect("default disclosure policy"),
                teaching_settings(serde_json::json!({})),
            )),
        ))
        .await
        .expect("policies save response");
    assert_eq!(saved.status(), StatusCode::OK);
    assert_eq!(
        saved.headers().get("cache-control").expect("no-store"),
        "no-store"
    );
    let etag = saved
        .headers()
        .get(ETAG)
        .expect("policies ETag")
        .to_str()
        .expect("policies ETag text")
        .to_owned();
    let saved_json = response_json(saved).await;
    assert_eq!(
        saved_json["audience"]["groups"],
        serde_json::json!(expected_groups)
    );

    let malformed = fixture
        .app
        .clone()
        .oneshot(
            Request::put(&path)
                .header("cookie", &fixture.instructor_cookie)
                .header(IF_MATCH, "not-a-strong-etag")
                .body(Body::from("not decoded with malformed If-Match"))
                .expect("malformed policies request"),
        )
        .await
        .expect("malformed policies response");
    assert_eq!(malformed.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let reread = fixture
        .app
        .clone()
        .oneshot(request(
            "GET",
            format!(
                "/api/courses/{}/assignments/{}",
                fixture.course, state.assignment
            ),
            &fixture.instructor_cookie,
            None,
            None,
        ))
        .await
        .expect("policies reread response");
    assert_eq!(reread.headers().get(ETAG).expect("preserved ETag"), &etag);
    let reread_json = response_json(reread).await;
    assert_eq!(
        reread_json["audience"]["groups"],
        serde_json::json!(expected_groups)
    );

    let wrong_course = CourseId::from_uuid(super::super::fixtures::id(8_299));
    let wrong_course_read = fixture
        .app
        .clone()
        .oneshot(request(
            "GET",
            format!(
                "/api/courses/{wrong_course}/assignments/{}",
                state.assignment
            ),
            &fixture.instructor_cookie,
            None,
            None,
        ))
        .await
        .expect("wrong-course workspace response");
    assert_eq!(wrong_course_read.status(), StatusCode::NOT_FOUND);

    let revised_policy = question_model::LearnerDisclosurePolicy {
        score: question_model::LearnerDisclosureTiming::AfterDue,
        per_item_correctness: question_model::LearnerDisclosureTiming::AfterDue,
        feedback_text: question_model::LearnerDisclosureTiming::AfterClose,
        solution: question_model::LearnerDisclosureTiming::AfterClose,
        class_statistics: question_model::LearnerDisclosureTiming::Never,
    };
    let revised = fixture
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &path,
            &fixture.instructor_cookie,
            Some(&etag),
            Some(policy_body(
                serde_json::json!({"kind": "anyOfGroups", "groups": expected_groups}),
                serde_json::to_value(revised_policy).expect("revised disclosure policy"),
                teaching_settings(serde_json::json!({})),
            )),
        ))
        .await
        .expect("revised policies response");
    assert_eq!(revised.status(), StatusCode::OK);
    let revised_etag = revised
        .headers()
        .get(ETAG)
        .expect("revised policies ETag")
        .to_str()
        .expect("revised policies ETag text")
        .to_owned();
    assert_ne!(revised_etag, etag);
    let revised_json = response_json(revised).await;
    assert_eq!(
        revised_json["disclosurePolicy"],
        serde_json::to_value(revised_policy).expect("revised disclosure policy")
    );
    let reread = fixture
        .app
        .clone()
        .oneshot(request(
            "GET",
            format!(
                "/api/courses/{}/assignments/{}",
                fixture.course, state.assignment
            ),
            &fixture.instructor_cookie,
            None,
            None,
        ))
        .await
        .expect("revised assignment reread");
    assert_eq!(reread.status(), StatusCode::OK);
    assert_eq!(
        reread.headers().get(ETAG).expect("revised ETag"),
        &revised_etag
    );
    assert_eq!(
        response_json(reread).await["disclosurePolicy"],
        serde_json::to_value(revised_policy).expect("revised disclosure policy")
    );
}

#[tokio::test]
async fn assignment_policies_reject_invalid_schedule_and_unknown_input_without_mutation() {
    let state = create_assignment(build().await).await;
    let fixture = &state.fixture;
    let path = format!(
        "/api/courses/{}/assignments/{}/policies",
        fixture.course, state.assignment
    );
    let default_policy = |settings| {
        policy_body(
            serde_json::json!({"kind": "courseWide"}),
            serde_json::to_value(question_model::LearnerDisclosurePolicy::default())
                .expect("default disclosure policy"),
            teaching_settings(settings),
        )
    };
    let dst_due = fixture
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &path,
            &fixture.instructor_cookie,
            Some(&state.etag),
            Some(default_policy(serde_json::json!({
                "dueAt": "2026-11-01T01:30:00.000"
            }))),
        ))
        .await
        .expect("DST settings response");
    assert_eq!(dst_due.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        dst_due.headers().get("cache-control").expect("no-store"),
        "no-store"
    );
    let dst_due_json = response_json(dst_due).await;
    assert_eq!(
        dst_due_json,
        serde_json::json!({
            "error": "assignmentPoliciesInvalid",
            "issues": [{"kind": "teachingSettings", "correction": {
                "error": "assignmentTeachingSettingsInvalid", "field": "dueAt",
                "reason": "ambiguousLocalTime",
                "message": "Choose a local time outside the daylight-saving repeat hour."
            }}]
        })
    );

    let schedule_order = fixture
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &path,
            &fixture.instructor_cookie,
            Some(&state.etag),
            Some(default_policy(serde_json::json!({
                "availableAt": "2026-09-02T10:00:00.000",
                "dueAt": "2026-09-01T10:00:00.000"
            }))),
        ))
        .await
        .expect("out-of-order settings response");
    assert_eq!(schedule_order.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response_json(schedule_order).await["issues"][0]["correction"]["field"],
        "schedule"
    );

    let unknown = fixture
        .app
        .clone()
        .oneshot(request(
            "PUT",
            &path,
            &fixture.instructor_cookie,
            Some(&state.etag),
            Some(serde_json::json!({"unexpected": true})),
        ))
        .await
        .expect("unknown settings response");
    assert_eq!(unknown.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response_json(unknown).await,
        serde_json::json!({"error": "Use the Policies workspace to send complete valid settings."})
    );

    let stale = fixture
        .app
        .clone()
        .oneshot(
            Request::put(&path)
                .header("cookie", &fixture.instructor_cookie)
                .header(IF_MATCH, "\"999999\"")
                .body(Body::from("not json"))
                .expect("stale policies request"),
        )
        .await
        .expect("stale policies response");
    assert_eq!(stale.status(), StatusCode::PRECONDITION_FAILED);

    let reread = fixture
        .app
        .clone()
        .oneshot(request(
            "GET",
            format!(
                "/api/courses/{}/assignments/{}",
                fixture.course, state.assignment
            ),
            &fixture.instructor_cookie,
            None,
            None,
        ))
        .await
        .expect("unchanged assignment reread");
    assert_eq!(
        reread.headers().get(ETAG).expect("unchanged ETag"),
        &state.etag
    );
}

#[tokio::test]
async fn assignment_policy_updates_require_instructor_course_authority() {
    let state = create_assignment(build().await).await;
    let fixture = &state.fixture;
    let path = format!(
        "/api/courses/{}/assignments/{}/policies",
        fixture.course, state.assignment
    );
    for (cookie, expected) in [
        (&fixture.student_cookie, StatusCode::FORBIDDEN),
        (&fixture.outsider_cookie, StatusCode::NOT_FOUND),
    ] {
        let denied = fixture
            .app
            .clone()
            .oneshot(
                Request::put(&path)
                    .header("cookie", cookie)
                    .header("content-type", "application/json")
                    .body(Body::from("not json"))
                    .expect("unauthorized policies request"),
            )
            .await
            .expect("unauthorized policies response");
        assert_eq!(denied.status(), expected);
    }
}
