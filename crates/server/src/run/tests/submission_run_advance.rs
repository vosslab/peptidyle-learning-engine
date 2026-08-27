use super::*;

#[tokio::test]
async fn a_run_issues_only_one_active_question_then_advances() {
    let (store, backend, app, student_cookie, _, assignment_id, _) = fixture().await;
    let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id(1)));
    let stored_assignment = store
        .get_assignment_for_edit(context, assignment_id)
        .await
        .expect("assignment read")
        .expect("fixture assignment");
    let first = stored_assignment
        .record
        .items
        .first()
        .expect("fixture assignment has one item");
    store
        .add_assignment_fixed_item(
            context,
            learning_data_access::AddAssignmentFixedItemCommand {
                actor: UserId::from_uuid(id(2)),
                course: stored_assignment.record.course_id,
                assignment: assignment_id,
                expected_revision: stored_assignment.revision,
                item: question_model::AssignmentItem {
                    id: question_model::AssignmentItemId::from_uuid(id(1_100_000)),
                    reference: first.reference,
                    position: 1,
                    points_possible: first.points_possible,
                    delivery_state: first.delivery_state,
                    scoring_mode: first.scoring_mode,
                },
            },
        )
        .await
        .expect("two-position assignment");

    let started = app
        .clone()
        .oneshot(start_run_request(
            CourseId::from_uuid(id(5)),
            assignment_id,
            &student_cookie,
        ))
        .await
        .expect("start response");
    let run: AssignmentRun = serde_json::from_value(json(started).await).expect("run response");
    let first_page_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{}/attempts?pageSize=1", run.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("attempt request"),
        )
        .await
        .expect("first attempt page");
    let first_page: Page<QuestionAttempt> =
        serde_json::from_value(json(first_page_response).await).expect("attempt page");
    assert_eq!(first_page.items.len(), 1);
    assert_eq!(first_page.items[0].assignment_position, 0);

    let submission = Request::builder()
        .method("POST")
        .uri(submission_path(
            CourseId::from_uuid(id(5)),
            assignment_id,
            first_page.items[0].id,
        ))
        .header("cookie", &student_cookie)
        .header("content-type", "application/json")
        .header("idempotency-key", "advance-to-second")
        .body(Body::from(
            serde_json::json!({
                "response": { "kind": "numeric", "value": 18.0 }
            })
            .to_string(),
        ))
        .expect("submission request");
    let submission_response = app
        .clone()
        .oneshot(submission)
        .await
        .expect("submission response");
    assert_eq!(submission_response.status(), StatusCode::ACCEPTED);
    assert_eq!(json(submission_response).await["kind"], "accepted_pending");
    drain_one_accepted_submission(&store, backend.clone()).await;
    let resumed = app
        .clone()
        .oneshot(start_run_request(
            CourseId::from_uuid(id(5)),
            assignment_id,
            &student_cookie,
        ))
        .await
        .expect("resume response");
    assert_eq!(resumed.status(), StatusCode::CREATED);

    let second_page_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{}/attempts?pageSize=1", run.id))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("second attempt request"),
        )
        .await
        .expect("second attempt page");
    let second_page: Page<QuestionAttempt> =
        serde_json::from_value(json(second_page_response).await).expect("attempt page");
    assert_eq!(second_page.items.len(), 1);
    assert!(second_page.items[0].response.is_none());
    // Accepted grading has committed, but assignment recalculation remains a
    // separate worker transition. Learner attempt lists keep the result
    // answer-free until that scoring generation becomes current.
    assert!(second_page.items[0].result.is_none());
    let cursor = second_page
        .next_cursor
        .expect("bounded first attempt page must continue");
    let continued_page_response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/runs/{}/attempts?pageSize=1&cursor={}",
                    run.id,
                    cursor.as_str()
                ))
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("continued attempt request"),
        )
        .await
        .expect("continued attempt page");
    let continued_page: Page<QuestionAttempt> =
        serde_json::from_value(json(continued_page_response).await).expect("attempt page");
    assert_eq!(continued_page.items.len(), 1);
    assert_ne!(second_page.items[0].id, continued_page.items[0].id);
    assert_eq!(continued_page.items[0].assignment_position, 1);
    assert!(continued_page.items[0].response.is_none());
    assert_eq!(continued_page.next_cursor, None);
    assert_eq!(backend.issued_seeds.lock().expect("seed record").len(), 2);
}
