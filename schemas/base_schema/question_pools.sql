-- Stable published Question Pool identity and immutable member pins. This
-- foundation deliberately stores no selected-count setting: selection is
-- Assessment Entry policy, never Pool lineage state.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.question_pool (
    question_pool_id uuid PRIMARY KEY,
    -- Compact storage for the human-facing `AAAA-ZBBB` Pool ID. The server's
    -- Question-ID issuer owns randomness and the HMAC check character; the
    -- database remains the final collision authority and owns no secret.
    public_question_pool_id text NOT NULL UNIQUE CHECK (
        public_question_pool_id ~ '^[0-9A-HJKMNP-TV-Z]{8}$'
    ),
    metadata_etag uuid NOT NULL,
    current_revision_number bigint NOT NULL DEFAULT 1 CHECK (current_revision_number > 0),
    -- A fork is a new immutable lineage.  Its source names one exact immutable
    -- published Pool Revision; original published Pools have no source pair.
    source_question_pool_id uuid,
    source_question_pool_revision_number bigint,
    created_at timestamptz NOT NULL,
    CHECK ((source_question_pool_id IS NULL) = (source_question_pool_revision_number IS NULL))
);

-- A Question Pool is a stable lineage. This narrow schema makes an exact
-- immutable revision reference representable without inventing Pool contents,
-- current state, ownership, or a draft lifecycle.
CREATE TABLE ple_data.question_pool_revision (
    question_pool_id uuid NOT NULL
        REFERENCES ple_data.question_pool(question_pool_id),
    revision_number bigint NOT NULL CHECK (revision_number > 0),
    member_count integer NOT NULL CHECK (member_count > 0),
    interchangeability_attested_by_account_id uuid NOT NULL
        REFERENCES ple_private.account(account_id),
    interchangeability_attested_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL,
    PRIMARY KEY (question_pool_id, revision_number)
);

CREATE TABLE ple_data.question_pool_revision_member (
    question_pool_id uuid NOT NULL,
    revision_number bigint NOT NULL,
    member_position integer NOT NULL CHECK (member_position > 0),
    question_id text NOT NULL,
    question_revision_number integer NOT NULL CHECK (question_revision_number > 0),
    PRIMARY KEY (question_pool_id, revision_number, member_position),
    UNIQUE (question_pool_id, revision_number, question_id, question_revision_number),
    FOREIGN KEY (question_pool_id, revision_number)
        REFERENCES ple_data.question_pool_revision(question_pool_id, revision_number),
    FOREIGN KEY (question_id, question_revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number)
);

ALTER TABLE ple_data.question_pool
    ADD CONSTRAINT question_pool_source_revision_exists
    FOREIGN KEY (source_question_pool_id, source_question_pool_revision_number)
    REFERENCES ple_data.question_pool_revision(question_pool_id, revision_number);

CREATE FUNCTION ple_data.reject_question_pool_immutable_change()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Question Pool and Question Pool Revision records are immutable';
END
$$;

CREATE FUNCTION ple_data.validate_question_pool_lineage_update()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NEW.question_pool_id <> OLD.question_pool_id
       OR NEW.public_question_pool_id <> OLD.public_question_pool_id
       OR NEW.source_question_pool_id IS DISTINCT FROM OLD.source_question_pool_id
       OR NEW.source_question_pool_revision_number IS DISTINCT FROM OLD.source_question_pool_revision_number
       OR NEW.created_at <> OLD.created_at
       OR NEW.current_revision_number <> OLD.current_revision_number + 1
       OR NEW.metadata_etag = OLD.metadata_etag THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Question Pool identity is immutable and Revision append must advance metadata ETag once';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_data.validate_question_pool_revision_members()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE member_count integer; last_position integer;
BEGIN
    SELECT count(*), max(member_position) INTO member_count, last_position
      FROM ple_data.question_pool_revision_member
     WHERE question_pool_id = NEW.question_pool_id
       AND revision_number = NEW.revision_number;
    IF member_count = 0 OR last_position <> member_count THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Pool Revision requires a nonempty contiguous ordered member list';
    END IF;
    RETURN NULL;
END
$$;

