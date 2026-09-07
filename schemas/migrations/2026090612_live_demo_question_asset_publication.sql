-- Immutable public Question Asset publication registry.
--
-- The registry is the committed outbox boundary between a published Question
-- Revision's private, verified still-image bytes and its final Public Assets
-- object.  A browser never selects either Object Address or storage input.
-- The restricted Publisher capability is created by the superuser-only
-- pre-migration bootstrap, not here: PostgreSQL 17 gives a non-superuser role
-- creator an unremovable ADMIN membership in every role it creates.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.question_asset_publication (
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    asset_id uuid NOT NULL,
    source_object_id uuid NOT NULL REFERENCES ple_private.object_record (object_id),
    source_object_checksum bytea NOT NULL CHECK (pg_catalog.octet_length(source_object_checksum) = 32),
    public_object_id uuid NOT NULL UNIQUE,
    public_object_checksum bytea NOT NULL CHECK (pg_catalog.octet_length(public_object_checksum) = 32),
    public_byte_length bigint NOT NULL CHECK (public_byte_length >= 0),
    verified_media_type text NOT NULL CHECK (verified_media_type IN (
        'image/png', 'image/jpeg', 'image/webp'
    )),
    intrinsic_width integer NOT NULL CHECK (intrinsic_width > 0),
    intrinsic_height integer NOT NULL CHECK (intrinsic_height > 0),
    delivery_id uuid NOT NULL UNIQUE,
    job_id uuid NOT NULL UNIQUE REFERENCES ple_private.job (job_id),
    publication_state text NOT NULL CHECK (publication_state IN ('pending', 'ready')),
    PRIMARY KEY (question_id, revision_number, asset_id),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    FOREIGN KEY (delivery_id, public_object_id)
        REFERENCES ple_data.object_delivery (delivery_id, object_id)
);

-- The publication's ownership, checksums, final typed Object Address, delivery,
-- and job target are all fixed at creation.  The publisher may make exactly one
-- mechanical transition after it has written and verified the immutable public
-- bytes.
CREATE FUNCTION ple_private.reject_question_asset_publication_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.question_id IS DISTINCT FROM OLD.question_id
       OR NEW.revision_number IS DISTINCT FROM OLD.revision_number
       OR NEW.asset_id IS DISTINCT FROM OLD.asset_id
       OR NEW.source_object_id IS DISTINCT FROM OLD.source_object_id
       OR NEW.source_object_checksum IS DISTINCT FROM OLD.source_object_checksum
       OR NEW.public_object_id IS DISTINCT FROM OLD.public_object_id
       OR NEW.public_object_checksum IS DISTINCT FROM OLD.public_object_checksum
       OR NEW.public_byte_length IS DISTINCT FROM OLD.public_byte_length
       OR NEW.verified_media_type IS DISTINCT FROM OLD.verified_media_type
       OR NEW.intrinsic_width IS DISTINCT FROM OLD.intrinsic_width
       OR NEW.intrinsic_height IS DISTINCT FROM OLD.intrinsic_height
       OR NEW.delivery_id IS DISTINCT FROM OLD.delivery_id
       OR NEW.job_id IS DISTINCT FROM OLD.job_id
       OR OLD.publication_state <> 'pending'
       OR NEW.publication_state <> 'ready'
       OR pg_catalog.current_setting(
            'ple.question_asset_publication_activation', true
          ) IS DISTINCT FROM OLD.job_id::text THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Asset Publication is immutable apart from Pending to Ready';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER question_asset_publication_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_asset_publication
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_asset_publication_change();

