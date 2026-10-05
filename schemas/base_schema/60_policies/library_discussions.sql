-- Row security policies from library impact notices.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.library_impact_notice ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.library_impact_notice FORCE ROW LEVEL SECURITY;

CREATE POLICY library_impact_notice_data_owner_access
    ON ple_data.library_impact_notice FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);
