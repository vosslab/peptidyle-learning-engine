-- Functions, triggers, and views from statistics.sql.

SET LOCAL ROLE ple_data_owner;

-- ASVS 15.4.2: the private receipt winner increments in the same transaction.
-- Retained counts never depend on reconstructing deleted Student evidence.
-- p_normalized_credit NULL means a blank Issued Question (no saved response).
CREATE FUNCTION ple_data.increment_question_revision_statistics(
    p_published_question_id text,
    p_revision_number integer,
    p_normalized_credit numeric,
    p_observed_on date,
    p_issued_floor bigint,
    p_blank_floor bigint,
    p_answered_floor bigint,
    p_correct_floor bigint,
    p_partial_floor bigint,
    p_incorrect_floor bigint
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
       OR p_issued_floor < 0 OR p_blank_floor < 0 OR p_answered_floor < 0
       OR p_correct_floor < 0 OR p_partial_floor < 0 OR p_incorrect_floor < 0
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
        issued_contributor_floor, blank_contributor_floor, answered_contributor_floor,
        correct_contributor_floor, partial_contributor_floor, incorrect_contributor_floor,
        credit_sum, credit_sum_sq, updated_on
    ) VALUES (
        p_published_question_id, p_revision_number,
        1, blank_delta, answered_delta,
        correct_delta, partial_delta, incorrect_delta,
        least(p_issued_floor, 1), least(p_blank_floor, blank_delta),
        least(p_answered_floor, answered_delta), least(p_correct_floor, correct_delta),
        least(p_partial_floor, partial_delta), least(p_incorrect_floor, incorrect_delta),
        credit_delta, credit_delta * credit_delta, p_observed_on
    )
    ON CONFLICT (published_question_id, revision_number) DO UPDATE
        SET issued_count = retained.issued_count + 1,
            blank_count = retained.blank_count + EXCLUDED.blank_count,
            answered_count = retained.answered_count + EXCLUDED.answered_count,
            correct_count = retained.correct_count + EXCLUDED.correct_count,
            partial_count = retained.partial_count + EXCLUDED.partial_count,
            incorrect_count = retained.incorrect_count + EXCLUDED.incorrect_count,
            issued_contributor_floor = greatest(retained.issued_contributor_floor,
                least(p_issued_floor, retained.issued_count + 1)),
            blank_contributor_floor = greatest(retained.blank_contributor_floor,
                least(p_blank_floor, retained.blank_count + EXCLUDED.blank_count)),
            answered_contributor_floor = greatest(retained.answered_contributor_floor,
                least(p_answered_floor, retained.answered_count + EXCLUDED.answered_count)),
            correct_contributor_floor = greatest(retained.correct_contributor_floor,
                least(p_correct_floor, retained.correct_count + EXCLUDED.correct_count)),
            partial_contributor_floor = greatest(retained.partial_contributor_floor,
                least(p_partial_floor, retained.partial_count + EXCLUDED.partial_count)),
            incorrect_contributor_floor = greatest(retained.incorrect_contributor_floor,
                least(p_incorrect_floor, retained.incorrect_count + EXCLUDED.incorrect_count)),
            credit_sum = retained.credit_sum + EXCLUDED.credit_sum,
            credit_sum_sq = retained.credit_sum_sq + EXCLUDED.credit_sum_sq,
            updated_on = greatest(retained.updated_on, EXCLUDED.updated_on);
END
$$;

