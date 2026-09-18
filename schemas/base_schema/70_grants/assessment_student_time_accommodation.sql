-- Privileges from assessment_student_time_accommodation.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.student_assessment_time_configuration(uuid, text, text, boolean, bigint, numeric) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.student_assessment_time_configuration(uuid, text, text, boolean, bigint, numeric) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.student_assessment_time_configuration(text, text, text, boolean, bigint, numeric) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.student_assessment_time_configuration(text, text, text, boolean, bigint, numeric) TO ple_app;

