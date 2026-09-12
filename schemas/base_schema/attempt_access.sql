-- Authenticated Student Assignment route reads.  Current Assignment state
-- determines whether another Attempt can start; existing Attempts provide
-- their own retained interpretation evidence.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.read_student_assignment_access(p_course_reference_number bigint, p_assignment_reference_number bigint)
RETURNS TABLE (start_decision text, assignment_title text, question_count integer, points_possible double precision, assignment_attempt_time_limit_seconds integer, previous_attempts jsonb)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assignment_row ple_data.assignment%ROWTYPE; accommodation_row ple_private.student_assignment_accommodation%ROWTYPE;
DECLARE student_record_id_value uuid; active_attempt ple_private.assignment_attempt%ROWTYPE;
DECLARE evaluated_at timestamptz := pg_catalog.statement_timestamp(); started_attempt_count integer;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647 OR p_assignment_reference_number NOT BETWEEN 1 AND 2147483647 THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable'; END IF;
    SELECT assignment.* INTO assignment_row FROM ple_data.assignment AS assignment WHERE assignment.reference_number = p_assignment_reference_number;
    IF NOT FOUND OR ple_api.course_reference_number_for_attempt(assignment_row.course_id) IS DISTINCT FROM p_course_reference_number THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable'; END IF;
    SELECT ple_api.current_session_student_record_id(assignment_row.course_id) INTO student_record_id_value;
    IF student_record_id_value IS NULL OR NOT ple_api.current_session_account_owns_student_record(assignment_row.course_id, student_record_id_value) THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable'; END IF;
    SELECT * INTO accommodation_row FROM ple_private.student_assignment_accommodation WHERE student_record_id = student_record_id_value AND assignment_id = assignment_row.assignment_id;
    SELECT * INTO active_attempt FROM ple_private.assignment_attempt WHERE student_record_id = student_record_id_value AND assignment_id = assignment_row.assignment_id AND completed_at IS NULL ORDER BY attempt_number DESC LIMIT 1;
    IF FOUND THEN
        assignment_title := active_attempt.assignment_title; assignment_attempt_time_limit_seconds := active_attempt.assignment_attempt_time_limit_seconds;
        SELECT count(*)::integer, COALESCE(sum(issued.point_value), 0)::double precision INTO question_count, points_possible FROM ple_private.issued_question AS issued WHERE issued.assignment_attempt_id = active_attempt.assignment_attempt_id;
    ELSE
        assignment_title := assignment_row.assignment_title; assignment_attempt_time_limit_seconds := COALESCE(accommodation_row.assignment_attempt_time_limit_seconds, assignment_row.assignment_attempt_time_limit_seconds);
        SELECT COALESCE(sum(CASE entry.entry_kind WHEN 'fixed_question' THEN 1 ELSE entry.selection_count END), 0)::integer, COALESCE(sum(CASE entry.entry_kind WHEN 'fixed_question' THEN entry.points_possible ELSE entry.points_per_item * entry.selection_count END), 0)::double precision INTO question_count, points_possible FROM ple_data.assignment_entry AS entry WHERE entry.assignment_id = assignment_row.assignment_id AND entry.availability = 'available';
    END IF;
    SELECT count(*)::integer INTO started_attempt_count FROM ple_private.assignment_attempt AS attempt WHERE attempt.student_record_id = student_record_id_value AND attempt.assignment_id = assignment_row.assignment_id;
    IF assignment_row.assignment_status <> 'released' OR (COALESCE(accommodation_row.available_at, assignment_row.available_at) IS NOT NULL AND evaluated_at < COALESCE(accommodation_row.available_at, assignment_row.available_at)) THEN
        start_decision := CASE WHEN assignment_row.assignment_status = 'released' THEN 'not_yet_available' ELSE 'closed' END;
    ELSIF COALESCE(accommodation_row.closes_at, assignment_row.closes_at) IS NOT NULL AND evaluated_at > COALESCE(accommodation_row.closes_at, assignment_row.closes_at) THEN start_decision := 'closed';
    ELSIF assignment_row.late_work_rule = 'reject' AND COALESCE(accommodation_row.due_at, assignment_row.due_at) IS NOT NULL AND evaluated_at > COALESCE(accommodation_row.due_at, assignment_row.due_at) THEN start_decision := 'late_work_refused';
    ELSIF COALESCE(accommodation_row.attempt_limit, assignment_row.attempt_limit) IS NOT NULL AND started_attempt_count >= COALESCE(accommodation_row.attempt_limit, assignment_row.attempt_limit) THEN start_decision := 'attempt_limit_reached';
    ELSE start_decision := 'may_start'; END IF;
    SELECT COALESCE(jsonb_agg(jsonb_build_object('assignmentAttempt', 'R-' || attempt.reference_number, 'attemptNumber', attempt.attempt_number, 'state', CASE WHEN submission.assignment_attempt_id IS NULL THEN 'closed' ELSE 'submitted' END) ORDER BY attempt.attempt_number DESC, attempt.reference_number DESC), '[]'::jsonb) INTO previous_attempts FROM ple_private.assignment_attempt AS attempt LEFT JOIN ple_private.assignment_submission AS submission ON submission.assignment_attempt_id = attempt.assignment_attempt_id WHERE attempt.student_record_id = student_record_id_value AND attempt.assignment_id = assignment_row.assignment_id AND attempt.completed_at IS NOT NULL;
    RETURN NEXT;
