-- Functions, triggers, and views from delivery.sql.

SET LOCAL ROLE ple_private_owner;

-- Delivery procedures are the narrow bridge from authenticated Students and
-- grading workers to retained Question Attempt evidence.  They lock the
-- Assessment before its Student Work, so Unrelease wins or loses atomically.
CREATE FUNCTION ple_private.require_owned_open_question_attempt(
    p_course_instance_id text, p_assessment_id text, p_question_attempt_id uuid
) RETURNS ple_private.question_attempt
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE result ple_private.question_attempt%ROWTYPE;
BEGIN
    -- Assessment is intentionally the first row lock in every delivery write.
    PERFORM 1 FROM ple_data.assessment AS assessment
     WHERE assessment.assessment_id = p_assessment_id AND assessment.course_instance_id = p_course_instance_id
     FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question delivery is unavailable'; END IF;
    SELECT question_attempt.* INTO result
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assessment_attempt AS assessment_attempt ON assessment_attempt.assessment_attempt_id = issued.assessment_attempt_id
      JOIN ple_data.student_record AS student ON student.student_record_id = assessment_attempt.student_record_id
     WHERE question_attempt.question_attempt_id = p_question_attempt_id
       AND assessment_attempt.assessment_id = p_assessment_id
       AND student.course_instance_id = p_course_instance_id
       AND student.student_account_id = ple_api.current_session_account_id()
       AND NOT EXISTS (
           SELECT 1 FROM ple_private.assessment_submission AS submission
            WHERE submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id
       )
       AND ple_api.current_session_account_owns_student_record(p_course_instance_id, student.student_record_id)
     FOR UPDATE OF question_attempt, issued, assessment_attempt;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question delivery is unavailable'; END IF;
    RETURN result;
END $$;


