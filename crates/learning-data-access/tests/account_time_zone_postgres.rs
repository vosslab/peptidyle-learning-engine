#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for Student-owned Account time zones.

use learning_data_access::postgres::{
    PostgresAccountTimeZoneStore, PostgresCourseRosterStore, lazy_pool,
};
use learning_data_access::{
    AccountTimeZoneStore, CourseRosterImportEntry, CourseRosterImportInput, CourseRosterStore,
    SessionTokenHash, StoreError,
};
use question_model::{AccountTimeZone, CourseInstanceReference};
use sqlx::Row;
use uuid::Uuid;

const INSTRUCTOR: u128 = 0xee01;
const EXISTING_STUDENT: u128 = 0xee02;
const INSTRUCTOR_SESSION: u128 = 0xee03;
const EXISTING_STUDENT_SESSION: u128 = 0xee04;
const NEW_STUDENT_SESSION: u128 = 0xee05;
const BLUEPRINT: u128 = 0xef01;
const COURSE: u128 = 0xef02;
const INSTRUCTOR_MEMBERSHIP: u128 = 0xef03;
const REFERENCE_NUMBER: i64 = 920_002;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn token(byte: u8) -> SessionTokenHash {
    SessionTokenHash::compute(&[byte; 32])
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
                ($2, 'student', clock_timestamp())",
    )
    .bind(id(INSTRUCTOR))
    .bind(id(EXISTING_STUDENT))
    .execute(&mut *tx)
    .await
    .expect("fixture Accounts");
    sqlx::query(
        "UPDATE ple_private.account_time_zone \
            SET time_zone = CASE account_id \
                WHEN $1 THEN 'America/New_York' ELSE 'America/Denver' END \
          WHERE account_id IN ($1, $2)",
    )
    .bind(id(INSTRUCTOR))
    .bind(id(EXISTING_STUDENT))
    .execute(&mut *tx)
    .await
    .expect("fixture Account time zones");
    sqlx::query(
        "INSERT INTO ple_private.account_authentication_email \
         (account_id, normalized_email, delivery_email, verified_at, updated_at) \
         VALUES ($1, 'existing.student@example.test', 'existing.student@example.test', \
                 clock_timestamp(), clock_timestamp())",
    )
    .bind(id(EXISTING_STUDENT))
    .execute(&mut *tx)
    .await
    .expect("existing Student Authentication Email");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, product_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'instructor', decode($3, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour'), \
                ($4, $5, 'student', decode($6, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour')",
    )
    .bind(id(INSTRUCTOR_SESSION))
    .bind(id(INSTRUCTOR))
    .bind(token(0xf1).to_string())
    .bind(id(EXISTING_STUDENT_SESSION))
    .bind(id(EXISTING_STUDENT))
    .bind(token(0xf2).to_string())
    .execute(&mut *tx)
    .await
    .expect("fixture Authenticated Sessions");

    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *tx)
        .await
        .expect("API fixture role");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_course \
         (blueprint_id, reference_number, owner_account_id, short_name, long_name, \
          metadata_etag, created_at) OVERRIDING SYSTEM VALUE \
         VALUES ($1, $2, $3, 'ZONE', 'Student Time Zone Blueprint', \
                 '00000000-0000-0000-0000-00000000ef04', clock_timestamp())",
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
        "INSERT INTO ple_data.blueprint_revision_event \
         (blueprint_course_reference_number, blueprint_revision_number, actor_account_id, \
          request_checksum, occurred_at) \
         VALUES ($1, 1, $2, decode(repeat('ef', 32), 'hex'), clock_timestamp())",
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
         VALUES ($1, $2, $2, 1, $3, 'instructor', 'ZONE', \
                 'Student Time Zone Course', current_date, current_date + 1, clock_timestamp())",
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
    tx.commit().await.expect("fixture commit");
}

async fn new_student_id_and_session(admin: &sqlx::postgres::PgPool) -> Uuid {
    let mut tx = admin
        .begin()
        .await
        .expect("new Student session transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("private session role");
    let account_id: Uuid = sqlx::query_scalar(
        "SELECT account_id FROM ple_private.account_authentication_email \
         WHERE normalized_email = 'new.student@example.test'",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("new Student Account");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, product_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'student', decode($3, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour')",
    )
    .bind(id(NEW_STUDENT_SESSION))
    .bind(account_id)
    .bind(token(0xf3).to_string())
    .execute(&mut *tx)
    .await
    .expect("new Student Authenticated Session");
    tx.commit().await.expect("new Student session commit");
    account_id
}

async fn account_time_zone(admin: &sqlx::postgres::PgPool, account_id: Uuid) -> (String, bool) {
    let mut tx = admin.begin().await.expect("Account time-zone transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("private Account time-zone role");
    let row = sqlx::query(
        "SELECT time_zone, student_invitation_default_pending \
         FROM ple_private.account_time_zone WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&mut *tx)
    .await
    .expect("Account time-zone preference");
    let result = (
        row.try_get("time_zone").expect("time zone"),
        row.try_get("student_invitation_default_pending")
            .expect("pending Student default"),
    );
    tx.commit().await.expect("Account time-zone commit");
    result
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
async fn invitation_acceptance_defaults_only_a_new_student_account_to_the_inviting_instructor_zone()
{
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    seed(&admin).await;

    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let application = lazy_pool(&application_url).expect("application pool");
    let roster = PostgresCourseRosterStore::new(application.clone());
    let course = CourseInstanceReference::new(REFERENCE_NUMBER as u64).expect("Course reference");
    let imported = roster
        .import_course_roster(
            token(0xf1),
            course,
            CourseRosterImportInput {
                entries: vec![
                    CourseRosterImportEntry {
                        email: "existing.student@example.test".to_string(),
                        roster_id: "EXISTING".to_string(),
                    },
                    CourseRosterImportEntry {
                        email: "new.student@example.test".to_string(),
                        roster_id: "NEW".to_string(),
                    },
                ],
            },
        )
        .await
        .expect("Course Roster Import");
    assert_eq!(imported.len(), 2);

    let new_student = new_student_id_and_session(&admin).await;
    assert_eq!(
        account_time_zone(&admin, new_student).await,
        ("America/Chicago".to_string(), true),
        "new Student waits for the invitation default until acceptance"
    );
    roster
        .claim_course_invitation(token(0xf3), course)
        .await
        .expect("new Student invitation acceptance");
    roster
        .claim_course_invitation(token(0xf2), course)
        .await
        .expect("existing Student invitation acceptance");
    assert_eq!(
        account_time_zone(&admin, new_student).await,
        ("America/New_York".to_string(), false)
    );
    assert_eq!(
        account_time_zone(&admin, id(EXISTING_STUDENT)).await,
        ("America/Denver".to_string(), false),
        "an existing Student Account keeps its preference"
    );

    let time_zones = PostgresAccountTimeZoneStore::new(application);
    let saved = time_zones
        .update_authenticated_student_time_zone(
            token(0xf3),
            AccountTimeZone::parse("America/Los_Angeles").expect("valid Student zone"),
        )
        .await
        .expect("Student-owned time-zone update");
    assert_eq!(saved.as_str(), "America/Los_Angeles");
    assert_eq!(
        time_zones
            .update_authenticated_student_time_zone(
                token(0xf1),
                AccountTimeZone::parse("UTC").expect("valid Instructor-requested zone"),
            )
            .await,
        Err(StoreError::Forbidden),
        "an Instructor session cannot set a Student time zone"
    );
}
