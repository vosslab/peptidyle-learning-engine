-- Row security policies from content_classification.sql.

SET LOCAL ROLE ple_data_owner;



-- ASVS 8.2.1/8.2.2: no runtime/API direct reads or writes are exposed by
-- the foundation. SQL ownership is an installation capability, not Sysadmin
-- Product Role authorization. Authenticated operations grant the private command
-- owner explicit access separately; API/runtime roles retain no table grants.
ALTER TABLE ple_data.content_discipline ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.content_discipline FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.content_subject ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.content_subject FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.content_subject_discipline ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.content_subject_discipline FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.content_topic ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.content_topic FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.content_subtopic ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.content_subtopic FORCE ROW LEVEL SECURITY;

CREATE POLICY content_discipline_data_owner_access ON ple_data.content_discipline
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY content_subject_data_owner_access ON ple_data.content_subject
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY content_subject_discipline_data_owner_access ON ple_data.content_subject_discipline
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY content_topic_data_owner_access ON ple_data.content_topic
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY content_subtopic_data_owner_access ON ple_data.content_subtopic
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

