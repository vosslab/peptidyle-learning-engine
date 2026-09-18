-- Privileges from assessment_attempt_history.sql.

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.current_course_student_record_ids(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.current_course_student_record_ids(text) TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.current_student_cohort_completed_assessment(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.current_student_cohort_completed_assessment(text) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.read_student_assessment_attempt_history(bigint) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.read_student_assessment_attempt_history(bigint) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.read_student_assessment_attempt_history(bigint) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_student_assessment_attempt_history(bigint) TO ple_app;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.read_student_assessment_attempt_history_response_sources(bigint)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.read_student_assessment_attempt_history_response_sources(bigint)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.read_student_assessment_attempt_history_response_sources(bigint)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_student_assessment_attempt_history_response_sources(bigint)
    TO ple_app;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.read_course_student_work_for_retention(uuid)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.read_course_student_work_for_retention(uuid)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.read_archived_course_student_work_for_retention(text)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_archived_course_student_work_for_retention(text)
    TO ple_course_retention_executor;

