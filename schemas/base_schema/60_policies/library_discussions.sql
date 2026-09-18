-- Row security policies from library_discussions.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.library_improvement_thread ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.library_improvement_thread FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.library_improvement_post ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.library_improvement_post FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.library_impact_notice ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.library_impact_notice FORCE ROW LEVEL SECURITY;

CREATE POLICY library_improvement_thread_data_owner_access
    ON ple_data.library_improvement_thread FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);

CREATE POLICY library_improvement_post_data_owner_access
    ON ple_data.library_improvement_post FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);

CREATE POLICY library_impact_notice_data_owner_access
    ON ple_data.library_impact_notice FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);

