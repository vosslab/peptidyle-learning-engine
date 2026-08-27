use super::fixtures::id;
use super::*;
use axum::body::Body;
use axum::http::Request;
use axum::http::header::ETAG;
use learning_data_access::{
    CourseRecord, CourseRosterStore, CreateCourseCommand, JobLeaseDuration, JobPayload, JobStore,
    RetentionWorkerCommand, RetentionWorkerStore, UpsertCourseMember,
};
use question_model::{AssignmentId, CourseId, ObjectId};
use tower::ServiceExt;

mod authoring_boundary;
mod fixture_setup;
mod teaching_settings;
mod workspace_validation;

use workspace_validation::response_json;

#[tokio::test]
async fn membership_scopes_courses_and_exact_assignment_references_survive() {
    let fixture_setup::AssignmentFixture {
        store,
        tenant,
        context,
        instructor,
        student,
        instructor_cookie,
        student_cookie,
        outsider_cookie,
        sysadmin_cookie,
        foreign_cookie,
        app,
        course,
        question_id,
    } = fixture_setup::build().await;

    let created_draft = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/courses/{course}/assignments/drafts"))
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"Peptide bond mastery"}"#))
                .expect("draft request"),
        )
        .await
        .expect("draft response");
    assert_eq!(created_draft.status(), StatusCode::CREATED);
    let draft_etag = created_draft
        .headers()
        .get(ETAG)
        .expect("created draft ETag")
        .to_str()
        .expect("ASCII ETag")
        .to_string();
    let draft = response_json(created_draft).await;
    let assignment: AssignmentId =
        serde_json::from_value(draft["id"].clone()).expect("assignment ID response");
    let entries = serde_json::json!([{
        "kind": "fixed",
        "questionId": question_id,
        "position": 0,
        "pointsPossible": "1",
        "deliveryState": "active",
        "scoringMode": "normal"
    }]);
    let created_assignment = app
        .clone()
        .oneshot(
            Request::put(format!(
                "/api/courses/{course}/assignments/{assignment}/content"
            ))
            .header("cookie", &instructor_cookie)
            .header(IF_MATCH, &draft_etag)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "title": "Peptide bond mastery", "entries": entries,
                })
                .to_string(),
            ))
            .expect("assignment content request"),
        )
        .await
        .expect("assignment content response");
    assert_eq!(created_assignment.status(), StatusCode::OK);
    let assignment_etag = created_assignment
        .headers()
        .get(ETAG)
        .expect("assignment content ETag")
        .to_str()
        .expect("ASCII ETag")
        .to_string();
    let created_assignment = response_json(created_assignment).await;
    assert_eq!(created_assignment["courseId"], serde_json::json!(course));
    assert_eq!(
        created_assignment["items"][0]["questionId"],
        serde_json::json!(question_id),
        "the editor projects the selected stable Question ID"
    );
    assert!(created_assignment["items"][0].get("reference").is_none());
    assert!(created_assignment["items"][0]["id"].is_string());
    assert_eq!(created_assignment["teachingSettings"]["lifecycle"], "draft");
    assert_eq!(created_assignment["teachingSettings"]["instructions"], "");
    assert_eq!(
        created_assignment["currentState"],
        serde_json::json!({ "state": "draft" })
    );
    let update_entries = serde_json::json!([{
        "kind": "fixed",
        "questionId": created_assignment["items"][0]["questionId"],
        "position": created_assignment["items"][0]["position"],
        "pointsPossible": created_assignment["items"][0]["pointsPossible"],
        "deliveryState": created_assignment["items"][0]["deliveryState"],
        "scoringMode": created_assignment["items"][0]["scoringMode"],
    }]);

    let incomplete_content_update = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/content"
                ))
                .header("cookie", &instructor_cookie)
                .header(IF_MATCH, &assignment_etag)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "missing required content",
                    })
                    .to_string(),
                ))
                .expect("incomplete content update request"),
        )
        .await
        .expect("incomplete content update response");
    assert_eq!(
        incomplete_content_update.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "instructor content updates must state every content field"
    );

    for request in [
        Request::builder()
            .uri(format!("/api/courses/{course}/assignments/{assignment}"))
            .header("cookie", &foreign_cookie)
            .body(Body::empty())
            .expect("foreign exact request"),
        Request::builder()
            .method("PUT")
            .uri(format!(
                "/api/courses/{course}/assignments/{assignment}/content"
            ))
            .header("cookie", &foreign_cookie)
            .header(IF_MATCH, &assignment_etag)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "title": "foreign", "entries": update_entries.clone(),
                })
                .to_string(),
            ))
            .expect("foreign update request"),
        Request::builder()
            .method("PUT")
            .uri(format!(
                "/api/courses/{course}/assignments/{assignment}/content"
            ))
            .header("cookie", &foreign_cookie)
            .header(IF_MATCH, "W/\"1\"")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "title": "foreign malformed", "entries": update_entries.clone(),
                })
                .to_string(),
            ))
            .expect("foreign malformed update request"),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request)
                .await
                .expect("foreign response")
                .status(),
            StatusCode::NOT_FOUND,
            "foreign tenant must not enumerate an assignment"
        );
    }

    let nested_unknown = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/content"
                ))
                .header("cookie", &instructor_cookie)
                .header(IF_MATCH, &assignment_etag)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "Peptide bond mastery",
                        "entries": update_entries.clone(),
                        "unexpected": true,
                    })
                    .to_string(),
                ))
                .expect("nested unknown request"),
        )
        .await
        .expect("nested unknown response");
    assert_eq!(nested_unknown.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let updated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/content"
                ))
                .header("cookie", &instructor_cookie)
                .header(IF_MATCH, &assignment_etag)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "Peptide bond mastery revised",
                        "entries": update_entries.clone(),
                    })
                    .to_string(),
                ))
                .expect("assignment update request"),
        )
        .await
        .expect("assignment update response");
    assert_eq!(updated.status(), StatusCode::OK);
    let mut updated_etag = updated
        .headers()
        .get(ETAG)
        .expect("updated ETag")
        .to_str()
        .expect("ASCII ETag")
        .to_string();
    assert_ne!(updated_etag, assignment_etag);
    let updated = response_json(updated).await;
    assert_eq!(updated["teachingSettings"]["lifecycle"], "draft");

    let editor_get = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("editor GET request"),
        )
        .await
        .expect("editor GET response");
    assert_eq!(editor_get.status(), StatusCode::OK);
    assert_eq!(
        editor_get.headers().get("cache-control"),
        Some(&HeaderValue::from_static("no-store")),
        "workspace reads are sensitive instructor state"
    );
    assert_eq!(
        editor_get
            .headers()
            .get(ETAG)
            .expect("GET ETag")
            .to_str()
            .expect("ASCII GET ETag"),
        updated_etag
    );
    let editor_get = response_json(editor_get).await;
    assert_eq!(editor_get["teachingSettings"]["lifecycle"], "draft");

    updated_etag = teaching_settings::publish_and_assert(
        &app,
        course,
        assignment,
        &instructor_cookie,
        &student_cookie,
        &updated_etag,
    )
    .await;

    super::pre_activity_progress::assert_read_only_no_activity(
        &app,
        store.as_ref(),
        context,
        student,
        assignment,
        &student_cookie,
        &outsider_cookie,
    )
    .await;

    let stale = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/content"
                ))
                .header("cookie", &instructor_cookie)
                .header(IF_MATCH, &assignment_etag)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                    "title": "stale overwrite", "entries": update_entries.clone(),
                    })
                    .to_string(),
                ))
                .expect("stale update request"),
        )
        .await
        .expect("stale update response");
    assert_eq!(stale.status(), StatusCode::PRECONDITION_FAILED);
    assert_eq!(
        store
            .get_assignment(context, assignment)
            .await
            .expect("stored assignment")
            .expect("assignment")
            .title,
        "Peptide bond mastery revised"
    );

    let sysadmin_get = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                .header("cookie", &sysadmin_cookie)
                .body(Body::empty())
                .expect("sysadmin assignment request"),
        )
        .await
        .expect("sysadmin assignment response");
    assert_eq!(
        sysadmin_get.status(),
        StatusCode::NOT_FOUND,
        "sysadmin status alone must not disclose FERPA course definitions"
    );

    let wrong_course = CourseId::from_uuid(id(99));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: wrong_course,
                    tenant,
                    title: "BIOC 399: Wrong course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: crate::test_fixtures::sysadmin_course_creation_authority(
                    store.as_ref(),
                    tenant,
                    wrong_course,
                    instructor,
                )
                .await,
            },
        )
        .await
        .expect("wrong-course fixture");
    let wrong_course_update = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/courses/{wrong_course}/assignments/{assignment}/content"
                ))
                .header("cookie", &instructor_cookie)
                .header(IF_MATCH, updated_etag)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "must not move course", "entries": update_entries.clone(),
                    })
                    .to_string(),
                ))
                .expect("wrong-course update request"),
        )
        .await
        .expect("wrong-course update response");
    assert_eq!(wrong_course_update.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        store
            .get_assignment(context, assignment)
            .await
            .expect("stored assignment")
            .expect("assignment")
            .course_id,
        course
    );

    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Biochemistry Learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("gradebook fixture roster member");

    let gradebook = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/gradebook"))
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("gradebook request"),
        )
        .await
        .expect("gradebook response");
    assert_eq!(gradebook.status(), StatusCode::OK);
    assert_eq!(
        gradebook
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
    let gradebook = response_json(gradebook).await;
    let rows = gradebook["items"].as_array().expect("gradebook rows");
    assert!(
        rows.is_empty(),
        "a published assignment has no gradebook row until learner activity creates a summary"
    );

    let sysadmin_gradebook = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/gradebook"))
                .header("cookie", &sysadmin_cookie)
                .body(Body::empty())
                .expect("sysadmin gradebook request"),
        )
        .await
        .expect("sysadmin gradebook response");
    assert_eq!(
        sysadmin_gradebook.status(),
        StatusCode::NOT_FOUND,
        "sysadmin status alone must not disclose FERPA gradebook data"
    );

    let student_gradebook = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/gradebook"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("student gradebook request"),
        )
        .await
        .expect("student gradebook response");
    assert_eq!(student_gradebook.status(), StatusCode::FORBIDDEN);

    let outsider_gradebook = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/gradebook"))
                .header("cookie", &outsider_cookie)
                .body(Body::empty())
                .expect("outsider gradebook request"),
        )
        .await
        .expect("outsider gradebook response");
    assert_eq!(outsider_gradebook.status(), StatusCode::NOT_FOUND);

    let second_assignment = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/courses/{course}/assignments/drafts"))
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"title": "Second assignment"}).to_string(),
                ))
                .expect("second assignment request"),
        )
        .await
        .expect("second assignment response");
    assert_eq!(second_assignment.status(), StatusCode::CREATED);

    let second_course = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/courses")
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"BIOC 302: Enzymes","term":{"startDate":"2027-01-11","endDate":"2027-05-07","timeZone":"America/Chicago"}}"#,
                ))
                .expect("second course request"),
        )
        .await
        .expect("second course response");
    assert_eq!(second_course.status(), StatusCode::CREATED);
    let second_course = response_json(second_course).await;
    let second_course: CourseId =
        serde_json::from_value(second_course["id"].clone()).expect("second course ID response");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course: second_course,
                user: student,
                display_name: "Enzymes learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("second student membership save");

    let student_courses = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/courses?pageSize=1")
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("student courses request"),
        )
        .await
        .expect("student courses response");
    let student_courses = response_json(student_courses).await;
    assert_eq!(student_courses["items"][0]["role"], "student");
    assert_eq!(student_courses["items"].as_array().map(Vec::len), Some(1));
    let course_cursor = student_courses["nextCursor"]
        .as_str()
        .expect("course continuation cursor");
    let continued_courses = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses?pageSize=1&cursor={course_cursor}"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("course continuation request"),
        )
        .await
        .expect("course continuation response");
    assert_eq!(continued_courses.status(), StatusCode::OK);
    let continued_courses = response_json(continued_courses).await;
    assert_eq!(continued_courses["items"].as_array().map(Vec::len), Some(1));
    assert_ne!(student_courses["items"][0], continued_courses["items"][0]);
    assert_eq!(continued_courses["nextCursor"], serde_json::Value::Null);

    let exact_course = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("exact course request"),
        )
        .await
        .expect("exact course response");
    assert_eq!(exact_course.status(), StatusCode::OK);
    assert_eq!(response_json(exact_course).await["role"], "student");

    let student_assignments = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/assignments?pageSize=1"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("student assignments request"),
        )
        .await
        .expect("student assignments response");
    assert_eq!(student_assignments.status(), StatusCode::OK);
    let student_assignments = response_json(student_assignments).await;
    assert_eq!(
        student_assignments["items"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(student_assignments["nextCursor"], serde_json::Value::Null);
    assert_eq!(
        student_assignments["items"][0]["id"],
        assignment.to_string()
    );

    for path in [
        "/api/courses".to_string(),
        format!("/api/courses/{course}/assignments"),
    ] {
        for query in ["pageSize=0", "pageSize=101", "cursor=", "offset=1"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("{path}?{query}"))
                        .header("cookie", &student_cookie)
                        .body(Body::empty())
                        .expect("invalid pagination request"),
                )
                .await
                .expect("invalid pagination response");
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "{path}?{query} must be rejected"
            );
        }
    }

    let exact = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("exact assignment request"),
        )
        .await
        .expect("exact assignment response");
    assert_eq!(
        exact.status(),
        StatusCode::FORBIDDEN,
        "students use the learner-detail projection rather than the instructor editor"
    );

    let outsider_courses = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/courses")
                .header("cookie", &outsider_cookie)
                .body(Body::empty())
                .expect("outsider courses request"),
        )
        .await
        .expect("outsider courses response");
    assert!(
        response_json(outsider_courses).await["items"]
            .as_array()
            .expect("course items")
            .is_empty()
    );

    let hidden_course = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}"))
                .header("cookie", &outsider_cookie)
                .body(Body::empty())
                .expect("hidden course request"),
        )
        .await
        .expect("hidden course response");
    assert_eq!(hidden_course.status(), StatusCode::NOT_FOUND);

    let hidden = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/assignments"))
                .header("cookie", &outsider_cookie)
                .body(Body::empty())
                .expect("hidden assignments request"),
        )
        .await
        .expect("hidden assignments response");
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);

    let student_update = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/content"
                ))
                .header("cookie", &student_cookie)
                .header(IF_MATCH, &assignment_etag)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "student overwrite", "entries": update_entries.clone(),
                    })
                    .to_string(),
                ))
                .expect("student update request"),
        )
        .await
        .expect("student update response");
    assert_eq!(student_update.status(), StatusCode::FORBIDDEN);

    let student_missing_revision = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/content"
                ))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "student missing revision", "entries": update_entries.clone(),
                    })
                    .to_string(),
                ))
                .expect("student missing revision request"),
        )
        .await
        .expect("student missing revision response");
    assert_eq!(student_missing_revision.status(), StatusCode::FORBIDDEN);

    let student_write = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/courses/{course}/assignments/drafts"))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"title": "Second assignment"}).to_string(),
                ))
                .expect("student write request"),
        )
        .await
        .expect("student write response");
    assert_eq!(student_write.status(), StatusCode::FORBIDDEN);

    store
        .seed_retention_cleanup_for_test(
            tenant,
            course,
            (0..4)
                .map(|offset| ObjectId::from_uuid(id(100 + offset)))
                .collect(),
        )
        .expect("archive cleanup fixture");
    let claim = store
        .claim_next_job(
            &learning_data_access::JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease duration"),
        )
        .await
        .expect("archive claim")
        .expect("archive job");
    let (stage, generation) = match claim.payload {
        JobPayload::Retention {
            course: claimed_course,
            stage,
            generation,
        } => {
            assert_eq!(claimed_course, course);
            (stage, generation)
        }
        _ => panic!("fixture must claim retention work"),
    };
    store
        .prepare_retention_work(RetentionWorkerCommand {
            tenant,
            course,
            stage,
            generation,
            job: claim.id,
            lease: claim.lease_token,
        })
        .await
        .expect("archive prepare fence");

    for uri in [
        format!("/api/courses/{course}"),
        format!("/api/courses/{course}/assignments"),
        format!("/api/courses/{course}/assignments/{assignment}"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("archived learner request"),
            )
            .await
            .expect("archived learner response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get("cache-control"),
            Some(&HeaderValue::from_static("no-store"))
        );
    }

    let student_courses = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/courses")
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived learner course list"),
        )
        .await
        .expect("archived learner course response");
    let student_courses = response_json(student_courses).await;
    assert!(
        student_courses["items"]
            .as_array()
            .expect("course items")
            .iter()
            .all(|item| item["id"] != serde_json::json!(course)),
        "archived course leaked into learner list: {student_courses}"
    );

    let instructor_courses = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/courses")
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("retained instructor course list"),
        )
        .await
        .expect("retained instructor course response");
    let instructor_courses = response_json(instructor_courses).await;
    assert!(
        instructor_courses["items"]
            .as_array()
            .expect("course items")
            .iter()
            .any(|item| item["id"] == serde_json::json!(course)),
        "retained course missing from instructor list: {instructor_courses}"
    );

    for (cookie, uri) in [
        (&instructor_cookie, format!("/api/courses/{course}")),
        (
            &instructor_cookie,
            format!("/api/courses/{course}/assignments"),
        ),
        (
            &instructor_cookie,
            format!("/api/courses/{course}/assignments/{assignment}"),
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .expect("retained instructor definition request"),
            )
            .await
            .expect("retained instructor definition response");
        assert_eq!(response.status(), StatusCode::OK);
    }
    let sysadmin_course = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}"))
                .header("cookie", &sysadmin_cookie)
                .body(Body::empty())
                .expect("sysadmin retained-course request"),
        )
        .await
        .expect("sysadmin retained-course response");
    assert_eq!(sysadmin_course.status(), StatusCode::NOT_FOUND);

    let archived_gradebook = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/gradebook"))
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("archived gradebook request"),
        )
        .await
        .expect("archived gradebook response");
    assert_eq!(archived_gradebook.status(), StatusCode::NOT_FOUND);

    let archived_student_update = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/content"
                ))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("archived learner update request"),
        )
        .await
        .expect("archived learner update response");
    assert_eq!(archived_student_update.status(), StatusCode::NOT_FOUND);
}
