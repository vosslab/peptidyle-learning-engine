-- Privileges from assessment_attempt_operations.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.assessment_effective_duration_seconds(text, numeric),
    ple_private.assessment_has_started_attempts(text),
    ple_private.assessment_question_has_been_issued(text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.assessment_effective_duration_seconds(text, numeric)
    TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.assessment_has_started_attempts(text)
    TO ple_data_owner;
GRANT EXECUTE ON FUNCTION ple_private.assessment_question_has_been_issued(text, text)
    TO ple_data_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.current_session_student_record_id(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.current_session_student_record_id(text) TO ple_private_owner;

REVOKE ALL ON FUNCTION ple_api.course_instance_id_for_assessment_attempt(text),
    ple_api.course_display_for_assessment_attempt(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.course_instance_id_for_assessment_attempt(text),
    ple_api.course_display_for_assessment_attempt(text) TO ple_private_owner;
