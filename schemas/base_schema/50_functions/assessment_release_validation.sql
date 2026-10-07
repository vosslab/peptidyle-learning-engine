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
    SELECT COALESCE(sum(CASE entry.entry_kind
        WHEN 'fixed_question' THEN 1 ELSE pool_entry.selection_count END), 0)::bigint
      FROM ple_data.assessment_entry AS entry
      LEFT JOIN ple_data.assessment_entry_pool AS pool_entry
        ON pool_entry.assessment_entry_id = entry.assessment_entry_id
     WHERE entry.assessment_id = p_assessment_id AND entry.availability = 'available'
$$;



-- ASVS 2.2.1, 2.2.2, 2.3.2: one read/start authority resolves the finite base.
-- Empty drafts have no duration yet; valid starts must have 1..250 Questions.
CREATE FUNCTION ple_data.assessment_effective_base_duration_seconds(p_assessment_id text)
RETURNS integer LANGUAGE plpgsql STABLE
SET search_path = pg_catalog, ple_data AS $$
DECLARE authored_seconds integer;
DECLARE question_count bigint;
BEGIN
    SELECT policy.assessment_attempt_time_limit_seconds INTO authored_seconds
      FROM ple_data.assessment AS assessment
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE assessment.assessment_id = p_assessment_id;
    question_count := ple_data.assessment_delivered_question_count(p_assessment_id);
    IF question_count NOT BETWEEN 1 AND 250 THEN
        RETURN NULL;
    END IF;
    RETURN COALESCE(authored_seconds, ((3 * question_count + 1) / 2 * 60)::integer);
END $$;

