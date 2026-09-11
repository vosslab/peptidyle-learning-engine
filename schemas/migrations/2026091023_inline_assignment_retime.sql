-- Assignment retime preserves immutable delivery facts for active Attempts.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091023 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.assignment_attempt
    ADD COLUMN delivery_assignment_title text,
    ADD COLUMN delivery_due_at timestamp with time zone;

CREATE FUNCTION ple_private.assignment_attempt_delivery_is_immutable()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.delivery_assignment_title IS DISTINCT FROM OLD.delivery_assignment_title
       OR NEW.delivery_due_at IS DISTINCT FROM OLD.delivery_due_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assignment Attempt delivery facts are immutable';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER assignment_attempt_delivery_is_immutable
BEFORE UPDATE OF delivery_assignment_title, delivery_due_at
ON ple_private.assignment_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.assignment_attempt_delivery_is_immutable();

CREATE OR REPLACE FUNCTION ple_private.start_assignment_attempt_with_supplied_order(
    p_assignment_attempt_id uuid,
    p_student_record_id uuid,
    p_assignment_id uuid,
    p_question_pool_selections jsonb,
    p_issued_questions jsonb
)
RETURNS TABLE (
    assignment_attempt_id uuid,
    attempt_number integer,
    resumed boolean
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_course_id uuid;
    v_assignment_revision_id uuid;
    v_delivery_assignment_title text;
    v_delivery_due_at timestamp with time zone;
    v_attempt_number integer;
    v_existing_attempt_id uuid;
    v_existing_attempt_number integer;
    v_available_at timestamp with time zone;
    v_closes_at timestamp with time zone;
    v_attempt_limit integer;
    v_continuation_rule text;
    v_max_additional_attempts integer;
    v_question_pool_reuse_rule text;
    v_question_variation_rule text;
    v_completed_attempt_count integer;
    v_started_attempt_count integer;
    v_now timestamp with time zone := pg_catalog.clock_timestamp();
BEGIN
    IF jsonb_typeof(p_question_pool_selections) <> 'array'
       OR jsonb_typeof(p_issued_questions) <> 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assignment Attempt start requires Selection and Issued Question arrays';
    END IF;

    SELECT assignment.course_id, assignment.released_assignment_revision_id,
           assignment.assignment_title, assignment.due_at,
           revision.available_at, revision.closes_at, revision.attempt_limit,
           revision.assignment_attempt_continuation_rule,
           revision.max_additional_assignment_attempts,
           revision.question_pool_reuse_rule, revision.question_variation_rule
      INTO v_course_id, v_assignment_revision_id, v_delivery_assignment_title, v_delivery_due_at,
           v_available_at, v_closes_at,
           v_attempt_limit, v_continuation_rule, v_max_additional_attempts,
           v_question_pool_reuse_rule, v_question_variation_rule
      FROM ple_data.assignment AS assignment
      JOIN ple_data.assignment_revision AS revision
        ON revision.assignment_id = assignment.assignment_id
       AND revision.assignment_revision_id = assignment.released_assignment_revision_id
     WHERE assignment.assignment_id = p_assignment_id
       AND assignment.assignment_status = 'released'
     FOR UPDATE OF assignment;

    IF v_course_id IS NULL
       OR NOT ple_api.current_session_account_owns_student_record(
           v_course_id, p_student_record_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt start requires active Student ownership of a Released Assignment';
    END IF;
    IF (v_available_at IS NOT NULL AND v_now < v_available_at)
       OR (v_closes_at IS NOT NULL AND v_now >= v_closes_at) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt start is outside the Released Assignment availability window';
    END IF;

    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(p_student_record_id::text || ':' || p_assignment_id::text, 0)
    );
    SELECT existing.assignment_attempt_id, existing.attempt_number
      INTO v_existing_attempt_id, v_existing_attempt_number
      FROM ple_private.assignment_attempt AS existing
     WHERE existing.student_record_id = p_student_record_id
       AND existing.assignment_id = p_assignment_id
       AND existing.completed_at IS NULL
     ORDER BY existing.attempt_number DESC
     LIMIT 1
     FOR UPDATE;
    IF v_existing_attempt_id IS NOT NULL THEN
        RETURN QUERY SELECT v_existing_attempt_id, v_existing_attempt_number, true;
        RETURN;
    END IF;

    SELECT count(*)::integer,
           count(*) FILTER (WHERE completed_at IS NOT NULL)::integer
      INTO v_started_attempt_count, v_completed_attempt_count
      FROM ple_private.assignment_attempt
     WHERE student_record_id = p_student_record_id
       AND assignment_id = p_assignment_id;
    IF v_attempt_limit IS NOT NULL AND v_started_attempt_count >= v_attempt_limit THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt Limit does not allow another Attempt';
    END IF;
    IF v_completed_attempt_count > 0
       AND (
           v_continuation_rule = 'closed'
           OR (
               v_continuation_rule = 'capped'
               AND v_completed_attempt_count - 1 >= v_max_additional_attempts
           )
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt Continuation Rule does not allow another Attempt';
    END IF;

    SELECT COALESCE(max(existing.attempt_number), 0) + 1
      INTO v_attempt_number
      FROM ple_private.assignment_attempt AS existing
     WHERE existing.student_record_id = p_student_record_id
       AND existing.assignment_id = p_assignment_id;

    INSERT INTO ple_private.assignment_attempt (
        assignment_attempt_id, student_record_id, assignment_id, assignment_revision_id,
        started_at, completed_at, attempt_number, question_pool_reuse_rule,
        question_variation_rule, delivery_assignment_title, delivery_due_at
    ) VALUES (
        p_assignment_attempt_id, p_student_record_id, p_assignment_id, v_assignment_revision_id,
        v_now, NULL, v_attempt_number, v_question_pool_reuse_rule,
        v_question_variation_rule, v_delivery_assignment_title, v_delivery_due_at
    );

    IF EXISTS (
        SELECT 1
          FROM jsonb_to_recordset(p_question_pool_selections) AS input (
              question_pool_selection_id uuid,
              assignment_entry_id uuid,
              reused_from_question_pool_selection_id uuid,
              selected_items jsonb
          )
         WHERE jsonb_typeof(input.selected_items) <> 'array'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'each prepared Question Pool Selection requires an Item array';
    END IF;

    IF EXISTS (
        (SELECT question_pool.assignment_entry_id
           FROM ple_data.assignment_revision_question_pool AS question_pool
          WHERE question_pool.assignment_revision_id = v_assignment_revision_id
         EXCEPT
         SELECT input.assignment_entry_id
           FROM jsonb_to_recordset(p_question_pool_selections) AS input (
               question_pool_selection_id uuid,
               assignment_entry_id uuid,
               reused_from_question_pool_selection_id uuid,
               selected_items jsonb
           ))
        UNION ALL
        (SELECT input.assignment_entry_id
           FROM jsonb_to_recordset(p_question_pool_selections) AS input (
               question_pool_selection_id uuid,
               assignment_entry_id uuid,
               reused_from_question_pool_selection_id uuid,
               selected_items jsonb
           )
         EXCEPT
         SELECT question_pool.assignment_entry_id
           FROM ple_data.assignment_revision_question_pool AS question_pool
          WHERE question_pool.assignment_revision_id = v_assignment_revision_id)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'prepared Question Pool Selections must cover each exact Released Assignment Entry once';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM ple_data.assignment_revision_question_pool AS question_pool
          JOIN jsonb_to_recordset(p_question_pool_selections) AS input (
              question_pool_selection_id uuid,
              assignment_entry_id uuid,
              reused_from_question_pool_selection_id uuid,
              selected_items jsonb
          ) ON input.assignment_entry_id = question_pool.assignment_entry_id
         WHERE question_pool.assignment_revision_id = v_assignment_revision_id
           AND jsonb_array_length(input.selected_items) <> question_pool.selection_count
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'prepared Question Pool Selection count must match its Released Assignment Entry';
    END IF;

    INSERT INTO ple_private.question_pool_selection (
        question_pool_selection_id, assignment_attempt_id, assignment_entry_id,
        created_at, selected_question_count, reused_from_question_pool_selection_id
    )
    SELECT input.question_pool_selection_id, p_assignment_attempt_id, input.assignment_entry_id,
           v_now, jsonb_array_length(input.selected_items),
           input.reused_from_question_pool_selection_id
      FROM jsonb_to_recordset(p_question_pool_selections) AS input (
          question_pool_selection_id uuid,
          assignment_entry_id uuid,
          reused_from_question_pool_selection_id uuid,
          selected_items jsonb
      );

    INSERT INTO ple_private.question_pool_selected_item (
        question_pool_selection_id, question_pool_item_id, selection_position,
        question_id, revision_number
    )
    SELECT input.question_pool_selection_id, item.question_pool_item_id,
           item.selection_position, item.question_id, item.revision_number
      FROM jsonb_to_recordset(p_question_pool_selections) AS input (
          question_pool_selection_id uuid,
          assignment_entry_id uuid,
          reused_from_question_pool_selection_id uuid,
          selected_items jsonb
      )
      CROSS JOIN LATERAL jsonb_to_recordset(input.selected_items) AS item (
          question_pool_item_id uuid,
          selection_position integer,
          question_id text,
          revision_number integer
      );

    INSERT INTO ple_private.issued_question (
        issued_question_id, assignment_attempt_id, assignment_entry_id, question_id,
        revision_number, issued_position, point_value, scoring_rule, question_statistics_eligibility,
        question_pool_selection_id, question_pool_item_id
    )
    SELECT input.issued_question_id, p_assignment_attempt_id, input.assignment_entry_id,
           input.question_id, input.revision_number, input.issued_position,
           entry.point_value, entry.scoring_rule,
           entry.scoring_rule = 'normal' AND entry.point_value > 0,
           input.question_pool_selection_id, input.question_pool_item_id
      FROM jsonb_to_recordset(p_issued_questions) AS input (
          issued_question_id uuid,
          assignment_entry_id uuid,
          issued_position integer,
          question_id text,
          revision_number integer,
          question_pool_selection_id uuid,
          question_pool_item_id uuid
      )
      JOIN ple_data.assignment_revision_entry AS entry
        ON entry.assignment_revision_id = v_assignment_revision_id
       AND entry.assignment_entry_id = input.assignment_entry_id;

    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.issued_question AS issued_question
         WHERE issued_question.assignment_attempt_id = p_assignment_attempt_id
    ) OR EXISTS (
        WITH expected AS (
            SELECT fixed_question.assignment_entry_id, NULL::uuid AS question_pool_item_id,
                   fixed_question.question_id, fixed_question.revision_number
              FROM ple_data.assignment_revision_fixed_question AS fixed_question
             WHERE fixed_question.assignment_revision_id = v_assignment_revision_id
            UNION ALL
            SELECT selection.assignment_entry_id, item.question_pool_item_id,
                   item.question_id, item.revision_number
              FROM ple_private.question_pool_selection AS selection
              JOIN ple_private.question_pool_selected_item AS item
                ON item.question_pool_selection_id = selection.question_pool_selection_id
             WHERE selection.assignment_attempt_id = p_assignment_attempt_id
        ), actual AS (
            SELECT issued.assignment_entry_id, issued.question_pool_item_id,
                   issued.question_id, issued.revision_number
              FROM ple_private.issued_question AS issued
             WHERE issued.assignment_attempt_id = p_assignment_attempt_id
        )
        SELECT 1
          FROM (
              (SELECT * FROM expected EXCEPT SELECT * FROM actual)
              UNION ALL
              (SELECT * FROM actual EXCEPT SELECT * FROM expected)
          ) AS difference
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Issued Questions must cover the exact fixed pins and selected Question Pool Items';
    END IF;

    RETURN QUERY SELECT p_assignment_attempt_id, v_attempt_number, false;
END
$$;

RESET ROLE;

-- Preserve an Attempt's explicit no-deadline value during response saves.
SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
CREATE OR REPLACE FUNCTION ple_api.save_student_assignment_attempt_response(
    p_assignment_attempt_reference_number bigint, p_position integer, p_student_response jsonb
) RETURNS TABLE (assignment_attempt_reference_number bigint, issued_position integer,
    response_state text, saved_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_attempt ple_private.assignment_attempt%ROWTYPE;
    v_question_attempt ple_private.question_attempt%ROWTYPE;
    v_attempt_time_limit_seconds integer; v_due_at timestamp with time zone;
    v_late_work_rule text; v_now timestamp with time zone;
BEGIN
    IF p_assignment_attempt_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_position NOT BETWEEN 1 AND 2147483647
       OR jsonb_typeof(p_student_response) IS DISTINCT FROM 'object' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Student response save facts are invalid';
    END IF;
    SELECT attempt.* INTO v_attempt FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.student_record AS student ON student.student_record_id = attempt.student_record_id
      JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
      JOIN ple_private.account AS account ON account.account_id = student.student_account_id
     WHERE attempt.reference_number = p_assignment_attempt_reference_number AND attempt.completed_at IS NULL
       AND account.product_role = 'student' AND student.course_id = assignment.course_id
       AND student.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(assignment.course_id, student.student_record_id)
       AND EXISTS (SELECT 1 FROM ple_data.course_membership AS membership WHERE membership.course_id = assignment.course_id AND membership.account_id = student.student_account_id AND membership.role = 'student' AND ple_data.course_membership_is_active(membership.membership_id))
     FOR UPDATE OF attempt;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student response save is unavailable'; END IF;
    SELECT revision.assignment_attempt_time_limit_seconds,
           CASE WHEN v_attempt.delivery_assignment_title IS NOT NULL THEN v_attempt.delivery_due_at ELSE revision.due_at END,
           revision.late_work_rule INTO v_attempt_time_limit_seconds, v_due_at, v_late_work_rule
      FROM ple_data.assignment_revision AS revision WHERE revision.assignment_id = v_attempt.assignment_id AND revision.assignment_revision_id = v_attempt.assignment_revision_id;
    SELECT question_attempt.* INTO v_question_attempt FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
     WHERE issued.assignment_attempt_id = v_attempt.assignment_attempt_id AND issued.issued_position = p_position - 1 FOR UPDATE OF question_attempt;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student response save is unavailable'; END IF;
    v_now := clock_timestamp();
    IF v_question_attempt.question_attempt_state <> 'open'
       OR (v_attempt_time_limit_seconds IS NOT NULL AND v_attempt.started_at + v_attempt_time_limit_seconds * interval '1 second' <= v_now)
       OR (v_due_at IS NOT NULL AND v_late_work_rule = 'reject' AND v_now > v_due_at)
       OR (v_question_attempt.deadline_at IS NOT NULL AND v_question_attempt.deadline_at <= v_now) THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Student response cannot be saved';
    END IF;
    INSERT INTO ple_private.assignment_attempt_saved_response(question_attempt_id, student_response, saved_at)
      VALUES (v_question_attempt.question_attempt_id, p_student_response, v_now)
      ON CONFLICT (question_attempt_id) DO UPDATE SET student_response = EXCLUDED.student_response, saved_at = EXCLUDED.saved_at
      WHERE ple_private.assignment_attempt_saved_response.student_response IS DISTINCT FROM EXCLUDED.student_response;
    RETURN QUERY SELECT v_attempt.reference_number, p_position, 'saved'::text,
      COALESCE((SELECT saved_response.saved_at FROM ple_private.assignment_attempt_saved_response AS saved_response WHERE saved_response.question_attempt_id = v_question_attempt.question_attempt_id), v_now);
END
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.save_student_assignment_attempt_response(bigint, integer, jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.save_student_assignment_attempt_response(bigint, integer, jsonb) TO ple_app;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;

DROP FUNCTION ple_api.list_course_assignments(bigint);

CREATE OR REPLACE FUNCTION ple_api.list_course_assignments(p_course_reference_number bigint)
RETURNS TABLE (
    assignment_reference_number bigint,
    assignment_title text,
    due_at_millis bigint,
    assignment_status text,
    assignment_edit_number bigint
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_course_id uuid;
BEGIN
    SELECT course.course_id INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Assignments require a current Instructor Course Membership';
    END IF;
    RETURN QUERY SELECT assignment.reference_number, assignment.assignment_title,
        CASE WHEN assignment.due_at IS NULL THEN NULL
             ELSE floor(extract(epoch FROM assignment.due_at) * 1000)::bigint END,
        assignment.assignment_status, assignment.assignment_edit_number
      FROM ple_data.assignment AS assignment
     WHERE assignment.course_id = v_course_id
     ORDER BY assignment.assignment_title, assignment.reference_number;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_course_assignments(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_course_assignments(bigint) TO ple_app;

CREATE FUNCTION ple_api.save_live_demo_assignment_inline(
    p_course_reference_number bigint, p_assignment_reference_number bigint,
    p_expected_edit_number bigint, p_title text, p_due_at_millis bigint
) RETURNS TABLE (
    assignment_reference_number bigint, assignment_title text, due_at_millis bigint,
    assignment_status text, assignment_edit_number bigint
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    v_assignment ple_data.assignment%ROWTYPE;
    v_due_at timestamp with time zone;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_assignment_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_edit_number IS NULL OR p_expected_edit_number < 1
       OR p_title IS NULL OR p_title !~ '[^[:space:]]' OR char_length(p_title) > 200
       OR (p_due_at_millis IS NOT NULL
           AND p_due_at_millis NOT BETWEEN -62135596800000 AND 253402300799999) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Inline Assignment save is invalid';
    END IF;
    v_due_at := CASE
        WHEN p_due_at_millis IS NULL THEN NULL
        ELSE to_timestamp(p_due_at_millis::double precision / 1000)
    END;
    SELECT assignment.* INTO v_assignment
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     FOR UPDATE OF assignment;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    IF v_assignment.assignment_edit_number <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assignment Edit Number changed';
    END IF;
    IF v_assignment.assignment_status NOT IN ('unreleased', 'released') THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    IF v_assignment.assignment_title IS DISTINCT FROM p_title
       OR v_assignment.due_at IS DISTINCT FROM v_due_at THEN
        UPDATE ple_data.assignment AS updated_assignment
           SET assignment_title = p_title,
            due_at = v_due_at,
            assignment_edit_number = updated_assignment.assignment_edit_number + 1,
            updated_at = pg_catalog.clock_timestamp()
         WHERE updated_assignment.assignment_id = v_assignment.assignment_id
         RETURNING updated_assignment.* INTO v_assignment;
    END IF;
    RETURN QUERY SELECT v_assignment.reference_number, v_assignment.assignment_title,
        CASE WHEN v_assignment.due_at IS NULL THEN NULL
             ELSE floor(extract(epoch FROM v_assignment.due_at) * 1000)::bigint END,
        v_assignment.assignment_status, v_assignment.assignment_edit_number;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.save_live_demo_assignment_inline(bigint,bigint,bigint,text,bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.save_live_demo_assignment_inline(bigint,bigint,bigint,text,bigint) TO ple_app;

CREATE OR REPLACE FUNCTION ple_api.live_demo_assignment_access(
    p_course_reference_number bigint, p_assignment_reference_number bigint
) RETURNS TABLE (start_decision text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_assignment ple_data.assignment%ROWTYPE; v_student_record_id uuid;
    v_revision ple_data.assignment_revision%ROWTYPE; v_active_attempt ple_private.assignment_attempt%ROWTYPE;
    v_started_attempt_count integer; v_now timestamp with time zone := clock_timestamp();
    v_due_at timestamp with time zone;
BEGIN
    SELECT assignment.* INTO v_assignment FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number AND assignment.reference_number = p_assignment_reference_number;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable'; END IF;
    SELECT student_record.student_record_id INTO v_student_record_id FROM ple_data.student_record AS student_record
     WHERE student_record.course_id = v_assignment.course_id AND student_record.student_account_id = ple_api.current_session_account_id();
    IF NOT FOUND OR NOT ple_api.current_session_account_owns_student_record(v_assignment.course_id, v_student_record_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    SELECT attempt.* INTO v_active_attempt FROM ple_private.assignment_attempt AS attempt
     WHERE attempt.student_record_id = v_student_record_id AND attempt.assignment_id = v_assignment.assignment_id AND attempt.completed_at IS NULL
     ORDER BY attempt.attempt_number DESC LIMIT 1;
    SELECT revision.* INTO v_revision FROM ple_data.assignment_revision AS revision
     WHERE revision.assignment_revision_id = CASE WHEN v_active_attempt.assignment_attempt_id IS NULL
         THEN v_assignment.released_assignment_revision_id ELSE v_active_attempt.assignment_revision_id END;
    IF v_assignment.assignment_status <> 'released' OR NOT FOUND THEN start_decision := 'closed'; RETURN NEXT; RETURN; END IF;
    IF v_revision.available_at IS NOT NULL AND v_now < v_revision.available_at THEN start_decision := 'not_yet_available'; RETURN NEXT; RETURN; END IF;
    IF v_revision.closes_at IS NOT NULL AND v_now >= v_revision.closes_at THEN start_decision := 'closed'; RETURN NEXT; RETURN; END IF;
    v_due_at := CASE WHEN v_active_attempt.assignment_attempt_id IS NULL THEN v_assignment.due_at
      WHEN v_active_attempt.delivery_assignment_title IS NOT NULL THEN v_active_attempt.delivery_due_at ELSE v_revision.due_at END;
    IF v_due_at IS NOT NULL AND v_now > v_due_at AND v_revision.late_work_rule = 'reject' THEN start_decision := 'late_work_refused'; RETURN NEXT; RETURN; END IF;
    IF v_active_attempt.assignment_attempt_id IS NOT NULL THEN start_decision := 'may_start'; RETURN NEXT; RETURN; END IF;
    SELECT count(*)::integer INTO v_started_attempt_count FROM ple_private.assignment_attempt AS attempt
     WHERE attempt.student_record_id = v_student_record_id AND attempt.assignment_id = v_assignment.assignment_id;
    start_decision := CASE WHEN v_revision.attempt_limit IS NOT NULL AND v_started_attempt_count >= v_revision.attempt_limit THEN 'attempt_limit_reached' ELSE 'may_start' END;
    RETURN NEXT;
END
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.live_demo_assignment_access(bigint,bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.live_demo_assignment_access(bigint,bigint) TO ple_app;

RESET ROLE;
SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;

CREATE OR REPLACE FUNCTION ple_api.read_student_assignment_attempt_context(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (assignment_attempt_reference_number bigint, attempt_number integer,
    course_reference_number bigint, course_short_name text, course_long_name text, course_theme text,
    assignment_reference_number bigint, assignment_title text, timer_remaining_milliseconds bigint)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    WITH evaluated_at AS (SELECT pg_catalog.statement_timestamp() AS value)
    SELECT attempt.reference_number, attempt.attempt_number, course.reference_number,
        course.course_short_name, course.course_long_name, course.course_theme, assignment.reference_number,
        CASE WHEN attempt.delivery_assignment_title IS NOT NULL
             THEN attempt.delivery_assignment_title ELSE revision.assignment_title END,
        CASE WHEN revision.assignment_attempt_time_limit_seconds IS NULL THEN NULL ELSE GREATEST(0::bigint,
            floor(extract(epoch FROM (attempt.started_at + revision.assignment_attempt_time_limit_seconds * interval '1 second' - evaluated_at.value)) * 1000)::bigint) END
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.student_record AS student ON student.student_record_id = attempt.student_record_id
      JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
      JOIN ple_data.assignment_revision AS revision ON revision.assignment_id = attempt.assignment_id AND revision.assignment_revision_id = attempt.assignment_revision_id
      JOIN ple_data.course_instance AS course ON course.course_id = assignment.course_id
      CROSS JOIN evaluated_at
     WHERE p_assignment_attempt_reference_number BETWEEN 1 AND 2147483647
       AND attempt.reference_number = p_assignment_attempt_reference_number
       AND (attempt.completed_at IS NULL OR EXISTS (SELECT 1 FROM ple_private.assignment_submission AS submission WHERE submission.assignment_attempt_id = attempt.assignment_attempt_id))
       AND student.course_id = assignment.course_id AND student.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(assignment.course_id, student.student_record_id)
       AND EXISTS (SELECT 1 FROM ple_data.course_membership AS membership WHERE membership.course_id = assignment.course_id AND membership.account_id = student.student_account_id AND membership.role = 'student' AND ple_data.course_membership_is_active(membership.membership_id))
$$;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
