//! Connected access, progress, history, and practice-statistics oracle.

use super::*;
use learning_data_access::postgres::{
    PostgresLiveAssessmentDeliveryStore, PostgresLiveStudentCourseLandingStore, lazy_pool,
};
use learning_data_access::{
    AssessmentStartDecision, LiveAssessmentDeliveryStore, LiveStudentCourseLandingStore,
    PageRequest, PageSize,
};
use question_model::{AssessmentId, AssessmentType, CourseInstanceId};
use sqlx::Row;

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
async fn access_reader_projects_one_authoritative_decision_and_effective_policy() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let migration_url = runtime.migration_url().expose();
    let admin = lazy_pool(migration_url).expect("migration pool");
    let fixture = seed(&admin).await;
    let course_instance_id = fixture.course_id.clone();
    let assessment_id = fixture.assessment_id.clone();
    let course = CourseInstanceId::new(&course_instance_id).expect("Course Instance ID");
    let assessment = AssessmentId::new(&assessment_id).expect("Assessment ID");

    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let application = lazy_pool(&application_url).expect("application pool");
    // ASVS 8.2.2 and 8.3.1: an authenticated app session has access only to
    // its exact Student Work identity, at the trusted database boundary.
    assert!(
        authenticated_student_record_ownership(
            &application,
            token(0xe1),
            fixture.course_id.as_str(),
            id(STUDENT_RECORD),
        )
        .await,
        "the authenticated Student may access their exact Student Work record",
    );
    assert!(
        !authenticated_student_record_ownership(
            &application,
            token(0xe2),
            fixture.course_id.as_str(),
            id(STUDENT_RECORD),
        )
        .await,
        "another Student in the Course cannot access this Student Work record",
    );
    assert!(
        !authenticated_student_record_ownership(
            &application,
            token(0xe3),
            fixture.course_id.as_str(),
            id(STUDENT_RECORD),
        )
        .await,
        "a nonmember cannot access this Student Work record",
    );
    assert!(
        !authenticated_student_record_ownership(
            &application,
            token(0xe1),
            fixture.course_id.as_str(),
            id(OTHER_COURSE_STUDENT_RECORD),
        )
        .await,
        "the same Account cannot combine one Course with Student Work from another Course",
    );
    assert!(
        !authenticated_student_record_ownership(
            &application,
            token(0xe4),
            fixture.course_id.as_str(),
            id(STUDENT_RECORD),
        )
        .await,
        "an ordinary Sysadmin cannot access Student Work without Student ownership",
    );
    let store = PostgresLiveAssessmentDeliveryStore::new(application.clone());
    let access = store
        .live_assessment_access(token(0xe1), course.clone(), assessment.clone())
        .await
        .expect("authorized Student Assessment Access");
    assert_eq!(
        access.decision.start_decision,
        AssessmentStartDecision::NotYetAvailable
    );
    assert_eq!(
        access.decision.public_reason.as_deref(),
        Some("This Assessment is not yet available.")
    );
    assert_eq!(access.question_count, 1);
    assert_eq!(access.assessment_type, AssessmentType::RegularAssignment);
    assert_eq!(access.points_possible, 2.0);
    assert_eq!(access.decision.time_limit_seconds, Some(600));
    assert_eq!(access.decision.attempt_limit, Some(2));

    let landing_store = PostgresLiveStudentCourseLandingStore::new(application.clone());
    let landing = landing_store
        .list_released_live_student_assessments(token(0xe1), course.clone())
        .await
        .expect("authorized Student Assessment landing");
    assert_eq!(landing.len(), 1, "scheduled Assessment remains visible");
    assert_eq!(
        landing[0].assessment_type,
        AssessmentType::RegularAssignment
    );
    assert!(!landing[0].can_resume_assessment_attempt);
    let landing_decision = &landing[0].decision;
    assert_eq!(
        landing_decision.start_decision,
        access.decision.start_decision
    );
    assert_eq!(landing_decision.available_at, access.decision.available_at);
    assert_eq!(landing_decision.due_at, access.decision.due_at);
    assert_eq!(landing_decision.closes_at, access.decision.closes_at);
    assert_eq!(
        landing_decision.time_limit_seconds,
        access.decision.time_limit_seconds
    );
    assert_eq!(
        landing_decision.attempt_limit,
        access.decision.attempt_limit
    );
    assert_eq!(
        landing_decision.late_work_rule,
        access.decision.late_work_rule
    );
    assert_eq!(
        landing_decision.display_time_zone,
        access.decision.display_time_zone
    );
    assert_eq!(
        landing_decision.public_reason,
        access.decision.public_reason
    );

    let mut progress_fixture = admin.begin().await.expect("Progress Attempt fixture");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *progress_fixture)
        .await
        .expect("private Progress fixture role");
    sqlx::query(
        "INSERT INTO ple_private.student_assessment_accommodation( \
             accommodation_id, course_instance_id, student_record_id, assessment_id, \
             available_at, due_at, closes_at, time_multiplier, created_at \
         ) VALUES ( \
             $1, $2, $3, $4, clock_timestamp() - interval '1 hour', \
             clock_timestamp() + interval '1 hour', clock_timestamp() + interval '2 hours', \
             1, pg_catalog.transaction_timestamp() \
         )",
    )
    .bind(id(STUDENT_RESUME_ACCOMMODATION))
    .bind(&fixture.course_id)
    .bind(id(STUDENT_RECORD))
    .bind(&fixture.assessment_id)
    .execute(&mut *progress_fixture)
    .await
    .expect("currently available Assessment for resumable Attempt fixture");
    sqlx::query(
        "INSERT INTO ple_private.assessment_attempt ( \
             course_instance_id, assessment_attempt_id, student_record_id, assessment_id, \
             assessment_attempt_number, started_at, expires_at, assessment_policy_snapshot_id \
         ) \
         SELECT assessment.course_instance_id, attempts.assessment_attempt_id, $1, \
                assessment.assessment_id, attempts.attempt_number, attempts.started_at, \
                expiration.expires_at, assessment.assessment_policy_snapshot_id \
           FROM ple_data.assessment AS assessment \
           CROSS JOIN (VALUES ($2::uuid, 1, pg_catalog.clock_timestamp() - interval '2 minutes'), \
                              ($3::uuid, 2, pg_catalog.clock_timestamp() - interval '1 minute')) \
                AS attempts(assessment_attempt_id, attempt_number, started_at) \
         CROSS JOIN LATERAL (SELECT attempts.started_at + interval '10 minutes' AS expires_at) \
                AS expiration \
          WHERE assessment.course_instance_id = $4 AND assessment.assessment_id = $5",
    )
    .bind(id(STUDENT_RECORD))
    .bind(id(PROGRESS_ATTEMPT))
    .bind(id(HISTORY_ATTEMPT))
    .bind(&fixture.course_id)
    .bind(&fixture.assessment_id)
    .execute(&mut *progress_fixture)
    .await
    .expect("unsubmitted Student Attempt");
    progress_fixture
        .commit()
        .await
        .expect("Progress Attempt fixture commit");

    let progress = landing_store
        .list_live_student_course_progress(token(0xe1), course.clone())
        .await
        .expect("authorized Course Progress");
    assert_eq!(progress.len(), 1);
    assert_eq!(progress[0].assessment_attempt_count, 2);
    assert_eq!(progress[0].submitted_assessment_attempt_count, 0);
    assert_eq!(progress[0].latest_assessment_attempt_number, Some(2));
    assert_eq!(
        progress[0].latest_assessment_attempt_completion,
        Some(question_model::AssessmentAttemptCompletion::InProgress)
    );
    assert!(progress[0].latest_activity_at_millis.is_some());
    assert!(progress[0].assessment_score.is_none());
    assert_eq!(
        landing_store
            .get_live_student_course_active_attempt(token(0xe1), course.clone())
            .await
            .expect("authorized Course Active Attempt")
            .map(|attempt| attempt.assessment_attempt_id),
        Some(question_model::AssessmentAttemptId::from_uuid(id(
            HISTORY_ATTEMPT
        ))),
        "the newest Attempt with a running clock is the Course shortcut target",
    );
    assert_eq!(
        landing_store
            .get_live_student_course_active_attempt(token(0xe2), course.clone())
            .await
            .expect("another Course member has no active Attempt"),
        None,
    );
    assert!(
        landing_store
            .get_live_student_course_active_attempt(token(0xe3), course.clone())
            .await
            .is_err(),
        "a nonmember cannot read the Course Active Attempt",
    );
    let other_course = CourseInstanceId::new(&fixture.other_course_id).expect("other Course ID");
    assert_eq!(
        landing_store
            .get_live_student_course_active_attempt(token(0xe1), other_course)
            .await
            .expect("the same Student is authorized in the other Course"),
        None,
        "an Active Attempt in one Course does not appear in another Course's scoped read",
    );
    let first_history_page = landing_store
        .list_live_student_course_attempt_history(
            token(0xe1),
            course.clone(),
            PageRequest::first(PageSize::new(1).expect("one-row page")),
        )
        .await
        .expect("authorized first Attempt History page");
    assert_eq!(first_history_page.items.len(), 1);
    assert_eq!(first_history_page.items[0].assessment_attempt_number, 2);
    assert!(first_history_page.items[0].assessment_score.is_none());
    let older_cursor = first_history_page
        .next_cursor
        .expect("first page has a continuation");
    let second_history_page = landing_store
        .list_live_student_course_attempt_history(
            token(0xe1),
            course.clone(),
            PageRequest::after(older_cursor, PageSize::new(1).expect("one-row page")),
        )
        .await
        .expect("authorized older Attempt History page");
    assert_eq!(second_history_page.items.len(), 1);
    assert_eq!(second_history_page.items[0].assessment_attempt_number, 1);
    assert!(second_history_page.next_cursor.is_none());
    let other_student_history = landing_store
        .list_live_student_course_attempt_history(
            token(0xe2),
            course.clone(),
            PageRequest::first(PageSize::new(10).expect("bounded page")),
        )
        .await
        .expect("other Student sees only their empty history");
    assert!(other_student_history.items.is_empty());
    let other_student_progress = landing_store
        .list_live_student_course_progress(token(0xe2), course.clone())
        .await
        .expect("other Student's self-only Course Progress");
    assert_eq!(other_student_progress[0].assessment_attempt_count, 0);
    assert!(
        landing_store
            .list_live_student_course_progress(token(0xe3), course.clone())
            .await
            .is_err(),
        "nonmember cannot read Course Progress",
    );
    assert_eq!(
        landing_store
            .get_live_student_latest_feedback_attempt(token(0xe1))
            .await
            .expect("unsubmitted Attempts have no released feedback"),
        None,
    );

    // Student Work fixture for the same-open-revision Response Stats aggregation and the
    // authenticated cumulative Question display-duration checkpoint.
    let mut practice_fixture = admin.begin().await.expect("Response Stats fixture");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *practice_fixture)
        .await
        .expect("private Response Stats fixture role");
    sqlx::query(
        "INSERT INTO ple_private.object_record ( \
             object_record_id, object_address, object_storage_area, object_data_class, \
             sha256, size_bytes, media_type, created_at \
         ) VALUES ( \
             $1, jsonb_build_object( \
                 'kind', 'questionSource', \
                 'publishedQuestionRevisionTuple', jsonb_build_object( \
                     'publishedQuestionId', $2, 'revisionNumber', 1 \
                 ), \
                 'objectId', $1 \
         ), 'private-content', 'question-source', decode(repeat('aa', 32), 'hex'), \
             1, 'application/json', transaction_timestamp() \
         ) ON CONFLICT (object_record_id) DO NOTHING",
    )
    .bind(id(0xec10))
    .bind(PUBLISHED_QUESTION)
    .execute(&mut *practice_fixture)
    .await
    .expect("exact private Question source Object Record");
    sqlx::query(
        "INSERT INTO ple_private.question_revision_source_binding ( \
             published_question_id, revision_number, backend, question_format, webwork_pg_path, \
             source_object_record_id, source_object_checksum, created_at \
         ) VALUES ($1, 1, 'ple', 'pleQuestionJson', NULL, $2, repeat('a', 64), clock_timestamp()) \
         ON CONFLICT (published_question_id, revision_number) DO NOTHING",
    )
    .bind(PUBLISHED_QUESTION)
    .bind(id(0xec10))
    .execute(&mut *practice_fixture)
    .await
    .expect("Published Question source binding");
    let snapshot_id: Vec<u8> = sqlx::query_scalar("SELECT decode(repeat('b', 64), 'hex')")
        .fetch_one(&mut *practice_fixture)
        .await
        .expect("Question Entry snapshot digest");
    sqlx::query(
        "INSERT INTO ple_private.assessment_entry_snapshot ( \
             assessment_entry_snapshot_id, entry_kind, scoring_rule, points, \
             published_question_id, question_revision_number, created_at \
         ) VALUES ($1, 'fixed_question', 'normal', 2, $2, 1, clock_timestamp()) \
         ON CONFLICT (assessment_entry_snapshot_id) DO NOTHING",
    )
    .bind(&snapshot_id)
    .bind(PUBLISHED_QUESTION)
    .execute(&mut *practice_fixture)
    .await
    .expect("immutable Question Entry snapshot");
    let delivery_toolchain_id: Uuid = sqlx::query_scalar(
        "SELECT ple_private.ensure_delivery_toolchain( \
             'ple', 'fixture', NULL, NULL, 'fixture', '1', 'not_applicable' \
         )",
    )
    .fetch_one(&mut *practice_fixture)
    .await
    .expect("Question delivery toolchain");
    sqlx::query(
        "INSERT INTO ple_private.assessment_attempt ( \
             course_instance_id, assessment_attempt_id, student_record_id, assessment_id, \
             assessment_attempt_number, started_at, assessment_policy_snapshot_id \
         ) SELECT assessment.course_instance_id, attempts.assessment_attempt_id, $1, \
                    assessment.assessment_id, attempts.attempt_number, attempts.started_at, \
                    assessment.assessment_policy_snapshot_id \
             FROM ple_data.assessment AS assessment \
             CROSS JOIN (VALUES \
                 ($2::uuid, 3, clock_timestamp() - interval '2 minutes'), \
                 ($3::uuid, 4, clock_timestamp() - interval '1 minute') \
             ) AS attempts(assessment_attempt_id, attempt_number, started_at) \
            WHERE assessment.course_instance_id = $4 AND assessment.assessment_id = $5",
    )
    .bind(id(STUDENT_RECORD))
    .bind(id(PRACTICE_ATTEMPT_ONE))
    .bind(id(PRACTICE_ATTEMPT_TWO))
    .bind(&fixture.course_id)
    .bind(&fixture.assessment_id)
    .execute(&mut *practice_fixture)
    .await
    .expect("submitted Response Stats Attempts");
    sqlx::query(
        "INSERT INTO ple_private.issued_question ( \
             course_instance_id, issued_question_id, assessment_attempt_id, assessment_entry_id, \
             assessment_entry_snapshot_id, assessment_content_entry_index, issued_position, \
             published_question_id, revision_number, question_statistics_eligibility \
         ) VALUES \
             ($1, $2, $3, $4, $5, 0, 0, $6, 1, true), \
             ($1, $7, $8, $4, $5, 0, 0, $6, 1, true), \
             ($1, $9, $10, $4, $5, 0, 0, $6, 1, true)",
    )
    .bind(&fixture.course_id)
    .bind(id(0xeb20))
    .bind(id(PROGRESS_ATTEMPT))
    .bind(id(ASSESSMENT_ENTRY))
    .bind(&snapshot_id)
    .bind(PUBLISHED_QUESTION)
    .bind(id(PRACTICE_ISSUED_QUESTION_ONE))
    .bind(id(PRACTICE_ATTEMPT_ONE))
    .bind(id(PRACTICE_ISSUED_QUESTION_TWO))
    .bind(id(PRACTICE_ATTEMPT_TWO))
    .execute(&mut *practice_fixture)
    .await
    .expect("exact-revision Issued Questions");
    sqlx::query(
        "INSERT INTO ple_private.question_attempt ( \
             course_instance_id, question_attempt_id, issued_question_id, issued_at, \
             display_duration_ms, delivery_toolchain_id, rendered_question_sha256 \
         ) VALUES \
             ($1, $2, $3, clock_timestamp(), NULL, $4, decode(repeat('c', 64), 'hex')), \
             ($1, $5, $6, clock_timestamp() - interval '90 seconds', 10000, $4, decode(repeat('d', 64), 'hex')), \
             ($1, $7, $8, clock_timestamp() - interval '45 seconds', NULL, $4, decode(repeat('e', 64), 'hex'))",
    )
    .bind(&fixture.course_id)
    .bind(id(DISPLAY_QUESTION_ATTEMPT))
    .bind(id(0xeb20))
    .bind(delivery_toolchain_id)
    .bind(id(PRACTICE_QUESTION_ATTEMPT_ONE))
    .bind(id(PRACTICE_ISSUED_QUESTION_ONE))
    .bind(id(PRACTICE_QUESTION_ATTEMPT_TWO))
    .bind(id(PRACTICE_ISSUED_QUESTION_TWO))
    .execute(&mut *practice_fixture)
    .await
    .expect("Question Attempts with measured and unmeasured duration");
    sqlx::query(
        "INSERT INTO ple_private.assessment_attempt_saved_response ( \
             course_instance_id, question_attempt_id, student_response, saved_at \
         ) VALUES \
             ($1, $2, '{\"kind\":\"multipleChoice\",\"selected\":[]}'::jsonb, clock_timestamp() - interval '80 seconds'), \
             ($1, $3, '{\"kind\":\"multipleChoice\",\"selected\":[]}'::jsonb, clock_timestamp() - interval '30 seconds')",
    )
    .bind(&fixture.course_id)
    .bind(id(PRACTICE_QUESTION_ATTEMPT_ONE))
    .bind(id(PRACTICE_QUESTION_ATTEMPT_TWO))
    .execute(&mut *practice_fixture)
    .await
    .expect("submitted Response Stats responses");
    sqlx::query(
        "SELECT ple_private.record_direct_automated_grading_result($1, 0.5, clock_timestamp()), \
                ple_private.record_direct_automated_grading_result($2, 1, clock_timestamp())",
    )
    .bind(id(PRACTICE_QUESTION_ATTEMPT_ONE))
    .bind(id(PRACTICE_QUESTION_ATTEMPT_TWO))
    .execute(&mut *practice_fixture)
    .await
    .expect("automated Question outcomes");
    sqlx::query(
        "INSERT INTO ple_private.assessment_submission ( \
             course_instance_id, assessment_submission_id, assessment_attempt_id, submitted_at \
         ) VALUES ($1, $2, $3, clock_timestamp() - interval '50 seconds'), \
                  ($1, $4, $5, clock_timestamp() - interval '10 seconds')",
    )
    .bind(&fixture.course_id)
    .bind(id(PRACTICE_SUBMISSION_ONE))
    .bind(id(PRACTICE_ATTEMPT_ONE))
    .bind(id(PRACTICE_SUBMISSION_TWO))
    .bind(id(PRACTICE_ATTEMPT_TWO))
    .execute(&mut *practice_fixture)
    .await
    .expect("Response Stats Assessment submissions");
    sqlx::query(
        "UPDATE ple_private.question_attempt AS question_attempt \
            SET finalized_at = submission.submitted_at \
           FROM ple_private.issued_question AS issued \
           JOIN ple_private.assessment_submission AS submission \
             ON submission.course_instance_id = issued.course_instance_id \
            AND submission.assessment_attempt_id = issued.assessment_attempt_id \
          WHERE question_attempt.course_instance_id = issued.course_instance_id \
            AND question_attempt.issued_question_id = issued.issued_question_id \
            AND question_attempt.question_attempt_id IN ($1, $2)",
    )
    .bind(id(PRACTICE_QUESTION_ATTEMPT_ONE))
    .bind(id(PRACTICE_QUESTION_ATTEMPT_TWO))
    .execute(&mut *practice_fixture)
    .await
    .expect("finalized Question Attempts");
    sqlx::query(
        "UPDATE ple_private.assessment_attempt_saved_response AS response \
            SET finalized_at = submission.submitted_at, \
                assessment_submission_id = submission.assessment_submission_id \
           FROM ple_private.question_attempt AS question_attempt \
           JOIN ple_private.issued_question AS issued \
             ON issued.course_instance_id = question_attempt.course_instance_id \
            AND issued.issued_question_id = question_attempt.issued_question_id \
           JOIN ple_private.assessment_submission AS submission \
             ON submission.course_instance_id = issued.course_instance_id \
            AND submission.assessment_attempt_id = issued.assessment_attempt_id \
          WHERE response.course_instance_id = question_attempt.course_instance_id \
            AND response.question_attempt_id = question_attempt.question_attempt_id \
            AND question_attempt.question_attempt_id IN ($1, $2)",
    )
    .bind(id(PRACTICE_QUESTION_ATTEMPT_ONE))
    .bind(id(PRACTICE_QUESTION_ATTEMPT_TWO))
    .execute(&mut *practice_fixture)
    .await
    .expect("finalized saved responses");
    practice_fixture
        .commit()
        .await
        .expect("Response Stats fixture commit");

    assert_eq!(
        store
            .checkpoint_student_question_display_duration(
                token(0xe1),
                question_model::AssessmentAttemptId::from_uuid(id(PROGRESS_ATTEMPT)),
                1,
                1_200,
            )
            .await
            .expect("authorized duration checkpoint"),
        1_200,
    );
    assert_eq!(
        store
            .checkpoint_student_question_display_duration(
                token(0xe1),
                question_model::AssessmentAttemptId::from_uuid(id(PROGRESS_ATTEMPT)),
                1,
                800,
            )
            .await
            .expect("stale duration checkpoint remains idempotent"),
        1_200,
    );
    assert!(
        store
            .checkpoint_student_question_display_duration(
                token(0xe2),
                question_model::AssessmentAttemptId::from_uuid(id(PROGRESS_ATTEMPT)),
                1,
                1_300,
            )
            .await
            .is_err()
    );
    let mut finalize_duration = admin.begin().await.expect("finalize Question duration");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *finalize_duration)
        .await
        .expect("private Question finalization role");
    sqlx::query(
        "UPDATE ple_private.question_attempt SET finalized_at = clock_timestamp() \
         WHERE question_attempt_id = $1",
    )
    .bind(id(DISPLAY_QUESTION_ATTEMPT))
    .execute(&mut *finalize_duration)
    .await
    .expect("finalize display-duration Question");
    finalize_duration
        .commit()
        .await
        .expect("Question duration finalization commit");
    assert!(
        store
            .checkpoint_student_question_display_duration(
                token(0xe1),
                question_model::AssessmentAttemptId::from_uuid(id(PROGRESS_ATTEMPT)),
                1,
                1_300,
            )
            .await
            .is_err()
    );

    let practice = landing_store
        .list_live_student_course_response_stats(token(0xe1), course.clone())
        .await
        .expect("self-only exact-revision Response Stats");
    assert_eq!(
        practice.len(),
        1,
        "the exact immutable Question Revision groups once"
    );
    assert_eq!(practice[0].full_credit_attempt_count, 1);
    assert_eq!(practice[0].partial_credit_attempt_count, 1);
    assert_eq!(practice[0].incorrect_attempt_count, 0);
    assert_eq!(practice[0].unanswered_attempt_count, 0);
    assert_eq!(practice[0].disclosed_attempt_count, 2);
    assert_eq!(practice[0].not_full_credit_count, 1);
    assert_eq!(practice[0].display_duration_sample_count, 1);
    assert_eq!(practice[0].average_display_duration_ms, Some(10_000.0));
    assert_eq!(
        practice[0].relevant_assessment_attempt_id.as_uuid(),
        id(PRACTICE_ATTEMPT_ONE)
    );
    assert!(
        landing_store
            .list_live_student_course_response_stats(token(0xe2), course.clone())
            .await
            .expect("other Student gets only their own Response Stats")
            .is_empty()
    );
    assert!(
        landing_store
            .list_live_student_course_response_stats(token(0xe3), course.clone())
            .await
            .is_err()
    );
    assert_eq!(
        landing_store
            .get_live_student_latest_feedback_attempt(token(0xe1))
            .await
            .expect("Student's latest released feedback Attempt"),
        Some(question_model::AssessmentAttemptId::from_uuid(id(
            PRACTICE_ATTEMPT_TWO
        ))),
    );
    assert_eq!(
        landing_store
            .get_live_student_latest_feedback_attempt(token(0xe2))
            .await
            .expect("other Student has no released feedback Attempt"),
        None,
    );
    assert_eq!(
        landing_store
            .get_live_student_latest_feedback_attempt(token(0xe3))
            .await
            .expect("a Student with no Course has no Latest Feedback target"),
        None,
    );

    let mut tx = application.begin().await.expect("application transaction");
    sqlx::query("SET LOCAL ROLE ple_auth")
        .execute(&mut *tx)
        .await
        .expect("authentication role");
    sqlx::query("SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))")
        .bind(token(0xe1).to_string())
        .fetch_one(&mut *tx)
        .await
        .expect("install Student session");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("application role");
    let row = sqlx::query(
        "SELECT start_decision, assessment_attempt_limit AS attempt_limit, late_work_rule, display_time_zone, \
                evaluated_at < available_at AS evaluation_before_available, \
                available_at < due_at AS available_before_due, \
                due_at < closes_at AS due_before_close \
           FROM ple_api.read_student_assessment_access($1, $2)",
    )
    .bind(&course_instance_id)
    .bind(&assessment_id)
    .fetch_one(&mut *tx)
    .await
    .expect("complete Assessment Access projection");
    assert_eq!(
        row.try_get::<String, _>("start_decision")
            .expect("start decision"),
        "attempt_limit_reached"
    );
    assert_eq!(
        row.try_get::<i32, _>("attempt_limit")
            .expect("Attempt limit"),
        2
    );
    assert_eq!(
        row.try_get::<String, _>("late_work_rule")
            .expect("late-work rule"),
        "reject"
    );
    assert_eq!(
        row.try_get::<String, _>("display_time_zone")
            .expect("display time zone"),
        "America/Denver"
    );
    assert!(
        !row.try_get::<bool, _>("evaluation_before_available")
            .expect("available boundary follows the Student accommodation")
    );
    assert!(
        row.try_get::<bool, _>("available_before_due")
            .expect("due boundary")
    );
    assert!(
        row.try_get::<bool, _>("due_before_close")
            .expect("close boundary")
    );
    tx.commit().await.expect("application read commit");

    let mut exact = admin.begin().await.expect("exact-boundary transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *exact)
        .await
        .expect("private exact-boundary role");
    let row = sqlx::query(
        "SELECT \
         ple_private.assessment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject'::ple_data.late_work_rule, \
             '2026-01-01 09:59:59.999+00') AS before_available, \
         ple_private.assessment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject'::ple_data.late_work_rule, \
             '2026-01-01 10:00:00+00') AS at_available, \
         ple_private.assessment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject'::ple_data.late_work_rule, \
             '2026-01-01 20:00:00+00') AS at_due, \
         ple_private.assessment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject'::ple_data.late_work_rule, \
             '2026-01-01 20:00:00.001+00') AS after_due, \
         ple_private.assessment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject'::ple_data.late_work_rule, \
             '2026-01-01 23:00:00+00') AS at_close, \
         ple_private.assessment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 2, 'reject'::ple_data.late_work_rule, \
             '2026-01-01 20:00:00.001+00') AS limit_before_late",
    )
    .fetch_one(&mut *exact)
    .await
    .expect("exact Assessment Start Decision boundaries");
    assert_eq!(
        row.try_get::<String, _>("before_available")
            .expect("before available"),
        "not_yet_available"
    );
    assert_eq!(
        row.try_get::<String, _>("at_available")
            .expect("at available"),
        "may_start"
    );
    assert_eq!(
        row.try_get::<String, _>("at_due").expect("at due"),
        "may_start"
    );
    assert_eq!(
        row.try_get::<String, _>("after_due").expect("after due"),
        "late_work_refused"
    );
    assert_eq!(
        row.try_get::<String, _>("at_close").expect("at close"),
        "closed"
    );
    assert_eq!(
        row.try_get::<String, _>("limit_before_late")
            .expect("Attempt limit before late work"),
        "attempt_limit_reached"
    );
    exact.commit().await.expect("exact-boundary commit");
}
