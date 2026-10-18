-- Privileges from student_course_attempt_history.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.student_attempt_history_matches(text) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_private.read_student_attempt_history(
    bigint, integer, text, text, text[], text
) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_private.student_attempt_history_facet_choices(
    text, text, text[], text, text, text, integer
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.read_student_attempt_history(
    bigint, integer, text, text, text[], text
) TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.student_attempt_history_facet_choices(
    text, text, text[], text, text, text, integer
) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_student_attempt_history(
    bigint, integer, text, text, text[], text
) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.list_student_attempt_history_facet_choices(
    text, text, text[], text, text, text, integer
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_student_attempt_history(
    bigint, integer, text, text, text[], text
) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.list_student_attempt_history_facet_choices(
    text, text, text[], text, text, text, integer
) TO ple_app;