END $$;

CREATE FUNCTION ple_private.read_active_student_assignment_attempt_reference(p_course_reference_number bigint, p_assignment_reference_number bigint)
RETURNS TABLE (assignment_attempt_reference_number bigint) LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT attempt.reference_number FROM ple_private.assignment_attempt AS attempt JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id WHERE p_course_reference_number BETWEEN 1 AND 2147483647 AND p_assignment_reference_number BETWEEN 1 AND 2147483647 AND ple_api.course_reference_number_for_attempt(assignment.course_id) = p_course_reference_number AND assignment.reference_number = p_assignment_reference_number AND attempt.completed_at IS NULL AND ple_api.current_session_account_owns_student_record(assignment.course_id, attempt.student_record_id) ORDER BY attempt.attempt_number DESC LIMIT 1
$$;

CREATE FUNCTION ple_private.read_reusable_question_pool_selection(p_assignment_id uuid, p_student_record_id uuid, p_assignment_entry_id uuid)
RETURNS TABLE (question_pool_selection_id uuid, selection_position integer, question_pool_item_id uuid, question_id text, revision_number integer)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE course_id_value uuid;
BEGIN
    SELECT assignment.course_id INTO course_id_value FROM ple_data.assignment AS assignment WHERE assignment.assignment_id = p_assignment_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_owns_student_record(course_id_value, p_student_record_id) THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Pool Selection is unavailable'; END IF;
    RETURN QUERY WITH latest AS (SELECT selection.question_pool_selection_id FROM ple_private.question_pool_selection AS selection JOIN ple_private.assignment_attempt AS attempt ON attempt.assignment_attempt_id = selection.assignment_attempt_id WHERE attempt.student_record_id = p_student_record_id AND attempt.assignment_id = p_assignment_id AND selection.assignment_entry_id = p_assignment_entry_id ORDER BY attempt.attempt_number DESC, selection.created_at DESC, selection.question_pool_selection_id DESC LIMIT 1) SELECT selected.question_pool_selection_id, selected.selection_position, selected.question_pool_item_id, selected.question_id, selected.revision_number FROM latest JOIN ple_private.question_pool_selected_item AS selected ON selected.question_pool_selection_id = latest.question_pool_selection_id ORDER BY selected.selection_position;
END $$;

