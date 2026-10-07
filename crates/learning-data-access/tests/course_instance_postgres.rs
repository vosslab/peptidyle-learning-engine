#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for Empty Course provenance and peer authority.

use learning_data_access::postgres::{PostgresCourseInstanceStore, lazy_pool};
use learning_data_access::{
    CourseInstanceCreationSource, CourseInstanceLifecycleState, CourseInstanceStore,
    CreateCourseInstanceInput, SessionTokenHash, StoreError,
};
use question_model::{AccountId, CourseTerm};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

const ASSIGNED_SESSION: u128 = 0xc711;
const CO_INSTRUCTOR_SESSION: u128 = 0xc712;
const NONMEMBER_SESSION: u128 = 0xc713;
const LIFETIME_SESSION: u128 = 0xc751;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn token(marker: u8) -> SessionTokenHash {
    SessionTokenHash::compute(&[marker; 32])
}

fn empty_course_input() -> CreateCourseInstanceInput {
    CreateCourseInstanceInput {
        classification: question_model::CourseClassification {
            discipline_uuid: uuid::Uuid::from_u128(0xcc01),
            subject_uuid: None,
            topic_uuid: None,
            subtopic_uuid: None,
            tags: Vec::new(),
        },
        source: CourseInstanceCreationSource::Empty,
        short_name: "EMPTY-1".to_owned(),
        long_name: "Empty Course contract".to_owned(),
        term: CourseTerm::from_parts("2026-01-01", "2026-05-01").expect("fixture term"),
        assigned_instructor_account_id: None,
    }
}

async fn ensure_assigned_instructor_session(admin: &sqlx::postgres::PgPool) {
    let mut transaction = admin.begin().await.expect("session lookup");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("private fixture role");
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT account_id::text FROM ple_private.authenticated_session WHERE session_id = $1",
    )
    .bind(id(ASSIGNED_SESSION))
    .fetch_optional(&mut *transaction)
    .await
    .expect("assigned session lookup");
    transaction
        .rollback()
        .await
        .expect("session lookup rollback");
    if existing.is_none() {
        seed(admin).await;
    }
}

async fn seed(admin: &sqlx::postgres::PgPool) -> (AccountId, AccountId) {
    let mut transaction = admin.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("classification fixture owner");
    sqlx::query(
        "INSERT INTO ple_data.content_discipline (content_discipline_id, name) \
         VALUES ('00000000-0000-0000-0000-00000000cc01', 'Course fixture discipline') \
         ON CONFLICT (content_discipline_id) DO NOTHING",
    )
    .execute(&mut *transaction)
    .await
    .expect("explicit fixture Discipline");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("private fixture role");
    let assigned_id: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ('U00000009', 'instructor', pg_catalog.transaction_timestamp()) RETURNING account_id",
    )
    .fetch_one(&mut *transaction)
    .await
    .expect("assigned Instructor Account");
    let co_instructor_id: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ('U00000009', 'instructor', pg_catalog.transaction_timestamp()) RETURNING account_id",
    )
    .fetch_one(&mut *transaction)
    .await
    .expect("co-Instructor Account");
    let target_instructor_id: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ('U00000009', 'instructor', pg_catalog.transaction_timestamp()) RETURNING account_id",
    )
    .fetch_one(&mut *transaction)
    .await
    .expect("target Instructor Account");
    let nonmember_id: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ('U00000009', 'instructor', pg_catalog.transaction_timestamp()) RETURNING account_id",
    )
    .fetch_one(&mut *transaction)
    .await
    .expect("nonmember Instructor Account");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, user_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'instructor', decode($3, 'hex'), pg_catalog.transaction_timestamp(), \
                 pg_catalog.transaction_timestamp() + interval '1 hour'), \
                ($4, $5, 'instructor', decode($6, 'hex'), pg_catalog.transaction_timestamp(), \
                 pg_catalog.transaction_timestamp() + interval '1 hour'), \
                ($7, $8, 'instructor', decode($9, 'hex'), pg_catalog.transaction_timestamp(), \
                 pg_catalog.transaction_timestamp() + interval '1 hour')",
    )
    .bind(id(ASSIGNED_SESSION))
    .bind(&assigned_id)
    .bind(token(0xc1).to_string())
    .bind(id(CO_INSTRUCTOR_SESSION))
    .bind(&co_instructor_id)
    .bind(token(0xc2).to_string())
    .bind(id(NONMEMBER_SESSION))
    .bind(&nonmember_id)
    .bind(token(0xc3).to_string())
    .execute(&mut *transaction)
    .await
    .expect("fixture Instructor sessions");
    transaction.commit().await.expect("fixture commit");
    (
        AccountId::new(co_instructor_id).expect("fixture Account ID"),
        AccountId::new(target_instructor_id).expect("fixture Account ID"),
    )
}

