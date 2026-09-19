#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for server-owned Student Assessment Access.

use learning_data_access::postgres::{
    PostgresLiveAssessmentDeliveryStore, PostgresLiveStudentCourseLandingStore, lazy_pool,
};
use learning_data_access::{
    AssessmentStartDecision, LiveAssessmentDeliveryStore, LiveStudentCourseLandingStore,
    SessionTokenHash,
};
use question_model::{AssessmentId, AssessmentType, CourseInstanceId};
use sqlx::Row;
use uuid::Uuid;

const STUDENT_SESSION: u128 = 0xea03;
const NONMEMBER_SESSION: u128 = 0xea05;
const SYSADMIN_SESSION: u128 = 0xea07;
const STUDENT_RECORD: u128 = 0xeb02;
const STUDENT_MEMBERSHIP: u128 = 0xeb03;
const INSTRUCTOR_MEMBERSHIP: u128 = 0xeb04;
const OTHER_STUDENT_RECORD: u128 = 0xeb06;
const OTHER_STUDENT_MEMBERSHIP: u128 = 0xeb07;
const OTHER_STUDENT_SESSION: u128 = 0xeb08;
const OTHER_COURSE_STUDENT_RECORD: u128 = 0xeb0a;
const OTHER_COURSE_STUDENT_MEMBERSHIP: u128 = 0xeb0b;
const OTHER_COURSE_INSTRUCTOR_MEMBERSHIP: u128 = 0xeb0c;
const BLUEPRINT_MODULE: u128 = 0xec02;
const BLUEPRINT_ASSESSMENT: u128 = 0xec03;
const ASSESSMENT_ENTRY: u128 = 0xed02;
const OTHER_STUDENT_ACCOMMODATION: u128 = 0xed03;
const PUBLISHED_QUESTION: &str = "BCDE-2FGH";

struct AccessFixture {
    course_id: String,
    assessment_id: String,
}

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn token(marker: u8) -> SessionTokenHash {
    SessionTokenHash::compute(&[marker; 32])
}

async fn mint_account(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, role: &str) -> String {
    sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, product_role, created_at) \
         VALUES ('U00000009', $1, clock_timestamp()) RETURNING account_id",
    )
    .bind(role)
    .fetch_one(&mut **tx)
    .await
    .expect("minted Account")
}

