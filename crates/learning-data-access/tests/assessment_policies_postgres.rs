#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for current Assessment policy-only persistence.

use learning_data_access::postgres::{PostgresLiveAssessmentStore, lazy_pool};
use learning_data_access::{
    AssessmentReleaseIssue, LiveAssessmentStore, SaveBaseAssessmentPolicyInput, SessionTokenHash,
    StoreError,
};
use question_model::{AssessmentEditNumber, AssessmentId, CourseInstanceId};
use sqlx::{Connection, Row};
use uuid::Uuid;

const ASSESSMENT_ENTRY: u128 = 0xdc02;
const MODULE: u128 = 0xdd02;
const BLUEPRINT_ASSESSMENT: u128 = 0xdd03;
const PUBLISHED_QUESTION: &str = "ABCD-8XEF";

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn token() -> SessionTokenHash {
    SessionTokenHash::compute(&[0xd1; 32])
}

fn policy(expected_edit_number: u64, instructions: &str) -> SaveBaseAssessmentPolicyInput {
    let mut input: SaveBaseAssessmentPolicyInput = serde_json::from_value(serde_json::json!({
        "expectedEditNumber": expected_edit_number.to_string(),
        "instructions": instructions,
        "assessmentAttemptTimeLimitSeconds": 600
    }))
    .expect("closed policy fixture");
    input.expected_edit_number =
        AssessmentEditNumber::new(expected_edit_number).expect("positive fixture Edit Number");
    input
}

