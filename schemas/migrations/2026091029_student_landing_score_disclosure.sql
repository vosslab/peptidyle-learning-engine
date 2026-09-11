-- Current Student Course landing score disclosure.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091029 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

-- PostgreSQL cannot replace a function when its table return shape changes.
DROP FUNCTION ple_api.list_released_live_student_assignments(bigint);
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

-- ASVS 8.2.3, 8.3.1, and 14.2.6: the trusted procedure omits both score
-- fields until the active Attempt's pinned disclosure rule permits the pair.
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
    v_now timestamp with time zone := pg_catalog.clock_timestamp();
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
           aggregate_score.graded_question_count,
           CASE WHEN attempt.assignment_attempt_id IS NULL THEN assignment.question_count
                ELSE aggregate_score.question_count END,
           CASE WHEN aggregate_score.score_is_current
                     AND CASE revision.feedback_score
                         WHEN 'during_attempt' THEN true
                         WHEN 'after_submit' THEN submission.assignment_attempt_id IS NOT NULL
                         WHEN 'after_due' THEN CASE
                             WHEN attempt.delivery_assignment_title IS NOT NULL
                                 THEN attempt.delivery_due_at
                             ELSE revision.due_at END IS NOT NULL
                              AND v_now >= CASE
                                 WHEN attempt.delivery_assignment_title IS NOT NULL
                                     THEN attempt.delivery_due_at
                                 ELSE revision.due_at END
                         WHEN 'after_close' THEN revision.closes_at IS NOT NULL
                              AND v_now >= revision.closes_at
                         ELSE false
                     END
                THEN aggregate_score.points_earned ELSE NULL END,
           CASE WHEN aggregate_score.score_is_current
                     AND CASE revision.feedback_score
                         WHEN 'during_attempt' THEN true
                         WHEN 'after_submit' THEN submission.assignment_attempt_id IS NOT NULL
                         WHEN 'after_due' THEN CASE
                             WHEN attempt.delivery_assignment_title IS NOT NULL
                                 THEN attempt.delivery_due_at
                             ELSE revision.due_at END IS NOT NULL
                              AND v_now >= CASE
                                 WHEN attempt.delivery_assignment_title IS NOT NULL
                                     THEN attempt.delivery_due_at
                                 ELSE revision.due_at END
                         WHEN 'after_close' THEN revision.closes_at IS NOT NULL
                              AND v_now >= revision.closes_at
                         ELSE false
                     END
                THEN aggregate_score.points_possible ELSE NULL END
      FROM released_assignment AS assignment
      LEFT JOIN LATERAL (
          SELECT candidate.assignment_attempt_id, candidate.assignment_revision_id,
                 candidate.attempt_number, candidate.completed_at,
                 candidate.delivery_assignment_title, candidate.delivery_due_at
            FROM ple_private.assignment_attempt AS candidate
           WHERE candidate.student_record_id = v_student_record_id
             AND candidate.assignment_id = assignment.assignment_id
           ORDER BY candidate.started_at DESC,
                    candidate.assignment_attempt_id DESC
           LIMIT 1
      ) AS attempt ON true
      LEFT JOIN ple_data.assignment_revision AS revision
        ON revision.assignment_revision_id = attempt.assignment_revision_id
      LEFT JOIN ple_private.assignment_submission AS submission
        ON submission.assignment_attempt_id = attempt.assignment_attempt_id
      LEFT JOIN LATERAL (
          SELECT count(DISTINCT issued.issued_question_id)::bigint AS question_count,
                 count(*) FILTER (WHERE chain.result_count = 1)::bigint AS graded_question_count,
                 count(DISTINCT issued.issued_question_id) > 0
                   AND bool_and(
                       chain.question_attempt_count = 1
                       AND chain.question_submission_count = 1
                       AND chain.grading_count = 1
                       AND chain.result_count = 1
                       AND chain.grading_is_graded
                       AND chain.has_immutable_receipt
                   ) AS score_is_current,
                 COALESCE(sum(chain.points_earned), 0)::double precision AS points_earned,
                 COALESCE(sum(chain.points_possible), 0)::double precision AS points_possible
            FROM ple_private.issued_question AS issued
            LEFT JOIN LATERAL (
                SELECT count(DISTINCT question_attempt.question_attempt_id)::bigint
                           AS question_attempt_count,
                       count(DISTINCT question_submission.submission_id)::bigint
                           AS question_submission_count,
                       count(DISTINCT grading.question_submission_grading_id)::bigint
                           AS grading_count,
                       count(DISTINCT result.grading_result_id)::bigint AS result_count,
                       COALESCE(bool_and(grading.grading_state = 'graded'), false)
                           AS grading_is_graded,
                       COALESCE(bool_and(ple_api.has_automated_grading_receipt(
                           grading.question_submission_grading_id,
                           result.grading_result_id
                       )), false) AS has_immutable_receipt,
                       COALESCE(sum(result.points_earned), 0)::double precision AS points_earned,
                       COALESCE(sum(result.points_possible), 0)::double precision AS points_possible
                  FROM ple_private.question_attempt AS question_attempt
                  LEFT JOIN ple_private.question_submission AS question_submission
                    ON question_submission.question_attempt_id = question_attempt.question_attempt_id
                  LEFT JOIN ple_private.question_submission_grading AS grading
                    ON grading.submission_id = question_submission.submission_id
                  LEFT JOIN ple_private.grading_result AS result
                    ON result.question_submission_grading_id = grading.question_submission_grading_id
                   AND result.submission_id = question_submission.submission_id
                   AND result.question_attempt_id = question_attempt.question_attempt_id
                 WHERE question_attempt.issued_question_id = issued.issued_question_id
            ) AS chain ON true
           WHERE issued.assignment_attempt_id = attempt.assignment_attempt_id
      ) AS aggregate_score ON true
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
