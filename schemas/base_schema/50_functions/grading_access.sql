-- Functions, triggers, and views from grading_access.sql.

-- Gradebook aggregation and worker-facing API wrappers. Core evidence tables
-- and internal worker transitions are defined first by grading.sql.

-- Gradebook aggregation reads retained Student Work rather than mutable
-- Assessment configuration.  The course-facing API below supplies current
-- released Assessments and active Students. This helper selects one Student's
-- highest grading-complete submitted Assessment Attempt for one Assessment and
-- exposes only answer-free facts. When no score is established, the latest
-- Attempt continues to expose current progress or expiry instead.
-- ASVS 1.2.4, 8.2.1-8.2.3, 8.3.1, 14.2.6: fixed parameters and a private
-- execution grant preserve the authorized, minimum-field Gradebook boundary.
CREATE FUNCTION ple_private.read_assessment_gradebook_evidence(
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
    WITH assessment_attempts AS (
        SELECT assessment_attempt.assessment_attempt_id, assessment_attempt.started_at,
               assessment_attempt.expires_at, assessment.assessment_type,
               EXISTS (
                   SELECT 1 FROM ple_private.assessment_submission AS submission
                    WHERE submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id
               ) AS is_submitted
          FROM ple_private.assessment_attempt AS assessment_attempt
          JOIN ple_data.assessment AS assessment
            ON assessment.assessment_id = assessment_attempt.assessment_id
         WHERE assessment_attempt.student_record_id = p_student_record_id
           AND assessment_attempt.assessment_id = p_assessment_id
    ), assessment_attempt_scores AS (
        SELECT assessment_attempt.assessment_attempt_id, assessment_attempt.started_at,
               assessment_attempt.expires_at,
               assessment_attempt.is_submitted,
               CASE WHEN assessment_attempt.is_submitted
                     AND count(issued.issued_question_id) > 0
                     AND count(*) FILTER (
                         WHERE result.grading_result_id IS NOT NULL
                            OR question_attempt.question_attempt_state = 'closed_unanswered'
                     ) = count(issued.issued_question_id)
                THEN coalesce(sum(score.points_earned), 0)
                END AS points_earned,
               CASE WHEN assessment_attempt.is_submitted
                     AND count(issued.issued_question_id) > 0
                     AND count(*) FILTER (
                         WHERE result.grading_result_id IS NOT NULL
                            OR question_attempt.question_attempt_state = 'closed_unanswered'
                     ) = count(issued.issued_question_id)
                THEN coalesce(sum(ple_private.grade_contribution_points_possible(
                    assessment_attempt.assessment_type,
                    issued.scoring_rule,
                    score.points_possible
                )), 0)
                END AS points_possible
          FROM assessment_attempts AS assessment_attempt
          LEFT JOIN ple_private.issued_question AS issued
            ON issued.assessment_attempt_id = assessment_attempt.assessment_attempt_id
          LEFT JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.issued_question_id = issued.issued_question_id
          LEFT JOIN ple_private.grading_result AS result
            ON result.question_attempt_id = question_attempt.question_attempt_id
          LEFT JOIN ple_data.assessment_entry AS entry
            ON entry.assessment_entry_id = issued.assessment_entry_id
          LEFT JOIN LATERAL ple_private.score_recorded_credit(
              result.normalized_credit, issued.scoring_rule,
              CASE entry.entry_kind
                  WHEN 'fixed_question' THEN entry.points_possible
                  ELSE entry.points_per_item
              END
          ) AS score ON entry.assessment_entry_id IS NOT NULL
         GROUP BY assessment_attempt.assessment_attempt_id, assessment_attempt.started_at,
                  assessment_attempt.expires_at,
                  assessment_attempt.assessment_type, assessment_attempt.is_submitted
    ), chosen_assessment_attempt AS (
        SELECT assessment_attempt_id, expires_at, is_submitted,
               points_earned, points_possible
          FROM assessment_attempt_scores
         ORDER BY (points_earned IS NOT NULL) DESC, points_earned DESC NULLS LAST,
                  started_at DESC, assessment_attempt_id DESC
         LIMIT 1
    )
    SELECT chosen.assessment_attempt_id,
           CASE WHEN chosen.is_submitted THEN 'completed'
                ELSE 'in_progress' END,
           NOT chosen.is_submitted AND chosen.expires_at IS NOT NULL
               AND chosen.expires_at <= pg_catalog.statement_timestamp()
               AND NOT chosen.is_submitted,
           chosen.points_earned::double precision,
           chosen.points_possible::double precision
      FROM chosen_assessment_attempt AS chosen
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.has_automated_grading_receipt(
    p_question_response_grading_id uuid,
    p_grading_result_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_audit AS $$
    SELECT p_question_response_grading_id IS NOT NULL
       AND p_grading_result_id IS NOT NULL
       AND EXISTS (
           SELECT 1 FROM ple_audit.automated_grading_receipt AS receipt
            WHERE receipt.question_response_grading_id = p_question_response_grading_id
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
    roster_name text,
    assessment_reference_number text,
    assessment_title text,
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
           AND instance.retention_lifecycle_state = 'active'
           AND ple_api.current_session_account_is_course_instructor(instance.course_id)
    ), active_student AS (
        SELECT record.student_record_id, profile.roster_id, profile.roster_name
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
        -- ASVS 8.2.2/8.2.3: titles stay in this authorized Course's released set.
        SELECT assessment.assessment_id, assessment.public_reference, assessment.assessment_title
          FROM course
          JOIN ple_data.assessment AS assessment
            ON assessment.course_id = course.course_id
           AND assessment.assessment_status = 'released'
         GROUP BY assessment.assessment_id, assessment.public_reference, assessment.assessment_title
    ), gradebook AS (
        SELECT course.public_reference AS course_public_reference,
               student.roster_id,
               student.roster_name,
               assessment.public_reference AS assessment_reference_number,
               assessment.assessment_title,
               evidence.assessment_attempt_completion,
               coalesce(evidence.expired_submitting, false) AS expired_submitting,
               evidence.points_earned,
               evidence.points_possible
          FROM course
          CROSS JOIN active_student AS student
          CROSS JOIN released_assessment AS assessment
          LEFT JOIN LATERAL ple_private.read_assessment_gradebook_evidence(
              student.student_record_id, assessment.assessment_id
          ) AS evidence ON true
    )
    SELECT course_public_reference, roster_id, roster_name, assessment_reference_number, assessment_title,
           assessment_attempt_completion, expired_submitting, points_earned, points_possible
      FROM gradebook
    UNION ALL
    SELECT course.public_reference, NULL::text, NULL::text, NULL::text, NULL::text, NULL::text, NULL::boolean,
           NULL::double precision, NULL::double precision
      FROM course
     WHERE NOT EXISTS (SELECT 1 FROM gradebook)
     ORDER BY roster_id NULLS FIRST, assessment_reference_number NULLS FIRST
$$;

