use super::fixtures::{
    id, issued_cookie_for_tenant, policies, publish_fixture, publish_fixture_with_identity,
};
use super::*;
use axum::body::Body;
use axum::http::Request;
use axum::http::header::ETAG;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    CatalogStore, CourseRecord, CourseRosterStore, CreateCourseCommand, Store, TenantContext,
    UpsertCourseMember,
};
use question_model::{CourseId, TenantId, UserId, UserRole};
use std::sync::Arc;
use tower::ServiceExt;

#[test]
fn assignment_revision_requires_one_positive_strong_etag() {
    let accepted = HeaderMap::from_iter([(IF_MATCH, HeaderValue::from_static("\"7\""))]);
    assert_eq!(
        required_assignment_revision(&accepted).expect("strong revision"),
        serde_json::from_str("7").expect("revision")
    );
    for value in ["7", "W/\"7\"", "\"0\"", "\"-1\"", "\"9223372036854775808\""] {
        let headers =
            HeaderMap::from_iter([(IF_MATCH, HeaderValue::from_str(value).expect("test header"))]);
        assert_eq!(
            required_assignment_revision(&headers),
            Err(AssignmentRevisionHeaderError::Malformed)
        );
    }
    assert_eq!(
        required_assignment_revision(&HeaderMap::new()),
        Err(AssignmentRevisionHeaderError::Missing)
    );
}

