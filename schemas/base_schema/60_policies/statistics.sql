-- Row security policies from statistics.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.question_revision_statistics ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_revision_statistics FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_pool_statistics ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_pool_statistics FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_pool_member_statistics ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_pool_member_statistics FORCE ROW LEVEL SECURITY;

CREATE POLICY question_revision_statistics_data_owner_access
    ON ple_data.question_revision_statistics
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_pool_statistics_data_owner_access
    ON ple_data.question_pool_statistics
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_pool_member_statistics_data_owner_access
    ON ple_data.question_pool_member_statistics
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_revision_statistics_api_read
    ON ple_data.question_revision_statistics FOR SELECT TO ple_api_owner USING (true);

CREATE POLICY question_pool_statistics_api_read
    ON ple_data.question_pool_statistics FOR SELECT TO ple_api_owner USING (true);

CREATE POLICY question_pool_member_statistics_api_read
    ON ple_data.question_pool_member_statistics FOR SELECT TO ple_api_owner USING (true);

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.question_statistics_observation_receipt ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_statistics_observation_receipt FORCE ROW LEVEL SECURITY;

CREATE POLICY question_statistics_receipt_private_owner_access
    ON ple_private.question_statistics_observation_receipt
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
