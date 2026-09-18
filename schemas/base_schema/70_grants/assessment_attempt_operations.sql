-- Privileges from assessment_attempt_operations.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.assessment_effective_duration_seconds(uuid, numeric) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.assessment_effective_duration_seconds(uuid, numeric)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.current_session_student_record_id(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.current_session_student_record_id(uuid) TO ple_private_owner;

REVOKE ALL ON FUNCTION ple_api.course_reference_number_for_assessment_attempt(uuid),
    ple_api.course_display_for_assessment_attempt(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.course_reference_number_for_assessment_attempt(uuid),
    ple_api.course_display_for_assessment_attempt(uuid) TO ple_private_owner;