async fn malformed_empty_materialization_rejection(
    application_url: &str,
    session_token: SessionTokenHash,
) -> Result<(), sqlx::Error> {
    let mut connection = PgConnection::connect(application_url).await?;
    let mut transaction = connection.begin().await?;
    sqlx::query("SET LOCAL ROLE ple_auth")
        .execute(&mut *transaction)
        .await?;
    sqlx::query("SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))")
        .bind(session_token.to_string())
        .fetch_one(&mut *transaction)
        .await?;
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "SELECT course_instance_id FROM ple_api.create_course_instance( \
         'CI0000000Y', \
         $1, $2, $3, 'empty', NULL, NULL, 'EMPTY-BAD', \
         'Rejected nonempty Empty Course', '2026-01-01'::date, '2026-05-01'::date, \
         NULL, jsonb_build_array(jsonb_build_object('source', $4::text)), \
         '00000000-0000-0000-0000-00000000cc01', NULL, NULL, NULL, ARRAY[]::text[])",
    )
    .bind(id(0xc722))
    .bind(id(0xc723))
    .bind(id(0xc724))
    .bind(id(0xc725))
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn seed_lifetime_instructor(admin: &sqlx::postgres::PgPool) {
    let mut transaction = admin
        .begin()
        .await
        .expect("Active-lifetime fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("Active-lifetime private fixture role");
    let lifetime_id: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ('U00000009', 'instructor', pg_catalog.transaction_timestamp()) RETURNING account_id",
    )
    .fetch_one(&mut *transaction)
    .await
    .expect("Active-lifetime fixture Instructor Account");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, user_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'instructor', decode($3, 'hex'), pg_catalog.transaction_timestamp(), \
                 pg_catalog.transaction_timestamp() + interval '1 hour')",
    )
    .bind(id(LIFETIME_SESSION))
    .bind(&lifetime_id)
    .bind(token(0xc4).to_string())
    .execute(&mut *transaction)
    .await
    .expect("Active-lifetime fixture Instructor session");
    transaction
        .commit()
        .await
        .expect("Active-lifetime fixture commit");
}

async fn course_term_beyond_active_lifetime_rejection(
    application_url: &str,
    session_token: SessionTokenHash,
) -> Result<(), sqlx::Error> {
    let mut connection = PgConnection::connect(application_url).await?;
    let mut transaction = connection.begin().await?;
    sqlx::query("SET LOCAL ROLE ple_auth")
        .execute(&mut *transaction)
        .await?;
    sqlx::query("SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))")
        .bind(session_token.to_string())
        .fetch_one(&mut *transaction)
        .await?;
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "SELECT course_instance_id FROM ple_api.create_course_instance( \
         'CI0000000Y', \
         $1, $2, $3, 'empty', NULL, NULL, 'TERM-BAD', \
         'Rejected Active lifetime extension', \
         (transaction_timestamp() AT TIME ZONE 'UTC')::date, \
         (((transaction_timestamp() AT TIME ZONE 'UTC') + interval '6 months')::date + 1), \
         NULL, '[]'::jsonb, '00000000-0000-0000-0000-00000000cc01', NULL, NULL, NULL, ARRAY[]::text[])",
    )
    .bind(id(0xc732))
    .bind(id(0xc733))
    .bind(id(0xc734))
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

