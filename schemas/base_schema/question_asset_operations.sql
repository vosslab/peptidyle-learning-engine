-- Late operations for immutable Question Revision public assets.  This module
-- follows Jobs and retained presentation evidence; it owns the resulting
-- publication transition and opaque resolver rather than a corrective layer.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.question_asset_publication
    ADD CONSTRAINT question_asset_publication_job_fkey
    FOREIGN KEY (job_id) REFERENCES ple_private.job(job_id);

CREATE FUNCTION ple_private.validate_question_asset_publication_job()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE publication ple_private.question_asset_publication%ROWTYPE;
BEGIN
    SELECT * INTO publication
      FROM ple_private.question_asset_publication
     WHERE job_id = NEW.job_id;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;
    IF NEW.job_kind <> 'publish_public_assets'
       OR NEW.job_target_kind <> 'public_asset_publication'
       OR NEW.worker_kind <> 'public_asset_publisher'
       OR NEW.question_submission_id IS NOT NULL
       OR NEW.question_id <> publication.question_id
       OR NEW.revision_number <> publication.revision_number THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Asset Publication requires its exact Public Asset publisher Job';
    END IF;
    RETURN NULL;
END $$;

CREATE CONSTRAINT TRIGGER question_asset_publication_has_exact_job
AFTER INSERT OR UPDATE ON ple_private.question_asset_publication
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
EXECUTE FUNCTION ple_private.validate_question_asset_publication_job();

