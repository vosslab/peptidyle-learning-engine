use super::*;
use learning_data_access::{
    AssignmentScoringCommitOutcome, AssignmentScoringPreparationOutcome,
    AssignmentScoringWorkerCommand, AssignmentScoringWorkerStore, CourseItemAnalysisCommitOutcome,
    CourseItemAnalysisWorkerCommand, CourseItemAnalysisWorkerStore, EnqueueJob, JobClaimFilter,
    JobFailureKind, JobLeaseDuration, JobPayload, JobStore,
};
pub(super) fn assert_redacted_item_surface(value: &serde_json::Value, status: &str, surface: &str) {
    assert_eq!(value["scoringStatus"], status, "{surface} status");
    let attempt = value.get("attempt").unwrap_or(value);
    assert!(
        attempt["result"].is_null(),
        "{surface} result must be absent: {attempt}"
    );
    if let Some(feedback) = value.get("feedback") {
        assert!(
            feedback["pointsEarned"].is_null(),
            "{surface} earned points"
        );
        assert!(
            feedback["pointsPossible"].is_null(),
            "{surface} possible points"
        );
    }
}

pub(super) async fn fail_assignment_scoring_job(store: &MemoryStore, assignment: AssignmentId) {
    loop {
        let claim = store
            .claim_next_job(
                &JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(30).expect("lease"),
            )
            .await
            .expect("claim job")
            .expect("pending assignment scoring job");
        match claim.payload {
            JobPayload::RecalculateAssignment {
                assignment: candidate,
                ..
            } if candidate == assignment => {
                let context = TenantContext::from_authenticated_session(claim.tenant);
                store
                    .fail_job(
                        context,
                        claim.id,
                        claim.lease_token,
                        JobFailureKind::Permanent,
                    )
                    .await
                    .expect("permanent score-worker failure");
                let status = store
                    .get_assignment_for_edit(context, assignment)
                    .await
                    .expect("assignment scoring read")
                    .expect("fixture assignment")
                    .scoring_status;
                if matches!(status, question_model::ScoringStatus::Failed) {
                    return;
                }
            }
            _ => store
                .complete_job(claim.id, claim.lease_token)
                .await
                .expect("complete unrelated job"),
        }
    }
}

