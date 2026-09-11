-- Selected completed Student Assignment Attempt history reader.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091026 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_api.read_student_assignment_attempt_history(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (
    course_reference_number bigint, course_title text, course_theme text,
    assignment_reference_number bigint, assignment_title text, attempt_number integer,
    state text, questions jsonb, feedback_rule jsonb, due_at_millis bigint,
    closes_at_millis bigint, submitted_at_millis bigint, evaluated_at_millis bigint,
    grading_is_current boolean,
    grading_results jsonb
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
    WITH evaluated_at AS (
        SELECT pg_catalog.statement_timestamp() AS value
    ), owned_attempt AS (
        SELECT attempt.*, course.reference_number AS course_reference_number,
               course.course_title, course.course_theme,
               assignment.reference_number AS assignment_reference_number,
               CASE WHEN attempt.delivery_assignment_title IS NOT NULL
                    THEN attempt.delivery_assignment_title ELSE revision.assignment_title END AS assignment_title,
               CASE WHEN attempt.delivery_assignment_title IS NOT NULL
                    THEN attempt.delivery_due_at ELSE revision.due_at END AS due_at,
               revision.closes_at,
               jsonb_build_object(
                   'score', revision.feedback_score,
                   'per_item_correctness', revision.feedback_per_item_correctness,
                   'submitted_response', revision.feedback_submitted_response,
                   'question_feedback', revision.feedback_question_feedback,
                   'question_answer', revision.feedback_question_answer,
                   'question_answer_explanation', revision.feedback_question_answer_explanation,
                   'class_statistics', revision.feedback_class_statistics
               ) AS feedback_rule
          FROM ple_private.assignment_attempt AS attempt
          JOIN ple_data.student_record AS student ON student.student_record_id = attempt.student_record_id
          JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
          JOIN ple_data.course_instance AS course ON course.course_id = assignment.course_id
          JOIN ple_data.assignment_revision AS revision ON revision.assignment_revision_id = attempt.assignment_revision_id
         WHERE attempt.reference_number = p_assignment_attempt_reference_number
           AND attempt.completed_at IS NOT NULL
           AND student.course_id = assignment.course_id
           AND student.student_account_id = ple_api.current_session_account_id()
           AND ple_api.current_session_account_owns_student_record(assignment.course_id, student.student_record_id)
           AND EXISTS (
               SELECT 1 FROM ple_data.course_membership AS membership
                WHERE membership.course_id = assignment.course_id
                  AND membership.account_id = student.student_account_id
                  AND membership.role = 'student'
                  AND ple_data.course_membership_is_active(membership.membership_id)
           )
    )
    SELECT owned.course_reference_number, owned.course_title, owned.course_theme,
           owned.assignment_reference_number, owned.assignment_title, owned.attempt_number,
           CASE WHEN assignment_submission.assignment_attempt_id IS NOT NULL
                THEN 'submitted'::text ELSE 'closed'::text END,
           COALESCE(jsonb_agg(jsonb_build_object(
               'position', issued.issued_position + 1, 'responseState',
               CASE WHEN submission.question_attempt_id IS NULL THEN 'closed' ELSE 'submitted' END
           ) ORDER BY issued.issued_position), '[]'::jsonb),
           owned.feedback_rule,
           floor(extract(epoch FROM owned.due_at) * 1000)::bigint,
           floor(extract(epoch FROM owned.closes_at) * 1000)::bigint,
           floor(extract(epoch FROM assignment_submission.submitted_at) * 1000)::bigint,
           floor(extract(epoch FROM evaluated_at.value) * 1000)::bigint,
           complete_grading.grading_is_current,
           complete_grading.grading_results
      FROM owned_attempt AS owned
      JOIN ple_private.issued_question AS issued ON issued.assignment_attempt_id = owned.assignment_attempt_id
      JOIN ple_private.question_attempt AS question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.question_submission AS submission ON submission.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_private.assignment_submission AS assignment_submission
        ON assignment_submission.assignment_attempt_id = owned.assignment_attempt_id
      CROSS JOIN LATERAL (
          SELECT count(*) > 0
                 AND count(result.grading_result_id) = count(*)
                 AND bool_and(grading.grading_state = 'graded')
                 AND count(*) FILTER (WHERE ple_api.has_automated_grading_receipt(
                         grading.question_submission_grading_id,
                         result.grading_result_id
                     )) = count(*) AS grading_is_current,
                 CASE WHEN count(*) > 0
                           AND count(result.grading_result_id) = count(*)
                           AND bool_and(grading.grading_state = 'graded')
                           AND count(*) FILTER (WHERE ple_api.has_automated_grading_receipt(
                                   grading.question_submission_grading_id,
                                   result.grading_result_id
                               )) = count(*)
                      THEN jsonb_agg(jsonb_build_object(
                          'position', grading_issued.issued_position + 1,
                          'correct', result.correct,
                          'pointsEarned', result.points_earned,
                          'pointsPossible', result.points_possible
                      ) ORDER BY grading_issued.issued_position)
                      ELSE '[]'::jsonb END AS grading_results
            FROM ple_private.issued_question AS grading_issued
            JOIN ple_private.question_attempt AS grading_attempt
              ON grading_attempt.issued_question_id = grading_issued.issued_question_id
            LEFT JOIN ple_private.question_submission AS grading_submission
              ON grading_submission.question_attempt_id = grading_attempt.question_attempt_id
            LEFT JOIN ple_private.question_submission_grading AS grading
              ON grading.submission_id = grading_submission.submission_id
            LEFT JOIN ple_private.grading_result AS result
              ON result.question_submission_grading_id = grading.question_submission_grading_id
             AND result.submission_id = grading_submission.submission_id
             AND result.question_attempt_id = grading_attempt.question_attempt_id
           WHERE grading_issued.assignment_attempt_id = owned.assignment_attempt_id
      ) AS complete_grading
      CROSS JOIN evaluated_at
     GROUP BY owned.course_reference_number, owned.course_title, owned.course_theme,
              owned.assignment_reference_number, owned.assignment_title, owned.attempt_number,
              owned.feedback_rule, owned.due_at, owned.closes_at,
              assignment_submission.assignment_attempt_id, assignment_submission.submitted_at,
              evaluated_at.value, complete_grading.grading_is_current,
              complete_grading.grading_results
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.read_student_assignment_attempt_history(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_student_assignment_attempt_history(bigint) TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
