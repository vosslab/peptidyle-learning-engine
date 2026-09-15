-- Immutable ancestry for independently owned Blueprint Course forks.
--
-- A fork is a new Private Blueprint lineage at Revision 1.  Its ancestry
-- records one exact source Revision, while its copied content deliberately
-- remains independent: later source saves cannot alter the child.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.blueprint_course_fork (
    blueprint_course_reference_number bigint PRIMARY KEY
        REFERENCES ple_data.blueprint_course (reference_number),
    source_blueprint_course_reference_number bigint NOT NULL,
    source_blueprint_revision_number bigint NOT NULL CHECK (
        source_blueprint_revision_number > 0
    ),
    forked_at timestamp with time zone NOT NULL,
    CHECK (blueprint_course_reference_number <> source_blueprint_course_reference_number),
    FOREIGN KEY (
        source_blueprint_course_reference_number, source_blueprint_revision_number
    ) REFERENCES ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number
    )
);

-- An idempotent fork request is distinct from ordinary Blueprint creation:
-- the receipt preserves the source fact and prevents a retry from creating a
-- second child lineage.
CREATE TABLE ple_data.blueprint_course_fork_receipt (
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    source_blueprint_course_reference_number bigint NOT NULL,
    source_blueprint_revision_number bigint NOT NULL CHECK (
        source_blueprint_revision_number > 0
    ),
    metadata_etag uuid NOT NULL,
    accepted_at timestamp with time zone NOT NULL,
    PRIMARY KEY (actor_account_id, request_checksum),
    FOREIGN KEY (
        source_blueprint_course_reference_number, source_blueprint_revision_number
    ) REFERENCES ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number
    )
);
CREATE INDEX blueprint_course_fork_source_idx ON ple_data.blueprint_course_fork (
    source_blueprint_course_reference_number, source_blueprint_revision_number
);

ALTER TABLE ple_data.blueprint_course_fork ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_fork FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_fork_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_fork_receipt FORCE ROW LEVEL SECURITY;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE
    ple_data.blueprint_course_fork, ple_data.blueprint_course_fork_receipt
TO ple_api_owner;
CREATE POLICY blueprint_course_fork_api_owner_all ON ple_data.blueprint_course_fork
    TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_course_fork_receipt_api_owner_all
    ON ple_data.blueprint_course_fork_receipt
    TO ple_api_owner USING (true) WITH CHECK (true);

