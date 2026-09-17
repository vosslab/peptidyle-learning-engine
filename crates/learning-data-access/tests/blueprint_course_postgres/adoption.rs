//! Stable relational oracle for exact Blueprint-to-Course adoption.

use sqlx::{Connection, PgConnection, Row};

/// The adopted Course must preserve the sealed source Revision's policy,
/// Question pins, and immutable Pool-fork provenance. This is a durable
/// teaching-data integrity contract. If it fails, repair adoption persistence.
pub(super) async fn assert_adoption_projection(
    audit_inspection: &mut PgConnection,
    course_reference: i64,
    blueprint_reference: i64,
    blueprint_revision: i64,
    independent_course_reference: i64,
) {
    assert_projection(
        audit_inspection,
        course_reference,
        blueprint_reference,
        blueprint_revision,
        independent_course_reference,
        false,
        blueprint_revision,
    )
    .await;
}

pub(super) async fn assert_append_projection(
    inspection: &mut PgConnection,
    course: i64,
    blueprint: i64,
    revision: i64,
    independent_course: i64,
    adoption_revision: i64,
) {
    assert_projection(
        inspection,
        course,
        blueprint,
        revision,
        independent_course,
        true,
        adoption_revision,
    )
    .await;
}

async fn assert_projection(
    audit_inspection: &mut PgConnection,
    course_reference: i64,
    blueprint_reference: i64,
    blueprint_revision: i64,
    independent_course_reference: i64,
    appended_only: bool,
    adoption_revision: i64,
) {
    let mut inspection = audit_inspection
        .begin()
        .await
        .expect("adoption inspection transaction");
    // FORCE RLS grants sealed Blueprint reads to the API owner, not the data owner.
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *inspection)
        .await
        .expect("sealed Blueprint inspection role");
    let source_content: serde_json::Value = sqlx::query_scalar(
        "SELECT content FROM ple_data.blueprint_course_revision \
         WHERE blueprint_course_reference_number = $1 AND blueprint_revision_number = $2",
    )
    .bind(blueprint_reference)
    .bind(blueprint_revision)
    .fetch_one(&mut *inspection)
    .await
    .expect("sealed Blueprint source exists");
    let prior_sources: Vec<String> = sqlx::query_scalar(
        "SELECT blueprint_assessment_reference::text FROM ple_data.blueprint_revision_assessment \
         WHERE blueprint_course_reference_number = $1 AND blueprint_revision_number < $2",
    )
    .bind(blueprint_reference)
    .bind(blueprint_revision)
    .fetch_all(&mut *inspection)
    .await
    .expect("prior sealed Blueprint Assessment identities");
    let provenance_matches: bool = sqlx::query_scalar(
        "SELECT count(*) = 1 AND bool_and(course.source_kind = 'adopted' \
         AND course.blueprint_course_reference_number = $2 AND course.blueprint_revision_number = $3 \
         AND origin.source_kind = 'adopted' AND origin.blueprint_course_reference_number = $2 \
         AND origin.blueprint_revision_number = $3 AND origin.source_course_id IS NULL) \
         FROM ple_data.course_instance course JOIN ple_data.course_origin origin ON origin.course_id = course.course_id \
         WHERE course.reference_number = $1",
    )
    .bind(course_reference)
    .bind(blueprint_reference)
    .bind(adoption_revision)
    .fetch_one(&mut *inspection)
    .await
    .expect("Course adoption provenance projection");
    assert!(
        provenance_matches,
        "Course adoption provenance preserves its original Revision"
    );
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *inspection)
        .await
        .expect("adoption data inspection role");
    let row = sqlx::query(
        r#"
WITH source_assessment AS (
    SELECT assessment_row.assessment ->> 'blueprint_assessment_reference' AS source,
           assessment_row.assessment -> 'content' AS content
      FROM jsonb_array_elements($6::jsonb -> 'modules') AS module_row(module)
      CROSS JOIN LATERAL jsonb_array_elements(module_row.module -> 'assessments') AS assessment_row(assessment)
     WHERE NOT $5 OR NOT ((assessment_row.assessment ->> 'blueprint_assessment_reference') = ANY($7::text[]))
), target_assessment AS (
    SELECT assessment.*
      FROM ple_data.assessment AS assessment
      JOIN ple_data.course_instance AS course ON course.course_id = assessment.course_id
     WHERE course.reference_number = $1
       AND (NOT $5 OR assessment.source_blueprint_revision_number = $3)
), policy_matches AS (
    SELECT count(*) = (SELECT count(*) FROM source_assessment)
       AND bool_and(
           target.assessment_id::text IS DISTINCT FROM source.source
           AND target.origin_kind = 'adopted'
           AND target.source_blueprint_course_reference_number = $2
           AND target.source_blueprint_revision_number = $3
           AND target.source_blueprint_assessment_reference::text = source.source
           AND target.assessment_type = source.content ->> 'assessment_type'
           AND target.assessment_title = source.content ->> 'title'
           AND target.assessment_instructions = source.content ->> 'instructions'
           AND target.assessment_status = 'unreleased'
           AND target.assessment_edit_number = 1
           AND target.available_at IS NULL AND target.due_at IS NULL AND target.closes_at IS NULL
           AND target.assessment_attempt_time_limit_seconds IS NOT DISTINCT FROM
               (source.content #>> '{defaults,assessment_attempt_time_limit_seconds}')::integer
           AND target.assessment_attempt_limit IS NOT DISTINCT FROM
               (source.content #>> '{defaults,assessment_attempt_limit}')::integer
           AND target.late_work_rule = CASE source.content #>> '{defaults,late_work_rule}'
               WHEN 'accept' THEN 'accept' WHEN 'mark_late' THEN 'mark_late'
               WHEN 'reject' THEN 'reject' END
           AND target.question_variation_rule = CASE source.content #>> '{defaults,activity_rules,questionVariationRule}'
               WHEN 'reuseVariation' THEN 'reuse_variation' WHEN 'newVariation' THEN 'new_variation' END
           AND target.assessment_question_order_rule = CASE source.content #>> '{defaults,activity_rules,assessmentQuestionOrderRule}'
               WHEN 'authoredOrder' THEN 'authored_order' WHEN 'shuffled' THEN 'shuffled' END
           AND target.feedback_score = source.content #>> '{defaults,student_feedback_release_rule,score}'
           AND target.feedback_per_item_correctness = source.content #>> '{defaults,student_feedback_release_rule,per_item_correctness}'
           AND target.feedback_submitted_response = source.content #>> '{defaults,student_feedback_release_rule,submitted_response}'
           AND target.feedback_question_answer = source.content #>> '{defaults,student_feedback_release_rule,question_answer}'
           AND target.feedback_question_answer_explanation = source.content #>> '{defaults,student_feedback_release_rule,question_answer_explanation}'
           AND target.feedback_class_statistics = source.content #>> '{defaults,student_feedback_release_rule,class_statistics}'
       ) AS matches
      FROM source_assessment AS source
      JOIN target_assessment AS target ON target.source_blueprint_assessment_reference::text = source.source
), source_entries AS (
    SELECT source.source, entry.ordinality::integer - 1 AS position, entry.value AS entry
      FROM source_assessment AS source
      CROSS JOIN LATERAL jsonb_array_elements(source.content -> 'entries') WITH ORDINALITY AS entry(value, ordinality)
), entry_count_matches AS (
    SELECT (SELECT count(*) FROM source_entries) =
           (SELECT count(*) FROM ple_data.assessment_entry AS entry
             JOIN target_assessment AS target ON target.assessment_id = entry.assessment_id) AS matches
), fixed_entries_match AS (
    SELECT count(*) = count(*) FILTER (WHERE entry.entry_kind = 'fixed_question')
       AND bool_and(
           entry.availability = 'available'
           AND entry.question_id = source.entry #>> '{question_revision,questionId}'
           AND entry.question_revision_number = (source.entry #>> '{question_revision,revisionNumber}')::integer
           AND entry.points_possible::text = source.entry ->> 'points_possible'
           AND entry.scoring_rule = CASE source.entry ->> 'scoring_rule'
               WHEN 'normal' THEN 'normal' WHEN 'fullCredit' THEN 'full_credit'
               WHEN 'extraCredit' THEN 'extra_credit' WHEN 'excluded' THEN 'excluded' END
           AND entry.question_attempt_limit IS NOT DISTINCT FROM
               (source.entry #>> '{question_attempt_limit,maxAttempts}')::integer
           AND entry.question_attempt_time_limit_seconds IS NOT DISTINCT FROM
               (source.entry #>> '{question_attempt_time_limit,seconds}')::integer
           AND entry.question_attempt_grace_seconds IS NOT DISTINCT FROM
               (source.entry #>> '{question_attempt_time_limit,graceSeconds}')::integer
       ) AS matches
      FROM source_entries AS source
      JOIN target_assessment AS target ON target.source_blueprint_assessment_reference::text = source.source
      JOIN ple_data.assessment_entry AS entry
        ON entry.assessment_id = target.assessment_id AND entry.authored_position = source.position
     WHERE source.entry ->> 'kind' = 'fixed'
), pool_forks_match AS (
    SELECT count(*) = count(*) FILTER (WHERE fork.question_pool_id IS NOT NULL)
       AND bool_and(
           entry.entry_kind = 'question_pool'
           AND entry.availability = 'available'
           AND entry.question_pool_revision_number = 1
           AND entry.selection_count = (source.entry ->> 'selection_count')::integer
           AND entry.points_per_item::text = source.entry ->> 'points_per_item'
           AND entry.scoring_rule = CASE source.entry ->> 'scoring_rule'
               WHEN 'normal' THEN 'normal' WHEN 'fullCredit' THEN 'full_credit'
               WHEN 'extraCredit' THEN 'extra_credit' WHEN 'excluded' THEN 'excluded' END
           AND entry.selected_question_order = CASE source.entry #>> '{selection_rule,selectedQuestionOrder}'
               WHEN 'questionPoolOrder' THEN 'question_pool_order' WHEN 'randomOrder' THEN 'random_order' END
           AND entry.question_attempt_limit IS NOT DISTINCT FROM
               (source.entry #>> '{question_attempt_limit,maxAttempts}')::integer
           AND entry.question_attempt_time_limit_seconds IS NOT DISTINCT FROM
               (source.entry #>> '{question_attempt_time_limit,seconds}')::integer
           AND entry.question_attempt_grace_seconds IS NOT DISTINCT FROM
               (source.entry #>> '{question_attempt_time_limit,graceSeconds}')::integer
           AND fork.origin_question_pool_revision_number = 1
           AND child.question_pool_id <> root.question_pool_id
           AND child.source_question_pool_id = root.question_pool_id
           AND child.source_question_pool_revision_number
                 = (source.entry #>> '{question_pool_revision,revisionNumber}')::bigint
           AND child_revision.interchangeability_attested_by_account_id =
               root_revision.interchangeability_attested_by_account_id
           AND child_revision.interchangeability_attested_at =
               root_revision.interchangeability_attested_at
           AND root.public_question_pool_id
                 = replace(source.entry #>> '{question_pool_revision,questionPoolId}', '-', '')
           AND NOT EXISTS (
               (SELECT member_position, question_id, question_revision_number
                  FROM ple_data.question_pool_revision_member
                 WHERE question_pool_id = child.question_pool_id AND revision_number = 1)
               EXCEPT ALL
               (SELECT member_position, question_id, question_revision_number
                  FROM ple_data.question_pool_revision_member
                 WHERE question_pool_id = root.question_pool_id
                   AND revision_number = child.source_question_pool_revision_number)
           )
           AND NOT EXISTS (
               (SELECT member_position, question_id, question_revision_number
                  FROM ple_data.question_pool_revision_member
                 WHERE question_pool_id = root.question_pool_id
                   AND revision_number = child.source_question_pool_revision_number)
               EXCEPT ALL
               (SELECT member_position, question_id, question_revision_number
                  FROM ple_data.question_pool_revision_member
                 WHERE question_pool_id = child.question_pool_id AND revision_number = 1)
           )
       ) AS matches
      FROM source_entries AS source
      JOIN target_assessment AS target ON target.source_blueprint_assessment_reference::text = source.source
      JOIN ple_data.assessment_entry AS entry
        ON entry.assessment_id = target.assessment_id AND entry.authored_position = source.position
      LEFT JOIN ple_data.assessment_question_pool_fork AS fork
        ON fork.assessment_entry_id = entry.assessment_entry_id AND fork.assessment_id = entry.assessment_id
      LEFT JOIN ple_data.question_pool AS child ON child.question_pool_id = entry.question_pool_id
      LEFT JOIN ple_data.question_pool AS root ON root.question_pool_id = child.source_question_pool_id
      LEFT JOIN ple_data.question_pool_revision AS child_revision
        ON child_revision.question_pool_id = child.question_pool_id AND child_revision.revision_number = 1
      LEFT JOIN ple_data.question_pool_revision AS root_revision
        ON root_revision.question_pool_id = root.question_pool_id
       AND root_revision.revision_number = child.source_question_pool_revision_number
     WHERE source.entry ->> 'kind' = 'pool'
), independent_pool_forks AS (
    SELECT NOT EXISTS (
        SELECT 1
          FROM ple_data.assessment_entry AS first_entry
          JOIN ple_data.assessment AS first_assessment ON first_assessment.assessment_id = first_entry.assessment_id
          JOIN ple_data.course_instance AS first_course ON first_course.course_id = first_assessment.course_id
          JOIN ple_data.assessment_entry AS second_entry ON second_entry.question_pool_id = first_entry.question_pool_id
          JOIN ple_data.assessment AS second_assessment ON second_assessment.assessment_id = second_entry.assessment_id
          JOIN ple_data.course_instance AS second_course ON second_course.course_id = second_assessment.course_id
         WHERE first_course.reference_number = $1 AND second_course.reference_number = $4
    ) AS matches
)
SELECT COALESCE((SELECT matches FROM policy_matches), false) AS policy_matches,
       (SELECT jsonb_agg(jsonb_build_object('source', source.content, 'target', to_jsonb(target)))
          FROM source_assessment source JOIN target_assessment target
            ON target.source_blueprint_assessment_reference::text = source.source) AS policy_projection,
       COALESCE((SELECT matches FROM entry_count_matches), false) AS entry_count_matches,
       COALESCE((SELECT matches FROM fixed_entries_match), false) AS fixed_entries_match,
       COALESCE((SELECT matches FROM pool_forks_match), false) AS pool_forks_match,
       COALESCE((SELECT matches FROM independent_pool_forks), false) AS independent_pool_forks
"#,
    )
    .bind(course_reference)
    .bind(blueprint_reference)
    .bind(blueprint_revision)
    .bind(independent_course_reference)
    .bind(appended_only)
    .bind(source_content)
    .bind(prior_sources)
    .fetch_one(&mut *inspection)
    .await
    .expect("relational adoption projection");
    assert!(
        row.get::<bool, _>("policy_matches"),
        "adopted policy differs from sealed source: {:?}",
        row.get::<Option<serde_json::Value>, _>("policy_projection")
    );
    assert!(row.get::<bool, _>("entry_count_matches"));
    assert!(row.get::<bool, _>("fixed_entries_match"));
    assert!(row.get::<bool, _>("pool_forks_match"));
    assert!(row.get::<bool, _>("independent_pool_forks"));
    inspection
        .commit()
        .await
        .expect("adoption inspection commit");

    let mut audit_transaction = audit_inspection
        .begin()
        .await
        .expect("adoption audit inspection transaction");
    sqlx::query("SET LOCAL ROLE ple_audit_owner")
        .execute(&mut *audit_transaction)
        .await
        .expect("adoption audit inspection role");
    // The audit owner has only an INSERT policy under FORCE RLS. As in the
    // controlled tamper oracle, permit owner inspection only until rollback.
    sqlx::query("ALTER TABLE ple_audit.course_instance_creation_event NO FORCE ROW LEVEL SECURITY")
        .execute(&mut *audit_transaction)
        .await
        .expect("controlled audit owner inspection");
    let audit_matches: bool = sqlx::query_scalar(
        "SELECT count(*) = 1 FROM ple_audit.course_instance_creation_event \
         WHERE course_reference_number = $1 AND source_kind = 'adopted' \
           AND blueprint_course_reference_number = $2 AND blueprint_revision_number = $3",
    )
    .bind(course_reference)
    .bind(blueprint_reference)
    .bind(adoption_revision)
    .fetch_one(&mut *audit_transaction)
    .await
    .expect("adoption audit projection");
    assert!(audit_matches);
    audit_transaction
        .rollback()
        .await
        .expect("restore forced audit RLS after inspection");
    let forced: bool = sqlx::query_scalar(
        "SELECT relation.relforcerowsecurity FROM pg_catalog.pg_class relation \
         JOIN pg_catalog.pg_namespace namespace ON namespace.oid = relation.relnamespace \
         WHERE namespace.nspname = 'ple_audit' AND relation.relname = 'course_instance_creation_event'",
    )
    .fetch_one(audit_inspection)
    .await
    .expect("audit forced-RLS restoration inspection");
    assert!(forced, "privileged audit inspection restores forced RLS");
}
