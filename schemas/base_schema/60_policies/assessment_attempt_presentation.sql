-- Row security policies from assessment_attempt_presentation.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.question_attempt_presentation_binding ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_attempt_presentation_binding FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_attempt_response_item_binding ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_attempt_response_item_binding FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_attempt_presentation_asset_binding ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_attempt_presentation_asset_binding FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_attempt_presentation_asset_rendition ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_attempt_presentation_asset_rendition FORCE ROW LEVEL SECURITY;

CREATE POLICY question_attempt_presentation_binding_private_owner_access ON ple_private.question_attempt_presentation_binding FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY question_attempt_response_item_binding_private_owner_access ON ple_private.question_attempt_response_item_binding FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY question_attempt_presentation_asset_binding_private_owner_access ON ple_private.question_attempt_presentation_asset_binding FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY question_attempt_presentation_asset_rendition_private_owner_access ON ple_private.question_attempt_presentation_asset_rendition FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

