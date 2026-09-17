#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for current Assessment policy-only persistence.

use learning_data_access::postgres::{PostgresLiveAssessmentStore, lazy_pool};
use learning_data_access::{
    AssessmentReleaseIssue, LiveAssessmentStore, SaveBaseAssessmentPolicyInput, SessionTokenHash,
    StoreError,
};
use question_model::{AssessmentEditNumber, AssessmentReference, CourseInstanceReference};
use sqlx::{Connection, Row};
use uuid::Uuid;

const INSTRUCTOR: u128 = 0xda01;
const COURSE: u128 = 0xdb01;
const ASSESSMENT: u128 = 0xdc01;
const ASSESSMENT_ENTRY: u128 = 0xdc02;
const BLUEPRINT: u128 = 0xdd01;
const MODULE: u128 = 0xdd02;
const BLUEPRINT_ASSESSMENT: u128 = 0xdd03;

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

async fn seed(admin: &sqlx::postgres::PgPool) -> (CourseInstanceReference, AssessmentReference) {
    let mut tx = admin.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("classification fixture owner");
    sqlx::query("INSERT INTO ple_data.content_discipline (discipline_uuid, name) VALUES ('00000000-0000-0000-0000-00000000cc01', 'Course fixture discipline') ON CONFLICT (discipline_uuid) DO NOTHING").execute(&mut *tx).await.expect("explicit fixture Discipline");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("private fixture role");
    sqlx::query(
        "INSERT INTO ple_private.account (account_id, product_role, created_at) \
         VALUES ($1, 'instructor', clock_timestamp())",
    )
    .bind(id(INSTRUCTOR))
    .execute(&mut *tx)
    .await
    .expect("Instructor account");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, product_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'instructor', decode($3, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour')",
    )
    .bind(id(0xda02))
    .bind(id(INSTRUCTOR))
    .bind(token().to_string())
    .execute(&mut *tx)
    .await
    .expect("Instructor session");

    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("data fixture role");
    sqlx::query("INSERT INTO ple_data.published_question (question_id, created_at) VALUES ('ABCD-8XEF', clock_timestamp())")
        .execute(&mut *tx).await.expect("published Question");
    sqlx::query("INSERT INTO ple_data.question_revision (question_id, revision_number, backend, question_type, published_at) VALUES ('ABCD-8XEF', 1, 'ple', 'multipleChoice', clock_timestamp())")
        .execute(&mut *tx).await.expect("Question Revision");

    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *tx)
        .await
        .expect("API fixture role");
    sqlx::query("INSERT INTO ple_data.blueprint_course (blueprint_id, reference_number, owner_account_id, short_name, long_name, metadata_etag, created_at, discipline_uuid, tags) OVERRIDING SYSTEM VALUE VALUES ($1, 1, $2, 'POL-1', 'Policy oracle Blueprint', '00000000-0000-0000-0000-00000000bd02', clock_timestamp(), '00000000-0000-0000-0000-00000000cc01', ARRAY[]::text[])")
        .bind(id(BLUEPRINT)).bind(id(INSTRUCTOR)).execute(&mut *tx).await.expect("Blueprint");
    sqlx::query("INSERT INTO ple_data.blueprint_course_revision (blueprint_course_reference_number, blueprint_revision_number, content, content_checksum, saved_at) VALUES (1, 1, '{}'::jsonb, decode(repeat('0', 64), 'hex'), clock_timestamp())")
        .execute(&mut *tx).await.expect("Blueprint Revision");
    sqlx::query("INSERT INTO ple_data.blueprint_revision_module (blueprint_course_reference_number, blueprint_revision_number, blueprint_module_reference, module_position) VALUES (1, 1, $1, 1)")
        .bind(id(MODULE)).execute(&mut *tx).await.expect("Blueprint Module");
    sqlx::query("INSERT INTO ple_data.blueprint_revision_assessment (blueprint_course_reference_number, blueprint_revision_number, blueprint_module_reference, blueprint_assessment_reference, assessment_position) VALUES (1, 1, $1, $2, 1)")
        .bind(id(MODULE)).bind(id(BLUEPRINT_ASSESSMENT)).execute(&mut *tx).await.expect("Blueprint Assessment");
    sqlx::query("INSERT INTO ple_data.blueprint_revision_event (blueprint_course_reference_number, blueprint_revision_number, actor_account_id, request_checksum, occurred_at) VALUES (1, 1, $1, decode(repeat('bd', 32), 'hex'), clock_timestamp())")
        .bind(id(INSTRUCTOR)).execute(&mut *tx).await.expect("Blueprint Revision event");
    sqlx::query("INSERT INTO ple_data.course_instance (course_id, reference_number, source_kind, blueprint_course_reference_number, blueprint_revision_number, assigned_instructor_account_id, assigned_instructor_role, course_short_name, course_long_name, term_starts_on, term_ends_on, created_at, discipline_uuid, tags) OVERRIDING SYSTEM VALUE VALUES ($1, 1, 'adopted', 1, 1, $2, 'instructor', 'POL-1', 'Policy oracle Course', current_date, current_date + 1, clock_timestamp(), '00000000-0000-0000-0000-00000000cc01', ARRAY[]::text[])")
        .bind(id(COURSE)).bind(id(INSTRUCTOR)).execute(&mut *tx).await.expect("Course");
    sqlx::query("INSERT INTO ple_data.course_membership (membership_id, course_id, account_id, role, student_record_id, joined_at) VALUES ($1, $2, $3, 'instructor', NULL, clock_timestamp())")
        .bind(id(0xdb02)).bind(id(COURSE)).bind(id(INSTRUCTOR)).execute(&mut *tx).await.expect("Instructor membership");

    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("Assessment fixture role");
    sqlx::query("INSERT INTO ple_data.assessment (assessment_id, reference_number, course_id, origin_kind, source_blueprint_course_reference_number, source_blueprint_revision_number, source_blueprint_assessment_reference, created_at, updated_at, assessment_type, assessment_title, assessment_instructions, assessment_attempt_time_limit_seconds, assessment_attempt_limit, late_work_rule, question_variation_rule, assessment_question_order_rule, feedback_score, feedback_per_item_correctness, feedback_submitted_response, feedback_question_answer, feedback_question_answer_explanation, feedback_class_statistics) OVERRIDING SYSTEM VALUE VALUES ($1, 1, $2, 'adopted', 1, 1, $3, clock_timestamp(), clock_timestamp(), 'quiz', 'Title must survive policy save', 'before policy save', 300, 1, 'reject', 'new_variation', 'shuffled', 'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit')")
        .bind(id(ASSESSMENT)).bind(id(COURSE)).bind(id(BLUEPRINT_ASSESSMENT)).execute(&mut *tx).await.expect("Assessment");
    sqlx::query("INSERT INTO ple_data.assessment_entry (assessment_entry_id, assessment_id, authored_position, entry_kind, availability, scoring_rule, question_id, question_revision_number, points_possible) VALUES ($1, $2, 0, 'fixed_question', 'available', 'normal', 'ABCD-8XEF', 1, 2)")
        .bind(id(ASSESSMENT_ENTRY)).bind(id(ASSESSMENT)).execute(&mut *tx).await.expect("Assessment Entry");
    let course_public_reference: String = sqlx::query_scalar(
        "SELECT public_reference FROM ple_data.course_instance WHERE course_id = $1",
    )
    .bind(id(COURSE))
    .fetch_one(&mut *tx)
    .await
    .expect("Course public reference");
    let assessment_public_reference: String = sqlx::query_scalar(
        "SELECT public_reference FROM ple_data.assessment WHERE assessment_id = $1",
    )
    .bind(id(ASSESSMENT))
    .fetch_one(&mut *tx)
    .await
    .expect("Assessment public reference");
    tx.commit().await.expect("fixture commit");
    (
        CourseInstanceReference::new(course_public_reference).expect("Course reference"),
        AssessmentReference::new(assessment_public_reference).expect("Assessment reference"),
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

    let row = sqlx::query("SELECT assessment_title, assessment_instructions, assessment_edit_number FROM ple_data.assessment WHERE assessment_id = $1")
        .bind(id(ASSESSMENT)).fetch_one(&mut inspection).await.expect("Assessment inspection");
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
    .bind(id(ASSESSMENT))
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
