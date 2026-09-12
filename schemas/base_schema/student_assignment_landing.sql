-- Student assignment landing projection.  Current Assignment configuration
-- supplies only an unstarted Assignment's visible question count; an Attempt
-- is interpreted entirely through its retained Student Work evidence.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.read_student_released_assignment_landing_evidence(
    p_course_id uuid,
    p_student_record_id uuid,
    p_now timestamptz
) RETURNS TABLE (
    assignment_reference_number bigint,
    assignment_title text,
    assignment_attempt_number integer,
    assignment_attempt_completion text,
    graded_question_count bigint,
    question_count bigint,
    points_earned double precision,
    points_possible double precision
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private, ple_audit AS $$
    WITH released_assignment AS (
        SELECT assignment.assignment_id,
               assignment.reference_number,
               assignment.assignment_title,
               coalesce(sum(CASE entry.entry_kind
                   WHEN 'fixed_question' THEN 1
                   ELSE entry.selection_count
               END) FILTER (WHERE entry.availability = 'available'), 0)::bigint
                   AS current_question_count
          FROM ple_data.assignment AS assignment
          LEFT JOIN ple_data.assignment_entry AS entry
            ON entry.assignment_id = assignment.assignment_id
         WHERE assignment.course_id = p_course_id
           AND assignment.assignment_status = 'released'
         GROUP BY assignment.assignment_id, assignment.reference_number,
                  assignment.assignment_title
    )
    SELECT assignment.reference_number,
           CASE WHEN attempt.assignment_attempt_id IS NULL
                THEN assignment.assignment_title
                ELSE attempt.assignment_title
           END,
           attempt.attempt_number,
           CASE
               WHEN attempt.assignment_attempt_id IS NULL THEN NULL
               WHEN attempt.completed_at IS NULL THEN 'in_progress'
               ELSE 'completed'
           END,
           coalesce(evidence.graded_question_count, 0)::bigint,
           CASE WHEN attempt.assignment_attempt_id IS NULL
                THEN assignment.current_question_count
                ELSE coalesce(evidence.question_count, 0)::bigint
           END,
           CASE WHEN evidence.score_is_current
                     AND CASE attempt.feedback_score
                         WHEN 'during_attempt' THEN true
                         WHEN 'after_submit' THEN submission.assignment_attempt_id IS NOT NULL
                         WHEN 'after_due' THEN attempt.due_at IS NOT NULL AND p_now >= attempt.due_at
                         WHEN 'after_close' THEN attempt.closes_at IS NOT NULL AND p_now >= attempt.closes_at
                         ELSE false
                     END
                THEN evidence.points_earned ELSE NULL END,
           CASE WHEN evidence.score_is_current
                     AND CASE attempt.feedback_score
                         WHEN 'during_attempt' THEN true
                         WHEN 'after_submit' THEN submission.assignment_attempt_id IS NOT NULL
                         WHEN 'after_due' THEN attempt.due_at IS NOT NULL AND p_now >= attempt.due_at
                         WHEN 'after_close' THEN attempt.closes_at IS NOT NULL AND p_now >= attempt.closes_at
                         ELSE false
                     END
                THEN evidence.points_possible ELSE NULL END
      FROM released_assignment AS assignment
      LEFT JOIN LATERAL (
          SELECT candidate.assignment_attempt_id, candidate.assignment_title,
                 candidate.attempt_number,
                 candidate.completed_at, candidate.feedback_score,
                 candidate.due_at, candidate.closes_at
            FROM ple_private.assignment_attempt AS candidate
           WHERE candidate.student_record_id = p_student_record_id
             AND candidate.assignment_id = assignment.assignment_id
           ORDER BY candidate.started_at DESC, candidate.assignment_attempt_id DESC
           LIMIT 1
      ) AS attempt ON true
      LEFT JOIN ple_private.assignment_submission AS submission
        ON submission.assignment_attempt_id = attempt.assignment_attempt_id
      LEFT JOIN LATERAL (
          SELECT count(issued.issued_question_id)::bigint AS question_count,
                 count(*) FILTER (WHERE chain.is_complete)::bigint AS graded_question_count,
                 count(issued.issued_question_id) > 0
                   AND bool_and(chain.is_complete) AS score_is_current,
                 coalesce(sum(chain.points_earned) FILTER (WHERE chain.is_complete), 0)::double precision
                   AS points_earned,
                 coalesce(sum(chain.points_possible) FILTER (WHERE chain.is_complete), 0)::double precision
                   AS points_possible
            FROM ple_private.issued_question AS issued
            LEFT JOIN LATERAL (
                SELECT count(DISTINCT question_attempt.question_attempt_id) = 1
                           AND count(DISTINCT question_submission.submission_id) = 1
                           AND count(DISTINCT grading.question_submission_grading_id) = 1
                           AND count(DISTINCT result.grading_result_id) = 1
                           AND count(DISTINCT receipt.automated_grading_receipt_id) = 1
                           AND coalesce(bool_and(grading.grading_state = 'graded'), false)
                           AS is_complete,
                       coalesce(sum(result.points_earned), 0)::double precision AS points_earned,
                       coalesce(sum(result.points_possible), 0)::double precision AS points_possible
                  FROM ple_private.question_attempt AS question_attempt
                  LEFT JOIN ple_private.question_submission AS question_submission
                    ON question_submission.question_attempt_id = question_attempt.question_attempt_id
                  LEFT JOIN ple_private.question_submission_grading AS grading
                    ON grading.submission_id = question_submission.submission_id
                  LEFT JOIN ple_private.grading_result AS result
                    ON result.question_submission_grading_id = grading.question_submission_grading_id
                   AND result.submission_id = question_submission.submission_id
                   AND result.question_attempt_id = question_attempt.question_attempt_id
                  LEFT JOIN ple_audit.automated_grading_receipt AS receipt
                    ON receipt.question_submission_grading_id = grading.question_submission_grading_id
                   AND receipt.grading_result_id = result.grading_result_id
                 WHERE question_attempt.issued_question_id = issued.issued_question_id
            ) AS chain ON true
           WHERE issued.assignment_attempt_id = attempt.assignment_attempt_id
      ) AS evidence ON true
     ORDER BY assignment.assignment_title, assignment.reference_number
$$;

REVOKE ALL ON FUNCTION ple_private.read_student_released_assignment_landing_evidence(
    uuid, uuid, timestamptz
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.read_student_released_assignment_landing_evidence(
    uuid, uuid, timestamptz
) TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

-- The public projection establishes exact active Student membership before it
-- delegates evidence reads.  A failed lookup is indistinguishable from a
-- foreign Course.
CREATE FUNCTION ple_api.list_released_live_student_assignments(
    p_course_reference_number bigint
) RETURNS TABLE (
    assignment_reference_number bigint,
    assignment_title text,
    assignment_attempt_number integer,
    assignment_attempt_completion text,
    graded_question_count bigint,
    question_count bigint,
    points_earned double precision,
    points_possible double precision
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    course_id_value uuid;
    student_record_id_value uuid;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647 THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course is unavailable';
    END IF;

    SELECT course.course_id, student.student_record_id
      INTO course_id_value, student_record_id_value
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS state_event ON state_event.state = 'active'
      JOIN ple_data.course_membership AS membership
        ON membership.account_id = account.account_id
       AND membership.role = 'student'
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_instance AS course
        ON course.course_id = membership.course_id
      JOIN ple_data.student_record AS student
        ON student.student_record_id = membership.student_record_id
       AND student.course_id = course.course_id
       AND student.student_account_id = account.account_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'student'
       AND course.reference_number = p_course_reference_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course is unavailable';
    END IF;

    RETURN QUERY
    SELECT * FROM ple_private.read_student_released_assignment_landing_evidence(
        course_id_value, student_record_id_value, pg_catalog.clock_timestamp()
    );
END
$$;

REVOKE ALL ON FUNCTION ple_api.list_released_live_student_assignments(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_released_live_student_assignments(bigint) TO ple_app;

RESET ROLE;