-- The source Course and source Revision are permanent ancestry facts.  The
-- forked Blueprint's own content remains independently editable through its
-- ordinary immutable Revision sequence.
-- ASVS 8.2.2, 8.3.1: enforce this data-specific boundary in trusted PostgreSQL.
CREATE FUNCTION ple_data.reject_blueprint_course_fork_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Blueprint Course fork origin is immutable';
END
$$;
CREATE TRIGGER blueprint_course_fork_origin_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_course_fork
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_course_fork_change();
REVOKE ALL ON FUNCTION ple_data.reject_blueprint_course_fork_change() FROM PUBLIC;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.fork_blueprint_course(
    p_blueprint_id uuid,
    p_source_reference text,
    p_source_revision_number bigint,
    p_request_checksum bytea
) RETURNS TABLE (
    public_reference text,
    blueprint_revision_number bigint,
    metadata_etag uuid,
    accepted_at timestamp with time zone
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_actor uuid;
    v_source ple_data.blueprint_course%ROWTYPE;
    v_source_revision ple_data.blueprint_course_revision%ROWTYPE;
    v_child_reference bigint;
    v_child_revision bigint := 1;
    v_now timestamp with time zone;
    v_metadata_etag uuid;
    v_source_reference_number bigint;
BEGIN
    IF p_blueprint_id IS NULL
       OR p_source_reference !~ '^BP[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$'
       OR p_source_revision_number <= 0
       OR octet_length(p_request_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint Course fork is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    SELECT course.reference_number INTO v_source_reference_number
      FROM ple_data.blueprint_course AS course
     WHERE course.public_reference = p_source_reference;
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        pg_catalog.format('ple:blueprint-course-fork:%s:%s', v_actor,
            pg_catalog.encode(p_request_checksum, 'hex')), 0));
    SELECT course.public_reference, 1, receipt.metadata_etag,
           receipt.accepted_at
      INTO public_reference, blueprint_revision_number, metadata_etag, accepted_at
      FROM ple_data.blueprint_course_fork_receipt AS receipt
      JOIN ple_data.blueprint_course AS course
        ON course.reference_number = receipt.blueprint_course_reference_number
     WHERE receipt.actor_account_id = v_actor
       AND receipt.request_checksum = p_request_checksum;
    IF FOUND THEN
        RETURN NEXT;
        RETURN;
    END IF;

    -- Lifecycle is evaluated at the transaction boundary. A source that is
    -- changed to Private concurrently cannot be forked after this lock.
    SELECT * INTO v_source
      FROM ple_data.blueprint_course AS source_course
     WHERE source_course.reference_number = v_source_reference_number
       AND source_course.availability IN ('public', 'archived')
     FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Blueprint Course fork requires a Public or Archived source';
    END IF;
    SELECT * INTO v_source_revision
      FROM ple_data.blueprint_course_revision AS source_revision
     WHERE source_revision.blueprint_course_reference_number = v_source_reference_number
       AND source_revision.blueprint_revision_number = p_source_revision_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23503',
            MESSAGE = 'Blueprint Course fork source Revision is unavailable';
    END IF;

    v_now := pg_catalog.clock_timestamp();
    v_metadata_etag := pg_catalog.gen_random_uuid();
    INSERT INTO ple_data.blueprint_course AS child (
        blueprint_id, owner_account_id, short_name, long_name, availability,
        metadata_etag, current_blueprint_revision_number, created_at
    ) VALUES (
        p_blueprint_id, v_actor, v_source.short_name, v_source.long_name, 'private',
        v_metadata_etag, 1, v_now
    ) RETURNING child.reference_number INTO v_child_reference;
    INSERT INTO ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number,
        content, content_checksum, saved_at
    ) VALUES (
        v_child_reference, v_child_revision, v_source_revision.content,
        v_source_revision.content_checksum, v_now
    );
    INSERT INTO ple_data.blueprint_revision_question_pin
    SELECT v_child_reference, v_child_revision, pin.content_path,
           pin.question_id, pin.question_revision_number
      FROM ple_data.blueprint_revision_question_pin AS pin
     WHERE pin.blueprint_course_reference_number = v_source_reference_number
       AND pin.blueprint_revision_number = p_source_revision_number;
    INSERT INTO ple_data.blueprint_revision_module
    SELECT v_child_reference, v_child_revision, member.blueprint_module_reference,
           member.module_position
      FROM ple_data.blueprint_revision_module AS member
     WHERE member.blueprint_course_reference_number = v_source_reference_number
       AND member.blueprint_revision_number = p_source_revision_number;
    INSERT INTO ple_data.blueprint_revision_assessment
    SELECT v_child_reference, v_child_revision, member.blueprint_module_reference,
           member.blueprint_assessment_reference, member.assessment_position
      FROM ple_data.blueprint_revision_assessment AS member
     WHERE member.blueprint_course_reference_number = v_source_reference_number
       AND member.blueprint_revision_number = p_source_revision_number;
    INSERT INTO ple_data.blueprint_revision_event (
        blueprint_course_reference_number, blueprint_revision_number, actor_account_id,
        request_checksum, occurred_at
    ) VALUES (
        v_child_reference, v_child_revision, v_actor, p_request_checksum, v_now
    );
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_reference_number, actor_account_id, short_name, long_name,
        availability, metadata_etag, occurred_at
    ) VALUES (
        v_child_reference, v_actor, v_source.short_name, v_source.long_name,
        'private', v_metadata_etag, v_now
    );
    INSERT INTO ple_data.blueprint_course_fork VALUES (
        v_child_reference, v_source_reference_number, p_source_revision_number, v_now
    );
    INSERT INTO ple_data.blueprint_course_fork_receipt VALUES (
        v_actor, p_request_checksum, v_child_reference, v_source_reference_number,
        p_source_revision_number, v_metadata_etag, v_now
    );
    SELECT course.public_reference INTO public_reference
      FROM ple_data.blueprint_course AS course
     WHERE course.reference_number = v_child_reference;
    blueprint_revision_number := v_child_revision;
    metadata_etag := v_metadata_etag;
    accepted_at := v_now;
    RETURN NEXT;
END
$$;

REVOKE ALL PRIVILEGES ON TABLE
    ple_data.blueprint_course_fork, ple_data.blueprint_course_fork_receipt FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.fork_blueprint_course(uuid, text, bigint, bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.fork_blueprint_course(uuid, text, bigint, bytea)
TO ple_app;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.blueprint_course_fork IS
    'Exact immutable source Blueprint Revision for one independent child Blueprint lineage.';
RESET ROLE;
