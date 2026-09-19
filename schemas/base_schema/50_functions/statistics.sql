-- Functions, triggers, and views from statistics.sql.

SET LOCAL ROLE ple_data_owner;

-- ASVS 15.4.2: the private receipt winner increments in the same transaction.
-- Retained counts never depend on reconstructing deleted Student evidence.
-- p_normalized_credit NULL means a blank Issued Question (no saved response).
CREATE FUNCTION ple_data.increment_question_revision_statistics(
    p_published_question_id text,
    p_revision_number integer,
    p_normalized_credit numeric,
    p_observed_on date
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE
    blank_delta bigint;
    answered_delta bigint;
    correct_delta bigint;
    partial_delta bigint;
    incorrect_delta bigint;
    credit_delta numeric;
BEGIN
    IF p_published_question_id IS NULL
       OR p_published_question_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
       OR substr(p_published_question_id, 6, 1) <> ple_private.crockford_checksum_character(
           substr(p_published_question_id, 1, 4) || substr(p_published_question_id, 7, 3)
       )
       OR p_revision_number IS NULL OR p_revision_number <= 0
       OR p_observed_on IS NULL
       OR (p_normalized_credit IS NOT NULL
           AND (p_normalized_credit < 0 OR p_normalized_credit > 1)) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Revision Statistics increment is invalid';
    END IF;

    IF p_normalized_credit IS NULL THEN
        blank_delta := 1;
        answered_delta := 0;
        correct_delta := 0;
        partial_delta := 0;
        incorrect_delta := 0;
        credit_delta := 0;
    ELSE
        blank_delta := 0;
        answered_delta := 1;
        correct_delta := CASE WHEN p_normalized_credit = 1 THEN 1 ELSE 0 END;
        partial_delta := CASE WHEN p_normalized_credit > 0 AND p_normalized_credit < 1 THEN 1 ELSE 0 END;
        incorrect_delta := CASE WHEN p_normalized_credit = 0 THEN 1 ELSE 0 END;
        credit_delta := p_normalized_credit;
    END IF;

    INSERT INTO ple_data.question_revision_statistics AS retained (
        published_question_id, revision_number,
        issued_count, blank_count, answered_count,
        correct_count, partial_count, incorrect_count,
        credit_sum, credit_sum_sq, updated_on
    ) VALUES (
        p_published_question_id, p_revision_number,
        1, blank_delta, answered_delta,
        correct_delta, partial_delta, incorrect_delta,
        credit_delta, credit_delta * credit_delta, p_observed_on
    )
    ON CONFLICT (published_question_id, revision_number) DO UPDATE
        SET issued_count = retained.issued_count + 1,
            blank_count = retained.blank_count + EXCLUDED.blank_count,
            answered_count = retained.answered_count + EXCLUDED.answered_count,
            correct_count = retained.correct_count + EXCLUDED.correct_count,
            partial_count = retained.partial_count + EXCLUDED.partial_count,
            incorrect_count = retained.incorrect_count + EXCLUDED.incorrect_count,
            credit_sum = retained.credit_sum + EXCLUDED.credit_sum,
            credit_sum_sq = retained.credit_sum_sq + EXCLUDED.credit_sum_sq,
            updated_on = greatest(retained.updated_on, EXCLUDED.updated_on);
END
$$;

CREATE FUNCTION ple_data.increment_question_pool_issue_statistics(
    p_question_pool_id text,
    p_published_question_id text,
    p_observed_on date
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF p_question_pool_id IS NULL
       OR p_published_question_id IS NULL
       OR p_observed_on IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool Statistics increment is invalid';
    END IF;

    INSERT INTO ple_data.question_pool_statistics AS retained (
        question_pool_id, issued_count, updated_on
    ) VALUES (p_question_pool_id, 1, p_observed_on)
    ON CONFLICT (question_pool_id) DO UPDATE
        SET issued_count = retained.issued_count + 1,
            updated_on = greatest(retained.updated_on, EXCLUDED.updated_on);

    INSERT INTO ple_data.question_pool_member_statistics AS retained (
        question_pool_id, published_question_id, selected_count, updated_on
    ) VALUES (p_question_pool_id, p_published_question_id, 1, p_observed_on)
    ON CONFLICT (question_pool_id, published_question_id) DO UPDATE
        SET selected_count = retained.selected_count + 1,
            updated_on = greatest(retained.updated_on, EXCLUDED.updated_on);
END
$$;

CREATE FUNCTION ple_data.drop_question_pool_member_statistics_when_unselected()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_pool_member AS member
         WHERE member.question_pool_id = OLD.question_pool_id
           AND member.published_question_id = OLD.published_question_id
    ) THEN
        DELETE FROM ple_data.question_pool_member_statistics
         WHERE question_pool_id = OLD.question_pool_id
           AND published_question_id = OLD.published_question_id;
    END IF;
    RETURN NULL;
END
$$;

CREATE TRIGGER question_pool_member_statistics_follow_member
AFTER DELETE ON ple_data.question_pool_member
FOR EACH ROW EXECUTE FUNCTION ple_data.drop_question_pool_member_statistics_when_unselected();

SET LOCAL ROLE ple_private_owner;

-- One Issued Question contributes at most once, at its Assessment Submission.
CREATE FUNCTION ple_private.capture_issued_question_statistics_observation(
    p_course_instance_id text,
    p_issued_question_id uuid,
    p_assessment_submission_id uuid,
    p_observed_at timestamptz
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private, ple_data AS $$
DECLARE
    v_question_id text;
    v_revision_number integer;
    v_pool_id text;
    v_credit numeric;
    v_inserted boolean := false;
BEGIN
    IF p_course_instance_id IS NULL
       OR p_issued_question_id IS NULL
       OR p_assessment_submission_id IS NULL
       OR p_observed_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Statistics Observation facts are invalid';
    END IF;

    SELECT issued.published_question_id, issued.revision_number, pool.question_pool_id
      INTO v_question_id, v_revision_number, v_pool_id
      FROM ple_private.issued_question AS issued
      LEFT JOIN ple_private.question_pool_selection AS pool
        ON pool.course_instance_id = issued.course_instance_id
       AND pool.question_pool_selection_id = issued.question_pool_selection_id
     WHERE issued.course_instance_id = p_course_instance_id
       AND issued.issued_question_id = p_issued_question_id
       AND issued.question_statistics_eligibility;
    IF NOT FOUND THEN
        RETURN;
    END IF;

    INSERT INTO ple_private.question_statistics_observation_receipt (
        course_instance_id, issued_question_id, assessment_submission_id,
        published_question_id, revision_number, observed_at
    ) VALUES (
        p_course_instance_id, p_issued_question_id, p_assessment_submission_id,
        v_question_id, v_revision_number, p_observed_at
    ) ON CONFLICT (course_instance_id, issued_question_id) DO NOTHING
      RETURNING true INTO v_inserted;
    IF COALESCE(v_inserted, false) IS NOT TRUE THEN
        RETURN;
    END IF;

    SELECT result.normalized_credit INTO v_credit
      FROM ple_private.question_attempt AS attempt
      LEFT JOIN ple_private.assessment_attempt_saved_response AS saved
        ON saved.course_instance_id = attempt.course_instance_id
       AND saved.question_attempt_id = attempt.question_attempt_id
      LEFT JOIN ple_private.grading_result AS result
        ON result.course_instance_id = attempt.course_instance_id
       AND result.question_attempt_id = attempt.question_attempt_id
     WHERE attempt.course_instance_id = p_course_instance_id
       AND attempt.issued_question_id = p_issued_question_id;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM ple_private.question_attempt AS attempt
          JOIN ple_private.assessment_attempt_saved_response AS saved
            ON saved.course_instance_id = attempt.course_instance_id
           AND saved.question_attempt_id = attempt.question_attempt_id
          LEFT JOIN ple_private.grading_result AS result
            ON result.course_instance_id = attempt.course_instance_id
           AND result.question_attempt_id = attempt.question_attempt_id
         WHERE attempt.course_instance_id = p_course_instance_id
           AND attempt.issued_question_id = p_issued_question_id
           AND result.grading_result_id IS NULL
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Statistics Observation requires a grading result for a saved response';
    END IF;

    PERFORM ple_data.increment_question_revision_statistics(
        v_question_id, v_revision_number, v_credit, p_observed_at::date);
    IF v_pool_id IS NOT NULL THEN
        PERFORM ple_data.increment_question_pool_issue_statistics(
            v_pool_id, v_question_id, p_observed_at::date);
    END IF;
END
$$;

SET LOCAL ROLE ple_data_owner;

-- All-Revision rollup for bulk Question Library views. Missing statistic rows
-- contribute zeros so Instructors always receive a stable Available shape.
CREATE FUNCTION ple_data.question_usage_statistics_rollups(
    p_published_question_ids text[]
) RETURNS TABLE (
    published_question_id text,
    issued_count bigint,
    blank_count bigint,
    answered_count bigint,
    correct_count bigint,
    partial_count bigint,
    incorrect_count bigint,
    credit_sum numeric,
    credit_sum_sq numeric
) LANGUAGE sql STABLE
SET search_path = pg_catalog, ple_data AS $$
    SELECT requested.published_question_id,
           COALESCE(SUM(stats.issued_count), 0),
           COALESCE(SUM(stats.blank_count), 0),
           COALESCE(SUM(stats.answered_count), 0),
           COALESCE(SUM(stats.correct_count), 0),
           COALESCE(SUM(stats.partial_count), 0),
           COALESCE(SUM(stats.incorrect_count), 0),
           COALESCE(SUM(stats.credit_sum), 0),
           COALESCE(SUM(stats.credit_sum_sq), 0)
      FROM unnest(p_published_question_ids) AS requested(published_question_id)
      LEFT JOIN ple_data.question_revision_statistics AS stats
        ON stats.published_question_id = requested.published_question_id
     GROUP BY requested.published_question_id
$$;

-- Per-Revision rows for the Question detail page. Every accepted Revision
-- appears, including Revisions that have never been issued.
CREATE FUNCTION ple_data.question_revision_usage_statistics(
    p_published_question_id text
) RETURNS TABLE (
    revision_number integer,
    issued_count bigint,
    blank_count bigint,
    answered_count bigint,
    correct_count bigint,
    partial_count bigint,
    incorrect_count bigint,
    credit_sum numeric,
    credit_sum_sq numeric
) LANGUAGE sql STABLE
SET search_path = pg_catalog, ple_data AS $$
    SELECT revision.revision_number,
           COALESCE(stats.issued_count, 0),
           COALESCE(stats.blank_count, 0),
           COALESCE(stats.answered_count, 0),
           COALESCE(stats.correct_count, 0),
           COALESCE(stats.partial_count, 0),
           COALESCE(stats.incorrect_count, 0),
           COALESCE(stats.credit_sum, 0),
           COALESCE(stats.credit_sum_sq, 0)
      FROM ple_data.question_revision AS revision
      LEFT JOIN ple_data.question_revision_statistics AS stats
        ON stats.published_question_id = revision.published_question_id
       AND stats.revision_number = revision.revision_number
     WHERE revision.published_question_id = p_published_question_id
     ORDER BY revision.revision_number
$$;

-- Pool issued_count plus current-member all-Revision rollup. Outcome rates
-- use the member-sum denominators; Pool draws stay in pool_issued_count.
-- Member mean_credit is therefore weighted by answered_count.
CREATE FUNCTION ple_data.question_pool_usage_statistics(
    p_question_pool_id text
) RETURNS TABLE (
    pool_issued_count bigint,
    issued_count bigint,
    blank_count bigint,
    answered_count bigint,
    correct_count bigint,
    partial_count bigint,
    incorrect_count bigint,
    credit_sum numeric,
    credit_sum_sq numeric
) LANGUAGE sql STABLE
SET search_path = pg_catalog, ple_data AS $$
    SELECT COALESCE((
               SELECT pool_stats.issued_count
                 FROM ple_data.question_pool_statistics AS pool_stats
                WHERE pool_stats.question_pool_id = p_question_pool_id
           ), 0),
           COALESCE(SUM(stats.issued_count), 0),
           COALESCE(SUM(stats.blank_count), 0),
           COALESCE(SUM(stats.answered_count), 0),
           COALESCE(SUM(stats.correct_count), 0),
           COALESCE(SUM(stats.partial_count), 0),
           COALESCE(SUM(stats.incorrect_count), 0),
           COALESCE(SUM(stats.credit_sum), 0),
           COALESCE(SUM(stats.credit_sum_sq), 0)
      FROM ple_data.question_pool_member AS member
      LEFT JOIN ple_data.question_revision_statistics AS stats
        ON stats.published_question_id = member.published_question_id
     WHERE member.question_pool_id = p_question_pool_id
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.read_question_library_usage_statistics(
    p_published_question_ids text[]
) RETURNS TABLE (
    published_question_id text,
    issued_count bigint,
    blank_count bigint,
    answered_count bigint,
    correct_count bigint,
    partial_count bigint,
    incorrect_count bigint,
    credit_sum numeric,
    credit_sum_sq numeric
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_has_platform_administration()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Library requires an active Instructor or Sysadmin Account';
    END IF;
    IF p_published_question_ids IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Library usage statistics selection is invalid';
    END IF;
    RETURN QUERY
    SELECT * FROM ple_data.question_usage_statistics_rollups(p_published_question_ids);
END
$$;

CREATE FUNCTION ple_api.read_question_library_revision_usage_statistics(
    p_published_question_id text
) RETURNS TABLE (
    revision_number integer,
    issued_count bigint,
    blank_count bigint,
    answered_count bigint,
    correct_count bigint,
    partial_count bigint,
    incorrect_count bigint,
    credit_sum numeric,
    credit_sum_sq numeric
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_has_platform_administration()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Library requires an active Instructor or Sysadmin Account';
    END IF;
    IF p_published_question_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Library usage statistics selection is invalid';
    END IF;
    RETURN QUERY
    SELECT * FROM ple_data.question_revision_usage_statistics(p_published_question_id);
END
$$;

CREATE FUNCTION ple_api.read_question_pool_library_usage_statistics(
    p_question_pool_id text
) RETURNS TABLE (
    pool_issued_count bigint,
    issued_count bigint,
    blank_count bigint,
    answered_count bigint,
    correct_count bigint,
    partial_count bigint,
    incorrect_count bigint,
    credit_sum numeric,
    credit_sum_sq numeric
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_has_platform_administration()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Library requires an active Instructor or Sysadmin Account';
    END IF;
    IF p_question_pool_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool usage statistics selection is invalid';
    END IF;
    RETURN QUERY
    SELECT * FROM ple_data.question_pool_usage_statistics(p_question_pool_id);
END
$$;
