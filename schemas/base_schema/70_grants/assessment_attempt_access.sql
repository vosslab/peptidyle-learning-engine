-- Privileges from assessment_attempt_access.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.assessment_start_decision(ple_data.assessment_status, timestamptz, timestamptz, timestamptz, integer, integer, ple_data.late_work_rule, timestamptz), ple_private.read_student_assessment_access(text, text), ple_private.read_active_student_assessment_attempt_id(text, text), ple_private.read_student_assessment_attempt_pool_selection(uuid), ple_private.read_student_assessment_attempt_context(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.read_student_assessment_access(text, text), ple_private.read_active_student_assessment_attempt_id(text, text), ple_private.read_student_assessment_attempt_pool_selection(uuid), ple_private.read_student_assessment_attempt_context(uuid) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.read_student_assessment_access(text, text), ple_api.read_active_student_assessment_attempt_id(text, text), ple_api.read_student_assessment_attempt_pool_selection(uuid), ple_api.read_student_assessment_attempt_context(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_student_assessment_access(text, text), ple_api.read_active_student_assessment_attempt_id(text, text), ple_api.read_student_assessment_attempt_pool_selection(uuid), ple_api.read_student_assessment_attempt_context(uuid) TO ple_app;

