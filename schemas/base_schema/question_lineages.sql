-- Shared Question Library lineages.  Availability is current lineage state;
-- exact immutable revisions remain available to authorized historical readers.

-- The lineage rows record their publishing actor.  These grants must precede
-- the cross-schema foreign keys below: question_stewardship is intentionally
-- later because it depends on the lineage tables.
SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.published_question (
    question_id text PRIMARY KEY,
    availability text NOT NULL DEFAULT 'available'
        CHECK (availability IN ('available', 'archived')),
    availability_edit_number bigint NOT NULL DEFAULT 1
        CHECK (availability_edit_number > 0),
    created_at timestamptz NOT NULL,
    CONSTRAINT published_question_id_is_crockford_shape CHECK (
        question_id ~ '^[0-9A-HJKMNP-TV-Z]{7}$'
    )
);

CREATE TABLE ple_data.question_revision (
    question_id text NOT NULL REFERENCES ple_data.published_question(question_id),
    revision_number integer NOT NULL CHECK (revision_number > 0),
    backend text NOT NULL CHECK (backend IN ('ple', 'webwork', 'imathas')),
    published_at timestamptz NOT NULL,
    PRIMARY KEY (question_id, revision_number)
);

CREATE TABLE ple_data.published_question_metadata (
    question_id text PRIMARY KEY REFERENCES ple_data.published_question(question_id),
    question_title text NOT NULL CHECK (
        question_title = btrim(question_title)
        AND char_length(question_title) BETWEEN 1 AND 512
        AND question_title !~ '[[:cntrl:]]'
    ),
    question_description text NOT NULL CHECK (
        question_description = btrim(question_description)
        AND char_length(question_description) BETWEEN 1 AND 4000
        AND question_description !~ '[[:cntrl:]]'
    ),
    language text NOT NULL CHECK (
        language = btrim(language) AND char_length(language) BETWEEN 2 AND 35
    ),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at)
);

CREATE TABLE ple_data.question_publication_event (
    event_id uuid PRIMARY KEY,
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    occurred_at timestamptz NOT NULL,
    UNIQUE (question_id, revision_number),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number)
);

CREATE TABLE ple_data.question_availability_event (
    event_id uuid PRIMARY KEY,
    question_id text NOT NULL REFERENCES ple_data.published_question(question_id),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    availability text NOT NULL CHECK (availability IN ('available', 'archived')),
    edit_number bigint NOT NULL CHECK (edit_number > 0),
    reason text,
    occurred_at timestamptz NOT NULL,
    UNIQUE (question_id, edit_number),
    CHECK (
        (availability = 'available' AND reason IS NULL)
        OR (availability = 'archived' AND reason = btrim(reason)
            AND char_length(reason) BETWEEN 1 AND 1000)
    )
);

CREATE FUNCTION ple_data.reject_question_lineage_immutable_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Question Revision and Question event records are immutable';
END
$$;

CREATE FUNCTION ple_data.validate_question_availability_event()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE
    current_availability text;
    current_edit_number bigint;
    has_prior_event boolean;
BEGIN
    SELECT availability, availability_edit_number
      INTO current_availability, current_edit_number
      FROM ple_data.published_question
     WHERE question_id = NEW.question_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Published Question Availability Edit Number is stale';
    END IF;
    SELECT EXISTS (
        SELECT 1 FROM ple_data.question_availability_event
         WHERE question_id = NEW.question_id
    ) INTO has_prior_event;
    IF NOT has_prior_event THEN
        IF NEW.availability <> 'available' OR NEW.edit_number <> 1 THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Question lineage publication records Available at Edit Number 1';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.edit_number <> current_edit_number + 1 THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Published Question Availability Edit Number is stale';
    END IF;
    IF NEW.availability = current_availability THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Published Question Availability transition must change state';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER question_revision_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_lineage_immutable_change();
CREATE TRIGGER question_publication_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_publication_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_lineage_immutable_change();
CREATE TRIGGER question_availability_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_availability_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_lineage_immutable_change();
CREATE TRIGGER question_availability_event_is_current
BEFORE INSERT ON ple_data.question_availability_event
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_availability_event();

CREATE INDEX published_question_available_discovery_idx
    ON ple_data.published_question(question_id) WHERE availability = 'available';
CREATE INDEX published_question_metadata_search_idx ON ple_data.published_question_metadata
    USING gin (to_tsvector('simple', question_title || ' ' || question_description));

ALTER TABLE ple_data.published_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.published_question FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.published_question_metadata ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.published_question_metadata FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_publication_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_publication_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_availability_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_availability_event FORCE ROW LEVEL SECURITY;

CREATE POLICY published_question_data_owner_access ON ple_data.published_question
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_revision_data_owner_access ON ple_data.question_revision
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY published_question_metadata_data_owner_access ON ple_data.published_question_metadata
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_publication_event_data_owner_access ON ple_data.question_publication_event
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_availability_event_data_owner_access ON ple_data.question_availability_event
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY published_question_private_publication_insert ON ple_data.published_question
    FOR INSERT TO ple_private_owner WITH CHECK (true);
-- Successor publication serializes its immutable revision number by locking
-- the stable lineage.  This is deliberately read-only: only the dedicated
-- availability transition owned by ple_data_owner changes lineage state.
CREATE POLICY published_question_private_publication_read ON ple_data.published_question
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY published_question_private_publication_lock ON ple_data.published_question
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (false);
CREATE POLICY published_question_metadata_private_publication_insert
    ON ple_data.published_question_metadata FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY published_question_metadata_private_publication_read
    ON ple_data.published_question_metadata FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY published_question_metadata_private_publication_update
    ON ple_data.published_question_metadata FOR UPDATE TO ple_private_owner
    USING (true) WITH CHECK (true);
