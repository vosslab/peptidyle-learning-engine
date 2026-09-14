-- Gradebook aggregation and worker-facing API wrappers. Core evidence tables
-- and internal worker transitions are defined first by grading.sql.

-- Gradebook aggregation reads retained Student Work rather than mutable
-- Assignment configuration.  The course-facing API below supplies current
-- released Assignments and active Students; this helper selects one Student's
-- most recent Attempt for one Assignment and exposes only answer-free facts.
CREATE FUNCTION ple_private.read_latest_assignment_attempt_gradebook_evidence(
    p_student_record_id uuid,
    p_assignment_id uuid
) RETURNS TABLE (
    assignment_attempt_id uuid,
    assignment_attempt_completion text,
    expired_submitting boolean,
    points_earned double precision,
    points_possible double precision
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
    WITH latest_attempt AS (
        SELECT attempt.assignment_attempt_id, attempt.completed_at, attempt.expires_at,
               EXISTS (
                   SELECT 1 FROM ple_private.assignment_submission AS submission
                    WHERE submission.assignment_attempt_id = attempt.assignment_attempt_id
               ) AS is_submitted
          FROM ple_private.assignment_attempt AS attempt
         WHERE attempt.student_record_id = p_student_record_id
           AND attempt.assignment_id = p_assignment_id
         ORDER BY attempt.started_at DESC, attempt.assignment_attempt_id DESC
         LIMIT 1
    )
    SELECT latest_attempt.assignment_attempt_id,
           CASE WHEN latest_attempt.completed_at IS NULL THEN 'in_progress'
                ELSE 'completed' END,
           latest_attempt.completed_at IS NULL
               AND latest_attempt.expires_at IS NOT NULL
               AND latest_attempt.expires_at <= pg_catalog.statement_timestamp()
               AND NOT latest_attempt.is_submitted,
           CASE WHEN latest_attempt.is_submitted
                     AND count(*) FILTER (
                         WHERE result.grading_result_id IS NOT NULL
                            OR question_attempt.question_attempt_state = 'closed_at_deadline'
                     ) = count(issued.issued_question_id)
                THEN coalesce(sum(score.points_earned), 0)::double precision END,
           CASE WHEN latest_attempt.is_submitted
                     AND count(*) FILTER (
                         WHERE result.grading_result_id IS NOT NULL
                            OR question_attempt.question_attempt_state = 'closed_at_deadline'
                     ) = count(issued.issued_question_id)
                THEN coalesce(sum(score.points_possible), 0)::double precision END
      FROM latest_attempt
      LEFT JOIN ple_private.issued_question AS issued
        ON issued.assignment_attempt_id = latest_attempt.assignment_attempt_id
      LEFT JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.question_submission AS submission
        ON submission.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_private.grading_result AS result
        ON result.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_data.assignment_entry AS entry
        ON entry.assignment_entry_id = issued.assignment_entry_id
      LEFT JOIN LATERAL ple_private.score_recorded_credit(
          coalesce(result.normalized_credit, 0), issued.scoring_rule,
          CASE entry.entry_kind
              WHEN 'fixed_question' THEN entry.points_possible
              ELSE entry.points_per_item
          END
      ) AS score ON entry.assignment_entry_id IS NOT NULL
     GROUP BY latest_attempt.assignment_attempt_id, latest_attempt.completed_at,
              latest_attempt.expires_at, latest_attempt.is_submitted
$$;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.has_automated_grading_receipt(
    p_question_submission_grading_id uuid,
    p_grading_result_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_audit AS $$
    SELECT p_question_submission_grading_id IS NOT NULL
       AND p_grading_result_id IS NOT NULL
       AND EXISTS (
           SELECT 1 FROM ple_audit.automated_grading_receipt AS receipt
            WHERE receipt.question_submission_grading_id = p_question_submission_grading_id
              AND receipt.grading_result_id = p_grading_result_id
       )
$$;
-- An Instructor's course gradebook combines current released Assignment
-- aggregates with the chosen Attempt's immutable issue/submission/grading
-- evidence.  It contains neither responses nor Question content.
CREATE FUNCTION ple_api.read_course_gradebook(p_course_reference_number bigint)
RETURNS TABLE (
    course_reference_number bigint,
    roster_id text,
    assignment_reference_number bigint,
    assignment_attempt_completion text,
    expired_submitting boolean,
    points_earned double precision,
    points_possible double precision
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    WITH course AS (
        SELECT instance.course_id, instance.reference_number
          FROM ple_data.course_instance AS instance
         WHERE instance.reference_number = p_course_reference_number
           AND ple_api.current_session_account_is_course_instructor(instance.course_id)
    ), active_student AS (
        SELECT record.student_record_id, profile.roster_id
          FROM course
          JOIN ple_data.student_record AS record
            ON record.course_id = course.course_id
          JOIN ple_data.course_membership AS membership
            ON membership.course_id = course.course_id
           AND membership.student_record_id = record.student_record_id
           AND membership.account_id = record.student_account_id
           AND membership.role = 'student'
           AND ple_data.course_membership_is_active(membership.membership_id)
          JOIN ple_private.course_roster_profile AS profile
            ON profile.course_id = course.course_id
           AND profile.student_account_id = record.student_account_id
    ), released_assignment AS (
        SELECT assignment.assignment_id, assignment.reference_number
          FROM course
          JOIN ple_data.assignment AS assignment
            ON assignment.course_id = course.course_id
           AND assignment.assignment_status = 'released'
         GROUP BY assignment.assignment_id, assignment.reference_number
    ), gradebook AS (
        SELECT course.reference_number AS course_reference_number,
               student.roster_id,
               assignment.reference_number AS assignment_reference_number,
               evidence.assignment_attempt_completion,
               coalesce(evidence.expired_submitting, false) AS expired_submitting,
               evidence.points_earned,
               evidence.points_possible
          FROM course
          CROSS JOIN active_student AS student
          CROSS JOIN released_assignment AS assignment
          LEFT JOIN LATERAL ple_private.read_latest_assignment_attempt_gradebook_evidence(
              student.student_record_id, assignment.assignment_id
          ) AS evidence ON true
    )
    SELECT course_reference_number, roster_id, assignment_reference_number,
           assignment_attempt_completion, expired_submitting, points_earned, points_possible
      FROM gradebook
    UNION ALL
    SELECT course.reference_number, NULL::text, NULL::bigint, NULL::text, NULL::boolean,
           NULL::double precision, NULL::double precision
      FROM course
     WHERE NOT EXISTS (SELECT 1 FROM gradebook)
     ORDER BY roster_id NULLS FIRST, assignment_reference_number NULLS FIRST
$$;

REVOKE ALL ON FUNCTION ple_api.has_automated_grading_receipt(uuid, uuid),
    ple_api.read_course_gradebook(bigint)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.has_automated_grading_receipt(uuid, uuid)
    TO ple_private_owner;
GRANT EXECUTE ON FUNCTION ple_api.read_course_gradebook(bigint)
    TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
REVOKE ALL ON FUNCTION ple_private.reject_grading_evidence_change(),
    ple_private.complete_assignment_attempt_after_grading(),
    ple_private.record_direct_automated_grading_result(uuid, uuid, numeric, timestamptz),
    ple_private.read_latest_assignment_attempt_gradebook_evidence(uuid, uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.read_latest_assignment_attempt_gradebook_evidence(uuid, uuid)
    TO ple_api_owner;

COMMENT ON TABLE ple_private.grading_result IS
    'One immutable normalized-credit outcome for an accepted Submission; current Assignment Entry points calculate scores on read.';
SET LOCAL ROLE ple_audit_owner;
REVOKE ALL ON FUNCTION ple_audit.reject_automated_grading_receipt_change() FROM PUBLIC;
COMMENT ON TABLE ple_audit.automated_grading_receipt IS
    'Immutable receipt for one automated grading commit; deleted only with its exclusive Student Work root.';

RESET ROLE;
