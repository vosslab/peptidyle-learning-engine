-- Assessment API operations expose one current aggregate.  The relations and
-- guarded mutations live in assessments.sql; this module owns only the
-- session-authorized projections and API delegates.

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_course_assessments(p_course_reference_number text)
RETURNS TABLE (
    assessment_reference_number text,
    assessment_type text,
    assessment_title text,
    due_at_millis bigint,
    assessment_status text,
    assessment_edit_number bigint
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assessment.public_reference,
           assessment.assessment_type,
           assessment.assessment_title,
           CASE WHEN assessment.due_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM assessment.due_at) * 1000)::bigint END,
           assessment.assessment_status,
           assessment.assessment_edit_number
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_id = course.course_id
     WHERE course.public_reference = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     ORDER BY assessment.due_at NULLS LAST, assessment.reference_number
$$;

CREATE FUNCTION ple_api.list_assessments_due_soon()
RETURNS TABLE (
    course_reference_number text,
    course_long_name text,
    assessment_reference_number text,
    assessment_type text,
    assessment_title text,
    assessment_status text,
    due_at_millis bigint
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.public_reference,
           course.course_long_name,
           assessment.public_reference,
           assessment.assessment_type,
           assessment.assessment_title,
           assessment.assessment_status,
           floor(extract(epoch FROM assessment.due_at) * 1000)::bigint
      FROM ple_data.assessment AS assessment
      JOIN ple_data.course_instance AS course ON course.course_id = assessment.course_id
     -- ASVS 8.2.2 and 8.3.1: derive every returned Course from the current
     -- session's server-side Instructor authority.
     WHERE ple_api.current_session_account_is_course_instructor(course.course_id)
       AND assessment.assessment_status IN ('unreleased', 'released')
       AND assessment.due_at >= pg_catalog.statement_timestamp()
       AND assessment.due_at < pg_catalog.statement_timestamp() + interval '7 days'
     ORDER BY assessment.due_at, course.reference_number, assessment.reference_number
$$;

-- Discovery returns one exact currently accepted Revision for every Available
-- Question lineage.  Assessment saves carry this Revision number, so a later
-- publication cannot silently change a selected Question.
CREATE FUNCTION ple_api.list_assessment_question_picker(p_course_reference_number text)
RETURNS TABLE (
    question_id text,
    question_revision_number integer,
    question_title text,
    question_description text
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT lineage.question_id,
           accepted.revision_number,
           metadata.question_title,
           metadata.question_description
      FROM ple_data.course_instance AS course
      CROSS JOIN ple_data.published_question AS lineage
      JOIN LATERAL (
          SELECT revision.revision_number
            FROM ple_data.question_revision_acceptance AS revision
           WHERE revision.question_id = lineage.question_id
           ORDER BY revision.revision_number DESC
           LIMIT 1
      ) AS accepted ON true
      JOIN ple_data.published_question_metadata AS metadata
        ON metadata.question_id = lineage.question_id
     WHERE course.public_reference = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
       AND lineage.availability = 'available'
     ORDER BY metadata.question_title, lineage.question_id
$$;

-- A normalized row projection keeps pool membership explicit for the Store.
-- It resolves the exact pinned revision even after its lineage is Archived;
-- only ordinary picker discovery filters archived lineages.
CREATE FUNCTION ple_api.load_assessment_workspace_rows(
    p_course_reference_number text,
    p_assessment_reference_number text
)
RETURNS TABLE (
    assessment_reference_number text,
    assessment_edit_number bigint,
    assessment_status text,
    origin_kind text,
    source_blueprint_course_reference_number text,
    source_blueprint_revision_number bigint,
    source_blueprint_assessment_reference uuid,
    assessment_type text,
    assessment_title text,
    assessment_instructions text,
    available_at_millis bigint,
    due_at_millis bigint,
    closes_at_millis bigint,
    assessment_attempt_time_limit_seconds integer,
    attempt_limit integer,
    late_work_rule text,
    assessment_attempt_grade_rule text,
    question_pool_reuse_rule text,
    question_variation_rule text,
    assessment_attempt_resume_rule text,
    assessment_question_display_rule text,
    assessment_navigation_rule text,
    assessment_question_order_rule text,
    feedback_score text,
    feedback_per_item_correctness text,
    feedback_submitted_response text,
    feedback_question_answer text,
    feedback_question_answer_explanation text,
    feedback_class_statistics text,
    assessment_entry_id uuid,
    authored_position integer,
    entry_kind text,
    entry_availability text,
    scoring_rule text,
    points_possible numeric,
    selection_count integer,
    points_per_item numeric,
    selected_question_order text,
    question_attempt_limit integer,
    question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer,
    question_pool_id uuid,
    question_pool_public_id text,
    question_pool_revision_number bigint,
    member_position integer,
    question_id text,
    question_revision_number integer,
    question_title text,
    question_description text
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assessment.public_reference,
           assessment.assessment_edit_number,
           assessment.assessment_status,
           assessment.origin_kind,
           blueprint.public_reference,
           assessment.source_blueprint_revision_number,
           assessment.source_blueprint_assessment_reference,
           assessment.assessment_type,
           assessment.assessment_title,
           assessment.assessment_instructions,
           CASE WHEN assessment.available_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM assessment.available_at) * 1000)::bigint END,
           CASE WHEN assessment.due_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM assessment.due_at) * 1000)::bigint END,
           CASE WHEN assessment.closes_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM assessment.closes_at) * 1000)::bigint END,
           assessment.assessment_attempt_time_limit_seconds,
           assessment.assessment_attempt_limit,
           assessment.late_work_rule,
           assessment.assessment_attempt_grade_rule,
           assessment.question_pool_reuse_rule,
           assessment.question_variation_rule,
           assessment.assessment_attempt_resume_rule,
           assessment.assessment_question_display_rule,
           assessment.assessment_navigation_rule,
           assessment.assessment_question_order_rule,
           assessment.feedback_score,
           assessment.feedback_per_item_correctness,
           assessment.feedback_submitted_response,
           assessment.feedback_question_answer,
           assessment.feedback_question_answer_explanation,
           assessment.feedback_class_statistics,
           entry.assessment_entry_id,
           entry.authored_position,
           entry.entry_kind,
           entry.availability,
           entry.scoring_rule,
           entry.points_possible,
           entry.selection_count,
           entry.points_per_item,
           entry.selected_question_order,
           entry.question_attempt_limit,
           entry.question_attempt_time_limit_seconds,
           entry.question_attempt_grace_seconds,
           entry.question_pool_id,
           pool.public_question_pool_id,
           entry.question_pool_revision_number,
           item.member_position,
           COALESCE(item.question_id, entry.question_id),
           COALESCE(item.question_revision_number, entry.question_revision_number),
           metadata.question_title,
           metadata.question_description
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_id = course.course_id
      LEFT JOIN ple_data.blueprint_course AS blueprint
        ON blueprint.reference_number = assessment.source_blueprint_course_reference_number
      LEFT JOIN ple_data.assessment_entry AS entry ON entry.assessment_id = assessment.assessment_id
      LEFT JOIN ple_data.question_pool AS pool ON pool.question_pool_id = entry.question_pool_id
      LEFT JOIN ple_data.question_pool_revision_member AS item
        ON item.question_pool_id = entry.question_pool_id
       AND item.revision_number = entry.question_pool_revision_number
      LEFT JOIN ple_data.published_question_metadata AS metadata
        ON metadata.question_id = COALESCE(item.question_id, entry.question_id)
     WHERE course.public_reference = p_course_reference_number
       AND assessment.public_reference = p_assessment_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     ORDER BY entry.authored_position NULLS LAST, item.member_position NULLS LAST
