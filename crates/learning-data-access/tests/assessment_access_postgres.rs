#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for server-owned Student Assessment Access.

use learning_data_access::postgres::{
    PostgresLiveAssessmentDeliveryStore, PostgresLiveStudentCourseLandingStore, lazy_pool,
};
use learning_data_access::{
    AssessmentStartDecision, LiveAssessmentDeliveryStore, LiveStudentCourseLandingStore,
    SessionTokenHash,
};
use question_model::{AssessmentReference, AssessmentType, CourseInstanceReference};
use sqlx::Row;
use uuid::Uuid;

const INSTRUCTOR: u128 = 0xea01;
const STUDENT: u128 = 0xea02;
const STUDENT_SESSION: u128 = 0xea03;
const NONMEMBER: u128 = 0xea04;
const NONMEMBER_SESSION: u128 = 0xea05;
const SYSADMIN: u128 = 0xea06;
const SYSADMIN_SESSION: u128 = 0xea07;
const COURSE: u128 = 0xeb01;
const STUDENT_RECORD: u128 = 0xeb02;
const STUDENT_MEMBERSHIP: u128 = 0xeb03;
const INSTRUCTOR_MEMBERSHIP: u128 = 0xeb04;
const OTHER_STUDENT: u128 = 0xeb05;
const OTHER_STUDENT_RECORD: u128 = 0xeb06;
const OTHER_STUDENT_MEMBERSHIP: u128 = 0xeb07;
const OTHER_STUDENT_SESSION: u128 = 0xeb08;
const OTHER_COURSE: u128 = 0xeb09;
const OTHER_COURSE_STUDENT_RECORD: u128 = 0xeb0a;
const OTHER_COURSE_STUDENT_MEMBERSHIP: u128 = 0xeb0b;
const OTHER_COURSE_INSTRUCTOR_MEMBERSHIP: u128 = 0xeb0c;
const BLUEPRINT: u128 = 0xec01;
const BLUEPRINT_MODULE: u128 = 0xec02;
const BLUEPRINT_ASSESSMENT: u128 = 0xec03;
const ASSESSMENT: u128 = 0xed01;
const ASSESSMENT_ENTRY: u128 = 0xed02;
const OTHER_STUDENT_ACCOMMODATION: u128 = 0xed03;
const REFERENCE_NUMBER: i64 = 920_001;
const COURSE_REFERENCE: &str = "CI92ABCD";
const OTHER_COURSE_REFERENCE: &str = "CI92ABCE";
const ASSESSMENT_REFERENCE_PLACEHOLDER: &str = "A92ABCD";

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn token(marker: u8) -> SessionTokenHash {
    SessionTokenHash::compute(&[marker; 32])
}

