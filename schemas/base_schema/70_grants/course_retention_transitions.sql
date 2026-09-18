-- Privileges from course_retention_transitions.sql.

SET LOCAL ROLE ple_data_owner;

GRANT SELECT, UPDATE ON ple_data.course_instance TO ple_course_retention_executor;

GRANT SELECT ON ple_data.course_retention_policy TO ple_course_retention_executor;

GRANT SELECT, UPDATE (assessment_id) ON ple_data.assessment TO ple_course_retention_executor;

GRANT SELECT, DELETE ON ple_data.student_record, ple_data.course_membership,
    ple_data.course_membership_event TO ple_course_retention_executor;

SET LOCAL ROLE ple_private_owner;

GRANT SELECT, DELETE ON ple_private.assessment_attempt,
    ple_private.student_assessment_accommodation,
    ple_private.course_invitation,
    ple_private.course_invitation_event,
    ple_private.course_roster_profile TO ple_course_retention_executor;

SET LOCAL ROLE ple_audit_owner;

GRANT SELECT, DELETE ON ple_audit.course_roster_event TO ple_course_retention_executor;

SET LOCAL ROLE ple_data_owner;





-- The no-login capability owns these procedures.  C215 may grant execution
-- only to its worker profile; no session role receives table privileges.
GRANT EXECUTE ON FUNCTION ple_data.course_retention_due_actions(timestamp with time zone)
    TO ple_course_retention_executor;

SET LOCAL ROLE ple_course_retention_executor;



-- ASVS 8.2.1: these functions belong only to the isolated executor capability.
REVOKE ALL ON FUNCTION ple_api.mark_course_instance_inactive(uuid, timestamp with time zone),
    ple_api.archive_course_student_records(uuid, timestamp with time zone),
    ple_api.delete_course_student_records(uuid, timestamp with time zone) FROM PUBLIC;

SET LOCAL ROLE ple_api_owner;

REVOKE CREATE ON SCHEMA ple_api FROM ple_course_retention_executor;

