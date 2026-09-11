-- Browser-safe current Course Summary for an exact active Course Member.
--
-- The endpoint accepts an opaque UUID, so membership authorization and the
-- caller's exact Course Membership Role are both derived in this procedure.

SET LOCAL ROLE ple_api_owner;

-- ASVS 1.2.3, 8.2.2, and 8.3.1: conceal every nonmember by requiring the
-- installed session's exact active Course Membership in the trusted database
-- boundary. The row join derives the caller's current membership role rather
-- than accepting a client-selected role or relying on observer-permitting RLS.
CREATE FUNCTION ple_api.read_course_summary(p_course_id uuid)
RETURNS TABLE (
    course_id uuid,
    reference_number bigint,
    short_name text,
    long_name text,
    term_starts_on date,
    term_ends_on date,
    course_time_zone text,
    membership_role text
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course.course_id,
           course.reference_number,
           course.course_short_name,
           course.course_long_name,
           schedule.term_starts_on,
           schedule.term_ends_on,
           schedule.course_time_zone,
           membership.role
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership
        ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_schedule_revision AS schedule
        ON schedule.course_id = course.course_id
       AND schedule.revision_number = 1
     WHERE course.course_id = p_course_id
       AND ple_api.current_session_account_is_course_member(p_course_id)
$$;

REVOKE ALL ON FUNCTION ple_api.read_course_summary(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_course_summary(uuid) TO ple_app;

-- ASVS 1.2.3, 8.2.2, and 8.3.1: this opaque C-reference resolver uses the
-- same installed-session active Course Membership boundary as Course Summary.
CREATE FUNCTION ple_api.resolve_course_navigation(p_reference_number bigint)
RETURNS TABLE (course_id uuid)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course.course_id
      FROM ple_data.course_instance AS course
     WHERE p_reference_number BETWEEN 1 AND 2147483647
       AND course.reference_number = p_reference_number
       AND ple_api.current_session_account_is_course_member(course.course_id)
$$;

REVOKE ALL ON FUNCTION ple_api.resolve_course_navigation(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.resolve_course_navigation(bigint) TO ple_app;

RESET ROLE;
