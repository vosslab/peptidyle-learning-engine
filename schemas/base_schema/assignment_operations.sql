-- Assignment API operations expose one current aggregate.  The relations and
-- guarded mutations live in assignments.sql; this module owns only the
-- session-authorized projections and API delegates.

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_course_assignments(p_course_reference_number bigint)
RETURNS TABLE (
    assignment_reference_number bigint,
    assignment_title text,
    due_at_millis bigint,
    assignment_status text,
    assignment_edit_number bigint
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assignment.reference_number,
           assignment.assignment_title,
           CASE WHEN assignment.due_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM assignment.due_at) * 1000)::bigint END,
           assignment.assignment_status,
           assignment.assignment_edit_number
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE p_course_reference_number BETWEEN 1 AND 2147483647
       AND course.reference_number = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     ORDER BY assignment.due_at NULLS LAST, assignment.reference_number
$$;

CREATE FUNCTION ple_api.list_assignments_due_soon()
RETURNS TABLE (
    course_reference_number bigint,
    course_long_name text,
    assignment_reference_number bigint,
    assignment_title text,
    assignment_status text,
    due_at_millis bigint
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.reference_number,
           course.course_long_name,
           assignment.reference_number,
           assignment.assignment_title,
           assignment.assignment_status,
           floor(extract(epoch FROM assignment.due_at) * 1000)::bigint
      FROM ple_data.assignment AS assignment
      JOIN ple_data.course_instance AS course ON course.course_id = assignment.course_id
     WHERE ple_api.current_session_account_is_course_instructor(course.course_id)
       AND assignment.assignment_status IN ('unreleased', 'released')
       AND assignment.due_at >= pg_catalog.statement_timestamp()
       AND assignment.due_at < pg_catalog.statement_timestamp() + interval '7 days'
     ORDER BY assignment.due_at, course.reference_number, assignment.reference_number
$$;

-- Discovery returns one exact currently accepted Revision for every Available
-- Question lineage.  Assignment saves carry this Revision number, so a later
-- publication cannot silently change a selected Question.
CREATE FUNCTION ple_api.list_assignment_question_picker(p_course_reference_number bigint)
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
     WHERE p_course_reference_number BETWEEN 1 AND 2147483647
       AND course.reference_number = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
       AND lineage.availability = 'available'
     ORDER BY metadata.question_title, lineage.question_id
$$;

-- An Assignment begins from one stable member of the Course's already-pinned
-- immutable Blueprint Revision.  The label is read from that exact Revision
-- content; it is never reconstructed from a mutable Draft.  A later archive
-- changes Blueprint discovery, but cannot erase this Course provenance.
CREATE FUNCTION ple_api.list_course_assignment_source_choices(p_course_reference_number bigint)
RETURNS TABLE (
    source_blueprint_course_reference_number bigint,
    source_blueprint_revision_number bigint,
    source_blueprint_assignment_reference uuid,
    source_label text
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.blueprint_course_reference_number,
           course.blueprint_revision_number,
           source.blueprint_assignment_reference,
           source_content.assignment -> 'content' ->> 'title'
      FROM ple_data.course_instance AS course
      JOIN ple_data.blueprint_course_revision AS revision
        ON revision.blueprint_course_reference_number = course.blueprint_course_reference_number
       AND revision.blueprint_revision_number = course.blueprint_revision_number
      JOIN ple_data.blueprint_revision_assignment AS source
        ON source.blueprint_course_reference_number = revision.blueprint_course_reference_number
       AND source.blueprint_revision_number = revision.blueprint_revision_number
      JOIN ple_data.blueprint_revision_module AS module
        ON module.blueprint_course_reference_number = source.blueprint_course_reference_number
       AND module.blueprint_revision_number = source.blueprint_revision_number
       AND module.blueprint_module_reference = source.blueprint_module_reference
      JOIN LATERAL pg_catalog.jsonb_array_elements(revision.content -> 'modules')
        AS module_content(module) ON true
      JOIN LATERAL pg_catalog.jsonb_array_elements(module_content.module -> 'assignments')
        AS source_content(assignment)
        ON (source_content.assignment ->> 'blueprint_assignment_reference')::uuid
            = source.blueprint_assignment_reference
     WHERE p_course_reference_number BETWEEN 1 AND 2147483647
       AND course.reference_number = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     ORDER BY module.module_position, source.assignment_position
$$;

-- A normalized row projection keeps pool membership explicit for the Store.
-- It resolves the exact pinned revision even after its lineage is Archived;
-- only ordinary picker discovery filters archived lineages.
CREATE FUNCTION ple_api.load_assignment_workspace_rows(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint
)
RETURNS TABLE (
    assignment_reference_number bigint,
    assignment_edit_number bigint,
    assignment_status text,
    source_blueprint_course_reference_number bigint,
    source_blueprint_revision_number bigint,
    source_blueprint_assignment_reference uuid,
    assignment_title text,
    assignment_instructions text,
    available_at_millis bigint,
    due_at_millis bigint,
    closes_at_millis bigint,
    assignment_attempt_time_limit_seconds integer,
    attempt_limit integer,
    late_work_rule text,
    assignment_completion_rule text,
    assignment_completion_score_threshold numeric,
    assignment_attempt_grade_rule text,
    assignment_attempt_continuation_rule text,
    max_additional_assignment_attempts integer,
    question_pool_reuse_rule text,
    question_variation_rule text,
    assignment_attempt_resume_rule text,
    assignment_question_display_rule text,
    assignment_navigation_rule text,
    assignment_question_order_rule text,
    feedback_score text,
    feedback_per_item_correctness text,
    feedback_submitted_response text,
    feedback_question_feedback text,
    feedback_question_answer text,
    feedback_question_answer_explanation text,
    feedback_class_statistics text,
    assignment_entry_id uuid,
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
    question_pool_item_id uuid,
    item_position integer,
    item_availability text,
    question_id text,
    question_revision_number integer,
    question_title text,
    question_description text
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT assignment.reference_number,
           assignment.assignment_edit_number,
           assignment.assignment_status,
           assignment.source_blueprint_course_reference_number,
           assignment.source_blueprint_revision_number,
           assignment.source_blueprint_assignment_reference,
           assignment.assignment_title,
           assignment.assignment_instructions,
           CASE WHEN assignment.available_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM assignment.available_at) * 1000)::bigint END,
           CASE WHEN assignment.due_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM assignment.due_at) * 1000)::bigint END,
           CASE WHEN assignment.closes_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM assignment.closes_at) * 1000)::bigint END,
           assignment.assignment_attempt_time_limit_seconds,
           assignment.attempt_limit,
           assignment.late_work_rule,
           assignment.assignment_completion_rule,
           assignment.assignment_completion_score_threshold,
           assignment.assignment_attempt_grade_rule,
           assignment.assignment_attempt_continuation_rule,
           assignment.max_additional_assignment_attempts,
           assignment.question_pool_reuse_rule,
           assignment.question_variation_rule,
           assignment.assignment_attempt_resume_rule,
           assignment.assignment_question_display_rule,
           assignment.assignment_navigation_rule,
           assignment.assignment_question_order_rule,
           assignment.feedback_score,
           assignment.feedback_per_item_correctness,
           assignment.feedback_submitted_response,
           assignment.feedback_question_feedback,
           assignment.feedback_question_answer,
           assignment.feedback_question_answer_explanation,
           assignment.feedback_class_statistics,
           entry.assignment_entry_id,
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
           item.question_pool_item_id,
           item.item_position,
           item.availability,
           COALESCE(item.question_id, entry.question_id),
           COALESCE(item.question_revision_number, entry.question_revision_number),
           metadata.question_title,
           metadata.question_description
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
      LEFT JOIN ple_data.assignment_entry AS entry ON entry.assignment_id = assignment.assignment_id
      LEFT JOIN ple_data.question_pool_item AS item
        ON item.assignment_entry_id = entry.assignment_entry_id
      LEFT JOIN ple_data.published_question_metadata AS metadata
        ON metadata.question_id = COALESCE(item.question_id, entry.question_id)
     WHERE p_course_reference_number BETWEEN 1 AND 2147483647
       AND p_assignment_reference_number BETWEEN 1 AND 2147483647
       AND course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     ORDER BY entry.authored_position NULLS LAST, item.item_position NULLS LAST
