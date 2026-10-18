-- Global, self-only Student Attempt History. The same authorized match set owns
-- page rows and facet choices; no browser-supplied Account or Student ID is used.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.student_attempt_history_matches(p_search text)
RETURNS TABLE (
    course_instance_id text,
    course_short_name text,
    course_long_name text,
    assessment_attempt_id uuid,
    assessment_id text,
    assessment_title text,
    partial_credit_enabled boolean,
    assessment_attempt_number integer,
    started_at timestamptz,
    submitted_at timestamptz
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT DISTINCT course.course_instance_id, course.course_short_name, course.course_long_name,
           attempt.assessment_attempt_id, assessment.assessment_id::text,
           policy.assessment_title, current_policy.partial_credit_enabled,
           attempt.assessment_attempt_number, attempt.started_at, submission.submitted_at
      FROM ple_api.list_live_student_course_landing() AS course
      CROSS JOIN LATERAL ple_api.current_session_student_record_id(
          course.course_instance_id
      ) AS student(student_record_id)
      JOIN ple_private.assessment_attempt AS attempt
        ON attempt.course_instance_id = course.course_instance_id
       AND attempt.student_record_id = student.student_record_id
      JOIN ple_data.assessment AS assessment
        ON assessment.course_instance_id = attempt.course_instance_id
       AND assessment.assessment_id = attempt.assessment_id
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = attempt.assessment_policy_snapshot_id
      JOIN ple_data.assessment_policy_snapshot AS current_policy
        ON current_policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
      LEFT JOIN ple_private.assessment_submission AS submission
        ON submission.course_instance_id = attempt.course_instance_id
       AND submission.assessment_attempt_id = attempt.assessment_attempt_id
     WHERE ple_api.course_student_work_is_ordinarily_visible(course.course_instance_id)
       AND (p_search = '' OR position(lower(p_search) in lower(policy.assessment_title)) > 0
            OR position(lower(p_search) in lower(course.course_long_name)) > 0
            OR position(lower(p_search) in lower(course.course_short_name)) > 0)
$$;

CREATE FUNCTION ple_private.read_student_attempt_history(
    p_page_number bigint, p_page_size integer, p_search text, p_state text,
    p_course_ids text[], p_sort text
) RETURNS TABLE (
    has_item boolean,
    resolved_page_number bigint,
    total_count bigint,
    course_instance_id text,
    course_short_name text,
    course_long_name text,
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
BEGIN
    IF p_page_number IS NULL OR p_page_number NOT BETWEEN 1 AND 4294967295
       OR p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 100
       OR p_search IS NULL OR char_length(p_search) > 200
       OR p_state IS NULL OR p_state NOT IN ('all', 'submitted', 'inProgress')
       OR p_sort IS NULL OR p_sort NOT IN ('startedNewest', 'startedOldest', 'titleAscending',
                         'titleDescending', 'courseAscending', 'courseDescending')
       OR p_course_ids IS NULL OR cardinality(p_course_ids) > 100 THEN
        RAISE EXCEPTION 'invalid Student Attempt History query' USING ERRCODE = '22023';
    END IF;

    RETURN QUERY
    WITH matched AS MATERIALIZED (
        SELECT * FROM ple_private.student_attempt_history_matches(p_search) AS item
         WHERE (cardinality(p_course_ids) = 0 OR item.course_instance_id = ANY(p_course_ids))
           AND (p_state = 'all'
                OR (p_state = 'submitted' AND item.submitted_at IS NOT NULL)
                OR (p_state = 'inProgress' AND item.submitted_at IS NULL))
    ), counted AS (
        SELECT count(*)::bigint AS total FROM matched AS matched_item
    ), resolved AS (
        SELECT CASE WHEN counted.total = 0 THEN 1::bigint ELSE
                    least(p_page_number, ((counted.total - 1) / p_page_size) + 1) END AS page_number,
               counted.total AS total_count
          FROM counted AS counted
    ), page AS MATERIALIZED (
        SELECT item.* FROM matched AS item CROSS JOIN resolved AS resolved_page
         ORDER BY
           CASE WHEN p_sort = 'startedNewest' THEN item.started_at END DESC,
           CASE WHEN p_sort = 'startedOldest' THEN item.started_at END ASC,
           CASE WHEN p_sort = 'titleAscending' THEN lower(item.assessment_title) COLLATE "C" END ASC,
           CASE WHEN p_sort = 'titleDescending' THEN lower(item.assessment_title) COLLATE "C" END DESC,
           CASE WHEN p_sort = 'courseAscending' THEN lower(item.course_long_name) COLLATE "C" END ASC,
           CASE WHEN p_sort = 'courseDescending' THEN lower(item.course_long_name) COLLATE "C" END DESC,
           CASE WHEN p_sort = 'courseAscending' THEN item.course_instance_id END ASC,
           CASE WHEN p_sort = 'courseDescending' THEN item.course_instance_id END DESC,
           CASE WHEN p_sort IN ('startedOldest', 'titleAscending', 'courseAscending')
                THEN item.assessment_attempt_id END ASC,
           CASE WHEN p_sort IN ('startedNewest', 'titleDescending', 'courseDescending')
                THEN item.assessment_attempt_id END DESC
         OFFSET (((SELECT resolved_page.page_number FROM resolved AS resolved_page) - 1)
                 * p_page_size::bigint)
         LIMIT p_page_size
    ), evidence AS MATERIALIZED (
        SELECT page.*,
               score.is_complete, score.points_earned, score.points_possible
          FROM page AS page
          CROSS JOIN LATERAL (
              SELECT count(issued.issued_question_id) > 0
                         AND bool_and(
                             saved.question_attempt_id IS NULL
                             OR (result.grading_result_id IS NOT NULL
                                 AND ple_api.has_automated_grading_receipt(result.grading_result_id))
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
                    ple_private.current_assessment_entry_points(issued.assessment_entry_id, snapshot.points),
                    page.partial_credit_enabled
                ) AS credit
               WHERE issued.course_instance_id = page.course_instance_id
                 AND issued.assessment_attempt_id = page.assessment_attempt_id
          ) AS score
    )
    SELECT (item.assessment_attempt_id IS NOT NULL), resolved_page.page_number,
           resolved_page.total_count, item.course_instance_id, item.course_short_name,
           item.course_long_name, item.assessment_attempt_id, item.assessment_id,
           item.assessment_title, item.assessment_attempt_number,
           floor(extract(epoch FROM item.started_at) * 1000)::bigint,
           CASE WHEN item.submitted_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM item.submitted_at) * 1000)::bigint END,
           CASE WHEN item.submitted_at IS NOT NULL AND item.is_complete
                THEN item.points_earned::double precision END,
           CASE WHEN item.submitted_at IS NOT NULL AND item.is_complete
                THEN item.points_possible::double precision END
      FROM resolved AS resolved_page
      LEFT JOIN evidence AS item ON true
     ORDER BY
       CASE WHEN p_sort = 'startedNewest' THEN item.started_at END DESC,
       CASE WHEN p_sort = 'startedOldest' THEN item.started_at END ASC,
       CASE WHEN p_sort = 'titleAscending' THEN lower(item.assessment_title) COLLATE "C" END ASC,
       CASE WHEN p_sort = 'titleDescending' THEN lower(item.assessment_title) COLLATE "C" END DESC,
       CASE WHEN p_sort = 'courseAscending' THEN lower(item.course_long_name) COLLATE "C" END ASC,
       CASE WHEN p_sort = 'courseDescending' THEN lower(item.course_long_name) COLLATE "C" END DESC,
       CASE WHEN p_sort = 'courseAscending' THEN item.course_instance_id END ASC,
       CASE WHEN p_sort = 'courseDescending' THEN item.course_instance_id END DESC,
       CASE WHEN p_sort IN ('startedOldest', 'titleAscending', 'courseAscending')
            THEN item.assessment_attempt_id END ASC,
       CASE WHEN p_sort IN ('startedNewest', 'titleDescending', 'courseDescending')
            THEN item.assessment_attempt_id END DESC;
