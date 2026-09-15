-- Shared Assessment release validation and hard-gate authority.

SET LOCAL ROLE ple_data_owner;

-- ASVS 2.1.2, 2.2.3, and 2.3.2: one deterministic trusted-layer helper
-- owns every release issue. Unreleased drafts may retain invalid dates so the
-- interactive projection can explain and correct them; released state always
-- passes the thin hard gate below.
CREATE FUNCTION ple_data.assessment_release_issues(
    p_assessment_id uuid,
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
      JOIN ple_data.course_instance AS course ON course.course_id = assessment.course_id
     WHERE assessment.assessment_id = p_assessment_id;
    IF NOT FOUND THEN
        RETURN;
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
    IF assessment_row.assessment_attempt_time_limit_seconds IS NULL THEN
        issue := 'assessment_attempt_time_limit_required';
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
    p_assessment_id uuid,
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

REVOKE ALL ON FUNCTION
    ple_data.assessment_release_issues(uuid, timestamptz, boolean),
    ple_data.validate_assessment_release(uuid, timestamptz, boolean)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_data.assessment_release_issues(uuid, timestamptz, boolean),
    ple_data.validate_assessment_release(uuid, timestamptz, boolean)
    TO ple_api_owner;

RESET ROLE;