CREATE FUNCTION ple_data.validate_question_pool_revision_member_insert()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE expected_count integer; existing_count integer;
BEGIN
    SELECT member_count INTO expected_count FROM ple_data.question_pool_revision
     WHERE question_pool_id = NEW.question_pool_id AND revision_number = NEW.revision_number;
    SELECT count(*) INTO existing_count FROM ple_data.question_pool_revision_member
     WHERE question_pool_id = NEW.question_pool_id AND revision_number = NEW.revision_number;
    IF expected_count IS NULL OR existing_count >= expected_count
       OR NEW.member_position <> existing_count + 1 THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Question Pool Revision member set is immutable and ordered';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER question_pool_identity_is_immutable
BEFORE DELETE ON ple_data.question_pool
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_pool_immutable_change();
CREATE TRIGGER question_pool_append_updates_lineage
BEFORE UPDATE ON ple_data.question_pool
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_pool_lineage_update();
CREATE TRIGGER question_pool_revision_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_pool_revision
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_pool_immutable_change();
CREATE TRIGGER question_pool_revision_member_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_pool_revision_member
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_pool_immutable_change();
CREATE TRIGGER question_pool_revision_member_insert_is_ordered
BEFORE INSERT ON ple_data.question_pool_revision_member
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_pool_revision_member_insert();
CREATE CONSTRAINT TRIGGER question_pool_revision_has_members
AFTER INSERT ON ple_data.question_pool_revision
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
EXECUTE FUNCTION ple_data.validate_question_pool_revision_members();

ALTER TABLE ple_data.question_pool ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_pool FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_pool_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_pool_revision FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_pool_revision_member ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_pool_revision_member FORCE ROW LEVEL SECURITY;

REVOKE ALL ON TABLE ple_data.question_pool, ple_data.question_pool_revision,
    ple_data.question_pool_revision_member FROM PUBLIC;

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
CREATE POLICY question_pool_revision_member_private_owner_lookup ON ple_data.question_pool_revision_member
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_pool_revision_member_api_owner_lookup ON ple_data.question_pool_revision_member
    FOR SELECT TO ple_api_owner USING (true);
GRANT SELECT ON ple_data.question_pool, ple_data.question_pool_revision_member
    TO ple_private_owner, ple_api_owner;

COMMENT ON TABLE ple_data.question_pool IS
    'Stable Question Pool lineage, immutable exact source-Pool provenance for forks, server-issued compact public Crockford ID, and current metadata ETag.';
COMMENT ON TABLE ple_data.question_pool_revision IS
    'Append-only sequential immutable Pool Revision with creating Instructor attestation and exact member count.';
COMMENT ON TABLE ple_data.question_pool_revision_member IS
    'Ordered distinct exact Published Question Revision pins; backend-neutral and intentionally no selected count.';

RESET ROLE;

