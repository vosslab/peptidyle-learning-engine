-- Row security policies from question_pools.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.question_pool ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_pool FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_pool_revision ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_pool_revision FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_pool_revision_member ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_pool_revision_member FORCE ROW LEVEL SECURITY;

CREATE POLICY question_pool_data_owner_access ON ple_data.question_pool
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_pool_revision_data_owner_access ON ple_data.question_pool_revision
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_pool_revision_member_data_owner_access ON ple_data.question_pool_revision_member
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_pool_private_owner_lookup ON ple_data.question_pool
    FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY question_pool_api_owner_lookup ON ple_data.question_pool
    FOR SELECT TO ple_api_owner USING (true);

CREATE POLICY question_pool_revision_private_owner_lookup ON ple_data.question_pool_revision
    FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY question_pool_revision_api_owner_lookup ON ple_data.question_pool_revision
    FOR SELECT TO ple_api_owner USING (true);

CREATE POLICY question_pool_revision_member_private_owner_lookup ON ple_data.question_pool_revision_member
    FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY question_pool_revision_member_api_owner_lookup ON ple_data.question_pool_revision_member
    FOR SELECT TO ple_api_owner USING (true);

