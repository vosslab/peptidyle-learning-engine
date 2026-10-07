#![cfg(feature = "postgres")]

//! Connected two-connection proofs for direct Assessment Attempt finalization.

use std::sync::OnceLock;
use std::time::Duration;

use adapter_ple::{PleQuestionBackend, ResolvedPleQuestionJsonSource};
use objects::{ObjectAddress, ObjectStore, PutObject, memory::MemoryObjectStore};
use question_model::{
    ObjectId, PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionRevisionNumber,
    SourceObjectChecksum, StudentResponse, Timestamp, response::ResponseItemId,
};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use uuid::Uuid;

const STUDENT_RECORD: &str = "00000000-0000-0000-0000-00000000f506";
const PUBLISHED_QUESTION: &str = "HJKM-PNPQ";

static STUDENT_ACCOUNT_ID: OnceLock<String> = OnceLock::new();
static COURSE_INSTANCE_ID: OnceLock<String> = OnceLock::new();

async fn migration_pool() -> PgPool {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect_lazy(runtime.migration_url().expose())
        .expect("two-connection migration pool")
}

// Protects partial-credit policy scoring and the independent Full Credit and
// unanswered-zero rules at the SQL scoring boundary.
#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn unanswered_scoring_preserves_evaluated_zero_credit_distinction() {
    let pool = migration_pool().await;
    let mut tx = pool.begin().await.expect("scoring contract transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("private scoring role");
    let mismatches: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM (VALUES \
         (NULL::numeric, 'full_credit', 8::numeric, false, 0::numeric), \
         (0::numeric, 'full_credit', 8::numeric, false, 8::numeric), \
         (0::numeric, 'normal', 8::numeric, true, 0::numeric), \
         (1::numeric, 'normal', 8::numeric, true, 8::numeric), \
         (0.5::numeric, 'normal', 8::numeric, true, 4::numeric), \
         (0.5::numeric, 'normal', 8::numeric, false, 0::numeric), \
         (1::numeric, 'normal', 8::numeric, false, 8::numeric) \
         ) AS expected(credit, rule, points, partial_credit_enabled, earned) \
         LEFT JOIN LATERAL ple_private.score_recorded_credit(\
             expected.credit, expected.rule::ple_data.scoring_rule, expected.points, \
             expected.partial_credit_enabled) AS actual ON true \
         WHERE actual.points_earned IS DISTINCT FROM expected.earned \
            OR actual.points_possible IS DISTINCT FROM expected.points",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("current-point scoring contract");
    assert_eq!(
        mismatches, 0,
        "unanswered work earns zero while evaluated credit retains its scoring treatment"
    );
    tx.rollback().await.expect("scoring contract rollback");
}

async fn set_student(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<(), String> {
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut **tx)
        .await
        .map_err(|error| format!("Student API role: {error}"))?;
    sqlx::query("SELECT set_config('ple.session_account_id', $1, true)")
        .bind(STUDENT_ACCOUNT_ID.get().expect("seeded Student Account"))
        .execute(&mut **tx)
        .await
        .map_err(|error| format!("Student session: {error}"))?;
    Ok(())
}