END
$$;

CREATE FUNCTION ple_private.student_attempt_history_facet_choices(
    p_search text, p_state text, p_course_ids text[], p_field text,
    p_choice_search text, p_after text, p_page_size integer
) RETURNS TABLE (
    continuation_key text, facet_key text, facet_label text, matching_count bigint
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_search IS NULL OR char_length(p_search) > 200
       OR p_state IS NULL OR p_state NOT IN ('all', 'submitted', 'inProgress')
       OR p_course_ids IS NULL OR cardinality(p_course_ids) > 100
       OR p_field IS NULL OR p_field NOT IN ('course', 'state')
       OR p_choice_search IS NULL OR char_length(p_choice_search) > 100
       OR p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 50 THEN
        RAISE EXCEPTION 'invalid Student Attempt History facet query'
            USING ERRCODE = '22023';
    END IF;

    IF p_field = 'state' THEN
        RETURN QUERY
        WITH matched AS (
            SELECT item.submitted_at
              FROM ple_private.student_attempt_history_matches(p_search) AS item
             WHERE cardinality(p_course_ids) = 0
                OR item.course_instance_id = ANY(p_course_ids)
        ), choices AS (
            SELECT 'inProgress'::text AS key, 'In progress'::text AS label,
                   count(*) FILTER (WHERE submitted_at IS NULL) AS count FROM matched
            UNION ALL
            SELECT 'submitted'::text, 'Submitted'::text,
                   count(*) FILTER (WHERE submitted_at IS NOT NULL) FROM matched
        )
        SELECT choice.key, choice.key, choice.label, choice.count
          FROM choices AS choice
         WHERE (p_choice_search = ''
                OR position(lower(p_choice_search) in lower(choice.label)) > 0)
           AND (p_after IS NULL OR choice.key > p_after)
         ORDER BY choice.label
         LIMIT p_page_size + 1;
        RETURN;
    END IF;

    RETURN QUERY
    WITH choices AS MATERIALIZED (
        SELECT item.course_instance_id AS key,
               item.course_long_name || ' (Course ID: ' || item.course_instance_id || ')' AS label,
               count(*) AS count
          FROM ple_private.student_attempt_history_matches(p_search) AS item
         WHERE p_state = 'all'
            OR (p_state = 'submitted' AND item.submitted_at IS NOT NULL)
            OR (p_state = 'inProgress' AND item.submitted_at IS NULL)
         GROUP BY item.course_instance_id, item.course_long_name
    ), filtered AS (
        SELECT choice.* FROM choices AS choice
         WHERE p_choice_search = ''
            OR position(lower(p_choice_search) in lower(choice.label)) > 0
    ), anchor AS (
        SELECT * FROM filtered WHERE key = p_after
    )
    SELECT choice.key, choice.key, choice.label, choice.count
      FROM filtered AS choice
      LEFT JOIN anchor ON true
     WHERE p_after IS NULL
        OR (lower(choice.label) COLLATE "C", choice.key)
             > (lower(anchor.label) COLLATE "C", anchor.key)
     ORDER BY lower(choice.label) COLLATE "C", choice.key
     LIMIT p_page_size + 1;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_student_attempt_history(
    p_page_number bigint, p_page_size integer, p_search text, p_state text,
    p_course_ids text[], p_sort text
) RETURNS TABLE (
    has_item boolean, resolved_page_number bigint, total_count bigint,
    course_instance_id text, course_short_name text, course_long_name text,
    assessment_attempt_id uuid, assessment_id text, assessment_title text,
    assessment_attempt_number integer, started_at_millis bigint, submitted_at_millis bigint,
    assessment_score_points_earned double precision, assessment_score_points_possible double precision
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT history_row.* FROM ple_private.read_student_attempt_history(
        p_page_number, p_page_size, p_search, p_state, p_course_ids, p_sort
    ) AS history_row
$$;

CREATE FUNCTION ple_api.list_student_attempt_history_facet_choices(
    p_search text, p_state text, p_course_ids text[], p_field text,
    p_choice_search text, p_after text, p_page_size integer
) RETURNS TABLE (
    continuation_key text, facet_key text, facet_label text, matching_count bigint
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT * FROM ple_private.student_attempt_history_facet_choices(
        p_search, p_state, p_course_ids, p_field, p_choice_search, p_after, p_page_size
    )
$$;
