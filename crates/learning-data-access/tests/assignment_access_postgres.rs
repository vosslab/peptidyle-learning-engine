#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for server-owned Student Assignment Access.

use learning_data_access::postgres::{
    PostgresLiveAssignmentDeliveryStore, PostgresLiveStudentCourseLandingStore, lazy_pool,
};
use learning_data_access::{
    AssignmentStartDecision, LiveAssignmentDeliveryStore, LiveStudentCourseLandingStore,
    SessionTokenHash,
};
use question_model::{AssignmentReference, CourseInstanceReference};
use sqlx::Row;
use uuid::Uuid;

const INSTRUCTOR: u128 = 0xea01;
const STUDENT: u128 = 0xea02;
const STUDENT_SESSION: u128 = 0xea03;
const COURSE: u128 = 0xeb01;
const STUDENT_RECORD: u128 = 0xeb02;
const STUDENT_MEMBERSHIP: u128 = 0xeb03;
const INSTRUCTOR_MEMBERSHIP: u128 = 0xeb04;
const OTHER_STUDENT: u128 = 0xeb05;
const OTHER_STUDENT_RECORD: u128 = 0xeb06;
const OTHER_STUDENT_MEMBERSHIP: u128 = 0xeb07;
const BLUEPRINT: u128 = 0xec01;
const BLUEPRINT_MODULE: u128 = 0xec02;
const BLUEPRINT_ASSIGNMENT: u128 = 0xec03;
const ASSIGNMENT: u128 = 0xed01;
const ASSIGNMENT_ENTRY: u128 = 0xed02;
const OTHER_STUDENT_ACCOMMODATION: u128 = 0xed03;
const REFERENCE_NUMBER: i64 = 920_001;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn token() -> SessionTokenHash {
    SessionTokenHash::compute(&[0xe1; 32])
}