fn error_code(error: &sqlx::Error) -> Option<String> {
    match error {
        sqlx::Error::Database(database) => database.code().map(|code| code.into_owned()),
        _ => None,
    }
}

/// Prevents accidental Empty-Course materialization and an assigned-Instructor
/// rank proxy from silently weakening Course teaching-team authority.
///
/// Failure means creation could introduce unrequested teaching content or a
/// current peer Instructor could lose the same membership authority. Repair the
/// Course creation or membership procedure before accepting a release.
#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn empty_course_has_no_initial_content_and_current_instructors_are_peers() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let migration_url = runtime.migration_url().expose();
    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let admin = lazy_pool(migration_url).expect("migration pool");
    let (co_instructor, target_instructor) = seed(&admin).await;

    let store = PostgresCourseInstanceStore::new(lazy_pool(&application_url).expect("app pool"));
    let created = store
        .create_course_instance(token(0xc1), empty_course_input())
        .await
        .expect("Empty Course creation");
    let course = created.course_instance.id;
    let workspace = store
        .load_course_instance(token(0xc1), course.clone())
        .await
        .expect("assigned Instructor Course workspace");
    assert_eq!(workspace.active_instructor_count, 1);

    let mut inspection = PgConnection::connect(migration_url)
        .await
        .expect("inspection connection");
    let mut provenance_inspection = inspection
        .begin()
        .await
        .expect("Empty Course provenance transaction");
    // ASVS 8.2.1: this assertion uses the explicit data-owner fixture role.
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *provenance_inspection)
        .await
        .expect("Empty Course provenance fixture role");
    let provenance: (String, Option<String>, Option<i64>, i64) = sqlx::query_as(
        "SELECT course.source_kind::text, course.blueprint_course_id::text, \
                course.blueprint_revision_number, count(assessment.assessment_id) \
           FROM ple_data.course_instance AS course \
           LEFT JOIN ple_data.assessment AS assessment \
             ON assessment.course_instance_id = course.course_instance_id \
          WHERE course.course_instance_id = $1 \
          GROUP BY course.source_kind, course.blueprint_course_id, \
                   course.blueprint_revision_number",
    )
    .bind(course.as_string())
    .fetch_one(&mut *provenance_inspection)
    .await
    .expect("Empty Course provenance");
    provenance_inspection
        .commit()
        .await
        .expect("Empty Course provenance inspection commit");
    assert_eq!(provenance, ("empty".to_owned(), None, None, 0));

    let mut provenance_mutation = inspection
        .begin()
        .await
        .expect("provenance mutation transaction");
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *provenance_mutation)
        .await
        .expect("provenance mutation role");
    let provenance_error = sqlx::query(
        "UPDATE ple_data.course_instance SET source_kind = 'adopted' \
         WHERE course_instance_id = $1",
    )
    .bind(course.as_string())
    .execute(&mut *provenance_mutation)
    .await
    .expect_err("Course source provenance is immutable");
    assert_eq!(error_code(&provenance_error).as_deref(), Some("55000"));
    drop(provenance_mutation);

    let malformed = malformed_empty_materialization_rejection(&application_url, token(0xc2))
        .await
        .expect_err("Empty Course rejects supplied initial materialization");
    assert_eq!(error_code(&malformed).as_deref(), Some("22023"));

    store
        .add_course_instructor(token(0xc1), course.clone(), co_instructor)
        .await
        .expect("assigned Instructor adds a co-Instructor");
    let peer_workspace = store
        .load_course_instance(token(0xc2), course.clone())
        .await
        .expect("co-Instructor Course workspace");
    assert_eq!(peer_workspace.active_instructor_count, 2);
    store
        .add_course_instructor(token(0xc2), course.clone(), target_instructor.clone())
        .await
        .expect("current co-Instructor has equal teaching-team authority");
    assert_eq!(
        store
            .add_course_instructor(token(0xc3), course.clone(), target_instructor)
            .await,
        Err(StoreError::Forbidden),
        "an Instructor without an active Course membership cannot add peers"
    );
    let final_workspace = store
        .load_course_instance(token(0xc1), course)
        .await
        .expect("final Course workspace");
    assert_eq!(final_workspace.active_instructor_count, 3);

    admin.close().await;
    inspection
        .close()
        .await
        .expect("inspection connection close");
}

