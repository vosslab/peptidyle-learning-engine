-- Functions and triggers from question_thumbnails.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.reject_question_revision_thumbnail_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Question Revision Thumbnail is immutable';
END
$$;

CREATE FUNCTION ple_private.validate_question_revision_thumbnail_object_record()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
DECLARE expected_address jsonb;
BEGIN
    expected_address := jsonb_build_object(
        'kind', 'questionThumbnail',
        'publishedQuestionRevisionTuple', jsonb_build_object(
            'publishedQuestionId', NEW.published_question_id,
            'revisionNumber', NEW.revision_number),
        'objectId', NEW.object_record_id);
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.object_record AS record
         WHERE record.object_record_id = NEW.object_record_id
           AND record.object_address = expected_address
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = 'question-thumbnail'
           AND record.media_type = 'image/webp'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision Thumbnail requires its exact private WebP Object Record';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER question_revision_thumbnail_has_exact_object_record
BEFORE INSERT ON ple_private.question_revision_thumbnail
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_revision_thumbnail_object_record();

CREATE TRIGGER question_revision_thumbnail_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_revision_thumbnail
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_revision_thumbnail_change();

-- This is the sole database resolver for thumbnail delivery. The existing
-- Library relation supplies its active-Instructor/Sysadmin authorization and
-- historical exact-Revision behavior before private storage metadata is read.
CREATE FUNCTION ple_private.resolve_question_thumbnail(
    p_published_question_id text, p_revision_number integer
) RETURNS TABLE (
    object_record_id uuid, object_address jsonb, object_storage_area text,
    object_data_class text, sha256 bytea, size_bytes bigint, media_type text,
    created_at_millis bigint, intrinsic_width integer, intrinsic_height integer
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT record.object_record_id, record.object_address,
           record.object_storage_area::text, record.object_data_class::text,
           record.sha256, record.size_bytes, record.media_type,
           floor(extract(epoch FROM record.created_at) * 1000)::bigint,
           thumbnail.intrinsic_width, thumbnail.intrinsic_height
      FROM ple_private.question_library_entries(
            p_published_question_id, p_revision_number, false
      ) AS entry
      JOIN ple_private.question_revision_thumbnail AS thumbnail
        ON thumbnail.published_question_id = entry.published_question_id
       AND thumbnail.revision_number = entry.revision_number
      JOIN ple_private.object_record AS record
        ON record.object_record_id = thumbnail.object_record_id
$$;

-- Explicit maintenance only: neither startup nor Library browsing calls this
-- projection. LEFT JOINs preserve malformed/missing evidence for exact reporting.
CREATE FUNCTION ple_private.question_thumbnail_backfill_snapshot(
    p_published_question_id text DEFAULT NULL, p_revision_number integer DEFAULT NULL
) RETURNS TABLE (
    published_question_id text, revision_number integer,
    source_binding jsonb, source_record jsonb, thumbnail jsonb
) LANGUAGE plpgsql STABLE SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF (p_published_question_id IS NULL) <> (p_revision_number IS NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Maintenance requires both exact Revision fields';
    END IF;
    RETURN QUERY
    SELECT revision.published_question_id::text, revision.revision_number,
           to_jsonb(binding),
           CASE WHEN source.object_record_id IS NOT NULL THEN jsonb_build_object(
               'id', source.object_record_id, 'address', source.object_address,
               'storageArea', source.object_storage_area, 'dataClass', source.object_data_class,
               'sha256', encode(source.sha256, 'hex'), 'sizeBytes', source.size_bytes,
               'mediaType', source.media_type,
               'createdAt', floor(extract(epoch FROM source.created_at) * 1000)::bigint,
               'publishedQuestionRevisionTuple', jsonb_build_object(
                   'publishedQuestionId', revision.published_question_id,
                   'revisionNumber', revision.revision_number)) END,
           CASE WHEN artifact.published_question_id IS NOT NULL THEN jsonb_build_object(
               'intrinsicWidth', artifact.intrinsic_width, 'intrinsicHeight', artifact.intrinsic_height,
               'objectRecord', CASE WHEN image.object_record_id IS NOT NULL THEN jsonb_build_object(
                   'id', image.object_record_id, 'address', image.object_address,
                   'storageArea', image.object_storage_area, 'dataClass', image.object_data_class,
                   'sha256', encode(image.sha256, 'hex'), 'sizeBytes', image.size_bytes,
                   'mediaType', image.media_type,
                   'createdAt', floor(extract(epoch FROM image.created_at) * 1000)::bigint,
                   'publishedQuestionRevisionTuple', jsonb_build_object(
                       'publishedQuestionId', revision.published_question_id,
                       'revisionNumber', revision.revision_number)) END) END
      FROM ple_data.question_revision AS revision
      LEFT JOIN ple_private.question_revision_source_binding AS binding
        USING (published_question_id, revision_number)
      LEFT JOIN ple_private.object_record AS source
        ON source.object_record_id = binding.source_object_record_id
      LEFT JOIN ple_private.question_revision_thumbnail AS artifact
        USING (published_question_id, revision_number)
      LEFT JOIN ple_private.object_record AS image
        ON image.object_record_id = artifact.object_record_id
     WHERE revision.backend = 'webwork'
       AND (p_published_question_id IS NULL OR (
           revision.published_question_id = p_published_question_id
           AND revision.revision_number = p_revision_number))
     ORDER BY revision.published_question_id, revision.revision_number;
END
$$;

-- ASVS 8.2.1/15.4.2: owner-only maintenance serializes on the existing
-- lineage and its exact source binding. A concurrent winner remains immutable.
CREATE FUNCTION ple_private.register_existing_question_thumbnail(
    p_published_question_id text, p_revision_number integer,
    p_source_object_id uuid, p_source_sha256 bytea, p_source_address jsonb,
    p_thumbnail_object_id uuid, p_thumbnail_address jsonb,
    p_thumbnail_sha256 bytea, p_thumbnail_size bigint, p_created_at_millis bigint
) RETURNS boolean LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    revision ple_data.question_revision%ROWTYPE;
    binding ple_private.question_revision_source_binding%ROWTYPE;
    source ple_private.object_record%ROWTYPE;
    expected_source_address jsonb;
    expected_thumbnail_address jsonb;
BEGIN
    -- Reuse publication's existing lock-only lineage privilege/policy. Immutable
    -- Revision rows deliberately carry no UPDATE privilege for this owner.
    PERFORM 1 FROM ple_data.published_question AS item
     WHERE item.published_question_id = p_published_question_id FOR UPDATE;
    SELECT * INTO STRICT revision FROM ple_data.question_revision AS item
     WHERE item.published_question_id = p_published_question_id
       AND item.revision_number = p_revision_number;
    SELECT * INTO STRICT binding FROM ple_private.question_revision_source_binding AS item
     WHERE item.published_question_id = p_published_question_id
       AND item.revision_number = p_revision_number FOR UPDATE;
    SELECT * INTO STRICT source FROM ple_private.object_record AS item
     WHERE item.object_record_id = binding.source_object_record_id FOR UPDATE;
    expected_source_address := jsonb_build_object('kind', 'questionSource',
        'publishedQuestionRevisionTuple', jsonb_build_object(
            'publishedQuestionId', p_published_question_id, 'revisionNumber', p_revision_number),
        'objectId', p_source_object_id);
    expected_thumbnail_address := jsonb_build_object('kind', 'questionThumbnail',
        'publishedQuestionRevisionTuple', jsonb_build_object(
            'publishedQuestionId', p_published_question_id, 'revisionNumber', p_revision_number),
        'objectId', p_thumbnail_object_id);
    -- ASVS 2.2.1/2.2.3: recheck authoritative source facts under the lock.
    IF revision.backend <> 'webwork' OR binding.backend <> 'webwork'
       OR binding.question_format NOT IN ('webworkPg', 'webworkPgml')
       OR binding.webwork_pg_path IS NULL
       OR source.object_record_id IS DISTINCT FROM p_source_object_id
       OR source.sha256 IS DISTINCT FROM p_source_sha256
       OR binding.source_object_checksum IS DISTINCT FROM encode(source.sha256, 'hex')
       OR source.object_address IS DISTINCT FROM expected_source_address
       OR p_source_address IS DISTINCT FROM expected_source_address
       OR source.object_storage_area <> 'private-content'
       OR source.object_data_class <> 'question-source'
       OR source.media_type <> 'text/x-wework-pg' THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Maintenance source evidence changed or is invalid';
    END IF;
    IF EXISTS (SELECT 1 FROM ple_private.question_revision_thumbnail AS item
        WHERE item.published_question_id = p_published_question_id
          AND item.revision_number = p_revision_number) THEN
        RETURN false;
    END IF;
    IF p_thumbnail_object_id IS NULL
       OR p_thumbnail_address IS DISTINCT FROM expected_thumbnail_address
       OR p_thumbnail_sha256 IS NULL OR octet_length(p_thumbnail_sha256) <> 32
       OR p_thumbnail_size IS NULL OR p_thumbnail_size NOT BETWEEN 1 AND 2000000
       OR p_created_at_millis IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Maintenance thumbnail evidence is invalid';
    END IF;
    INSERT INTO ple_private.object_record(
        object_record_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (p_thumbnail_object_id, expected_thumbnail_address, 'private-content', 'question-thumbnail',
        p_thumbnail_sha256, p_thumbnail_size, 'image/webp',
        to_timestamp(p_created_at_millis::double precision / 1000.0));
    INSERT INTO ple_private.question_revision_thumbnail(
        published_question_id, revision_number, object_record_id, intrinsic_width, intrinsic_height
    ) VALUES (p_published_question_id, p_revision_number, p_thumbnail_object_id, 256, 256);
    RETURN true;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.resolve_question_thumbnail(
    p_published_question_id text, p_revision_number integer
) RETURNS TABLE (
    object_record_id uuid, object_address jsonb, object_storage_area text,
    object_data_class text, sha256 bytea, size_bytes bigint, media_type text,
    created_at_millis bigint, intrinsic_width integer, intrinsic_height integer
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.resolve_question_thumbnail(
        p_published_question_id, p_revision_number)
$$;