#[tokio::test]
async fn mixed_assignment_disclosure_projects_each_field_at_http_boundary() {
    let (store, backend, app, student_cookie, outsider_cookie, assignment) =
        native_feedback_fixture().await;
    let mixed_policy = question_model::StudentDisclosurePolicy {
        score: question_model::StudentDisclosureTiming::AfterSubmit,
        per_item_correctness: question_model::StudentDisclosureTiming::Never,
        feedback_text: question_model::StudentDisclosureTiming::AfterSubmit,
        solution: question_model::StudentDisclosureTiming::Never,
        class_statistics: question_model::StudentDisclosureTiming::AfterSubmit,
    };
    replace_disclosure_policy(store.as_ref(), assignment, mixed_policy).await;

    let first = active_attempt_for(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        &student_cookie,
    )
    .await;
    let before_submit = run_summary(&app, first.run, &student_cookie).await;
    assert!(before_submit["outcomes"]["items"][0]["submittedAt"].is_null());
    assert!(before_submit["outcomes"]["items"][0]["feedback"].is_null());
    assert!(before_submit["summary"].get("class_statistics").is_none());

    let ester = presented_choice_id(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        first.id,
        &student_cookie,
        0,
    )
    .await;
    let request = || {
        Request::builder()
            .method("POST")
            .uri(submission_path(
                CourseId::from_uuid(id(205)),
                assignment,
                first.id,
            ))
            .header("cookie", &student_cookie)
            .header("content-type", "application/json")
            .header("idempotency-key", "s4-assignment-disclosure")
            .body(Body::from(
                serde_json::json!({
                    "response": { "kind": "multipleChoice", "selected": [ester] }
                })
                .to_string(),
            ))
            .expect("submission request")
    };
    let accepted = app.clone().oneshot(request()).await.expect("submission");
    assert_eq!(accepted.status(), StatusCode::ACCEPTED);
    drain_one_accepted_submission(&store, Arc::clone(&backend)).await;
    publish_pending_assignment_scoring(
        store.as_ref(),
        TenantContext::from_authenticated_session(TenantId::from_uuid(id(201))),
    )
    .await;
    let response = app
        .clone()
        .oneshot(request())
        .await
        .expect("receipt replay");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let receipt = json(response).await;
    assert!(receipt["attempt"]["result"].is_null());
    assert!(receipt["feedback"].get("correctness").is_none());
    assert_eq!(receipt["feedback"]["pointsEarned"], 0.0);
    assert_eq!(receipt["feedback"]["pointsPossible"], 2.0);
    assert!(receipt["feedback"]["hint"].is_array());
    assert!(receipt["feedback"]["rationale"].is_array());
    assert!(receipt["feedback"].get("correctResponse").is_none());
    assert_eq!(backend.submissions.load(Ordering::SeqCst), 1);

    let replay = app.clone().oneshot(request()).await.expect("replay");
    assert_eq!(replay.status(), StatusCode::OK);
    let replay_body = json(replay).await;
    assert_eq!(replay_body, receipt);
    assert_eq!(backend.submissions.load(Ordering::SeqCst), 1);

    let second: QuestionAttemptId = serde_json::from_value(receipt["nextIssued"]["id"].clone())
        .expect("receipt successor attempt id");
    let second_ester = presented_choice_id(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        second,
        &student_cookie,
        0,
    )
    .await;
    let second_submission = replay_submission(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        second,
        &student_cookie,
        "s4-mixed-assignment-disclosure-second",
        serde_json::json!({ "kind": "multipleChoice", "selected": [second_ester] }),
    )
    .await;
    assert_eq!(second_submission.status(), StatusCode::ACCEPTED);
    drain_one_accepted_submission(&store, Arc::clone(&backend)).await;
    let second_submission = replay_submission(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        second,
        &student_cookie,
        "s4-mixed-assignment-disclosure-second",
        serde_json::json!({ "kind": "multipleChoice", "selected": [second_ester] }),
    )
    .await;
    assert_eq!(second_submission.status(), StatusCode::OK);
    publish_pending_assignment_scoring(
        store.as_ref(),
        TenantContext::from_authenticated_session(TenantId::from_uuid(id(201))),
    )
    .await;

    let summary = run_summary(&app, first.run, &student_cookie).await;
    assert_eq!(
        summary["outcomes"]["items"][0]["feedback"],
        receipt["feedback"]
    );
    assert!(summary["run"]["score"].is_number());
    assert_eq!(
        summary["summary"]["class_statistics"],
        serde_json::json!({ "state": "insufficient_evidence" })
    );
    for forbidden in ["answerKey", "checker", "provider", "source", "launchUrl"] {
        assert!(!summary.to_string().contains(forbidden));
    }

    replace_disclosure_policy(
        store.as_ref(),
        assignment,
        question_model::StudentDisclosurePolicy {
            score: question_model::StudentDisclosureTiming::Never,
            per_item_correctness: question_model::StudentDisclosureTiming::Never,
            feedback_text: question_model::StudentDisclosureTiming::Never,
            solution: question_model::StudentDisclosureTiming::Never,
            class_statistics: question_model::StudentDisclosureTiming::Never,
        },
    )
    .await;
    let revised = run_summary(&app, first.run, &student_cookie).await;
    assert!(revised["run"]["score"].is_null());
    assert!(revised["outcomes"]["items"][0]["feedback"].is_null());
    assert_eq!(revised["summary"]["score_state"], "withheld");
    assert!(revised["summary"].get("class_statistics").is_none());
    assert!(revised["summary"].get("tenant").is_none());
    assert!(revised["summary"].get("enrollment").is_none());

    let instructor_cookie = issued_cookie_for(
        store.as_ref(),
        TenantId::from_uuid(id(201)),
        UserId::from_uuid(id(202)),
        "Instructor",
    )
    .await;
    let instructor = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{}/summary", first.run))
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("instructor summary request"),
        )
        .await
        .expect("instructor summary response");
    assert_eq!(instructor.status(), StatusCode::OK);
    let instructor = json(instructor).await;
    assert_eq!(instructor["scoringStatus"], "current");
    assert!(instructor["run"]["score"].is_number());
    assert!(instructor["summary"]["currentScore"].is_number());
    assert!(instructor["summary"].get("tenant").is_some());
    assert!(instructor["summary"].get("enrollment").is_some());
    assert!(instructor["outcomes"]["items"][0]["feedback"].is_null());
    for forbidden in ["answerKey", "checker", "provider", "source", "launchUrl"] {
        assert!(!instructor.to_string().contains(forbidden));
    }

    set_assignment_item_points(store.as_ref(), assignment, 3).await;
    let recalculating = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{}/summary", first.run))
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("recalculating instructor summary request"),
        )
        .await
        .expect("recalculating instructor summary response");
    assert_eq!(recalculating.status(), StatusCode::OK);
    let recalculating = json(recalculating).await;
    assert_eq!(recalculating["scoringStatus"], "recalculating");
    assert!(recalculating["run"]["score"].is_null());
    assert!(recalculating["summary"]["currentScore"].is_null());
    assert!(recalculating["summary"]["bestScore"].is_null());
    assert!(recalculating["summary"]["latestScore"].is_null());

    let outsider = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{}/summary", first.run))
                .header("cookie", outsider_cookie)
                .body(Body::empty())
                .expect("outsider request"),
        )
        .await
        .expect("outsider response");
    assert_eq!(outsider.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn during_attempt_class_statistics_are_projected_without_score_activity() {
    let (store, _backend, app, student_cookie, _outsider_cookie, assignment) =
        native_feedback_fixture().await;
    replace_disclosure_policy(
        store.as_ref(),
        assignment,
        question_model::StudentDisclosurePolicy {
            class_statistics: question_model::StudentDisclosureTiming::DuringAttempt,
            ..Default::default()
        },
    )
    .await;

    let first = active_attempt_for(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        &student_cookie,
    )
    .await;
    let summary = run_summary(&app, first.run, &student_cookie).await;

    assert_eq!(summary["summary"]["score_state"], "no_activity");
    assert_eq!(
        summary["summary"]["class_statistics"],
        serde_json::json!({ "state": "insufficient_evidence" })
    );
    assert!(summary["summary"].get("disclosurePolicy").is_none());
}

#[tokio::test]
async fn pre_receipt_assignment_summary_projects_policy_without_materializing() {
    let (store, _backend, app, student_cookie, outsider_cookie, assignment) =
        native_feedback_fixture().await;
    replace_disclosure_policy(
        store.as_ref(),
        assignment,
        question_model::StudentDisclosurePolicy {
            class_statistics: question_model::StudentDisclosureTiming::DuringAttempt,
            ..Default::default()
        },
    )
    .await;
    let tenant = TenantId::from_uuid(id(201));
    let context = TenantContext::from_authenticated_session(tenant);
    let student = UserId::from_uuid(id(203));
    assert!(
        store
            .student_get_enrollment_for_assignment(context, student, assignment)
            .await
            .expect("pre-receipt enrollment lookup")
            .is_none()
    );
    let app = app.merge(crate::course::router(Arc::clone(&store)));
    let summary = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/assignments/{assignment}/summary"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("pre-receipt assignment summary request"),
        )
        .await
        .expect("pre-receipt assignment summary response");
    assert_eq!(summary.status(), StatusCode::OK);
    let summary = json(summary).await;
    assert_eq!(summary["score_state"], "no_activity");
    assert_eq!(
        summary["class_statistics"],
        serde_json::json!({ "state": "insufficient_evidence" })
    );
    assert!(
        store
            .student_get_enrollment_for_assignment(context, student, assignment)
            .await
            .expect("post-projection enrollment lookup")
            .is_none()
    );
    let denied = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/assignments/{assignment}/summary"))
                .header("cookie", outsider_cookie)
                .body(Body::empty())
                .expect("denied assignment summary request"),
        )
        .await
        .expect("denied assignment summary response");
    assert_eq!(denied.status(), StatusCode::NOT_FOUND);
}

