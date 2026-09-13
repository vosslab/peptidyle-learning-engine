#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for current Assignment policy-only persistence.

use learning_data_access::postgres::{PostgresLiveAssignmentStore, lazy_pool};
use learning_data_access::{
    LiveAssignmentStore, SaveBaseAssignmentPolicyInput, SessionTokenHash, StoreError,
};
use question_model::{AssignmentEditNumber, AssignmentReference, CourseInstanceReference};
use sqlx::{Connection, Row};
use uuid::Uuid;

const INSTRUCTOR: u128 = 0xda01;
const COURSE: u128 = 0xdb01;
const ASSIGNMENT: u128 = 0xdc01;
const ASSIGNMENT_ENTRY: u128 = 0xdc02;
const BLUEPRINT: u128 = 0xdd01;
const MODULE: u128 = 0xdd02;
const BLUEPRINT_ASSIGNMENT: u128 = 0xdd03;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn token() -> SessionTokenHash {
    SessionTokenHash::compute(&[0xd1; 32])
}

fn policy(expected_edit_number: u64, instructions: &str) -> SaveBaseAssignmentPolicyInput {
    let mut input: SaveBaseAssignmentPolicyInput = serde_json::from_value(serde_json::json!({
        "expectedEditNumber": expected_edit_number.to_string(),
        "instructions": instructions,
        "assignmentAttemptTimeLimitSeconds": 600
    }))
    .expect("closed policy fixture");
    input.expected_edit_number =
        AssignmentEditNumber::new(expected_edit_number).expect("positive fixture Edit Number");
    input
}

async fn seed(admin: &sqlx::postgres::PgPool) {
    let mut tx = admin.begin().await.expect("fixture transaction");
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
    sqlx::query("INSERT INTO ple_data.published_question (question_id, created_at) VALUES ('ABCDEF0', clock_timestamp())")
        .execute(&mut *tx).await.expect("published Question");
    sqlx::query("INSERT INTO ple_data.question_revision (question_id, revision_number, backend, question_type, published_at) VALUES ('ABCDEF0', 1, 'ple', 'multipleChoice', clock_timestamp())")
        .execute(&mut *tx).await.expect("Question Revision");

    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *tx)
        .await
        .expect("API fixture role");
    sqlx::query("INSERT INTO ple_data.blueprint_course (blueprint_id, reference_number, owner_account_id, created_at) OVERRIDING SYSTEM VALUE VALUES ($1, 1, $2, clock_timestamp())")
        .bind(id(BLUEPRINT)).bind(id(INSTRUCTOR)).execute(&mut *tx).await.expect("Blueprint");
    sqlx::query("INSERT INTO ple_data.blueprint_course_revision (blueprint_course_reference_number, blueprint_revision_number, title, content, content_checksum, published_at) VALUES (1, 1, 'Policy oracle Blueprint', '{}'::jsonb, decode(repeat('0', 64), 'hex'), clock_timestamp())")
        .execute(&mut *tx).await.expect("Blueprint Revision");
    sqlx::query("INSERT INTO ple_data.blueprint_revision_module (blueprint_course_reference_number, blueprint_revision_number, blueprint_module_reference, module_position) VALUES (1, 1, $1, 1)")
        .bind(id(MODULE)).execute(&mut *tx).await.expect("Blueprint Module");
    sqlx::query("INSERT INTO ple_data.blueprint_revision_assignment (blueprint_course_reference_number, blueprint_revision_number, blueprint_module_reference, blueprint_assignment_reference, assignment_position) VALUES (1, 1, $1, $2, 1)")
        .bind(id(MODULE)).bind(id(BLUEPRINT_ASSIGNMENT)).execute(&mut *tx).await.expect("Blueprint Assignment");
    sqlx::query("INSERT INTO ple_data.course_instance (course_id, reference_number, blueprint_course_reference_number, blueprint_revision_number, assigned_instructor_account_id, assigned_instructor_role, course_short_name, course_long_name, term_starts_on, term_ends_on, created_at) OVERRIDING SYSTEM VALUE VALUES ($1, 1, 1, 1, $2, 'instructor', 'POL-1', 'Policy oracle Course', current_date, current_date + 1, clock_timestamp())")
        .bind(id(COURSE)).bind(id(INSTRUCTOR)).execute(&mut *tx).await.expect("Course");
    sqlx::query("INSERT INTO ple_data.course_membership (membership_id, course_id, account_id, role, student_record_id, joined_at) VALUES ($1, $2, $3, 'instructor', NULL, clock_timestamp())")
        .bind(id(0xdb02)).bind(id(COURSE)).bind(id(INSTRUCTOR)).execute(&mut *tx).await.expect("Instructor membership");

    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("Assignment fixture role");
    sqlx::query("INSERT INTO ple_data.assignment (assignment_id, reference_number, course_id, source_blueprint_course_reference_number, source_blueprint_revision_number, source_blueprint_assignment_reference, created_at, updated_at, assignment_title, assignment_instructions, assignment_attempt_time_limit_seconds, late_work_rule, assignment_completion_rule, assignment_attempt_grade_rule, assignment_attempt_continuation_rule, question_pool_reuse_rule, question_variation_rule, assignment_attempt_resume_rule, assignment_question_display_rule, assignment_navigation_rule, assignment_question_order_rule, feedback_score, feedback_per_item_correctness, feedback_submitted_response, feedback_question_feedback, feedback_question_answer, feedback_question_answer_explanation, feedback_class_statistics) OVERRIDING SYSTEM VALUE VALUES ($1, 1, $2, 1, 1, $3, clock_timestamp(), clock_timestamp(), 'Title must survive policy save', 'before policy save', 300, 'reject', 'answer_all', 'highest', 'unlimited', 'reuse_selection', 'new_variation', 'resumable', 'one_question_at_a_time', 'free_navigation', 'shuffled', 'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit')")
        .bind(id(ASSIGNMENT)).bind(id(COURSE)).bind(id(BLUEPRINT_ASSIGNMENT)).execute(&mut *tx).await.expect("Assignment");
    sqlx::query("INSERT INTO ple_data.assignment_entry (assignment_entry_id, assignment_id, authored_position, entry_kind, availability, scoring_rule, question_id, question_revision_number, points_possible) VALUES ($1, $2, 0, 'fixed_question', 'available', 'normal', 'ABCDEF0', 1, 2)")
        .bind(id(ASSIGNMENT_ENTRY)).bind(id(ASSIGNMENT)).execute(&mut *tx).await.expect("Assignment Entry");
    tx.commit().await.expect("fixture commit");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