CREATE FUNCTION ple_private.claim_question_asset_publication_job(
    p_lease_token uuid,
    p_lease_expires_at timestamptz
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
    claimed_at timestamptz := pg_catalog.clock_timestamp();
BEGIN
    IF p_lease_token IS NULL
       OR p_lease_expires_at <= claimed_at
       OR p_lease_expires_at > claimed_at + interval '300 seconds' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Asset Publication Job lease is invalid or exceeds 300 seconds';
    END IF;

    SELECT registry.* INTO publication
      FROM ple_private.job AS candidate
      JOIN ple_private.question_asset_publication AS registry
        ON registry.job_id = candidate.job_id
     WHERE registry.publication_state = 'pending'
       AND candidate.job_kind = 'publish_public_assets'
       AND candidate.job_target_kind = 'public_asset_publication'
       AND candidate.worker_kind = 'public_asset_publisher'
       AND candidate.question_id = registry.question_id
       AND candidate.revision_number = registry.revision_number
       AND candidate.attempt_count < candidate.max_attempts
       AND ((candidate.state = 'ready' AND candidate.available_at <= claimed_at)
         OR (candidate.state = 'leased' AND candidate.lease_expires_at <= claimed_at))
     ORDER BY candidate.available_at, candidate.job_id
     LIMIT 1 FOR UPDATE OF candidate, registry SKIP LOCKED;
    IF NOT FOUND THEN
        RETURN;
    END IF;

    SELECT * INTO publication_job FROM ple_private.job
     WHERE job_id = publication.job_id;
    UPDATE ple_private.job
       SET state = 'leased', lease_token = p_lease_token,
           lease_expires_at = p_lease_expires_at,
           attempt_count = publication_job.attempt_count + 1
     WHERE job_id = publication_job.job_id
       AND job_kind = 'publish_public_assets'
       AND job_target_kind = 'public_asset_publication'
       AND worker_kind = 'public_asset_publisher'
       AND question_id = publication.question_id
       AND revision_number = publication.revision_number
       AND attempt_count = publication_job.attempt_count
       AND ((state = 'ready' AND available_at <= claimed_at)
         OR (state = 'leased' AND lease_expires_at <= claimed_at));
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
END $$;

CREATE FUNCTION ple_private.activate_question_asset_publication(
    p_job_id uuid,
    p_job_lease_token uuid
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    publication ple_private.question_asset_publication%ROWTYPE;
    publication_job ple_private.job%ROWTYPE;
    delivery ple_data.object_delivery%ROWTYPE;
    expected_public_address jsonb;
    activated_at timestamptz := pg_catalog.clock_timestamp();
BEGIN
    IF p_job_id IS NULL OR p_job_lease_token IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Asset Publication activation requires a Job and lease token';
    END IF;
    SELECT * INTO publication FROM ple_private.question_asset_publication
     WHERE job_id = p_job_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Asset Publication activation does not own a publication Job';
    END IF;
    SELECT * INTO publication_job FROM ple_private.job
     WHERE job_id = p_job_id FOR UPDATE;
    IF NOT FOUND
       OR publication_job.job_kind <> 'publish_public_assets'
       OR publication_job.job_target_kind <> 'public_asset_publication'
       OR publication_job.worker_kind <> 'public_asset_publisher'
       OR publication_job.question_id <> publication.question_id
       OR publication_job.revision_number <> publication.revision_number
       OR publication_job.state <> 'leased'
       OR publication_job.lease_token <> p_job_lease_token
       OR publication_job.lease_expires_at <= activated_at THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Asset Publication activation does not own the current Job lease';
    END IF;
    IF publication.publication_state <> 'pending' THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Question Asset Publication is not Pending';
    END IF;
    SELECT * INTO delivery FROM ple_data.object_delivery
     WHERE delivery_id = publication.delivery_id FOR UPDATE;
    IF NOT FOUND
       OR delivery.object_id <> publication.public_object_id
       OR delivery.sha256 <> publication.public_object_checksum
       OR delivery.media_type <> publication.verified_media_type
       OR delivery.byte_length <> publication.public_byte_length
       OR delivery.delivery_state <> 'pending'
       OR NOT EXISTS (
            SELECT 1 FROM ple_data.question_asset_delivery AS asset_delivery
             WHERE asset_delivery.delivery_id = publication.delivery_id
               AND asset_delivery.object_id = publication.public_object_id
               AND asset_delivery.question_id = publication.question_id
               AND asset_delivery.revision_number = publication.revision_number
               AND asset_delivery.asset_id = publication.asset_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Asset Publication activation requires its exact Pending delivery';
    END IF;

    expected_public_address := jsonb_build_object(
        'kind', 'questionAsset',
        'questionRevision', jsonb_build_object(
            'questionId', publication.question_id,
            'revisionNumber', publication.revision_number),
        'asset', publication.asset_id, 'object', publication.public_object_id);
    INSERT INTO ple_private.object_record (
        object_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        publication.public_object_id, expected_public_address, 'public-assets',
        'question-asset', publication.public_object_checksum,
        publication.public_byte_length, publication.verified_media_type, activated_at
    ) ON CONFLICT (object_id) DO NOTHING;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.object_record AS record
         WHERE record.object_id = publication.public_object_id
           AND record.object_address = expected_public_address
           AND record.object_storage_area = 'public-assets'
           AND record.object_data_class = 'question-asset'
           AND record.sha256 = publication.public_object_checksum
           AND record.size_bytes = publication.public_byte_length
           AND record.media_type = publication.verified_media_type
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23505',
            MESSAGE = 'Question Asset Publication public Object Record is not its exact immutable rendition';
    END IF;

    UPDATE ple_data.object_delivery SET delivery_state = 'available'
     WHERE delivery_id = publication.delivery_id
       AND object_id = publication.public_object_id
       AND delivery_state = 'pending';
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Asset Publication Pending delivery changed during activation';
    END IF;
    PERFORM pg_catalog.set_config(
        'ple.question_asset_publication_activation', publication.job_id::text, true);
    UPDATE ple_private.question_asset_publication SET publication_state = 'ready'
     WHERE job_id = publication.job_id AND publication_state = 'pending';
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Asset Publication Pending registry changed during activation';
    END IF;
    UPDATE ple_private.job
       SET state = 'completed', lease_token = NULL, lease_expires_at = NULL,
           completed_at = activated_at
     WHERE job_id = publication.job_id
       AND state = 'leased' AND lease_token = p_job_lease_token
       AND lease_expires_at > activated_at;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Asset Publication Job lease changed during activation';
    END IF;
END $$;

-- Returns one exact Question Revision rendition only for an Instructor or the
-- Student whose retained presentation names it.  The public identity is the
-- immutable Question Revision plus its asset UUID: an asset UUID is scoped to
-- a Question Revision and is deliberately not made globally unique merely to
-- shorten this resolver's signature.  An empty result intentionally conceals
-- absent, pending, and unauthorized assets alike.
CREATE FUNCTION ple_private.resolve_ready_question_asset_delivery(
    p_question_id text,
    p_revision_number integer,
    p_asset_id uuid
)
RETURNS TABLE (
    question_id text, revision_number integer, asset_id uuid,
    public_object_id uuid, rendition_checksum bytea
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT publication.question_id, publication.revision_number,
           publication.asset_id, publication.public_object_id,
           publication.public_object_checksum
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
       AND publication.asset_id = p_asset_id
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
       AND (
            ple_api.current_session_account_is_instructor()
            OR EXISTS (
                SELECT 1
                  FROM ple_private.issued_question AS issued
                  JOIN ple_private.assignment_attempt AS attempt
                    ON attempt.assignment_attempt_id = issued.assignment_attempt_id
                  JOIN ple_private.question_attempt AS question_attempt
                    ON question_attempt.issued_question_id = issued.issued_question_id
                  JOIN ple_private.question_attempt_presentation_asset_rendition AS presented_asset
                    ON presented_asset.question_attempt_id = question_attempt.question_attempt_id
                   AND presented_asset.asset_id = publication.asset_id
                   AND presented_asset.rendition_checksum = publication.public_object_checksum
                  JOIN ple_private.question_attempt_presentation_asset_binding AS presented_assets
                    ON presented_assets.question_attempt_id = presented_asset.question_attempt_id
                  JOIN ple_private.question_attempt_presentation_binding AS presentation
                    ON presentation.question_attempt_id = presented_asset.question_attempt_id
                  JOIN ple_data.assignment AS assignment
                    ON assignment.assignment_id = attempt.assignment_id
                 WHERE issued.question_id = publication.question_id
                   AND issued.revision_number = publication.revision_number
                   AND ple_api.current_session_account_owns_student_record(
                       assignment.course_id, attempt.student_record_id)
            )
       )
$$;

REVOKE ALL ON FUNCTION ple_private.validate_question_asset_publication_job(),
    ple_private.claim_question_asset_publication_job(uuid, timestamptz),
    ple_private.activate_question_asset_publication(uuid, uuid),
    ple_private.resolve_ready_question_asset_delivery(text, integer, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.claim_question_asset_publication_job(uuid, timestamptz),
    ple_private.activate_question_asset_publication(uuid, uuid) TO ple_public_asset_publisher;
GRANT EXECUTE ON FUNCTION ple_private.resolve_ready_question_asset_delivery(text, integer, uuid) TO ple_api_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner, ple_public_asset_publisher;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.resolve_ready_question_asset(
    p_question_id text,
    p_revision_number integer,
    p_asset_id uuid
)
RETURNS TABLE (
    question_id text, revision_number integer, asset_id uuid,
    public_object_id uuid, rendition_checksum bytea
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT question_id, revision_number, asset_id, public_object_id, rendition_checksum
      FROM ple_private.resolve_ready_question_asset_delivery(
          p_question_id, p_revision_number, p_asset_id)
$$;
REVOKE ALL ON FUNCTION ple_api.resolve_ready_question_asset(text, integer, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.resolve_ready_question_asset(text, integer, uuid) TO ple_app;

RESET ROLE;
