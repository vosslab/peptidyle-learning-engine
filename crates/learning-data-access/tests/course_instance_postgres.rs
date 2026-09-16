#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for Empty Course provenance and peer authority.

use learning_data_access::postgres::{PostgresCourseInstanceStore, lazy_pool};
use learning_data_access::{
    CourseInstanceCreationSource, CourseInstanceStore, CreateCourseInstanceInput, SessionTokenHash,
    StoreError,
};
use question_model::{AccountReference, CourseTerm};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

const ASSIGNED_INSTRUCTOR: u128 = 0xc701;
const CO_INSTRUCTOR: u128 = 0xc702;
const TARGET_INSTRUCTOR: u128 = 0xc703;
const NONMEMBER_INSTRUCTOR: u128 = 0xc704;
const LIFETIME_INSTRUCTOR: u128 = 0xc741;
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
        assigned_instructor: None,
    }
}

async fn seed(admin: &sqlx::postgres::PgPool) -> (AccountReference, AccountReference) {
    let mut transaction = admin.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("classification fixture owner");
    sqlx::query("INSERT INTO ple_data.content_discipline (discipline_uuid, name) VALUES ('00000000-0000-0000-0000-00000000cc01', 'Course fixture discipline') ON CONFLICT (discipline_uuid) DO NOTHING").execute(&mut *transaction).await.expect("explicit fixture Discipline");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("private fixture role");
    sqlx::query(
        "INSERT INTO ple_private.account (account_id, product_role, created_at) \
         VALUES ($1, 'instructor', clock_timestamp()), \
                ($2, 'instructor', clock_timestamp()), \
                ($3, 'instructor', clock_timestamp()), \
                ($4, 'instructor', clock_timestamp())",
    )
    .bind(id(ASSIGNED_INSTRUCTOR))
    .bind(id(CO_INSTRUCTOR))
    .bind(id(TARGET_INSTRUCTOR))
    .bind(id(NONMEMBER_INSTRUCTOR))
    .execute(&mut *transaction)
    .await
    .expect("fixture Instructor Accounts");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, product_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'instructor', decode($3, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour'), \
                ($4, $5, 'instructor', decode($6, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour'), \
                ($7, $8, 'instructor', decode($9, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour')",
    )
    .bind(id(ASSIGNED_SESSION))
    .bind(id(ASSIGNED_INSTRUCTOR))
    .bind(token(0xc1).to_string())
    .bind(id(CO_INSTRUCTOR_SESSION))
    .bind(id(CO_INSTRUCTOR))
    .bind(token(0xc2).to_string())
    .bind(id(NONMEMBER_SESSION))
    .bind(id(NONMEMBER_INSTRUCTOR))
    .bind(token(0xc3).to_string())
    .execute(&mut *transaction)
    .await
    .expect("fixture Instructor sessions");
    let references: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT account_id, public_reference FROM ple_private.account \
         WHERE account_id IN ($1, $2) ORDER BY account_id",
    )
    .bind(id(CO_INSTRUCTOR))
    .bind(id(TARGET_INSTRUCTOR))
    .fetch_all(&mut *transaction)
    .await
    .expect("fixture Instructor references");
    transaction.commit().await.expect("fixture commit");

    let reference_for = |account_id| {
        let public_reference = references
            .iter()
            .find_map(|(found, reference)| (*found == account_id).then_some(reference))
            .expect("fixture Account public reference");
        AccountReference::new(public_reference).expect("fixture Account reference")
    };
    (
        reference_for(id(CO_INSTRUCTOR)),
        reference_for(id(TARGET_INSTRUCTOR)),
    )
}

async fn malformed_empty_materialization_rejection(
    application_url: &str,
) -> Result<(), sqlx::Error> {
    let mut connection = PgConnection::connect(application_url).await?;
    let mut transaction = connection.begin().await?;
    sqlx::query("SET LOCAL ROLE ple_auth")
        .execute(&mut *transaction)
        .await?;
    sqlx::query("SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))")
        .bind(token(0xc1).to_string())
        .fetch_one(&mut *transaction)
        .await?;
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "SELECT public_reference FROM ple_api.create_course_instance( \
         $1, $2, $3, $4, 'empty', NULL, NULL, 'EMPTY-BAD', \
         'Rejected nonempty Empty Course', '2026-01-01'::date, '2026-05-01'::date, \
         NULL, jsonb_build_array(jsonb_build_object('source', $5::text)), '00000000-0000-0000-0000-00000000cc01', NULL, NULL, NULL, ARRAY[]::text[])",
    )
    .bind(id(0xc721))
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
    sqlx::query(
        "INSERT INTO ple_private.account (account_id, product_role, created_at) \
         VALUES ($1, 'instructor', clock_timestamp())",
    )
    .bind(id(LIFETIME_INSTRUCTOR))
    .execute(&mut *transaction)
    .await
    .expect("Active-lifetime fixture Instructor Account");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, product_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'instructor', decode($3, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour')",
    )
    .bind(id(LIFETIME_SESSION))
    .bind(id(LIFETIME_INSTRUCTOR))
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
        "SELECT public_reference FROM ple_api.create_course_instance( \
         $1, $2, $3, $4, 'empty', NULL, NULL, 'TERM-BAD', \
         'Rejected Active lifetime extension', \
         (transaction_timestamp() AT TIME ZONE 'UTC')::date, \
         (((transaction_timestamp() AT TIME ZONE 'UTC') + interval '6 months')::date + 1), \
         NULL, '[]'::jsonb, '00000000-0000-0000-0000-00000000cc01', NULL, NULL, NULL, ARRAY[]::text[])",
    )
    .bind(id(0xc731))
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
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
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
    let course = created.course.reference;
    let workspace = store
        .load_course_instance(token(0xc1), course)
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
    let provenance: (String, Option<i64>, Option<i64>, i64) = sqlx::query_as(
        "SELECT course.source_kind, course.blueprint_course_reference_number, \
                course.blueprint_revision_number, count(assessment.assessment_id) \
           FROM ple_data.course_instance AS course \
           LEFT JOIN ple_data.assessment AS assessment ON assessment.course_id = course.course_id \
          WHERE course.public_reference = $1 \
          GROUP BY course.source_kind, course.blueprint_course_reference_number, \
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
         WHERE public_reference = $1",
    )
    .bind(course.as_string())
    .execute(&mut *provenance_mutation)
    .await
    .expect_err("Course source provenance is immutable");
    assert_eq!(error_code(&provenance_error).as_deref(), Some("55000"));
    drop(provenance_mutation);

    let malformed = malformed_empty_materialization_rejection(&application_url)
        .await
        .expect_err("Empty Course rejects supplied initial materialization");
    assert_eq!(error_code(&malformed).as_deref(), Some("22023"));

    store
        .add_course_instructor(token(0xc1), course, co_instructor)
        .await
        .expect("assigned Instructor adds a co-Instructor");
    let peer_workspace = store
        .load_course_instance(token(0xc2), course)
        .await
        .expect("co-Instructor Course workspace");
    assert_eq!(peer_workspace.active_instructor_count, 2);
    store
        .add_course_instructor(token(0xc2), course, target_instructor)
        .await
        .expect("current co-Instructor has equal teaching-team authority");
    assert_eq!(
        store
            .add_course_instructor(token(0xc3), course, target_instructor)
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
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
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