$$;

CREATE FUNCTION ple_api.validate_assessment_release(
    p_course_reference_number text,
    p_assessment_reference_number text
)
RETURNS TABLE (issue text)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    assessment_id_value uuid;
    assessment_status_value text;
BEGIN
    SELECT assessment.assessment_id, assessment.assessment_status
      INTO assessment_id_value, assessment_status_value
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_id = course.course_id
     WHERE course.public_reference = p_course_reference_number
       AND assessment.public_reference = p_assessment_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    -- ASVS 2.2.2 and 2.3.1-2.3.3: this authorized projection and every
    -- mutating hard gate consume the same closed issue-producing authority.
    RETURN QUERY
    SELECT release_issue.issue
      FROM ple_data.assessment_release_issues(
        assessment_id_value,
        transaction_timestamp(),
        assessment_status_value = 'unreleased'
      ) AS release_issue;
END
$$;

CREATE FUNCTION ple_api.create_assessment(
    p_assessment_id uuid,
    p_course_reference_number text,
    p_assessment_type text,
    p_title text,
    p_instructions text
)
RETURNS TABLE (
    assessment_reference_number text,
    assessment_edit_number bigint,
    assessment_status text,
    assessment_title text,
    assessment_instructions text
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT result.assessment_public_reference, result.assessment_edit_number,
           result.assessment_status, result.assessment_title, result.assessment_instructions
      FROM ple_data.create_assessment(
        p_assessment_id,
        (SELECT course.reference_number FROM ple_data.course_instance AS course
          WHERE course.public_reference = p_course_reference_number),
        p_assessment_type,
        p_title, p_instructions
    ) AS result
$$;

CREATE FUNCTION ple_api.save_assessment(
    p_course_reference_number text,
    p_assessment_reference_number text,
    p_expected_edit_number bigint,
    p_values jsonb,
    p_entries jsonb
)
RETURNS TABLE (
    assessment_reference_number text,
    assessment_edit_number bigint,
    assessment_status text,
    assessment_title text,
    assessment_instructions text
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assessment.public_reference, result.assessment_edit_number,
           result.assessment_status, result.assessment_title, result.assessment_instructions
      FROM ple_data.save_assessment(
        (SELECT course.reference_number FROM ple_data.course_instance AS course
          WHERE course.public_reference = p_course_reference_number),
        (SELECT assessment.reference_number FROM ple_data.course_instance AS course
          JOIN ple_data.assessment ON assessment.course_id = course.course_id
         WHERE course.public_reference = p_course_reference_number
           AND assessment.public_reference = p_assessment_reference_number), p_expected_edit_number,
        p_values, p_entries
    ) AS result
      JOIN ple_data.assessment ON assessment.reference_number = result.assessment_reference_number
$$;

CREATE FUNCTION ple_api.save_assessment_inline(
    p_course_reference_number text,
    p_assessment_reference_number text,
    p_expected_edit_number bigint,
    p_title text,
    p_due_at timestamptz
)
RETURNS TABLE (
    assessment_reference_number text,
    assessment_type text,
    assessment_title text,
    due_at_millis bigint,
    assessment_status text,
    assessment_edit_number bigint
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assessment.public_reference, assessment.assessment_type,
           result.assessment_title, result.due_at_millis,
           result.assessment_status, result.assessment_edit_number
      FROM ple_data.save_assessment_inline(
        (SELECT course.reference_number FROM ple_data.course_instance AS course
          WHERE course.public_reference = p_course_reference_number),
        (SELECT assessment.reference_number FROM ple_data.course_instance AS course
          JOIN ple_data.assessment ON assessment.course_id = course.course_id
         WHERE course.public_reference = p_course_reference_number
           AND assessment.public_reference = p_assessment_reference_number), p_expected_edit_number,
        p_title, p_due_at
    ) AS result
      JOIN ple_data.assessment ON assessment.reference_number = result.assessment_reference_number
$$;

CREATE FUNCTION ple_api.save_assessment_policies(
    p_course_reference_number text, p_assessment_reference_number text,
    p_expected_edit_number bigint, p_policies jsonb
) RETURNS TABLE (
    assessment_reference_number text, assessment_edit_number bigint,
    assessment_status text, assessment_title text, assessment_instructions text
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assessment.public_reference, result.assessment_edit_number,
           result.assessment_status, result.assessment_title, result.assessment_instructions
      FROM ple_data.save_assessment_policies(
        (SELECT course.reference_number FROM ple_data.course_instance AS course
          WHERE course.public_reference = p_course_reference_number),
        (SELECT assessment.reference_number FROM ple_data.course_instance AS course
          JOIN ple_data.assessment ON assessment.course_id = course.course_id
         WHERE course.public_reference = p_course_reference_number
           AND assessment.public_reference = p_assessment_reference_number),
        p_expected_edit_number, p_policies) AS result
      JOIN ple_data.assessment ON assessment.reference_number = result.assessment_reference_number
$$;

CREATE FUNCTION ple_api.release_assessment(
    p_course_reference_number text,
    p_assessment_reference_number text,
    p_expected_edit_number bigint
)
RETURNS TABLE (
    assessment_reference_number text,
    assessment_title text,
    assessment_status text,
    assessment_edit_number bigint
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assessment.public_reference, result.assessment_title,
           result.assessment_status, result.assessment_edit_number
      FROM ple_data.release_assessment(
        (SELECT course.reference_number FROM ple_data.course_instance AS course
          WHERE course.public_reference = p_course_reference_number),
        (SELECT assessment.reference_number FROM ple_data.course_instance AS course
          JOIN ple_data.assessment ON assessment.course_id = course.course_id
         WHERE course.public_reference = p_course_reference_number
           AND assessment.public_reference = p_assessment_reference_number),
        p_expected_edit_number
    ) AS result
      JOIN ple_data.assessment ON assessment.reference_number = result.assessment_reference_number
$$;

REVOKE ALL ON FUNCTION ple_api.list_course_assessments(text),
    ple_api.list_assessments_due_soon(),
    ple_api.list_assessment_question_picker(text),
    ple_api.load_assessment_workspace_rows(text, text),
    ple_api.validate_assessment_release(text, text),
    ple_api.create_assessment(uuid, text, text, text, text),
    ple_api.save_assessment(text, text, bigint, jsonb, jsonb),
    ple_api.save_assessment_inline(text, text, bigint, text, timestamptz),
    ple_api.save_assessment_policies(text, text, bigint, jsonb),
    ple_api.release_assessment(text, text, bigint)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_course_assessments(text),
    ple_api.list_assessments_due_soon(),
    ple_api.list_assessment_question_picker(text),
    ple_api.load_assessment_workspace_rows(text, text),
    ple_api.validate_assessment_release(text, text),
    ple_api.create_assessment(uuid, text, text, text, text),
    ple_api.save_assessment(text, text, bigint, jsonb, jsonb),
    ple_api.save_assessment_inline(text, text, bigint, text, timestamptz),
    ple_api.save_assessment_policies(text, text, bigint, jsonb),
    ple_api.release_assessment(text, text, bigint)
    TO ple_app;

RESET ROLE;
