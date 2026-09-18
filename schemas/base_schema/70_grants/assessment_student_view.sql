-- Privileges from assessment_student_view.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.load_instructor_student_view_source_binding(
    text, integer
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.load_instructor_student_view_source_binding(
    text, integer
) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.read_instructor_student_view_duration_seconds(text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_instructor_student_view_duration_seconds(text, text) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.load_instructor_student_view_question_source(
    text, text, bigint, integer, text, integer
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.load_instructor_student_view_question_source(
    text, text, bigint, integer, text, integer
) TO ple_app;