async fn seed(admin: &sqlx::postgres::PgPool) -> (CourseInstanceId, AssessmentId) {
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
    let instructor_id: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, product_role, created_at) \
         VALUES ('U00000009', 'instructor', clock_timestamp()) RETURNING account_id",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("Instructor account");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, product_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'instructor', decode($3, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour')",
    )
    .bind(id(0xda02))
    .bind(&instructor_id)
    .bind(token().to_string())
    .execute(&mut *tx)
    .await
    .expect("Instructor session");

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
    .expect("published Question");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_tuple \
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
                 $1, 'POL-1', 'Policy oracle Blueprint', 1, clock_timestamp(), \
                 '00000000-0000-0000-0000-00000000cc01', ARRAY[]::text[]) \
         RETURNING blueprint_course_id",
    )
    .bind(&instructor_id)
    .fetch_one(&mut *tx)
    .await
    .expect("Blueprint");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_course_revision \
         (blueprint_course_id, blueprint_revision_number, content, content_checksum, saved_at) \
         VALUES ($1, 1, '{}'::jsonb, decode(repeat('0', 64), 'hex'), clock_timestamp())",
    )
    .bind(&blueprint_id)
    .execute(&mut *tx)
    .await
    .expect("Blueprint Revision");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_module \
         (blueprint_course_id, blueprint_revision_number, blueprint_module_id, \
          module_position) VALUES ($1, 1, $2, 1)",
    )
    .bind(&blueprint_id)
    .bind(id(MODULE))
    .execute(&mut *tx)
    .await
    .expect("Blueprint Module");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_assessment \
         (blueprint_course_id, blueprint_revision_number, blueprint_module_id, \
          blueprint_assessment_id, assessment_position) VALUES ($1, 1, $2, $3, 1)",
    )
    .bind(&blueprint_id)
    .bind(id(MODULE))
    .bind(id(BLUEPRINT_ASSESSMENT))
    .execute(&mut *tx)
    .await
    .expect("Blueprint Assessment");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_event \
         (blueprint_course_id, blueprint_revision_number, actor_account_id, request_checksum, \
          occurred_at) VALUES ($1, 1, $2, decode(repeat('bd', 32), 'hex'), clock_timestamp())",
    )
    .bind(&blueprint_id)
    .bind(&instructor_id)
    .execute(&mut *tx)
    .await
    .expect("Blueprint Revision event");
    let course_id: String = sqlx::query_scalar(
        "INSERT INTO ple_data.course_instance \
         (course_instance_id, source_kind, blueprint_course_id, blueprint_revision_number, \
          course_short_name, course_long_name, term_starts_on, term_ends_on, created_at, \
          content_discipline_id, tags) \
         VALUES ('CI0000000' || ple_private.crockford_checksum_character('CI0000000'), \
                 'adopted', $1, 1, 'POL-1', 'Policy oracle Course', current_date, \
                 current_date + 1, clock_timestamp(), \
                 '00000000-0000-0000-0000-00000000cc01', ARRAY[]::text[]) \
         RETURNING course_instance_id",
    )
    .bind(&blueprint_id)
    .fetch_one(&mut *tx)
    .await
    .expect("Course");
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (course_membership_id, course_instance_id, account_id, role, student_record_id, \
          joined_at) VALUES ($1, $2, $3, 'instructor', NULL, clock_timestamp())",
    )
    .bind(id(0xdb02))
    .bind(&course_id)
    .bind(&instructor_id)
    .execute(&mut *tx)
    .await
    .expect("Instructor membership");

    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("Assessment fixture role");
    let snapshot_id: Vec<u8> = sqlx::query_scalar(
        "SELECT ple_private.ensure_assessment_policy_snapshot( \
             'Title must survive policy save', 'before policy save', \
             NULL, NULL, NULL, 300, 1, 'reject', 'new_variation', 'shuffled', \
             'after_submit', 'after_submit', 'after_submit', \
             'after_submit', 'after_submit', 'after_submit', 'quiz')",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("Assessment policy snapshot");
    let assessment_id: String = sqlx::query_scalar(
        "INSERT INTO ple_data.assessment \
         (assessment_id, course_instance_id, origin_kind, source_blueprint_course_id, \
          source_blueprint_revision_number, source_blueprint_assessment_id, \
          created_at, updated_at, assessment_type, assessment_policy_snapshot_id) \
         VALUES ('A0000000' || ple_private.crockford_checksum_character('A0000000'), \
                 $1, 'adopted', $2, 1, $3, clock_timestamp(), clock_timestamp(), \
                 'quiz', $4) \
         RETURNING assessment_id",
    )
    .bind(&course_id)
    .bind(&blueprint_id)
    .bind(id(BLUEPRINT_ASSESSMENT))
    .bind(&snapshot_id)
    .fetch_one(&mut *tx)
    .await
    .expect("Assessment");
    sqlx::query(
        "INSERT INTO ple_data.assessment_entry \
         (assessment_entry_id, assessment_id, authored_position, entry_kind, availability, \
          scoring_rule) VALUES ($1, $2, 0, 'fixed_question', 'available', 'normal')",
    )
    .bind(id(ASSESSMENT_ENTRY))
    .bind(&assessment_id)
    .execute(&mut *tx)
    .await
    .expect("Assessment Entry");
    sqlx::query(
        "INSERT INTO ple_data.assessment_entry_question \
         (assessment_entry_id, assessment_id, published_question_id, question_revision_number, \
          points_possible) VALUES ($1, $2, $3, 1, 2)",
    )
    .bind(id(ASSESSMENT_ENTRY))
    .bind(&assessment_id)
    .bind(PUBLISHED_QUESTION)
    .execute(&mut *tx)
    .await
    .expect("Assessment Entry Question");
    tx.commit().await.expect("fixture commit");
    (
        CourseInstanceId::new(course_id).expect("Course Instance ID"),
        AssessmentId::new(assessment_id).expect("Assessment ID"),
    )
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
async fn policy_save_is_isolated_conflict_checked_and_reports_unreleased_invalid_dates() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let migration_url = runtime.migration_url().expose();
    let admin = lazy_pool(migration_url).expect("migration pool");
    let (course, assessment) = seed(&admin).await;
    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let store =
        PostgresLiveAssessmentStore::new(lazy_pool(&application_url).expect("application pool"));
    let mut inspection = sqlx::postgres::PgConnection::connect(migration_url)
        .await
        .expect("inspection connection");
    sqlx::query("SET ROLE ple_data_owner")
        .execute(&mut inspection)
        .await
        .expect("inspection role");
    let saved = store
        .save_base_assessment_policy(
            token(),
            course.clone(),
            assessment.clone(),
            policy(1, "persisted policy"),
        )
        .await
        .expect("policy save");
    assert_eq!(saved.instructions.as_str(), "persisted policy");
    assert_eq!(saved.edit_number.value(), 2);

    let row = sqlx::query(
        "SELECT snapshot.assessment_title, snapshot.assessment_instructions, \
                assessment.assessment_edit_number \
           FROM ple_data.assessment AS assessment \
           JOIN ple_data.assessment_policy_snapshot AS snapshot \
             ON snapshot.assessment_policy_snapshot_id = \
                assessment.assessment_policy_snapshot_id \
          WHERE assessment.assessment_id = $1",
    )
    .bind(assessment.as_str())
    .fetch_one(&mut inspection)
    .await
    .expect("Assessment inspection");
    assert_eq!(
        row.try_get::<String, _>("assessment_title").expect("title"),
        "Title must survive policy save"
    );
    assert_eq!(
        row.try_get::<String, _>("assessment_instructions")
            .expect("instructions"),
        "persisted policy"
    );
    assert_eq!(
        row.try_get::<i64, _>("assessment_edit_number")
            .expect("edit number"),
        2
    );
    let entries: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_data.assessment_entry WHERE assessment_id = $1",
    )
    .bind(assessment.as_str())
    .fetch_one(&mut inspection)
    .await
    .expect("Entries inspection");
    assert_eq!(
        entries, 1,
        "policy save preserves the normalized Question Entries"
    );

    assert!(matches!(
        store
            .save_base_assessment_policy(
                token(),
                course.clone(),
                assessment.clone(),
                policy(1, "stale policy"),
            )
            .await,
        Err(StoreError::RetryableTransaction) | Err(StoreError::Conflict)
    ));
    let mut invalid_ordering: SaveBaseAssessmentPolicyInput =
        serde_json::from_value(serde_json::json!({
            "expectedEditNumber": "2", "instructions": "invalid ordering",
            "dueAt": "2026-09-12T12:00:00.000", "closesAt": "2026-09-12T11:00:00.000"
        }))
        .expect("closed invalid policy fixture");
    invalid_ordering.expected_edit_number =
        AssessmentEditNumber::new(2).expect("fixture Edit Number");
    let invalid_saved = store
        .save_base_assessment_policy(
            token(),
            course.clone(),
            assessment.clone(),
            invalid_ordering,
        )
        .await
        .expect("Unreleased invalid dates remain correctable");
    assert_eq!(invalid_saved.edit_number.value(), 3);
    let validation = store
        .validate_live_assessment_release(token(), course, assessment)
        .await
        .expect("interactive release validation");
    assert!(!validation.can_release);
    assert!(
        validation
            .issues
            .contains(&AssessmentReleaseIssue::DueDateAfterClose),
        "interactive readiness reports the actionable invalid date order"
    );
}
