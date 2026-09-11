-- Expose the existing Course Theme only in Instructor-authorized Course summaries.
-- A list row may identify its Course; it does not acquire Course route scope.

SET LOCAL ROLE ple_api_owner;

DROP FUNCTION ple_api.list_live_demo_course_instances();

CREATE FUNCTION ple_api.list_live_demo_course_instances()
RETURNS TABLE (
    reference_number bigint,
    short_name text,
    long_name text,
    term_starts_on date,
    term_ends_on date,
    course_time_zone text,
    course_theme text
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course.reference_number, course.course_short_name, course.course_long_name,
           schedule.term_starts_on, schedule.term_ends_on, schedule.course_time_zone,
           course.course_theme
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership
        ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_schedule_revision AS schedule
        ON schedule.course_id = course.course_id
       AND schedule.revision_number = 1
     WHERE ple_api.current_session_account_is_instructor()
     ORDER BY course.course_long_name, course.reference_number
$$;

DROP FUNCTION ple_api.load_live_demo_course_instance(bigint);

CREATE FUNCTION ple_api.load_live_demo_course_instance(p_reference_number bigint)
RETURNS TABLE (
    reference_number bigint,
    short_name text,
    long_name text,
    term_starts_on date,
    term_ends_on date,
    course_time_zone text,
    course_theme text,
    is_assigned_instructor boolean,
    active_instructor_count bigint
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course.reference_number, course.course_short_name, course.course_long_name,
           schedule.term_starts_on, schedule.term_ends_on, schedule.course_time_zone,
           course.course_theme,
           course.assigned_instructor_account_id = ple_api.current_session_account_id(),
           (
               SELECT count(*)
                 FROM ple_data.course_membership AS team_member
                WHERE team_member.course_id = course.course_id
                  AND team_member.role = 'instructor'
                  AND ple_data.course_membership_is_active(team_member.membership_id)
           )
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership
        ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_schedule_revision AS schedule
        ON schedule.course_id = course.course_id
       AND schedule.revision_number = 1
     WHERE p_reference_number BETWEEN 1 AND 2147483647
       AND course.reference_number = p_reference_number
       AND ple_api.current_session_account_is_instructor()
$$;

REVOKE ALL ON FUNCTION ple_api.list_live_demo_course_instances() FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.load_live_demo_course_instance(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_live_demo_course_instances(),
    ple_api.load_live_demo_course_instance(bigint) TO ple_app;

RESET ROLE;
