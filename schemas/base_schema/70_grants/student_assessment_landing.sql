-- Privileges from student_assessment_landing.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.read_student_released_assessment_landing_evidence(text, uuid, timestamptz) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.read_student_released_assessment_landing_evidence(text, uuid, timestamptz) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_released_live_student_assessments(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_released_live_student_assessments(text) TO ple_app;

