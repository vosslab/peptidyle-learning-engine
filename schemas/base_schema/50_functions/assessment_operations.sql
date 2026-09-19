-- Functions, triggers, and views from assessment_operations.sql.

SET LOCAL ROLE ple_api_owner;

-- Assessment API operations expose one current aggregate.  The relations and
-- guarded mutations live in assessments.sql; this module owns only the
-- session-authorized projections and API delegates.
CREATE FUNCTION ple_api.list_course_assessments(p_course_instance_id text)
RETURNS TABLE (
    assessment_id text,
    assessment_type text,
    assessment_title text,
    due_at_millis bigint,
    assessment_status text,
    assessment_edit_number bigint
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assessment.assessment_id,
           assessment.assessment_type,
           policy.assessment_title,
           CASE WHEN policy.due_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM policy.due_at) * 1000)::bigint END,
           assessment.assessment_status,
           assessment.assessment_edit_number
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_instance_id = course.course_instance_id
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE course.course_instance_id = p_course_instance_id
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
     ORDER BY policy.due_at NULLS LAST, assessment.assessment_id
$$;

CREATE FUNCTION ple_api.list_assessments_due_soon()
RETURNS TABLE (
    course_instance_id text,
    course_long_name text,
    assessment_id text,
    assessment_type text,
    assessment_title text,
    assessment_status text,
    due_at_millis bigint
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.course_instance_id,
           course.course_long_name,
           assessment.assessment_id,
           assessment.assessment_type,
           policy.assessment_title,
           assessment.assessment_status,
           floor(extract(epoch FROM policy.due_at) * 1000)::bigint
      FROM ple_data.assessment AS assessment
      JOIN ple_data.course_instance AS course ON course.course_instance_id = assessment.course_instance_id
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     -- ASVS 8.2.2 and 8.3.1: derive every returned Course from the current
     -- session's server-side Instructor authority.
     WHERE ple_api.current_session_account_is_course_instructor(course.course_instance_id)
       AND assessment.assessment_status IN ('unreleased', 'released')
       AND policy.due_at >= pg_catalog.statement_timestamp()
       AND policy.due_at < pg_catalog.statement_timestamp() + interval '7 days'
     ORDER BY policy.due_at, course.course_instance_id, assessment.assessment_id
$$;



-- Discovery returns one exact currently accepted Revision for every Available
-- Question lineage.  Assessment saves carry this Revision number, so a later
-- publication cannot silently change a selected Question.
CREATE FUNCTION ple_api.list_assessment_question_picker(p_course_instance_id text)
RETURNS TABLE (
    published_question_id text,
    question_revision_number integer,
    question_title text,
    question_description text,
    bloom_cognitive_process text,
    bloom_knowledge_dimension text,
    bloom_classification_edit_number bigint
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT lineage.published_question_id,
           accepted.revision_number,
           metadata.question_title,
           metadata.question_description,
           bloom.bloom_cognitive_process,
           bloom.bloom_knowledge_dimension,
           bloom.bloom_classification_edit_number
      FROM ple_data.course_instance AS course
      CROSS JOIN ple_data.published_question AS lineage
      JOIN LATERAL (
          SELECT revision.revision_number
            FROM ple_data.question_revision_acceptance AS revision
           WHERE revision.published_question_id = lineage.published_question_id
           ORDER BY revision.revision_number DESC
           LIMIT 1
      ) AS accepted ON true
      JOIN ple_data.question_revision AS revision
        ON revision.published_question_id = lineage.published_question_id
       AND revision.revision_number = accepted.revision_number
      JOIN ple_data.published_question_metadata AS metadata
        ON metadata.published_question_id = lineage.published_question_id
      JOIN LATERAL ple_private.question_library_entries(
          lineage.published_question_id, accepted.revision_number, true
      ) AS bloom ON true
     WHERE course.course_instance_id = p_course_instance_id
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
       AND lineage.availability = 'available'
       AND ple_private.question_backend_is_supported_for_production(revision.backend)
     ORDER BY metadata.question_title, lineage.published_question_id
$$;



-- A normalized row projection keeps pool membership explicit for the Store.
-- It resolves the exact pinned revision even after its lineage is Archived;
-- only ordinary picker discovery filters archived lineages.
CREATE FUNCTION ple_api.load_assessment_workspace_rows(
    p_course_instance_id text,
    p_assessment_id text
)
RETURNS TABLE (
    assessment_id text,
    assessment_edit_number bigint,
    assessment_status text,
    origin_kind text,
    source_blueprint_course_id text,
    source_blueprint_revision_number bigint,
    source_blueprint_assessment_id uuid,
    assessment_type text,
    assessment_title text,
    assessment_instructions text,
    available_at_millis bigint,
    due_at_millis bigint,
    closes_at_millis bigint,
    assessment_attempt_time_limit_seconds integer,
    attempt_limit integer,
    late_work_rule text,
    question_variation_rule text,
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
    question_pool_id text,
    question_pool_edit_number bigint,
    member_position integer,
    published_question_id text,
    question_revision_number integer,
    question_title text,
    question_description text,
    bloom_cognitive_process text,
    bloom_knowledge_dimension text,
    bloom_classification_edit_number bigint
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assessment.assessment_id,
           assessment.assessment_edit_number,
           assessment.assessment_status,
           assessment.origin_kind,
           blueprint.blueprint_course_id,
           assessment.source_blueprint_revision_number,
           assessment.source_blueprint_assessment_id,
           assessment.assessment_type,
           policy.assessment_title,
           policy.assessment_instructions,
           CASE WHEN policy.available_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM policy.available_at) * 1000)::bigint END,
           CASE WHEN policy.due_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM policy.due_at) * 1000)::bigint END,
           CASE WHEN policy.closes_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM policy.closes_at) * 1000)::bigint END,
           policy.assessment_attempt_time_limit_seconds,
           policy.assessment_attempt_limit,
           policy.late_work_rule,
           policy.question_variation_rule,
           policy.assessment_question_order_rule,
           policy.feedback_score,
           policy.feedback_per_item_correctness,
           policy.feedback_submitted_response,
           policy.feedback_question_answer,
           policy.feedback_question_answer_explanation,
           policy.feedback_class_statistics,
           entry.assessment_entry_id,
           entry.authored_position,
           entry.entry_kind,
           entry.availability,
           entry.scoring_rule,
           question.points_possible,
           pool_entry.selection_count,
           pool_entry.points_per_item,
           pool_entry.selected_question_order,
           entry.question_attempt_limit,
           entry.question_attempt_time_limit_seconds,
           entry.question_attempt_grace_seconds,
           pool_entry.question_pool_id,
           pool.question_pool_edit_number,
           item.member_position,
           COALESCE(item.published_question_id, question.published_question_id),
           COALESCE(item.question_revision_number, question.question_revision_number),
           metadata.question_title,
           metadata.question_description,
           question_bloom.bloom_cognitive_process,
           question_bloom.bloom_knowledge_dimension,
           question_bloom.bloom_classification_edit_number
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_instance_id = course.course_instance_id
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
      LEFT JOIN ple_data.blueprint_course AS blueprint
        ON blueprint.blueprint_course_id = assessment.source_blueprint_course_id
      -- ASVS 8.2.2 and 8.3.1: this Course-Instructor projection retains
      -- every exact Entry; availability remains a separate delivery gate.
      LEFT JOIN ple_data.assessment_entry AS entry
        ON entry.assessment_id = assessment.assessment_id
      LEFT JOIN ple_data.assessment_entry_question AS question
        ON question.assessment_entry_id = entry.assessment_entry_id
      LEFT JOIN ple_data.assessment_entry_pool AS pool_entry
        ON pool_entry.assessment_entry_id = entry.assessment_entry_id
      LEFT JOIN ple_data.question_pool AS pool ON pool.question_pool_id = pool_entry.question_pool_id
      LEFT JOIN ple_data.question_pool_member AS item
        ON item.question_pool_id = pool_entry.question_pool_id
      LEFT JOIN ple_data.published_question_metadata AS metadata
        ON metadata.published_question_id = COALESCE(item.published_question_id, question.published_question_id)
      LEFT JOIN LATERAL ple_private.question_library_entries(
          COALESCE(item.published_question_id, question.published_question_id, ''),
          COALESCE(item.question_revision_number, question.question_revision_number, 0),
          false
      ) AS question_bloom ON true
     WHERE course.course_instance_id = p_course_instance_id
       AND assessment.assessment_id = p_assessment_id
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
     ORDER BY entry.authored_position NULLS LAST, item.member_position NULLS LAST
