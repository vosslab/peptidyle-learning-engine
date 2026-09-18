-- Row security policies from course_blueprint_publication.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.blueprint_course_instance_source ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course_instance_source FORCE ROW LEVEL SECURITY;

CREATE POLICY blueprint_course_instance_source_api_owner_all
    ON ple_data.blueprint_course_instance_source TO ple_api_owner
    USING (true) WITH CHECK (true);

