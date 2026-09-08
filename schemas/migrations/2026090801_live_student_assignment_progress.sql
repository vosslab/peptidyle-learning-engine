-- Add self-only, answer-free progress to released Student Assignment cards.

SET LOCAL ROLE ple_api_owner;
DROP FUNCTION ple_api.list_released_live_student_assignments(bigint);
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

-- ASVS 4.1.3 and 8.3.1: the current session Account and exact active Student
-- Course Membership are derived inside this boundary. Only the learner's own
-- Assignment Attempt completion and numeric aggregates leave ple_private.
CREATE FUNCTION ple_api.list_released_live_student_assignments(
    p_course_reference_number bigint
)
RETURNS TABLE (
    assignment_reference_number bigint,
    assignment_title text,
    assignment_attempt_number integer,
    assignment_attempt_completion text,
    graded_question_count bigint,
    question_count bigint,
    points_earned double precision,
    points_possible double precision
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_course_id uuid;
    v_student_record_id uuid;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647 THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course is unavailable';
    END IF;

    SELECT course.course_id, student_record.student_record_id
      INTO v_course_id, v_student_record_id
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS account_state ON account_state.state = 'active'
      JOIN ple_data.course_membership AS membership
        ON membership.account_id = account.account_id
       AND membership.role = 'student'
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_instance AS course
        ON course.course_id = membership.course_id
      JOIN ple_data.student_record AS student_record
        ON student_record.student_record_id = membership.student_record_id
       AND student_record.course_id = course.course_id
       AND student_record.student_account_id = account.account_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'student'
       AND course.reference_number = p_course_reference_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course is unavailable';
    END IF;

    RETURN QUERY
    WITH released_assignment AS (
        SELECT assignment.assignment_id,
               assignment.reference_number,
               assignment.assignment_title,
               COALESCE(question_total.question_count, 0)::bigint AS question_count
          FROM ple_data.assignment AS assignment
          LEFT JOIN LATERAL (
              SELECT sum(
                         CASE entry.entry_kind
                             WHEN 'fixed_question' THEN 1
                             ELSE pool.selection_count
                         END
                     ) AS question_count
                FROM ple_data.assignment_revision_entry AS entry
                LEFT JOIN ple_data.assignment_revision_question_pool AS pool
                  ON pool.assignment_revision_id = entry.assignment_revision_id
                 AND pool.assignment_entry_id = entry.assignment_entry_id
               WHERE entry.assignment_revision_id =
                     assignment.released_assignment_revision_id
          ) AS question_total ON true
         WHERE assignment.course_id = v_course_id
           AND assignment.assignment_status = 'released'
    )
    SELECT assignment.reference_number, assignment.assignment_title,
           attempt.attempt_number,
           CASE
               WHEN attempt.assignment_attempt_id IS NULL THEN NULL
               WHEN attempt.completed_at IS NOT NULL THEN 'completed'
               ELSE 'in_progress'
           END,
           count(DISTINCT result.grading_result_id)::bigint,
           assignment.question_count,
           COALESCE(sum(result.points_earned), 0)::double precision,
           COALESCE(sum(result.points_possible), 0)::double precision
      FROM released_assignment AS assignment
      LEFT JOIN LATERAL (
          SELECT candidate.assignment_attempt_id, candidate.attempt_number,
                 candidate.completed_at
            FROM ple_private.assignment_attempt AS candidate
           WHERE candidate.student_record_id = v_student_record_id
             AND candidate.assignment_id = assignment.assignment_id
           ORDER BY candidate.started_at DESC,
                    candidate.assignment_attempt_id DESC
           LIMIT 1
      ) AS attempt ON true
      LEFT JOIN ple_private.issued_question AS issued
        ON issued.assignment_attempt_id = attempt.assignment_attempt_id
      LEFT JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.grading_result AS result
        ON result.question_attempt_id = question_attempt.question_attempt_id
     GROUP BY assignment.reference_number, assignment.assignment_title,
              assignment.question_count, attempt.assignment_attempt_id,
              attempt.attempt_number, attempt.completed_at
     ORDER BY assignment.assignment_title, assignment.reference_number;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.list_released_live_student_assignments(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.list_released_live_student_assignments(bigint) TO ple_app;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