#[tokio::test]
async fn assignment_editor_uses_qids_and_focused_item_commands() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(id(8_200));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(8_201));
    let course = CourseId::from_uuid(id(8_202));
    let student = UserId::from_uuid(id(8_203));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Biochemistry".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("course save");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Biochemistry learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("student membership save");
    let reference = publish_fixture(&store, context, tenant, instructor).await;
    let replacement_reference =
        publish_fixture_with_identity(&store, context, tenant, instructor, 30).await;
    let question_id = store
        .get_catalog_problem(context, reference)
        .await
        .expect("catalog lookup")
        .expect("published fixture")
        .question_id;
    let replacement_question_id = store
        .get_catalog_problem(context, replacement_reference)
        .await
        .expect("replacement catalog lookup")
        .expect("replacement publication")
        .question_id;
    assert_ne!(question_id, replacement_question_id);
    let cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Instructor], instructor).await;
    let student_cookie =
        issued_cookie_for_tenant(&store, tenant, vec![UserRole::Student], student).await;
    let app = router(Arc::clone(&store));

    let unavailable = app
        .clone()
        .oneshot(
            Request::post(format!("/api/courses/{course}/assignments"))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "Unavailable", "questionIds": ["000-0000"], "policies": policies(),
                        "assignmentTiming": {"timeLimitSeconds": null}
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(unavailable.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let created = app
        .clone()
        .oneshot(
            Request::post(format!("/api/courses/{course}/assignments"))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({
                    "title": "Peptide practice", "questionIds": [question_id], "policies": policies(),
                    "disclosurePolicy": question_model::LearnerDisclosurePolicy::default(),
                    "assignmentTiming": {"timeLimitSeconds": null}
                }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(created.status(), StatusCode::CREATED);
    let mut etag = created
        .headers()
        .get(ETAG)
        .expect("ETag")
        .to_str()
        .expect("ETag text")
        .to_string();
    let created = axum::body::to_bytes(created.into_body(), 128 * 1024)
        .await
        .expect("body");
    let created: serde_json::Value = serde_json::from_slice(&created).expect("safe response");
    let assignment = created["id"].as_str().expect("assignment ID");
    let first_item = created["items"][0]["id"]
        .as_str()
        .expect("item ID")
        .to_string();
    assert_eq!(created["items"][0]["questionId"], question_id.to_string());
    assert!(created["items"][0].get("reference").is_none());
    assert_eq!(
        created["disclosurePolicy"],
        serde_json::to_value(question_model::LearnerDisclosurePolicy::default())
            .expect("default disclosure policy serializes")
    );

    let revised_policy = question_model::LearnerDisclosurePolicy {
        score: question_model::LearnerDisclosureTiming::AfterDue,
        per_item_correctness: question_model::LearnerDisclosureTiming::AfterDue,
        feedback_text: question_model::LearnerDisclosureTiming::AfterClose,
        solution: question_model::LearnerDisclosureTiming::AfterClose,
        class_statistics: question_model::LearnerDisclosureTiming::Never,
    };
    let update_items = serde_json::json!([{
        "id": created["items"][0]["id"],
        "questionId": created["items"][0]["questionId"],
        "position": created["items"][0]["position"],
        "pointsPossible": created["items"][0]["pointsPossible"],
        "deliveryState": created["items"][0]["deliveryState"],
        "scoringMode": created["items"][0]["scoringMode"],
    }]);
    let revised = app
        .clone()
        .oneshot(
            Request::put(format!("/api/courses/{course}/assignments/{assignment}"))
                .header("cookie", &cookie)
                .header(IF_MATCH, &etag)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "Peptide practice",
                        "items": update_items,
                        "disclosurePolicy": revised_policy,
                        "policies": policies(),
                        "assignmentTiming": {"timeLimitSeconds": null}
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(revised.status(), StatusCode::OK);
    etag = revised
        .headers()
        .get(ETAG)
        .expect("ETag")
        .to_str()
        .expect("ETag text")
        .to_string();
    let revised = axum::body::to_bytes(revised.into_body(), 128 * 1024)
        .await
        .expect("body");
    let revised: serde_json::Value = serde_json::from_slice(&revised).expect("safe response");
    assert_eq!(
        revised["disclosurePolicy"],
        serde_json::to_value(revised_policy).expect("revised disclosure policy serializes")
    );

    let reread = app
        .clone()
        .oneshot(
            Request::get(format!("/api/assignments/{assignment}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(reread.status(), StatusCode::OK);
    let reread = axum::body::to_bytes(reread.into_body(), 128 * 1024)
        .await
        .expect("body");
    let reread: serde_json::Value = serde_json::from_slice(&reread).expect("safe response");
    assert_eq!(
        reread["disclosurePolicy"],
        serde_json::to_value(revised_policy).expect("revised disclosure policy serializes")
    );

    let student_editor_read = app
        .clone()
        .oneshot(
            Request::get(format!("/api/assignments/{assignment}"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("student editor request"),
        )
        .await
        .expect("student editor response");
    assert_eq!(student_editor_read.status(), StatusCode::FORBIDDEN);

    let learner_read = app
        .clone()
        .oneshot(
            Request::get(format!("/api/assignments/{assignment}/learner"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("learner projection request"),
        )
        .await
        .expect("learner projection response");
    assert_eq!(learner_read.status(), StatusCode::OK);
    let learner_read = axum::body::to_bytes(learner_read.into_body(), 128 * 1024)
        .await
        .expect("learner projection body");
    let learner_read: serde_json::Value =
        serde_json::from_slice(&learner_read).expect("learner-safe response");
    for forbidden in [
        "tenant",
        "courseId",
        "disclosurePolicy",
        "policies",
        "assignmentTiming",
    ] {
        assert!(
            learner_read.get(forbidden).is_none(),
            "learner projection leaked {forbidden}: {learner_read}"
        );
    }

    let add = app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/courses/{course}/assignments/{assignment}/items"
            ))
            .header("cookie", &cookie)
            .header(IF_MATCH, &etag)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"questionId": question_id, "position": 1}).to_string(),
            ))
            .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(add.status(), StatusCode::OK);
    etag = add
        .headers()
        .get(ETAG)
        .expect("ETag")
        .to_str()
        .expect("ETag text")
        .to_string();
    let add = axum::body::to_bytes(add.into_body(), 128 * 1024)
        .await
        .expect("body");
    let add: serde_json::Value = serde_json::from_slice(&add).expect("safe response");
    let added_item = add["items"][1]["id"]
        .as_str()
        .expect("added item")
        .to_string();

    let replace = app
        .clone()
        .oneshot(
            Request::put(format!(
                "/api/courses/{course}/assignments/{assignment}/items/{first_item}/question"
            ))
            .header("cookie", &cookie)
            .header(IF_MATCH, &etag)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"questionId": replacement_question_id}).to_string(),
            ))
            .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(replace.status(), StatusCode::OK);
    etag = replace
        .headers()
        .get(ETAG)
        .expect("ETag")
        .to_str()
        .expect("ETag text")
        .to_string();
    let replace = axum::body::to_bytes(replace.into_body(), 128 * 1024)
        .await
        .expect("body");
    let replace: serde_json::Value = serde_json::from_slice(&replace).expect("safe response");
    assert_eq!(replace["items"][0]["id"], first_item);
    assert_eq!(
        replace["items"][0]["questionId"],
        replacement_question_id.to_string()
    );
    assert!(replace["items"][0].get("reference").is_none());
    assert!(replace["items"][0].get("problem").is_none());
    assert!(replace["items"][0].get("version").is_none());

    let nonempty_remove = app
        .clone()
        .oneshot(
            Request::delete(format!(
                "/api/courses/{course}/assignments/{assignment}/items/{added_item}"
            ))
            .header("cookie", &cookie)
            .header(IF_MATCH, &etag)
            .body(Body::from("unexpected"))
            .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(nonempty_remove.status(), StatusCode::BAD_REQUEST);

    let removed = app
        .clone()
        .oneshot(
            Request::delete(format!(
                "/api/courses/{course}/assignments/{assignment}/items/{added_item}"
            ))
            .header("cookie", &cookie)
            .header(IF_MATCH, &etag)
            .body(Body::empty())
            .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(removed.status(), StatusCode::OK);
    let stale = app
        .oneshot(
            Request::post(format!(
                "/api/courses/{course}/assignments/{assignment}/items"
            ))
            .header("cookie", cookie)
            .header(IF_MATCH, etag)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"questionId": question_id, "position": 1}).to_string(),
            ))
            .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(stale.status(), StatusCode::CONFLICT);
}
