#![cfg(feature = "postgres")]

//! Connected two-connection proofs for direct Assignment Attempt finalization.

use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

const STUDENT_ACCOUNT: &str = "00000000-0000-0000-0000-00000000eb05";
const STUDENT_RECORD: &str = "00000000-0000-0000-0000-00000000eb06";
const BASE_ASSIGNMENT: &str = "00000000-0000-0000-0000-00000000ed01";
const BASE_ENTRY: &str = "00000000-0000-0000-0000-00000000ed02";

async fn migration_pool() -> PgPool {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect_lazy(runtime.migration_url().expose())
        .expect("two-connection migration pool")
}

async fn set_student(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<(), String> {
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut **tx)
        .await
        .map_err(|error| format!("Student API role: {error}"))?;
    sqlx::query("SELECT set_config('ple.session_account_id', $1, true)")
        .bind(STUDENT_ACCOUNT)
        .execute(&mut **tx)
        .await
        .map_err(|error| format!("Student session: {error}"))?;
    Ok(())
}

async fn make_attempt(
    pool: &PgPool,
    assignment_id: Uuid,
    entry_id: Uuid,
    attempt_id: Uuid,
    issued_id: Uuid,
    question_attempt_id: Uuid,
) -> i64 {
    let mut tx = pool.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("data fixture role");
    sqlx::query(
        "INSERT INTO ple_data.assignment (assignment_id, course_id, \
         source_blueprint_course_reference_number, source_blueprint_revision_number, \
         source_blueprint_assignment_reference, created_at, updated_at, assignment_title, \
         assignment_instructions, available_at, due_at, closes_at, \
         assignment_attempt_time_limit_seconds, attempt_limit, late_work_rule, \
         assignment_completion_rule, assignment_attempt_grade_rule, \
         assignment_attempt_continuation_rule, question_pool_reuse_rule, question_variation_rule, \
         assignment_attempt_resume_rule, assignment_question_display_rule, \
         assignment_navigation_rule, assignment_question_order_rule, feedback_score, \
         feedback_per_item_correctness, feedback_submitted_response, feedback_question_feedback, \
         feedback_question_answer, feedback_question_answer_explanation, feedback_class_statistics, \
         assignment_status) \
         SELECT $1, course_id, source_blueprint_course_reference_number, \
                source_blueprint_revision_number, source_blueprint_assignment_reference, \
                clock_timestamp(), clock_timestamp(), assignment_title, assignment_instructions, \
                clock_timestamp() - interval '1 hour', clock_timestamp() + interval '1 hour', \
                clock_timestamp() + interval '2 hours', 60, 1, late_work_rule, \
                assignment_completion_rule, assignment_attempt_grade_rule, \
                assignment_attempt_continuation_rule, question_pool_reuse_rule, question_variation_rule, \
                assignment_attempt_resume_rule, assignment_question_display_rule, \
                assignment_navigation_rule, assignment_question_order_rule, feedback_score, \
                feedback_per_item_correctness, feedback_submitted_response, feedback_question_feedback, \
                feedback_question_answer, feedback_question_answer_explanation, feedback_class_statistics, \
                assignment_status FROM ple_data.assignment WHERE assignment_id = $2",
    ).bind(assignment_id).bind(Uuid::parse_str(BASE_ASSIGNMENT).unwrap())
        .execute(&mut *tx).await.expect("cloned Assignment");
    sqlx::query(
        "INSERT INTO ple_data.assignment_entry (assignment_entry_id, assignment_id, authored_position, \
         entry_kind, availability, scoring_rule, question_id, question_revision_number, points_possible) \
         SELECT $1, $2, authored_position, entry_kind, availability, scoring_rule, question_id, \
                question_revision_number, points_possible FROM ple_data.assignment_entry \
          WHERE assignment_entry_id = $3",
    ).bind(entry_id).bind(assignment_id).bind(Uuid::parse_str(BASE_ENTRY).unwrap())
        .execute(&mut *tx).await.expect("cloned Entry");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("private fixture role");
    sqlx::query(
        "INSERT INTO ple_private.student_assignment_accommodation (accommodation_id, student_record_id, \
         assignment_id, available_at, due_at, closes_at, assignment_attempt_time_limit_seconds, \
         attempt_limit, created_at) VALUES ($1, $2, $3, clock_timestamp() - interval '1 hour', \
         clock_timestamp() + interval '1 hour', clock_timestamp() + interval '2 hours', 60, 1, clock_timestamp())",
    ).bind(Uuid::from_u128(assignment_id.as_u128() + 0x100)).bind(Uuid::parse_str(STUDENT_RECORD).unwrap()).bind(assignment_id)
        .execute(&mut *tx).await.expect("Student accommodation");
    set_student(&mut tx).await.expect("Student API session");
    let started_attempt_id: Uuid = sqlx::query_scalar(
        "SELECT assignment_attempt_id FROM ple_api.start_assignment_attempt(\
         $1, $2, $3, '[]'::jsonb, jsonb_build_array(jsonb_build_object(\
         'issued_question_id', $4, 'assignment_entry_id', $5, 'issued_position', 0, \
         'question_id', 'BCDEFG0', 'revision_number', 1, 'question_seed', '501')))",
    )
    .bind(attempt_id)
    .bind(Uuid::parse_str(STUDENT_RECORD).unwrap())
    .bind(assignment_id)
    .bind(issued_id)
    .bind(entry_id)
    .fetch_one(&mut *tx)
    .await
    .expect("start Attempt");
    let reference: i64 = sqlx::query_scalar(
        "SELECT assignment_attempt_reference_number \
         FROM ple_api.read_started_student_assignment_attempt($1)",
    )
    .bind(started_attempt_id)
    .fetch_one(&mut *tx)
    .await
    .expect("started Attempt reference");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("question fixture role");
    sqlx::query(
        "INSERT INTO ple_private.question_attempt (question_attempt_id, issued_question_id, question_seed, \
         generated_parameter_sha256, issued_at, question_attempt_state, backend_name, backend_version, \
         grader_name, grader_version, rendered_question_sha256, issued_capability) \
         VALUES ($1, $2, 501, repeat('51', 32), clock_timestamp(), 'open', \
         'ple', '1', 'ple', '1', decode(repeat('51', 32), 'hex'), 'not_applicable')",
    ).bind(question_attempt_id).bind(issued_id).execute(&mut *tx).await.expect("Question Attempt");
    set_student(&mut tx).await.expect("Student save session");
    let initial_state: String = sqlx::query_scalar("SELECT response_state FROM ple_api.save_student_assignment_attempt_response($1, 1, '{\"kind\":\"shortText\",\"text\":\"saved\"}'::jsonb)")
        .bind(reference).fetch_one(&mut *tx).await.expect("initial saved response");
    assert_eq!(
        initial_state, "saved",
        "fixture response reached saved work"
    );
    tx.commit().await.expect("committed fixture");
    reference
}

