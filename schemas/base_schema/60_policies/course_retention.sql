-- Row security policies from course_retention.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.course_retention_policy ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_retention_policy FORCE ROW LEVEL SECURITY;

CREATE POLICY course_retention_policy_data_owner_access
    ON ple_data.course_retention_policy FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);