CREATE FUNCTION ple_private.read_student_assignment_attempt_context(p_assignment_attempt_reference_number bigint)
RETURNS TABLE (assignment_attempt_reference_number bigint, attempt_number integer, course_reference_number bigint, course_short_name text, course_long_name text, course_theme text, assignment_reference_number bigint, assignment_title text, timer_remaining_milliseconds bigint)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT attempt.reference_number, attempt.attempt_number, course.course_reference_number,
           course.course_short_name, course.course_long_name, course.course_theme,
           assignment.reference_number, attempt.assignment_title,
           CASE WHEN attempt.assignment_attempt_time_limit_seconds IS NULL THEN NULL
                ELSE greatest(0::bigint, floor(extract(epoch FROM (
                    attempt.started_at + pg_catalog.make_interval(
                        secs => attempt.assignment_attempt_time_limit_seconds
                    ) - pg_catalog.statement_timestamp()
                )) * 1000)::bigint) END
      FROM ple_private.assignment_attempt AS attempt
      JOIN ple_data.assignment AS assignment ON assignment.assignment_id = attempt.assignment_id
      JOIN LATERAL ple_api.course_display_for_attempt(assignment.course_id) AS course ON true
     WHERE p_assignment_attempt_reference_number BETWEEN 1 AND 2147483647
       AND attempt.reference_number = p_assignment_attempt_reference_number
       AND (attempt.completed_at IS NULL OR EXISTS (
           SELECT 1 FROM ple_private.assignment_submission AS submission
            WHERE submission.assignment_attempt_id = attempt.assignment_attempt_id
       ))
       AND ple_api.current_session_account_owns_student_record(
           assignment.course_id, attempt.student_record_id
       )
$$;

REVOKE ALL ON FUNCTION ple_private.read_student_assignment_access(bigint, bigint), ple_private.read_active_student_assignment_attempt_reference(bigint, bigint), ple_private.read_reusable_question_pool_selection(uuid, uuid, uuid), ple_private.read_student_assignment_attempt_context(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.read_student_assignment_access(bigint, bigint), ple_private.read_active_student_assignment_attempt_reference(bigint, bigint), ple_private.read_reusable_question_pool_selection(uuid, uuid, uuid), ple_private.read_student_assignment_attempt_context(bigint) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.read_student_assignment_access(bigint, bigint) RETURNS TABLE (start_decision text, assignment_title text, question_count integer, points_possible double precision, assignment_attempt_time_limit_seconds integer, previous_attempts jsonb) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$ SELECT * FROM ple_private.read_student_assignment_access($1, $2) $$;
CREATE FUNCTION ple_api.read_active_student_assignment_attempt_reference(bigint, bigint) RETURNS TABLE (assignment_attempt_reference_number bigint) LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$ SELECT * FROM ple_private.read_active_student_assignment_attempt_reference($1, $2) $$;
CREATE FUNCTION ple_api.read_reusable_question_pool_selection(uuid, uuid, uuid) RETURNS TABLE (question_pool_selection_id uuid, selection_position integer, question_pool_item_id uuid, question_id text, revision_number integer) LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$ SELECT * FROM ple_private.read_reusable_question_pool_selection($1, $2, $3) $$;
CREATE FUNCTION ple_api.read_student_assignment_attempt_context(bigint) RETURNS TABLE (assignment_attempt_reference_number bigint, attempt_number integer, course_reference_number bigint, course_short_name text, course_long_name text, course_theme text, assignment_reference_number bigint, assignment_title text, timer_remaining_milliseconds bigint) LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$ SELECT * FROM ple_private.read_student_assignment_attempt_context($1) $$;
REVOKE ALL ON FUNCTION ple_api.read_student_assignment_access(bigint, bigint), ple_api.read_active_student_assignment_attempt_reference(bigint, bigint), ple_api.read_reusable_question_pool_selection(uuid, uuid, uuid), ple_api.read_student_assignment_attempt_context(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_student_assignment_access(bigint, bigint), ple_api.read_active_student_assignment_attempt_reference(bigint, bigint), ple_api.read_reusable_question_pool_selection(uuid, uuid, uuid), ple_api.read_student_assignment_attempt_context(bigint) TO ple_app;
RESET ROLE;