async fn seed(admin: &sqlx::postgres::PgPool) {
    let mut tx = admin.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("private fixture role");
    sqlx::query(
        "INSERT INTO ple_private.account (account_id, product_role, created_at) \
         VALUES ($1, 'instructor', clock_timestamp()), \
                ($2, 'student', clock_timestamp()), \
                ($3, 'student', clock_timestamp())",
    )
    .bind(id(INSTRUCTOR))
    .bind(id(STUDENT))
    .bind(id(OTHER_STUDENT))
    .execute(&mut *tx)
    .await
    .expect("fixture Accounts");
    sqlx::query(
        "UPDATE ple_private.account_time_zone SET time_zone = 'America/Denver' \
         WHERE account_id = $1",
    )
    .bind(id(STUDENT))
    .execute(&mut *tx)
    .await
    .expect("Student Account Time Zone");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, product_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'student', decode($3, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour')",
    )
    .bind(id(STUDENT_SESSION))
    .bind(id(STUDENT))
    .bind(token().to_string())
    .execute(&mut *tx)
    .await
    .expect("Student Authenticated Session");

    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("data fixture role");
    sqlx::query(
        "INSERT INTO ple_data.published_question (question_id, created_at) \
         VALUES ('BCDEFG0', clock_timestamp())",
    )
    .execute(&mut *tx)
    .await
    .expect("Published Question");
    sqlx::query(
        "INSERT INTO ple_data.question_revision \
         (question_id, revision_number, backend, question_type, published_at) \
         VALUES ('BCDEFG0', 1, 'ple', 'multipleChoice', clock_timestamp())",
    )
    .execute(&mut *tx)
    .await
    .expect("Question Revision");

    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *tx)
        .await
        .expect("API fixture role");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_course \
         (blueprint_id, reference_number, owner_account_id, short_name, long_name, \
          metadata_etag, created_at) OVERRIDING SYSTEM VALUE \
         VALUES ($1, $2, $3, 'ACCESS', 'Assignment Access Blueprint', \
                 '00000000-0000-0000-0000-00000000ec04', clock_timestamp())",
    )
    .bind(id(BLUEPRINT))
    .bind(REFERENCE_NUMBER)
    .bind(id(INSTRUCTOR))
    .execute(&mut *tx)
    .await
    .expect("Blueprint Course");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_course_revision \
         (blueprint_course_reference_number, blueprint_revision_number, content, \
          content_checksum, saved_at) \
         VALUES ($1, 1, '{}'::jsonb, decode(repeat('0', 64), 'hex'), clock_timestamp())",
    )
    .bind(REFERENCE_NUMBER)
    .execute(&mut *tx)
    .await
    .expect("Blueprint Revision");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_module \
         (blueprint_course_reference_number, blueprint_revision_number, \
          blueprint_module_reference, module_position) VALUES ($1, 1, $2, 1)",
    )
    .bind(REFERENCE_NUMBER)
    .bind(id(BLUEPRINT_MODULE))
    .execute(&mut *tx)
    .await
    .expect("Blueprint Module");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_assignment \
         (blueprint_course_reference_number, blueprint_revision_number, \
          blueprint_module_reference, blueprint_assignment_reference, assignment_position) \
         VALUES ($1, 1, $2, $3, 1)",
    )
    .bind(REFERENCE_NUMBER)
    .bind(id(BLUEPRINT_MODULE))
    .bind(id(BLUEPRINT_ASSIGNMENT))
    .execute(&mut *tx)
    .await
    .expect("Blueprint Assignment");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_event \
         (blueprint_course_reference_number, blueprint_revision_number, actor_account_id, \
          request_checksum, occurred_at) \
         VALUES ($1, 1, $2, decode(repeat('ec', 32), 'hex'), clock_timestamp())",
    )
    .bind(REFERENCE_NUMBER)
    .bind(id(INSTRUCTOR))
    .execute(&mut *tx)
    .await
    .expect("Blueprint Revision Event");
    sqlx::query(
        "INSERT INTO ple_data.course_instance \
         (course_id, reference_number, blueprint_course_reference_number, \
          blueprint_revision_number, assigned_instructor_account_id, assigned_instructor_role, \
          course_short_name, course_long_name, term_starts_on, term_ends_on, created_at) \
         OVERRIDING SYSTEM VALUE \
         VALUES ($1, $2, $2, 1, $3, 'instructor', 'ACCESS', \
                 'Assignment Access Course', current_date, current_date + 1, clock_timestamp())",
    )
    .bind(id(COURSE))
    .bind(REFERENCE_NUMBER)
    .bind(id(INSTRUCTOR))
    .execute(&mut *tx)
    .await
    .expect("Course Instance");
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (membership_id, course_id, account_id, role, student_record_id, joined_at) \
         VALUES ($1, $2, $3, 'instructor', NULL, clock_timestamp())",
    )
    .bind(id(INSTRUCTOR_MEMBERSHIP))
    .bind(id(COURSE))
    .bind(id(INSTRUCTOR))
    .execute(&mut *tx)
    .await
    .expect("Instructor Course Membership");
    sqlx::query(
        "INSERT INTO ple_data.student_record \
         (student_record_id, course_id, student_account_id, created_at) \
         VALUES ($1, $2, $3, clock_timestamp()), \
                ($4, $2, $5, clock_timestamp())",
    )
    .bind(id(STUDENT_RECORD))
    .bind(id(COURSE))
    .bind(id(STUDENT))
    .bind(id(OTHER_STUDENT_RECORD))
    .bind(id(OTHER_STUDENT))
    .execute(&mut *tx)
    .await
    .expect("Student Record");
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (membership_id, course_id, account_id, role, student_record_id, joined_at) \
         VALUES ($1, $2, $3, 'student', $4, clock_timestamp())",
    )
    .bind(id(STUDENT_MEMBERSHIP))
    .bind(id(COURSE))
    .bind(id(STUDENT))
    .bind(id(STUDENT_RECORD))
    .execute(&mut *tx)
    .await
    .expect("Student Course Membership");
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (membership_id, course_id, account_id, role, student_record_id, joined_at) \
         VALUES ($1, $2, $3, 'student', $4, clock_timestamp())",
    )
    .bind(id(OTHER_STUDENT_MEMBERSHIP))
    .bind(id(COURSE))
    .bind(id(OTHER_STUDENT))
    .bind(id(OTHER_STUDENT_RECORD))
    .execute(&mut *tx)
    .await
    .expect("other Student Course Membership");

    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("Assignment fixture role");
    sqlx::query(
        "INSERT INTO ple_data.assignment \
         (assignment_id, reference_number, course_id, source_blueprint_course_reference_number, \
          source_blueprint_revision_number, source_blueprint_assignment_reference, created_at, \
          updated_at, assignment_title, assignment_instructions, available_at, due_at, closes_at, \
          assignment_attempt_time_limit_seconds, attempt_limit, late_work_rule, \
          assignment_completion_rule, assignment_attempt_grade_rule, \
          assignment_attempt_continuation_rule, question_pool_reuse_rule, question_variation_rule, \
          assignment_attempt_resume_rule, assignment_question_display_rule, \
          assignment_navigation_rule, assignment_question_order_rule, feedback_score, \
          feedback_per_item_correctness, feedback_submitted_response, feedback_question_feedback, \
          feedback_question_answer, feedback_question_answer_explanation, \
          feedback_class_statistics, assignment_status) OVERRIDING SYSTEM VALUE \
         VALUES ($1, $2, $3, $2, 1, $4, clock_timestamp(), clock_timestamp(), \
                 'Server-owned Assignment Access', 'Read the policy before starting.', \
                 clock_timestamp() + interval '1 hour', \
                 clock_timestamp() + interval '2 hours', \
                 clock_timestamp() + interval '3 hours', 600, 2, 'reject', 'answer_all', \
                 'highest', 'unlimited', 'reuse_selection', 'new_variation', 'resumable', \
                 'one_question_at_a_time', 'free_navigation', 'shuffled', 'after_submit', \
                 'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit', \
                 'after_submit', 'released')",
    )
    .bind(id(ASSIGNMENT))
    .bind(REFERENCE_NUMBER)
    .bind(id(COURSE))
    .bind(id(BLUEPRINT_ASSIGNMENT))
    .execute(&mut *tx)
    .await
    .expect("released Assignment");
    sqlx::query(
        "INSERT INTO ple_data.assignment_entry \
         (assignment_entry_id, assignment_id, authored_position, entry_kind, availability, \
          scoring_rule, question_id, question_revision_number, points_possible) \
         VALUES ($1, $2, 0, 'fixed_question', 'available', 'normal', 'BCDEFG0', 1, 2)",
    )
    .bind(id(ASSIGNMENT_ENTRY))
    .bind(id(ASSIGNMENT))
    .execute(&mut *tx)
    .await
    .expect("Assignment Entry");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("private accommodation role");
    sqlx::query(
        "INSERT INTO ple_private.student_assignment_accommodation( \
             accommodation_id, student_record_id, assignment_id, available_at, due_at, \
             closes_at, assignment_attempt_time_limit_seconds, attempt_limit, created_at \
         ) VALUES ( \
             $1, $2, $3, clock_timestamp() - interval '1 hour', \
             clock_timestamp() + interval '1 hour', clock_timestamp() + interval '2 hours', \
             60, 1, clock_timestamp() \
         )",
    )
    .bind(id(OTHER_STUDENT_ACCOMMODATION))
    .bind(id(OTHER_STUDENT_RECORD))
    .bind(id(ASSIGNMENT))
    .execute(&mut *tx)
    .await
    .expect("other Student accommodation");
    tx.commit().await.expect("fixture commit");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
