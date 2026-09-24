-- Student Course-wide, self-only Assessment Attempt history.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.read_student_course_attempt_history(
    p_course_instance_id text,
    p_student_record_id uuid,
    p_after text,
    p_page_size integer
) RETURNS TABLE (
    continuation_key text,
    assessment_attempt_id uuid,
    assessment_id text,
    assessment_title text,
    assessment_attempt_number integer,
    started_at_millis bigint,
    submitted_at_millis bigint,
    assessment_score_points_earned double precision,
    assessment_score_points_possible double precision
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_after_started_at timestamptz;
    v_after_attempt_id uuid;
    v_now timestamptz := pg_catalog.statement_timestamp();
BEGIN
    IF p_course_instance_id IS NULL OR p_student_record_id IS NULL THEN
        RAISE EXCEPTION 'Student Course not found' USING ERRCODE = '42501';
    END IF;
    IF p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 100 THEN
        RAISE EXCEPTION 'invalid Student Attempt History page' USING ERRCODE = '22023';
    END IF;
    IF p_after IS NOT NULL THEN
        IF p_after !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]{6}Z\|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' THEN
            RAISE EXCEPTION 'invalid Student Attempt History page' USING ERRCODE = '22023';
        END IF;
        BEGIN
            v_after_started_at := split_part(p_after, '|', 1)::timestamptz;
            v_after_attempt_id := split_part(p_after, '|', 2)::uuid;
        EXCEPTION WHEN OTHERS THEN
            RAISE EXCEPTION 'invalid Student Attempt History page' USING ERRCODE = '22023';
        END;
    END IF;

    RETURN QUERY
    WITH page_attempts AS MATERIALIZED (
        SELECT attempt.course_instance_id,
               attempt.assessment_attempt_id,
               assessment.assessment_id,
               policy.assessment_title,
               attempt.assessment_attempt_number,
               attempt.started_at,
               submission.submitted_at,
               policy.feedback_score,
               policy.due_at,
               policy.closes_at
          FROM ple_private.assessment_attempt AS attempt
          JOIN ple_data.assessment AS assessment
            ON assessment.course_instance_id = attempt.course_instance_id
           AND assessment.assessment_id = attempt.assessment_id
          JOIN ple_data.assessment_policy_snapshot AS policy
            ON policy.assessment_policy_snapshot_id = attempt.assessment_policy_snapshot_id
          LEFT JOIN ple_private.assessment_submission AS submission
            ON submission.course_instance_id = attempt.course_instance_id
           AND submission.assessment_attempt_id = attempt.assessment_attempt_id
         WHERE attempt.course_instance_id = p_course_instance_id
           AND attempt.student_record_id = p_student_record_id
           AND ple_api.course_student_work_is_ordinarily_visible(attempt.course_instance_id)
           AND (p_after IS NULL OR attempt.started_at < v_after_started_at
                OR (attempt.started_at = v_after_started_at
                    AND attempt.assessment_attempt_id < v_after_attempt_id))
         ORDER BY attempt.started_at DESC, attempt.assessment_attempt_id DESC
         LIMIT p_page_size + 1
    ), attempt_evidence AS (
        SELECT page.assessment_attempt_id,
               page.assessment_id,
               page.assessment_title,
               page.assessment_attempt_number,
               page.started_at,
               page.submitted_at,
               page.feedback_score,
               page.due_at,
               page.closes_at,
               score.is_complete,
               score.points_earned,
               score.points_possible
          FROM page_attempts AS page
          CROSS JOIN LATERAL (
              SELECT count(issued.issued_question_id) > 0
                         AND bool_and(
                             saved.question_attempt_id IS NULL
                             OR (result.grading_result_id IS NOT NULL
                                 AND ple_api.has_automated_grading_receipt(
                                     result.grading_result_id
                                 ))
                         ) AS is_complete,
                     coalesce(sum(credit.points_earned), 0) AS points_earned,
                     coalesce(sum(credit.points_possible), 0) AS points_possible
                FROM ple_private.issued_question AS issued
                JOIN ple_private.question_attempt AS question_attempt
                  ON question_attempt.course_instance_id = issued.course_instance_id
                 AND question_attempt.issued_question_id = issued.issued_question_id
                LEFT JOIN ple_private.assessment_attempt_saved_response AS saved
                  ON saved.course_instance_id = question_attempt.course_instance_id
                 AND saved.question_attempt_id = question_attempt.question_attempt_id
                LEFT JOIN ple_private.grading_result AS result
                  ON result.course_instance_id = question_attempt.course_instance_id
                 AND result.question_attempt_id = question_attempt.question_attempt_id
                JOIN ple_private.assessment_entry_snapshot AS snapshot
                  ON snapshot.assessment_entry_snapshot_id = issued.assessment_entry_snapshot_id
                CROSS JOIN LATERAL ple_private.score_recorded_credit(
                    result.normalized_credit, snapshot.scoring_rule,
                    coalesce(
                        (SELECT question.points_possible
                           FROM ple_data.assessment_entry_question AS question
                          WHERE question.assessment_entry_id = issued.assessment_entry_id),
                        (SELECT pool.points_per_item
                           FROM ple_data.assessment_entry_pool AS pool
                          WHERE pool.assessment_entry_id = issued.assessment_entry_id),
                        snapshot.points
                    )
                ) AS credit
               WHERE issued.course_instance_id = page.course_instance_id
                 AND issued.assessment_attempt_id = page.assessment_attempt_id
          ) AS score
    )
    SELECT to_char(evidence.started_at AT TIME ZONE 'UTC',
                   'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') || '|' ||
               evidence.assessment_attempt_id::text,
           evidence.assessment_attempt_id,
           evidence.assessment_id::text,
           evidence.assessment_title,
           evidence.assessment_attempt_number,
           floor(extract(epoch FROM evidence.started_at) * 1000)::bigint,
           CASE WHEN evidence.submitted_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM evidence.submitted_at) * 1000)::bigint END,
           CASE WHEN evidence.submitted_at IS NOT NULL
                     AND evidence.is_complete
                     AND ple_private.student_assessment_score_is_released(
                         evidence.feedback_score, evidence.submitted_at,
                         evidence.due_at, evidence.closes_at, v_now
                     )
                THEN evidence.points_earned::double precision END,
           CASE WHEN evidence.submitted_at IS NOT NULL
                     AND evidence.is_complete
                     AND ple_private.student_assessment_score_is_released(
                         evidence.feedback_score, evidence.submitted_at,
                         evidence.due_at, evidence.closes_at, v_now
                     )
                THEN evidence.points_possible::double precision END
     FROM attempt_evidence AS evidence
     ORDER BY evidence.started_at DESC, evidence.assessment_attempt_id DESC;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_live_student_course_attempt_history(
    p_course_instance_id text,
    p_after text,
    p_page_size integer
) RETURNS TABLE (
    continuation_key text,
    assessment_attempt_id uuid,
    assessment_id text,
    assessment_title text,
    assessment_attempt_number integer,
    started_at_millis bigint,
    submitted_at_millis bigint,
    assessment_score_points_earned double precision,
    assessment_score_points_possible double precision
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    student_record_id_value uuid;
BEGIN
    SELECT student.student_record_id
      INTO student_record_id_value
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
       AND ple_data.course_membership_is_active(membership.course_membership_id)
      JOIN ple_data.course_instance AS course
        ON course.course_instance_id = membership.course_instance_id
       AND course.retention_lifecycle_state = 'active'
      JOIN ple_data.student_record AS student
        ON student.student_record_id = membership.student_record_id
       AND student.course_instance_id = course.course_instance_id
       AND student.student_account_id = account.account_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'student'
       AND course.course_instance_id = p_course_instance_id;
    IF student_record_id_value IS NULL THEN
        RAISE EXCEPTION 'Student Course not found' USING ERRCODE = '42501';
    END IF;

    RETURN QUERY
    SELECT history.continuation_key, history.assessment_attempt_id,
           history.assessment_id, history.assessment_title,
           history.assessment_attempt_number, history.started_at_millis,
           history.submitted_at_millis, history.assessment_score_points_earned,
           history.assessment_score_points_possible
      FROM ple_private.read_student_course_attempt_history(
          p_course_instance_id, student_record_id_value, p_after, p_page_size
      ) AS history;
END
$$;
