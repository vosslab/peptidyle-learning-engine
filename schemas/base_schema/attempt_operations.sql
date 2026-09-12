-- Assignment-first Student Work operations.  Delivery owns creation of a
-- Question Attempt's backend-specific reproduction bundle; this module owns
-- current Assignment selection, response persistence, and finalization.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.lock_assignment_for_student_work(p_assignment_id uuid)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    PERFORM 1 FROM ple_data.assignment
     WHERE assignment_id = p_assignment_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Student Work operation is unavailable';
    END IF;
END $$;

CREATE FUNCTION ple_private.assert_current_student_attempt(
    p_assignment_attempt_reference_number bigint
) RETURNS ple_private.assignment_attempt LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE result ple_private.assignment_attempt%ROWTYPE;
DECLARE course_id_value uuid;
BEGIN
    SELECT attempt.* INTO result
      FROM ple_private.assignment_attempt AS attempt
     WHERE attempt.reference_number = p_assignment_attempt_reference_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt is unavailable';
    END IF;
    SELECT assignment.course_id INTO course_id_value
      FROM ple_data.assignment AS assignment
     WHERE assignment.assignment_id = result.assignment_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_owns_student_record(
           course_id_value, result.student_record_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt is unavailable';
    END IF;
    RETURN result;
END $$;

CREATE FUNCTION ple_private.start_assignment_attempt(
    p_assignment_attempt_id uuid,
    p_student_record_id uuid,
    p_assignment_id uuid,
    p_selections jsonb,
    p_issued_questions jsonb
) RETURNS TABLE (assignment_attempt_id uuid, attempt_number integer, resumed boolean)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assignment_row ple_data.assignment%ROWTYPE;
DECLARE existing_attempt ple_private.assignment_attempt%ROWTYPE;
DECLARE account_id uuid := ple_api.current_session_account_id();
DECLARE now_value timestamptz := pg_catalog.clock_timestamp();
DECLARE next_attempt_number integer;
DECLARE selection jsonb;
DECLARE issued jsonb;
DECLARE entry_row ple_data.assignment_entry%ROWTYPE;
DECLARE accommodation_row ple_private.student_assignment_accommodation%ROWTYPE;
DECLARE selection_id uuid;
DECLARE selection_entry_id uuid;
DECLARE issued_entry_id uuid;
DECLARE issued_selection_id uuid;
DECLARE issued_item_id uuid;
BEGIN
    IF p_assignment_attempt_id IS NULL OR p_student_record_id IS NULL OR p_assignment_id IS NULL
       OR jsonb_typeof(p_selections) <> 'array' OR jsonb_typeof(p_issued_questions) <> 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assignment Attempt start arguments are invalid';
    END IF;

    -- Every Student Work mutator acquires this Assignment row first.  The
    -- guarded Unrelease procedure uses the identical first lock.
    PERFORM ple_private.lock_assignment_for_student_work(p_assignment_id);
    SELECT * INTO assignment_row FROM ple_data.assignment WHERE assignment_id = p_assignment_id;
    IF NOT FOUND OR assignment_row.assignment_status <> 'released'
       OR account_id IS NULL
       OR NOT ple_api.current_session_account_owns_student_record(
           assignment_row.course_id, p_student_record_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment Attempt start is unavailable';
    END IF;
    SELECT * INTO accommodation_row
      FROM ple_private.student_assignment_accommodation
     WHERE student_record_id = p_student_record_id AND assignment_id = p_assignment_id;
    IF COALESCE(accommodation_row.available_at, assignment_row.available_at) > now_value
       OR COALESCE(accommodation_row.closes_at, assignment_row.closes_at) < now_value
       OR (assignment_row.late_work_rule = 'reject'
           AND COALESCE(accommodation_row.due_at, assignment_row.due_at) IS NOT NULL
           AND now_value > COALESCE(accommodation_row.due_at, assignment_row.due_at)) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Assignment Attempt start is outside its effective availability';
    END IF;

    SELECT * INTO existing_attempt FROM ple_private.assignment_attempt
     WHERE student_record_id = p_student_record_id AND assignment_id = p_assignment_id
       AND completed_at IS NULL
     ORDER BY attempt_number DESC LIMIT 1;
    -- Resume is interpretation of an existing Attempt, so it follows the
    -- retained Attempt rule rather than a later released Assignment edit.
    IF FOUND AND existing_attempt.assignment_attempt_resume_rule = 'resumable' THEN
        assignment_attempt_id := existing_attempt.assignment_attempt_id;
        attempt_number := existing_attempt.attempt_number;
        resumed := true;
        RETURN NEXT;
        RETURN;
    END IF;
    SELECT COALESCE(max(existing_assignment_attempt.attempt_number), 0) + 1 INTO next_attempt_number
      FROM ple_private.assignment_attempt AS existing_assignment_attempt
     WHERE existing_assignment_attempt.student_record_id = p_student_record_id
       AND existing_assignment_attempt.assignment_id = p_assignment_id;
    IF COALESCE(accommodation_row.attempt_limit, assignment_row.attempt_limit) IS NOT NULL
       AND next_attempt_number > COALESCE(accommodation_row.attempt_limit, assignment_row.attempt_limit) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Assignment Attempt limit is reached';
    END IF;

    INSERT INTO ple_private.assignment_attempt(
        assignment_attempt_id, student_record_id, assignment_id, attempt_number, started_at,
        assignment_title, assignment_instructions, available_at, due_at, closes_at,
        assignment_attempt_time_limit_seconds, attempt_limit, late_work_rule,
        assignment_completion_rule, assignment_completion_score_threshold,
        assignment_attempt_grade_rule, assignment_attempt_continuation_rule,
        max_additional_assignment_attempts, question_pool_reuse_rule, question_variation_rule,
        assignment_attempt_resume_rule, assignment_question_display_rule,
        assignment_navigation_rule, assignment_question_order_rule, feedback_score,
        feedback_per_item_correctness, feedback_submitted_response, feedback_question_feedback,
        feedback_question_answer, feedback_question_answer_explanation, feedback_class_statistics,
        schedule_accommodation_id, schedule_accommodation_edit_number,
        time_limit_accommodation_id, time_limit_accommodation_edit_number,
        attempt_limit_accommodation_id, attempt_limit_accommodation_edit_number
    ) VALUES (
        p_assignment_attempt_id, p_student_record_id, p_assignment_id, next_attempt_number, now_value,
        assignment_row.assignment_title, assignment_row.assignment_instructions,
        COALESCE(accommodation_row.available_at, assignment_row.available_at),
        COALESCE(accommodation_row.due_at, assignment_row.due_at),
        COALESCE(accommodation_row.closes_at, assignment_row.closes_at),
        COALESCE(accommodation_row.assignment_attempt_time_limit_seconds, assignment_row.assignment_attempt_time_limit_seconds),
        COALESCE(accommodation_row.attempt_limit, assignment_row.attempt_limit),
        assignment_row.late_work_rule, assignment_row.assignment_completion_rule,
        assignment_row.assignment_completion_score_threshold, assignment_row.assignment_attempt_grade_rule,
        assignment_row.assignment_attempt_continuation_rule, assignment_row.max_additional_assignment_attempts,
        assignment_row.question_pool_reuse_rule, assignment_row.question_variation_rule,
        assignment_row.assignment_attempt_resume_rule, assignment_row.assignment_question_display_rule,
        assignment_row.assignment_navigation_rule, assignment_row.assignment_question_order_rule,
        assignment_row.feedback_score, assignment_row.feedback_per_item_correctness,
        assignment_row.feedback_submitted_response, assignment_row.feedback_question_feedback,
        assignment_row.feedback_question_answer, assignment_row.feedback_question_answer_explanation,
        assignment_row.feedback_class_statistics,
        CASE WHEN accommodation_row.available_at IS NOT NULL OR accommodation_row.due_at IS NOT NULL
               OR accommodation_row.closes_at IS NOT NULL THEN accommodation_row.accommodation_id END,
        CASE WHEN accommodation_row.available_at IS NOT NULL OR accommodation_row.due_at IS NOT NULL
               OR accommodation_row.closes_at IS NOT NULL THEN accommodation_row.accommodation_edit_number END,
        CASE WHEN accommodation_row.assignment_attempt_time_limit_seconds IS NOT NULL THEN accommodation_row.accommodation_id END,
        CASE WHEN accommodation_row.assignment_attempt_time_limit_seconds IS NOT NULL THEN accommodation_row.accommodation_edit_number END,
        CASE WHEN accommodation_row.attempt_limit IS NOT NULL THEN accommodation_row.accommodation_id END,
        CASE WHEN accommodation_row.attempt_limit IS NOT NULL THEN accommodation_row.accommodation_edit_number END
    );

    FOR selection IN SELECT value FROM jsonb_array_elements(p_selections) LOOP
        selection_id := (selection ->> 'question_pool_selection_id')::uuid;
        selection_entry_id := (selection ->> 'assignment_entry_id')::uuid;
        SELECT * INTO entry_row FROM ple_data.assignment_entry
         WHERE assignment_entry_id = selection_entry_id AND assignment_id = p_assignment_id
           AND entry_kind = 'question_pool' AND availability = 'available';
        IF NOT FOUND OR selection_id IS NULL
           OR jsonb_typeof(selection -> 'selected_items') <> 'array'
           OR jsonb_array_length(selection -> 'selected_items') <> entry_row.selection_count THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Pool Selection is not current released Assignment content';
        END IF;
        INSERT INTO ple_private.question_pool_selection(
            question_pool_selection_id, assignment_attempt_id, assignment_entry_id,
            created_at, selected_question_count, reused_from_question_pool_selection_id
        ) VALUES (
            selection_id, p_assignment_attempt_id, selection_entry_id, now_value,
            entry_row.selection_count, NULLIF(selection ->> 'reused_from_question_pool_selection_id', '')::uuid
        );
        INSERT INTO ple_private.question_pool_selected_item(
            question_pool_selection_id, question_pool_item_id, selection_position, question_id, revision_number
        )
        SELECT selection_id, (item.value ->> 'question_pool_item_id')::uuid,
               item.ordinality - 1, pool.question_id, pool.question_revision_number
          FROM jsonb_array_elements(selection -> 'selected_items') WITH ORDINALITY AS item(value, ordinality)
          JOIN ple_data.question_pool_item AS pool
            ON pool.question_pool_item_id = (item.value ->> 'question_pool_item_id')::uuid
           AND pool.assignment_entry_id = selection_entry_id
           AND pool.assignment_id = p_assignment_id
           AND pool.availability = 'available';
        IF (SELECT count(*) FROM ple_private.question_pool_selected_item WHERE question_pool_selection_id = selection_id)
             <> entry_row.selection_count THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Pool Selection contains an unavailable or repeated Item';
        END IF;
        -- A reused Selection retains its exact ordered prior membership.  The
        -- reference link records provenance; this equality check prevents a
        -- caller from labelling a newly selected set as reused.
        IF NULLIF(selection ->> 'reused_from_question_pool_selection_id', '') IS NOT NULL
           AND EXISTS (
               (SELECT selected.selection_position, selected.question_pool_item_id,
                       selected.question_id, selected.revision_number
                  FROM ple_private.question_pool_selected_item AS selected
                 WHERE selected.question_pool_selection_id = selection_id
                EXCEPT
                SELECT earlier.selection_position, earlier.question_pool_item_id,
                       earlier.question_id, earlier.revision_number
                  FROM ple_private.question_pool_selected_item AS earlier
                 WHERE earlier.question_pool_selection_id =
                       (selection ->> 'reused_from_question_pool_selection_id')::uuid)
               UNION ALL
               (SELECT earlier.selection_position, earlier.question_pool_item_id,
                       earlier.question_id, earlier.revision_number
                  FROM ple_private.question_pool_selected_item AS earlier
                 WHERE earlier.question_pool_selection_id =
                       (selection ->> 'reused_from_question_pool_selection_id')::uuid
                EXCEPT
                SELECT selected.selection_position, selected.question_pool_item_id,
                       selected.question_id, selected.revision_number
                  FROM ple_private.question_pool_selected_item AS selected
                 WHERE selected.question_pool_selection_id = selection_id)
           ) THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Reused Question Pool Selection must retain its exact prior Items';
        END IF;
    END LOOP;

    FOR issued IN SELECT value FROM jsonb_array_elements(p_issued_questions) LOOP
        issued_entry_id := (issued ->> 'assignment_entry_id')::uuid;
        issued_selection_id := NULLIF(issued ->> 'question_pool_selection_id', '')::uuid;
        issued_item_id := NULLIF(issued ->> 'question_pool_item_id', '')::uuid;
        SELECT * INTO entry_row FROM ple_data.assignment_entry
         WHERE assignment_entry_id = issued_entry_id AND assignment_id = p_assignment_id
           AND availability = 'available';
        IF NOT FOUND OR (issued ->> 'issued_question_id') IS NULL
           OR (issued ->> 'question_seed') IS NULL
           OR (issued ->> 'question_seed') !~ '^(0|[1-9][0-9]{0,19})$'
           OR (issued ->> 'question_seed')::numeric > 18446744073709551615 THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Issued Question is not current released Assignment content';
        END IF;
        IF entry_row.entry_kind = 'fixed_question' THEN
            IF issued_selection_id IS NOT NULL OR issued_item_id IS NOT NULL
               OR issued ->> 'question_id' <> entry_row.question_id
               OR (issued ->> 'revision_number')::integer <> entry_row.question_revision_number THEN
                RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Fixed Issued Question does not match its current Assignment Entry';
            END IF;
        ELSE
            IF issued_selection_id IS NULL OR issued_item_id IS NULL OR NOT EXISTS (
                SELECT 1 FROM ple_private.question_pool_selected_item AS selected
                 WHERE selected.question_pool_selection_id = issued_selection_id
                   AND selected.question_pool_item_id = issued_item_id
                   AND selected.question_id = issued ->> 'question_id'
                   AND selected.revision_number = (issued ->> 'revision_number')::integer
            ) THEN
                RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Pooled Issued Question does not match its selected Item';
            END IF;
        END IF;
        INSERT INTO ple_private.issued_question(
            issued_question_id, assignment_attempt_id, assignment_entry_id,
            assignment_content_entry_index, issued_position, question_id, revision_number,
            question_seed, point_value, scoring_rule, question_statistics_eligibility,
            question_attempt_limit, question_attempt_time_limit_seconds, question_attempt_grace_seconds,
            question_pool_selection_id, question_pool_item_id
        ) VALUES (
            (issued ->> 'issued_question_id')::uuid, p_assignment_attempt_id, issued_entry_id,
            entry_row.authored_position, (issued ->> 'issued_position')::integer,
            issued ->> 'question_id', (issued ->> 'revision_number')::integer,
            (issued ->> 'question_seed')::numeric,
            CASE WHEN entry_row.entry_kind = 'fixed_question' THEN entry_row.points_possible ELSE entry_row.points_per_item END,
            entry_row.scoring_rule, entry_row.scoring_rule <> 'excluded',
            entry_row.question_attempt_limit, entry_row.question_attempt_time_limit_seconds,
            entry_row.question_attempt_grace_seconds, issued_selection_id, issued_item_id
        );
    END LOOP;
    IF EXISTS (
        SELECT 1 FROM ple_data.assignment_entry AS entry
         WHERE entry.assignment_id = p_assignment_id AND entry.availability = 'available'
           AND entry.entry_kind = 'question_pool'
           AND NOT EXISTS (
               SELECT 1 FROM ple_private.question_pool_selection AS selection
                WHERE selection.assignment_attempt_id = p_assignment_attempt_id
                  AND selection.assignment_entry_id = entry.assignment_entry_id
           )
    ) OR EXISTS (
        SELECT 1 FROM ple_data.assignment_entry AS entry
         WHERE entry.assignment_id = p_assignment_id AND entry.availability = 'available'
           AND entry.entry_kind = 'fixed_question'
           AND NOT EXISTS (
               SELECT 1 FROM ple_private.issued_question AS issued
                WHERE issued.assignment_attempt_id = p_assignment_attempt_id
                  AND issued.assignment_entry_id = entry.assignment_entry_id
                  AND issued.question_id = entry.question_id
                  AND issued.revision_number = entry.question_revision_number
           )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assignment Attempt must retain the complete current released issue set';
    END IF;
    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.issued_question AS retained_issued_question
         WHERE retained_issued_question.assignment_attempt_id = p_assignment_attempt_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Assignment Attempt requires issued Questions';
    END IF;
    assignment_attempt_id := p_assignment_attempt_id;
    attempt_number := next_attempt_number;
    resumed := false;
    RETURN NEXT;
END $$;

-- Course membership owns the current Student lookup.  This helper remains
-- executable only by the private Attempt boundary, so application sessions
-- cannot enumerate Student records.  The public ownership predicate continues
-- to decide whether the resulting record is usable for Student Work.
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.current_session_student_record_id(p_course_id uuid)
RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT student.student_record_id
      FROM ple_data.student_record AS student
      JOIN ple_data.course_membership AS membership
        ON membership.student_record_id = student.student_record_id
       AND membership.course_id = student.course_id
       AND membership.account_id = student.student_account_id
       AND membership.role = 'student'
       AND ple_data.course_membership_is_active(membership.membership_id)
     WHERE student.course_id = p_course_id
       AND student.student_account_id = ple_api.current_session_account_id()
$$;
REVOKE ALL ON FUNCTION ple_api.current_session_student_record_id(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.current_session_student_record_id(uuid) TO ple_private_owner;

-- Private Attempt readers need stable Course route/display facts but do not
-- receive direct access to the Course relation.  These API-owner functions
-- are executable only by that trusted private boundary.
CREATE FUNCTION ple_api.course_reference_number_for_attempt(p_course_id uuid)
RETURNS bigint LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT course.reference_number
      FROM ple_data.course_instance AS course
     WHERE course.course_id = p_course_id
$$;
CREATE FUNCTION ple_api.course_display_for_attempt(p_course_id uuid)
RETURNS TABLE (
    course_reference_number bigint,
    course_short_name text,
    course_long_name text,
    course_theme text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT course.reference_number,
           course.course_short_name,
           course.course_long_name,
           course.course_theme
      FROM ple_data.course_instance AS course
     WHERE course.course_id = p_course_id
$$;
REVOKE ALL ON FUNCTION ple_api.course_reference_number_for_attempt(uuid),
    ple_api.course_display_for_attempt(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.course_reference_number_for_attempt(uuid),
    ple_api.course_display_for_attempt(uuid) TO ple_private_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;

-- This prepares, but does not mint, a Student Work root.  The application
-- keeps cryptographic randomness at its existing boundary, selects an exact
-- set of available pool Items, and immediately calls start_assignment_attempt
-- in the same transaction.  Taking the Assignment lock here preserves the
-- one Student Work lock order while the selection is prepared; the canonical
-- start function repeats the current-state checks before it writes evidence.
-- ASVS 2.2.1, 2.3.1, and 8.2.1: the authenticated database boundary resolves
-- the Student and route references rather than accepting either identity from
-- the browser.
CREATE FUNCTION ple_private.prepare_current_assignment_attempt_start(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint
) RETURNS TABLE (
    student_record_id uuid,
    assignment_id uuid,
    assignment_entry_id uuid,
    entry_kind text,
    authored_position integer,
    fixed_question_id text,
    fixed_revision_number integer,
    question_pool_item_id uuid,
    pool_question_id text,
    pool_revision_number integer,
    selection_count integer,
    pool_selection_rule text,
    question_pool_reuse_rule text,
    question_variation_rule text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assignment_row ple_data.assignment%ROWTYPE;
DECLARE student_record_id_value uuid;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_assignment_reference_number NOT BETWEEN 1 AND 2147483647 THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt start is unavailable';
    END IF;

    SELECT assignment.* INTO assignment_row
      FROM ple_data.assignment AS assignment
     WHERE assignment.reference_number = p_assignment_reference_number;
    IF NOT FOUND OR ple_api.course_reference_number_for_attempt(
        assignment_row.course_id
    ) IS DISTINCT FROM p_course_reference_number THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt start is unavailable';
    END IF;

    -- This is the first Student Work lock.  The route lookup above performs
    -- no child lock; all subsequent selection reads are stable until the
    -- surrounding start transaction commits or rolls back.
    PERFORM ple_private.lock_assignment_for_student_work(assignment_row.assignment_id);
    SELECT assignment.* INTO assignment_row
      FROM ple_data.assignment AS assignment
     WHERE assignment.assignment_id = assignment_row.assignment_id;
    SELECT ple_api.current_session_student_record_id(assignment_row.course_id)
      INTO student_record_id_value;
    IF assignment_row.assignment_status <> 'released' OR NOT FOUND
       OR NOT ple_api.current_session_account_owns_student_record(
           assignment_row.course_id, student_record_id_value
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt start is unavailable';
    END IF;

    RETURN QUERY
    SELECT student_record_id_value,
           assignment_row.assignment_id,
           entry.assignment_entry_id,
           entry.entry_kind,
           entry.authored_position,
           entry.question_id,
           entry.question_revision_number,
           item.question_pool_item_id,
           item.question_id,
           item.question_revision_number,
           entry.selection_count,
           entry.selected_question_order,
           assignment_row.question_pool_reuse_rule,
           assignment_row.question_variation_rule
      FROM ple_data.assignment_entry AS entry
      LEFT JOIN ple_data.question_pool_item AS item
        ON item.assignment_entry_id = entry.assignment_entry_id
       AND item.availability = 'available'
     WHERE entry.assignment_id = assignment_row.assignment_id
       AND entry.availability = 'available'
       AND (entry.entry_kind = 'fixed_question' OR item.question_pool_item_id IS NOT NULL)
     ORDER BY entry.authored_position, item.item_position NULLS FIRST;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt start is unavailable';
    END IF;
END $$;

-- The start response reads its title and instructions from the retained
-- Attempt evidence.  Course and Assignment reference numbers are stable
-- route identities, while authored content is never re-read from mutable
-- Assignment configuration after an Attempt exists.
CREATE FUNCTION ple_private.read_started_student_assignment_attempt(
    p_assignment_attempt_id uuid
) RETURNS TABLE (
    assignment_attempt_reference_number bigint,
    course_reference_number bigint,
    assignment_reference_number bigint,
    attempt_number integer,
    assignment_title text,
    assignment_instructions text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_assignment_attempt_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt is unavailable';
    END IF;
    RETURN QUERY
    SELECT attempt.reference_number,
           ple_api.course_reference_number_for_attempt(assignment.course_id),
           assignment.reference_number,
           attempt.attempt_number,
           attempt.assignment_title,
           attempt.assignment_instructions
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
     WHERE attempt.assignment_attempt_id = p_assignment_attempt_id
       AND ple_api.current_session_account_owns_student_record(
           assignment.course_id, attempt.student_record_id
       );
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt is unavailable';
    END IF;
END $$;

CREATE FUNCTION ple_private.save_student_assignment_attempt_response(
    p_assignment_attempt_reference_number bigint, p_issued_position integer, p_student_response jsonb
) RETURNS TABLE (assignment_attempt_reference_number bigint, issued_position integer, response_state text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE attempt_row ple_private.assignment_attempt%ROWTYPE;
DECLARE question_attempt_id_value uuid;
DECLARE now_value timestamptz := pg_catalog.clock_timestamp();
BEGIN
    IF p_issued_position < 0 OR jsonb_typeof(p_student_response) <> 'object' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Student response arguments are invalid';
    END IF;
    attempt_row := ple_private.assert_current_student_attempt(p_assignment_attempt_reference_number);
    PERFORM ple_private.lock_assignment_for_student_work(attempt_row.assignment_id);
    IF (attempt_row.closes_at IS NOT NULL AND attempt_row.closes_at < now_value)
       OR (attempt_row.late_work_rule = 'reject' AND attempt_row.due_at IS NOT NULL
           AND attempt_row.due_at < now_value)
       OR (attempt_row.assignment_attempt_time_limit_seconds IS NOT NULL
           AND attempt_row.started_at + pg_catalog.make_interval(
               secs => attempt_row.assignment_attempt_time_limit_seconds) < now_value) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Assignment Attempt no longer accepts responses';
    END IF;
    SELECT question_attempt.question_attempt_id INTO question_attempt_id_value
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
     WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
       AND issued.issued_position = p_issued_position
       AND question_attempt.question_attempt_state = 'open'
       AND (question_attempt.deadline_at IS NULL OR question_attempt.deadline_at >= now_value)
     FOR UPDATE OF question_attempt;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Issued Question is unavailable';
    END IF;
    INSERT INTO ple_private.assignment_attempt_saved_response(question_attempt_id, student_response, saved_at)
    VALUES (question_attempt_id_value, p_student_response, now_value)
    ON CONFLICT (question_attempt_id) DO UPDATE
       SET student_response = EXCLUDED.student_response, saved_at = EXCLUDED.saved_at;
    assignment_attempt_reference_number := attempt_row.reference_number;
    issued_position := p_issued_position;
    response_state := 'saved';
    RETURN NEXT;
END $$;

CREATE FUNCTION ple_private.finalize_student_assignment_attempt(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (submission_state text, missing_positions integer[])
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE attempt_row ple_private.assignment_attempt%ROWTYPE;
DECLARE now_value timestamptz := pg_catalog.clock_timestamp();
DECLARE finalized_question_attempt record;
DECLARE question_submission_id_value uuid;
DECLARE question_submission_grading_id_value uuid;
DECLARE grading_job_id_value uuid;
BEGIN
    attempt_row := ple_private.assert_current_student_attempt(p_assignment_attempt_reference_number);
    PERFORM ple_private.lock_assignment_for_student_work(attempt_row.assignment_id);
    PERFORM 1 FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = question_attempt.issued_question_id
     WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
     FOR UPDATE OF question_attempt;

    -- A lost response to an accepted whole-Attempt finalization converges on
    -- its immutable Assignment Submission.  It neither reopens Questions nor
    -- creates another Submission or grading Job.
    IF EXISTS (
        SELECT 1 FROM ple_private.assignment_submission AS submission
         WHERE submission.assignment_attempt_id = attempt_row.assignment_attempt_id
    ) THEN
        submission_state := 'submitted';
        missing_positions := ARRAY[]::integer[];
        RETURN NEXT;
        RETURN;
    END IF;

    SELECT array_agg(issued.issued_position ORDER BY issued.issued_position) INTO missing_positions
      FROM ple_private.issued_question AS issued
      LEFT JOIN ple_private.question_attempt AS question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.assignment_attempt_saved_response AS response ON response.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
       AND (question_attempt.question_attempt_state <> 'open' OR response.question_attempt_id IS NULL);
    IF missing_positions IS NOT NULL THEN
        submission_state := 'missing_responses';
        RETURN NEXT;
        RETURN;
    END IF;

    -- Saved-response finalization owns only the database-native and WeBWorK
    -- paths.  iMathAS persists its verified external result through delivery,
    -- rather than allowing this generic operation to manufacture evidence.
    IF EXISTS (
        SELECT 1
          FROM ple_private.issued_question AS issued
          LEFT JOIN ple_private.question_revision_source_binding AS source
            ON source.question_id = issued.question_id
           AND source.revision_number = issued.revision_number
         WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
           AND (source.question_id IS NULL OR source.backend NOT IN ('ple', 'webwork'))
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assignment Attempt submission is unavailable';
    END IF;

    UPDATE ple_private.question_attempt AS question_attempt
       SET question_attempt_state = 'submission_accepted', submitted_at = now_value
      FROM ple_private.issued_question AS issued
     WHERE question_attempt.issued_question_id = issued.issued_question_id
       AND issued.assignment_attempt_id = attempt_row.assignment_attempt_id;

    FOR finalized_question_attempt IN
        SELECT question_attempt.question_attempt_id, response.student_response,
               CASE source.backend
                   WHEN 'ple' THEN 'native_ple_grading'
                   WHEN 'webwork' THEN 'webwork_grading'
               END AS worker_kind
          FROM ple_private.question_attempt AS question_attempt
          JOIN ple_private.issued_question AS issued
            ON issued.issued_question_id = question_attempt.issued_question_id
          JOIN ple_private.assignment_attempt_saved_response AS response
            ON response.question_attempt_id = question_attempt.question_attempt_id
          JOIN ple_private.question_revision_source_binding AS source
            ON source.question_id = issued.question_id
           AND source.revision_number = issued.revision_number
         WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
         ORDER BY issued.issued_position
    LOOP
        question_submission_id_value := pg_catalog.gen_random_uuid();
        question_submission_grading_id_value := pg_catalog.gen_random_uuid();
        grading_job_id_value := pg_catalog.gen_random_uuid();
        INSERT INTO ple_private.question_submission(
            submission_id, question_attempt_id, submitted_at, student_response
        ) VALUES (
            question_submission_id_value,
            finalized_question_attempt.question_attempt_id,
            now_value,
            finalized_question_attempt.student_response
        );
        PERFORM ple_private.enqueue_grade_accepted_submission(
            grading_job_id_value,
            question_submission_grading_id_value,
            question_submission_id_value,
            finalized_question_attempt.worker_kind,
            '{}'::jsonb,
            now_value,
            3,
            now_value
        );
    END LOOP;
    INSERT INTO ple_private.assignment_submission(
        assignment_submission_id, assignment_attempt_id, submitted_at, authorized_by_account_id, receipt
    ) VALUES (
        pg_catalog.gen_random_uuid(), attempt_row.assignment_attempt_id, now_value,
        ple_api.current_session_account_id(), jsonb_build_object('submissionState', 'submitted')
    );
    submission_state := 'submitted';
    missing_positions := ARRAY[]::integer[];
    RETURN NEXT;
END $$;

CREATE FUNCTION ple_private.read_student_assignment_attempt_progress(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (
    assignment_attempt_reference_number bigint, question_count integer,
    recommended_position integer, issued_position integer, response_state text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE attempt_row ple_private.assignment_attempt%ROWTYPE;
BEGIN
    attempt_row := ple_private.assert_current_student_attempt(p_assignment_attempt_reference_number);
    RETURN QUERY
    SELECT attempt_row.reference_number, count(*) OVER ()::integer,
           min(issued.issued_position) FILTER (WHERE question_attempt.question_attempt_state = 'open'
               AND response.question_attempt_id IS NULL) OVER (),
           issued.issued_position,
           CASE WHEN question_attempt.question_attempt_state = 'submission_accepted' THEN 'submitted'
                WHEN question_attempt.question_attempt_state = 'closed_at_deadline' THEN 'closed'
                WHEN response.question_attempt_id IS NOT NULL THEN 'saved' ELSE 'unanswered' END
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.assignment_attempt_saved_response AS response ON response.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
     ORDER BY issued.issued_position;
END $$;

-- A saved response is mutable working state for one currently active,
-- student-owned Attempt.  This reader resolves only the retained Issued
-- Question and Question Attempt evidence; it does not consult mutable
-- Assignment configuration.
CREATE FUNCTION ple_private.read_student_assignment_attempt_saved_response(
    p_assignment_attempt_reference_number bigint,
    p_issued_position integer
) RETURNS TABLE (issued_position integer, student_response jsonb)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE attempt_row ple_private.assignment_attempt%ROWTYPE;
BEGIN
    IF p_issued_position < 0 THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Saved Student response is unavailable';
    END IF;

    attempt_row := ple_private.assert_current_student_attempt(
        p_assignment_attempt_reference_number
    );
    IF attempt_row.completed_at IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Saved Student response is unavailable';
    END IF;

    RETURN QUERY
    SELECT issued.issued_position,
           response.student_response
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.assignment_attempt_saved_response AS response
        ON response.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
       AND issued.issued_position = p_issued_position;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Saved Student response is unavailable';
    END IF;
END $$;

-- Grading workers call this before recording a result.  It deliberately takes
-- the same Assignment row lock as student mutations so Unrelease either wins
-- before work begins or waits for the already-authorized commit to finish.
CREATE FUNCTION ple_private.lock_question_attempt_for_grading(p_question_attempt_id uuid)
RETURNS TABLE (
    question_attempt_id uuid, issued_question_id uuid, assignment_attempt_id uuid,
    assignment_id uuid, question_id text, revision_number integer, question_seed numeric
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE assignment_id_value uuid;
BEGIN
    SELECT attempt.assignment_id INTO assignment_id_value
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id = issued.assignment_attempt_id
     WHERE question_attempt.question_attempt_id = p_question_attempt_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Attempt grading target is unavailable';
    END IF;
    PERFORM ple_private.lock_assignment_for_student_work(assignment_id_value);
    RETURN QUERY
    SELECT question_attempt.question_attempt_id, issued.issued_question_id,
           attempt.assignment_attempt_id, attempt.assignment_id, issued.question_id,
           issued.revision_number, issued.question_seed
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id = issued.assignment_attempt_id
     WHERE question_attempt.question_attempt_id = p_question_attempt_id
     FOR KEY SHARE OF question_attempt, issued, attempt;
END $$;

CREATE FUNCTION ple_private.read_student_assignment_attempt_history_evidence(
    p_assignment_attempt_reference_number bigint
) RETURNS TABLE (
    assignment_attempt_reference_number bigint, assignment_title text, assignment_instructions text,
    issued_position integer, question_id text, revision_number integer, question_seed numeric,
    question_attempt_limit integer, question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer, question_attempt_state text, student_response jsonb
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE attempt_row ple_private.assignment_attempt%ROWTYPE;
BEGIN
    attempt_row := ple_private.assert_current_student_attempt(p_assignment_attempt_reference_number);
    RETURN QUERY
    SELECT attempt_row.reference_number, attempt_row.assignment_title, attempt_row.assignment_instructions,
           issued.issued_position, issued.question_id, issued.revision_number, issued.question_seed,
           issued.question_attempt_limit, issued.question_attempt_time_limit_seconds,
           issued.question_attempt_grace_seconds, question_attempt.question_attempt_state,
           COALESCE(submission.student_response, response.student_response)
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.assignment_attempt_saved_response AS response ON response.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_private.question_submission AS submission ON submission.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
     ORDER BY issued.issued_position;
END $$;

-- Accommodations are current teaching configuration.  An Attempt copies its
-- effective values and sources at start, while this guarded CAS path remains
-- available for later Attempts.
CREATE FUNCTION ple_private.save_student_assignment_accommodation(
    p_accommodation_id uuid,
    p_student_record_id uuid,
    p_assignment_id uuid,
    p_expected_edit_number bigint,
    p_available_at timestamptz,
    p_due_at timestamptz,
    p_closes_at timestamptz,
    p_assignment_attempt_time_limit_seconds integer,
    p_attempt_limit integer
) RETURNS TABLE (accommodation_edit_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE current_row ple_private.student_assignment_accommodation%ROWTYPE;
DECLARE course_id_value uuid;
BEGIN
    IF p_accommodation_id IS NULL OR p_student_record_id IS NULL OR p_assignment_id IS NULL
       OR p_expected_edit_number IS NULL OR p_expected_edit_number < 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Student Assignment Accommodation save is invalid';
    END IF;
    SELECT assignment.course_id INTO course_id_value
      FROM ple_data.assignment AS assignment
     WHERE assignment.assignment_id = p_assignment_id;
    IF NOT FOUND
       OR NOT ple_data.student_assignment_has_course_scope(p_student_record_id, p_assignment_id)
       OR NOT ple_api.current_session_account_is_course_instructor(course_id_value) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student Assignment Accommodation is unavailable';
    END IF;
    -- Shares the Student Work root lock so a newly accepted current
    -- accommodation and an Attempt start observe one ordering.
    PERFORM ple_private.lock_assignment_for_student_work(p_assignment_id);
    SELECT * INTO current_row FROM ple_private.student_assignment_accommodation
     WHERE student_record_id = p_student_record_id AND assignment_id = p_assignment_id
     FOR UPDATE;
    IF NOT FOUND THEN
        IF p_expected_edit_number <> 0 THEN
            RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Student Assignment Accommodation Edit Number is stale';
        END IF;
        INSERT INTO ple_private.student_assignment_accommodation(
            accommodation_id, student_record_id, assignment_id, available_at, due_at, closes_at,
            assignment_attempt_time_limit_seconds, attempt_limit, created_at
        ) VALUES (
            p_accommodation_id, p_student_record_id, p_assignment_id, p_available_at, p_due_at,
            p_closes_at, p_assignment_attempt_time_limit_seconds, p_attempt_limit,
            pg_catalog.transaction_timestamp()
        ) RETURNING ple_private.student_assignment_accommodation.accommodation_edit_number
          INTO accommodation_edit_number;
        RETURN NEXT;
        RETURN;
    END IF;
    IF current_row.accommodation_id <> p_accommodation_id THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Student Assignment Accommodation identity is unavailable';
    END IF;
    IF current_row.accommodation_edit_number <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Student Assignment Accommodation Edit Number is stale';
    END IF;
    IF ROW(current_row.available_at, current_row.due_at, current_row.closes_at,
           current_row.assignment_attempt_time_limit_seconds, current_row.attempt_limit)
       IS NOT DISTINCT FROM ROW(p_available_at, p_due_at, p_closes_at,
                                p_assignment_attempt_time_limit_seconds, p_attempt_limit) THEN
        accommodation_edit_number := current_row.accommodation_edit_number;
        RETURN NEXT;
        RETURN;
    END IF;
    UPDATE ple_private.student_assignment_accommodation
       SET available_at = p_available_at,
           due_at = p_due_at,
           closes_at = p_closes_at,
           assignment_attempt_time_limit_seconds = p_assignment_attempt_time_limit_seconds,
           attempt_limit = p_attempt_limit,
           accommodation_edit_number = current_row.accommodation_edit_number + 1
     WHERE accommodation_id = p_accommodation_id
 RETURNING ple_private.student_assignment_accommodation.accommodation_edit_number
      INTO accommodation_edit_number;
    RETURN NEXT;
END $$;

REVOKE ALL ON FUNCTION ple_private.lock_assignment_for_student_work(uuid),
    ple_private.assert_current_student_attempt(bigint),
    ple_private.start_assignment_attempt(uuid, uuid, uuid, jsonb, jsonb),
    ple_private.prepare_current_assignment_attempt_start(bigint, bigint),
    ple_private.read_started_student_assignment_attempt(uuid),
    ple_private.save_student_assignment_attempt_response(bigint, integer, jsonb),
    ple_private.finalize_student_assignment_attempt(bigint),
    ple_private.read_student_assignment_attempt_progress(bigint),
    ple_private.read_student_assignment_attempt_saved_response(bigint, integer),
    ple_private.lock_question_attempt_for_grading(uuid),
    ple_private.read_student_assignment_attempt_history_evidence(bigint),
    ple_private.save_student_assignment_accommodation(uuid, uuid, uuid, bigint, timestamptz, timestamptz, timestamptz, integer, integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.lock_assignment_for_student_work(uuid),
    ple_private.start_assignment_attempt(uuid, uuid, uuid, jsonb, jsonb),
    ple_private.prepare_current_assignment_attempt_start(bigint, bigint),
    ple_private.read_started_student_assignment_attempt(uuid),
    ple_private.save_student_assignment_attempt_response(bigint, integer, jsonb),
    ple_private.finalize_student_assignment_attempt(bigint),
    ple_private.read_student_assignment_attempt_progress(bigint),
    ple_private.read_student_assignment_attempt_saved_response(bigint, integer),
    ple_private.read_student_assignment_attempt_history_evidence(bigint),
    ple_private.save_student_assignment_accommodation(uuid, uuid, uuid, bigint, timestamptz, timestamptz, timestamptz, integer, integer) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.start_assignment_attempt(uuid, uuid, uuid, jsonb, jsonb)
RETURNS TABLE (assignment_attempt_id uuid, attempt_number integer, resumed boolean)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.start_assignment_attempt($1, $2, $3, $4, $5)
$$;
CREATE FUNCTION ple_api.prepare_current_assignment_attempt_start(bigint, bigint)
RETURNS TABLE (
    student_record_id uuid, assignment_id uuid, assignment_entry_id uuid,
    entry_kind text, authored_position integer, fixed_question_id text,
    fixed_revision_number integer, question_pool_item_id uuid,
    pool_question_id text, pool_revision_number integer, selection_count integer,
    pool_selection_rule text, question_pool_reuse_rule text,
    question_variation_rule text
)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.prepare_current_assignment_attempt_start($1, $2)
$$;
CREATE FUNCTION ple_api.read_started_student_assignment_attempt(uuid)
RETURNS TABLE (
    assignment_attempt_reference_number bigint, course_reference_number bigint,
    assignment_reference_number bigint, attempt_number integer,
    assignment_title text, assignment_instructions text
)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.read_started_student_assignment_attempt($1)
$$;
CREATE FUNCTION ple_api.save_student_assignment_attempt_response(bigint, integer, jsonb)
RETURNS TABLE (assignment_attempt_reference_number bigint, issued_position integer, response_state text)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT assignment_attempt_reference_number, issued_position + 1, response_state
      FROM ple_private.save_student_assignment_attempt_response($1, $2 - 1, $3)
$$;
CREATE FUNCTION ple_api.finalize_student_assignment_attempt(bigint)
RETURNS TABLE (submission_state text, missing_positions integer[])
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT submission_state,
           ARRAY(SELECT position + 1 FROM unnest(missing_positions) AS position)
      FROM ple_private.finalize_student_assignment_attempt($1)
$$;
CREATE FUNCTION ple_api.read_student_assignment_attempt_progress(bigint)
RETURNS TABLE (assignment_attempt_reference_number bigint, question_count integer,
    recommended_position integer, issued_position integer, response_state text)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT assignment_attempt_reference_number, question_count,
           recommended_position + 1, issued_position + 1, response_state
      FROM ple_private.read_student_assignment_attempt_progress($1)
$$;
CREATE FUNCTION ple_api.read_student_assignment_attempt_saved_response(bigint, integer)
RETURNS TABLE (issued_position integer, student_response jsonb)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT issued_position + 1, student_response
      FROM ple_private.read_student_assignment_attempt_saved_response($1, $2 - 1)
$$;
CREATE FUNCTION ple_api.read_student_assignment_attempt_history_evidence(bigint)
RETURNS TABLE (assignment_attempt_reference_number bigint, assignment_title text, assignment_instructions text,
    issued_position integer, question_id text, revision_number integer, question_seed numeric,
    question_attempt_limit integer, question_attempt_time_limit_seconds integer,
    question_attempt_grace_seconds integer, question_attempt_state text, student_response jsonb)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT assignment_attempt_reference_number, assignment_title, assignment_instructions,
           issued_position + 1, question_id, revision_number, question_seed,
           question_attempt_limit, question_attempt_time_limit_seconds,
           question_attempt_grace_seconds, question_attempt_state, student_response
      FROM ple_private.read_student_assignment_attempt_history_evidence($1)
$$;
CREATE FUNCTION ple_api.save_student_assignment_accommodation(
    uuid, uuid, uuid, bigint, timestamptz, timestamptz, timestamptz, integer, integer
) RETURNS TABLE (accommodation_edit_number bigint)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT * FROM ple_private.save_student_assignment_accommodation($1, $2, $3, $4, $5, $6, $7, $8, $9)
$$;
REVOKE ALL ON FUNCTION ple_api.start_assignment_attempt(uuid, uuid, uuid, jsonb, jsonb),
    ple_api.prepare_current_assignment_attempt_start(bigint, bigint),
    ple_api.read_started_student_assignment_attempt(uuid),
    ple_api.save_student_assignment_attempt_response(bigint, integer, jsonb),
    ple_api.finalize_student_assignment_attempt(bigint), ple_api.read_student_assignment_attempt_progress(bigint),
    ple_api.read_student_assignment_attempt_saved_response(bigint, integer),
    ple_api.read_student_assignment_attempt_history_evidence(bigint),
    ple_api.save_student_assignment_accommodation(uuid, uuid, uuid, bigint, timestamptz, timestamptz, timestamptz, integer, integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.start_assignment_attempt(uuid, uuid, uuid, jsonb, jsonb),
    ple_api.prepare_current_assignment_attempt_start(bigint, bigint),
    ple_api.read_started_student_assignment_attempt(uuid),
    ple_api.save_student_assignment_attempt_response(bigint, integer, jsonb),
    ple_api.finalize_student_assignment_attempt(bigint), ple_api.read_student_assignment_attempt_progress(bigint),
    ple_api.read_student_assignment_attempt_saved_response(bigint, integer),
    ple_api.read_student_assignment_attempt_history_evidence(bigint),
    ple_api.save_student_assignment_accommodation(uuid, uuid, uuid, bigint, timestamptz, timestamptz, timestamptz, integer, integer) TO ple_app;
RESET ROLE;
