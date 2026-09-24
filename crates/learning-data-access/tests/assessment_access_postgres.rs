#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for server-owned Student Assessment Access.

use learning_data_access::SessionTokenHash;
use uuid::Uuid;

const STUDENT_SESSION: u128 = 0xea03;
const NONMEMBER_SESSION: u128 = 0xea05;
const SYSADMIN_SESSION: u128 = 0xea07;
const STUDENT_RECORD: u128 = 0xeb02;
const STUDENT_MEMBERSHIP: u128 = 0xeb03;
const INSTRUCTOR_MEMBERSHIP: u128 = 0xeb04;
const EXPIRY_ORACLE_STUDENT_RECORD: u128 = 0xeb18;
const EXPIRY_ORACLE_STUDENT_MEMBERSHIP: u128 = 0xeb19;
const OTHER_STUDENT_RECORD: u128 = 0xeb06;
const OTHER_STUDENT_MEMBERSHIP: u128 = 0xeb07;
const OTHER_STUDENT_SESSION: u128 = 0xeb08;
const OTHER_COURSE_STUDENT_RECORD: u128 = 0xeb0a;
const OTHER_COURSE_STUDENT_MEMBERSHIP: u128 = 0xeb0b;
const OTHER_COURSE_INSTRUCTOR_MEMBERSHIP: u128 = 0xeb0c;
const PROGRESS_ATTEMPT: u128 = 0xeb0d;
const HISTORY_ATTEMPT: u128 = 0xeb0e;
const DISPLAY_QUESTION_ATTEMPT: u128 = 0xeb0f;
const PRACTICE_ATTEMPT_ONE: u128 = 0xeb10;
const PRACTICE_ATTEMPT_TWO: u128 = 0xeb11;
const PRACTICE_QUESTION_ATTEMPT_ONE: u128 = 0xeb12;
const PRACTICE_QUESTION_ATTEMPT_TWO: u128 = 0xeb13;
const PRACTICE_ISSUED_QUESTION_ONE: u128 = 0xeb14;
const PRACTICE_ISSUED_QUESTION_TWO: u128 = 0xeb15;
const PRACTICE_SUBMISSION_ONE: u128 = 0xeb16;
const PRACTICE_SUBMISSION_TWO: u128 = 0xeb17;
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
         VALUES ('U00000009', $1::ple_data.product_role, pg_catalog.transaction_timestamp()) RETURNING account_id",
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
    let expiry_oracle_student_id = mint_account(&mut tx, "student").await;
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
         VALUES ($1, $2, 'student', decode($3, 'hex'), pg_catalog.transaction_timestamp(), \
                 pg_catalog.transaction_timestamp() + interval '1 hour'), \
                ($4, $5, 'student', decode($6, 'hex'), pg_catalog.transaction_timestamp(), \
                 pg_catalog.transaction_timestamp() + interval '1 hour'), \
                ($7, $8, 'student', decode($9, 'hex'), pg_catalog.transaction_timestamp(), \
                 pg_catalog.transaction_timestamp() + interval '1 hour'), \
                ($10, $11, 'sysadmin', decode($12, 'hex'), pg_catalog.transaction_timestamp(), \
                 pg_catalog.transaction_timestamp() + interval '1 hour')",
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
          blueprint_module_id, module_position) VALUES ($1, 1, $2, 1)",
    )
    .bind(&blueprint_id)
    .bind(id(BLUEPRINT_MODULE))
    .execute(&mut *tx)
    .await
    .expect("Blueprint Module");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_assessment \
         (blueprint_course_id, blueprint_revision_number, \
          blueprint_module_id, blueprint_assessment_id, assessment_position) \
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
                 current_date, current_date + 1, pg_catalog.transaction_timestamp(), \
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
                 pg_catalog.transaction_timestamp(), '00000000-0000-0000-0000-00000000cc01', ARRAY[]::text[]) \
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
         VALUES ($1, $2, $3, pg_catalog.transaction_timestamp()), \
                ($4, $2, $5, pg_catalog.transaction_timestamp())",
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
         VALUES ($1, $2, $3, pg_catalog.transaction_timestamp())",
    )
    .bind(id(EXPIRY_ORACLE_STUDENT_RECORD))
    .bind(&course_id)
    .bind(&expiry_oracle_student_id)
    .execute(&mut *tx)
    .await
    .expect("isolated Attempt-expiry Student Record");
    sqlx::query(
        "INSERT INTO ple_data.student_record \
         (student_record_id, course_instance_id, student_account_id, created_at) \
         VALUES ($1, $2, $3, pg_catalog.transaction_timestamp())",
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
    .bind(id(EXPIRY_ORACLE_STUDENT_MEMBERSHIP))
    .bind(&course_id)
    .bind(&expiry_oracle_student_id)
    .bind(id(EXPIRY_ORACLE_STUDENT_RECORD))
    .execute(&mut *tx)
    .await
    .expect("isolated Attempt-expiry Course Membership");
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
             1.5, 1, pg_catalog.transaction_timestamp() \
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

#[path = "assessment_access_postgres/access_reader.rs"]
mod access_reader;
