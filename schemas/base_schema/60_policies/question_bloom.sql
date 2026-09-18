-- Row security policies from question_bloom.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.question_revision_bloom ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_revision_bloom FORCE ROW LEVEL SECURITY;

CREATE POLICY question_revision_bloom_owner_access ON ple_data.question_revision_bloom
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_revision_bloom_private_read ON ple_data.question_revision_bloom
    FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY question_revision_bloom_private_insert ON ple_data.question_revision_bloom
    FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY question_revision_bloom_private_update ON ple_data.question_revision_bloom
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);

ALTER TABLE ple_data.question_pool_bloom ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_pool_bloom FORCE ROW LEVEL SECURITY;

CREATE POLICY question_pool_bloom_owner_access ON ple_data.question_pool_bloom
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY question_pool_bloom_private_read ON ple_data.question_pool_bloom
    FOR SELECT TO ple_private_owner USING (true);

-- Pool Library projections expose only the exact pair and its correction
-- precondition through session-authorized SECURITY DEFINER readers.
CREATE POLICY question_pool_bloom_api_projection ON ple_data.question_pool_bloom
    FOR SELECT TO ple_api_owner USING (true);

CREATE POLICY question_pool_bloom_private_insert ON ple_data.question_pool_bloom
    FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY question_pool_bloom_private_update ON ple_data.question_pool_bloom
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.bloom_preparation_receipt ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.bloom_preparation_receipt FORCE ROW LEVEL SECURITY;

CREATE POLICY bloom_preparation_receipt_private_owner_access
    ON ple_private.bloom_preparation_receipt
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