async fn access_reader_projects_one_authoritative_decision_and_effective_policy() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let migration_url = runtime.migration_url().expose();
    let admin = lazy_pool(migration_url).expect("migration pool");
    seed(&admin).await;

    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let application = lazy_pool(&application_url).expect("application pool");
    let store = PostgresLiveAssignmentDeliveryStore::new(application.clone());
    let course = CourseInstanceReference::new(REFERENCE_NUMBER as u64).expect("Course reference");
    let assignment =
        AssignmentReference::new(REFERENCE_NUMBER as u64).expect("Assignment reference");
    let access = store
        .live_assignment_access(token(), course, assignment)
        .await
        .expect("authorized Student Assignment Access");
    assert_eq!(
        access.decision.start_decision,
        AssignmentStartDecision::NotYetAvailable
    );
    assert_eq!(
        access.decision.public_reason.as_deref(),
        Some("This Assignment is not yet available.")
    );
    assert_eq!(access.question_count, 1);
    assert_eq!(access.points_possible, 2.0);
    assert_eq!(access.decision.time_limit_seconds, Some(600));
    assert_eq!(access.decision.attempt_limit, Some(2));

    let landing_store = PostgresLiveStudentCourseLandingStore::new(application.clone());
    let landing = landing_store
        .list_released_live_student_assignments(token(), course)
        .await
        .expect("authorized Student Assignment landing");
    assert_eq!(landing.len(), 1, "scheduled Assignment remains visible");
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

    let mut tx = application.begin().await.expect("application transaction");
    sqlx::query("SET LOCAL ROLE ple_auth")
        .execute(&mut *tx)
        .await
        .expect("authentication role");
    sqlx::query("SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))")
        .bind(token().to_string())
        .fetch_one(&mut *tx)
        .await
        .expect("install Student session");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("application role");
    let row = sqlx::query(
        "SELECT start_decision, attempt_limit, late_work_rule, display_time_zone, \
                evaluated_at < available_at AS evaluation_before_available, \
                available_at < due_at AS available_before_due, \
                due_at < closes_at AS due_before_close \
           FROM ple_api.read_student_assignment_access($1, $2)",
    )
    .bind(REFERENCE_NUMBER)
    .bind(REFERENCE_NUMBER)
    .fetch_one(&mut *tx)
    .await
    .expect("complete Assignment Access projection");
    assert_eq!(
        row.try_get::<String, _>("start_decision")
            .expect("start decision"),
        "not_yet_available"
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
        row.try_get::<bool, _>("evaluation_before_available")
            .expect("available boundary")
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
         ple_private.assignment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject', \
             '2026-01-01 09:59:59.999+00') AS before_available, \
         ple_private.assignment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject', \
             '2026-01-01 10:00:00+00') AS at_available, \
         ple_private.assignment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject', \
             '2026-01-01 20:00:00+00') AS at_due, \
         ple_private.assignment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject', \
             '2026-01-01 20:00:00.001+00') AS after_due, \
         ple_private.assignment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject', \
             '2026-01-01 23:00:00+00') AS at_close, \
         ple_private.assignment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 2, 'reject', \
             '2026-01-01 20:00:00.001+00') AS limit_before_late",
    )
    .fetch_one(&mut *exact)
    .await
    .expect("exact Assignment Start Decision boundaries");
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
