-- Cross-Course Instructor Due Soon reader.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091028 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_assignments_due_soon()
RETURNS TABLE (
    course_reference_number bigint,
    course_title text,
    assignment_reference_number bigint,
    assignment_title text,
    assignment_status text,
    due_at_millis bigint
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    v_window_start timestamp with time zone := pg_catalog.statement_timestamp();
BEGIN
    -- One statement timestamp keeps both bounds of the rolling next-seven-days window coherent.
    RETURN QUERY
    SELECT course.reference_number, course.course_title, assignment.reference_number,
           assignment.assignment_title, assignment.assignment_status,
           floor(extract(epoch FROM assignment.due_at) * 1000)::bigint
      FROM ple_data.assignment AS assignment
      JOIN ple_data.course_instance AS course ON course.course_id = assignment.course_id
     WHERE ple_api.current_session_account_is_course_instructor(course.course_id)
       AND assignment.assignment_status IN ('unreleased', 'released')
       AND assignment.due_at >= v_window_start
       AND assignment.due_at < v_window_start + interval '7 days'
     ORDER BY assignment.due_at, course.reference_number, assignment.reference_number;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_assignments_due_soon() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_assignments_due_soon() TO ple_app;
RESET ROLE;