-- The later typed trusted server command supplies an already HMAC-validated
-- canonical compact ID. This schema is not an issuer; the narrow session-bound
-- API wrapper below is the only application creation capability. Pool
-- ownership/content rules belong to the later published-Pool closure.
SET LOCAL ROLE ple_data_owner;
CREATE FUNCTION ple_data.create_question_pool(
    p_question_pool_id uuid, p_public_question_pool_id text,
    p_member_question_ids text[], p_member_revision_numbers integer[],
    p_interchangeability_attested boolean
) RETURNS TABLE (
    question_pool_id uuid, public_question_pool_id text, revision_number bigint, metadata_etag uuid
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE actor_id uuid; created_at timestamptz := pg_catalog.clock_timestamp(); next_etag uuid;
BEGIN
    IF p_question_pool_id IS NULL OR p_public_question_pool_id IS NULL
       OR p_public_question_pool_id !~ '^[0-9A-HJKMNP-TV-Z]{8}$'
       OR p_member_question_ids IS NULL OR p_member_revision_numbers IS NULL
       OR cardinality(p_member_question_ids) IS NULL OR cardinality(p_member_question_ids) = 0
       OR cardinality(p_member_question_ids) > 1024
       OR cardinality(p_member_question_ids) <> cardinality(p_member_revision_numbers)
       OR p_interchangeability_attested IS DISTINCT FROM true THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool creation requires a canonical ID, nonempty ordered members, and interchangeability attestation';
    END IF;
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Active Instructor authority is required for Question Pool creation';
    END IF;
    actor_id := ple_api.current_session_account_id();
    next_etag := pg_catalog.gen_random_uuid();
    INSERT INTO ple_data.question_pool(
        question_pool_id, public_question_pool_id, metadata_etag, current_revision_number, created_at
    ) VALUES (p_question_pool_id, p_public_question_pool_id, next_etag, 1, created_at);
    INSERT INTO ple_data.question_pool_revision(
        question_pool_id, revision_number, member_count, interchangeability_attested_by_account_id,
        interchangeability_attested_at, created_at
    ) VALUES (p_question_pool_id, 1, cardinality(p_member_question_ids), actor_id, created_at, created_at);
    INSERT INTO ple_data.question_pool_revision_member(
        question_pool_id, revision_number, member_position, question_id, question_revision_number
    )
    SELECT p_question_pool_id, 1, member.ordinality::integer, member.question_id,
           p_member_revision_numbers[member.ordinality]
      FROM unnest(p_member_question_ids) WITH ORDINALITY AS member(question_id, ordinality)
     ORDER BY member.ordinality;
    RETURN QUERY SELECT pool.question_pool_id, pool.public_question_pool_id, 1::bigint, pool.metadata_etag
      FROM ple_data.question_pool AS pool WHERE pool.question_pool_id = p_question_pool_id;
END
$$;

CREATE FUNCTION ple_data.append_question_pool_revision(
    p_question_pool_id uuid, p_expected_metadata_etag uuid,
    p_member_question_ids text[], p_member_revision_numbers integer[],
    p_interchangeability_attested boolean
) RETURNS TABLE (revision_number bigint, metadata_etag uuid) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE pool_row ple_data.question_pool%ROWTYPE; actor_id uuid;
    next_revision_number bigint; next_etag uuid; created_at timestamptz := pg_catalog.clock_timestamp();
BEGIN
    IF p_question_pool_id IS NULL OR p_expected_metadata_etag IS NULL
       OR p_member_question_ids IS NULL OR p_member_revision_numbers IS NULL
       OR cardinality(p_member_question_ids) IS NULL OR cardinality(p_member_question_ids) = 0
       OR cardinality(p_member_question_ids) <> cardinality(p_member_revision_numbers)
       OR p_interchangeability_attested IS DISTINCT FROM true THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Pool Revision append requires metadata ETag, nonempty ordered members, and interchangeability attestation';
    END IF;
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Active Instructor authority is required for Question Pool Revision append';
    END IF;
    -- The stable lineage lock serializes appenders; expected-current is CAS.
    SELECT * INTO pool_row FROM ple_data.question_pool
     WHERE question_pool_id = p_question_pool_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23503', MESSAGE = 'Question Pool does not exist';
    END IF;
    IF pool_row.metadata_etag <> p_expected_metadata_etag THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Pool metadata ETag is stale';
    END IF;
    actor_id := ple_api.current_session_account_id();
    next_revision_number := pool_row.current_revision_number + 1;
    next_etag := pg_catalog.gen_random_uuid();
    INSERT INTO ple_data.question_pool_revision(
        question_pool_id, revision_number, member_count, interchangeability_attested_by_account_id,
        interchangeability_attested_at, created_at
    ) VALUES (p_question_pool_id, next_revision_number, cardinality(p_member_question_ids), actor_id, created_at, created_at);
    INSERT INTO ple_data.question_pool_revision_member(
        question_pool_id, revision_number, member_position, question_id, question_revision_number
    )
    SELECT p_question_pool_id, next_revision_number, member.ordinality::integer, member.question_id,
           p_member_revision_numbers[member.ordinality]
      FROM unnest(p_member_question_ids) WITH ORDINALITY AS member(question_id, ordinality)
     ORDER BY member.ordinality;
    UPDATE ple_data.question_pool SET current_revision_number = next_revision_number, metadata_etag = next_etag
     WHERE question_pool_id = p_question_pool_id;
    revision_number := next_revision_number;
    metadata_etag := next_etag;
    RETURN NEXT;
END
$$;

-- Importing a reusable Pool into an Assessment never aliases the published
-- lineage.  The trusted server supplies a fresh HMAC-validated Pool identity;
-- this boundary copies the source's exact immutable Revision as revision 1.
CREATE FUNCTION ple_data.fork_question_pool_revision(
    p_question_pool_id uuid,
    p_public_question_pool_id text,
    p_source_question_pool_id uuid,
    p_source_question_pool_revision_number bigint
) RETURNS TABLE (
    question_pool_id uuid, public_question_pool_id text, revision_number bigint, metadata_etag uuid
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE actor_id uuid; created_at timestamptz := pg_catalog.clock_timestamp(); next_etag uuid;
DECLARE source_revision ple_data.question_pool_revision%ROWTYPE;
BEGIN
    IF p_question_pool_id IS NULL OR p_public_question_pool_id IS NULL
       OR p_public_question_pool_id !~ '^[0-9A-HJKMNP-TV-Z]{8}$'
       OR p_source_question_pool_id IS NULL OR p_source_question_pool_revision_number IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Pool fork is unavailable';
    END IF;
    SELECT revision.* INTO source_revision
      FROM ple_data.question_pool AS source_pool
      JOIN ple_data.question_pool_revision AS revision
        ON revision.question_pool_id = source_pool.question_pool_id
     WHERE source_pool.question_pool_id = p_source_question_pool_id
       AND source_pool.source_question_pool_id IS NULL
       AND revision.revision_number = p_source_question_pool_revision_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23503', MESSAGE = 'Question Pool source Revision does not exist';
    END IF;
    actor_id := ple_api.current_session_account_id();
    next_etag := pg_catalog.gen_random_uuid();
    INSERT INTO ple_data.question_pool(
        question_pool_id, public_question_pool_id, metadata_etag, current_revision_number,
        source_question_pool_id, source_question_pool_revision_number, created_at
    ) VALUES (
        p_question_pool_id, p_public_question_pool_id, next_etag, 1,
        p_source_question_pool_id, p_source_question_pool_revision_number, created_at
    );
    INSERT INTO ple_data.question_pool_revision(
        question_pool_id, revision_number, member_count, interchangeability_attested_by_account_id,
        interchangeability_attested_at, created_at
    ) VALUES (p_question_pool_id, 1, source_revision.member_count, actor_id, created_at, created_at);
    INSERT INTO ple_data.question_pool_revision_member(
        question_pool_id, revision_number, member_position, question_id, question_revision_number
    )
    SELECT p_question_pool_id, 1, member.member_position, member.question_id,
           member.question_revision_number
      FROM ple_data.question_pool_revision_member AS member
     WHERE member.question_pool_id = p_source_question_pool_id
       AND member.revision_number = p_source_question_pool_revision_number
     ORDER BY member.member_position;
    RETURN QUERY SELECT pool.question_pool_id, pool.public_question_pool_id, 1::bigint, pool.metadata_etag
      FROM ple_data.question_pool AS pool WHERE pool.question_pool_id = p_question_pool_id;
END
$$;
REVOKE ALL ON FUNCTION ple_data.reject_question_pool_immutable_change() FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.validate_question_pool_lineage_update(),
    ple_data.validate_question_pool_revision_members(),
    ple_data.validate_question_pool_revision_member_insert() FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.create_question_pool(uuid, text, text[], integer[], boolean) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.append_question_pool_revision(uuid, uuid, text[], integer[], boolean) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.fork_question_pool_revision(uuid, text, uuid, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.create_question_pool(uuid, text, text[], integer[], boolean) TO ple_api_owner;
RESET ROLE;

-- The application receives only this session-bound capability. The data-owner
-- procedure remains private, and it derives active Instructor authority and
-- the attesting Account from the installed session rather than browser input.
SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.create_question_pool(
    p_question_pool_id uuid, p_public_question_pool_id text,
    p_member_question_ids text[], p_member_revision_numbers integer[],
    p_interchangeability_attested boolean
) RETURNS TABLE (
    question_pool_id uuid, public_question_pool_id text, revision_number bigint, metadata_etag uuid
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.create_question_pool(
        p_question_pool_id, p_public_question_pool_id, p_member_question_ids,
        p_member_revision_numbers, p_interchangeability_attested)
$$;
REVOKE ALL ON FUNCTION ple_api.create_question_pool(uuid, text, text[], integer[], boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.create_question_pool(uuid, text, text[], integer[], boolean) TO ple_app;

RESET ROLE;

-- This is the sole SQL presentation projection for a stored public Question
-- or Pool identifier. It receives only compact values already constrained by
-- their owning tables; it does not parse browser input, normalize aliases, or
-- validate the server HMAC. Keeping the 4-4 rendering here lets one actual
-- schema projection hand the same canonical public reference to both kinds.
SET LOCAL ROLE ple_data_owner;
CREATE FUNCTION ple_data.canonical_public_crockford_display(p_compact_id text)
RETURNS text LANGUAGE plpgsql IMMUTABLE STRICT
SET search_path = pg_catalog AS $$
BEGIN
    IF p_compact_id !~ '^[0-9A-HJKMNP-TV-Z]{8}$' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'public Crockford ID must be canonical compact storage form';
    END IF;
    RETURN substr(p_compact_id, 1, 4) || '-' || substr(p_compact_id, 5, 4);
END
$$;
REVOKE ALL ON FUNCTION ple_data.canonical_public_crockford_display(text) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_data.canonical_public_crockford_display(text) TO ple_api_owner;
RESET ROLE;

-- A deliberately narrow, answer-free public-reference projection. It names
-- no internal UUID and does not make Pool content, membership, selection,
-- ownership, or lifecycle state visible. C355 may consume these stable IDs;
-- it must supply all selection semantics separately.
SET LOCAL ROLE ple_data_owner;
CREATE FUNCTION ple_data.list_published_content_identities()
RETURNS TABLE (
    content_kind text,
    public_id text,
    current_revision_number bigint
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Active Instructor authority is required for published content identities';
    END IF;
    RETURN QUERY
    SELECT 'question'::text,
           ple_data.canonical_public_crockford_display(question.question_id),
           max(revision.revision_number)::bigint
      FROM ple_data.published_question AS question
      JOIN ple_data.question_revision AS revision ON revision.question_id = question.question_id
     GROUP BY question.question_id
    UNION ALL
    SELECT 'pool'::text,
           ple_data.canonical_public_crockford_display(pool.public_question_pool_id),
           max(revision.revision_number)
      FROM ple_data.question_pool AS pool
      JOIN ple_data.question_pool_revision AS revision
        ON revision.question_pool_id = pool.question_pool_id
     GROUP BY pool.question_pool_id, pool.public_question_pool_id
     ORDER BY 1, 2;
END
$$;
REVOKE ALL ON FUNCTION ple_data.list_published_content_identities() FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_data.list_published_content_identities() TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.list_published_content_identities()
RETURNS TABLE (
    content_kind text,
    public_id text,
    current_revision_number bigint
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.list_published_content_identities()
$$;
REVOKE ALL ON FUNCTION ple_api.list_published_content_identities() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_published_content_identities() TO ple_app;

-- The server resolves an author-visible reusable Pool identity once, under
-- the installed Instructor session.  It never accepts a caller-selected
-- Revision and deliberately conceals unknown or already-owned child Pools.
CREATE FUNCTION ple_api.resolve_current_root_question_pool(p_public_question_pool_id text)
RETURNS TABLE (question_pool_id uuid, current_revision_number bigint)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT pool.question_pool_id, pool.current_revision_number
      FROM ple_data.question_pool AS pool
     WHERE ple_api.current_session_account_is_instructor()
       AND ple_data.canonical_public_crockford_display(pool.public_question_pool_id)
           = p_public_question_pool_id
       AND pool.source_question_pool_id IS NULL
       AND pool.source_question_pool_revision_number IS NULL
$$;
REVOKE ALL ON FUNCTION ple_api.resolve_current_root_question_pool(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.resolve_current_root_question_pool(text) TO ple_app;
RESET ROLE;
