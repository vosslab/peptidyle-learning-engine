-- QSOM1 schema-foundation ownership split. Draft and Published Question
-- metadata have distinct owners and lifecycles; source bindings do too.
-- This is deliberately schema-only. The later P1 migration owns the first
-- publication transaction that creates the complete published aggregate.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.question_metadata_fields_are_valid(
    p_question_title text,
    p_question_description text
)
RETURNS boolean LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog AS $$
    SELECT char_length(btrim(p_question_title)) BETWEEN 1 AND 500
       AND char_length(btrim(p_question_description)) BETWEEN 1 AND 4000
$$;
GRANT EXECUTE ON FUNCTION ple_private.question_metadata_fields_are_valid(text, text)
    TO ple_data_owner;

CREATE TABLE ple_private.draft_question_metadata (
    draft_question_uuid uuid PRIMARY KEY
        REFERENCES ple_private.draft_question (draft_question_uuid) ON DELETE CASCADE,
    question_title text NOT NULL,
    question_description text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT draft_question_metadata_fields_are_valid CHECK (
        ple_private.question_metadata_fields_are_valid(question_title, question_description)
    ),
    CONSTRAINT draft_question_metadata_timestamps_are_ordered CHECK (updated_at >= created_at)
);

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.published_question_metadata (
    question_id text PRIMARY KEY
        REFERENCES ple_data.published_question (question_id) ON DELETE CASCADE,
    question_title text NOT NULL,
    question_description text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT published_question_metadata_fields_are_valid CHECK (
        ple_private.question_metadata_fields_are_valid(question_title, question_description)
    ),
    CONSTRAINT published_question_metadata_timestamps_are_ordered CHECK (updated_at >= created_at)
);

-- The old baseline had no independent Draft Question Description or Published
-- Question Title. These values only bridge an empty pre-production baseline;
-- future publication supplies validated values explicitly.
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.draft_question_metadata (
    draft_question_uuid, question_title, question_description, created_at, updated_at
)
SELECT draft_question_uuid, title, title, created_at, updated_at
  FROM ple_private.draft_question;
SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.published_question_metadata (
    question_id, question_title, question_description, created_at, updated_at
)
SELECT revision.question_id, revision.question_id, revision.question_description,
       revision.published_at, revision.published_at
  FROM ple_data.question_revision AS revision
 WHERE NOT EXISTS (
     SELECT 1 FROM ple_data.published_question_metadata AS metadata
      WHERE metadata.question_id = revision.question_id
 );

SET LOCAL ROLE ple_private_owner;
ALTER TABLE ple_private.draft_question_metadata ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_metadata FORCE ROW LEVEL SECURITY;
SET LOCAL ROLE ple_data_owner;
ALTER TABLE ple_data.published_question_metadata ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.published_question_metadata FORCE ROW LEVEL SECURITY;
SET LOCAL ROLE ple_private_owner;
CREATE POLICY draft_question_metadata_private_owner_access ON ple_private.draft_question_metadata
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY draft_question_metadata_workspace_access ON ple_private.draft_question_metadata
    FOR ALL TO ple_app
    USING (EXISTS (
        SELECT 1 FROM ple_private.draft_question AS question
         WHERE question.draft_question_uuid = draft_question_metadata.draft_question_uuid
           AND ple_api.current_session_account_can_access_authoring_workspace(question.workspace_id)
    ))
    WITH CHECK (EXISTS (
        SELECT 1 FROM ple_private.draft_question AS question
         WHERE question.draft_question_uuid = draft_question_metadata.draft_question_uuid
           AND ple_api.current_session_account_can_access_authoring_workspace(question.workspace_id)
    ));
SET LOCAL ROLE ple_data_owner;
CREATE POLICY published_question_metadata_data_owner_access ON ple_data.published_question_metadata
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY published_question_metadata_api_owner_read ON ple_data.published_question_metadata
    FOR SELECT TO ple_api_owner USING (true);
REVOKE ALL PRIVILEGES ON TABLE ple_data.published_question_metadata FROM PUBLIC;
GRANT SELECT ON TABLE ple_data.published_question_metadata TO ple_api_owner;

SET LOCAL ROLE ple_private_owner;
REVOKE ALL PRIVILEGES ON TABLE ple_private.draft_question_metadata FROM PUBLIC;

CREATE INDEX draft_question_metadata_title_idx
    ON ple_private.draft_question_metadata (question_title);
SET LOCAL ROLE ple_data_owner;
CREATE INDEX published_question_metadata_search_idx
    ON ple_data.published_question_metadata
    USING gin (to_tsvector('simple', question_title || ' ' || question_description));

SET LOCAL ROLE ple_private_owner;
ALTER TABLE ple_private.draft_question DROP COLUMN title;
ALTER TABLE ple_private.draft_question DROP COLUMN question_content;
SET LOCAL ROLE ple_api_owner;
DROP VIEW ple_api.published_question_summary;
SET LOCAL ROLE ple_data_owner;
DROP INDEX IF EXISTS ple_data.question_revision_question_description_search_idx;
ALTER TABLE ple_data.question_revision DROP COLUMN public_metadata CASCADE;

-- Published reads use only stable Published Question Metadata, never a Draft
-- Question or mutable fields on an immutable Question Revision.
SET LOCAL ROLE ple_api_owner;
CREATE VIEW ple_api.published_question_summary
WITH (security_barrier = true) AS
SELECT questions.question_id,
       latest_acceptance.revision_number AS latest_question_revision_number,
       versions.backend,
       versions.published_at,
       metadata.question_title,
       metadata.question_description
  FROM ple_data.published_question AS questions
  JOIN ple_data.published_question_metadata AS metadata
    ON metadata.question_id = questions.question_id
  JOIN LATERAL (
      SELECT acceptance.revision_number
        FROM ple_data.question_revision_acceptance AS acceptance
       WHERE acceptance.question_id = questions.question_id
       ORDER BY acceptance.revision_number DESC
       LIMIT 1
  ) AS latest_acceptance ON true
  JOIN ple_data.question_revision AS versions
    ON versions.question_id = questions.question_id
   AND versions.revision_number = latest_acceptance.revision_number
 WHERE ple_api.current_session_account_is_instructor();
REVOKE ALL PRIVILEGES ON TABLE ple_api.published_question_summary FROM PUBLIC;
GRANT SELECT ON TABLE ple_api.published_question_summary TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.draft_question_metadata IS
    'Mutable private Draft Question Title and Question Description owned by one Draft Question.';
COMMENT ON TABLE ple_private.draft_question_source_binding IS
    'Current mutable Source Binding for one exact Draft Question; its Object Address is exact.';
COMMENT ON TABLE ple_private.question_revision_source_binding IS
    'Immutable Source Binding for one exact Question Revision; its Object Address is exact.';
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.published_question_metadata IS
    'Mutable Published Question Title and Question Description owned by one stable Question lineage.';
RESET ROLE;
