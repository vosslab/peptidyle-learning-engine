-- Functions, triggers, and views from assessment_release_validation.sql.

SET LOCAL ROLE ple_data_owner;

-- Shared Assessment release validation and hard-gate authority.



-- Delivered current content, never whole Pool membership or a derived cache.
-- ASVS 8.3.1: trusted callers authorize the Assessment before using this
-- owner-only lookup; ple_app cannot execute it. A fixed owner also prevents
-- a nested cached SQL call from retaining an earlier caller's RLS policy.
CREATE FUNCTION ple_data.assessment_delivered_question_count(p_assessment_id text)
RETURNS bigint LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT COALESCE(sum(CASE entry_kind
        WHEN 'fixed_question' THEN 1 ELSE selection_count END), 0)::bigint
      FROM ple_data.assessment_entry
     WHERE assessment_id = p_assessment_id AND availability = 'available'
$$;



-- ASVS 2.2.1, 2.2.2, 2.3.2: one read/start authority resolves the finite base.
-- Empty drafts have no duration yet; valid starts must have 1..250 Questions.
CREATE FUNCTION ple_data.assessment_effective_base_duration_seconds(p_assessment_id text)
RETURNS integer LANGUAGE plpgsql STABLE
SET search_path = pg_catalog, ple_data AS $$
DECLARE authored_seconds integer;
DECLARE question_count bigint;
BEGIN
    SELECT assessment_attempt_time_limit_seconds INTO authored_seconds
      FROM ple_data.assessment WHERE assessment_id = p_assessment_id;
    question_count := ple_data.assessment_delivered_question_count(p_assessment_id);
    IF question_count NOT BETWEEN 1 AND 250 THEN
        RETURN NULL;
    END IF;
    RETURN COALESCE(authored_seconds, ((3 * question_count + 1) / 2 * 60)::integer);
END $$;



-- ASVS 2.1.2, 2.2.3, and 2.3.2: one deterministic trusted-layer helper
-- owns every release issue. Unreleased drafts may retain invalid dates so the
-- interactive projection can explain and correct them; released state always
-- passes the thin hard gate below.
CREATE FUNCTION ple_data.assessment_release_issues(
    p_assessment_id text,
    p_evaluated_at timestamptz,
    p_require_rolling_24_hours boolean
) RETURNS TABLE (issue text)
LANGUAGE plpgsql STABLE
SET search_path = pg_catalog, ple_data AS $$
DECLARE
    assessment_row record;
BEGIN
    SELECT assessment.*, course.active_until_at
      INTO assessment_row
      FROM ple_data.assessment AS assessment
      JOIN ple_data.course_instance AS course ON course.course_instance_id = assessment.course_instance_id
     WHERE assessment.assessment_id = p_assessment_id;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    IF ple_data.assessment_delivered_question_count(p_assessment_id) > 250 THEN
        issue := 'question_count_exceeded';
        RETURN NEXT;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.assessment_entry
         WHERE assessment_id = p_assessment_id AND availability = 'available'
    ) THEN
        issue := 'questions_required';
        RETURN NEXT;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM ple_data.assessment_entry AS entry
          JOIN ple_data.question_pool_revision AS pool_revision
            ON pool_revision.question_pool_id = entry.question_pool_id
           AND pool_revision.revision_number = entry.question_pool_revision_number
         WHERE entry.assessment_id = p_assessment_id
           AND entry.availability = 'available'
           AND entry.entry_kind = 'question_pool'
           AND entry.selection_count > pool_revision.member_count
    ) THEN
        issue := 'question_pool_insufficient_items';
        RETURN NEXT;
    END IF;
    IF assessment_row.due_at IS NULL THEN
        issue := 'due_date_required';
        RETURN NEXT;
    ELSE
        IF p_require_rolling_24_hours
           AND assessment_row.due_at < p_evaluated_at + interval '24 hours' THEN
            issue := 'due_date_less_than_24_hours_ahead';
            RETURN NEXT;
        END IF;
        IF assessment_row.due_at > assessment_row.active_until_at THEN
            issue := 'due_date_after_course_active_until';
            RETURN NEXT;
        END IF;
        IF assessment_row.available_at IS NOT NULL
           AND assessment_row.available_at > assessment_row.due_at THEN
            issue := 'availability_after_due_date';
            RETURN NEXT;
        END IF;
        IF assessment_row.closes_at IS NOT NULL
           AND assessment_row.due_at > assessment_row.closes_at THEN
            issue := 'due_date_after_close';
            RETURN NEXT;
        END IF;
    END IF;
END
$$;

CREATE FUNCTION ple_data.validate_assessment_release(
    p_assessment_id text,
    p_evaluated_at timestamptz,
    p_require_rolling_24_hours boolean
) RETURNS void LANGUAGE plpgsql STABLE
SET search_path = pg_catalog, ple_data AS $$
DECLARE first_issue text;
BEGIN
    SELECT issue INTO first_issue
      FROM ple_data.assessment_release_issues(
        p_assessment_id, p_evaluated_at, p_require_rolling_24_hours
      )
     LIMIT 1;
    IF first_issue IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment Release blocked: ' || first_issue;
    END IF;
END
$$;

