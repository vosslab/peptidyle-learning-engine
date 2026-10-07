-- Functions, triggers, and views from statistics.sql.

SET LOCAL ROLE ple_data_owner;

-- ASVS 8.2.1 and 8.3.1: isolate aggregate writes in narrow data-owner
-- functions; the runtime EXECUTE grant is limited to ple_private_owner.
CREATE FUNCTION ple_data.record_question_revision_delivery(
    p_published_question_id ple_data.question_family_id,
    p_revision_number integer,
    p_issued_contributor_floor bigint,
    p_observed_on date
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF p_published_question_id IS NULL OR p_revision_number IS NULL OR p_revision_number <= 0
       OR p_issued_contributor_floor IS NULL OR p_issued_contributor_floor < 0
       OR p_observed_on IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Revision delivery statistics are invalid';
    END IF;

    INSERT INTO ple_data.question_revision_statistics AS retained (
        published_question_id, revision_number, issued_count,
        issued_contributor_floor, updated_on
    ) VALUES (
        p_published_question_id, p_revision_number, 1,
        least(p_issued_contributor_floor, 1), p_observed_on
    ) ON CONFLICT (published_question_id, revision_number) DO UPDATE
        SET issued_count = retained.issued_count + 1,
            issued_contributor_floor = greatest(retained.issued_contributor_floor,
                least(p_issued_contributor_floor, retained.issued_count + 1)),
            updated_on = greatest(retained.updated_on, EXCLUDED.updated_on);
END
$$;

CREATE FUNCTION ple_data.record_question_pool_delivery(
    p_question_pool_id ple_data.question_family_id,
    p_issued_contributor_floor bigint,
    p_observed_on date
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF p_question_pool_id IS NULL OR p_issued_contributor_floor IS NULL
       OR p_issued_contributor_floor < 0 OR p_observed_on IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool delivery statistics are invalid';
    END IF;

    INSERT INTO ple_data.question_pool_statistics AS retained (
        question_pool_id, issued_count, issued_contributor_floor, updated_on
    ) VALUES (
        p_question_pool_id, 1, least(p_issued_contributor_floor, 1), p_observed_on
    ) ON CONFLICT (question_pool_id) DO UPDATE
        SET issued_count = retained.issued_count + 1,
            issued_contributor_floor = greatest(retained.issued_contributor_floor,
                least(p_issued_contributor_floor, retained.issued_count + 1)),
            updated_on = greatest(retained.updated_on, EXCLUDED.updated_on);
END
$$;

CREATE FUNCTION ple_data.record_question_revision_outcome(
    p_published_question_id ple_data.question_family_id,
    p_revision_number integer,
    p_normalized_credit numeric,
    p_blank_contributor_floor bigint,
    p_answered_contributor_floor bigint,
    p_correct_contributor_floor bigint,
    p_partial_contributor_floor bigint,
    p_incorrect_contributor_floor bigint,
    p_observed_on date
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF p_published_question_id IS NULL OR p_revision_number IS NULL OR p_revision_number <= 0
       OR (p_normalized_credit IS NOT NULL AND p_normalized_credit NOT BETWEEN 0 AND 1)
       OR p_blank_contributor_floor IS NULL OR p_blank_contributor_floor < 0
       OR p_answered_contributor_floor IS NULL OR p_answered_contributor_floor < 0
       OR p_correct_contributor_floor IS NULL OR p_correct_contributor_floor < 0
       OR p_partial_contributor_floor IS NULL OR p_partial_contributor_floor < 0
       OR p_incorrect_contributor_floor IS NULL OR p_incorrect_contributor_floor < 0
       OR p_observed_on IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Revision outcome statistics are invalid';
    END IF;

    UPDATE ple_data.question_revision_statistics AS stats
       SET blank_count = stats.blank_count + CASE WHEN p_normalized_credit IS NULL THEN 1 ELSE 0 END,
           answered_count = stats.answered_count + CASE WHEN p_normalized_credit IS NULL THEN 0 ELSE 1 END,
           correct_count = stats.correct_count + CASE WHEN p_normalized_credit = 1 THEN 1 ELSE 0 END,
           partial_count = stats.partial_count + CASE WHEN p_normalized_credit > 0 AND p_normalized_credit < 1 THEN 1 ELSE 0 END,
           incorrect_count = stats.incorrect_count + CASE WHEN p_normalized_credit = 0 THEN 1 ELSE 0 END,
           blank_contributor_floor = greatest(stats.blank_contributor_floor,
               least(p_blank_contributor_floor, stats.blank_count + CASE WHEN p_normalized_credit IS NULL THEN 1 ELSE 0 END)),
           answered_contributor_floor = greatest(stats.answered_contributor_floor,
               least(p_answered_contributor_floor, stats.answered_count + CASE WHEN p_normalized_credit IS NULL THEN 0 ELSE 1 END)),
           correct_contributor_floor = greatest(stats.correct_contributor_floor,
               least(p_correct_contributor_floor, stats.correct_count + CASE WHEN p_normalized_credit = 1 THEN 1 ELSE 0 END)),
           partial_contributor_floor = greatest(stats.partial_contributor_floor,
               least(p_partial_contributor_floor, stats.partial_count + CASE WHEN p_normalized_credit > 0 AND p_normalized_credit < 1 THEN 1 ELSE 0 END)),
           incorrect_contributor_floor = greatest(stats.incorrect_contributor_floor,
               least(p_incorrect_contributor_floor, stats.incorrect_count + CASE WHEN p_normalized_credit = 0 THEN 1 ELSE 0 END)),
           credit_sum = stats.credit_sum + COALESCE(p_normalized_credit, 0),
           credit_sum_sq = stats.credit_sum_sq + COALESCE(p_normalized_credit * p_normalized_credit, 0),
           updated_on = greatest(stats.updated_on, p_observed_on)
     WHERE stats.published_question_id = p_published_question_id
       AND stats.revision_number = p_revision_number;
END
$$;

CREATE FUNCTION ple_data.record_question_pool_outcome(
    p_question_pool_id ple_data.question_family_id,
    p_normalized_credit numeric,
    p_observed_on date
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF p_question_pool_id IS NULL
       OR (p_normalized_credit IS NOT NULL AND p_normalized_credit NOT BETWEEN 0 AND 1)
       OR p_observed_on IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool outcome statistics are invalid';
    END IF;

    UPDATE ple_data.question_pool_statistics AS stats
       SET answered_count = stats.answered_count + CASE WHEN p_normalized_credit IS NULL THEN 0 ELSE 1 END,
           correct_count = stats.correct_count + CASE WHEN p_normalized_credit = 1 THEN 1 ELSE 0 END,
           partial_count = stats.partial_count + CASE WHEN p_normalized_credit > 0 AND p_normalized_credit < 1 THEN 1 ELSE 0 END,
           incorrect_count = stats.incorrect_count + CASE WHEN p_normalized_credit = 0 THEN 1 ELSE 0 END,
           credit_sum = stats.credit_sum + COALESCE(p_normalized_credit, 0),
           credit_sum_sq = stats.credit_sum_sq + COALESCE(p_normalized_credit * p_normalized_credit, 0),
           updated_on = greatest(stats.updated_on, p_observed_on)
     WHERE stats.question_pool_id = p_question_pool_id;
END
$$;

CREATE FUNCTION ple_data.record_question_pool_contributor_floors(
    p_question_pool_id ple_data.question_family_id,
    p_answered_contributor_floor bigint,
    p_correct_contributor_floor bigint,
    p_partial_contributor_floor bigint,
    p_incorrect_contributor_floor bigint
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF p_question_pool_id IS NULL
       OR p_answered_contributor_floor IS NULL OR p_answered_contributor_floor < 0
       OR p_correct_contributor_floor IS NULL OR p_correct_contributor_floor < 0
       OR p_partial_contributor_floor IS NULL OR p_partial_contributor_floor < 0
       OR p_incorrect_contributor_floor IS NULL OR p_incorrect_contributor_floor < 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool contributor floors are invalid';
    END IF;

    UPDATE ple_data.question_pool_statistics AS stats
       SET answered_contributor_floor = greatest(stats.answered_contributor_floor,
               least(p_answered_contributor_floor, stats.answered_count)),
           correct_contributor_floor = greatest(stats.correct_contributor_floor,
               least(p_correct_contributor_floor, stats.correct_count)),
           partial_contributor_floor = greatest(stats.partial_contributor_floor,
               least(p_partial_contributor_floor, stats.partial_count)),
           incorrect_contributor_floor = greatest(stats.incorrect_contributor_floor,
               least(p_incorrect_contributor_floor, stats.incorrect_count))
     WHERE stats.question_pool_id = p_question_pool_id;
END
$$;

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

-- The receipt is created at the first committed presentation and finalized
-- once after the Backend's stored result exists.
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

    IF p_assessment_submission_id IS NULL THEN
        INSERT INTO ple_private.question_statistics_observation_receipt (
            course_instance_id, issued_question_id, published_question_id,
            revision_number, question_pool_id, observed_at
        ) VALUES (
            p_course_instance_id, p_issued_question_id, v_question_id,
            v_revision_number, v_pool_id, p_observed_at
        ) ON CONFLICT (course_instance_id, issued_question_id) DO NOTHING
          RETURNING true INTO v_inserted;
        IF COALESCE(v_inserted, false) IS NOT TRUE THEN
            RETURN;
        END IF;

        SELECT array_agg(attempt.student_record_id), array_agg('issued'::text)
          INTO v_student_record_ids, v_cohorts
          FROM ple_private.question_statistics_observation_receipt AS receipt
          JOIN ple_private.issued_question AS issued USING (course_instance_id, issued_question_id)
          JOIN ple_private.assessment_attempt AS attempt
            ON attempt.course_instance_id = issued.course_instance_id
           AND attempt.assessment_attempt_id = issued.assessment_attempt_id
         WHERE receipt.published_question_id = v_question_id
           AND receipt.revision_number = v_revision_number;
        SELECT * INTO v_floors
          FROM ple_api.count_statistics_contributors(v_student_record_ids, v_cohorts);
        PERFORM ple_data.record_question_revision_delivery(
            v_question_id::ple_data.question_family_id,
            v_revision_number,
            v_floors.issued_floor,
            p_observed_at::date
        );
        IF v_pool_id IS NOT NULL THEN
            SELECT array_agg(attempt.student_record_id), array_agg('issued'::text)
              INTO v_student_record_ids, v_cohorts
              FROM ple_private.question_statistics_observation_receipt AS receipt
              JOIN ple_private.issued_question AS issued USING (course_instance_id, issued_question_id)
              JOIN ple_private.assessment_attempt AS attempt
                ON attempt.course_instance_id = issued.course_instance_id
               AND attempt.assessment_attempt_id = issued.assessment_attempt_id
             WHERE receipt.question_pool_id = v_pool_id;
            SELECT count_statistics.issued_floor INTO v_floors
              FROM ple_api.count_statistics_contributors(v_student_record_ids, v_cohorts) AS count_statistics;
            PERFORM ple_data.record_question_pool_delivery(
                v_pool_id::ple_data.question_family_id,
                v_floors.issued_floor,
                p_observed_at::date
            );
        END IF;
        RETURN;
    END IF;

    UPDATE ple_private.question_statistics_observation_receipt AS receipt
       SET assessment_submission_id = p_assessment_submission_id,
           observed_at = p_observed_at
     WHERE receipt.course_instance_id = p_course_instance_id
       AND receipt.issued_question_id = p_issued_question_id
       AND receipt.assessment_submission_id IS NULL
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
    UPDATE ple_private.question_statistics_observation_receipt
       SET normalized_credit = v_credit
     WHERE course_instance_id = p_course_instance_id
       AND issued_question_id = p_issued_question_id;
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
               WHEN receipt.assessment_submission_id IS NULL THEN 'issued'
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
    PERFORM ple_data.record_question_revision_outcome(
        v_question_id::ple_data.question_family_id,
        v_revision_number,
        v_credit,
        v_floors.blank_floor,
        v_floors.answered_floor,
        v_floors.correct_floor,
        v_floors.partial_floor,
        v_floors.incorrect_floor,
        p_observed_at::date
    );

    IF v_pool_id IS NOT NULL THEN
        PERFORM ple_data.record_question_pool_outcome(
            v_pool_id::ple_data.question_family_id,
            v_credit,
            p_observed_at::date
        );
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
        SELECT array_agg(attempt.student_record_id),
               array_agg(CASE
                   WHEN receipt.assessment_submission_id IS NULL THEN 'issued'
                   WHEN receipt.normalized_credit = 1 THEN 'correct'
                   WHEN receipt.normalized_credit > 0 THEN 'partial'
                   WHEN receipt.normalized_credit = 0 THEN 'incorrect'
                   ELSE 'blank'
               END)
          INTO v_student_record_ids, v_cohorts
          FROM ple_private.question_statistics_observation_receipt AS receipt
          JOIN ple_private.issued_question AS issued USING (course_instance_id, issued_question_id)
          JOIN ple_private.assessment_attempt AS attempt
            ON attempt.course_instance_id = issued.course_instance_id
           AND attempt.assessment_attempt_id = issued.assessment_attempt_id
         WHERE receipt.question_pool_id = v_pool_id;
        SELECT * INTO v_floors
          FROM ple_api.count_statistics_contributors(v_student_record_ids, v_cohorts);
        PERFORM ple_data.record_question_pool_contributor_floors(
            v_pool_id::ple_data.question_family_id,
            v_floors.answered_floor,
            v_floors.correct_floor,
            v_floors.partial_floor,
            v_floors.incorrect_floor
        );
    END IF;
END
$$;

SET LOCAL ROLE ple_data_owner;

-- Search attaches statistics for the current Revision only. Missing rows
-- contribute zeros so Instructors receive a stable Available shape.
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
       AND stats.revision_number = (
           SELECT max(revision.revision_number)
             FROM ple_data.question_revision AS revision
            WHERE revision.published_question_id = requested.published_question_id
       )
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

-- Pool statistics come from the Pool recorded on each delivery receipt.
-- Current Pool membership has no effect on this retained history.
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
    SELECT COALESCE(stats.issued_count, 0),
           COALESCE(stats.issued_contributor_floor, 0),
           COALESCE(stats.issued_count, 0),
           0::bigint,
           COALESCE(stats.answered_count, 0),
           COALESCE(stats.correct_count, 0),
           COALESCE(stats.partial_count, 0),
           COALESCE(stats.incorrect_count, 0),
           COALESCE(stats.issued_contributor_floor, 0),
           0::bigint,
           COALESCE(stats.answered_contributor_floor, 0),
           COALESCE(stats.correct_contributor_floor, 0),
           COALESCE(stats.partial_contributor_floor, 0),
           COALESCE(stats.incorrect_contributor_floor, 0),
           COALESCE(stats.credit_sum, 0),
           COALESCE(stats.credit_sum_sq, 0)
      FROM (SELECT p_question_pool_id::text AS question_pool_id) AS requested
      LEFT JOIN ple_data.question_pool_statistics AS stats
        ON stats.question_pool_id = requested.question_pool_id
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