CREATE FUNCTION ple_data.increment_question_pool_issue_statistics(
    p_question_pool_id text,
    p_published_question_id text,
    p_observed_on date,
    p_issued_contributor_floor bigint
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF p_question_pool_id IS NULL
       OR p_published_question_id IS NULL
       OR p_observed_on IS NULL OR p_issued_contributor_floor < 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool Statistics increment is invalid';
    END IF;

    INSERT INTO ple_data.question_pool_statistics AS retained (
        question_pool_id, issued_count, issued_contributor_floor, updated_on
    ) VALUES (p_question_pool_id, 1, least(p_issued_contributor_floor, 1), p_observed_on)
    ON CONFLICT (question_pool_id) DO UPDATE
        SET issued_count = retained.issued_count + 1,
            issued_contributor_floor = greatest(retained.issued_contributor_floor,
                least(p_issued_contributor_floor, retained.issued_count + 1)),
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

-- Resolve Student Record identities only long enough to count distinct
-- Accounts. The only returned values are six anonymous cohort counts.
SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.count_statistics_contributors(
    p_student_record_ids uuid[], p_cohorts text[]
) RETURNS TABLE (
    issued_floor bigint, blank_floor bigint, answered_floor bigint,
    correct_floor bigint, partial_floor bigint, incorrect_floor bigint
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT count(DISTINCT student.student_account_id),
           count(DISTINCT student.student_account_id) FILTER (WHERE observation.cohort = 'blank'),
           count(DISTINCT student.student_account_id) FILTER (WHERE observation.cohort IN ('answered', 'correct', 'partial', 'incorrect')),
           count(DISTINCT student.student_account_id) FILTER (WHERE observation.cohort = 'correct'),
           count(DISTINCT student.student_account_id) FILTER (WHERE observation.cohort = 'partial'),
           count(DISTINCT student.student_account_id) FILTER (WHERE observation.cohort = 'incorrect')
      FROM unnest(p_student_record_ids, p_cohorts) AS observation(student_record_id, cohort)
      JOIN ple_data.student_record AS student
        ON student.student_record_id = observation.student_record_id
     WHERE observation.cohort IN ('issued', 'blank', 'answered', 'correct', 'partial', 'incorrect')
$$;
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
    v_student_record_ids uuid[];
    v_cohorts text[];
    v_floors record;
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

    SELECT array_agg(attempt.student_record_id),
           array_agg(CASE
               WHEN saved.question_attempt_id IS NULL THEN 'blank'
               WHEN result.normalized_credit = 1 THEN 'correct'
               WHEN result.normalized_credit > 0 THEN 'partial'
               WHEN result.normalized_credit = 0 THEN 'incorrect'
               ELSE 'answered'
           END)
      INTO v_student_record_ids, v_cohorts
      FROM ple_private.question_statistics_observation_receipt AS receipt
      JOIN ple_private.issued_question AS issued
        ON issued.course_instance_id = receipt.course_instance_id
       AND issued.issued_question_id = receipt.issued_question_id
      JOIN ple_private.assessment_attempt AS attempt
        ON attempt.course_instance_id = issued.course_instance_id
       AND attempt.assessment_attempt_id = issued.assessment_attempt_id
      LEFT JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.course_instance_id = issued.course_instance_id
       AND question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.assessment_attempt_saved_response AS saved
        ON saved.course_instance_id = question_attempt.course_instance_id
       AND saved.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_private.grading_result AS result
        ON result.course_instance_id = question_attempt.course_instance_id
       AND result.question_attempt_id = question_attempt.question_attempt_id
     WHERE receipt.published_question_id = v_question_id
       AND receipt.revision_number = v_revision_number;
    SELECT * INTO v_floors
      FROM ple_api.count_statistics_contributors(v_student_record_ids, v_cohorts);
    PERFORM ple_data.increment_question_revision_statistics(
        v_question_id, v_revision_number, v_credit, p_observed_at::date,
        v_floors.issued_floor, v_floors.blank_floor, v_floors.answered_floor,
        v_floors.correct_floor, v_floors.partial_floor, v_floors.incorrect_floor);

    IF v_pool_id IS NOT NULL THEN
        SELECT array_agg(attempt.student_record_id), array_agg('issued'::text)
          INTO v_student_record_ids, v_cohorts
          FROM ple_private.question_statistics_observation_receipt AS receipt
          JOIN ple_private.issued_question AS issued
            ON issued.course_instance_id = receipt.course_instance_id
           AND issued.issued_question_id = receipt.issued_question_id
          JOIN ple_private.question_pool_selection AS pool
            ON pool.course_instance_id = issued.course_instance_id
           AND pool.question_pool_selection_id = issued.question_pool_selection_id
          JOIN ple_private.assessment_attempt AS attempt
            ON attempt.course_instance_id = issued.course_instance_id
           AND attempt.assessment_attempt_id = issued.assessment_attempt_id
         WHERE pool.question_pool_id = v_pool_id;
        SELECT count_statistics.issued_floor INTO v_floors
          FROM ple_api.count_statistics_contributors(v_student_record_ids, v_cohorts) AS count_statistics;
        PERFORM ple_data.increment_question_pool_issue_statistics(
            v_pool_id, v_question_id, p_observed_at::date, v_floors.issued_floor);
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
    issued_contributor_floor bigint,
    blank_contributor_floor bigint,
    answered_contributor_floor bigint,
    correct_contributor_floor bigint,
    partial_contributor_floor bigint,
    incorrect_contributor_floor bigint,
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
           COALESCE(MAX(stats.issued_contributor_floor), 0),
           COALESCE(MAX(stats.blank_contributor_floor), 0),
           COALESCE(MAX(stats.answered_contributor_floor), 0),
           COALESCE(MAX(stats.correct_contributor_floor), 0),
           COALESCE(MAX(stats.partial_contributor_floor), 0),
           COALESCE(MAX(stats.incorrect_contributor_floor), 0),
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
    issued_contributor_floor bigint,
    blank_contributor_floor bigint,
    answered_contributor_floor bigint,
    correct_contributor_floor bigint,
    partial_contributor_floor bigint,
    incorrect_contributor_floor bigint,
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
           COALESCE(stats.issued_contributor_floor, 0),
           COALESCE(stats.blank_contributor_floor, 0),
           COALESCE(stats.answered_contributor_floor, 0),
           COALESCE(stats.correct_contributor_floor, 0),
           COALESCE(stats.partial_contributor_floor, 0),
           COALESCE(stats.incorrect_contributor_floor, 0),
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
    pool_issued_contributor_floor bigint,
    issued_count bigint,
    blank_count bigint,
    answered_count bigint,
    correct_count bigint,
    partial_count bigint,
    incorrect_count bigint,
    issued_contributor_floor bigint,
    blank_contributor_floor bigint,
    answered_contributor_floor bigint,
    correct_contributor_floor bigint,
    partial_contributor_floor bigint,
    incorrect_contributor_floor bigint,
    credit_sum numeric,
    credit_sum_sq numeric
) LANGUAGE sql STABLE
SET search_path = pg_catalog, ple_data AS $$
    SELECT COALESCE((
               SELECT pool_stats.issued_count
                 FROM ple_data.question_pool_statistics AS pool_stats
                WHERE pool_stats.question_pool_id = p_question_pool_id
           ), 0),
           COALESCE((SELECT pool_stats.issued_contributor_floor
                 FROM ple_data.question_pool_statistics AS pool_stats
                WHERE pool_stats.question_pool_id = p_question_pool_id), 0),
           COALESCE(SUM(stats.issued_count), 0),
           COALESCE(SUM(stats.blank_count), 0),
           COALESCE(SUM(stats.answered_count), 0),
           COALESCE(SUM(stats.correct_count), 0),
           COALESCE(SUM(stats.partial_count), 0),
           COALESCE(SUM(stats.incorrect_count), 0),
           COALESCE(MAX(stats.issued_contributor_floor), 0),
           COALESCE(MAX(stats.blank_contributor_floor), 0),
           COALESCE(MAX(stats.answered_contributor_floor), 0),
           COALESCE(MAX(stats.correct_contributor_floor), 0),
           COALESCE(MAX(stats.partial_contributor_floor), 0),
           COALESCE(MAX(stats.incorrect_contributor_floor), 0),
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
    issued_contributor_floor bigint,
    blank_contributor_floor bigint,
    answered_contributor_floor bigint,
    correct_contributor_floor bigint,
    partial_contributor_floor bigint,
    incorrect_contributor_floor bigint,
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
    issued_contributor_floor bigint,
    blank_contributor_floor bigint,
    answered_contributor_floor bigint,
    correct_contributor_floor bigint,
    partial_contributor_floor bigint,
    incorrect_contributor_floor bigint,
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
    pool_issued_contributor_floor bigint,
    issued_count bigint,
    blank_count bigint,
    answered_count bigint,
    correct_count bigint,
    partial_count bigint,
    incorrect_count bigint,
    issued_contributor_floor bigint,
    blank_contributor_floor bigint,
    answered_contributor_floor bigint,
    correct_contributor_floor bigint,
    partial_contributor_floor bigint,
    incorrect_contributor_floor bigint,
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
