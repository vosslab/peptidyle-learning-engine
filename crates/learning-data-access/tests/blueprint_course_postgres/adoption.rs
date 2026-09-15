//! Stable relational oracle for exact Blueprint-to-Course adoption.

use sqlx::{PgPool, Row};

/// The adopted Course must be a fresh child that preserves the sealed source
/// Revision's complete teaching policy and ordered Question pins. This is a
/// durable product/data-integrity contract: a regression silently changes
/// teaching behavior or provenance. If it fails, repair adoption persistence
/// or its validation transaction; do not weaken this oracle.
pub(super) async fn assert_adoption_projection(
    audit_inspection: &PgPool,
    course_reference: i64,
    blueprint_reference: i64,
    blueprint_revision: i64,
    independent_course_reference: i64,
) {
    let mut inspection = audit_inspection
        .begin()
        .await
        .expect("adoption inspection transaction");
    let row = sqlx::query(
        r#"
WITH source_assessment AS (
    SELECT assessment_row.assessment ->> 'blueprint_assessment_reference' AS source,
           assessment_row.assessment -> 'content' AS content
      FROM ple_data.blueprint_course_revision AS revision
      CROSS JOIN LATERAL jsonb_array_elements(revision.content -> 'modules') AS module_row(module)
      CROSS JOIN LATERAL jsonb_array_elements(module_row.module -> 'assessments') AS assessment_row(assessment)
     WHERE revision.blueprint_course_reference_number = $2
       AND revision.blueprint_revision_number = $3
), target_assessment AS (
    SELECT assessment.*
      FROM ple_data.assessment AS assessment
      JOIN ple_data.course_instance AS course ON course.course_id = assessment.course_id
     WHERE course.reference_number = $1
), policy_matches AS (
    SELECT count(*) = (SELECT count(*) FROM source_assessment) AS exact_count,
           bool_and(
               target.assessment_id::text IS DISTINCT FROM source.source
               AND target.source_blueprint_assessment_reference::text = source.source
               AND jsonb_build_object(
                   'assessment_title', target.assessment_title,
                   'assessment_instructions', target.assessment_instructions,
                   'assessment_attempt_time_limit_seconds', target.assessment_attempt_time_limit_seconds,
                   'attempt_limit', target.attempt_limit,
                   'late_work_rule', target.late_work_rule,
                   'assessment_completion_rule', target.assessment_completion_rule,
                   'assessment_completion_score_threshold', target.assessment_completion_score_threshold,
                   'assessment_attempt_grade_rule', target.assessment_attempt_grade_rule,
                   'assessment_attempt_continuation_rule', target.assessment_attempt_continuation_rule,
                   'max_additional_assessment_attempts', target.max_additional_assessment_attempts,
                   'question_pool_reuse_rule', target.question_pool_reuse_rule,
                   'question_variation_rule', target.question_variation_rule,
                   'assessment_attempt_resume_rule', target.assessment_attempt_resume_rule,
                   'assessment_question_display_rule', target.assessment_question_display_rule,
                   'assessment_navigation_rule', target.assessment_navigation_rule,
                   'assessment_question_order_rule', target.assessment_question_order_rule,
                   'feedback_score', target.feedback_score,
                   'feedback_per_item_correctness', target.feedback_per_item_correctness,
                   'feedback_submitted_response', target.feedback_submitted_response,
                   'feedback_question_feedback', target.feedback_question_feedback,
                   'feedback_question_answer', target.feedback_question_answer,
                   'feedback_question_answer_explanation', target.feedback_question_answer_explanation,
                   'feedback_class_statistics', target.feedback_class_statistics
               ) = jsonb_build_object(
                   'assessment_title', source.content -> 'title',
                   'assessment_instructions', source.content -> 'instructions',
                   'assessment_attempt_time_limit_seconds', source.content #> '{defaults,assessment_attempt_time_limit_seconds}',
                   'attempt_limit', source.content #> '{defaults,attempt_limit}',
                   'late_work_rule', source.content #> '{defaults,late_work_rule}',
                   'assessment_completion_rule', CASE source.content #>> '{defaults,activity_rules,assessmentCompletionRule,kind}'
                       WHEN 'answerAll' THEN 'answer_all' WHEN 'allCorrect' THEN 'all_correct'
                       WHEN 'scoreAtLeast' THEN 'score_at_least' END,
                   'assessment_completion_score_threshold', source.content #> '{defaults,activity_rules,assessmentCompletionRule,fraction}',
                   'assessment_attempt_grade_rule', CASE source.content #>> '{defaults,activity_rules,assessmentAttemptGradeRule}'
                       WHEN 'first' THEN 'first' WHEN 'latest' THEN 'latest' WHEN 'highest' THEN 'highest'
                       WHEN 'instructorSelected' THEN 'instructor_selected' END,
                   'assessment_attempt_continuation_rule', CASE source.content #>> '{defaults,activity_rules,assessmentAttemptContinuationRule,kind}'
                       WHEN 'unlimited' THEN 'unlimited' WHEN 'capped' THEN 'capped' WHEN 'closed' THEN 'closed' END,
                   'max_additional_assessment_attempts', source.content #> '{defaults,activity_rules,assessmentAttemptContinuationRule,maxAdditionalAssessmentAttempts}',
                   'question_pool_reuse_rule', CASE source.content #>> '{defaults,activity_rules,questionPoolReuseRule}'
                       WHEN 'reuseSelection' THEN 'reuse_selection' WHEN 'selectAgain' THEN 'select_again' END,
                   'question_variation_rule', CASE source.content #>> '{defaults,activity_rules,questionVariationRule}'
                       WHEN 'reuseVariation' THEN 'reuse_variation' WHEN 'newVariation' THEN 'new_variation' END,
                   'assessment_attempt_resume_rule', CASE source.content #>> '{defaults,activity_rules,assessmentAttemptResumeRule}'
                       WHEN 'resumable' THEN 'resumable' WHEN 'singleSession' THEN 'single_session' END,
                   'assessment_question_display_rule', CASE source.content #>> '{defaults,activity_rules,assessmentQuestionDisplayRule}'
                       WHEN 'allQuestions' THEN 'all_questions' WHEN 'oneQuestionAtATime' THEN 'one_question_at_a_time' END,
                   'assessment_navigation_rule', CASE source.content #>> '{defaults,activity_rules,assessmentNavigationRule}'
                       WHEN 'freeNavigation' THEN 'free_navigation' WHEN 'forwardOnly' THEN 'forward_only' END,
                   'assessment_question_order_rule', CASE source.content #>> '{defaults,activity_rules,assessmentQuestionOrderRule}'
                       WHEN 'authoredOrder' THEN 'authored_order' WHEN 'shuffled' THEN 'shuffled' END,
                   'feedback_score', source.content #> '{defaults,student_feedback_release_rule,score}',
                   'feedback_per_item_correctness', source.content #> '{defaults,student_feedback_release_rule,per_item_correctness}',
                   'feedback_submitted_response', source.content #> '{defaults,student_feedback_release_rule,submitted_response}',
                   'feedback_question_feedback', source.content #> '{defaults,student_feedback_release_rule,question_feedback}',
                   'feedback_question_answer', source.content #> '{defaults,student_feedback_release_rule,question_answer}',
                   'feedback_question_answer_explanation', source.content #> '{defaults,student_feedback_release_rule,question_answer_explanation}',
                   'feedback_class_statistics', source.content #> '{defaults,student_feedback_release_rule,class_statistics}'
               )
               AND target.assessment_status = 'unreleased'
               AND target.assessment_edit_number = 1
               AND target.available_at IS NULL AND target.due_at IS NULL AND target.closes_at IS NULL
           ) AS matches
      FROM source_assessment AS source
      JOIN target_assessment AS target ON target.source_blueprint_assessment_reference::text = source.source
), source_entries AS (
    SELECT source.source, entry.ordinality::integer - 1 AS position, entry.value AS entry
      FROM source_assessment AS source
      CROSS JOIN LATERAL jsonb_array_elements(source.content -> 'entries') WITH ORDINALITY AS entry(value, ordinality)
), target_entries AS (
    SELECT target.source_blueprint_assessment_reference::text AS source,
           entry.authored_position AS position,
           jsonb_build_object(
               'kind', entry.entry_kind, 'availability', entry.availability,
               'position', entry.authored_position, 'question_id', entry.question_id,
               'revision_number', entry.question_revision_number,
               'points_possible', entry.points_possible::text, 'scoring_rule', entry.scoring_rule,
               'selection_count', entry.selection_count, 'points_per_item', entry.points_per_item::text,
               'selected_question_order', entry.selected_question_order,
               'question_attempt_limit', entry.question_attempt_limit,
               'question_attempt_time_limit_seconds', entry.question_attempt_time_limit_seconds,
               'question_attempt_grace_seconds', entry.question_attempt_grace_seconds,
               'items', COALESCE((
                   SELECT jsonb_agg(jsonb_build_object(
                       'position', item.item_position, 'question_id', item.question_id,
                       'revision_number', item.question_revision_number, 'availability', item.availability
                   ) ORDER BY item.item_position)
                     FROM ple_data.question_pool_item AS item
                    WHERE item.assessment_entry_id = entry.assessment_entry_id
               ), '[]'::jsonb)
           ) AS value
      FROM target_assessment AS target
      JOIN ple_data.assessment_entry AS entry ON entry.assessment_id = target.assessment_id
), entry_matches AS (
    SELECT count(*) = (SELECT count(*) FROM source_entries) AS exact_count,
           bool_and(target.value = jsonb_build_object(
               'kind', CASE source.entry ->> 'kind' WHEN 'fixed' THEN 'fixed_question' WHEN 'pool' THEN 'question_pool' END,
               'availability', 'available', 'position', source.position,
               'question_id', CASE WHEN source.entry ->> 'kind' = 'fixed' THEN replace(source.entry #>> '{question_revision,questionId}', '-', '') END,
               'revision_number', CASE WHEN source.entry ->> 'kind' = 'fixed' THEN (source.entry #>> '{question_revision,revisionNumber}')::integer END,
               'points_possible', CASE WHEN source.entry ->> 'kind' = 'fixed' THEN source.entry ->> 'points_possible' END,
               'scoring_rule', CASE source.entry ->> 'scoring_rule'
                   WHEN 'normal' THEN 'normal' WHEN 'fullCredit' THEN 'full_credit'
                   WHEN 'extraCredit' THEN 'extra_credit' WHEN 'excluded' THEN 'excluded' END,
               'selection_count', CASE WHEN source.entry ->> 'kind' = 'pool' THEN (source.entry ->> 'selection_count')::integer END,
               'points_per_item', CASE WHEN source.entry ->> 'kind' = 'pool' THEN source.entry ->> 'points_per_item' END,
               'selected_question_order', CASE source.entry #>> '{selection_rule,selectedQuestionOrder}'
                   WHEN 'questionPoolOrder' THEN 'question_pool_order' WHEN 'randomOrder' THEN 'random_order' END,
               'question_attempt_limit', (source.entry #>> '{question_attempt_limit,maxAttempts}')::integer,
               'question_attempt_time_limit_seconds', (source.entry #>> '{question_attempt_time_limit,seconds}')::integer,
               'question_attempt_grace_seconds', (source.entry #>> '{question_attempt_time_limit,graceSeconds}')::integer,
               'items', CASE WHEN source.entry ->> 'kind' = 'pool' THEN COALESCE((
                   SELECT jsonb_agg(jsonb_build_object(
                       'position', item.ordinality::integer - 1, 'question_id', replace(item.value ->> 'questionId', '-', ''),
                       'revision_number', (item.value ->> 'revisionNumber')::integer, 'availability', 'available'
                   ) ORDER BY item.ordinality)
                     FROM jsonb_array_elements(source.entry -> 'question_revisions') WITH ORDINALITY AS item(value, ordinality)
               ), '[]'::jsonb) ELSE '[]'::jsonb END
           )) AS matches
      FROM source_entries AS source
      JOIN target_entries AS target ON target.source = source.source AND target.position = source.position
), provenance_matches AS (
    SELECT count(*) = 1 AS course_count,
           bool_and(course.source_kind = 'adopted'
               AND course.blueprint_course_reference_number = $2 AND course.blueprint_revision_number = $3
               AND origin.source_kind = 'adopted' AND origin.blueprint_course_reference_number = $2
               AND origin.blueprint_revision_number = $3 AND origin.source_course_id IS NULL) AS matches
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_origin AS origin ON origin.course_id = course.course_id
     WHERE course.reference_number = $1
), fresh_child_ids_match AS (
    SELECT NOT EXISTS (
        SELECT 1
          FROM ple_data.assessment AS first_assessment
          JOIN ple_data.course_instance AS first_course ON first_course.course_id = first_assessment.course_id
          JOIN ple_data.assessment AS second_assessment ON second_assessment.assessment_id = first_assessment.assessment_id
          JOIN ple_data.course_instance AS second_course ON second_course.course_id = second_assessment.course_id
         WHERE first_course.reference_number = $1 AND second_course.reference_number = $4
        UNION ALL
        SELECT 1
          FROM ple_data.assessment_entry AS first_entry
          JOIN ple_data.assessment AS first_assessment ON first_assessment.assessment_id = first_entry.assessment_id
          JOIN ple_data.course_instance AS first_course ON first_course.course_id = first_assessment.course_id
          JOIN ple_data.assessment_entry AS second_entry ON second_entry.assessment_entry_id = first_entry.assessment_entry_id
          JOIN ple_data.assessment AS second_assessment ON second_assessment.assessment_id = second_entry.assessment_id
          JOIN ple_data.course_instance AS second_course ON second_course.course_id = second_assessment.course_id
         WHERE first_course.reference_number = $1 AND second_course.reference_number = $4
        UNION ALL
        SELECT 1
          FROM ple_data.question_pool_item AS first_item
          JOIN ple_data.assessment AS first_assessment ON first_assessment.assessment_id = first_item.assessment_id
          JOIN ple_data.course_instance AS first_course ON first_course.course_id = first_assessment.course_id
          JOIN ple_data.question_pool_item AS second_item ON second_item.question_pool_item_id = first_item.question_pool_item_id
          JOIN ple_data.assessment AS second_assessment ON second_assessment.assessment_id = second_item.assessment_id
          JOIN ple_data.course_instance AS second_course ON second_course.course_id = second_assessment.course_id
         WHERE first_course.reference_number = $1 AND second_course.reference_number = $4
    ) AS matches
)
SELECT COALESCE((SELECT exact_count AND matches FROM policy_matches), false) AS policy_matches,
       COALESCE((SELECT exact_count AND matches FROM entry_matches), false) AS entry_matches,
       COALESCE((SELECT course_count AND matches FROM provenance_matches), false) AS provenance_matches,
       COALESCE((SELECT matches FROM fresh_child_ids_match), false) AS fresh_child_ids_match
"#,
    )
    .bind(course_reference)
    .bind(blueprint_reference)
    .bind(blueprint_revision)
    .bind(independent_course_reference)
    .fetch_one(&mut *inspection)
    .await
    .expect("relational adoption projection");
    assert!(row.get::<bool, _>("policy_matches"));
    assert!(row.get::<bool, _>("entry_matches"));
    assert!(row.get::<bool, _>("provenance_matches"));
    assert!(row.get::<bool, _>("fresh_child_ids_match"));
    inspection
        .commit()
        .await
        .expect("adoption inspection commit");

    let mut audit_inspection = audit_inspection
        .begin()
        .await
        .expect("adoption audit inspection transaction");
    let audit_matches: bool = sqlx::query_scalar(
        "SELECT count(*) = 1 FROM ple_audit.course_instance_creation_event \
         WHERE course_reference_number = $1 AND source_kind = 'adopted' \
           AND blueprint_course_reference_number = $2 AND blueprint_revision_number = $3",
    )
    .bind(course_reference)
    .bind(blueprint_reference)
    .bind(blueprint_revision)
    .fetch_one(&mut *audit_inspection)
    .await
    .expect("adoption audit projection");
    assert!(audit_matches);
    audit_inspection
        .commit()
        .await
        .expect("adoption inspection commit");
}
