-- Student-only Course Response Stats from disclosed, exact-revision outcomes.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.read_student_course_response_stats(
    p_course_instance_id text,
    p_student_record_id uuid
) RETURNS TABLE (
    published_question_id text,
    revision_number integer,
    full_credit_attempt_count bigint,
    partial_credit_attempt_count bigint,
    incorrect_attempt_count bigint,
    unanswered_attempt_count bigint,
    disclosed_attempt_count bigint,
    not_full_credit_count bigint,
    average_display_duration_ms double precision,
    display_duration_sample_count bigint,
    relevant_assessment_attempt_id uuid
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    WITH disclosed_question_attempts AS (
        SELECT issued.published_question_id,
               issued.revision_number,
               attempt.assessment_attempt_id,
               attempt.started_at,
               question_attempt.display_duration_ms,
               CASE WHEN saved.question_attempt_id IS NULL THEN 'unanswered'
                    WHEN result.normalized_credit = 1 THEN 'full_credit'
                    WHEN result.normalized_credit > 0 THEN 'partial_credit'
                    ELSE 'incorrect' END AS outcome
          FROM ple_private.assessment_attempt AS attempt
          JOIN ple_private.assessment_submission AS submission
            ON submission.course_instance_id = attempt.course_instance_id
           AND submission.assessment_attempt_id = attempt.assessment_attempt_id
          JOIN ple_data.assessment_policy_snapshot AS policy
            ON policy.assessment_policy_snapshot_id = attempt.assessment_policy_snapshot_id
          JOIN ple_private.issued_question AS issued
            ON issued.course_instance_id = attempt.course_instance_id
           AND issued.assessment_attempt_id = attempt.assessment_attempt_id
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.course_instance_id = issued.course_instance_id
           AND question_attempt.issued_question_id = issued.issued_question_id
          LEFT JOIN ple_private.assessment_attempt_saved_response AS saved
            ON saved.course_instance_id = question_attempt.course_instance_id
           AND saved.question_attempt_id = question_attempt.question_attempt_id
          LEFT JOIN ple_private.grading_result AS result
            ON result.course_instance_id = question_attempt.course_instance_id
           AND result.question_attempt_id = question_attempt.question_attempt_id
         WHERE attempt.course_instance_id = p_course_instance_id
           AND attempt.student_record_id = p_student_record_id
           AND ple_api.course_student_work_is_ordinarily_visible(attempt.course_instance_id)
           AND ple_private.student_assessment_score_is_released(
               policy.feedback_score, submission.submitted_at,
               policy.due_at, policy.closes_at, pg_catalog.statement_timestamp()
           )
           AND ple_private.student_assessment_score_is_released(
               policy.feedback_per_item_correctness, submission.submitted_at,
               policy.due_at, policy.closes_at, pg_catalog.statement_timestamp()
           )
           AND (
               (saved.question_attempt_id IS NULL
                    AND question_attempt.finalized_at IS NOT NULL)
               OR (saved.question_attempt_id IS NOT NULL
                   AND result.grading_result_id IS NOT NULL
                   AND ple_api.has_automated_grading_receipt(result.grading_result_id))
           )
    ), question_counts AS (
        SELECT disclosed.published_question_id,
               disclosed.revision_number,
               count(*) FILTER (WHERE disclosed.outcome = 'full_credit')::bigint
                   AS full_credit_attempt_count,
               count(*) FILTER (WHERE disclosed.outcome = 'partial_credit')::bigint
                   AS partial_credit_attempt_count,
               count(*) FILTER (WHERE disclosed.outcome = 'incorrect')::bigint
                   AS incorrect_attempt_count,
               count(*) FILTER (WHERE disclosed.outcome = 'unanswered')::bigint
                   AS unanswered_attempt_count,
               count(*)::bigint AS disclosed_attempt_count,
               count(*) FILTER (WHERE disclosed.outcome <> 'full_credit')::bigint
                   AS not_full_credit_count,
               avg(disclosed.display_duration_ms)::double precision
                   AS average_display_duration_ms,
               count(disclosed.display_duration_ms)::bigint
                   AS display_duration_sample_count
          FROM disclosed_question_attempts AS disclosed
         GROUP BY disclosed.published_question_id, disclosed.revision_number
    )
    SELECT counts.published_question_id::text,
           counts.revision_number,
           counts.full_credit_attempt_count,
           counts.partial_credit_attempt_count,
           counts.incorrect_attempt_count,
           counts.unanswered_attempt_count,
           counts.disclosed_attempt_count,
           counts.not_full_credit_count,
           counts.average_display_duration_ms,
           counts.display_duration_sample_count,
           coalesce(
               (SELECT disclosed.assessment_attempt_id
                  FROM disclosed_question_attempts AS disclosed
                 WHERE disclosed.published_question_id = counts.published_question_id
                   AND disclosed.revision_number = counts.revision_number
                   AND disclosed.outcome <> 'full_credit'
                 ORDER BY disclosed.started_at DESC,
                          disclosed.assessment_attempt_id DESC
                 LIMIT 1),
               (SELECT disclosed.assessment_attempt_id
                  FROM disclosed_question_attempts AS disclosed
                 WHERE disclosed.published_question_id = counts.published_question_id
                   AND disclosed.revision_number = counts.revision_number
                 ORDER BY disclosed.started_at DESC,
                          disclosed.assessment_attempt_id DESC
                 LIMIT 1)
           )
      FROM question_counts AS counts
     ORDER BY counts.not_full_credit_count DESC,
              counts.disclosed_attempt_count DESC,
              counts.published_question_id,
              counts.revision_number
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_live_student_course_response_stats(
    p_course_instance_id text
) RETURNS TABLE (
    published_question_id text,
    revision_number integer,
    full_credit_attempt_count bigint,
    partial_credit_attempt_count bigint,
    incorrect_attempt_count bigint,
    unanswered_attempt_count bigint,
    disclosed_attempt_count bigint,
    not_full_credit_count bigint,
    average_display_duration_ms double precision,
    display_duration_sample_count bigint,
    relevant_assessment_attempt_id uuid
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
    SELECT stats.published_question_id, stats.revision_number,
           stats.full_credit_attempt_count, stats.partial_credit_attempt_count,
           stats.incorrect_attempt_count, stats.unanswered_attempt_count,
           stats.disclosed_attempt_count, stats.not_full_credit_count,
           stats.average_display_duration_ms, stats.display_duration_sample_count,
           stats.relevant_assessment_attempt_id
      FROM ple_private.read_student_course_response_stats(
          p_course_instance_id, student_record_id_value
      ) AS stats;
END
$$;