/// A five-Student cohort reaches this route only after ordinary HTTP attempts,
/// the real scoring worker, and the real item-analysis worker have produced a
/// current report.  Keep the fixture end-to-end so safe metrics cannot become
/// a hand-built test-only projection.
#[tokio::test]
async fn current_five_student_analysis_projects_only_safe_available_class_statistics_at_http() {
    let (store, backend, app, student_cookie, _outsider_cookie, assignment) =
        native_feedback_fixture().await;
    let tenant = TenantId::from_uuid(id(201));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(202));
    let course = CourseId::from_uuid(id(205));
    set_assignment_item_points(store.as_ref(), assignment, 2).await;
    replace_disclosure_policy(
        store.as_ref(),
        assignment,
        question_model::StudentDisclosurePolicy {
            score: question_model::StudentDisclosureTiming::Never,
            class_statistics: question_model::StudentDisclosureTiming::AfterSubmit,
            ..Default::default()
        },
    )
    .await;

    let mut students = vec![(UserId::from_uuid(id(203)), student_cookie)];
    for offset in 0..4_u128 {
        let student = UserId::from_uuid(id(250 + offset));
        store
            .upsert_course_member(
                context,
                instructor,
                UpsertCourseMember {
                    course,
                    user: student,
                    display_name: format!("Class statistics Student {offset}"),
                    roster_contact: None,
                },
            )
            .await
            .expect("cohort roster membership");
        let entitlement = store
            .issue_assignment_entitlement(
                context,
                MaterializeAssignmentEntitlementCommand::for_instructor_action(
                    student,
                    course,
                    assignment,
                    instructor,
                    EntitlementPurpose::InstructorIssue,
                )
                .expect("cohort instructor issue command"),
            )
            .await
            .expect("cohort entitlement materialization");
        assert!(matches!(
            entitlement,
            learning_data_access::AssignmentEntitlementMaterialization::Granted(_)
        ));
        students.push((
            student,
            issued_cookie_for(store.as_ref(), tenant, student, "Class statistics Student").await,
        ));
    }

    let mut student_runs = Vec::new();
    for (_student, cookie) in &students {
        let first =
            active_attempt_for(&app, CourseId::from_uuid(id(205)), assignment, cookie).await;
        let second = submit_peptide_linkage(
            &app,
            &store,
            &backend,
            CourseId::from_uuid(id(205)),
            assignment,
            first.id,
            cookie,
        )
        .await;
        submit_peptide_linkage(
            &app,
            &store,
            &backend,
            CourseId::from_uuid(id(205)),
            assignment,
            second.expect("first response must issue the next attempt"),
            cookie,
        )
        .await;
        student_runs.push(first.run);
    }

    publish_pending_assignment_scoring(store.as_ref(), context).await;
    let scoring_generation = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("current assignment scoring read")
        .expect("current assignment")
        .scoring_generation;
    store
        .enqueue_job(
            context,
            EnqueueJob {
                tenant,
                payload: JobPayload::RecalculateCourseItemAnalysis {
                    assignment,
                    generation: scoring_generation,
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("enqueue current cohort analysis");
    publish_pending_scoring_and_analysis(store.as_ref(), context).await;
    let instructor_session = SessionTokenHash::compute(b"s4-class-statistics-instructor");
    store
        .create_session(
            instructor_session,
            SessionSubject::new(
                tenant,
                instructor,
                "Class statistics instructor",
                vec![UserRole::Instructor],
            )
            .expect("instructor session subject"),
            SessionLifetime::from_seconds(60).expect("instructor session lifetime"),
        )
        .await
        .expect("instructor session");
    let report = store
        .course_item_analysis(context, instructor_session, course, assignment)
        .await
        .expect("current analysis read")
        .expect("current analysis report");
    assert_eq!(report.completed_run_count, 5);
    assert!(!report.incomplete_scoring);
    assert!(!report.recent_rescoring);
    assert_eq!(report.assignment_average_score, Some(1.0));
    assert_eq!(
        store
            .student_class_statistics(context, students[0].0, course, assignment)
            .await
            .expect("current entitled Student statistics"),
        question_model::StudentClassStatistics::Available {
            completed_student_cohort_size: 5,
            assignment_average_score: 1.0,
        }
    );

    let summary = run_summary(&app, student_runs[0], &students[0].1).await;
    assert_eq!(summary["summary"]["score_state"], "withheld");
    assert_eq!(
        summary["summary"]["class_statistics"],
        serde_json::json!({
            "state": "available",
            "completed_student_cohort_size": 5,
            "assignment_average_score": 1.0,
        })
    );
    let class_statistics = &summary["summary"]["class_statistics"];
    assert_eq!(
        class_statistics.as_object().map(|value| value.len()),
        Some(3)
    );
    for forbidden in [
        "tenant",
        "course",
        "assignment",
        "student",
        "enrollment",
        "run",
        "attempt",
        "policy",
        "clock",
        "analyzedAt",
        "sourceScoringGeneration",
    ] {
        assert!(
            class_statistics.get(forbidden).is_none(),
            "safe statistics projection must omit {forbidden}"
        );
    }
}

pub(super) async fn publish_pending_scoring_and_analysis(
    store: &MemoryStore,
    context: TenantContext,
) {
    let mut published_current_analysis = false;
    while let Some(claim) = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("worker lease"),
        )
        .await
        .expect("worker claim")
    {
        match claim.payload {
            JobPayload::RecalculateAssignment {
                assignment,
                generation,
            } => {
                let command = AssignmentScoringWorkerCommand {
                    job: claim.id,
                    lease: claim.lease_token,
                    assignment,
                    generation,
                };
                store
                    .prepare_assignment_scoring(context, command)
                    .await
                    .expect("score cohort assignment");
                assert!(matches!(
                    store
                        .commit_assignment_scoring(context, command)
                        .await
                        .expect("publish cohort score"),
                    AssignmentScoringCommitOutcome::Committed
                        | AssignmentScoringCommitOutcome::Superseded
                ));
            }
            JobPayload::RecalculateCourseItemAnalysis {
                assignment,
                generation,
            } => {
                let command = CourseItemAnalysisWorkerCommand {
                    job: claim.id,
                    lease: claim.lease_token,
                    assignment,
                    generation,
                };
                store
                    .prepare_course_item_analysis(context, command)
                    .await
                    .expect("stage cohort analysis");
                published_current_analysis |= matches!(
                    store
                        .commit_course_item_analysis(context, command)
                        .await
                        .expect("publish cohort analysis"),
                    CourseItemAnalysisCommitOutcome::Committed
                );
            }
            payload => panic!("unexpected cohort worker payload: {payload:?}"),
        }
    }
    assert!(
        published_current_analysis,
        "current cohort analysis must publish"
    );
}

pub(super) async fn publish_pending_assignment_scoring(
    store: &MemoryStore,
    context: TenantContext,
) {
    let mut published = false;
    while let Some(claim) = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("worker lease"),
        )
        .await
        .expect("worker claim")
    {
        match claim.payload {
            JobPayload::RecalculateAssignment {
                assignment,
                generation,
            } => {
                let command = AssignmentScoringWorkerCommand {
                    job: claim.id,
                    lease: claim.lease_token,
                    assignment,
                    generation,
                };
                match store.prepare_assignment_scoring(context, command).await {
                    Ok(
                        AssignmentScoringPreparationOutcome::Prepared
                        | AssignmentScoringPreparationOutcome::Superseded,
                    ) => {
                        let outcome = store
                            .commit_assignment_scoring(context, command)
                            .await
                            .expect("publish or retire assignment score");
                        published |= outcome == AssignmentScoringCommitOutcome::Committed;
                    }
                    Err(error) => panic!("score assignment: {error}"),
                }
            }
            JobPayload::RecalculateCourseItemAnalysis { .. } => store
                .complete_job(claim.id, claim.lease_token)
                .await
                .expect("complete unrelated analysis job"),
            payload => panic!("unexpected assignment scoring worker payload: {payload:?}"),
        }
    }
    assert!(published, "accepted work must enqueue assignment scoring");
}

