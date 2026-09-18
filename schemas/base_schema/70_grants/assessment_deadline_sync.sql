-- Privileges from assessment_deadline_sync.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.synchronize_course_assessment_deadline(text) FROM PUBLIC;

