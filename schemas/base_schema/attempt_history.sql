-- Completed Student Assignment Attempt history.  This projection reads only
-- retained Attempt and Issued Question evidence; current Assignment content
-- is deliberately not an interpretation source.

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.read_student_assignment_attempt_history(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (
    course_id uuid, assignment_reference_number bigint, assignment_title text, attempt_number integer,
    state text, questions jsonb, feedback_rule jsonb, due_at_millis bigint,
    closes_at_millis bigint, submitted_at_millis bigint, evaluated_at_millis bigint,
    grading_is_current boolean, grading_results jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
    WITH owned_attempt AS (
        SELECT attempt.assignment_attempt_id, assignment.course_id,
               assignment.reference_number AS assignment_reference_number,
               attempt.assignment_title,
               attempt.attempt_number,
               attempt.due_at,
               attempt.closes_at,
               jsonb_build_object(
                   'score', attempt.feedback_score,
                   'per_item_correctness', attempt.feedback_per_item_correctness,
                   'submitted_response', attempt.feedback_submitted_response,
                   'question_feedback', attempt.feedback_question_feedback,
                   'question_answer', attempt.feedback_question_answer,
                   'question_answer_explanation', attempt.feedback_question_answer_explanation,
                   'class_statistics', attempt.feedback_class_statistics
               ) AS feedback_rule
          FROM ple_private.assignment_attempt AS attempt
          JOIN ple_data.assignment AS assignment
            ON assignment.assignment_id = attempt.assignment_id
         WHERE attempt.reference_number = p_assignment_attempt_reference_number
           AND attempt.completed_at IS NOT NULL
           AND ple_api.current_session_account_owns_student_record(
               assignment.course_id, attempt.student_record_id
           )
    )
    SELECT owned.course_id,
           owned.assignment_reference_number,
           owned.assignment_title,
           owned.attempt_number,
           CASE WHEN assignment_submission.assignment_attempt_id IS NOT NULL
                THEN 'submitted'::text ELSE 'closed'::text END,
           questions.questions,
           owned.feedback_rule,
           floor(extract(epoch FROM owned.due_at) * 1000)::bigint,
           floor(extract(epoch FROM owned.closes_at) * 1000)::bigint,
           floor(extract(epoch FROM assignment_submission.submitted_at) * 1000)::bigint,
           floor(extract(epoch FROM pg_catalog.statement_timestamp()) * 1000)::bigint,
           grading.grading_is_current,
           grading.grading_results
      FROM owned_attempt AS owned
      LEFT JOIN ple_private.assignment_submission AS assignment_submission
        ON assignment_submission.assignment_attempt_id = owned.assignment_attempt_id
      CROSS JOIN LATERAL (
          SELECT COALESCE(jsonb_agg(jsonb_build_object(
                     'position', issued.issued_position + 1,
                     'questionId', issued.question_id,
                     'revisionNumber', issued.revision_number,
                     'responseState', CASE question_attempt.question_attempt_state
                         WHEN 'submission_accepted' THEN 'submitted'
                         ELSE 'closed'
                     END
                 ) ORDER BY issued.issued_position), '[]'::jsonb) AS questions
            FROM ple_private.issued_question AS issued
            JOIN ple_private.question_attempt
              ON question_attempt.issued_question_id = issued.issued_question_id
           WHERE issued.assignment_attempt_id = owned.assignment_attempt_id
      ) AS questions
      CROSS JOIN LATERAL (
          SELECT count(*) > 0
                     AND count(result.grading_result_id) = count(*)
                     AND bool_and(grading_state.grading_state = 'graded')
                     AND count(*) FILTER (
                         WHERE ple_api.has_automated_grading_receipt(
                             grading_state.question_submission_grading_id,
                             result.grading_result_id
                         )
                     ) = count(*) AS grading_is_current,
                 CASE WHEN count(*) > 0
                           AND count(result.grading_result_id) = count(*)
                           AND bool_and(grading_state.grading_state = 'graded')
                           AND count(*) FILTER (
                               WHERE ple_api.has_automated_grading_receipt(
                                   grading_state.question_submission_grading_id,
                                   result.grading_result_id
                               )
                           ) = count(*)
                      THEN jsonb_agg(jsonb_build_object(
                          'position', issued.issued_position + 1,
                          'correct', result.correct,
                          'pointsEarned', result.points_earned,
                          'pointsPossible', result.points_possible
                      ) ORDER BY issued.issued_position)
                      ELSE '[]'::jsonb END AS grading_results
            FROM ple_private.issued_question AS issued
            JOIN ple_private.question_attempt
              ON question_attempt.issued_question_id = issued.issued_question_id
            LEFT JOIN ple_private.question_submission AS submission
              ON submission.question_attempt_id = question_attempt.question_attempt_id
            LEFT JOIN ple_private.question_submission_grading AS grading_state
              ON grading_state.submission_id = submission.submission_id
            LEFT JOIN ple_private.grading_result AS result
              ON result.question_submission_grading_id = grading_state.question_submission_grading_id
             AND result.submission_id = submission.submission_id
             AND result.question_attempt_id = question_attempt.question_attempt_id
           WHERE issued.assignment_attempt_id = owned.assignment_attempt_id
      ) AS grading
$$;

REVOKE ALL ON FUNCTION ple_private.read_student_assignment_attempt_history(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.read_student_assignment_attempt_history(bigint) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.read_student_assignment_attempt_history(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (
    course_reference_number bigint, course_short_name text, course_long_name text, course_theme text,
    assignment_reference_number bigint, assignment_title text, attempt_number integer,
    state text, questions jsonb, feedback_rule jsonb, due_at_millis bigint,
    closes_at_millis bigint, submitted_at_millis bigint, evaluated_at_millis bigint,
    grading_is_current boolean, grading_results jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT course.reference_number,
           course.course_short_name,
           course.course_long_name,
           course.course_theme,
           history.assignment_reference_number,
           history.assignment_title,
           history.attempt_number,
           history.state,
           history.questions,
           history.feedback_rule,
           history.due_at_millis,
           history.closes_at_millis,
           history.submitted_at_millis,
           history.evaluated_at_millis,
           history.grading_is_current,
           history.grading_results
      FROM ple_private.read_student_assignment_attempt_history(
               p_assignment_attempt_reference_number
           ) AS history
      JOIN ple_data.course_instance AS course ON course.course_id = history.course_id
$$;

REVOKE ALL ON FUNCTION ple_api.read_student_assignment_attempt_history(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_student_assignment_attempt_history(bigint) TO ple_app;
RESET ROLE;

-- Reproduction facts are private retained evidence.  The server receives them
-- only for an owned completed Attempt and only after at least one teaching
-- content field is releasable from that Attempt's copied feedback policy.
-- The selected Question Revision and source binding are immutable, so this
-- reader never interprets a past Attempt through current Assignment content.
SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.read_student_assignment_attempt_history_response_sources(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (
    "position" integer, assignment_attempt_id uuid, student_response jsonb, backend text, question_attempt_id uuid,
    question_id text, revision_number integer, source_object_id uuid, source_object_address jsonb,
    source_object_checksum text, webwork_pg_path text, question_seed text,
    presentation_nonce text, presentation_checksum text, presentation jsonb,
    question_asset_renditions jsonb, response_item_bindings jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    WITH owned_attempt AS (
        SELECT attempt.assignment_attempt_id,
               attempt.feedback_submitted_response,
               attempt.feedback_question_feedback,
               attempt.feedback_question_answer,
               attempt.feedback_question_answer_explanation,
               attempt.due_at,
               attempt.closes_at,
               assignment_submission.submitted_at AS assignment_submitted_at
          FROM ple_private.assignment_attempt AS attempt
          JOIN ple_data.student_record AS student
            ON student.student_record_id = attempt.student_record_id
          JOIN ple_data.assignment AS assignment
            ON assignment.assignment_id = attempt.assignment_id
          LEFT JOIN ple_private.assignment_submission
            ON assignment_submission.assignment_attempt_id = attempt.assignment_attempt_id
         WHERE attempt.reference_number = p_assignment_attempt_reference_number
           AND attempt.completed_at IS NOT NULL
           AND student.course_id = assignment.course_id
           AND student.student_account_id = ple_api.current_session_account_id()
           AND ple_api.current_session_account_owns_student_record(
               assignment.course_id, student.student_record_id
           )
           AND EXISTS (
               SELECT 1
                 FROM ple_data.course_membership AS membership
                WHERE membership.course_id = assignment.course_id
                  AND membership.account_id = student.student_account_id
                  AND membership.role = 'student'
                  AND ple_data.course_membership_is_active(membership.membership_id)
           )
    ), released_attempt AS (
        SELECT owned.*,
               (
                   owned.feedback_submitted_response = 'during_attempt'
                   OR (owned.feedback_submitted_response = 'after_submit'
                       AND owned.assignment_submitted_at IS NOT NULL)
                   OR (owned.feedback_submitted_response = 'after_due'
                       AND owned.due_at IS NOT NULL
                       AND pg_catalog.statement_timestamp() >= owned.due_at)
                   OR (owned.feedback_submitted_response = 'after_close'
                       AND owned.closes_at IS NOT NULL
                       AND pg_catalog.statement_timestamp() >= owned.closes_at)
               ) AS submitted_response_is_released,
               (
                   owned.feedback_question_feedback = 'during_attempt'
                   OR (owned.feedback_question_feedback = 'after_submit'
                       AND owned.assignment_submitted_at IS NOT NULL)
                   OR (owned.feedback_question_feedback = 'after_due'
                       AND owned.due_at IS NOT NULL
                       AND pg_catalog.statement_timestamp() >= owned.due_at)
                   OR (owned.feedback_question_feedback = 'after_close'
                       AND owned.closes_at IS NOT NULL
                       AND pg_catalog.statement_timestamp() >= owned.closes_at)
                   OR owned.feedback_question_answer = 'during_attempt'
                   OR (owned.feedback_question_answer = 'after_submit'
                       AND owned.assignment_submitted_at IS NOT NULL)
                   OR (owned.feedback_question_answer = 'after_due'
                       AND owned.due_at IS NOT NULL
                       AND pg_catalog.statement_timestamp() >= owned.due_at)
                   OR (owned.feedback_question_answer = 'after_close'
                       AND owned.closes_at IS NOT NULL
                       AND pg_catalog.statement_timestamp() >= owned.closes_at)
                   OR owned.feedback_question_answer_explanation = 'during_attempt'
                   OR (owned.feedback_question_answer_explanation = 'after_submit'
                       AND owned.assignment_submitted_at IS NOT NULL)
                   OR (owned.feedback_question_answer_explanation = 'after_due'
                       AND owned.due_at IS NOT NULL
                       AND pg_catalog.statement_timestamp() >= owned.due_at)
                   OR (owned.feedback_question_answer_explanation = 'after_close'
                       AND owned.closes_at IS NOT NULL
                       AND pg_catalog.statement_timestamp() >= owned.closes_at)
               ) AS teaching_content_is_released
          FROM owned_attempt AS owned
    )
    SELECT issued.issued_position,
           released.assignment_attempt_id,
           CASE WHEN released.submitted_response_is_released
                THEN submission.student_response END,
           binding.backend,
           question_attempt.question_attempt_id,
           issued.question_id,
           issued.revision_number,
           binding.source_object_id,
           source_object.object_address,
           binding.source_object_checksum,
           binding.webwork_pg_path,
           question_attempt.question_seed::text,
           presentation.presentation_nonce,
           presentation.presentation_checksum,
           presentation.presentation,
           COALESCE(asset_renditions.question_asset_renditions, '[]'::jsonb),
           COALESCE(response_item_bindings.response_item_bindings, '[]'::jsonb)
      FROM released_attempt AS released
      JOIN ple_private.issued_question AS issued
        ON issued.assignment_attempt_id = released.assignment_attempt_id
      JOIN ple_private.question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.question_submission AS submission
        ON submission.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_private.question_attempt_presentation_binding AS presentation
        ON presentation.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_private.question_revision_source_binding AS binding
        ON binding.question_id = issued.question_id
       AND binding.revision_number = issued.revision_number
      LEFT JOIN ple_private.object_record AS source_object
        ON source_object.object_id = binding.source_object_id
      LEFT JOIN LATERAL (
          SELECT jsonb_agg(jsonb_build_object(
              'asset_id', presented.asset_id,
              'question_asset_checksum', encode(presented.question_asset_checksum, 'hex'),
              'rendition_checksum', encode(presented.rendition_checksum, 'hex'),
              'intrinsic_width', presented.intrinsic_width,
              'intrinsic_height', presented.intrinsic_height
          ) ORDER BY presented.asset_id) AS question_asset_renditions
            FROM ple_private.question_attempt_presentation_asset_rendition AS presented
           WHERE presented.question_attempt_id = question_attempt.question_attempt_id
      ) AS asset_renditions ON true
      LEFT JOIN LATERAL (
          SELECT jsonb_agg(jsonb_build_object(
              'presentation_response_item_reference', response_item.presentation_response_item_reference,
              'response_item_reference', response_item.response_item_reference
          ) ORDER BY response_item.presentation_response_item_reference) AS response_item_bindings
            FROM ple_private.question_attempt_response_item_binding AS response_item
           WHERE response_item.question_attempt_id = question_attempt.question_attempt_id
      ) AS response_item_bindings ON true
     WHERE released.submitted_response_is_released
        OR released.teaching_content_is_released
     ORDER BY issued.issued_position
$$;
REVOKE ALL ON FUNCTION ple_private.read_student_assignment_attempt_history_response_sources(bigint)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.read_student_assignment_attempt_history_response_sources(bigint)
    TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.read_student_assignment_attempt_history_response_sources(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (
    "position" integer, assignment_attempt_id uuid, student_response jsonb, backend text, question_attempt_id uuid,
    question_id text, revision_number integer, source_object_id uuid, source_object_address jsonb,
    source_object_checksum text, webwork_pg_path text, question_seed text,
    presentation_nonce text, presentation_checksum text, presentation jsonb,
    question_asset_renditions jsonb, response_item_bindings jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT "position" + 1, assignment_attempt_id, student_response, backend, question_attempt_id,
           question_id, revision_number, source_object_id, source_object_address,
           source_object_checksum, webwork_pg_path, question_seed,
           presentation_nonce, presentation_checksum, presentation, question_asset_renditions,
           response_item_bindings
      FROM ple_private.read_student_assignment_attempt_history_response_sources($1)
$$;
REVOKE ALL ON FUNCTION ple_api.read_student_assignment_attempt_history_response_sources(bigint)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_student_assignment_attempt_history_response_sources(bigint)
    TO ple_app;
RESET ROLE;
