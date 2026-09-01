-- Exact immutable Question Fork Source relationships for private Draft
-- Questions and their eventual separate Published Question lineages.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.question_fork_source (
    forked_question_id text PRIMARY KEY REFERENCES ple_data.published_question (question_id),
    source_question_id text NOT NULL,
    source_revision_number integer NOT NULL CHECK (source_revision_number > 0),
    recorded_at timestamp with time zone NOT NULL,
    CONSTRAINT question_fork_source_source_revision_matches FOREIGN KEY (
        source_question_id, source_revision_number
    ) REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT question_fork_source_is_separate_lineage CHECK (
        forked_question_id <> source_question_id
    )
);

CREATE FUNCTION ple_data.reject_question_fork_source_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Question Fork Source records are immutable';
END
$$;

CREATE TRIGGER question_fork_source_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_fork_source
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_fork_source_change();

ALTER TABLE ple_data.question_fork_source ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_fork_source FORCE ROW LEVEL SECURITY;
CREATE POLICY question_fork_source_data_owner_access
    ON ple_data.question_fork_source FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);
CREATE POLICY question_fork_source_private_publication_insert
    ON ple_data.question_fork_source FOR INSERT TO ple_private_owner
    WITH CHECK (true);
CREATE POLICY question_fork_source_private_publication_lookup
    ON ple_data.question_fork_source FOR SELECT TO ple_private_owner
    USING (true);
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT INSERT, SELECT ON TABLE ple_data.question_fork_source TO ple_private_owner;
REVOKE ALL PRIVILEGES ON TABLE ple_data.question_fork_source FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_question_fork_source_change() FROM PUBLIC;

RESET ROLE;
SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.draft_question_fork_source (
    draft_question_uuid uuid PRIMARY KEY
        REFERENCES ple_private.draft_question (draft_question_uuid),
    source_question_id text NOT NULL,
    source_revision_number integer NOT NULL CHECK (source_revision_number > 0),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT draft_question_fork_source_source_revision_matches FOREIGN KEY (
        source_question_id, source_revision_number
    ) REFERENCES ple_data.question_revision (question_id, revision_number)
);

CREATE FUNCTION ple_private.reject_draft_question_fork_source_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Draft Question Fork Source records are immutable';
END
$$;

CREATE TRIGGER draft_question_fork_source_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.draft_question_fork_source
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_draft_question_fork_source_change();

ALTER TABLE ple_private.draft_question_fork_source ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.draft_question_fork_source FORCE ROW LEVEL SECURITY;
CREATE POLICY draft_question_fork_source_private_owner_access
    ON ple_private.draft_question_fork_source FOR ALL TO ple_private_owner
    USING (true) WITH CHECK (true);
REVOKE ALL PRIVILEGES ON TABLE ple_private.draft_question_fork_source FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_draft_question_fork_source_change() FROM PUBLIC;

-- This is the only session-authorized registration path for a Draft Question
-- fork. A future Fork Question Store creates the Draft Question and calls this
-- function in the same transaction.
CREATE FUNCTION ple_private.register_draft_question_fork_source(
    p_draft_question_uuid uuid,
    p_workspace_id uuid,
    p_source_question_id text,
    p_source_revision_number integer
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF NOT ple_api.current_session_account_can_access_workspace(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Draft Question Fork Source registration requires current workspace access';
    END IF;
    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.draft_question AS draft_question
         WHERE draft_question.draft_question_uuid = p_draft_question_uuid
           AND draft_question.workspace_id = p_workspace_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question Fork Source must name its exact Draft Question and workspace';
    END IF;

    INSERT INTO ple_private.draft_question_fork_source (
        draft_question_uuid, source_question_id, source_revision_number, created_at
    ) VALUES (
        p_draft_question_uuid, p_source_question_id, p_source_revision_number,
        pg_catalog.clock_timestamp()
    ) ON CONFLICT DO NOTHING;

    IF EXISTS (
        SELECT 1
          FROM ple_private.draft_question_fork_source AS source
         WHERE source.draft_question_uuid = p_draft_question_uuid
           AND source.source_question_id = p_source_question_id
           AND source.source_revision_number = p_source_revision_number
    ) THEN
        RETURN;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '23505',
        MESSAGE = 'Draft Question already has a different immutable Question Fork Source';
END
$$;

-- The trusted publication coordinator binds the existing Draft Question Fork
-- Source to the new, separate Published Question lineage. It is deliberately
-- unavailable to the application role until a complete publication Store owns
-- the surrounding validation, credit, license, source-transfer, and event work.
CREATE FUNCTION ple_private.publish_question_fork_source(
    p_draft_question_uuid uuid,
    p_forked_question_id text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    draft_source ple_private.draft_question_fork_source%ROWTYPE;
BEGIN
    SELECT source.*
      INTO draft_source
      FROM ple_private.draft_question_fork_source AS source
     WHERE source.draft_question_uuid = p_draft_question_uuid;
    IF NOT FOUND THEN
        RETURN;
    END IF;

    INSERT INTO ple_data.question_fork_source (
        forked_question_id, source_question_id, source_revision_number, recorded_at
    ) VALUES (
        p_forked_question_id, draft_source.source_question_id,
        draft_source.source_revision_number, pg_catalog.clock_timestamp()
    ) ON CONFLICT DO NOTHING;

    IF EXISTS (
        SELECT 1
          FROM ple_data.question_fork_source AS source
         WHERE source.forked_question_id = p_forked_question_id
           AND source.source_question_id = draft_source.source_question_id
           AND source.source_revision_number = draft_source.source_revision_number
    ) THEN
        RETURN;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '23505',
        MESSAGE = 'Published Question already has a different immutable Question Fork Source';
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.register_draft_question_fork_source(
    uuid, uuid, text, integer
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.register_draft_question_fork_source(
    uuid, uuid, text, integer
) TO ple_api_owner;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.publish_question_fork_source(uuid, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.publish_question_fork_source(uuid, text) TO ple_api_owner;

RESET ROLE;
SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.register_draft_question_fork_source(
    p_draft_question_uuid uuid,
    p_workspace_id uuid,
    p_source_question_id text,
    p_source_revision_number integer
)
RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.register_draft_question_fork_source(
        p_draft_question_uuid, p_workspace_id, p_source_question_id, p_source_revision_number
    )
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.register_draft_question_fork_source(
    uuid, uuid, text, integer
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.register_draft_question_fork_source(
    uuid, uuid, text, integer
) TO ple_app;

COMMENT ON FUNCTION ple_api.register_draft_question_fork_source(
    uuid, uuid, text, integer
) IS 'Registers one exact Question Fork Source for a Draft Question inside the current authorized Authoring Workspace.';

RESET ROLE;
SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.question_fork_source IS
    'Immutable Question Fork Source from one separately published Question lineage to the exact source Question Revision.';

RESET ROLE;
SET LOCAL ROLE ple_private_owner;

COMMENT ON TABLE ple_private.draft_question_fork_source IS
    'Immutable Question Fork Source from one private Draft Question to the exact source Question Revision.';
COMMENT ON FUNCTION ple_private.publish_question_fork_source(uuid, text) IS
    'Trusted publication-coordinator helper that binds an existing Draft Question Fork Source to its separate Published Question lineage and is a no-op for an ordinary Draft Question.';

RESET ROLE;
