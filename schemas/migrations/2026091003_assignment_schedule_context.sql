-- Resolves Assignment Workspace wall-clock input in the authenticated
-- Instructor Account zone; browser input supplies neither a zone nor an Account ID.

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.load_live_demo_assignment_schedule_context(
    p_course_reference_number bigint
)
RETURNS TABLE (term_starts_on date, term_ends_on date, account_time_zone text)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_course_id uuid;
BEGIN
    SELECT course.course_id INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number;
    -- ASVS 8.2.2/8.3.1: authorize the installed session before disclosing
    -- either calendar data or the authenticated Account preference.
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RETURN;
    END IF;
    RETURN QUERY
    SELECT schedule.term_starts_on, schedule.term_ends_on,
           ple_private.current_authenticated_account_time_zone()
      FROM LATERAL (
          SELECT revision.term_starts_on, revision.term_ends_on
            FROM ple_data.course_schedule_revision AS revision
           WHERE revision.course_id = v_course_id
           ORDER BY revision.revision_number DESC
           LIMIT 1
      ) AS schedule;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.load_live_demo_assignment_schedule_context(bigint)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.load_live_demo_assignment_schedule_context(bigint) TO ple_app;

RESET ROLE;
