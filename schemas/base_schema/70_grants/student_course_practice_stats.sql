-- Privileges for the Student Course Response Stats functions.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.read_student_course_response_stats(text, uuid)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.read_student_course_response_stats(text, uuid)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_live_student_course_response_stats(text)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_live_student_course_response_stats(text)
    TO ple_app;