/// Keeps the immutable six-month Active cutoff from being bypassed through a
/// longer Instructor-supplied Course term. Failure means a Course could remain
/// apparently teachable past its retention boundary; repair the SQL creation
/// predicate and its table invariant before accepting a release.
#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn course_creation_rejects_a_term_after_its_active_lifetime() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let migration_url = runtime.migration_url().expose();
    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let admin = lazy_pool(migration_url).expect("migration pool");
    seed_lifetime_instructor(&admin).await;

    let rejection = course_term_beyond_active_lifetime_rejection(&application_url, token(0xc4))
        .await
        .expect_err("Course term after the Active cutoff must be rejected");
    assert_eq!(error_code(&rejection).as_deref(), Some("22023"));

    admin.close().await;
}

/// Active Courses are the current teaching Course Instances. An Inactive Course
/// is a past Course Instance and keeps its metadata after Student data is removed.
///
/// Failure means the Active cutoff could leave a Course teaching, or Student-data
/// deletion could remove the Course name, term, or discipline. Repair the
/// retention transition before accepting a release.
#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn inactive_course_keeps_metadata_after_student_data_deletion() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    let mut tx = admin.begin().await.expect("course retention transaction");
    let discipline_id = Uuid::from_u128(0xc761);

    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("classification fixture owner");
    sqlx::query(
        "INSERT INTO ple_data.content_discipline (content_discipline_id, name) \
         VALUES ($1, 'Past Course discipline') ON CONFLICT (content_discipline_id) DO NOTHING",
    )
    .bind(discipline_id)
    .execute(&mut *tx)
    .await
    .expect("Course discipline");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("account fixture owner");
    let instructor_id: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ('U0000001' || ple_private.crockford_checksum_character('U0000001'), \
                 'instructor', pg_catalog.transaction_timestamp()) RETURNING account_id",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("Instructor Account");
    let student_id: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ('U00000009', 'student', pg_catalog.transaction_timestamp()) RETURNING account_id",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("Student Account");
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *tx)
        .await
        .expect("Course fixture owner");
    let (course_id, short_name, long_name, term_start, term_end): (
        String,
        String,
        String,
        String,
        String,
    ) = sqlx::query_as(
        "INSERT INTO ple_data.course_instance \
         (course_instance_id, source_kind, course_short_name, course_long_name, \
          content_discipline_id, tags, term_starts_on, term_ends_on, created_at) \
         VALUES ('CI0000000' || ple_private.crockford_checksum_character('CI0000000'), \
                 'empty', 'PAST-1', 'Past teaching Course', $1, ARRAY[]::text[], \
                 current_date, current_date + 1, pg_catalog.transaction_timestamp()) \
         RETURNING course_instance_id, course_short_name, course_long_name, \
                   term_starts_on::text, term_ends_on::text",
    )
    .bind(discipline_id)
    .fetch_one(&mut *tx)
    .await
    .expect("Active Course");
    let student_record_id = Uuid::from_u128(0xc762);
    sqlx::query(
        "INSERT INTO ple_data.student_record \
         (student_record_id, course_instance_id, student_account_id, created_at) \
         VALUES ($1, $2, $3, pg_catalog.transaction_timestamp())",
    )
    .bind(student_record_id)
    .bind(&course_id)
    .bind(&student_id)
    .execute(&mut *tx)
    .await
    .expect("Student record");
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (course_membership_id, course_instance_id, account_id, role, student_record_id, \
          joined_at) VALUES ($1, $2, $3, 'instructor', NULL, clock_timestamp())",
    )
    .bind(Uuid::from_u128(0xc763))
    .bind(&course_id)
    .bind(&instructor_id)
    .execute(&mut *tx)
    .await
    .expect("Instructor membership");
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (course_membership_id, course_instance_id, account_id, role, student_record_id, \
          joined_at) VALUES ($1, $2, $3, 'student', $4, clock_timestamp())",
    )
    .bind(Uuid::from_u128(0xc764))
    .bind(&course_id)
    .bind(&student_id)
    .bind(student_record_id)
    .execute(&mut *tx)
    .await
    .expect("Student membership");
    sqlx::query("SELECT set_config('ple.session_account_id', $1, true)")
        .bind(&instructor_id)
        .execute(&mut *tx)
        .await
        .expect("Instructor session");
    let active_listing: (String, String, String) = sqlx::query_as(
        "SELECT short_name, long_name, course_lifecycle_state \
           FROM ple_api.list_course_instances() WHERE course_instance_id = $1",
    )
    .bind(&course_id)
    .fetch_one(&mut *tx)
    .await
    .expect("current teaching Course");
    assert_eq!(
        active_listing,
        (short_name.clone(), long_name.clone(), "active".to_owned())
    );

    sqlx::query("SET LOCAL ROLE ple_course_retention_executor")
        .execute(&mut *tx)
        .await
        .expect("retention executor");
    let marked: bool = sqlx::query_scalar(
        "SELECT ple_api.mark_course_instance_inactive( \
             $1, \
             (SELECT active_until_at FROM ple_data.course_instance \
               WHERE course_instance_id = $1))",
    )
    .bind(&course_id)
    .fetch_one(&mut *tx)
    .await
    .expect("mark Inactive at the Active cutoff");
    assert!(marked, "the due Active cutoff marks the Course Inactive");
    let archived: bool = sqlx::query_scalar(
        "SELECT ple_api.archive_course_student_records( \
             $1, \
             (SELECT course.retention_starts_at + schedule.archive_after_retention_start \
                FROM ple_data.course_instance AS course \
                CROSS JOIN ple_data.retention_schedule() AS schedule \
               WHERE course.course_instance_id = $1))",
    )
    .bind(&course_id)
    .fetch_one(&mut *tx)
    .await
    .expect("archive Student records");
    assert!(archived, "Student-record archive starts retention deletion");
    let deleted: bool = sqlx::query_scalar(
        "SELECT ple_api.delete_course_student_records( \
             $1, \
             (SELECT course.retention_starts_at \
                     + schedule.archive_after_retention_start \
                     + schedule.delete_after_archive \
                FROM ple_data.course_instance AS course \
                CROSS JOIN ple_data.retention_schedule() AS schedule \
               WHERE course.course_instance_id = $1))",
    )
    .bind(&course_id)
    .fetch_one(&mut *tx)
    .await
    .expect("delete Student records");
    assert!(
        deleted,
        "due Student-record deletion removes FERPA Student data"
    );

    let kept: (String, String, String, String, Uuid, String, i64) = sqlx::query_as(
        "SELECT course_short_name, course_long_name, term_starts_on::text, term_ends_on::text, \
                content_discipline_id, course_lifecycle_state::text, \
                purged_students_ever_enrolled \
           FROM ple_data.course_instance WHERE course_instance_id = $1",
    )
    .bind(&course_id)
    .fetch_one(&mut *tx)
    .await
    .expect("retained Course metadata");
    assert_eq!(
        kept,
        (
            short_name.clone(),
            long_name.clone(),
            term_start,
            term_end,
            discipline_id,
            "inactive".to_owned(),
            1,
        )
    );
    let student_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_data.student_record WHERE course_instance_id = $1",
    )
    .bind(&course_id)
    .fetch_one(&mut *tx)
    .await
    .expect("Student record count");
    let student_memberships: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_data.course_membership \
          WHERE course_instance_id = $1 AND role = 'student'",
    )
    .bind(&course_id)
    .fetch_one(&mut *tx)
    .await
    .expect("Student membership count");
    let instructor_memberships: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_data.course_membership \
          WHERE course_instance_id = $1 AND role = 'instructor'",
    )
    .bind(&course_id)
    .fetch_one(&mut *tx)
    .await
    .expect("Instructor membership count");
    assert_eq!(student_rows, 0, "Student records are removed");
    assert_eq!(student_memberships, 0, "Student memberships are removed");
    assert_eq!(instructor_memberships, 1, "the teaching Instructor remains");

    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *tx)
        .await
        .expect("Instructor list role");
    sqlx::query("SELECT set_config('ple.session_account_id', $1, true)")
        .bind(&instructor_id)
        .execute(&mut *tx)
        .await
        .expect("Instructor session after deletion");
    let inactive_listing: (String, String, String) = sqlx::query_as(
        "SELECT short_name, long_name, course_lifecycle_state \
           FROM ple_api.list_course_instances() WHERE course_instance_id = $1",
    )
    .bind(&course_id)
    .fetch_one(&mut *tx)
    .await
    .expect("past Course");
    assert_eq!(
        inactive_listing,
        (short_name, long_name, "inactive".to_owned())
    );

    tx.rollback().await.expect("course retention rollback");
    admin.close().await;
}