pub(super) async fn submit_peptide_linkage(
    app: &Router,
    store: &Arc<MemoryStore>,
    backend: &Arc<CountingNativeBackend>,
    course: CourseId,
    assignment: AssignmentId,
    attempt: QuestionAttemptId,
    cookie: &str,
) -> Option<QuestionAttemptId> {
    let peptide_linkage = presented_choice_id(app, course, assignment, attempt, cookie, 1).await;
    let completed = submit_and_complete_accepted_submission(
        app,
        store,
        Arc::clone(backend),
        AcceptedSubmissionRoute {
            course,
            assignment,
            attempt,
            cookie,
            idempotency_key: &format!("s4-class-statistics-{attempt}"),
            response: serde_json::json!({
                "kind": "multipleChoice",
                "selected": [peptide_linkage]
            }),
        },
    )
    .await;
    let replay = replay_submission(
        app,
        course,
        assignment,
        attempt,
        cookie,
        &format!("s4-class-statistics-{attempt}"),
        serde_json::json!({
            "kind": "multipleChoice",
            "selected": [peptide_linkage]
        }),
    )
    .await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(
        completed["feedback"]["correctness"],
        serde_json::Value::Null,
        "correctness remains withheld until the score is current"
    );
    assert_eq!(completed["scoringStatus"], "recalculating");
    let replay_body = json(replay).await;
    replay_body
        .get("nextIssued")
        .filter(|next| !next.is_null())
        .map(|next| serde_json::from_value(next["id"].clone()).expect("successor attempt id"))
}

