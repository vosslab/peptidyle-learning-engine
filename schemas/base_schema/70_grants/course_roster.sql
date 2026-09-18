-- Privileges from course_roster.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.course_invitation,ple_private.course_invitation_event,ple_private.course_roster_profile FROM PUBLIC;


-- Invitation acceptance locks its immutable invitation row so concurrent
-- claims serialize before the unique acceptance event is inserted.  UPDATE is
-- used only for that row lock; the immutability trigger rejects data changes.
GRANT SELECT, INSERT, UPDATE (invitation_id) ON ple_private.course_invitation TO ple_api_owner;

GRANT SELECT, INSERT ON ple_private.course_invitation_event,
    ple_private.course_roster_profile TO ple_api_owner;

GRANT UPDATE (roster_name) ON ple_private.course_roster_profile TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.reject_course_invitation_change(),ple_private.reject_course_invitation_event_change(),ple_private.reject_course_roster_profile_change(),ple_private.assert_course_invitation_event_is_valid(),ple_private.pending_course_invitation_delivery_email(uuid, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.pending_course_invitation_delivery_email(uuid, uuid)
    TO ple_api_owner;

SET LOCAL ROLE ple_audit_owner;

REVOKE ALL ON TABLE ple_audit.course_roster_event FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_audit.reject_course_roster_event_change(),
    ple_audit.record_course_roster_event(uuid, uuid, uuid, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_audit.record_course_roster_event(uuid, uuid, uuid, text)
    TO ple_api_owner;

