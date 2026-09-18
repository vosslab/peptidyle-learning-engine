-- Privileges from course_retention.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.course_retention_policy FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.course_retention_due_actions(timestamp with time zone) FROM PUBLIC;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.course_student_work_is_ordinarily_visible(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.course_student_work_is_ordinarily_visible(uuid)
    TO ple_private_owner;