pub(super) async fn replay_submission(
    app: &Router,
    course: CourseId,
    assignment: AssignmentId,
    attempt: QuestionAttemptId,
    cookie: &str,
    idempotency_key: &str,
    response: serde_json::Value,
) -> Response {
    let mut request = Request::new(Body::from(
        serde_json::json!({ "response": response }).to_string(),
    ));
    *request.method_mut() = "POST".parse().expect("post method");
    *request.uri_mut() = submission_path(course, assignment, attempt)
        .parse()
        .expect("submission URI");
    request
        .headers_mut()
        .insert("cookie", cookie.parse().expect("cookie header"));
    request.headers_mut().insert(
        "content-type",
        "application/json".parse().expect("content type header"),
    );
    request.headers_mut().insert(
        "idempotency-key",
        idempotency_key.parse().expect("idempotency header"),
    );
    app.clone()
        .oneshot(request)
        .await
        .expect("submission replay")
}

#[tokio::test]
async fn feedback_release_is_content_free_audit_not_projection_authority() {
    let (store, backend, app, student_cookie, _outsider_cookie, assignment) =
        native_feedback_fixture().await;
    let instructor_cookie = issued_cookie_for(
        store.as_ref(),
        TenantId::from_uuid(id(201)),
        UserId::from_uuid(id(202)),
        "Instructor",
    )
    .await;
    let first = active_attempt_for(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        &student_cookie,
    )
    .await;
    let ester = presented_choice_id(
        &app,
        CourseId::from_uuid(id(205)),
        assignment,
        first.id,
        &student_cookie,
        0,
    )
    .await;
    let submitted = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(submission_path(
                    CourseId::from_uuid(id(205)),
                    assignment,
                    first.id,
                ))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "s4-release-audit")
                .body(Body::from(
                    serde_json::json!({
                        "response": { "kind": "multipleChoice", "selected": [ester] }
                    })
                    .to_string(),
                ))
                .expect("submission request"),
        )
        .await
        .expect("submission");
    assert_eq!(submitted.status(), StatusCode::ACCEPTED);
    drain_one_accepted_submission(&store, Arc::clone(&backend)).await;
    publish_pending_assignment_scoring(
        store.as_ref(),
        TenantContext::from_authenticated_session(TenantId::from_uuid(id(201))),
    )
    .await;
    let submitted = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(submission_path(
                    CourseId::from_uuid(id(205)),
                    assignment,
                    first.id,
                ))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "s4-release-audit")
                .body(Body::from(
                    serde_json::json!({
                        "response": { "kind": "multipleChoice", "selected": [ester] }
                    })
                    .to_string(),
                ))
                .expect("submission receipt replay request"),
        )
        .await
        .expect("submission receipt replay");
    assert_eq!(submitted.status(), StatusCode::OK);
    let receipt = json(submitted).await;
    let before = run_summary(&app, first.run, &student_cookie).await;
    let release = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/attempts/{}/feedback-release", first.id))
                .header("cookie", instructor_cookie)
                .body(Body::empty())
                .expect("release request"),
        )
        .await
        .expect("release");
    assert_eq!(release.status(), StatusCode::OK);
    assert_eq!(release.headers()["cache-control"], "no-store");
    assert_eq!(json(release).await, serde_json::json!({"released": true}));
    assert!(before["outcomes"]["items"][0]["feedback"]["hint"].is_array());
    assert!(before["outcomes"]["items"][0]["feedback"]["rationale"].is_array());
    assert!(before["outcomes"]["items"][0]["feedback"]["correctResponse"].is_array());
    assert_eq!(
        receipt["feedback"],
        before["outcomes"]["items"][0]["feedback"]
    );

    let revised_policy = question_model::StudentDisclosurePolicy {
        feedback_text: question_model::StudentDisclosureTiming::Never,
        solution: question_model::StudentDisclosureTiming::Never,
        ..Default::default()
    };
    replace_disclosure_policy(store.as_ref(), assignment, revised_policy).await;

    let after = run_summary(&app, first.run, &student_cookie).await;
    let feedback = &after["outcomes"]["items"][0]["feedback"];
    assert_eq!(feedback["correctness"], false);
    assert_eq!(feedback["pointsEarned"], 0.0);
    assert!(feedback.get("hint").is_none());
    assert!(feedback.get("rationale").is_none());
    assert!(feedback.get("correctResponse").is_none());
}

