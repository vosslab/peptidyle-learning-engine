-- Row security policies from question_thumbnails.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.question_revision_thumbnail ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_revision_thumbnail FORCE ROW LEVEL SECURITY;

CREATE POLICY question_revision_thumbnail_private_owner_access
    ON ple_private.question_revision_thumbnail
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
