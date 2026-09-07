-- Replace the data-bearing Student status projection without changing accepted
-- submission or grading evidence.

SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

DROP FUNCTION ple_api.read_live_demo_native_ple_submission_status(bigint, bigint, text);

CREATE FUNCTION ple_api.read_live_demo_native_ple_submission_status(
    p_course_reference_number bigint, p_assignment_reference_number bigint,
    p_presentation_nonce text
) RETURNS TABLE (presentation_nonce text, grading_state text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_presentation_nonce IS NULL OR p_presentation_nonce !~ '^[0-9a-f]{32}$' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Submission is unavailable';
    END IF;
    RETURN QUERY
    SELECT presentation.presentation_nonce, grading.grading_state
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_id = assignment.assignment_id
       AND assignment_attempt.completed_at IS NULL
      JOIN ple_data.student_record AS student_record
        ON student_record.student_record_id = assignment_attempt.student_record_id
      JOIN ple_private.issued_question AS issued
        ON issued.assignment_attempt_id = assignment_attempt.assignment_attempt_id
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
       AND student_record.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(
           assignment.course_id, student_record.student_record_id
       )
       AND presentation.presentation_nonce = p_presentation_nonce
       AND source_binding.backend = 'ple'
       AND source_binding.question_format = 'pleQuestionJson';
END $$;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.read_live_demo_native_ple_submission_status(bigint, bigint, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.read_live_demo_native_ple_submission_status(bigint, bigint, text) TO ple_app;

RESET ROLE;
SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
