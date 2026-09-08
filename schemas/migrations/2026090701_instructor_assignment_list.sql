-- Ordinary Instructor Assignment listing for one exact Course Instance.

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_course_assignments(p_course_reference_number bigint)
RETURNS TABLE (
    assignment_reference_number bigint,
    assignment_title text,
    assignment_status text,
    assignment_edit_number bigint
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    v_course_id uuid;
BEGIN
    -- ASVS 8.2.1, 8.2.2, and 8.3.1: derive the current Account from the
    -- installed session and require its exact active Instructor Course
    -- Membership before projecting this Course Instance's Assignments.
    SELECT course.course_id
      INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING
            ERRCODE = '42501',
            MESSAGE = 'Course Assignments require a current Instructor Course Membership';
    END IF;

    RETURN QUERY
    SELECT assignment.reference_number,
           assignment.assignment_title,
           assignment.assignment_status,
           assignment.assignment_edit_number
      FROM ple_data.assignment AS assignment
     WHERE assignment.course_id = v_course_id
     ORDER BY assignment.assignment_title, assignment.reference_number;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_course_assignments(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_course_assignments(bigint) TO ple_app;

RESET ROLE;
