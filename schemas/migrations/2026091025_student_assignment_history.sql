-- Student Assignment Access facts and owned completed-Attempt history.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091025 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

-- PostgreSQL cannot replace a function when its table return shape changes.
DROP FUNCTION ple_api.live_demo_assignment_access(bigint, bigint);

-- The Student-facing access reader needs only this receipt-existence proof.
-- Keep the audit relation behind its existing API-owner policy and expose no
-- receipt identity, checksum, or timestamp to the private delivery boundary.
CREATE FUNCTION ple_api.has_automated_grading_receipt(
    p_question_submission_grading_id uuid, p_grading_result_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_audit AS $$
    SELECT p_question_submission_grading_id IS NOT NULL
       AND p_grading_result_id IS NOT NULL
       AND EXISTS (
           SELECT 1
             FROM ple_audit.automated_grading_receipt AS receipt
            WHERE receipt.question_submission_grading_id = p_question_submission_grading_id
              AND receipt.grading_result_id = p_grading_result_id
       )
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.has_automated_grading_receipt(uuid, uuid)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.has_automated_grading_receipt(uuid, uuid)
TO ple_private_owner;

-- The access and history readers traverse Student Work that is deliberately
-- forced-RLS to the private owner.  Preserve ple_app's procedure-only
-- authority rather than granting it, or the API owner, direct table reads.
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_api.live_demo_assignment_access(
    p_course_reference_number bigint, p_assignment_reference_number bigint
) RETURNS TABLE (
    start_decision text, assignment_title text, question_count integer,
    points_possible double precision, assignment_attempt_time_limit_seconds integer,
    previous_attempts jsonb
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_assignment ple_data.assignment%ROWTYPE;
    v_student_record_id uuid;
    v_revision ple_data.assignment_revision%ROWTYPE;
    v_active_attempt ple_private.assignment_attempt%ROWTYPE;
    v_started_attempt_count integer;
    v_now timestamp with time zone := pg_catalog.clock_timestamp();
    v_due_at timestamp with time zone;
BEGIN
    SELECT assignment.* INTO v_assignment
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    SELECT student_record.student_record_id INTO v_student_record_id
      FROM ple_data.student_record AS student_record
     WHERE student_record.course_id = v_assignment.course_id
       AND student_record.student_account_id = ple_api.current_session_account_id();
    IF NOT FOUND
       OR NOT ple_api.current_session_account_owns_student_record(
           v_assignment.course_id, v_student_record_id
       )
       OR NOT EXISTS (
           SELECT 1 FROM ple_data.course_membership AS membership
            WHERE membership.course_id = v_assignment.course_id
              AND membership.account_id = ple_api.current_session_account_id()
              AND membership.role = 'student'
              AND ple_data.course_membership_is_active(membership.membership_id)
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;

    SELECT attempt.* INTO v_active_attempt
      FROM ple_private.assignment_attempt AS attempt
     WHERE attempt.student_record_id = v_student_record_id
       AND attempt.assignment_id = v_assignment.assignment_id
       AND attempt.completed_at IS NULL
     ORDER BY attempt.attempt_number DESC
     LIMIT 1;
    SELECT revision.* INTO v_revision
      FROM ple_data.assignment_revision AS revision
     WHERE revision.assignment_revision_id = CASE
         WHEN v_active_attempt.assignment_attempt_id IS NULL
             THEN v_assignment.released_assignment_revision_id
         ELSE v_active_attempt.assignment_revision_id
     END;
    IF v_assignment.assignment_status <> 'released' OR NOT FOUND THEN
        start_decision := 'closed';
    ELSIF v_revision.available_at IS NOT NULL AND v_now < v_revision.available_at THEN
        start_decision := 'not_yet_available';
    ELSIF v_revision.closes_at IS NOT NULL AND v_now >= v_revision.closes_at THEN
        start_decision := 'closed';
    ELSE
        v_due_at := CASE
            WHEN v_active_attempt.assignment_attempt_id IS NULL THEN v_assignment.due_at
            WHEN v_active_attempt.delivery_assignment_title IS NOT NULL
                THEN v_active_attempt.delivery_due_at
            ELSE v_revision.due_at
        END;
        IF v_due_at IS NOT NULL AND v_now > v_due_at
           AND v_revision.late_work_rule = 'reject' THEN
            start_decision := 'late_work_refused';
        ELSIF v_active_attempt.assignment_attempt_id IS NOT NULL THEN
            start_decision := 'may_start';
        ELSE
            SELECT count(*)::integer INTO v_started_attempt_count
              FROM ple_private.assignment_attempt AS attempt
             WHERE attempt.student_record_id = v_student_record_id
               AND attempt.assignment_id = v_assignment.assignment_id;
            start_decision := CASE
                WHEN v_revision.attempt_limit IS NOT NULL
                 AND v_started_attempt_count >= v_revision.attempt_limit
                    THEN 'attempt_limit_reached'
                ELSE 'may_start'
            END;
        END IF;
    END IF;

    assignment_title := CASE
        WHEN v_active_attempt.assignment_attempt_id IS NULL THEN v_assignment.assignment_title
        WHEN v_active_attempt.delivery_assignment_title IS NOT NULL
            THEN v_active_attempt.delivery_assignment_title
        ELSE v_revision.assignment_title
    END;
    assignment_attempt_time_limit_seconds := v_revision.assignment_attempt_time_limit_seconds;
    SELECT count(*)::integer, COALESCE(sum(issued.point_value), 0)::double precision
      INTO question_count, points_possible
      FROM ple_private.issued_question AS issued
     WHERE issued.assignment_attempt_id = v_active_attempt.assignment_attempt_id;
    IF v_active_attempt.assignment_attempt_id IS NULL THEN
        SELECT COALESCE(sum(CASE entry.entry_kind
                    WHEN 'fixed_question' THEN 1
                    ELSE question_pool.selection_count END), 0)::integer,
               COALESCE(sum(entry.point_value * CASE entry.entry_kind
                    WHEN 'fixed_question' THEN 1
                    ELSE question_pool.selection_count END), 0)::double precision
          INTO question_count, points_possible
          FROM ple_data.assignment_revision_entry AS entry
          LEFT JOIN ple_data.assignment_revision_question_pool AS question_pool
            ON question_pool.assignment_revision_id = entry.assignment_revision_id
           AND question_pool.assignment_entry_id = entry.assignment_entry_id
         WHERE entry.assignment_revision_id = v_revision.assignment_revision_id;
    END IF;
    SELECT COALESCE(jsonb_agg(history.value ORDER BY history.attempt_number DESC,
                               history.reference_number DESC), '[]'::jsonb)
      INTO previous_attempts
      FROM (
          SELECT attempt.reference_number, attempt.attempt_number,
                 jsonb_build_object(
                     'assignmentAttempt', 'R-' || attempt.reference_number,
                     'attemptNumber', attempt.attempt_number,
                     'state', CASE WHEN submission.assignment_attempt_id IS NOT NULL
                         THEN 'submitted' ELSE 'closed' END
                 ) || CASE
                    WHEN complete_score.score_is_current
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
                     END THEN jsonb_build_object('score', jsonb_build_object(
                        'pointsEarned', complete_score.points_earned,
                        'pointsPossible', complete_score.points_possible
                     )) ELSE '{}'::jsonb END AS value
            FROM ple_private.assignment_attempt AS attempt
            JOIN ple_data.assignment_revision AS revision
              ON revision.assignment_revision_id = attempt.assignment_revision_id
            LEFT JOIN ple_private.assignment_submission AS submission
              ON submission.assignment_attempt_id = attempt.assignment_attempt_id
            CROSS JOIN LATERAL (
                SELECT count(*) > 0
                       AND count(result.grading_result_id) = count(*)
                       AND bool_and(grading.grading_state = 'graded')
                       AND count(*) FILTER (WHERE ple_api.has_automated_grading_receipt(
                               grading.question_submission_grading_id,
                               result.grading_result_id
                           )) = count(*)
                       AS score_is_current,
                       COALESCE(sum(result.points_earned), 0)::double precision AS points_earned,
                       COALESCE(sum(result.points_possible), 0)::double precision AS points_possible
                  FROM ple_private.issued_question AS issued
                  JOIN ple_private.question_attempt AS question_attempt
                    ON question_attempt.issued_question_id = issued.issued_question_id
                  LEFT JOIN ple_private.question_submission AS question_submission
                    ON question_submission.question_attempt_id = question_attempt.question_attempt_id
                  LEFT JOIN ple_private.question_submission_grading AS grading
                    ON grading.submission_id = question_submission.submission_id
                  LEFT JOIN ple_private.grading_result AS result
                    ON result.question_submission_grading_id = grading.question_submission_grading_id
                   AND result.submission_id = question_submission.submission_id
                   AND result.question_attempt_id = question_attempt.question_attempt_id
                 WHERE issued.assignment_attempt_id = attempt.assignment_attempt_id
            ) AS complete_score
           WHERE attempt.student_record_id = v_student_record_id
             AND attempt.assignment_id = v_assignment.assignment_id
             AND attempt.completed_at IS NOT NULL
      ) AS history;
    RETURN NEXT;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.live_demo_assignment_access(bigint, bigint)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.live_demo_assignment_access(bigint, bigint) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.live_demo_assignment_access(bigint, bigint)
TO ple_private_owner;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
