-- Complete an Assignment Attempt when its released completion rule is met.

SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.complete_assignment_attempt_after_grading()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    v_assignment_attempt_id uuid;
    v_completion_rule text;
    v_score_threshold numeric;
    v_required_count bigint;
    v_graded_count bigint;
    v_all_correct boolean;
    v_points_earned numeric;
    v_points_possible numeric;
    v_complete boolean;
BEGIN
    SELECT assignment_attempt.assignment_attempt_id,
           revision.assignment_completion_rule,
           revision.assignment_completion_score_threshold
      INTO v_assignment_attempt_id, v_completion_rule, v_score_threshold
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_attempt_id =
           issued.assignment_attempt_id
      JOIN ple_data.assignment_revision AS revision
        ON revision.assignment_revision_id =
           assignment_attempt.assignment_revision_id
     WHERE question_attempt.question_attempt_id = NEW.question_attempt_id
       AND assignment_attempt.completed_at IS NULL
     FOR UPDATE OF assignment_attempt;
    IF NOT FOUND THEN
        RETURN NEW;
    END IF;

    SELECT count(*)::bigint,
           count(result.grading_result_id)::bigint,
           COALESCE(bool_and(result.correct), false),
           COALESCE(sum(result.points_earned), 0),
           COALESCE(sum(result.points_possible), 0)
      INTO v_required_count, v_graded_count, v_all_correct,
           v_points_earned, v_points_possible
      FROM ple_private.issued_question AS issued
      LEFT JOIN LATERAL (
          SELECT grading_result.grading_result_id,
                 grading_result.correct,
                 grading_result.points_earned,
                 grading_result.points_possible
            FROM ple_private.question_attempt AS question_attempt
            JOIN ple_private.grading_result AS grading_result
              ON grading_result.question_attempt_id =
                 question_attempt.question_attempt_id
           WHERE question_attempt.issued_question_id =
                 issued.issued_question_id
           ORDER BY grading_result.recorded_at DESC,
                    grading_result.grading_result_id DESC
           LIMIT 1
      ) AS result ON true
     WHERE issued.assignment_attempt_id = v_assignment_attempt_id;

    v_complete := v_required_count > 0
        AND v_graded_count = v_required_count
        AND CASE v_completion_rule
            WHEN 'answer_all' THEN true
            WHEN 'all_correct' THEN v_all_correct
            WHEN 'score_at_least' THEN
                v_points_possible > 0
                AND v_points_earned / v_points_possible >= v_score_threshold
            ELSE false
        END;
    IF v_complete THEN
        UPDATE ple_private.assignment_attempt
           SET completed_at = GREATEST(started_at, NEW.recorded_at)
         WHERE assignment_attempt_id = v_assignment_attempt_id
           AND completed_at IS NULL;
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER grading_result_completes_assignment_attempt
AFTER INSERT ON ple_private.grading_result
FOR EACH ROW
EXECUTE FUNCTION ple_private.complete_assignment_attempt_after_grading();

-- A completed Assignment Attempt retains self-only access to the accepted
-- Question Submission status that completed it.
CREATE OR REPLACE FUNCTION ple_api.read_live_demo_native_ple_submission_status(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint,
    p_presentation_nonce text
)
RETURNS TABLE (presentation_nonce text, grading_state text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_presentation_nonce IS NULL
       OR p_presentation_nonce !~ '^[0-9a-f]{32}$' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Submission is unavailable';
    END IF;
    RETURN QUERY
    SELECT presentation.presentation_nonce, grading.grading_state
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment
        ON assignment.course_id = course.course_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_id = assignment.assignment_id
      JOIN ple_data.student_record AS student_record
        ON student_record.student_record_id =
           assignment_attempt.student_record_id
      JOIN ple_private.issued_question AS issued
        ON issued.assignment_attempt_id =
           assignment_attempt.assignment_attempt_id
      JOIN ple_private.question_attempt AS attempt
        ON attempt.issued_question_id = issued.issued_question_id
       AND attempt.question_attempt_state = 'submission_accepted'
      JOIN ple_private.question_attempt_presentation_binding AS presentation
        ON presentation.question_attempt_id = attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS source_binding
        ON source_binding.question_id = issued.question_id
       AND source_binding.revision_number = issued.revision_number
      JOIN ple_private.question_submission AS submission
        ON submission.question_attempt_id = attempt.question_attempt_id
      JOIN ple_private.question_submission_grading AS grading
        ON grading.submission_id = submission.submission_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
       AND student_record.course_id = assignment.course_id
       AND student_record.student_account_id =
           ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(
           assignment.course_id, student_record.student_record_id
       )
       AND presentation.presentation_nonce = p_presentation_nonce
       AND source_binding.backend = 'ple'
       AND source_binding.question_format = 'pleQuestionJson';
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_private.complete_assignment_attempt_after_grading() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.read_live_demo_native_ple_submission_status(
        bigint, bigint, text
    ) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.read_live_demo_native_ple_submission_status(
        bigint, bigint, text
    ) TO ple_app;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