-- Keep the Pool-specific rule evidence available to trusted release checks.
-- The public release issue projection retains its stable string-only contract.
CREATE FUNCTION ple_data.assessment_pool_release_issues(p_assessment_id text)
RETURNS TABLE (assessment_entry_id uuid, question_pool_id text, issue text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_private, ple_data AS $$
    WITH pool_members AS (
        SELECT entry.assessment_entry_id,
               pool_entry.question_pool_id::text AS question_pool_id,
               pool_entry.selection_count,
               pool.question_type AS pool_question_type,
               pool.backend AS pool_backend,
               pool.content_discipline_id AS pool_discipline_id,
               pool.content_subject_id AS pool_subject_id,
               member.published_question_id,
               member.question_revision_number,
               revision.backend AS member_backend,
               metadata.question_type AS member_question_type,
               metadata.content_discipline_id AS member_discipline_id,
               metadata.content_subject_id AS member_subject_id,
               metadata.published_question_id AS metadata_question_id,
               ple_private.question_backend_is_supported_for_production(revision.backend)
                   AS member_backend_supported
          FROM ple_data.assessment_entry AS entry
          JOIN ple_data.assessment_entry_pool AS pool_entry
            ON pool_entry.assessment_entry_id = entry.assessment_entry_id
          JOIN ple_data.question_pool AS pool
            ON pool.question_pool_id = pool_entry.question_pool_id
          LEFT JOIN ple_data.question_pool_member AS member
            ON member.question_pool_id = pool.question_pool_id
          LEFT JOIN ple_data.question_revision AS revision
            ON revision.published_question_id = member.published_question_id
           AND revision.revision_number = member.question_revision_number
          LEFT JOIN ple_data.question_revision_metadata AS metadata
            ON metadata.published_question_id = member.published_question_id
           AND metadata.revision_number = member.question_revision_number
         WHERE entry.assessment_id = p_assessment_id
           AND entry.availability = 'available'
           AND entry.entry_kind = 'question_pool'
    ),
    classified_members AS (
        SELECT pool_members.*,
               (metadata_question_id IS NOT NULL
                AND member_backend_supported
                AND member_backend = pool_backend
                AND member_question_type = pool_question_type
                AND member_discipline_id = pool_discipline_id
                AND member_subject_id = pool_subject_id) AS member_is_valid
          FROM pool_members
    ),
    pool_counts AS (
        -- One Pool may occur in multiple Assessment entries with different
        -- selection counts, so validate each request independently.
        SELECT assessment_entry_id, question_pool_id,
               max(selection_count) AS selection_count,
               count(*) FILTER (WHERE published_question_id IS NOT NULL
                                AND member_is_valid)::bigint AS valid_member_count
          FROM classified_members
         GROUP BY assessment_entry_id, question_pool_id
    ),
    member_rules AS (
        SELECT DISTINCT assessment_entry_id, question_pool_id, rule.issue
          FROM classified_members
          CROSS JOIN LATERAL (VALUES
              ('question_pool_member_unavailable',
               metadata_question_id IS NULL),
              ('question_pool_member_backend_mismatch',
               member_backend IS DISTINCT FROM pool_backend OR NOT COALESCE(member_backend_supported, false)),
              ('question_pool_member_type_mismatch',
               metadata_question_id IS NOT NULL
               AND member_question_type IS DISTINCT FROM pool_question_type),
              ('question_pool_member_classification_mismatch',
               metadata_question_id IS NOT NULL
               AND (member_discipline_id IS DISTINCT FROM pool_discipline_id
                    OR member_subject_id IS DISTINCT FROM pool_subject_id))
          ) AS rule(issue, violated)
         WHERE published_question_id IS NOT NULL AND rule.violated
    )
    SELECT member_rules.assessment_entry_id,
           member_rules.question_pool_id, member_rules.issue
      FROM member_rules
    UNION
    SELECT pool_counts.assessment_entry_id,
           pool_counts.question_pool_id, 'question_pool_insufficient_items'
      FROM pool_counts
     WHERE pool_counts.valid_member_count < pool_counts.selection_count
    ORDER BY 1, 2, 3
$$;



-- ASVS 2.1.2, 2.2.3, and 2.3.2: schedule consistency is shared by release
-- and edits to an already released Assessment. Content readiness is required
-- only when releasing; later permitted removals must not be blocked by an
-- empty or insufficient remaining Pool.
CREATE FUNCTION ple_data.assessment_schedule_issues(
    p_assessment_id text,
    p_evaluated_at timestamptz,
    p_require_rolling_24_hours boolean
) RETURNS TABLE (issue text)
LANGUAGE plpgsql STABLE
SET search_path = pg_catalog, ple_data AS $$
DECLARE
    assessment_row record;
BEGIN
    SELECT assessment.assessment_id, course.active_until_at,
           policy.due_at, policy.available_at, policy.closes_at
      INTO assessment_row
      FROM ple_data.assessment AS assessment
      JOIN ple_data.course_instance AS course ON course.course_instance_id = assessment.course_instance_id
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE assessment.assessment_id = p_assessment_id;
    IF NOT FOUND THEN
        RETURN;
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

-- ASVS 2.1.2, 2.2.3, and 2.3.2: the release issue projection also requires
-- deliverable content, while unreleased drafts can retain problems for repair.
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
    SELECT assessment.assessment_id
      INTO assessment_row
      FROM ple_data.assessment AS assessment
      JOIN ple_data.course_instance AS course ON course.course_instance_id = assessment.course_instance_id
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
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
          FROM ple_data.assessment_pool_release_issues(p_assessment_id)
    ) THEN
        issue := 'question_pool_insufficient_items';
        RETURN NEXT;
    END IF;
    RETURN QUERY
        SELECT schedule_issue.issue
          FROM ple_data.assessment_schedule_issues(
              p_assessment_id, p_evaluated_at, p_require_rolling_24_hours
          ) AS schedule_issue;
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

-- ASVS 2.2.2 and 2.3.3: permitted writes to an issued Assessment preserve
-- its schedule contract atomically while allowing its remaining content to
-- reflect the Instructor's post-issue removals.
CREATE FUNCTION ple_data.validate_assessment_post_release_edit(
    p_assessment_id text,
    p_evaluated_at timestamptz,
    p_require_rolling_24_hours boolean
) RETURNS void LANGUAGE plpgsql STABLE
SET search_path = pg_catalog, ple_data AS $$
DECLARE first_issue text;
BEGIN
    SELECT issue INTO first_issue
      FROM ple_data.assessment_schedule_issues(
          p_assessment_id, p_evaluated_at, p_require_rolling_24_hours
      )
     LIMIT 1;
    IF first_issue IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment Release blocked: ' || first_issue;
    END IF;
END
$$;