-- Validate only trusted, revision-owned inputs and the closed public job
-- target.  The final public Object Record is intentionally absent while a
-- publication is Pending: writing it before the transaction commits would
-- create a public orphan.  It is required at Ready activation below.
CREATE FUNCTION ple_private.validate_question_asset_publication()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    expected_source_address jsonb := jsonb_build_object(
        'kind', 'restrictedQuestionAsset',
        'questionRevision', jsonb_build_object(
            'questionId', NEW.question_id,
            'revisionNumber', NEW.revision_number
        ),
        'asset', NEW.asset_id,
        'object', NEW.source_object_id
    );
    expected_public_address jsonb := jsonb_build_object(
        'kind', 'questionAsset',
        'questionRevision', jsonb_build_object(
            'questionId', NEW.question_id,
            'revisionNumber', NEW.revision_number
        ),
        'asset', NEW.asset_id,
        'object', NEW.public_object_id
    );
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.object_record AS source_record
         WHERE source_record.object_id = NEW.source_object_id
           AND source_record.object_address = expected_source_address
           AND source_record.object_storage_area = 'private-content'
           AND source_record.object_data_class = 'question-asset'
           AND source_record.sha256 = NEW.source_object_checksum
           AND source_record.media_type = NEW.verified_media_type
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Asset Publication requires its exact private revision-owned source Object Record';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM ple_private.job
         WHERE job_id = NEW.job_id
           AND job_kind = 'publish_public_assets'
           AND job_target_kind = 'public_asset_publication'
           AND question_id = NEW.question_id
           AND revision_number = NEW.revision_number
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Asset Publication requires its exact Publish Public Assets Job';
    END IF;

    IF NEW.publication_state = 'ready' AND NOT EXISTS (
        SELECT 1 FROM ple_private.object_record AS public_record
         WHERE public_record.object_id = NEW.public_object_id
           AND public_record.object_address = expected_public_address
           AND public_record.object_storage_area = 'public-assets'
           AND public_record.object_data_class = 'question-asset'
           AND public_record.sha256 = NEW.public_object_checksum
           AND public_record.size_bytes = NEW.public_byte_length
           AND public_record.media_type = NEW.verified_media_type
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Ready Question Asset Publication requires its exact public Object Record';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER question_asset_publication_has_trusted_objects_and_job
BEFORE INSERT OR UPDATE ON ple_private.question_asset_publication
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_asset_publication();

-- The existing delivery tables remain the one delivery authority.  This
-- deferred check permits the one transaction which installs a Pending outbox
-- record, its Pending delivery relationship, and its closed job, while making
-- every committed registry record complete.  Ready requires Available.
CREATE FUNCTION ple_private.require_complete_question_asset_publication_delivery()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    target_delivery_id uuid := COALESCE(NEW.delivery_id, OLD.delivery_id);
    publication ple_private.question_asset_publication%ROWTYPE;
BEGIN
    SELECT * INTO publication
      FROM ple_private.question_asset_publication
     WHERE delivery_id = target_delivery_id;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM ple_data.object_delivery AS delivery
         WHERE delivery.delivery_id = publication.delivery_id
           AND delivery.object_id = publication.public_object_id
           AND delivery.sha256 = publication.public_object_checksum
           AND delivery.media_type = publication.verified_media_type
           AND delivery.byte_length = publication.public_byte_length
           AND delivery.delivery_state = CASE publication.publication_state
               WHEN 'pending' THEN 'pending'
               WHEN 'ready' THEN 'available'
           END
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_data.question_asset_delivery AS asset_delivery
         WHERE asset_delivery.delivery_id = publication.delivery_id
           AND asset_delivery.object_id = publication.public_object_id
           AND asset_delivery.question_id = publication.question_id
           AND asset_delivery.revision_number = publication.revision_number
           AND asset_delivery.asset_id = publication.asset_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Asset Publication requires its exact state-matched Object Delivery and Question Asset Delivery';
    END IF;
    RETURN NULL;
END
$$;

CREATE CONSTRAINT TRIGGER question_asset_publication_requires_complete_delivery
AFTER INSERT OR UPDATE OR DELETE ON ple_private.question_asset_publication
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_private.require_complete_question_asset_publication_delivery();

-- ple_data_owner owns the two deferred delivery triggers and therefore needs
-- execute permission on their private invariant function; no runtime role
-- receives it directly.
GRANT EXECUTE ON FUNCTION ple_private.require_complete_question_asset_publication_delivery()
    TO ple_data_owner;

SET LOCAL ROLE ple_data_owner;
-- The private publisher procedure below is the sole runtime state transition.
-- Its definer needs only the two delivery relations; neither ple_app nor a
-- browser role receives direct table access.
-- Deferred ownership validation can fire after a publisher definer returns.
-- Keep that existing cross-delivery invariant under its data-schema owner,
-- with only read policies for the four relations it counts.
ALTER FUNCTION ple_data.require_exact_available_object_delivery_owner()
    SECURITY DEFINER
    SET search_path = pg_catalog, ple_data;
CREATE POLICY object_delivery_owner_trigger_data_owner_read
    ON ple_data.object_delivery
    FOR SELECT TO ple_data_owner USING (true);
CREATE POLICY question_asset_delivery_owner_trigger_data_owner_read
    ON ple_data.question_asset_delivery
    FOR SELECT TO ple_data_owner USING (true);
CREATE POLICY course_banner_delivery_owner_trigger_data_owner_read
    ON ple_data.course_banner_delivery
    FOR SELECT TO ple_data_owner USING (true);
CREATE POLICY course_object_delivery_owner_trigger_data_owner_read
    ON ple_data.course_object_delivery
    FOR SELECT TO ple_data_owner USING (true);
GRANT SELECT, UPDATE ON TABLE ple_data.object_delivery TO ple_private_owner;
GRANT SELECT ON TABLE ple_data.question_asset_delivery TO ple_private_owner;
CREATE POLICY question_asset_publication_private_owner_delivery_read
    ON ple_data.object_delivery
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_asset_publication_private_owner_delivery_update
    ON ple_data.object_delivery
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_asset_publication_private_owner_asset_delivery_read
    ON ple_data.question_asset_delivery
    FOR SELECT TO ple_private_owner USING (true);
CREATE CONSTRAINT TRIGGER object_delivery_preserves_question_asset_publication
AFTER INSERT OR UPDATE OR DELETE ON ple_data.object_delivery
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_private.require_complete_question_asset_publication_delivery();
CREATE CONSTRAINT TRIGGER question_asset_delivery_preserves_question_asset_publication
AFTER INSERT OR UPDATE OR DELETE ON ple_data.question_asset_delivery
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_private.require_complete_question_asset_publication_delivery();

SET LOCAL ROLE ple_private_owner;

-- `ple_private.job` is forced-RLS.  The publication definer may see and
-- update only the closed public-asset Job shape it claims and completes;
-- neither API nor publisher roles receive direct table visibility.
CREATE POLICY question_asset_publication_job_private_owner_read
    ON ple_private.job
    FOR SELECT TO ple_private_owner
    USING (
        job_kind = 'publish_public_assets'
        AND job_target_kind = 'public_asset_publication'
    );
CREATE POLICY question_asset_publication_job_private_owner_update
    ON ple_private.job
    FOR UPDATE TO ple_private_owner
    USING (
        job_kind = 'publish_public_assets'
        AND job_target_kind = 'public_asset_publication'
    )
    WITH CHECK (
        job_kind = 'publish_public_assets'
        AND job_target_kind = 'public_asset_publication'
    );

-- A publisher supplies only a fresh bounded lease.  It cannot choose a Job,
-- source, Object Address, or target: the query locks the next eligible
-- registry-backed public-assets Job and returns only its trusted facts.
CREATE FUNCTION ple_private.claim_question_asset_publication_job(
    p_lease_token uuid,
    p_lease_expires_at timestamp with time zone
) RETURNS TABLE (
    job_id uuid,
    question_id text,
    revision_number integer,
    asset_id uuid,
    source_object_id uuid,
    source_object_checksum bytea,
    public_object_id uuid,
    public_object_checksum bytea,
    public_byte_length bigint,
    verified_media_type text,
    intrinsic_width integer,
    intrinsic_height integer,
    delivery_id uuid
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE
    publication ple_private.question_asset_publication%ROWTYPE;
    publication_job ple_private.job%ROWTYPE;
    claimed_at timestamp with time zone := pg_catalog.clock_timestamp();
BEGIN
    IF p_lease_token IS NULL
       OR p_lease_expires_at <= claimed_at
       OR p_lease_expires_at > claimed_at + interval '300 seconds' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Asset Publication Job lease is invalid or exceeds 300 seconds';
    END IF;

    SELECT registry.*
      INTO publication
      FROM ple_private.job AS job_row
      JOIN ple_private.question_asset_publication AS registry
        ON registry.job_id = job_row.job_id
     WHERE registry.publication_state = 'pending'
       AND job_row.job_kind = 'publish_public_assets'
       AND job_row.job_target_kind = 'public_asset_publication'
       AND job_row.question_id = registry.question_id
       AND job_row.revision_number = registry.revision_number
       AND job_row.attempt_count < job_row.max_attempts
       AND (
            (job_row.state = 'ready'
             AND job_row.available_at <= claimed_at)
            OR (job_row.state = 'leased'
                AND job_row.lease_expires_at <= claimed_at)
       )
     ORDER BY job_row.available_at, job_row.job_id
     LIMIT 1
     FOR UPDATE OF job_row, registry SKIP LOCKED;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    SELECT * INTO publication_job
      FROM ple_private.job AS claimed_job
     WHERE claimed_job.job_id = publication.job_id;

    UPDATE ple_private.job AS claimed_job
       SET state = 'leased', lease_token = p_lease_token,
           lease_expires_at = p_lease_expires_at,
           attempt_count = publication_job.attempt_count + 1
     WHERE claimed_job.job_id = publication_job.job_id
       AND claimed_job.job_kind = 'publish_public_assets'
       AND claimed_job.job_target_kind = 'public_asset_publication'
       AND claimed_job.question_id = publication.question_id
       AND claimed_job.revision_number = publication.revision_number
       AND claimed_job.attempt_count = publication_job.attempt_count
       AND (
            (claimed_job.state = 'ready' AND claimed_job.available_at <= claimed_at)
            OR (claimed_job.state = 'leased' AND claimed_job.lease_expires_at <= claimed_at)
       );
    -- The immutable target predicate above and the row lock establish the
    -- claim.  This explicit identity check keeps a future query edit from
    -- turning the publisher into a caller-directed Job executor.
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Asset Publication Job changed during claim';
    END IF;

    RETURN QUERY SELECT publication_job.job_id, publication.question_id,
        publication.revision_number, publication.asset_id,
        publication.source_object_id, publication.source_object_checksum,
        publication.public_object_id, publication.public_object_checksum,
        publication.public_byte_length, publication.verified_media_type,
        publication.intrinsic_width, publication.intrinsic_height,
        publication.delivery_id;
END
$$;

-- Server-only publisher commit.  Object bytes have already been copied and
-- verified by the publisher before this call.  This procedure commits their
-- one immutable Public Assets record and its Ready registry/delivery state in
-- the same transaction as completion of the exact leased publication Job.
CREATE FUNCTION ple_private.activate_question_asset_publication(
    p_job_id uuid,
    p_job_lease_token uuid
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    publication ple_private.question_asset_publication%ROWTYPE;
    publication_job ple_private.job%ROWTYPE;
    delivery ple_data.object_delivery%ROWTYPE;
    expected_public_address jsonb;
    activated_at timestamp with time zone := pg_catalog.clock_timestamp();
BEGIN
    IF p_job_id IS NULL OR p_job_lease_token IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Asset Publication activation requires a Job and lease token';
    END IF;

    SELECT * INTO publication
      FROM ple_private.question_asset_publication
     WHERE job_id = p_job_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Asset Publication activation does not own a publication Job';
    END IF;

    SELECT * INTO publication_job
      FROM ple_private.job
     WHERE job_id = p_job_id
     FOR UPDATE;
    IF NOT FOUND
       OR publication_job.job_kind <> 'publish_public_assets'
       OR publication_job.job_target_kind <> 'public_asset_publication'
       OR publication_job.question_id <> publication.question_id
       OR publication_job.revision_number <> publication.revision_number
       OR publication_job.state <> 'leased'
       OR publication_job.lease_token <> p_job_lease_token
       OR publication_job.lease_expires_at <= activated_at THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Asset Publication activation does not own the current Publish Public Assets Job lease';
    END IF;

    IF publication.publication_state <> 'pending' THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Question Asset Publication is not Pending';
    END IF;

    SELECT * INTO delivery
      FROM ple_data.object_delivery
     WHERE delivery_id = publication.delivery_id
     FOR UPDATE;
    IF NOT FOUND
       OR delivery.object_id <> publication.public_object_id
       OR delivery.sha256 <> publication.public_object_checksum
       OR delivery.media_type <> publication.verified_media_type
       OR delivery.byte_length <> publication.public_byte_length
       OR delivery.delivery_state <> 'pending'
       OR NOT EXISTS (
            SELECT 1
              FROM ple_data.question_asset_delivery AS asset_delivery
             WHERE asset_delivery.delivery_id = publication.delivery_id
               AND asset_delivery.object_id = publication.public_object_id
               AND asset_delivery.question_id = publication.question_id
               AND asset_delivery.revision_number = publication.revision_number
               AND asset_delivery.asset_id = publication.asset_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Asset Publication activation requires its exact Pending Question Asset Delivery';
    END IF;

    expected_public_address := jsonb_build_object(
        'kind', 'questionAsset',
        'questionRevision', jsonb_build_object(
            'questionId', publication.question_id,
            'revisionNumber', publication.revision_number
        ),
        'asset', publication.asset_id,
        'object', publication.public_object_id
    );
    INSERT INTO ple_private.object_record (
        object_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        publication.public_object_id, expected_public_address, 'public-assets',
        'question-asset', publication.public_object_checksum,
        publication.public_byte_length, publication.verified_media_type,
        activated_at
    ) ON CONFLICT (object_id) DO NOTHING;

    IF NOT EXISTS (
        SELECT 1 FROM ple_private.object_record AS public_record
         WHERE public_record.object_id = publication.public_object_id
           AND public_record.object_address = expected_public_address
           AND public_record.object_storage_area = 'public-assets'
           AND public_record.object_data_class = 'question-asset'
           AND public_record.sha256 = publication.public_object_checksum
           AND public_record.size_bytes = publication.public_byte_length
           AND public_record.media_type = publication.verified_media_type
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23505',
            MESSAGE = 'Question Asset Publication public Object Record is not its exact immutable rendition';
    END IF;

    UPDATE ple_data.object_delivery
       SET delivery_state = 'available'
     WHERE delivery_id = publication.delivery_id
       AND object_id = publication.public_object_id
       AND delivery_state = 'pending';
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Asset Publication Pending delivery changed during activation';
    END IF;

    PERFORM pg_catalog.set_config(
        'ple.question_asset_publication_activation', publication.job_id::text, true
    );
    UPDATE ple_private.question_asset_publication
       SET publication_state = 'ready'
     WHERE job_id = publication.job_id
       AND publication_state = 'pending';
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Asset Publication Pending registry changed during activation';
    END IF;

    UPDATE ple_private.job
       SET state = 'completed', lease_token = NULL, lease_expires_at = NULL,
           completed_at = activated_at
     WHERE job_id = publication.job_id
       AND state = 'leased'
       AND lease_token = p_job_lease_token
       AND lease_expires_at > activated_at;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Asset Publication Job lease changed during activation';
    END IF;
END
$$;

-- This private result is deliberately rendition-only: it is enough to build a
-- Question Presentation, but never exposes a source Object, storage address,
-- or a direct table capability to the server's HTTP role.
CREATE FUNCTION ple_private.select_ready_question_asset_renditions(
    p_question_id text,
    p_revision_number integer
) RETURNS TABLE (
    asset_id uuid,
    question_asset_checksum text,
    rendition_checksum text,
    intrinsic_width integer,
    intrinsic_height integer
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT publication.asset_id,
           pg_catalog.encode(publication.source_object_checksum, 'hex'),
           pg_catalog.encode(publication.public_object_checksum, 'hex'),
           publication.intrinsic_width,
           publication.intrinsic_height
      FROM ple_private.question_asset_publication AS publication
      JOIN ple_data.object_delivery AS delivery
        ON delivery.delivery_id = publication.delivery_id
       AND delivery.object_id = publication.public_object_id
      JOIN ple_data.question_asset_delivery AS asset_delivery
        ON asset_delivery.delivery_id = publication.delivery_id
       AND asset_delivery.object_id = publication.public_object_id
       AND asset_delivery.question_id = publication.question_id
       AND asset_delivery.revision_number = publication.revision_number
       AND asset_delivery.asset_id = publication.asset_id
      JOIN ple_private.object_record AS public_record
        ON public_record.object_id = publication.public_object_id
     WHERE publication.question_id = p_question_id
       AND publication.revision_number = p_revision_number
       AND publication.publication_state = 'ready'
       AND delivery.delivery_state = 'available'
       AND delivery.sha256 = publication.public_object_checksum
       AND delivery.media_type = publication.verified_media_type
       AND delivery.byte_length = publication.public_byte_length
       AND public_record.object_storage_area = 'public-assets'
       AND public_record.object_data_class = 'question-asset'
       AND public_record.sha256 = publication.public_object_checksum
       AND public_record.size_bytes = publication.public_byte_length
       AND public_record.media_type = publication.verified_media_type
     ORDER BY publication.asset_id
$$;

-- The application role resolves only ready, revision-exact public rendition
-- facts through this narrow definer boundary.  It never receives a private
-- selector grant, Object ID, Object Address, or source bytes.
SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.select_live_demo_ready_question_asset_renditions(
    p_question_id text,
    p_revision_number integer
) RETURNS TABLE (
    asset_id uuid,
    question_asset_checksum text,
    rendition_checksum text,
    intrinsic_width integer,
    intrinsic_height integer
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT asset_id, question_asset_checksum, rendition_checksum,
           intrinsic_width, intrinsic_height
      FROM ple_private.select_ready_question_asset_renditions(
          p_question_id, p_revision_number
      )
$$;
RESET ROLE;

-- The preceding API-owned wrapper deliberately resets its role.  Resume the
-- private owner before closing grants on private routines and relations.
SET LOCAL ROLE ple_private_owner;
REVOKE ALL PRIVILEGES ON FUNCTION
    ple_private.claim_question_asset_publication_job(uuid, timestamp with time zone),
    ple_private.activate_question_asset_publication(uuid, uuid),
    ple_private.select_ready_question_asset_renditions(text, integer)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_private.claim_question_asset_publication_job(uuid, timestamp with time zone),
    ple_private.activate_question_asset_publication(uuid, uuid)
TO ple_public_asset_publisher;
GRANT EXECUTE ON FUNCTION ple_private.select_ready_question_asset_renditions(text, integer)
    TO ple_api_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner, ple_public_asset_publisher;

-- Only the API owner closes and grants the API-owned wrapper itself.
SET LOCAL ROLE ple_api_owner;
REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.select_live_demo_ready_question_asset_renditions(text, integer)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.select_live_demo_ready_question_asset_renditions(text, integer)
TO ple_app;

SET LOCAL ROLE ple_private_owner;
ALTER TABLE ple_private.question_asset_publication ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_asset_publication FORCE ROW LEVEL SECURITY;
CREATE POLICY question_asset_publication_private_owner_access
    ON ple_private.question_asset_publication
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
REVOKE ALL PRIVILEGES ON TABLE ple_private.question_asset_publication FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_question_asset_publication_change() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.validate_question_asset_publication() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.require_complete_question_asset_publication_delivery() FROM PUBLIC;

COMMENT ON TABLE ple_private.question_asset_publication IS
    'Immutable Question Asset publication registry: a Pending revision-owned private still image, exact final Public Assets Object and delivery, and closed Publish Public Assets Job; only publisher activation may change it to Ready.';
COMMENT ON COLUMN ple_private.question_asset_publication.publication_state IS
    'Pending has no public delivery. Ready requires the exact immutable public Object Record and Available Question Asset Delivery.';

RESET ROLE;