async fn policy_save_is_isolated_conflict_checked_and_revalidates_released_assignments() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let migration_url = runtime.migration_url().expose();
    let admin = lazy_pool(migration_url).expect("migration pool");
    seed(&admin).await;
    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let store =
        PostgresLiveAssignmentStore::new(lazy_pool(&application_url).expect("application pool"));
    let mut inspection = sqlx::postgres::PgConnection::connect(migration_url)
        .await
        .expect("inspection connection");
    sqlx::query("SET ROLE ple_data_owner")
        .execute(&mut inspection)
        .await
        .expect("inspection role");
    let course = CourseInstanceReference::new(1).expect("Course reference");
    let assignment = AssignmentReference::new(1).expect("Assignment reference");

    let saved = store
        .save_base_assignment_policy(token(), course, assignment, policy(1, "persisted policy"))
        .await
        .expect("policy save");
    assert_eq!(saved.instructions.as_str(), "persisted policy");
    assert_eq!(saved.edit_number.value(), 2);

    let row = sqlx::query("SELECT assignment_title, assignment_instructions, assignment_edit_number FROM ple_data.assignment WHERE assignment_id = $1")
        .bind(id(ASSIGNMENT)).fetch_one(&mut inspection).await.expect("Assignment inspection");
    assert_eq!(
        row.try_get::<String, _>("assignment_title").expect("title"),
        "Title must survive policy save"
    );
    assert_eq!(
        row.try_get::<String, _>("assignment_instructions")
            .expect("instructions"),
        "persisted policy"
    );
    assert_eq!(
        row.try_get::<i64, _>("assignment_edit_number")
            .expect("edit number"),
        2
    );
    let entries: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_data.assignment_entry WHERE assignment_id = $1",
    )
    .bind(id(ASSIGNMENT))
    .fetch_one(&mut inspection)
    .await
    .expect("Entries inspection");
    assert_eq!(
        entries, 1,
        "policy save preserves the normalized Question Entries"
    );

    assert!(matches!(
        store
            .save_base_assignment_policy(token(), course, assignment, policy(1, "stale policy"))
            .await,
        Err(StoreError::RetryableTransaction) | Err(StoreError::Conflict)
    ));
    let mut invalid_ordering: SaveBaseAssignmentPolicyInput =
        serde_json::from_value(serde_json::json!({
            "expectedEditNumber": "2", "instructions": "invalid ordering",
            "dueAt": "2026-09-12T12:00:00.000", "closesAt": "2026-09-12T11:00:00.000"
        }))
        .expect("closed invalid policy fixture");
    invalid_ordering.expected_edit_number =
        AssignmentEditNumber::new(2).expect("fixture Edit Number");
    assert!(matches!(
        store
            .save_base_assignment_policy(token(), course, assignment, invalid_ordering)
            .await,
        Err(StoreError::InvalidRecord(_))
    ));

    sqlx::query(
        "UPDATE ple_data.assignment SET assignment_status = 'released' WHERE assignment_id = $1",
    )
    .bind(id(ASSIGNMENT))
    .execute(&mut inspection)
    .await
    .expect("released fixture state");
    assert!(
        matches!(
            store
                .save_base_assignment_policy(
                    token(),
                    course,
                    assignment,
                    policy(2, "released policy")
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ),
        "a Released Assignment policy save repeats Assignment Release Validation"
    );
}