$$;

CREATE FUNCTION ple_api.load_assignment_preview_rows(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint
)
RETURNS TABLE (
    assignment_title text,
    assignment_instructions text,
    assignment_entry_id uuid,
    authored_position integer,
    entry_kind text,
    question_id text,
    question_revision_number integer,
    question_title text,
    question_description text
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT workspace.assignment_title,
           workspace.assignment_instructions,
           workspace.assignment_entry_id,
           workspace.authored_position,
           workspace.entry_kind,
           workspace.question_id,
           workspace.question_revision_number,
           workspace.question_title,
           workspace.question_description
      FROM ple_api.load_assignment_workspace_rows(
          p_course_reference_number, p_assignment_reference_number
      ) AS workspace
     ORDER BY workspace.authored_position NULLS LAST, workspace.question_revision_number
$$;

CREATE FUNCTION ple_api.validate_assignment_release(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint
)
RETURNS TABLE (issue text)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE assignment_id_value uuid;
BEGIN
    SELECT assignment.assignment_id INTO assignment_id_value
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE p_course_reference_number BETWEEN 1 AND 2147483647
       AND p_assignment_reference_number BETWEEN 1 AND 2147483647
       AND course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.assignment_entry
         WHERE assignment_id = assignment_id_value AND availability = 'available'
    ) THEN
        issue := 'questions_required';
        RETURN NEXT;
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_data.assignment_entry AS entry
         WHERE entry.assignment_id = assignment_id_value
           AND entry.availability = 'available'
           AND entry.entry_kind = 'question_pool'
           AND entry.selection_count > (
               SELECT count(*) FROM ple_data.question_pool_item AS item
                WHERE item.assignment_entry_id = entry.assignment_entry_id
                  AND item.availability = 'available'
           )
    ) THEN
        issue := 'question_pool_insufficient_items';
        RETURN NEXT;
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_data.assignment
         WHERE assignment_id = assignment_id_value
           AND assignment_attempt_time_limit_seconds IS NULL
    ) THEN
        issue := 'attempt_time_limit_required';
        RETURN NEXT;
    END IF;
