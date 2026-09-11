-- Reject response mutations after a reject-rule Assignment deadline.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091016 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

CREATE OR REPLACE FUNCTION ple_api.save_student_assignment_attempt_response(
    p_assignment_attempt_reference_number bigint,
    p_position integer,
    p_student_response jsonb
) RETURNS TABLE (
    assignment_attempt_reference_number bigint,
    issued_position integer,
    response_state text,
    saved_at timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_attempt ple_private.assignment_attempt%ROWTYPE;
    v_question_attempt ple_private.question_attempt%ROWTYPE;
    v_attempt_time_limit_seconds integer;
    v_due_at timestamp with time zone;
    v_late_work_rule text;
    v_now timestamp with time zone;
BEGIN
    IF p_assignment_attempt_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_position NOT BETWEEN 1 AND 2147483647
       OR pg_catalog.jsonb_typeof(p_student_response) IS DISTINCT FROM 'object' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Student response save facts are invalid';
    END IF;

    SELECT attempt.* INTO v_attempt
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.student_record AS student
        ON student.student_record_id = attempt.student_record_id
      JOIN ple_data.assignment AS assignment
        ON assignment.assignment_id = attempt.assignment_id
      JOIN ple_private.account AS account
        ON account.account_id = student.student_account_id
     WHERE attempt.reference_number = p_assignment_attempt_reference_number
       AND attempt.completed_at IS NULL
       AND account.product_role = 'student'
       AND student.course_id = assignment.course_id
       AND student.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(
             assignment.course_id, student.student_record_id
       )
       AND EXISTS (
           SELECT 1 FROM ple_data.course_membership AS membership
            WHERE membership.course_id = assignment.course_id
              AND membership.account_id = student.student_account_id
              AND membership.role = 'student'
              AND ple_data.course_membership_is_active(membership.membership_id)
       )
     FOR UPDATE OF attempt;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Student response save is unavailable';
    END IF;

    SELECT revision.assignment_attempt_time_limit_seconds,
           revision.due_at,
           revision.late_work_rule
      INTO v_attempt_time_limit_seconds, v_due_at, v_late_work_rule
      FROM ple_data.assignment_revision AS revision
     WHERE revision.assignment_id = v_attempt.assignment_id
       AND revision.assignment_revision_id = v_attempt.assignment_revision_id;

    SELECT question_attempt.* INTO v_question_attempt
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
     WHERE issued.assignment_attempt_id = v_attempt.assignment_attempt_id
       AND issued.issued_position = p_position - 1
     FOR UPDATE OF question_attempt;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Student response save is unavailable';
    END IF;

    v_now := pg_catalog.clock_timestamp();
    IF v_question_attempt.question_attempt_state <> 'open'
       OR (v_attempt_time_limit_seconds IS NOT NULL
           AND v_attempt.started_at
                 + v_attempt_time_limit_seconds * interval '1 second' <= v_now)
       OR (v_due_at IS NOT NULL
           AND v_late_work_rule = 'reject'
           AND v_now > v_due_at)
       OR (v_question_attempt.deadline_at IS NOT NULL
           AND v_question_attempt.deadline_at <= v_now) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Student response cannot be saved';
    END IF;

    INSERT INTO ple_private.assignment_attempt_saved_response (
        question_attempt_id, student_response, saved_at
    ) VALUES (
        v_question_attempt.question_attempt_id, p_student_response, v_now
    ) ON CONFLICT (question_attempt_id) DO UPDATE
        SET student_response = EXCLUDED.student_response,
            saved_at = EXCLUDED.saved_at
      WHERE ple_private.assignment_attempt_saved_response.student_response
            IS DISTINCT FROM EXCLUDED.student_response;

    RETURN QUERY
    SELECT v_attempt.reference_number, p_position, 'saved'::text,
           COALESCE(
               (SELECT saved_response.saved_at
                  FROM ple_private.assignment_attempt_saved_response AS saved_response
                 WHERE saved_response.question_attempt_id =
                       v_question_attempt.question_attempt_id),
               v_now
           );
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.save_student_assignment_attempt_response(bigint, integer, jsonb)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.save_student_assignment_attempt_response(bigint, integer, jsonb)
    TO ple_app;

RESET ROLE;
SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