async fn authenticated_student_record_ownership(
    application: &sqlx::postgres::PgPool,
    session_token: SessionTokenHash,
    course_id: &str,
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

async fn seed(admin: &sqlx::postgres::PgPool) -> AccessFixture {
    let mut tx = admin.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("classification fixture owner");
    sqlx::query(
        "INSERT INTO ple_data.content_discipline (content_discipline_id, name) \
         VALUES ('00000000-0000-0000-0000-00000000cc01', 'Course fixture discipline') \
         ON CONFLICT (content_discipline_id) DO NOTHING",
    )
    .execute(&mut *tx)
    .await
    .expect("explicit fixture Discipline");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("private fixture role");
    let instructor_id = mint_account(&mut tx, "instructor").await;
    let student_id = mint_account(&mut tx, "student").await;
    let other_student_id = mint_account(&mut tx, "student").await;
    let nonmember_id = mint_account(&mut tx, "student").await;
    let sysadmin_id = mint_account(&mut tx, "sysadmin").await;
    sqlx::query(
        "UPDATE ple_private.account_time_zone SET time_zone = 'America/Denver' \
         WHERE account_id = $1",
    )
    .bind(&student_id)
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
    .bind(&student_id)
    .bind(token(0xe1).to_string())
    .bind(id(OTHER_STUDENT_SESSION))
    .bind(&other_student_id)
    .bind(token(0xe2).to_string())
    .bind(id(NONMEMBER_SESSION))
    .bind(&nonmember_id)
    .bind(token(0xe3).to_string())
    .bind(id(SYSADMIN_SESSION))
    .bind(&sysadmin_id)
    .bind(token(0xe4).to_string())
    .execute(&mut *tx)
    .await
    .expect("Student Authenticated Session");

    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("data fixture role");
    sqlx::query(
        "INSERT INTO ple_data.published_question (published_question_id, created_at) \
         VALUES ($1, clock_timestamp())",
    )
    .bind(PUBLISHED_QUESTION)
    .execute(&mut *tx)
    .await
    .expect("Published Question");
    sqlx::query(
        "INSERT INTO ple_data.question_revision \
         (published_question_id, revision_number, backend, question_type, published_at) \
         VALUES ($1, 1, 'ple', 'multipleChoice', clock_timestamp())",
    )
    .bind(PUBLISHED_QUESTION)
    .execute(&mut *tx)
    .await
    .expect("Question Revision");

    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *tx)
        .await
        .expect("API fixture role");
    let blueprint_id: String = sqlx::query_scalar(
        "INSERT INTO ple_data.blueprint_course \
         (blueprint_course_id, owner_account_id, short_name, long_name, blueprint_edit_number, \
          created_at, content_discipline_id, tags) \
         VALUES ('BP0000000' || ple_private.crockford_checksum_character('BP0000000'), \
                 $1, 'ACCESS', 'Assessment Access Blueprint', 1, clock_timestamp(), \
                 '00000000-0000-0000-0000-00000000cc01', ARRAY[]::text[]) \
         RETURNING blueprint_course_id",
    )
    .bind(&instructor_id)
    .fetch_one(&mut *tx)
    .await
    .expect("Blueprint Course");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_course_revision \
         (blueprint_course_id, blueprint_revision_number, content, \
          content_checksum, saved_at) \
         VALUES ($1, 1, '{}'::jsonb, decode(repeat('0', 64), 'hex'), clock_timestamp())",
    )
    .bind(&blueprint_id)
    .execute(&mut *tx)
    .await
    .expect("Blueprint Revision");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_module \
         (blueprint_course_id, blueprint_revision_number, \
          blueprint_module_reference, module_position) VALUES ($1, 1, $2, 1)",
    )
    .bind(&blueprint_id)
    .bind(id(BLUEPRINT_MODULE))
    .execute(&mut *tx)
    .await
    .expect("Blueprint Module");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_assessment \
         (blueprint_course_id, blueprint_revision_number, \
          blueprint_module_reference, blueprint_assessment_id, assessment_position) \
         VALUES ($1, 1, $2, $3, 1)",
    )
    .bind(&blueprint_id)
    .bind(id(BLUEPRINT_MODULE))
    .bind(id(BLUEPRINT_ASSESSMENT))
    .execute(&mut *tx)
    .await
    .expect("Blueprint Assessment");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_event \
         (blueprint_course_id, blueprint_revision_number, actor_account_id, \
          request_checksum, occurred_at) \
         VALUES ($1, 1, $2, decode(repeat('ec', 32), 'hex'), clock_timestamp())",
    )
    .bind(&blueprint_id)
    .bind(&instructor_id)
    .execute(&mut *tx)
    .await
    .expect("Blueprint Revision Event");
    let course_id: String = sqlx::query_scalar(
        "INSERT INTO ple_data.course_instance \
         (course_instance_id, source_kind, blueprint_course_id, blueprint_revision_number, \
          course_short_name, course_long_name, term_starts_on, term_ends_on, created_at, \
          content_discipline_id, tags) \
         VALUES ('CI0000000' || ple_private.crockford_checksum_character('CI0000000'), \
                 'adopted', $1, 1, 'ACCESS', 'Assessment Access Course', \
                 current_date, current_date + 1, clock_timestamp(), \
                 '00000000-0000-0000-0000-00000000cc01', ARRAY[]::text[]) \
         RETURNING course_instance_id",
    )
    .bind(&blueprint_id)
    .fetch_one(&mut *tx)
    .await
    .expect("Course Instance");
    let other_course_id: String = sqlx::query_scalar(
        "INSERT INTO ple_data.course_instance \
         (course_instance_id, source_kind, blueprint_course_id, blueprint_revision_number, \
          course_short_name, course_long_name, term_starts_on, term_ends_on, created_at, \
          content_discipline_id, tags) \
         VALUES ('CI0000001' || ple_private.crockford_checksum_character('CI0000001'), \
                 'adopted', $1, 1, 'ACCESS-OTHER', \
                 'Other Course for exact Student Work scope', current_date, current_date + 1, \
                 clock_timestamp(), '00000000-0000-0000-0000-00000000cc01', ARRAY[]::text[]) \
         RETURNING course_instance_id",
    )
    .bind(&blueprint_id)
    .fetch_one(&mut *tx)
    .await
    .expect("other Course Instance");
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at) \
         VALUES ($1, $2, $3, 'instructor', NULL, clock_timestamp())",
    )
    .bind(id(INSTRUCTOR_MEMBERSHIP))
    .bind(&course_id)
    .bind(&instructor_id)
    .execute(&mut *tx)
    .await
    .expect("Instructor Course Membership");
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at) \
         VALUES ($1, $2, $3, 'instructor', NULL, clock_timestamp())",
    )
    .bind(id(OTHER_COURSE_INSTRUCTOR_MEMBERSHIP))
    .bind(&other_course_id)
    .bind(&instructor_id)
    .execute(&mut *tx)
    .await
    .expect("other Course Instructor Membership");
    sqlx::query(
        "INSERT INTO ple_data.student_record \
         (student_record_id, course_instance_id, student_account_id, created_at) \
         VALUES ($1, $2, $3, clock_timestamp()), \
                ($4, $2, $5, clock_timestamp())",
    )
    .bind(id(STUDENT_RECORD))
    .bind(&course_id)
    .bind(&student_id)
    .bind(id(OTHER_STUDENT_RECORD))
    .bind(&other_student_id)
    .execute(&mut *tx)
    .await
    .expect("Student Record");
    sqlx::query(
        "INSERT INTO ple_data.student_record \
         (student_record_id, course_instance_id, student_account_id, created_at) \
         VALUES ($1, $2, $3, clock_timestamp())",
    )
    .bind(id(OTHER_COURSE_STUDENT_RECORD))
    .bind(&other_course_id)
    .bind(&student_id)
    .execute(&mut *tx)
    .await
    .expect("same Account other Course Student Record");
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at) \
         VALUES ($1, $2, $3, 'student', $4, clock_timestamp())",
    )
    .bind(id(STUDENT_MEMBERSHIP))
    .bind(&course_id)
    .bind(&student_id)
    .bind(id(STUDENT_RECORD))
    .execute(&mut *tx)
    .await
    .expect("Student Course Membership");
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at) \
         VALUES ($1, $2, $3, 'student', $4, clock_timestamp())",
    )
    .bind(id(OTHER_STUDENT_MEMBERSHIP))
    .bind(&course_id)
    .bind(&other_student_id)
    .bind(id(OTHER_STUDENT_RECORD))
    .execute(&mut *tx)
    .await
    .expect("other Student Course Membership");
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at) \
         VALUES ($1, $2, $3, 'student', $4, clock_timestamp())",
    )
    .bind(id(OTHER_COURSE_STUDENT_MEMBERSHIP))
    .bind(&other_course_id)
    .bind(&student_id)
    .bind(id(OTHER_COURSE_STUDENT_RECORD))
    .execute(&mut *tx)
    .await
    .expect("same Account other Course Student Membership");

    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("Assessment fixture role");
    let snapshot_id: Vec<u8> = sqlx::query_scalar(
        "SELECT ple_private.ensure_assessment_policy_snapshot( \
             'Server-owned Assessment Access', 'Read the policy before starting.', \
             clock_timestamp() + interval '1 hour', \
             clock_timestamp() + interval '2 hours', \
             clock_timestamp() + interval '3 hours', \
             600, 2, 'reject', 'new_variation', 'shuffled', \
             'after_submit', 'after_submit', 'after_submit', \
             'after_submit', 'after_submit', 'after_submit', \
             'regular_assignment')",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("Assessment policy snapshot");
    let assessment_id: String = sqlx::query_scalar(
        "INSERT INTO ple_data.assessment \
         (assessment_id, course_instance_id, origin_kind, source_blueprint_course_id, \
          source_blueprint_revision_number, source_blueprint_assessment_id, created_at, \
          updated_at, assessment_type, assessment_policy_snapshot_id, assessment_status) \
         VALUES ('A0000000' || ple_private.crockford_checksum_character('A0000000'), \
                 $1, 'adopted', $2, 1, $3, clock_timestamp(), clock_timestamp(), \
                 'regular_assignment', $4, 'released') \
         RETURNING assessment_id",
    )
    .bind(&course_id)
    .bind(&blueprint_id)
    .bind(id(BLUEPRINT_ASSESSMENT))
    .bind(&snapshot_id)
    .fetch_one(&mut *tx)
    .await
    .expect("released Assessment");
    sqlx::query(
        "INSERT INTO ple_data.assessment_entry \
         (assessment_entry_id, assessment_id, authored_position, entry_kind, availability, \
          scoring_rule) \
         VALUES ($1, $2, 0, 'fixed_question', 'available', 'normal')",
    )
    .bind(id(ASSESSMENT_ENTRY))
    .bind(&assessment_id)
    .execute(&mut *tx)
    .await
    .expect("Assessment Entry");
    sqlx::query(
        "INSERT INTO ple_data.assessment_entry_question \
         (assessment_entry_id, assessment_id, published_question_id, question_revision_number, \
          points_possible) \
         VALUES ($1, $2, $3, 1, 2)",
    )
    .bind(id(ASSESSMENT_ENTRY))
    .bind(&assessment_id)
    .bind(PUBLISHED_QUESTION)
    .execute(&mut *tx)
    .await
    .expect("Assessment Entry Question");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("private accommodation role");
    sqlx::query(
        "INSERT INTO ple_private.student_assessment_accommodation( \
             accommodation_id, course_instance_id, student_record_id, assessment_id, \
             available_at, due_at, closes_at, time_multiplier, assessment_attempt_limit, \
             created_at \
         ) VALUES ( \
             $1, $2, $3, $4, clock_timestamp() - interval '1 hour', \
             clock_timestamp() + interval '1 hour', clock_timestamp() + interval '2 hours', \
             1.5, 1, clock_timestamp() \
         )",
    )
    .bind(id(OTHER_STUDENT_ACCOMMODATION))
    .bind(&course_id)
    .bind(id(OTHER_STUDENT_RECORD))
    .bind(&assessment_id)
    .execute(&mut *tx)
    .await
    .expect("other Student accommodation");
    tx.commit().await.expect("fixture commit");
    AccessFixture {
        course_id,
        assessment_id,
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
async fn access_reader_projects_one_authoritative_decision_and_effective_policy() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let migration_url = runtime.migration_url().expose();
    let admin = lazy_pool(migration_url).expect("migration pool");
    let fixture = seed(&admin).await;
    let course_public_reference = fixture.course_id.clone();
    let assessment_public_reference = fixture.assessment_id.clone();
    let course = CourseInstanceId::new(&course_public_reference).expect("Course reference");
    let assessment = AssessmentId::new(&assessment_public_reference).expect("Assessment reference");

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