END
$$;

CREATE FUNCTION ple_api.create_assignment(
    p_assignment_id uuid,
    p_course_reference_number bigint,
    p_blueprint_assignment_reference uuid,
    p_title text,
    p_instructions text
)
RETURNS TABLE (
    assignment_reference_number bigint,
    assignment_edit_number bigint,
    assignment_status text,
    assignment_title text,
    assignment_instructions text
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.create_assignment(
        p_assignment_id, p_course_reference_number, p_blueprint_assignment_reference,
        p_title, p_instructions
    )
$$;

CREATE FUNCTION ple_api.save_assignment(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint,
    p_expected_edit_number bigint,
    p_values jsonb,
    p_entries jsonb
)
RETURNS TABLE (
    assignment_reference_number bigint,
    assignment_edit_number bigint,
    assignment_status text,
    assignment_title text,
    assignment_instructions text
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.save_assignment(
        p_course_reference_number, p_assignment_reference_number, p_expected_edit_number,
        p_values, p_entries
    )
$$;

CREATE FUNCTION ple_api.save_assignment_inline(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint,
    p_expected_edit_number bigint,
    p_title text,
    p_due_at timestamptz
)
RETURNS TABLE (
    assignment_reference_number bigint,
    assignment_title text,
    due_at_millis bigint,
    assignment_status text,
    assignment_edit_number bigint
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.save_assignment_inline(
        p_course_reference_number, p_assignment_reference_number, p_expected_edit_number,
        p_title, p_due_at
    )
$$;

CREATE FUNCTION ple_api.release_assignment(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint,
    p_expected_edit_number bigint
)
RETURNS TABLE (
    assignment_reference_number bigint,
    assignment_title text,
    assignment_status text,
    assignment_edit_number bigint
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.release_assignment(
        p_course_reference_number, p_assignment_reference_number, p_expected_edit_number
    )
$$;

REVOKE ALL ON FUNCTION ple_api.list_course_assignments(bigint),
    ple_api.list_assignments_due_soon(),
    ple_api.list_assignment_question_picker(bigint),
    ple_api.list_course_assignment_source_choices(bigint),
    ple_api.load_assignment_workspace_rows(bigint, bigint),
    ple_api.load_assignment_preview_rows(bigint, bigint),
    ple_api.validate_assignment_release(bigint, bigint),
    ple_api.create_assignment(uuid, bigint, uuid, text, text),
    ple_api.save_assignment(bigint, bigint, bigint, jsonb, jsonb),
    ple_api.save_assignment_inline(bigint, bigint, bigint, text, timestamptz),
    ple_api.release_assignment(bigint, bigint, bigint)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_course_assignments(bigint),
    ple_api.list_assignments_due_soon(),
    ple_api.list_assignment_question_picker(bigint),
    ple_api.list_course_assignment_source_choices(bigint),
    ple_api.load_assignment_workspace_rows(bigint, bigint),
    ple_api.load_assignment_preview_rows(bigint, bigint),
    ple_api.validate_assignment_release(bigint, bigint),
    ple_api.create_assignment(uuid, bigint, uuid, text, text),
    ple_api.save_assignment(bigint, bigint, bigint, jsonb, jsonb),
    ple_api.save_assignment_inline(bigint, bigint, bigint, text, timestamptz),
    ple_api.release_assignment(bigint, bigint, bigint)
    TO ple_app;

RESET ROLE;
