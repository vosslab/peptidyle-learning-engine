use super::fixture_setup;
use super::teaching_settings;
use super::*;
use axum::body::Body;
use axum::http::Request;
use axum::http::header::ETAG;
use axum::response::Response;
use learning_data_access::LearnerWorkRoutingBinding;
use question_model::{AssignmentId, RunId};
use tower::ServiceExt;

pub(super) async fn response_json(response: Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), 128 * 1024)
        .await
        .expect("response body");
    serde_json::from_slice(&bytes).expect("JSON response")
}

#[tokio::test]
async fn issued_learner_work_returns_a_typed_structural_content_recovery() {
    let fixture = fixture_setup::build().await;

    let draft = fixture
        .app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/courses/{}/assignments/drafts",
                fixture.course
            ))
            .header("cookie", &fixture.instructor_cookie)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Peptide bond mastery"}"#))
            .expect("assignment draft request"),
        )
        .await
        .expect("assignment draft response");
    assert_eq!(draft.status(), StatusCode::CREATED);
    let draft_revision = draft
        .headers()
        .get(ETAG)
        .expect("draft ETag")
        .to_str()
        .expect("ASCII draft ETag")
        .to_owned();
    let draft = response_json(draft).await;
    let assignment: AssignmentId =
        serde_json::from_value(draft["id"].clone()).expect("assignment ID response");

    let content = fixture
        .app
        .clone()
        .oneshot(
            Request::put(format!(
                "/api/courses/{}/assignments/{assignment}/content",
                fixture.course
            ))
            .header("cookie", &fixture.instructor_cookie)
            .header(IF_MATCH, draft_revision)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "title": "Peptide bond mastery",
                    "entries": [{
                        "kind": "fixed",
                        "questionId": fixture.question_id,
                        "position": 0,
                        "pointsPossible": "1",
                        "deliveryState": "active",
                        "scoringMode": "normal"
                    }]
                })
                .to_string(),
            ))
            .expect("assignment content request"),
        )
        .await
        .expect("assignment content response");
    assert_eq!(content.status(), StatusCode::OK);
    let revision = content
        .headers()
        .get(ETAG)
        .expect("content ETag")
        .to_str()
        .expect("ASCII content ETag")
        .to_owned();

    let revision = teaching_settings::publish_and_assert(
        &fixture.app,
        fixture.course,
        assignment,
        &fixture.instructor_cookie,
        &fixture.student_cookie,
        &revision,
    )
    .await;
    fixture
        .store
        .set_authoritative_time(question_model::ActivityTimestamp::from_unix_millis(
            1_787_677_200_000,
        ))
        .expect("in-term authoritative clock");
    let in_term_instructor_cookie = super::fixtures::issued_cookie_for_tenant(
        fixture.store.as_ref(),
        fixture.tenant,
        vec![question_model::UserRole::Instructor],
        fixture.instructor,
    )
    .await;
    fixture
        .store
        .start_or_resume_run(
            fixture.context,
            fixture.student,
            LearnerWorkRoutingBinding::new(fixture.course, assignment),
            RunId::generate(),
        )
        .await
        .expect("issued learner run");

    let structural = fixture
        .app
        .clone()
        .oneshot(
            Request::put(format!(
                "/api/courses/{}/assignments/{assignment}/content",
                fixture.course
            ))
            .header("cookie", &in_term_instructor_cookie)
            .header(IF_MATCH, &revision)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "title": "Peptide bond mastery",
                    "entries": [{
                        "kind": "selectionGroup",
                        "candidateQuestionIds": [fixture.question_id],
                        "position": 0,
                        "drawCount": 1,
                        "pointsPerItem": "1",
                        "ordering": "candidateOrder"
                    }]
                })
                .to_string(),
            ))
            .expect("structural content request"),
        )
        .await
        .expect("structural content response");
    assert_eq!(structural.status(), StatusCode::CONFLICT);
    assert_eq!(structural.headers()["cache-control"], "no-store");
    assert_eq!(
        response_json(structural).await,
        serde_json::json!({ "kind": "issuedLearnerWork" })
    );

    let title_only = fixture
        .app
        .clone()
        .oneshot(
            Request::put(format!(
                "/api/courses/{}/assignments/{assignment}/content",
                fixture.course
            ))
            .header("cookie", &in_term_instructor_cookie)
            .header(IF_MATCH, &revision)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "title": "Peptide bond mastery: revised instructions",
                    "entries": [{
                        "kind": "fixed",
                        "questionId": fixture.question_id,
                        "position": 0,
                        "pointsPossible": "1",
                        "deliveryState": "active",
                        "scoringMode": "normal"
                    }]
                })
                .to_string(),
            ))
            .expect("title-only content request"),
        )
        .await
        .expect("title-only content response");
    assert_eq!(title_only.status(), StatusCode::OK);
}

#[tokio::test]
async fn policy_publish_of_an_empty_draft_returns_readiness_without_advancing_revision() {
    let fixture = fixture_setup::build().await;
    let draft = fixture
        .app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/courses/{}/assignments/drafts",
                fixture.course
            ))
            .header("cookie", &fixture.instructor_cookie)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"title":"Peptide bond mastery"}"#))
            .expect("assignment draft request"),
        )
        .await
        .expect("assignment draft response");
    assert_eq!(draft.status(), StatusCode::CREATED);
    let revision = draft
        .headers()
        .get(ETAG)
        .expect("draft ETag")
        .to_str()
        .expect("ASCII draft ETag")
        .to_owned();
    let draft = response_json(draft).await;
    let assignment: AssignmentId =
        serde_json::from_value(draft["id"].clone()).expect("assignment ID response");

    let response = fixture
        .app
        .clone()
        .oneshot(
            Request::put(format!(
                "/api/courses/{}/assignments/{assignment}/policies",
                fixture.course
            ))
            .header("cookie", &fixture.instructor_cookie)
            .header(IF_MATCH, &revision)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "audience": {"kind": "anyOfGroups", "groups": []},
                    "disclosurePolicy": question_model::LearnerDisclosurePolicy::default(),
                    "policies": super::fixtures::policies(),
                    "teachingSettings": {
                        "timeZone": "America/Chicago",
                        "lifecycle": "published",
                        "instructions": "",
                        "availableAt": null,
                        "dueAt": null,
                        "closesAt": null,
                        "timeLimitSeconds": null,
                        "attemptLimit": null,
                        "lateSubmission": "accept",
                        "deadlineBehavior": "autoSubmit"
                    }
                })
                .to_string(),
            ))
            .expect("empty draft policy request"),
        )
        .await
        .expect("empty draft policy response");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(
        response_json(response).await,
        serde_json::json!({
            "error": "assignmentPoliciesInvalid",
            "issues": [
                {"kind": "audience", "reason": "groupRequired"},
                {
                    "kind": "publicationReadiness",
                    "blockingIssues": [{"kind": "questionsRequired"}]
                }
            ]
        })
    );

    let reread = fixture
        .app
        .oneshot(
            Request::get(format!(
                "/api/courses/{}/assignments/{assignment}",
                fixture.course
            ))
            .header("cookie", &fixture.instructor_cookie)
            .body(Body::empty())
            .expect("assignment reread request"),
        )
        .await
        .expect("assignment reread response");
    assert_eq!(reread.headers()[ETAG], revision);
}
