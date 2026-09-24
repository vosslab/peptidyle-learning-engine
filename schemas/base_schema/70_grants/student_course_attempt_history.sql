-- Privileges from student_course_attempt_history.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.read_student_course_attempt_history(text, uuid, text, integer)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.read_student_course_attempt_history(text, uuid, text, integer)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_live_student_course_attempt_history(text, text, integer)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_live_student_course_attempt_history(text, text, integer)
    TO ple_app;