async fn seed_grading_graph(pool: &PgPool) {
    if STUDENT_ACCOUNT_ID.get().is_some() {
        return;
    }
    let mut tx = pool.begin().await.expect("grading seed transaction");
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
    sqlx::query(
        "INSERT INTO ple_data.published_question (published_question_id, created_at) \
         VALUES ($1, clock_timestamp()) ON CONFLICT (published_question_id) DO NOTHING",
    )
    .bind(PUBLISHED_QUESTION)
    .execute(&mut *tx)
    .await
    .expect("Published Question");
    sqlx::query(
        "INSERT INTO ple_data.question_revision \
         (published_question_id, revision_number, backend, published_at) \
         VALUES ($1, 1, 'ple', clock_timestamp()) \
         ON CONFLICT (published_question_id, revision_number) DO NOTHING",
    )
    .bind(PUBLISHED_QUESTION)
    .execute(&mut *tx)
    .await
    .expect("Question Revision");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("private fixture role");
    sqlx::query(
        "INSERT INTO ple_private.object_record \
         (object_record_id, object_address, object_storage_area, object_data_class, \
          sha256, size_bytes, media_type, created_at) \
         VALUES ($1, jsonb_build_object(\
                    'kind', 'questionSource', \
                    'publishedQuestionRevisionTuple', jsonb_build_object(\
                    'publishedQuestionId', $2, 'revisionNumber', 1), \
                    'objectId', $1), \
                 'private-content', 'question-source', decode(repeat('51', 32), 'hex'), \
                 1, 'application/json', pg_catalog.transaction_timestamp())",
    )
    .bind(Uuid::from_u128(0xf509))
    .bind(PUBLISHED_QUESTION)
    .execute(&mut *tx)
    .await
    .expect("Question source Object Record");
    sqlx::query(
        "INSERT INTO ple_private.question_revision_source_binding \
         (published_question_id, revision_number, backend, native_question_type, question_format, \
          source_object_record_id, source_object_checksum, created_at) \
         VALUES ($1, 1, 'ple', 'multipleChoice', 'pleQuestionJson', $2, repeat('51', 32), \
                 pg_catalog.transaction_timestamp())",
    )
    .bind(PUBLISHED_QUESTION)
    .bind(Uuid::from_u128(0xf509))
    .execute(&mut *tx)
    .await
    .expect("Question source binding");
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
        .expect("API fixture role");
    let course_id: String = sqlx::query_scalar(
        "INSERT INTO ple_data.course_instance \
         (course_instance_id, source_kind, course_short_name, course_long_name, \
          content_discipline_id, tags, term_starts_on, term_ends_on, created_at) \
         VALUES ('CI0000000' || ple_private.crockford_checksum_character('CI0000000'), \
                 'empty', 'GRADE', 'Grading lifecycle Course', \
                 '00000000-0000-0000-0000-00000000cc01', ARRAY[]::text[], \
                 current_date, current_date + 1, pg_catalog.transaction_timestamp()) \
         RETURNING course_instance_id",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("Course Instance");
    sqlx::query(
        "INSERT INTO ple_data.student_record \
         (student_record_id, course_instance_id, student_account_id, created_at) \
         VALUES ($1, $2, $3, pg_catalog.transaction_timestamp())",
    )
    .bind(Uuid::parse_str(STUDENT_RECORD).unwrap())
    .bind(&course_id)
    .bind(&student_id)
    .execute(&mut *tx)
    .await
    .expect("Student Record");
    sqlx::query(
        "INSERT INTO ple_data.course_membership \
         (course_membership_id, course_instance_id, account_id, role, student_record_id, \
          joined_at) VALUES ($1, $2, $3, 'instructor', NULL, clock_timestamp())",
    )
    .bind(Uuid::from_u128(0xf508))
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
    .bind(Uuid::from_u128(0xf507))
    .bind(&course_id)
    .bind(&student_id)
    .bind(Uuid::parse_str(STUDENT_RECORD).unwrap())
    .execute(&mut *tx)
    .await
    .expect("Student membership");
    tx.commit().await.expect("grading seed commit");
    STUDENT_ACCOUNT_ID
        .set(student_id)
        .expect("Student Account ID once");
    COURSE_INSTANCE_ID
        .set(course_id)
        .expect("Course Instance ID once");
}

struct AttemptFixture {
    attempt_id: Uuid,
    assessment_id: String,
}

async fn make_attempt(
    pool: &PgPool,
    entry_id: Uuid,
    attempt_id: Uuid,
    issued_id: Uuid,
    question_attempt_id: Uuid,
) -> AttemptFixture {
    seed_grading_graph(pool).await;
    let course_id = COURSE_INSTANCE_ID
        .get()
        .expect("seeded Course Instance")
        .as_str();
    let mut tx = pool.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("data fixture role");
    let snapshot_id: Vec<u8> = sqlx::query_scalar(
        "SELECT ple_private.ensure_assessment_policy_snapshot( \
             'Grading lifecycle Assessment', 'Answer the Question.', \
             clock_timestamp() - interval '1 hour', \
             clock_timestamp() + interval '1 hour', \
             clock_timestamp() + interval '2 hours', \
             60, 1, 'reject', 'reuse_variation', 'authored_order', \
             'after_submit', 'after_submit', 'after_submit', \
             'after_submit', 'after_submit', \
             'regular_assignment')",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("Assessment policy snapshot");
    let assessment_id: String = sqlx::query_scalar(
        "INSERT INTO ple_data.assessment \
         (assessment_id, course_instance_id, origin_kind, created_at, updated_at, \
          assessment_type, assessment_policy_snapshot_id, assessment_status) \
         VALUES ('A0000000' || ple_private.crockford_checksum_character('A0000000'), \
                 $1, 'direct', clock_timestamp(), clock_timestamp(), \
                 'regular_assignment', $2, 'released') \
         RETURNING assessment_id",
    )
    .bind(course_id)
    .bind(&snapshot_id)
    .fetch_one(&mut *tx)
    .await
    .expect("Assessment");
    sqlx::query(
        "INSERT INTO ple_data.assessment_entry \
         (assessment_entry_id, assessment_id, authored_position, entry_kind, availability, \
          scoring_rule) VALUES ($1, $2, 0, 'fixed_question', 'available', 'normal')",
    )
    .bind(entry_id)
    .bind(&assessment_id)
    .execute(&mut *tx)
    .await
    .expect("Assessment Entry");
    sqlx::query(
        "INSERT INTO ple_data.assessment_entry_question \
         (assessment_entry_id, assessment_id, published_question_id, question_revision_number, \
          points_possible) VALUES ($1, $2, $3, 1, 2)",
    )
    .bind(entry_id)
    .bind(&assessment_id)
    .bind(PUBLISHED_QUESTION)
    .execute(&mut *tx)
    .await
    .expect("Assessment Entry Question");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("private fixture role");
    sqlx::query(
        "INSERT INTO ple_private.student_assessment_accommodation \
         (accommodation_id, course_instance_id, student_record_id, assessment_id, \
          available_at, due_at, closes_at, time_multiplier, assessment_attempt_limit, \
          created_at) VALUES ($1, $2, $3, $4, clock_timestamp() - interval '1 hour', \
          clock_timestamp() + interval '1 hour', clock_timestamp() + interval '2 hours', \
          1, 1, pg_catalog.transaction_timestamp())",
    )
    .bind(Uuid::from_u128(entry_id.as_u128() + 0x100))
    .bind(course_id)
    .bind(Uuid::parse_str(STUDENT_RECORD).unwrap())
    .bind(&assessment_id)
    .execute(&mut *tx)
    .await
    .expect("Student accommodation");
    set_student(&mut tx).await.expect("Student API session");
    let started_attempt_id: Uuid = sqlx::query_scalar(
        "SELECT assessment_attempt_id FROM ple_api.start_assessment_attempt(\
         $1, $2, $3, '[]'::jsonb, jsonb_build_array(jsonb_build_object(\
         'issued_question_id', $4, 'assessment_entry_id', $5, 'issued_position', 0, \
         'published_question_id', $6, 'revision_number', 1)))",
    )
    .bind(attempt_id)
    .bind(Uuid::parse_str(STUDENT_RECORD).unwrap())
    .bind(&assessment_id)
    .bind(issued_id)
    .bind(entry_id)
    .bind(PUBLISHED_QUESTION)
    .fetch_one(&mut *tx)
    .await
    .expect("start Attempt");
    let attempt_id = started_attempt_id;
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("question fixture role");
    sqlx::query(
        "INSERT INTO ple_private.question_attempt (course_instance_id, question_attempt_id, issued_question_id, \
         issued_at, delivery_toolchain_id, rendered_question_sha256) \
         SELECT issued.course_instance_id, $1, $2, clock_timestamp(), \
                ple_private.ensure_delivery_toolchain('ple', '1', NULL, NULL, 'ple', '1', 'not_applicable'), \
                decode(repeat('51', 32), 'hex') \
           FROM ple_private.issued_question AS issued \
          WHERE issued.issued_question_id = $2",
    )
    .bind(question_attempt_id)
    .bind(issued_id)
    .execute(&mut *tx)
    .await
    .expect("Question Attempt");
    set_student(&mut tx).await.expect("Student save session");
    let initial_state: String = sqlx::query_scalar("SELECT response_state FROM ple_api.save_student_assessment_attempt_response($1, 1, '{\"kind\":\"shortText\",\"text\":\"saved\"}'::jsonb)")
        .bind(attempt_id).fetch_one(&mut *tx).await.expect("initial saved response");
    assert_eq!(
        initial_state, "saved",
        "fixture response reached saved work"
    );
    tx.commit().await.expect("committed fixture");
    AttemptFixture {
        attempt_id,
        assessment_id,
    }
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
    sqlx::query("ALTER TABLE ple_private.assessment_attempt DISABLE TRIGGER assessment_attempt_retains_evidence")
        .execute(&mut *tx)
        .await
        .expect("disable immutable Attempt trigger for expiry seam");
    sqlx::query(
        "UPDATE ple_private.assessment_attempt SET expires_at = clock_timestamp() + interval '2 seconds' \
         WHERE assessment_attempt_id = $1",
    )
    .bind(attempt_id)
    .execute(&mut *tx)
    .await
    .expect("persisted short expiry");
    sqlx::query("ALTER TABLE ple_private.assessment_attempt ENABLE TRIGGER assessment_attempt_retains_evidence")
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
            "SELECT clock_timestamp() >= expires_at FROM ple_private.assessment_attempt \
             WHERE assessment_attempt_id = $1",
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
    panic!("server clock did not reach the armed Assessment Attempt expiry");
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks() {
    let pool = migration_pool().await;
    let late_attempt = Uuid::from_u128(0xf5200000000000000000000000000001);
    let commit_attempt = Uuid::from_u128(0xf5200000000000000000000000000002);
    let late_fixture = make_attempt(
        &pool,
        Uuid::from_u128(0xf5300000000000000000000000000001),
        late_attempt,
        Uuid::from_u128(0xf5400000000000000000000000000001),
        Uuid::from_u128(0xf5500000000000000000000000000001),
    )
    .await;
    let commit_fixture = make_attempt(
        &pool,
        Uuid::from_u128(0xf5300000000000000000000000000002),
        commit_attempt,
        Uuid::from_u128(0xf5400000000000000000000000000002),
        Uuid::from_u128(0xf5500000000000000000000000000002),
    )
    .await;
    let late_attempt_id = late_fixture.attempt_id;
    let commit_attempt_id = commit_fixture.attempt_id;
    let commit_assessment = commit_fixture.assessment_id;
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
        let state: String = sqlx::query_scalar("SELECT response_state FROM ple_api.save_student_assessment_attempt_response($1, 1, '{\"kind\":\"shortText\",\"text\":\"late\"}'::jsonb)")
            .bind(late_attempt_id)
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
    sqlx::query("SELECT ple_private.lock_assessment_for_student_work($1)")
        .bind(&commit_assessment)
        .execute(&mut *root_lock)
        .await
        .expect("Assessment root lock");
    let mut commit = tokio::spawn(async move {
        let mut tx = commit_tx;
        let result = sqlx::query(
            "SELECT * FROM ple_api.commit_student_assessment_attempt_finalization(\
             $1, 'student', (\
                 SELECT jsonb_agg(jsonb_build_object(\
                     'question_attempt_id', prepared.question_attempt_id, \
                     'saved_at_millis', prepared.saved_at_millis, \
                     'student_response', prepared.student_response, \
                     'normalized_credit', 1\
                 ) ORDER BY prepared.question_attempt_id) \
                   FROM ple_api.prepare_student_assessment_attempt_finalization($1) AS prepared \
                  WHERE prepared.preparation_state = 'ready'\
             ))",
        )
        .bind(commit_attempt_id)
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
        .expect("release Assessment root lock");
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
        "SELECT count(*) FROM ple_private.assessment_submission WHERE assessment_attempt_id = $1",
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

const COLOR_QUESTION_JSON: &str = r#"{
  "format": "pleQuestionJson",
  "prompt": "What is my favorite color?",
  "response": {
    "kind": "singleChoice",
    "choices": [
      {"id": "blue", "text": "Blue", "feedback": "Blue is a calm choice."},
      {"id": "red", "text": "Red", "feedback": "Red is not my favorite."},
      {"id": "yellow", "text": "Yellow", "feedback": "Yellow is bright."}
    ],
    "correctChoice": "blue"
  },
  "feedback": {"correct": "Exactly right.", "incorrect": "Try thinking of a cool color."}
}"#;

async fn color_question_source() -> ResolvedPleQuestionJsonSource {
    let store = MemoryObjectStore::default();
    let published_question_revision_tuple = PublishedQuestionRevisionTuple {
        published_question_id: PublishedQuestionId::from_random_identifier("ABCDEFG")
            .expect("Question ID"),
        revision_number: QuestionRevisionNumber::new(1).expect("revision"),
    };
    let source_object_id = ObjectId::from_uuid(Uuid::from_u128(901));
    let record = store
        .put(PutObject {
            address: ObjectAddress::QuestionSource {
                published_question_revision_tuple: published_question_revision_tuple.clone(),
                object_id: source_object_id,
            },
            bytes: COLOR_QUESTION_JSON.as_bytes().to_vec(),
            media_type: adapter_ple::question_json::PLE_QUESTION_JSON_MEDIA_TYPE.to_string(),
            created_at: Timestamp::from_unix_millis(1),
        })
        .await
        .expect("Question source");
    ResolvedPleQuestionJsonSource::resolve(
        &store,
        published_question_revision_tuple,
        source_object_id,
        SourceObjectChecksum::parse(record.sha256.to_string()).expect("source checksum"),
    )
    .await
    .expect("compiled Question")
}

fn grade_credit(source: &ResolvedPleQuestionJsonSource, response: &StudentResponse) -> f64 {
    PleQuestionBackend::new()
        .grade_question_json(source, response)
        .expect("Question Backend grade")
        .evaluation
        .normalized_credit()
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn backend_returned_credit_is_stored_as_the_immutable_grading_outcome() {
    let source = color_question_source().await;
    let correct = StudentResponse::MultipleChoice {
        selected: vec![ResponseItemId::new("blue")],
    };
    let incorrect = StudentResponse::MultipleChoice {
        selected: vec![ResponseItemId::new("red")],
    };
    assert_eq!(grade_credit(&source, &correct), 1.0);
    assert_eq!(grade_credit(&source, &incorrect), 0.0);
    assert_eq!(
        grade_credit(&source, &incorrect),
        grade_credit(&source, &incorrect),
        "the same complete response returns the same credit fraction"
    );

    let pool = migration_pool().await;
    let attempt_id = Uuid::from_u128(0xf5600000000000000000000000000001);
    let fixture = make_attempt(
        &pool,
        Uuid::from_u128(0xf5610000000000000000000000000001),
        attempt_id,
        Uuid::from_u128(0xf5620000000000000000000000000001),
        Uuid::from_u128(0xf5630000000000000000000000000001),
    )
    .await;
    let saved_response = serde_json::to_value(&incorrect).expect("saved response JSON");
    let mut save_tx = pool.begin().await.expect("response transaction");
    set_student(&mut save_tx)
        .await
        .expect("Student save session");
    let saved_state: String = sqlx::query_scalar(
        "SELECT response_state FROM ple_api.save_student_assessment_attempt_response($1, 1, $2)",
    )
    .bind(fixture.attempt_id)
    .bind(&saved_response)
    .fetch_one(&mut *save_tx)
    .await
    .expect("saved graded response");
    assert_eq!(saved_state, "saved");
    save_tx.commit().await.expect("saved response commit");

    let mut commit_tx = pool.begin().await.expect("finalization transaction");
    set_student(&mut commit_tx)
        .await
        .expect("Student finalization session");
    let prepared = sqlx::query(
        "SELECT question_attempt_id, saved_at_millis, student_response \
         FROM ple_api.prepare_student_assessment_attempt_finalization($1) \
         WHERE preparation_state = 'ready'",
    )
    .bind(fixture.attempt_id)
    .fetch_one(&mut *commit_tx)
    .await
    .expect("ready saved response");
    let question_attempt_id: Uuid = prepared
        .try_get("question_attempt_id")
        .expect("Question Attempt");
    let saved_at_millis: i64 = prepared
        .try_get("saved_at_millis")
        .expect("saved response time");
    let prepared_response: serde_json::Value = prepared
        .try_get("student_response")
        .expect("prepared response");
    let graded_response: StudentResponse =
        serde_json::from_value(prepared_response.clone()).expect("prepared response decodes");
    let backend_credit = grade_credit(&source, &graded_response);
    assert_eq!(backend_credit, 0.0);
    let score = sqlx::query(
        "SELECT points_earned, points_possible \
         FROM ple_api.commit_student_assessment_attempt_finalization( \
             $1, 'student', jsonb_build_array(jsonb_build_object( \
                 'question_attempt_id', $2, \
                 'saved_at_millis', $3, \
                 'student_response', $4, \
                 'normalized_credit', $5::double precision)))",
    )
    .bind(fixture.attempt_id)
    .bind(question_attempt_id)
    .bind(saved_at_millis)
    .bind(&prepared_response)
    .bind(backend_credit)
    .fetch_one(&mut *commit_tx)
    .await
    .expect("finalization stores the backend credit");
    let points_earned: f64 = score.try_get("points_earned").expect("points earned");
    let points_possible: f64 = score.try_get("points_possible").expect("points possible");
    assert_eq!(points_earned, 0.0);
    assert_eq!(points_possible, 2.0);
    commit_tx.commit().await.expect("finalization commit");

    let mut stored_tx = pool.begin().await.expect("stored outcome transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *stored_tx)
        .await
        .expect("grading evidence role");
    let stored_credit: f64 = sqlx::query_scalar(
        "SELECT normalized_credit::float8 FROM ple_private.grading_result \
         WHERE question_attempt_id = $1",
    )
    .bind(question_attempt_id)
    .fetch_one(&mut *stored_tx)
    .await
    .expect("stored credit");
    assert_eq!(stored_credit, backend_credit);
    let rewrite = sqlx::query(
        "UPDATE ple_private.grading_result SET normalized_credit = 1 \
         WHERE question_attempt_id = $1",
    )
    .bind(question_attempt_id)
    .execute(&mut *stored_tx)
    .await;
    match rewrite {
        Err(sqlx::Error::Database(error)) => {
            assert_eq!(error.code().as_deref(), Some("55000"));
            assert_eq!(error.message(), "Grading evidence is immutable");
        }
        Ok(_) => panic!("stored grading credit was rewritten"),
        Err(error) => panic!("grading rewrite failed unexpectedly: {error}"),
    }
    stored_tx
        .rollback()
        .await
        .expect("grading rewrite observation rollback");
    let mut second_tx = pool.begin().await.expect("second outcome transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *second_tx)
        .await
        .expect("second outcome role");
    let second_record = sqlx::query(
        "SELECT ple_private.record_direct_automated_grading_result($1, $2::numeric, clock_timestamp())",
    )
    .bind(question_attempt_id)
    .bind(backend_credit)
    .execute(&mut *second_tx)
    .await;
    match second_record {
        Err(sqlx::Error::Database(error)) => {
            assert_eq!(error.code().as_deref(), Some("42501"));
            assert_eq!(
                error.message(),
                "Direct automated grading target is unavailable"
            );
        }
        Ok(_) => panic!("a second grading outcome was stored"),
        Err(error) => panic!("second grading outcome failed unexpectedly: {error}"),
    }
    second_tx
        .rollback()
        .await
        .expect("second outcome observation rollback");
}

#[path = "grading_lifecycle_postgres/grading_rescore_postgres.rs"]
mod grading_rescore_postgres;

#[path = "grading_lifecycle_postgres/partial_credit_policy.rs"]
mod partial_credit_policy;
