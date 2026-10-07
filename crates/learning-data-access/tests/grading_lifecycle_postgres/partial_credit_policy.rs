use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{
    COURSE_INSTANCE_ID, PUBLISHED_QUESTION, STUDENT_RECORD, migration_pool, seed_grading_graph,
    set_student,
};

fn fixture_uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

#[derive(Clone)]
struct AssessmentFixture {
    assessment_id: String,
    attempt_ids: [Uuid; 2],
    entry_ids: [Uuid; 2],
    question_attempt_ids: [[Uuid; 2]; 2],
}

async fn seed_two_attempt_assessment(pool: &PgPool) -> AssessmentFixture {
    seed_grading_graph(pool).await;
    let course_id = COURSE_INSTANCE_ID.get().expect("seeded Course Instance");
    let student_record_id = Uuid::parse_str(STUDENT_RECORD).expect("Student Record ID");
    let assessment_id = "A0000001";
    let entry_ids = [
        fixture_uuid(0xf5700000000000000000000000000001),
        fixture_uuid(0xf5700000000000000000000000000002),
    ];
    let attempt_ids = [
        fixture_uuid(0xf5710000000000000000000000000001),
        fixture_uuid(0xf5710000000000000000000000000002),
    ];
    let question_attempt_ids = [
        [
            fixture_uuid(0xf5720000000000000000000000000001),
            fixture_uuid(0xf5720000000000000000000000000002),
        ],
        [
            fixture_uuid(0xf5730000000000000000000000000001),
            fixture_uuid(0xf5730000000000000000000000000002),
        ],
    ];
    let mut tx = pool.begin().await.expect("Assessment fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("Assessment data owner");
    let policy_snapshot_id: Vec<u8> = sqlx::query_scalar(
        "SELECT ple_private.ensure_assessment_policy_snapshot(\
             'Partial Credit Highest Attempt', 'Answer both Questions.', \
             clock_timestamp() - interval '1 hour', \
             clock_timestamp() + interval '1 hour', \
             clock_timestamp() + interval '2 hours', \
             60, 2, 'reject', 'reuse_variation', 'authored_order', \
             'after_submit', 'after_submit', 'after_submit', \
             'after_submit', 'after_submit', 'regular_assignment')",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("Assessment policy snapshot");
    let inserted_assessment_id: String = sqlx::query_scalar(
        "INSERT INTO ple_data.assessment \
         (assessment_id, course_instance_id, origin_kind, created_at, updated_at, \
          assessment_type, assessment_policy_snapshot_id, assessment_status) \
         VALUES ($1 || ple_private.crockford_checksum_character($1), $2, 'direct', \
                 clock_timestamp(), clock_timestamp(), 'regular_assignment', $3, 'released') \
         RETURNING assessment_id",
    )
    .bind(assessment_id)
    .bind(course_id)
    .bind(&policy_snapshot_id)
    .fetch_one(&mut *tx)
    .await
    .expect("released Assessment");
    for (position, entry_id) in entry_ids.iter().enumerate() {
        sqlx::query(
            "INSERT INTO ple_data.assessment_entry \
             (assessment_entry_id, assessment_id, authored_position, entry_kind, availability, \
              scoring_rule) VALUES ($1, $2, $3, 'fixed_question', 'available', 'normal')",
        )
        .bind(entry_id)
        .bind(&inserted_assessment_id)
        .bind(position as i32)
        .execute(&mut *tx)
        .await
        .expect("fixed-question Assessment Entry");
        sqlx::query(
            "INSERT INTO ple_data.assessment_entry_question \
             (assessment_entry_id, assessment_id, published_question_id, question_revision_number, \
              points_possible) VALUES ($1, $2, $3, 1, 2)",
        )
        .bind(entry_id)
        .bind(&inserted_assessment_id)
        .bind(PUBLISHED_QUESTION)
        .execute(&mut *tx)
        .await
        .expect("two-point Assessment Question");
    }
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("accommodation owner");
    sqlx::query(
        "INSERT INTO ple_private.student_assessment_accommodation \
         (accommodation_id, course_instance_id, student_record_id, assessment_id, \
          available_at, due_at, closes_at, time_multiplier, assessment_attempt_limit, \
          created_at) VALUES ($1, $2, $3, $4, clock_timestamp() - interval '1 hour', \
          clock_timestamp() + interval '1 hour', clock_timestamp() + interval '2 hours', \
          1, 2, pg_catalog.transaction_timestamp())",
    )
    .bind(fixture_uuid(0xf5740000000000000000000000000001))
    .bind(course_id)
    .bind(student_record_id)
    .bind(&inserted_assessment_id)
    .execute(&mut *tx)
    .await
    .expect("two-attempt Student accommodation");
    tx.commit().await.expect("Assessment fixture commit");

    let fixture = AssessmentFixture {
        assessment_id: inserted_assessment_id.clone(),
        attempt_ids,
        entry_ids,
        question_attempt_ids,
    };
    for (attempt_index, attempt_id) in fixture.attempt_ids.iter().enumerate() {
        let issued_ids = [
            fixture_uuid(0xf5750000000000000000000000000001 + (attempt_index * 2) as u128),
            fixture_uuid(0xf5750000000000000000000000000002 + (attempt_index * 2) as u128),
        ];
        let mut start_tx = pool.begin().await.expect("start Attempt transaction");
        set_student(&mut start_tx)
            .await
            .expect("Student start session");
        let started_attempt_id: Uuid = sqlx::query_scalar(
            "SELECT assessment_attempt_id FROM ple_api.start_assessment_attempt(\
             $1, $2, $3, '[]'::jsonb, jsonb_build_array(\
                 jsonb_build_object('issued_question_id', $4, 'assessment_entry_id', $6, \
                     'issued_position', 0, 'published_question_id', $7, 'revision_number', 1), \
                 jsonb_build_object('issued_question_id', $5, 'assessment_entry_id', $8, \
                     'issued_position', 1, 'published_question_id', $7, 'revision_number', 1)))",
        )
        .bind(attempt_id)
        .bind(student_record_id)
        .bind(&inserted_assessment_id)
        .bind(issued_ids[0])
        .bind(issued_ids[1])
        .bind(entry_ids[0])
        .bind(PUBLISHED_QUESTION)
        .bind(entry_ids[1])
        .fetch_one(&mut *start_tx)
        .await
        .expect("start same-Assessment Attempt");
        assert_eq!(started_attempt_id, *attempt_id);
        sqlx::query("SET LOCAL ROLE ple_private_owner")
            .execute(&mut *start_tx)
            .await
            .expect("Question Attempt owner");
        for position in 0..2 {
            sqlx::query(
                "INSERT INTO ple_private.question_attempt \
                 (course_instance_id, question_attempt_id, issued_question_id, issued_at, \
                  delivery_toolchain_id, rendered_question_sha256) \
                 SELECT issued.course_instance_id, $1, $2, clock_timestamp(), \
                        ple_private.ensure_delivery_toolchain(\
                            'ple', '1', NULL, NULL, 'ple', '1', 'not_applicable'), \
                        decode(repeat('51', 32), 'hex') \
                   FROM ple_private.issued_question AS issued \
                  WHERE issued.issued_question_id = $2",
            )
            .bind(question_attempt_ids[attempt_index][position])
            .bind(issued_ids[position])
            .execute(&mut *start_tx)
            .await
            .expect("synthetic graded Question Attempt");
        }
        set_student(&mut start_tx)
            .await
            .expect("Student response session");
        for position in 0..2 {
            let response_state: String = sqlx::query_scalar(
                "SELECT response_state FROM ple_api.save_student_assessment_attempt_response(\
                 $1, $2, '{\"kind\":\"shortText\",\"text\":\"saved\"}'::jsonb)",
            )
            .bind(started_attempt_id)
            .bind(position + 1)
            .fetch_one(&mut *start_tx)
            .await
            .expect("saved synthetic response");
            assert_eq!(response_state, "saved");
        }
        start_tx.commit().await.expect("started Attempt commit");
        // The start operation resumes an unfinished Attempt. Submit the first
        // one before starting the second so this fixture creates two distinct
        // submitted Attempts through the ordinary Student lifecycle.
        if attempt_index == 0 {
            submit_attempt(pool, &fixture, attempt_index, [0.75, 0.75]).await;
        }
    }
    fixture
}

async fn submit_attempt(
    pool: &PgPool,
    fixture: &AssessmentFixture,
    attempt_index: usize,
    credits: [f64; 2],
) {
    let attempt_id = fixture.attempt_ids[attempt_index];
    let mut tx = pool.begin().await.expect("finalize synthetic Attempt");
    set_student(&mut tx)
        .await
        .expect("Student finalization session");
    let prepared = sqlx::query(
        "SELECT question_attempt_id, saved_at_millis, student_response \
           FROM ple_api.prepare_student_assessment_attempt_finalization($1) \
          WHERE preparation_state = 'ready' ORDER BY question_attempt_id",
    )
    .bind(attempt_id)
    .fetch_all(&mut *tx)
    .await
    .expect("ready saved responses");
    assert_eq!(prepared.len(), 2, "both Questions have saved work");
    let mut evaluations = Vec::with_capacity(2);
    for row in prepared {
        let question_attempt_id: Uuid = row.try_get("question_attempt_id").unwrap();
        let slot = (0..2)
            .find(|index| {
                question_attempt_id == fixture.question_attempt_ids[attempt_index][*index]
            })
            .expect("fixture Question Attempt belongs to Attempt");
        evaluations.push(json!({
            "question_attempt_id": question_attempt_id,
            "saved_at_millis": row.try_get::<i64, _>("saved_at_millis").unwrap(),
            "student_response": row.try_get::<Value, _>("student_response").unwrap(),
            "normalized_credit": credits[slot],
        }));
    }
    sqlx::query(
        "SELECT * FROM ple_api.commit_student_assessment_attempt_finalization($1, 'student', $2)",
    )
    .bind(attempt_id)
    .bind(Value::Array(evaluations))
    .fetch_one(&mut *tx)
    .await
    .expect("store synthetic immutable credit outcomes");
    tx.commit().await.expect("submitted Attempt commit");
}

async fn save_partial_credit_policy(pool: &PgPool, fixture: &AssessmentFixture, enabled: bool) {
    let course_id = COURSE_INSTANCE_ID.get().expect("seeded Course Instance");
    let mut tx = pool.begin().await.expect("policy save transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("policy read role");
    let instructor_id: String = sqlx::query_scalar(
        "SELECT account_id FROM ple_data.course_membership \
          WHERE course_instance_id = $1 AND role = 'instructor'",
    )
    .bind(course_id)
    .fetch_one(&mut *tx)
    .await
    .expect("Course Instructor");
    let expected_edit: i64 = sqlx::query_scalar(
        "SELECT assessment_edit_number FROM ple_data.assessment WHERE assessment_id = $1",
    )
    .bind(&fixture.assessment_id)
    .fetch_one(&mut *tx)
    .await
    .expect("Assessment Edit Number");
    let mut policy: Value = sqlx::query_scalar(
        "SELECT jsonb_build_object( \
            'assessment_title', snapshot.assessment_title, \
            'assessment_instructions', snapshot.assessment_instructions, \
            'available_at', snapshot.available_at, 'due_at', snapshot.due_at, \
            'closes_at', snapshot.closes_at, \
            'assessment_attempt_time_limit_seconds', snapshot.assessment_attempt_time_limit_seconds, \
            'assessment_attempt_limit', snapshot.assessment_attempt_limit, \
            'late_work_rule', snapshot.late_work_rule, \
            'partial_credit_enabled', snapshot.partial_credit_enabled, \
            'question_variation_rule', snapshot.question_variation_rule, \
            'assessment_question_order_rule', snapshot.assessment_question_order_rule, \
            'feedback_per_item_correctness', snapshot.feedback_per_item_correctness, \
            'feedback_submitted_response', snapshot.feedback_submitted_response, \
            'feedback_question_answer', snapshot.feedback_question_answer, \
            'feedback_question_answer_explanation', snapshot.feedback_question_answer_explanation, \
            'feedback_class_statistics', snapshot.feedback_class_statistics, \
            'feedback_hints', snapshot.feedback_hints, \
            'feedback_worked_solutions', snapshot.feedback_worked_solutions) \
           FROM ple_data.assessment AS assessment \
           JOIN ple_data.assessment_policy_snapshot AS snapshot \
             ON snapshot.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id \
          WHERE assessment.assessment_id = $1",
    )
    .bind(&fixture.assessment_id)
    .fetch_one(&mut *tx)
    .await
    .expect("current policy");
    policy["partial_credit_enabled"] = json!(enabled);
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *tx)
        .await
        .expect("Assessment save role");
    sqlx::query("SELECT set_config('ple.session_account_id', $1, true)")
        .bind(instructor_id)
        .execute(&mut *tx)
        .await
        .expect("Instructor session");
    let edit_number: i64 = sqlx::query_scalar(
        "SELECT assessment_edit_number FROM ple_api.save_assessment( \
             $1, $2, $3, $4, jsonb_build_array( \
                 jsonb_build_object('assessmentEntryId', $5::text, 'kind', 'fixed_question', \
                     'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 0, \
                     'questionId', $7::text, 'revisionNumber', 1, 'pointsPossible', '2'), \
                 jsonb_build_object('assessmentEntryId', $6::text, 'kind', 'fixed_question', \
                     'availability', 'available', 'scoringRule', 'normal', 'authoredPosition', 1, \
                     'questionId', $7::text, 'revisionNumber', 1, 'pointsPossible', '2'))) ",
    )
    .bind(course_id)
    .bind(&fixture.assessment_id)
    .bind(expected_edit)
    .bind(policy)
    .bind(fixture.entry_ids[0])
    .bind(fixture.entry_ids[1])
    .bind(PUBLISHED_QUESTION)
    .fetch_one(&mut *tx)
    .await
    .expect("save current Assessment policy");
    assert!(
        edit_number > expected_edit,
        "policy edit advances Assessment"
    );
    tx.commit().await.expect("policy save commit");
}

async fn selected_gradebook_attempt(
    pool: &PgPool,
    fixture: &AssessmentFixture,
) -> (Uuid, f64, f64) {
    let mut tx = pool.begin().await.expect("gradebook evidence transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("gradebook projection role");
    let row = sqlx::query(
        "SELECT assessment_attempt_id, points_earned, points_possible \
           FROM ple_private.read_assessment_gradebook_evidence($1, $2)",
    )
    .bind(Uuid::parse_str(STUDENT_RECORD).expect("Student Record ID"))
    .bind(&fixture.assessment_id)
    .fetch_one(&mut *tx)
    .await
    .expect("existing highest-attempt gradebook projection");
    let selected = (
        row.get("assessment_attempt_id"),
        row.get("points_earned"),
        row.get("points_possible"),
    );
    tx.rollback().await.expect("gradebook observation rollback");
    selected
}

async fn retained_outcomes(
    pool: &PgPool,
    fixture: &AssessmentFixture,
) -> Vec<(Uuid, i32, f64, Vec<u8>, Vec<u8>)> {
    let mut tx = pool.begin().await.expect("retained outcome transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *tx)
        .await
        .expect("retained outcome role");
    let rows = sqlx::query(
        "SELECT result.question_attempt_id, issued.issued_position, \
                result.normalized_credit::float8 AS normalized_credit, \
                issued.assessment_entry_snapshot_id, \
                assessment_attempt.assessment_policy_snapshot_id \
           FROM ple_private.grading_result AS result \
           JOIN ple_private.question_attempt AS question_attempt \
             ON question_attempt.question_attempt_id = result.question_attempt_id \
           JOIN ple_private.issued_question AS issued \
             ON issued.issued_question_id = question_attempt.issued_question_id \
           JOIN ple_private.assessment_attempt AS assessment_attempt \
             ON assessment_attempt.assessment_attempt_id = issued.assessment_attempt_id \
          WHERE issued.assessment_attempt_id = ANY($1) \
          ORDER BY array_position($1, issued.assessment_attempt_id), issued.issued_position",
    )
    .bind(fixture.attempt_ids.to_vec())
    .fetch_all(&mut *tx)
    .await
    .expect("retained immutable outcomes and snapshots");
    let mut values = Vec::with_capacity(rows.len());
    for row in rows {
        values.push((
            row.get("question_attempt_id"),
            row.get("issued_position"),
            row.get("normalized_credit"),
            row.get("assessment_entry_snapshot_id"),
            row.get("assessment_policy_snapshot_id"),
        ));
    }
    tx.rollback().await.expect("outcome observation rollback");
    values
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn partial_credit_toggle_reorders_highest_submitted_attempt_from_retained_fractions() {
    let pool = migration_pool().await;
    let fixture = seed_two_attempt_assessment(&pool).await;
    submit_attempt(&pool, &fixture, 1, [1.0, 0.0]).await;
    let retained_before = retained_outcomes(&pool, &fixture).await;
    assert_eq!(retained_before.len(), 4);
    let retained_credits: Vec<f64> = retained_before.iter().map(|row| row.2).collect();
    assert_eq!(retained_credits, [0.75, 0.75, 1.0, 0.0]);

    assert_eq!(
        selected_gradebook_attempt(&pool, &fixture).await,
        (fixture.attempt_ids[0], 3.0, 4.0),
        "initial highest Attempt under the default partial-credit policy",
    );

    for (enabled, expected_highest, expected_points) in [
        (false, fixture.attempt_ids[1], 2.0),
        (true, fixture.attempt_ids[0], 3.0),
    ] {
        save_partial_credit_policy(&pool, &fixture, enabled).await;
        assert_eq!(
            selected_gradebook_attempt(&pool, &fixture).await,
            (expected_highest, expected_points, 4.0),
            "gradebook highest Attempt under partial_credit_enabled={enabled}",
        );
        assert_eq!(
            retained_outcomes(&pool, &fixture).await,
            retained_before,
            "policy edits retain immutable credit fractions and Entry snapshots",
        );
    }
}