$$;

CREATE FUNCTION ple_api.validate_assessment_release(
    p_course_instance_id text,
    p_assessment_id text
)
RETURNS TABLE (issue text)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    assessment_id_value text;
    assessment_status_value text;
BEGIN
    SELECT assessment.assessment_id, assessment.assessment_status
      INTO assessment_id_value, assessment_status_value
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_instance_id = course.course_instance_id
     WHERE course.course_instance_id = p_course_instance_id
       AND assessment.assessment_id = p_assessment_id
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id);
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
    p_assessment_id text,
    p_course_instance_id text,
    p_assessment_type text,
    p_title text,
    p_instructions text
)
RETURNS TABLE (
    assessment_id text,
    assessment_edit_number bigint,
    assessment_status text,
    assessment_title text,
    assessment_instructions text
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT result.assessment_id, result.assessment_edit_number,
           result.assessment_status, result.assessment_title, result.assessment_instructions
      FROM ple_data.create_assessment(
        p_assessment_id,
        (SELECT course.course_instance_id FROM ple_data.course_instance AS course
          WHERE course.course_instance_id = p_course_instance_id),
        p_assessment_type,
        p_title, p_instructions
    ) AS result
$$;

CREATE FUNCTION ple_api.save_assessment(
    p_course_instance_id text,
    p_assessment_id text,
    p_expected_edit_number bigint,
    p_values jsonb,
    p_entries jsonb
)
RETURNS TABLE (
    assessment_id text,
    assessment_edit_number bigint,
    assessment_status text,
    assessment_title text,
    assessment_instructions text
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assessment.assessment_id, result.assessment_edit_number,
           result.assessment_status, result.assessment_title, result.assessment_instructions
      FROM ple_data.save_assessment(
        (SELECT course.course_instance_id FROM ple_data.course_instance AS course
          WHERE course.course_instance_id = p_course_instance_id),
        (SELECT assessment.assessment_id FROM ple_data.course_instance AS course
          JOIN ple_data.assessment ON assessment.course_instance_id = course.course_instance_id
         WHERE course.course_instance_id = p_course_instance_id
           AND assessment.assessment_id = p_assessment_id), p_expected_edit_number,
        p_values, p_entries
    ) AS result
      JOIN ple_data.assessment ON assessment.assessment_id = result.assessment_id
$$;

CREATE FUNCTION ple_api.save_assessment_inline(
    p_course_instance_id text,
    p_assessment_id text,
    p_expected_edit_number bigint,
    p_title text,
    p_due_at timestamptz
)
RETURNS TABLE (
    assessment_id text,
    assessment_type text,
    assessment_title text,
    due_at_millis bigint,
    assessment_status text,
    assessment_edit_number bigint
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assessment.assessment_id, assessment.assessment_type,
           result.assessment_title, result.due_at_millis,
           result.assessment_status, result.assessment_edit_number
      FROM ple_data.save_assessment_inline(
        (SELECT course.course_instance_id FROM ple_data.course_instance AS course
          WHERE course.course_instance_id = p_course_instance_id),
        (SELECT assessment.assessment_id FROM ple_data.course_instance AS course
          JOIN ple_data.assessment ON assessment.course_instance_id = course.course_instance_id
         WHERE course.course_instance_id = p_course_instance_id
           AND assessment.assessment_id = p_assessment_id), p_expected_edit_number,
        p_title, p_due_at
    ) AS result
      JOIN ple_data.assessment ON assessment.assessment_id = result.assessment_id
$$;

CREATE FUNCTION ple_api.save_assessment_policies(
    p_course_instance_id text, p_assessment_id text,
    p_expected_edit_number bigint, p_policies jsonb
) RETURNS TABLE (
    assessment_id text, assessment_edit_number bigint,
    assessment_status text, assessment_title text, assessment_instructions text
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assessment.assessment_id, result.assessment_edit_number,
           result.assessment_status, result.assessment_title, result.assessment_instructions
      FROM ple_data.save_assessment_policies(
        (SELECT course.course_instance_id FROM ple_data.course_instance AS course
          WHERE course.course_instance_id = p_course_instance_id),
        (SELECT assessment.assessment_id FROM ple_data.course_instance AS course
          JOIN ple_data.assessment ON assessment.course_instance_id = course.course_instance_id
         WHERE course.course_instance_id = p_course_instance_id
           AND assessment.assessment_id = p_assessment_id),
        p_expected_edit_number, p_policies) AS result
      JOIN ple_data.assessment ON assessment.assessment_id = result.assessment_id
$$;

CREATE FUNCTION ple_api.release_assessment(
    p_course_instance_id text,
    p_assessment_id text,
    p_expected_edit_number bigint
)
RETURNS TABLE (
    assessment_id text,
    assessment_title text,
    assessment_status text,
    assessment_edit_number bigint
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assessment.assessment_id, result.assessment_title,
           result.assessment_status, result.assessment_edit_number
      FROM ple_data.release_assessment(
        (SELECT course.course_instance_id FROM ple_data.course_instance AS course
          WHERE course.course_instance_id = p_course_instance_id),
        (SELECT assessment.assessment_id FROM ple_data.course_instance AS course
          JOIN ple_data.assessment ON assessment.course_instance_id = course.course_instance_id
         WHERE course.course_instance_id = p_course_instance_id
           AND assessment.assessment_id = p_assessment_id),
        p_expected_edit_number
    ) AS result
      JOIN ple_data.assessment ON assessment.assessment_id = result.assessment_id
$$;

