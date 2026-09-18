-- Privileges from course_retention.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.retention_schedule(),
    ple_data.course_retention_due_actions(timestamp with time zone) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.retention_schedule()
    TO ple_api_owner, ple_course_retention_executor,
    ple_course_retention_notification_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.course_student_work_is_ordinarily_visible(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.course_student_work_is_ordinarily_visible(text)
    TO ple_private_owner;