async fn wait_for_lock<T: std::fmt::Debug>(
    observer: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    application_name: &str,
    operation: &str,
    task: &mut tokio::task::JoinHandle<Result<T, String>>,
) {
    for _ in 0..100 {
        if task.is_finished() {
            match task.await {
                Ok(Ok(value)) => {
                    panic!("{operation} completed before its expected lock: {value:?}")
                }
                Ok(Err(error)) => panic!("{operation} failed before its expected lock: {error}"),
                Err(error) => panic!("{operation} task ended before its expected lock: {error}"),
            }
        }
        sqlx::query("SELECT pg_stat_clear_snapshot()")
            .execute(&mut **observer)
            .await
            .expect("refresh lock observation snapshot");
        let waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE application_name = $1 \
             AND pg_backend_pid() = ANY(pg_blocking_pids(pid)))",
        )
        .bind(application_name)
        .fetch_one(&mut **observer)
        .await
        .expect("lock observation");
        if waiting {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    sqlx::query("SELECT pg_stat_clear_snapshot()")
        .execute(&mut **observer)
        .await
        .expect("refresh lock-timeout observation snapshot");
    let sessions: String = sqlx::query_scalar(
        "SELECT coalesce(json_agg(json_build_object(\
             'pid', activity.pid, \
             'applicationName', activity.application_name, \
             'state', activity.state, \
             'waitEventType', activity.wait_event_type, \
             'waitEvent', activity.wait_event, \
             'blockingPids', pg_blocking_pids(activity.pid)\
         ) ORDER BY activity.pid)::text, '[]') \
           FROM pg_stat_activity AS activity \
          WHERE activity.datname = current_database()",
    )
    .fetch_one(&mut **observer)
    .await
    .expect("redacted lock-timeout observation");
    panic!("{operation} did not reach its expected database lock; sessions {sessions}");
}

