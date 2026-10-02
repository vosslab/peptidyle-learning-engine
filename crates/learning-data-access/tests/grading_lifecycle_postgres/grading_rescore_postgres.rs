use adapter_ple::ResolvedPleQuestionJsonSource;
use question_model::{StudentResponse, response::ResponseItemId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{
    AttemptFixture, COURSE_INSTANCE_ID, PUBLISHED_QUESTION, color_question_source, grade_credit,
    make_attempt, migration_pool, set_student,
};

async fn store_color_outcome(
    pool: &PgPool,
    source: &ResolvedPleQuestionJsonSource,
    response: &StudentResponse,
    entry_id: Uuid,
    attempt_id: Uuid,
    issued_id: Uuid,
    question_attempt_id: Uuid,
) -> (AttemptFixture, Uuid, f64) {
    let fixture = make_attempt(pool, entry_id, attempt_id, issued_id, question_attempt_id).await;
    let saved_response = serde_json::to_value(response).expect("saved response JSON");
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
    .expect("saved color response");
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
    let prepared_question_attempt_id: Uuid = prepared
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
    let backend_credit = grade_credit(source, &graded_response);
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
    .bind(prepared_question_attempt_id)
    .bind(saved_at_millis)
    .bind(&prepared_response)
    .bind(backend_credit)
    .fetch_one(&mut *commit_tx)
    .await
    .expect("finalization stores the backend credit");
    let points_earned: f64 = score.try_get("points_earned").expect("points earned");
    let points_possible: f64 = score.try_get("points_possible").expect("points possible");
    assert_eq!(points_possible, 2.0);
    assert_eq!(points_earned, backend_credit * points_possible);
    commit_tx.commit().await.expect("finalization commit");
    (fixture, prepared_question_attempt_id, backend_credit)
}

async fn save_current_points(
    pool: &PgPool,
    fixture: &AttemptFixture,
    entry_id: Uuid,
    points: &str,
) {
    let course_id = COURSE_INSTANCE_ID
        .get()
        .expect("seeded Course Instance")
        .clone();
    let mut tx = pool.begin().await.expect("point save transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tx)
        .await
        .expect("Assessment policy role");
    let instructor_id: String = sqlx::query_scalar(
        "SELECT account_id FROM ple_data.course_membership \
         WHERE course_instance_id = $1 AND role = 'instructor'",
    )
    .bind(&course_id)
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
    let values: serde_json::Value = sqlx::query_scalar(
        "SELECT jsonb_build_object( \
            'assessment_title', snapshot.assessment_title, \
            'assessment_instructions', snapshot.assessment_instructions, \
            'available_at', snapshot.available_at, \
            'due_at', snapshot.due_at, \
            'closes_at', snapshot.closes_at, \
            'assessment_attempt_time_limit_seconds', snapshot.assessment_attempt_time_limit_seconds, \
            'assessment_attempt_limit', snapshot.assessment_attempt_limit, \
            'late_work_rule', snapshot.late_work_rule, \
            'question_variation_rule', snapshot.question_variation_rule, \
            'assessment_question_order_rule', snapshot.assessment_question_order_rule, \
            'feedback_score', snapshot.feedback_score, \
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
    .expect("current Assessment policy");
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *tx)
        .await
        .expect("Assessment save role");
    sqlx::query("SELECT set_config('ple.session_account_id', $1, true)")
        .bind(&instructor_id)
        .execute(&mut *tx)
        .await
        .expect("Instructor session");
    let edit_number: i64 = sqlx::query_scalar(
        "SELECT assessment_edit_number FROM ple_api.save_assessment( \
             $1, $2, $3, $4, jsonb_build_array(jsonb_build_object( \
                 'assessmentEntryId', $5::text, \
                 'kind', 'fixed_question', \
                 'availability', 'available', \
                 'scoringRule', 'normal', \
                 'authoredPosition', 0, \
                 'questionId', $6::text, \
                 'revisionNumber', 1, \
                 'pointsPossible', $7::text)))",
    )
    .bind(&course_id)
    .bind(&fixture.assessment_id)
    .bind(expected_edit)
    .bind(&values)
    .bind(entry_id)
    .bind(PUBLISHED_QUESTION)
    .bind(points)
    .fetch_one(&mut *tx)
    .await
    .expect("Instructor save changes the Question point value");
    assert!(
        edit_number > expected_edit,
        "point change advances the Assessment Edit Number"
    );
    tx.commit().await.expect("point save commit");
}

async fn submitted_score_without_backend_work(pool: &PgPool, attempt_id: Uuid) -> (f64, f64) {
    let mut tx = pool.begin().await.expect("rescore transaction");
    set_student(&mut tx).await.expect("Student rescore session");
    let row = sqlx::query(
        "SELECT preparation_state, points_earned, points_possible, \
                (question_attempt_id IS NULL AND saved_at_millis IS NULL \
                 AND student_response IS NULL AND backend IS NULL \
                 AND source_object_record_id IS NULL) AS no_backend_work \
           FROM ple_api.prepare_student_assessment_attempt_finalization($1)",
    )
    .bind(attempt_id)
    .fetch_one(&mut *tx)
    .await
    .expect("submitted score");
    let state: String = row.try_get("preparation_state").expect("preparation state");
    let points_earned: f64 = row.try_get("points_earned").expect("rescored points");
    let points_possible: f64 = row
        .try_get("points_possible")
        .expect("rescored points possible");
    let no_backend_work: bool = row.try_get("no_backend_work").expect("backend work fields");
    assert_eq!(state, "already_submitted");
    assert!(
        no_backend_work,
        "rescore prepare returned Question Backend work"
    );
    tx.rollback().await.expect("rescore observation rollback");
    (points_earned, points_possible)
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn current_point_values_recalculate_stored_credit_without_another_backend_grade() {
    let source = color_question_source().await;
    let correct = StudentResponse::MultipleChoice {
        selected: vec![ResponseItemId::new("blue")],
    };
    let incorrect = StudentResponse::MultipleChoice {
        selected: vec![ResponseItemId::new("red")],
    };
    let pool = migration_pool().await;
    let correct_entry = Uuid::from_u128(0xf5810000000000000000000000000001);
    let (correct_fixture, correct_question_attempt, correct_credit) = store_color_outcome(
        &pool,
        &source,
        &correct,
        correct_entry,
        Uuid::from_u128(0xf5800000000000000000000000000001),
        Uuid::from_u128(0xf5820000000000000000000000000001),
        Uuid::from_u128(0xf5830000000000000000000000000001),
    )
    .await;
    let incorrect_entry = Uuid::from_u128(0xf5850000000000000000000000000001);
    let (incorrect_fixture, incorrect_question_attempt, incorrect_credit) = store_color_outcome(
        &pool,
        &source,
        &incorrect,
        incorrect_entry,
        Uuid::from_u128(0xf5840000000000000000000000000001),
        Uuid::from_u128(0xf5860000000000000000000000000001),
        Uuid::from_u128(0xf5870000000000000000000000000001),
    )
    .await;
    assert_eq!(correct_credit, 1.0);
    assert_eq!(incorrect_credit, 0.0);
    save_current_points(&pool, &correct_fixture, correct_entry, "5").await;
    save_current_points(&pool, &incorrect_fixture, incorrect_entry, "5").await;
    let (correct_earned, correct_possible) =
        submitted_score_without_backend_work(&pool, correct_fixture.attempt_id).await;
    let (incorrect_earned, incorrect_possible) =
        submitted_score_without_backend_work(&pool, incorrect_fixture.attempt_id).await;
    assert_eq!(correct_earned, correct_credit * 5.0);
    assert_eq!(correct_possible, 5.0);
    assert_eq!(incorrect_earned, incorrect_credit * 5.0);
    assert_eq!(incorrect_possible, 5.0);
    let mut stored_tx = pool.begin().await.expect("stored credit transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *stored_tx)
        .await
        .expect("stored credit role");
    for (question_attempt_id, backend_credit) in [
        (correct_question_attempt, correct_credit),
        (incorrect_question_attempt, incorrect_credit),
    ] {
        let stored_credit: f64 = sqlx::query_scalar(
            "SELECT normalized_credit::float8 FROM ple_private.grading_result \
             WHERE question_attempt_id = $1",
        )
        .bind(question_attempt_id)
        .fetch_one(&mut *stored_tx)
        .await
        .expect("stored credit");
        assert_eq!(stored_credit, backend_credit);
    }
    stored_tx
        .rollback()
        .await
        .expect("stored credit observation rollback");
}
