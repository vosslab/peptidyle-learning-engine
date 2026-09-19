-- Functions, triggers, and views from assessment_attempt_history.sql.

SET LOCAL ROLE ple_api_owner;

-- Completed Student Assessment Attempt history.  This projection reads only
-- retained Assessment Attempt and Issued Question evidence; current Assessment content
-- is deliberately not an interpretation source.

-- The API owner has the existing narrow Course Membership read capability.
-- Expose only current Student Record identifiers to the private completion
-- predicate; Account state and invitations are deliberately not cohort facts.
CREATE FUNCTION ple_api.current_course_student_record_ids(p_course_instance_id text)
RETURNS TABLE (student_record_id uuid)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT membership.student_record_id
      FROM ple_data.course_membership AS membership
     WHERE membership.course_instance_id = p_course_instance_id
       AND membership.role = 'student'
       AND ple_data.course_membership_is_active(membership.course_membership_id)
$$;

SET LOCAL ROLE ple_private_owner;





-- ASVS 2.3.1, 8.2.2, 15.4.2: derive cohort completion from current
-- membership and immutable Assessment Submission evidence at the read instant.
-- There is no latch, snapshot, invitation, or Account-active filter.
CREATE FUNCTION ple_private.current_student_cohort_completed_assessment(p_assessment_id text)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT EXISTS (
               SELECT 1 FROM ple_data.assessment AS assessment
                WHERE assessment.assessment_id = p_assessment_id
           )
       AND NOT EXISTS (
               SELECT 1
                 FROM ple_data.assessment AS assessment
                 CROSS JOIN LATERAL ple_api.current_course_student_record_ids(
                     assessment.course_instance_id
                 ) AS student
                WHERE assessment.assessment_id = p_assessment_id
                  AND NOT EXISTS (
                      SELECT 1
                        FROM ple_private.assessment_attempt AS assessment_attempt
                        JOIN ple_private.assessment_submission AS submission
                          ON submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id
                       WHERE assessment_attempt.assessment_id = assessment.assessment_id
                         AND assessment_attempt.student_record_id = student.student_record_id
                  )
           )
$$;

