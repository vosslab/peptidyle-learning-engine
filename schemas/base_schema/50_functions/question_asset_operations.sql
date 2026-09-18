-- Functions, triggers, and views from question_asset_operations.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.validate_question_asset_publication_job()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE
    publication ple_private.question_asset_publication%ROWTYPE;
    publication_job ple_private.job%ROWTYPE;
BEGIN
    SELECT * INTO publication
      FROM ple_private.question_asset_publication
     WHERE job_id = NEW.job_id;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;
    SELECT * INTO publication_job
      FROM ple_private.job
     WHERE job_id = publication.job_id;
    -- ASVS 2.1.2, 2.3.3: the deferred check binds the current publication
    -- atomically to its exact current immutable Job contract.
    IF NOT FOUND
       OR publication_job.job_kind IS DISTINCT FROM 'publish_public_assets'
       OR publication_job.job_target_kind IS DISTINCT FROM 'public_asset_publication'
       OR publication_job.worker_kind IS DISTINCT FROM 'public_asset_publisher'
       OR publication_job.published_question_id IS DISTINCT FROM publication.published_question_id
       OR publication_job.revision_number IS DISTINCT FROM publication.revision_number THEN
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
    published_question_id text,
    revision_number integer,
    asset_id uuid,
    source_object_record_id uuid,
    source_object_checksum bytea,
    public_object_id uuid,
    public_object_checksum bytea,
    public_byte_length bigint,
    verified_media_type text,
    intrinsic_width integer,
    intrinsic_height integer,
    object_delivery_id uuid
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
       AND candidate.published_question_id = registry.published_question_id
       AND candidate.revision_number = registry.revision_number
       AND candidate.attempt_count < candidate.max_attempts
       AND ((candidate.state = 'ready' AND candidate.available_at <= claimed_at)
         OR (candidate.state = 'leased' AND candidate.lease_expires_at <= claimed_at))
     ORDER BY candidate.available_at, candidate.job_id
     LIMIT 1 FOR UPDATE OF candidate, registry SKIP LOCKED;
    IF NOT FOUND THEN
        RETURN;
    END IF;

    SELECT job.* INTO publication_job
      FROM ple_private.job AS job
     WHERE job.job_id = publication.job_id;
    UPDATE ple_private.job AS job
       SET state = 'leased', lease_token = p_lease_token,
           lease_expires_at = p_lease_expires_at,
           attempt_count = publication_job.attempt_count + 1
     WHERE job.job_id = publication_job.job_id
       AND job.job_kind = 'publish_public_assets'
       AND job.job_target_kind = 'public_asset_publication'
       AND job.worker_kind = 'public_asset_publisher'
       AND job.published_question_id = publication.published_question_id
       AND job.revision_number = publication.revision_number
       AND job.attempt_count = publication_job.attempt_count
       AND ((job.state = 'ready' AND job.available_at <= claimed_at)
         OR (job.state = 'leased' AND job.lease_expires_at <= claimed_at));
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Question Asset Publication Job changed during claim';
    END IF;

    RETURN QUERY SELECT publication_job.job_id, publication.published_question_id,
        publication.revision_number, publication.asset_id,
        publication.source_object_record_id, publication.source_object_checksum,
        publication.public_object_id, publication.public_object_checksum,
        publication.public_byte_length, publication.verified_media_type,
        publication.intrinsic_width, publication.intrinsic_height,
        publication.object_delivery_id;
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
       OR publication_job.published_question_id <> publication.published_question_id
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
     WHERE object_delivery_id = publication.object_delivery_id FOR UPDATE;
    IF NOT FOUND
       OR delivery.object_record_id <> publication.public_object_id
       OR delivery.sha256 <> publication.public_object_checksum
       OR delivery.media_type <> publication.verified_media_type
       OR delivery.byte_length <> publication.public_byte_length
       OR delivery.delivery_state <> 'pending'
       OR NOT EXISTS (
            SELECT 1 FROM ple_data.question_asset_delivery AS asset_delivery
             WHERE asset_delivery.object_delivery_id = publication.object_delivery_id
               AND asset_delivery.object_record_id = publication.public_object_id
               AND asset_delivery.published_question_id = publication.published_question_id
               AND asset_delivery.revision_number = publication.revision_number
               AND asset_delivery.asset_id = publication.asset_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Asset Publication activation requires its exact Pending delivery';
    END IF;

    expected_public_address := jsonb_build_object(
        'kind', 'questionAsset',
        'questionRevision', jsonb_build_object(
            'questionId', publication.published_question_id,
            'revisionNumber', publication.revision_number),
        'asset', publication.asset_id, 'object', publication.public_object_id);
    INSERT INTO ple_private.object_record (
        object_record_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        publication.public_object_id, expected_public_address, 'public-assets',
        'question-asset', publication.public_object_checksum,
        publication.public_byte_length, publication.verified_media_type, activated_at
    ) ON CONFLICT (object_record_id) DO NOTHING;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.object_record AS record
         WHERE record.object_record_id = publication.public_object_id
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
     WHERE object_delivery_id = publication.object_delivery_id
       AND object_record_id = publication.public_object_id
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
    p_published_question_id text,
    p_revision_number integer,
    p_asset_id uuid
)
RETURNS TABLE (
    published_question_id text, revision_number integer, asset_id uuid,
    public_object_id uuid, rendition_checksum bytea
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT publication.published_question_id, publication.revision_number,
           publication.asset_id, publication.public_object_id,
           publication.public_object_checksum
      FROM ple_private.question_asset_publication AS publication
      JOIN ple_data.object_delivery AS delivery
        ON delivery.object_delivery_id = publication.object_delivery_id
       AND delivery.object_record_id = publication.public_object_id
      JOIN ple_data.question_asset_delivery AS asset_delivery
        ON asset_delivery.object_delivery_id = publication.object_delivery_id
       AND asset_delivery.object_record_id = publication.public_object_id
       AND asset_delivery.published_question_id = publication.published_question_id
       AND asset_delivery.revision_number = publication.revision_number
       AND asset_delivery.asset_id = publication.asset_id
      JOIN ple_private.object_record AS public_record
        ON public_record.object_record_id = publication.public_object_id
     WHERE publication.published_question_id = p_published_question_id
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
                  JOIN ple_private.assessment_attempt AS assessment_attempt
                    ON assessment_attempt.assessment_attempt_id = issued.assessment_attempt_id
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
                  JOIN ple_data.assessment AS assessment
                    ON assessment.assessment_id = assessment_attempt.assessment_id
                 WHERE issued.published_question_id = publication.published_question_id
                   AND issued.revision_number = publication.revision_number
                   AND ple_api.current_session_account_owns_student_record(
                       assessment.course_instance_id, assessment_attempt.student_record_id)
            )
       )
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.resolve_ready_question_asset(
    p_published_question_id text,
    p_revision_number integer,
    p_asset_id uuid
)
RETURNS TABLE (
    published_question_id text, revision_number integer, asset_id uuid,
    public_object_id uuid, rendition_checksum bytea
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT published_question_id, revision_number, asset_id, public_object_id, rendition_checksum
      FROM ple_private.resolve_ready_question_asset_delivery(
          p_published_question_id, p_revision_number, p_asset_id)
$$;

