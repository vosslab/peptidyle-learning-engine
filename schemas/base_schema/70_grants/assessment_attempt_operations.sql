-- Privileges from assessment_attempt_operations.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.assessment_effective_duration_seconds(text, numeric) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.assessment_effective_duration_seconds(text, numeric)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.current_session_student_record_id(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.current_session_student_record_id(text) TO ple_private_owner;

REVOKE ALL ON FUNCTION ple_api.course_reference_number_for_assessment_attempt(text),
    ple_api.course_display_for_assessment_attempt(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.course_reference_number_for_assessment_attempt(text),
    ple_api.course_display_for_assessment_attempt(text) TO ple_private_owner;

