-- Row security policies from draft_question_assets.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.draft_question_asset ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.draft_question_asset FORCE ROW LEVEL SECURITY;

CREATE POLICY draft_question_asset_private_owner_access ON ple_private.draft_question_asset
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