async fn authenticated_student_record_ownership(
    application: &sqlx::postgres::PgPool,
    session_token: SessionTokenHash,
    course_id: Uuid,
    student_record_id: Uuid,
) -> bool {
    let mut transaction = application.begin().await.expect("application transaction");
    sqlx::query("SET LOCAL ROLE ple_auth")
        .execute(&mut *transaction)
        .await
        .expect("authentication role");
    sqlx::query("SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))")
        .bind(session_token.to_string())
        .fetch_one(&mut *transaction)
        .await
        .expect("authenticated fixture session");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("application role");
    let owns_record: bool =
        sqlx::query_scalar("SELECT ple_api.current_session_account_owns_student_record($1, $2)")
            .bind(course_id)
            .bind(student_record_id)
            .fetch_one(&mut *transaction)
            .await
            .expect("Student Work ownership decision");
    transaction.commit().await.expect("application read commit");
    owns_record
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
                ($3, 'student', clock_timestamp()), \
                ($4, 'student', clock_timestamp()), \
                ($5, 'sysadmin', clock_timestamp())",
    )
    .bind(id(INSTRUCTOR))
    .bind(id(STUDENT))
    .bind(id(OTHER_STUDENT))
    .bind(id(NONMEMBER))
    .bind(id(SYSADMIN))
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
                 clock_timestamp() + interval '1 hour'), \
                ($4, $5, 'student', decode($6, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour'), \
                ($7, $8, 'student', decode($9, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour'), \
                ($10, $11, 'sysadmin', decode($12, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour')",
    )
    .bind(id(STUDENT_SESSION))
    .bind(id(STUDENT))
    .bind(token(0xe1).to_string())
    .bind(id(OTHER_STUDENT_SESSION))
    .bind(id(OTHER_STUDENT))
    .bind(token(0xe2).to_string())
    .bind(id(NONMEMBER_SESSION))
    .bind(id(NONMEMBER))
    .bind(token(0xe3).to_string())
    .bind(id(SYSADMIN_SESSION))
    .bind(id(SYSADMIN))
    .bind(token(0xe4).to_string())
    .execute(&mut *tx)
    .await
    .expect("Student Authenticated Session");

    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("data fixture role");
    sqlx::query(
        "INSERT INTO ple_data.published_question (question_id, created_at) \
         VALUES ('BCDEXFG0', clock_timestamp())",
    )
    .execute(&mut *tx)
    .await
    .expect("Published Question");
    sqlx::query(
        "INSERT INTO ple_data.question_revision \
         (question_id, revision_number, backend, question_type, published_at) \
         VALUES ('BCDEXFG0', 1, 'ple', 'multipleChoice', clock_timestamp())",
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
         VALUES ($1, $2, $3, 'ACCESS', 'Assessment Access Blueprint', \
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
        "INSERT INTO ple_data.blueprint_revision_assessment \
         (blueprint_course_reference_number, blueprint_revision_number, \
          blueprint_module_reference, blueprint_assessment_reference, assessment_position) \
         VALUES ($1, 1, $2, $3, 1)",
    )
    .bind(REFERENCE_NUMBER)
    .bind(id(BLUEPRINT_MODULE))
    .bind(id(BLUEPRINT_ASSESSMENT))
    .execute(&mut *tx)
    .await
    .expect("Blueprint Assessment");
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
         (course_id, reference_number, public_reference, source_kind, blueprint_course_reference_number, \
          blueprint_revision_number, assigned_instructor_account_id, assigned_instructor_role, \
          course_short_name, course_long_name, term_starts_on, term_ends_on, created_at) \
         OVERRIDING SYSTEM VALUE \
         VALUES ($1, $2, $4, 'adopted', $2, 1, $3, 'instructor', 'ACCESS', \
                 'Assessment Access Course', current_date, current_date + 1, clock_timestamp())",
    )
    .bind(id(COURSE))
    .bind(REFERENCE_NUMBER)
    .bind(id(INSTRUCTOR))
    .bind(COURSE_REFERENCE)
    .execute(&mut *tx)
    .await
    .expect("Course Instance");
    sqlx::query(
        "INSERT INTO ple_data.course_instance \
         (course_id, reference_number, public_reference, source_kind, blueprint_course_reference_number, \
          blueprint_revision_number, assigned_instructor_account_id, assigned_instructor_role, \
          course_short_name, course_long_name, term_starts_on, term_ends_on, created_at) \
         OVERRIDING SYSTEM VALUE \
         VALUES ($1, $2, $5, 'adopted', $3, 1, $4, 'instructor', 'ACCESS-OTHER', \
                 'Other Course for exact Student Work scope', current_date, current_date + 1, \
                 clock_timestamp())",
    )
    .bind(id(OTHER_COURSE))
    .bind(REFERENCE_NUMBER + 1)
    .bind(REFERENCE_NUMBER)
    .bind(id(INSTRUCTOR))
    .bind(OTHER_COURSE_REFERENCE)
    .execute(&mut *tx)
    .await
    .expect("other Course Instance");
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
        "INSERT INTO ple_data.course_membership \
         (membership_id, course_id, account_id, role, student_record_id, joined_at) \
         VALUES ($1, $2, $3, 'instructor', NULL, clock_timestamp())",
    )
    .bind(id(OTHER_COURSE_INSTRUCTOR_MEMBERSHIP))
    .bind(id(OTHER_COURSE))
    .bind(id(INSTRUCTOR))
    .execute(&mut *tx)
    .await
    .expect("other Course Instructor Membership");
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
        "INSERT INTO ple_data.student_record \
         (student_record_id, course_id, student_account_id, created_at) \
         VALUES ($1, $2, $3, clock_timestamp())",
    )
    .bind(id(OTHER_COURSE_STUDENT_RECORD))
    .bind(id(OTHER_COURSE))
    .bind(id(STUDENT))
    .execute(&mut *tx)
    .await
    .expect("same Account other Course Student Record");
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
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (membership_id, course_id, account_id, role, student_record_id, joined_at) \
         VALUES ($1, $2, $3, 'student', $4, clock_timestamp())",
    )
    .bind(id(OTHER_COURSE_STUDENT_MEMBERSHIP))
    .bind(id(OTHER_COURSE))
    .bind(id(STUDENT))
    .bind(id(OTHER_COURSE_STUDENT_RECORD))
    .execute(&mut *tx)
    .await
    .expect("same Account other Course Student Membership");

    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("Assessment fixture role");
    sqlx::query(
        "INSERT INTO ple_data.assessment \
         (assessment_id, reference_number, public_reference, course_id, origin_kind, source_blueprint_course_reference_number, \
          source_blueprint_revision_number, source_blueprint_assessment_reference, created_at, \
          updated_at, assessment_type, assessment_title, assessment_instructions, available_at, due_at, closes_at, \
          assessment_attempt_time_limit_seconds, assessment_attempt_limit, late_work_rule, \
          assessment_attempt_grade_rule, question_pool_reuse_rule, question_variation_rule, \
          assessment_attempt_resume_rule, assessment_question_display_rule, \
          assessment_navigation_rule, assessment_question_order_rule, feedback_score, \
          feedback_per_item_correctness, feedback_submitted_response, \
          feedback_question_answer, feedback_question_answer_explanation, \
          feedback_class_statistics, assessment_status) OVERRIDING SYSTEM VALUE \
         VALUES ($1, $2, $5, $3, 'adopted', $2, 1, $4, clock_timestamp(), clock_timestamp(), 'regular_assignment', \
                 'Server-owned Assessment Access', 'Read the policy before starting.', \
                 clock_timestamp() + interval '1 hour', \
                 clock_timestamp() + interval '2 hours', \
                 clock_timestamp() + interval '3 hours', 600, 2, 'reject', \
                 'highest', 'reuse_selection', 'new_variation', 'resumable', \
                 'one_question_at_a_time', 'free_navigation', 'shuffled', 'after_submit', \
                 'after_submit', 'after_submit', 'after_submit', 'after_submit', \
                 'after_submit', 'released')",
    )
    .bind(id(ASSESSMENT))
    .bind(REFERENCE_NUMBER)
    .bind(id(COURSE))
    .bind(id(BLUEPRINT_ASSESSMENT))
    .bind(ASSESSMENT_REFERENCE_PLACEHOLDER)
    .execute(&mut *tx)
    .await
    .expect("released Assessment");
    sqlx::query(
        "INSERT INTO ple_data.assessment_entry \
         (assessment_entry_id, assessment_id, authored_position, entry_kind, availability, \
          scoring_rule, question_id, question_revision_number, points_possible) \
         VALUES ($1, $2, 0, 'fixed_question', 'available', 'normal', 'BCDEXFG0', 1, 2)",
    )
    .bind(id(ASSESSMENT_ENTRY))
    .bind(id(ASSESSMENT))
    .execute(&mut *tx)
    .await
    .expect("Assessment Entry");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("private accommodation role");
    sqlx::query(
        "INSERT INTO ple_private.student_assessment_accommodation( \
             accommodation_id, student_record_id, assessment_id, available_at, due_at, \
             closes_at, assessment_attempt_time_limit_seconds, assessment_attempt_limit, created_at \
         ) VALUES ( \
             $1, $2, $3, clock_timestamp() - interval '1 hour', \
             clock_timestamp() + interval '1 hour', clock_timestamp() + interval '2 hours', \
             60, 1, clock_timestamp() \
         )",
    )
    .bind(id(OTHER_STUDENT_ACCOMMODATION))
    .bind(id(OTHER_STUDENT_RECORD))
    .bind(id(ASSESSMENT))
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
    let mut route_transaction = admin.begin().await.expect("route fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *route_transaction)
        .await
        .expect("route fixture role");
    let (course_public_reference, assessment_public_reference): (String, String) = sqlx::query_as(
        "SELECT course.public_reference, assessment.public_reference \
         FROM ple_data.course_instance AS course \
         JOIN ple_data.assessment AS assessment ON assessment.course_id = course.course_id \
         WHERE course.course_id = $1 AND assessment.assessment_id = $2",
    )
    .bind(id(COURSE))
    .bind(id(ASSESSMENT))
    .fetch_one(&mut *route_transaction)
    .await
    .expect("generated Student Assessment route references");
    route_transaction
        .commit()
        .await
        .expect("route fixture commit");
    let course = CourseInstanceReference::new(&course_public_reference).expect("Course reference");
    let assessment =
        AssessmentReference::new(&assessment_public_reference).expect("Assessment reference");

    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let application = lazy_pool(&application_url).expect("application pool");
    // ASVS 8.2.2 and 8.3.1: an authenticated app session has access only to
    // its exact Student Work identity, at the trusted database boundary.
    assert!(
        authenticated_student_record_ownership(
            &application,
            token(0xe1),
            id(COURSE),
            id(STUDENT_RECORD),
        )
        .await,
        "the authenticated Student may access their exact Student Work record",
    );
    assert!(
        !authenticated_student_record_ownership(
            &application,
            token(0xe2),
            id(COURSE),
            id(STUDENT_RECORD),
        )
        .await,
        "another Student in the Course cannot access this Student Work record",
    );
    assert!(
        !authenticated_student_record_ownership(
            &application,
            token(0xe3),
            id(COURSE),
            id(STUDENT_RECORD),
        )
        .await,
        "a nonmember cannot access this Student Work record",
    );
    assert!(
        !authenticated_student_record_ownership(
            &application,
            token(0xe1),
            id(COURSE),
            id(OTHER_COURSE_STUDENT_RECORD),
        )
        .await,
        "the same Account cannot combine one Course with Student Work from another Course",
    );
    assert!(
        !authenticated_student_record_ownership(
            &application,
            token(0xe4),
            id(COURSE),
            id(STUDENT_RECORD),
        )
        .await,
        "an ordinary Sysadmin cannot access Student Work without Student ownership",
    );
    let store = PostgresLiveAssessmentDeliveryStore::new(application.clone());
    let access = store
        .live_assessment_access(token(0xe1), course, assessment)
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
        .list_released_live_student_assessments(token(0xe1), course)
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
    .bind(&course_public_reference)
    .bind(&assessment_public_reference)
    .fetch_one(&mut *tx)
    .await
    .expect("complete Assessment Access projection");
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
         ple_private.assessment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject', \
             '2026-01-01 09:59:59.999+00') AS before_available, \
         ple_private.assessment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject', \
             '2026-01-01 10:00:00+00') AS at_available, \
         ple_private.assessment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject', \
             '2026-01-01 20:00:00+00') AS at_due, \
         ple_private.assessment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject', \
             '2026-01-01 20:00:00.001+00') AS after_due, \
         ple_private.assessment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 0, 'reject', \
             '2026-01-01 23:00:00+00') AS at_close, \
         ple_private.assessment_start_decision('released', '2026-01-01 10:00:00+00', \
             '2026-01-01 20:00:00+00', '2026-01-01 23:00:00+00', 2, 2, 'reject', \
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
