-- M18 protected direct-Instructor export of pending Student Course Invitations.
--
-- This exposes no invitation IDs, account IDs, events, tokens, or general
-- roster state. The route supplies one deployment-owned shared signup URL.

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.export_live_demo_pending_course_invitations(
    p_course_reference_number bigint
)
RETURNS TABLE (roster_email text, roster_id text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_course_id uuid;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Invitation export arguments are invalid';
    END IF;

    -- ASVS 4.1.3: the procedure, rather than the route role check, is the
    -- final authority for exact current Instructor Course Membership.
    SELECT course.course_id
      INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Invitation export requires a current Instructor Course Membership';
    END IF;

    RETURN QUERY
    SELECT profile.roster_email, profile.roster_id
      FROM ple_private.course_roster_profile AS profile
      JOIN ple_private.course_invitation AS invitation
        ON invitation.course_id = profile.course_id
       AND invitation.target_account_id = profile.student_account_id
       AND invitation.membership_role = 'student'
     WHERE profile.course_id = v_course_id
       AND invitation.expires_at > pg_catalog.clock_timestamp()
       AND NOT EXISTS (
           SELECT 1
             FROM ple_private.course_invitation_event AS event
            WHERE event.invitation_id = invitation.invitation_id
       )
     ORDER BY profile.roster_id;
END
$$;

CREATE FUNCTION ple_api.load_live_demo_invitation_export_course(
    p_course_reference_number bigint
)
RETURNS TABLE (course_name text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    v_course_id uuid;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Invitation export arguments are invalid';
    END IF;
    SELECT course.course_id
      INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.reference_number = p_course_reference_number;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Invitation export requires a current Instructor Course Membership';
    END IF;
    RETURN QUERY
    SELECT course.course_title
      FROM ple_data.course_instance AS course
     WHERE course.course_id = v_course_id;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.export_live_demo_pending_course_invitations(bigint)
    FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.load_live_demo_invitation_export_course(bigint)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.export_live_demo_pending_course_invitations(bigint) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.load_live_demo_invitation_export_course(bigint) TO ple_app;

RESET ROLE;