CREATE FUNCTION ple_private.read_student_assessment_attempt_history(
    p_assessment_attempt_id uuid
) RETURNS TABLE (
    course_instance_id text, assessment_id text, assessment_title text, assessment_type text,
    assessment_attempt_number integer,
    state text, questions jsonb, feedback_rule jsonb, due_at_millis bigint,
    closes_at_millis bigint, submitted_at_millis bigint, evaluated_at_millis bigint,
    all_students_completed boolean,
    grading_is_current boolean, grading_results jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
    WITH owned_assessment_attempt AS (
        SELECT assessment_attempt.assessment_attempt_id,
               assessment.course_instance_id,
               assessment.assessment_id,
               policy.assessment_title,
               assessment.assessment_type,
               assessment_attempt.assessment_attempt_number,
               policy.due_at,
               policy.closes_at,
               jsonb_build_object(
                   'score', policy.feedback_score,
                   'per_item_correctness', policy.feedback_per_item_correctness,
                   'submitted_response', policy.feedback_submitted_response,
                   'question_answer', policy.feedback_question_answer,
                   'question_answer_explanation', policy.feedback_question_answer_explanation,
                   'class_statistics', policy.feedback_class_statistics
               ) AS feedback_rule
          FROM ple_private.assessment_attempt AS assessment_attempt
          JOIN ple_data.assessment AS assessment
            ON assessment.assessment_id = assessment_attempt.assessment_id
          JOIN ple_data.assessment_policy_snapshot AS policy
            ON policy.assessment_policy_snapshot_id = assessment_attempt.assessment_policy_snapshot_id
         WHERE assessment_attempt.assessment_attempt_id = p_assessment_attempt_id
           AND EXISTS (
               SELECT 1 FROM ple_private.assessment_submission AS submission
                WHERE submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id
           )
           AND ple_api.current_session_account_owns_student_record(
               assessment.course_instance_id, assessment_attempt.student_record_id
           )
           -- ASVS 2.3.1: archive is an ordered retention transition.  Once
           -- it occurs, an otherwise-owned Assessment Attempt reference is not a path
           -- back into ordinary Student history.
           AND ple_api.course_student_work_is_ordinarily_visible(assessment.course_instance_id)
    )
    SELECT owned.course_instance_id,
           owned.assessment_id,
           owned.assessment_title,
           owned.assessment_type,
           owned.assessment_attempt_number,
           CASE WHEN assessment_submission.assessment_attempt_id IS NOT NULL
                THEN 'submitted'::text ELSE 'closed'::text END,
           questions.questions,
           owned.feedback_rule,
           floor(extract(epoch FROM owned.due_at) * 1000)::bigint,
           floor(extract(epoch FROM owned.closes_at) * 1000)::bigint,
           floor(extract(epoch FROM assessment_submission.submitted_at) * 1000)::bigint,
           floor(extract(epoch FROM pg_catalog.statement_timestamp()) * 1000)::bigint,
           ple_private.current_student_cohort_completed_assessment(
               owned.assessment_id
           ),
           grading.grading_is_current,
           grading.grading_results
      FROM owned_assessment_attempt AS owned
      LEFT JOIN ple_private.assessment_submission AS assessment_submission
        ON assessment_submission.assessment_attempt_id = owned.assessment_attempt_id
      CROSS JOIN LATERAL (
          SELECT COALESCE(jsonb_agg(jsonb_build_object(
                     'position', issued.issued_position + 1,
                     'questionId', issued.published_question_id,
                     'revisionNumber', issued.revision_number,
                     'responseState', CASE WHEN EXISTS (
                         SELECT 1 FROM ple_private.assessment_attempt_saved_response AS saved
                          WHERE saved.question_attempt_id = question_attempt.question_attempt_id
                     ) THEN 'submitted' ELSE 'closed' END
                 ) ORDER BY issued.issued_position), '[]'::jsonb) AS questions
            FROM ple_private.issued_question AS issued
            JOIN ple_private.question_attempt
              ON question_attempt.issued_question_id = issued.issued_question_id
           WHERE issued.assessment_attempt_id = owned.assessment_attempt_id
      ) AS questions
      CROSS JOIN LATERAL (
          SELECT count(*) > 0
                     AND bool_and(
                         saved.question_attempt_id IS NULL
                         OR (
                             result.grading_result_id IS NOT NULL
                             AND ple_api.has_automated_grading_receipt(
                                 result.grading_result_id
                             )
                         )
                     ) AS grading_is_current,
                 CASE WHEN count(*) > 0
                           AND bool_and(
                               saved.question_attempt_id IS NULL
                               OR (
                                   result.grading_result_id IS NOT NULL
                                   AND ple_api.has_automated_grading_receipt(
                                       result.grading_result_id
                                   )
                               )
                           )
                      THEN jsonb_agg(jsonb_build_object(
                          'position', issued.issued_position + 1,
                          'correct', coalesce(result.normalized_credit = 1, false),
                          'pointsEarned', score.points_earned,
                          'pointsPossible', score.points_possible
                      ) ORDER BY issued.issued_position)
                      ELSE '[]'::jsonb END AS grading_results
            FROM ple_private.issued_question AS issued
            JOIN ple_private.question_attempt
              ON question_attempt.issued_question_id = issued.issued_question_id
            LEFT JOIN ple_private.assessment_attempt_saved_response AS saved
              ON saved.question_attempt_id = question_attempt.question_attempt_id
            LEFT JOIN ple_private.grading_result AS result
              ON result.question_attempt_id = question_attempt.question_attempt_id
            JOIN ple_private.assessment_entry_snapshot AS snapshot
              ON snapshot.assessment_entry_snapshot_id = issued.assessment_entry_snapshot_id
            CROSS JOIN LATERAL ple_private.score_recorded_credit(
                result.normalized_credit, snapshot.scoring_rule,
                coalesce(
                    (SELECT question.points_possible
                       FROM ple_data.assessment_entry_question AS question
                      WHERE question.assessment_entry_id = issued.assessment_entry_id),
                    (SELECT pool.points_per_item
                       FROM ple_data.assessment_entry_pool AS pool
                      WHERE pool.assessment_entry_id = issued.assessment_entry_id),
                    snapshot.points
                )
            ) AS score
           WHERE issued.assessment_attempt_id = owned.assessment_attempt_id
      ) AS grading
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.read_student_assessment_attempt_history(
    p_assessment_attempt_id uuid
) RETURNS TABLE (
    course_instance_id text, course_short_name text, course_long_name text, course_theme text,
    assessment_id text, assessment_title text, assessment_type text,
    assessment_attempt_number integer,
    state text, questions jsonb, feedback_rule jsonb, due_at_millis bigint,
    closes_at_millis bigint, submitted_at_millis bigint, evaluated_at_millis bigint,
    all_students_completed boolean,
    grading_is_current boolean, grading_results jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT course.course_instance_id,
           course.course_short_name,
           course.course_long_name,
           course.course_theme_id AS course_theme,
           history.assessment_id,
           history.assessment_title,
           history.assessment_type,
           history.assessment_attempt_number,
           history.state,
           history.questions,
           history.feedback_rule,
           history.due_at_millis,
           history.closes_at_millis,
           history.submitted_at_millis,
           history.evaluated_at_millis,
           history.all_students_completed,
           history.grading_is_current,
           history.grading_results
      FROM ple_private.read_student_assessment_attempt_history(
               p_assessment_attempt_id
           ) AS history
      JOIN ple_data.course_instance AS course ON course.course_instance_id = history.course_instance_id
$$;

SET LOCAL ROLE ple_private_owner;




-- Reproduction facts are private retained evidence.  The server receives them
-- only for an owned completed Assessment Attempt. The server needs the exact
-- recorded response to select backend-provided feedback even when the public
-- response itself remains withheld by the copied disclosure policy.
-- The selected Question Revision and source binding are immutable, so this
-- reader never interprets a past Assessment Attempt through current Assessment content.
CREATE FUNCTION ple_private.read_student_assessment_attempt_history_response_sources(
    p_assessment_attempt_id uuid
) RETURNS TABLE (
    "position" integer, assessment_attempt_id uuid, student_response jsonb, backend text, question_attempt_id uuid,
    published_question_id text, revision_number integer, general_feedback text, source_object_record_id uuid, source_object_address jsonb,
    source_object_checksum text, webwork_pg_path text, question_seed text,
    generated_parameter_sha256 text,
    presentation_nonce text, presentation_checksum text, presentation jsonb, author_content jsonb,
    question_asset_renditions jsonb, response_item_bindings jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    WITH owned_assessment_attempt AS (
        SELECT assessment_attempt.assessment_attempt_id
          FROM ple_private.assessment_attempt AS assessment_attempt
          JOIN ple_data.assessment AS assessment
            ON assessment.assessment_id = assessment_attempt.assessment_id
          JOIN ple_private.assessment_submission
            ON assessment_submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id
         WHERE assessment_attempt.assessment_attempt_id = p_assessment_attempt_id
           -- ASVS 8.2.2 and 8.3.1: the trusted authorization helper proves
           -- the exact Course, Student Record, session Account, and active
           -- Student Membership without widening this private reader's table
           -- privileges.
           AND ple_api.current_session_account_owns_student_record(
               assessment.course_instance_id, assessment_attempt.student_record_id
           )
           -- ASVS 2.3.1: richer response-source history obeys the exact
           -- ordinary-visibility boundary too.
           AND ple_api.course_student_work_is_ordinarily_visible(assessment.course_instance_id)
    )
    SELECT issued.issued_position,
           owned.assessment_attempt_id,
           submission.student_response,
           binding.backend,
           question_attempt.question_attempt_id,
           issued.published_question_id,
           issued.revision_number,
           revision.general_feedback,
           binding.source_object_record_id,
           source_object.object_address,
           binding.source_object_checksum,
           binding.webwork_pg_path,
           question_attempt.question_seed::text,
           question_attempt.generated_parameter_sha256,
           presentation.presentation_nonce,
           encode(presentation.presentation_checksum, 'hex'),
           presentation.presentation,
           presentation.author_content,
           COALESCE(asset_renditions.question_asset_renditions, '[]'::jsonb),
           COALESCE(response_item_bindings.response_item_bindings, '[]'::jsonb)
      FROM owned_assessment_attempt AS owned
      JOIN ple_private.issued_question AS issued
        ON issued.assessment_attempt_id = owned.assessment_attempt_id
      JOIN ple_private.question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_data.question_revision AS revision
        ON revision.published_question_id = issued.published_question_id
       AND revision.revision_number = issued.revision_number
      LEFT JOIN ple_private.assessment_attempt_saved_response AS submission
        ON submission.question_attempt_id = question_attempt.question_attempt_id
       AND submission.finalized_at IS NOT NULL
      JOIN ple_private.question_attempt_presentation_binding AS presentation
        ON presentation.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_private.question_revision_source_binding AS binding
        ON binding.published_question_id = issued.published_question_id
       AND binding.revision_number = issued.revision_number
      LEFT JOIN ple_private.object_record AS source_object
        ON source_object.object_record_id = binding.source_object_record_id
      LEFT JOIN LATERAL (
          SELECT jsonb_agg(jsonb_build_object(
              'asset_id', presented.asset_id,
              'question_asset_checksum', encode(presented.question_asset_checksum, 'hex'),
              'rendition_checksum', encode(presented.rendition_checksum, 'hex'),
              'intrinsic_width', presented.intrinsic_width,
              'intrinsic_height', presented.intrinsic_height
          ) ORDER BY presented.asset_id) AS question_asset_renditions
            FROM ple_private.question_attempt_presentation_asset_rendition AS presented
           WHERE presented.question_attempt_presentation_asset_binding_id
                 = question_attempt.question_attempt_id
      ) AS asset_renditions ON true
      LEFT JOIN LATERAL (
          SELECT jsonb_agg(jsonb_build_object(
              'presentation_response_item_reference', response_item.presentation_response_item_reference,
              'response_item_reference', response_item.response_item_reference
          ) ORDER BY response_item.presentation_response_item_reference) AS response_item_bindings
            FROM ple_private.question_attempt_response_item_binding AS response_item
           WHERE response_item.question_attempt_presentation_binding_id
                 = question_attempt.question_attempt_id
      ) AS response_item_bindings ON true
     ORDER BY issued.issued_position
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.read_student_assessment_attempt_history_response_sources(
    p_assessment_attempt_id uuid
) RETURNS TABLE (
    "position" integer, assessment_attempt_id uuid, student_response jsonb, backend text, question_attempt_id uuid,
    published_question_id text, revision_number integer, general_feedback text, source_object_record_id uuid, source_object_address jsonb,
    source_object_checksum text, webwork_pg_path text, question_seed text,
    generated_parameter_sha256 text,
    presentation_nonce text, presentation_checksum text, presentation jsonb, author_content jsonb,
    question_asset_renditions jsonb, response_item_bindings jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT "position" + 1, assessment_attempt_id, student_response, backend, question_attempt_id,
           published_question_id, revision_number, general_feedback, source_object_record_id, source_object_address,
           source_object_checksum, webwork_pg_path, question_seed,
           generated_parameter_sha256,
           presentation_nonce, presentation_checksum, presentation, author_content, question_asset_renditions,
           response_item_bindings
      FROM ple_private.read_student_assessment_attempt_history_response_sources($1)
$$;

SET LOCAL ROLE ple_private_owner;




-- Archived Course Student Work is deliberately absent from ordinary Student
-- and Instructor projections, but C208's no-login retention capability may
-- verify the still-retained Work immediately before its scheduled deletion.
-- This is a protected pre-delete boundary, not a recovery interface, queue,
-- or state machine.  ASVS 2.3.1 keeps it reachable only after archive and
-- before the one-way delete transition.
CREATE FUNCTION ple_private.read_course_student_work_for_retention(
    p_student_record_id uuid
) RETURNS TABLE (
    assessment_attempt_id uuid,
    student_record_id uuid,
    assessment_id text,
    assessment_attempt_started_at timestamptz,
    assessment_submitted_at timestamptz
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT assessment_attempt.assessment_attempt_id,
           assessment_attempt.student_record_id,
           assessment_attempt.assessment_id,
           assessment_attempt.started_at,
           submission.submitted_at
      FROM ple_private.assessment_attempt AS assessment_attempt
      LEFT JOIN ple_private.assessment_submission AS submission
        ON submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id
     WHERE assessment_attempt.student_record_id = p_student_record_id
     ORDER BY assessment_attempt.assessment_attempt_id
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.read_archived_course_student_work_for_retention(
    p_course_instance_id text
) RETURNS TABLE (
    assessment_attempt_id uuid,
    student_record_id uuid,
    assessment_id text,
    assessment_attempt_started_at timestamptz,
    assessment_submitted_at timestamptz,
    student_data_archived_at timestamptz
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT work.assessment_attempt_id,
           work.student_record_id,
           work.assessment_id,
           work.assessment_attempt_started_at,
           work.assessment_submitted_at,
           course.student_data_archived_at
      FROM ple_data.course_instance AS course
      JOIN ple_data.student_record AS student
        ON student.course_instance_id = course.course_instance_id
      CROSS JOIN LATERAL ple_private.read_course_student_work_for_retention(student.student_record_id) AS work
     WHERE course.course_instance_id = p_course_instance_id
       AND course.retention_lifecycle_state = 'archived'
       AND course.student_data_archived_at IS NOT NULL
       AND course.student_data_deleted_at IS NULL
$$;

