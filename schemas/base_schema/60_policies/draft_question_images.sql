-- Row security policies from draft_question_images.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.draft_question_image ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.draft_question_image FORCE ROW LEVEL SECURITY;

CREATE POLICY draft_question_image_private_owner_access ON ple_private.draft_question_image
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

