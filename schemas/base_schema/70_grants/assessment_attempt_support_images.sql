-- This internal server operation has no browser request route.

SET LOCAL ROLE ple_private_owner;
REVOKE ALL ON FUNCTION ple_private.register_student_assessment_attempt_support_images(uuid, integer, text, integer, jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.register_student_assessment_attempt_support_images(uuid, integer, text, integer, jsonb) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;
REVOKE ALL ON FUNCTION ple_api.register_student_assessment_attempt_support_images(uuid, integer, text, integer, jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.register_student_assessment_attempt_support_images(uuid, integer, text, integer, jsonb) TO ple_app;
