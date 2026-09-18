-- Row security policies from question_assets.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.question_asset_delivery ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.question_asset_delivery FORCE ROW LEVEL SECURITY;

CREATE POLICY question_asset_delivery_data_owner_access ON ple_data.question_asset_delivery
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.question_asset_publication ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_asset_publication FORCE ROW LEVEL SECURITY;

CREATE POLICY question_asset_publication_private_owner_access ON ple_private.question_asset_publication
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_data_owner;

CREATE POLICY question_asset_publication_private_delivery_read ON ple_data.object_delivery
    FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY question_asset_publication_private_delivery_update ON ple_data.object_delivery
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY question_asset_publication_private_delivery_insert ON ple_data.object_delivery
    FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY question_asset_publication_private_asset_delivery_read ON ple_data.question_asset_delivery
    FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY question_asset_publication_private_asset_delivery_insert ON ple_data.question_asset_delivery
    FOR INSERT TO ple_private_owner WITH CHECK (true);