async fn arm_short_expiry(pool: &PgPool, attempt_id: Uuid) {
    let mut tx = pool.begin().await.expect("expiry seam transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("expiry seam role");
    sqlx::query("ALTER TABLE ple_private.assignment_attempt DISABLE TRIGGER assignment_attempt_retains_evidence")
        .execute(&mut *tx)
        .await
        .expect("disable immutable Attempt trigger for expiry seam");
    sqlx::query(
        "UPDATE ple_private.assignment_attempt SET expires_at = clock_timestamp() + interval '2 seconds' \
         WHERE assignment_attempt_id = $1",
    )
    .bind(attempt_id)
    .execute(&mut *tx)
    .await
    .expect("persisted short expiry");
    sqlx::query("ALTER TABLE ple_private.assignment_attempt ENABLE TRIGGER assignment_attempt_retains_evidence")
        .execute(&mut *tx)
        .await
        .expect("restore immutable Attempt trigger");
    tx.commit().await.expect("armed short expiry");
}

async fn wait_until_expired(
    observer: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    attempt_id: Uuid,
) {
    for _ in 0..1_000 {
        let expired: bool = sqlx::query_scalar(
            "SELECT clock_timestamp() >= expires_at FROM ple_private.assignment_attempt \
             WHERE assignment_attempt_id = $1",
        )
        .bind(attempt_id)
        .fetch_one(&mut **observer)
        .await
        .expect("server expiry observation");
        if expired {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("server clock did not reach the armed Assignment Attempt expiry");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
async fn late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks() {
    let pool = migration_pool().await;
    let late_assignment = Uuid::from_u128(0xf5100000000000000000000000000001);
    let commit_assignment = Uuid::from_u128(0xf5100000000000000000000000002);
    let late_attempt = Uuid::from_u128(0xf5200000000000000000000000000001);
    let commit_attempt = Uuid::from_u128(0xf5200000000000000000000000000002);
    let late_reference = make_attempt(
        &pool,
        late_assignment,
        Uuid::from_u128(0xf5300000000000000000000000000001),
        late_attempt,
        Uuid::from_u128(0xf5400000000000000000000000000001),
        Uuid::from_u128(0xf5500000000000000000000000000001),
    )
    .await;
    let commit_reference = make_attempt(
        &pool,
        commit_assignment,
        Uuid::from_u128(0xf5300000000000000000000000000002),
        commit_attempt,
        Uuid::from_u128(0xf5400000000000000000000000000002),
        Uuid::from_u128(0xf5500000000000000000000000000002),
    )
    .await;
    let mut save_tx = pool.begin().await.expect("late save transaction");
    sqlx::query("SET LOCAL application_name = 'direct-finalization-late-save'")
        .execute(&mut *save_tx)
        .await
        .expect("late save application name");
    set_student(&mut save_tx)
        .await
        .expect("late save Student session");
    arm_short_expiry(&pool, late_attempt).await;

    let mut row_lock = pool.begin().await.expect("row lock transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *row_lock)
        .await
        .expect("row lock role");
    let locked_question_attempt: Uuid = sqlx::query_scalar(
        "SELECT question_attempt_id FROM ple_private.question_attempt \
         WHERE question_attempt_id = $1 FOR UPDATE",
    )
    .bind(Uuid::from_u128(0xf5500000000000000000000000000001))
    .fetch_one(&mut *row_lock)
    .await
    .expect("Question Attempt lock");
    assert_eq!(
        locked_question_attempt,
        Uuid::from_u128(0xf5500000000000000000000000000001),
        "late save lock targets its issued Question Attempt"
    );
    let mut save = tokio::spawn(async move {
        let mut tx = save_tx;
        let state: String = sqlx::query_scalar("SELECT response_state FROM ple_api.save_student_assignment_attempt_response($1, 1, '{\"kind\":\"shortText\",\"text\":\"late\"}'::jsonb)")
            .bind(late_reference)
            .fetch_one(&mut *tx)
            .await
            .map_err(|error| format!("late save request: {error}"))?;
        tx.commit()
            .await
            .map_err(|error| format!("late save commit: {error}"))?;
        Ok(state)
    });
    wait_for_lock(
        &mut row_lock,
        "direct-finalization-late-save",
        "late save",
        &mut save,
    )
    .await;
    wait_until_expired(&mut row_lock, late_attempt).await;
    row_lock
        .commit()
        .await
        .expect("release Question Attempt lock");
    assert_eq!(
        save.await
            .expect("late save task")
            .expect("late save result"),
        "expired"
    );

    let mut commit_tx = pool.begin().await.expect("Student commit transaction");
    sqlx::query("SET LOCAL application_name = 'direct-finalization-commit'")
        .execute(&mut *commit_tx)
        .await
        .expect("Student commit application name");
    set_student(&mut commit_tx)
        .await
        .expect("Student commit session");
    arm_short_expiry(&pool, commit_attempt).await;
    let mut root_lock = pool.begin().await.expect("root lock transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *root_lock)
        .await
        .expect("root lock role");
    sqlx::query("SELECT ple_private.lock_assignment_for_student_work($1)")
        .bind(commit_assignment)
        .execute(&mut *root_lock)
        .await
        .expect("Assignment root lock");
    let mut commit = tokio::spawn(async move {
        let mut tx = commit_tx;
        let result = sqlx::query(
            "SELECT * FROM ple_api.commit_student_assignment_attempt_finalization(\
             $1, 'student', (\
                 SELECT jsonb_agg(jsonb_build_object(\
                     'question_attempt_id', prepared.question_attempt_id, \
                     'saved_at_millis', prepared.saved_at_millis, \
                     'student_response', prepared.student_response, \
                     'normalized_credit', 1\
                 ) ORDER BY prepared.question_attempt_id) \
                   FROM ple_api.prepare_student_assignment_attempt_finalization($1) AS prepared \
                  WHERE prepared.preparation_state = 'ready'\
             ))",
        )
        .bind(commit_reference)
        .execute(&mut *tx)
        .await;
        let sqlstate = match result {
            Ok(_) => return Err("Student commit unexpectedly succeeded after expiry".to_owned()),
            Err(sqlx::Error::Database(value)) => {
                let code = value.code().map(|code| code.into_owned());
                if code.as_deref() != Some("42501") {
                    return Err(format!(
                        "Student commit unexpected SQLSTATE {}, message {}",
                        code.as_deref().unwrap_or("unknown"),
                        value.message()
                    ));
                }
                code
            }
            Err(error) => return Err(format!("Student commit request: {error}")),
        };
        Ok(sqlstate)
    });
    wait_for_lock(
        &mut root_lock,
        "direct-finalization-commit",
        "Student commit",
        &mut commit,
    )
    .await;
    wait_until_expired(&mut root_lock, commit_attempt).await;
    root_lock
        .commit()
        .await
        .expect("release Assignment root lock");
    assert_eq!(
        commit
            .await
            .expect("commit task")
            .expect("commit task result")
            .as_deref(),
        Some("42501")
    );
    let mut evidence_tx = pool.begin().await.expect("evidence transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *evidence_tx)
        .await
        .expect("evidence role");
    let evidence: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_private.assignment_submission WHERE assignment_attempt_id = $1",
    )
    .bind(commit_attempt)
    .fetch_one(&mut *evidence_tx)
    .await
    .expect("commit evidence count");
    assert_eq!(
        evidence, 0,
        "late Student commit leaves no immutable submission"
    );
}
