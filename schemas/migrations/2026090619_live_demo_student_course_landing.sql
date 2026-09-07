-- M19 Student Course Landing: answer-free Student membership and invitation projections.
--
-- Migration 0619 has not reached an accepted, data-bearing environment, so
-- this pending M19 baseline is corrected here rather than by a 0620 follow-up.
-- M3 is accepted and immutable. The procedures derive the active Student
-- Account and exact current entitlement server-side.

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_live_student_course_landing()
RETURNS TABLE (course_reference_number bigint, course_title text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT course.reference_number, course.course_title
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS account_state ON account_state.state = 'active'
      JOIN ple_data.course_membership AS membership
        ON membership.account_id = account.account_id
       AND membership.role = 'student'
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_instance AS course ON course.course_id = membership.course_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'student'
     ORDER BY course.course_title, course.reference_number
$$;

CREATE FUNCTION ple_api.list_pending_live_student_course_invitations()
RETURNS TABLE (course_reference_number bigint, course_title text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    -- ASVS 2.2.1/2.2.2 and 2.3.1: derive the active Student Account here;
    -- choose the same current invitation as the claim transaction, but return
    -- only its Course Instance reference and title.
    SELECT course.reference_number, course.course_title
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS account_state ON account_state.state = 'active'
      JOIN LATERAL (
          SELECT DISTINCT ON (invitation.course_id) invitation.course_id
            FROM ple_private.course_invitation AS invitation
           WHERE invitation.target_account_id = account.account_id
             AND invitation.membership_role = 'student'
             AND invitation.expires_at > pg_catalog.clock_timestamp()
             AND NOT EXISTS (
                 SELECT 1
                   FROM ple_private.course_invitation_event AS event
                  WHERE event.invitation_id = invitation.invitation_id
             )
             AND NOT EXISTS (
                 SELECT 1
                   FROM ple_data.course_membership AS membership
                  WHERE membership.course_id = invitation.course_id
                    AND membership.account_id = account.account_id
                    AND membership.role = 'student'
                    AND ple_data.course_membership_is_active(membership.membership_id)
             )
           ORDER BY invitation.course_id, invitation.issued_at DESC, invitation.invitation_id DESC
      ) AS pending_invitation ON true
      JOIN ple_data.course_instance AS course
        ON course.course_id = pending_invitation.course_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'student'
     ORDER BY course.course_title, course.reference_number
$$;

CREATE FUNCTION ple_api.list_released_live_student_assignments(
    p_course_reference_number bigint
)
RETURNS TABLE (assignment_reference_number bigint, assignment_title text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_course_id uuid;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647 THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course is unavailable';
    END IF;

    SELECT course.course_id
      INTO v_course_id
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS account_state ON account_state.state = 'active'
      JOIN ple_data.course_membership AS membership
        ON membership.account_id = account.account_id
       AND membership.role = 'student'
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_instance AS course ON course.course_id = membership.course_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'student'
       AND course.reference_number = p_course_reference_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course is unavailable';
    END IF;

    RETURN QUERY
    SELECT assignment.reference_number, assignment.assignment_title
      FROM ple_data.assignment AS assignment
     WHERE assignment.course_id = v_course_id
       AND assignment.assignment_status = 'released'
     ORDER BY assignment.assignment_title, assignment.reference_number;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_live_student_course_landing() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_pending_live_student_course_invitations() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_released_live_student_assignments(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_live_student_course_landing() TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.list_pending_live_student_course_invitations() TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.list_released_live_student_assignments(bigint) TO ple_app;

RESET ROLE;
