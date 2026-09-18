-- Row security policies from archived_student_work_recovery.sql.

SET LOCAL ROLE ple_data_owner;

CREATE POLICY course_retention_policy_api_owner_recovery_read
    ON ple_data.course_retention_policy FOR SELECT TO ple_api_owner USING (true);