CREATE POLICY question_revision_private_publication_insert ON ple_data.question_revision
    FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_revision_private_publication_read ON ple_data.question_revision
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_publication_event_private_publication_insert
    ON ple_data.question_publication_event FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_availability_event_private_publication_insert
    ON ple_data.question_availability_event FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY published_question_api_summary_read ON ple_data.published_question
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY published_question_metadata_api_summary_read ON ple_data.published_question_metadata
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY question_revision_api_summary_read ON ple_data.question_revision
    FOR SELECT TO ple_api_owner USING (true);

REVOKE ALL ON TABLE ple_data.published_question, ple_data.question_revision,
    ple_data.published_question_metadata, ple_data.question_publication_event,
    ple_data.question_availability_event FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT INSERT, SELECT ON ple_data.published_question, ple_data.published_question_metadata,
    ple_data.question_revision, ple_data.question_publication_event,
    ple_data.question_availability_event TO ple_private_owner;
GRANT UPDATE ON ple_data.published_question TO ple_private_owner;
GRANT UPDATE ON ple_data.published_question_metadata TO ple_private_owner;
GRANT SELECT ON ple_data.published_question, ple_data.published_question_metadata,
    ple_data.question_revision TO ple_api_owner;
REVOKE ALL ON FUNCTION ple_data.reject_question_lineage_immutable_change(),
    ple_data.validate_question_availability_event() FROM PUBLIC;

COMMENT ON TABLE ple_data.published_question IS
    'Stable Question lineage with current availability and its qualified edit number.';
COMMENT ON TABLE ple_data.question_revision IS
    'Immutable exact published Question content identity; archive never removes this provenance.';
COMMENT ON TABLE ple_data.question_availability_event IS
    'Append-only actor-attributed current-lineage availability transitions.';
RESET ROLE;

-- ASVS 1.2.4, 2.2-2.3, 8.2-8.3, and 15.4: this capability is the only
-- transition path.  It locks the lineage, rechecks ownership, and records a
-- redacted, actor-attributed event in the same transaction.
SET LOCAL ROLE ple_api_owner;
GRANT USAGE ON SCHEMA ple_api TO ple_data_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
CREATE FUNCTION ple_data.set_question_availability(
    p_question_id text,
    p_expected_edit_number bigint,
    p_target_availability text,
    p_archive_confirmation_title text,
    p_event_id uuid
) RETURNS TABLE (availability text, availability_edit_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    current_question ple_data.published_question%ROWTYPE;
    current_title text;
    actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF actor_id IS NULL OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Instructor authority is required';
    END IF;
    IF p_target_availability NOT IN ('available', 'archived')
       OR p_expected_edit_number IS NULL OR p_expected_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'availability transition input is invalid';
    END IF;
    SELECT * INTO current_question FROM ple_data.published_question
     WHERE question_id = p_question_id FOR UPDATE;
    IF NOT FOUND OR NOT EXISTS (SELECT 1 FROM ple_data.question_current_owner
        WHERE question_id = p_question_id AND owner_account_id = actor_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Owner authority is required';
    END IF;
    IF current_question.availability_edit_number <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Published Question Availability Edit Number is stale';
    END IF;
    SELECT question_title INTO current_title FROM ple_data.published_question_metadata
     WHERE question_id = p_question_id;
    IF p_target_availability = 'archived'
       AND p_archive_confirmation_title IS DISTINCT FROM current_title THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Archive Published Question requires exact title confirmation';
    END IF;
    IF (p_target_availability = 'archived' AND current_question.availability <> 'available')
       OR (p_target_availability = 'available' AND current_question.availability <> 'archived') THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Question availability transition conflicts with its current state';
    END IF;
    INSERT INTO ple_data.question_availability_event(
        event_id, question_id, actor_account_id, availability, edit_number, reason, occurred_at
    ) VALUES (
        p_event_id, p_question_id, actor_id, p_target_availability,
        p_expected_edit_number + 1,
        CASE WHEN p_target_availability = 'archived' THEN 'archived by Question Owner' END,
        pg_catalog.clock_timestamp()
    );
    UPDATE ple_data.published_question
       SET availability = p_target_availability,
           availability_edit_number = p_expected_edit_number + 1
     WHERE question_id = p_question_id
     RETURNING published_question.availability, published_question.availability_edit_number
       INTO availability, availability_edit_number;
    RETURN NEXT;
END
$$;
REVOKE ALL ON FUNCTION ple_data.set_question_availability(text, bigint, text, text, uuid) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_data.set_question_availability(text, bigint, text, text, uuid) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.set_question_availability(
    p_question_id text, p_expected_edit_number bigint, p_target_availability text,
    p_archive_confirmation_title text, p_event_id uuid
) RETURNS TABLE (availability text, availability_edit_number bigint)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_api AS $$
    SELECT * FROM ple_data.set_question_availability(
        p_question_id, p_expected_edit_number, p_target_availability,
        p_archive_confirmation_title, p_event_id)
$$;
REVOKE ALL ON FUNCTION ple_api.set_question_availability(text, bigint, text, text, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.set_question_availability(text, bigint, text, text, uuid) TO ple_app;
RESET ROLE;
