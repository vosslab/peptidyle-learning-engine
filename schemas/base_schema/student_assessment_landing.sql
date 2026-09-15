-- Student assessment landing projection.  Current Assessment configuration
-- supplies only an unstarted Assessment's visible question count; an Assessment Attempt
-- is interpreted entirely through its retained Student Work evidence.

-- The private retained-Work projections cannot read Course rows directly.
-- Keep this state predicate in the API owner's narrow, unexposed capability
-- instead of widening their table privileges.  ASVS 2.3.1.
SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.course_student_work_is_ordinarily_visible(
    p_course_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT p_course_id IS NOT NULL
       AND EXISTS (
           SELECT 1
             FROM ple_data.course_instance AS course
            WHERE course.course_id = p_course_id
              AND course.retention_lifecycle_state = 'active'
       )
$$;
REVOKE ALL ON FUNCTION ple_api.course_student_work_is_ordinarily_visible(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.course_student_work_is_ordinarily_visible(uuid)
    TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

-- ASVS 2.2.2, 8.2.3, 14.2.6: derive the exact session Student's effective
-- values below the API owner and expose no Accommodation identity.  This read
-- uses the one p_now supplied from statement_timestamp(); the browser never
-- recomputes permission from its own clock.
CREATE FUNCTION ple_private.read_student_released_assessment_landing_evidence(
    p_course_id uuid,
    p_student_record_id uuid,
    p_now timestamptz
) RETURNS TABLE (
    assessment_reference_number text,
    assessment_title text,
    assessment_type text,
    start_decision text,
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    time_limit_seconds integer,
    assessment_attempt_limit integer,
    late_work_rule text,
    evaluated_at timestamptz,
    display_time_zone text,
    assessment_attempt_number integer,
    assessment_attempt_completion text,
    can_resume_assessment_attempt boolean,
    graded_question_count bigint,
    question_count bigint,
    assessment_score_points_earned double precision,
    assessment_score_points_possible double precision
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private, ple_audit AS $$
    -- ASVS 2.3.1: direct internal callers receive the same archive exclusion
    -- as the public Course lookup below; normal Student Work visibility is
    -- not conditional on which projection reaches this helper.
    WITH released_assessment AS (
        SELECT assessment.assessment_id,
               assessment.public_reference,
               assessment.assessment_title,
               assessment.assessment_type,
               assessment.assessment_status,
               assessment.available_at,
               assessment.due_at,
               assessment.closes_at,
               assessment.assessment_attempt_time_limit_seconds,
               assessment.assessment_attempt_limit,
               assessment.late_work_rule,
               coalesce(sum(CASE entry.entry_kind
                   WHEN 'fixed_question' THEN 1
                   ELSE entry.selection_count
               END) FILTER (WHERE entry.availability = 'available'), 0)::bigint
                   AS current_question_count
          FROM ple_data.assessment AS assessment
          LEFT JOIN ple_data.assessment_entry AS entry
            ON entry.assessment_id = assessment.assessment_id
         WHERE assessment.course_id = p_course_id
           AND assessment.assessment_status = 'released'
           AND ple_api.course_student_work_is_ordinarily_visible(assessment.course_id)
         GROUP BY assessment.assessment_id, assessment.public_reference,
                  assessment.assessment_title, assessment.assessment_type,
                  assessment.assessment_status,
                  assessment.available_at, assessment.due_at, assessment.closes_at,
                  assessment.assessment_attempt_time_limit_seconds,
                  assessment.assessment_attempt_limit, assessment.late_work_rule
    )
    SELECT assessment.public_reference,
           CASE WHEN assessment_attempt.assessment_attempt_id IS NULL
                THEN assessment.assessment_title
                ELSE assessment_attempt.assessment_title
           END,
           assessment.assessment_type,
           decision.start_decision,
           effective.available_at,
           effective.due_at,
           effective.closes_at,
           CASE WHEN assessment_attempt.assessment_attempt_id IS NOT NULL
                     AND assessment_attempt_submission.assessment_attempt_id IS NULL
                THEN assessment_attempt.assessment_attempt_time_limit_seconds
                ELSE effective.time_limit_seconds
           END,
           effective.assessment_attempt_limit,
           assessment.late_work_rule,
           p_now,
           (
               SELECT preference.time_zone
                 FROM ple_private.account_time_zone AS preference
                WHERE preference.account_id = ple_api.current_session_account_id()
           ),
           assessment_attempt.assessment_attempt_number,
           CASE
               WHEN assessment_attempt.assessment_attempt_id IS NULL THEN NULL
               WHEN assessment_attempt_submission.assessment_attempt_id IS NULL THEN 'in_progress'
               ELSE 'completed'
           END,
           coalesce(resumable.can_resume_assessment_attempt, false),
           coalesce(evidence.graded_question_count, 0)::bigint,
           CASE WHEN assessment_attempt.assessment_attempt_id IS NULL
                THEN assessment.current_question_count
                ELSE coalesce(evidence.question_count, 0)::bigint
           END,
           CASE WHEN grade_evidence.points_earned IS NOT NULL
                     AND CASE assessment_score_attempt.feedback_score
                         WHEN 'during_attempt' THEN true
                         WHEN 'after_submit' THEN assessment_score_submission.assessment_attempt_id IS NOT NULL
                         WHEN 'after_due' THEN assessment_score_attempt.due_at IS NOT NULL
                              AND p_now >= assessment_score_attempt.due_at
                         WHEN 'after_close' THEN assessment_score_attempt.closes_at IS NOT NULL
                              AND p_now >= assessment_score_attempt.closes_at
                         ELSE false
                     END
                THEN grade_evidence.points_earned ELSE NULL END,
           CASE WHEN grade_evidence.points_possible IS NOT NULL
                     AND CASE assessment_score_attempt.feedback_score
                         WHEN 'during_attempt' THEN true
                         WHEN 'after_submit' THEN assessment_score_submission.assessment_attempt_id IS NOT NULL
                         WHEN 'after_due' THEN assessment_score_attempt.due_at IS NOT NULL
                              AND p_now >= assessment_score_attempt.due_at
                         WHEN 'after_close' THEN assessment_score_attempt.closes_at IS NOT NULL
                              AND p_now >= assessment_score_attempt.closes_at
                         ELSE false
                     END
                THEN grade_evidence.points_possible ELSE NULL END
      FROM released_assessment AS assessment
      LEFT JOIN ple_private.student_assessment_accommodation AS accommodation
        ON accommodation.student_record_id = p_student_record_id
       AND accommodation.assessment_id = assessment.assessment_id
      LEFT JOIN LATERAL (
          SELECT coalesce(accommodation.available_at, assessment.available_at) AS available_at,
                 coalesce(accommodation.due_at, assessment.due_at) AS due_at,
                 coalesce(accommodation.closes_at, assessment.closes_at) AS closes_at,
                 coalesce(
                     accommodation.assessment_attempt_time_limit_seconds,
                     assessment.assessment_attempt_time_limit_seconds
                 ) AS time_limit_seconds,
                 CASE
                     WHEN assessment.assessment_type IN ('quiz', 'exam') THEN 1
                     ELSE coalesce(
                         accommodation.assessment_attempt_limit,
                         assessment.assessment_attempt_limit
                     )
                 END AS assessment_attempt_limit
      ) AS effective ON true
      LEFT JOIN LATERAL (
          SELECT candidate.assessment_attempt_id, candidate.assessment_title,
                 candidate.assessment_attempt_number,
                 candidate.feedback_score,
                 candidate.assessment_attempt_time_limit_seconds,
                 candidate.due_at, candidate.closes_at
            FROM ple_private.assessment_attempt AS candidate
           WHERE candidate.student_record_id = p_student_record_id
             AND candidate.assessment_id = assessment.assessment_id
           ORDER BY candidate.started_at DESC, candidate.assessment_attempt_id DESC
           LIMIT 1
      ) AS assessment_attempt ON true
      LEFT JOIN ple_private.assessment_submission AS assessment_attempt_submission
        ON assessment_attempt_submission.assessment_attempt_id = assessment_attempt.assessment_attempt_id
      LEFT JOIN LATERAL (
          SELECT count(*)::integer AS started_assessment_attempt_count
            FROM ple_private.assessment_attempt AS started_assessment_attempt
           WHERE started_assessment_attempt.student_record_id = p_student_record_id
             AND started_assessment_attempt.assessment_id = assessment.assessment_id
      ) AS started ON true
      LEFT JOIN LATERAL (
          SELECT ple_private.assessment_start_decision(
              assessment.assessment_status,
              effective.available_at,
              effective.due_at,
              effective.closes_at,
              effective.assessment_attempt_limit,
              started.started_assessment_attempt_count,
              assessment.late_work_rule,
              p_now
          ) AS start_decision
      ) AS decision ON true
      LEFT JOIN LATERAL (
          SELECT candidate.assessment_attempt_resume_rule = 'resumable'
                     AS can_resume_assessment_attempt
            FROM ple_private.assessment_attempt AS candidate
           WHERE candidate.student_record_id = p_student_record_id
             AND candidate.assessment_id = assessment.assessment_id
             AND NOT EXISTS (
                 SELECT 1
                   FROM ple_private.assessment_submission AS candidate_submission
                  WHERE candidate_submission.assessment_attempt_id = candidate.assessment_attempt_id
             )
             AND (candidate.expires_at IS NULL OR candidate.expires_at > p_now)
           ORDER BY candidate.assessment_attempt_number DESC
           LIMIT 1
      ) AS resumable
        ON decision.start_decision NOT IN ('closed', 'not_yet_available')
      -- The accepted Gradebook selector owns highest-submitted-Attempt choice
      -- and grade-contribution treatment. Progress above deliberately remains
      -- attached to the latest Attempt.
      LEFT JOIN LATERAL ple_private.read_assessment_gradebook_evidence(
          p_student_record_id, assessment.assessment_id
      ) AS grade_evidence ON true
      LEFT JOIN ple_private.assessment_attempt AS assessment_score_attempt
        ON assessment_score_attempt.assessment_attempt_id = grade_evidence.assessment_attempt_id
      LEFT JOIN ple_private.assessment_submission AS assessment_score_submission
        ON assessment_score_submission.assessment_attempt_id = assessment_score_attempt.assessment_attempt_id
      LEFT JOIN LATERAL (
          SELECT count(issued.issued_question_id)::bigint AS question_count,
                 count(*) FILTER (WHERE chain.is_complete)::bigint AS graded_question_count
            FROM ple_private.issued_question AS issued
            LEFT JOIN LATERAL (
                SELECT count(DISTINCT question_attempt.question_attempt_id) = 1
                           AND (
                               coalesce(bool_and(
                                   question_attempt.question_attempt_state = 'closed_unanswered'
                               ), false)
                               OR (
                                   count(DISTINCT question_submission.submission_id) = 1
                                   AND count(DISTINCT grading.question_submission_grading_id) = 1
                                   AND count(DISTINCT result.grading_result_id) = 1
                                   AND count(DISTINCT receipt.automated_grading_receipt_id) = 1
                                   AND coalesce(bool_and(grading.grading_state = 'graded'), false)
                               )
                           ) AS is_complete
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
           WHERE issued.assessment_attempt_id = assessment_attempt.assessment_attempt_id
      ) AS evidence ON true
     ORDER BY assessment.assessment_title, assessment.public_reference
$$;

REVOKE ALL ON FUNCTION ple_private.read_student_released_assessment_landing_evidence(
    uuid, uuid, timestamptz
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.read_student_released_assessment_landing_evidence(
    uuid, uuid, timestamptz
) TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

-- The public projection establishes exact active Student membership before it
-- delegates evidence reads.  A failed lookup is indistinguishable from a
-- foreign Course.
CREATE FUNCTION ple_api.list_released_live_student_assessments(
    p_course_public_reference text
) RETURNS TABLE (
    assessment_reference_number text,
    assessment_title text,
    assessment_type text,
    start_decision text,
    available_at timestamptz,
    due_at timestamptz,
    closes_at timestamptz,
    time_limit_seconds integer,
    assessment_attempt_limit integer,
    late_work_rule text,
    evaluated_at timestamptz,
    display_time_zone text,
    assessment_attempt_number integer,
    assessment_attempt_completion text,
    can_resume_assessment_attempt boolean,
    graded_question_count bigint,
    question_count bigint,
    assessment_score_points_earned double precision,
    assessment_score_points_possible double precision
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    course_id_value uuid;
    student_record_id_value uuid;
    evaluation_time timestamptz := pg_catalog.statement_timestamp();
BEGIN
    IF p_course_public_reference IS NULL THEN
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
       AND course.retention_lifecycle_state = 'active'
      JOIN ple_data.student_record AS student
        ON student.student_record_id = membership.student_record_id
       AND student.course_id = course.course_id
       AND student.student_account_id = account.account_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'student'
       AND course.public_reference = p_course_public_reference;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course is unavailable';
    END IF;

    RETURN QUERY
    SELECT * FROM ple_private.read_student_released_assessment_landing_evidence(
        course_id_value, student_record_id_value, evaluation_time
    );
END
$$;

REVOKE ALL ON FUNCTION ple_api.list_released_live_student_assessments(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_released_live_student_assessments(text) TO ple_app;

RESET ROLE;