/// The member Course summary carries the same stored activity state as creation and the list.
///
/// Failure means a course route could not tell an Active Course from an Inactive Course without a
/// second query. Repair `ple_api.read_course_summary` before accepting a release.
#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn read_course_summary_returns_course_lifecycle_state() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    ensure_assigned_instructor_session(&admin).await;

    let mut course_input = empty_course_input();
    course_input.short_name = "SUM-LIFE".to_owned();
    course_input.long_name = "Summary lifecycle Course".to_owned();
    let store = PostgresCourseInstanceStore::new(lazy_pool(&application_url).expect("app pool"));
    let created = store
        .create_course_instance(token(0xc1), course_input)
        .await
        .expect("Empty Course creation");
    let course_id = created.course_instance.id;
    let active = store
        .read_course_summary(token(0xc1), course_id.clone())
        .await
        .expect("active course summary");
    assert_eq!(
        active.lifecycle_state,
        CourseInstanceLifecycleState::Active,
        "a new Course is active"
    );

    let mut tx = admin.begin().await.expect("inactive transition");
    sqlx::query("SET LOCAL ROLE ple_course_retention_executor")
        .execute(&mut *tx)
        .await
        .expect("retention executor");
    let marked: bool = sqlx::query_scalar(
        "SELECT ple_api.mark_course_instance_inactive( \
             $1, \
             (SELECT active_until_at FROM ple_data.course_instance \
               WHERE course_instance_id = $1))",
    )
    .bind(course_id.as_str())
    .fetch_one(&mut *tx)
    .await
    .expect("mark Inactive at the Active cutoff");
    assert!(marked, "the due Active cutoff marks the Course Inactive");
    tx.commit().await.expect("inactive commit");

    let inactive = store
        .read_course_summary(token(0xc1), course_id)
        .await
        .expect("inactive course summary");
    assert_eq!(
        inactive.lifecycle_state,
        CourseInstanceLifecycleState::Inactive,
        "read_course_summary returns the stored inactive state"
    );

    admin.close().await;
}