pub(super) async fn run_summary(app: &Router, run: RunId, cookie: &str) -> serde_json::Value {
    let summary = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{run}/summary"))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("summary request"),
        )
        .await
        .expect("summary");
    assert_eq!(summary.status(), StatusCode::OK);
    json(summary).await
}

/// Changes only the current assignment policy through its revision-checked
/// store contract. Student routes must use this current value, not a receipt
/// or feedback-release audit event retained from an earlier policy.
pub(super) async fn replace_disclosure_policy(
    store: &MemoryStore,
    assignment: AssignmentId,
    disclosure_policy: question_model::StudentDisclosurePolicy,
) {
    let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id(201)));
    let stored = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("assignment read")
        .expect("fixture assignment");
    store
        .replace_assignment(
            context,
            learning_data_access::ReplaceAssignmentCommand {
                actor: UserId::from_uuid(id(202)),
                course: stored.record.course_id,
                assignment,
                expected_revision: stored.revision,
                update: AssignmentUpdate {
                    title: stored.record.title,
                    audience: stored.record.audience,
                    items: stored.record.items,
                    selection_groups: stored.record.selection_groups,
                    disclosure_policy,
                    policies: stored.record.policies,
                },
            },
        )
        .await
        .expect("revision-checked disclosure policy update");
}

pub(super) async fn set_assignment_item_points(
    store: &MemoryStore,
    assignment: AssignmentId,
    points: u32,
) {
    let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id(201)));
    let stored = store
        .get_assignment_for_edit(context, assignment)
        .await
        .expect("assignment read")
        .expect("fixture assignment");
    let mut items = stored.record.items;
    for item in &mut items {
        item.points_possible = question_model::PointValue::from_whole(points);
    }
    store
        .replace_assignment(
            context,
            learning_data_access::ReplaceAssignmentCommand {
                actor: UserId::from_uuid(id(202)),
                course: stored.record.course_id,
                assignment,
                expected_revision: stored.revision,
                update: AssignmentUpdate {
                    title: stored.record.title,
                    audience: stored.record.audience,
                    items,
                    selection_groups: stored.record.selection_groups,
                    disclosure_policy: stored.record.disclosure_policy,
                    policies: stored.record.policies,
                },
            },
        )
        .await
        .expect("revision-checked item-point update");
}
