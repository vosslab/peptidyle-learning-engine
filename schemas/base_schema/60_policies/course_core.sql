-- Row security policies from course_core.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.course_instance ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_instance FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_origin ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.course_origin FORCE ROW LEVEL SECURITY;

CREATE POLICY course_instance_api_owner_access ON ple_data.course_instance
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY course_instance_data_owner_deadline_schedule ON ple_data.course_instance
    FOR UPDATE TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY course_origin_api_owner_access ON ple_data.course_origin
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_audit_owner;

ALTER TABLE ple_audit.course_instance_creation_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.course_instance_creation_event FORCE ROW LEVEL SECURITY;

CREATE POLICY course_instance_creation_event_owner_write
    ON ple_audit.course_instance_creation_event FOR INSERT TO ple_audit_owner WITH CHECK (true);

