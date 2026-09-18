-- Privileges from assessment_attempt_access.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.assessment_start_decision(text, timestamptz, timestamptz, timestamptz, integer, integer, text, timestamptz), ple_private.read_student_assessment_access(bigint, text), ple_private.read_active_student_assessment_attempt_reference(bigint, text), ple_private.read_student_assessment_attempt_pool_selection(bigint), ple_private.read_student_assessment_attempt_context(bigint) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.read_student_assessment_access(bigint, text), ple_private.read_active_student_assessment_attempt_reference(bigint, text), ple_private.read_student_assessment_attempt_pool_selection(bigint), ple_private.read_student_assessment_attempt_context(bigint) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.read_student_assessment_access(text, text), ple_api.read_active_student_assessment_attempt_reference(text, text), ple_api.read_student_assessment_attempt_pool_selection(bigint), ple_api.read_student_assessment_attempt_context(bigint) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_student_assessment_access(text, text), ple_api.read_active_student_assessment_attempt_reference(text, text), ple_api.read_student_assessment_attempt_pool_selection(bigint), ple_api.read_student_assessment_attempt_context(bigint) TO ple_app;

