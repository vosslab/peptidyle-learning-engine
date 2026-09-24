-- Privileges from student_course_practice_stats.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.read_student_course_practice_stats(text, uuid)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.read_student_course_practice_stats(text, uuid)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_live_student_course_practice_stats(text)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_live_student_course_practice_stats(text)
    TO ple_app;
