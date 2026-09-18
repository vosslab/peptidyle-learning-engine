-- Row security policies from blueprints.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.blueprint_course ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course_revision ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course_revision FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_revision_question_pin ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_revision_question_pin FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_revision_module ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_revision_module FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_revision_assessment ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_revision_assessment FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_revision_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_revision_event FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_metadata_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_metadata_event FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course_create_receipt ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course_create_receipt FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course_save_receipt ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course_save_receipt FORCE ROW LEVEL SECURITY;

CREATE POLICY blueprint_api_owner_all ON ple_data.blueprint_course TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY blueprint_revision_api_owner_all ON ple_data.blueprint_course_revision TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY blueprint_revision_pin_api_owner_all ON ple_data.blueprint_revision_question_pin TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY blueprint_revision_module_api_owner_all ON ple_data.blueprint_revision_module TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY blueprint_revision_assessment_api_owner_all ON ple_data.blueprint_revision_assessment TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY blueprint_revision_event_api_owner_all ON ple_data.blueprint_revision_event TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY blueprint_metadata_event_api_owner_all ON ple_data.blueprint_metadata_event TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY blueprint_create_receipt_api_owner_all ON ple_data.blueprint_course_create_receipt TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY blueprint_save_receipt_api_owner_all ON ple_data.blueprint_course_save_receipt TO ple_api_owner USING (true) WITH CHECK (true);

ALTER TABLE ple_data.blueprint_course_fork ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course_fork FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course_fork_receipt ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.blueprint_course_fork_receipt FORCE ROW LEVEL SECURITY;

CREATE POLICY blueprint_course_fork_api_owner_all ON ple_data.blueprint_course_fork
    TO ple_api_owner USING (true) WITH CHECK (true);

CREATE POLICY blueprint_course_fork_receipt_api_owner_all
    ON ple_data.blueprint_course_fork_receipt
    TO ple_api_owner USING (true) WITH CHECK (true);

