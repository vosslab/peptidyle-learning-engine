-- M5 Student Assignment Attempt route context.
--
-- This public-reference projection supplies only the Student delivery chrome.
-- It reuses the Attempt's pinned Assignment Revision for title and time-limit
-- evidence, while Course identity remains current Course display context.

SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_api.read_student_assignment_attempt_context(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (
    assignment_attempt_reference_number bigint,
    attempt_number integer,
    course_reference_number bigint,
    course_title text,
    course_theme text,
    assignment_reference_number bigint,
    assignment_title text,
    timer_remaining_milliseconds bigint
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    WITH evaluated_at AS (
        SELECT pg_catalog.statement_timestamp() AS value
    )
    SELECT attempt.reference_number,
           attempt.attempt_number,
           course.reference_number,
           course.course_title,
           course.course_theme,
           assignment.reference_number,
           revision.assignment_title,
           CASE
               WHEN revision.assignment_attempt_time_limit_seconds IS NULL THEN NULL
               ELSE GREATEST(
                   0::bigint,
                   pg_catalog.floor(
                       EXTRACT(
                           epoch FROM (
                               attempt.started_at
                               + revision.assignment_attempt_time_limit_seconds * interval '1 second'
                               - evaluated_at.value
                           )
                       ) * 1000
                   )::bigint
               )
           END
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.student_record AS student
        ON student.student_record_id = attempt.student_record_id
      JOIN ple_data.assignment AS assignment
        ON assignment.assignment_id = attempt.assignment_id
      JOIN ple_data.assignment_revision AS revision
        ON revision.assignment_id = attempt.assignment_id
       AND revision.assignment_revision_id = attempt.assignment_revision_id
      JOIN ple_data.course_instance AS course
        ON course.course_id = assignment.course_id
      CROSS JOIN evaluated_at
     WHERE p_assignment_attempt_reference_number BETWEEN 1 AND 2147483647
       AND attempt.reference_number = p_assignment_attempt_reference_number
       AND (
           attempt.completed_at IS NULL
           OR EXISTS (
               SELECT 1 FROM ple_private.assignment_submission AS submission
                WHERE submission.assignment_attempt_id = attempt.assignment_attempt_id
           )
       )
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
$$;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.read_student_assignment_attempt_context(bigint)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.read_student_assignment_attempt_context(bigint)
    TO ple_app;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
