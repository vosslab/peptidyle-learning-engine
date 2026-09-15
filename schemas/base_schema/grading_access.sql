-- Gradebook aggregation and worker-facing API wrappers. Core evidence tables
-- and internal worker transitions are defined first by grading.sql.

-- Gradebook aggregation reads retained Student Work rather than mutable
-- Assessment configuration.  The course-facing API below supplies current
-- released Assessments and active Students; this helper selects one Student's
-- most recent Assessment Attempt for one Assessment and exposes only answer-free facts.
CREATE FUNCTION ple_private.read_latest_assessment_attempt_gradebook_evidence(
    p_student_record_id uuid,
    p_assessment_id uuid
) RETURNS TABLE (
    assessment_attempt_id uuid,
    assessment_attempt_completion text,
    expired_submitting boolean,
    points_earned double precision,
    points_possible double precision
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
    WITH latest_assessment_attempt AS (
        SELECT assessment_attempt.assessment_attempt_id, assessment_attempt.completed_at, assessment_attempt.expires_at,
               EXISTS (
                   SELECT 1 FROM ple_private.assessment_submission AS submission
                    WHERE submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id
               ) AS is_submitted
          FROM ple_private.assessment_attempt AS assessment_attempt
         WHERE assessment_attempt.student_record_id = p_student_record_id
           AND assessment_attempt.assessment_id = p_assessment_id
         ORDER BY assessment_attempt.started_at DESC, assessment_attempt.assessment_attempt_id DESC
         LIMIT 1
    )
    SELECT latest_assessment_attempt.assessment_attempt_id,
           CASE WHEN latest_assessment_attempt.completed_at IS NULL THEN 'in_progress'
                ELSE 'completed' END,
           latest_assessment_attempt.completed_at IS NULL
               AND latest_assessment_attempt.expires_at IS NOT NULL
               AND latest_assessment_attempt.expires_at <= pg_catalog.statement_timestamp()
               AND NOT latest_assessment_attempt.is_submitted,
           CASE WHEN latest_assessment_attempt.is_submitted
                     AND count(*) FILTER (
                         WHERE result.grading_result_id IS NOT NULL
                            OR question_attempt.question_attempt_state = 'closed_at_deadline'
                     ) = count(issued.issued_question_id)
                THEN coalesce(sum(score.points_earned), 0)::double precision END,
           CASE WHEN latest_assessment_attempt.is_submitted
                     AND count(*) FILTER (
                         WHERE result.grading_result_id IS NOT NULL
                            OR question_attempt.question_attempt_state = 'closed_at_deadline'
                     ) = count(issued.issued_question_id)
                THEN coalesce(sum(score.points_possible), 0)::double precision END
      FROM latest_assessment_attempt
      LEFT JOIN ple_private.issued_question AS issued
        ON issued.assessment_attempt_id = latest_assessment_attempt.assessment_attempt_id
      LEFT JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.question_submission AS submission
        ON submission.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_private.grading_result AS result
        ON result.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_data.assessment_entry AS entry
        ON entry.assessment_entry_id = issued.assessment_entry_id
      LEFT JOIN LATERAL ple_private.score_recorded_credit(
          coalesce(result.normalized_credit, 0), issued.scoring_rule,
          CASE entry.entry_kind
              WHEN 'fixed_question' THEN entry.points_possible
              ELSE entry.points_per_item
          END
      ) AS score ON entry.assessment_entry_id IS NOT NULL
     GROUP BY latest_assessment_attempt.assessment_attempt_id, latest_assessment_attempt.completed_at,
              latest_assessment_attempt.expires_at, latest_assessment_attempt.is_submitted
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
-- An Instructor's course gradebook combines current released Assessment
-- aggregates with the chosen Assessment Attempt's immutable issue/submission/grading
-- evidence.  It contains neither responses nor Question content.
CREATE FUNCTION ple_api.read_course_gradebook(p_course_public_reference text)
RETURNS TABLE (
    course_public_reference text,
    roster_id text,
    assessment_reference_number text,
    assessment_attempt_completion text,
    expired_submitting boolean,
    points_earned double precision,
    points_possible double precision
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    WITH course AS (
        SELECT instance.course_id, instance.public_reference
          FROM ple_data.course_instance AS instance
         WHERE instance.public_reference = p_course_public_reference
           -- Ordinary Instructor gradebook reads end with normal Student
           -- Work visibility. Retention's protected pre-delete capability is
           -- intentionally separate and does not reuse this projection.
           -- ASVS 2.3.1: the retention lifecycle gates the same aggregate
           -- evidence boundary that it gates ordinary Student reads.
           AND instance.retention_lifecycle_state IN ('not_started', 'active')
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
    ), released_assessment AS (
        SELECT assessment.assessment_id, assessment.public_reference
          FROM course
          JOIN ple_data.assessment AS assessment
            ON assessment.course_id = course.course_id
           AND assessment.assessment_status = 'released'
         GROUP BY assessment.assessment_id, assessment.public_reference
    ), gradebook AS (
        SELECT course.public_reference AS course_public_reference,
               student.roster_id,
               assessment.public_reference AS assessment_reference_number,
               evidence.assessment_attempt_completion,
               coalesce(evidence.expired_submitting, false) AS expired_submitting,
               evidence.points_earned,
               evidence.points_possible
          FROM course
          CROSS JOIN active_student AS student
          CROSS JOIN released_assessment AS assessment
          LEFT JOIN LATERAL ple_private.read_latest_assessment_attempt_gradebook_evidence(
              student.student_record_id, assessment.assessment_id
          ) AS evidence ON true
    )
    SELECT course_public_reference, roster_id, assessment_reference_number,
           assessment_attempt_completion, expired_submitting, points_earned, points_possible
      FROM gradebook
    UNION ALL
    SELECT course.public_reference, NULL::text, NULL::text, NULL::text, NULL::boolean,
           NULL::double precision, NULL::double precision
      FROM course
     WHERE NOT EXISTS (SELECT 1 FROM gradebook)
     ORDER BY roster_id NULLS FIRST, assessment_reference_number NULLS FIRST
$$;

REVOKE ALL ON FUNCTION ple_api.has_automated_grading_receipt(uuid, uuid),
    ple_api.read_course_gradebook(text)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.has_automated_grading_receipt(uuid, uuid)
    TO ple_private_owner;
GRANT EXECUTE ON FUNCTION ple_api.read_course_gradebook(text)
    TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
REVOKE ALL ON FUNCTION ple_private.reject_grading_evidence_change(),
    ple_private.complete_assessment_attempt_after_grading(),
    ple_private.record_direct_automated_grading_result(uuid, uuid, numeric, timestamptz),
    ple_private.read_latest_assessment_attempt_gradebook_evidence(uuid, uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.read_latest_assessment_attempt_gradebook_evidence(uuid, uuid)
    TO ple_api_owner;

COMMENT ON TABLE ple_private.grading_result IS
    'One immutable normalized-credit outcome for an accepted Submission; current Assessment Entry points calculate scores on read.';
SET LOCAL ROLE ple_audit_owner;
REVOKE ALL ON FUNCTION ple_audit.reject_automated_grading_receipt_change() FROM PUBLIC;
COMMENT ON TABLE ple_audit.automated_grading_receipt IS
    'Immutable receipt for one automated grading commit; deleted only with its exclusive Student Work root.';

RESET ROLE;
