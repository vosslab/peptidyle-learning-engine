-- Exact Question Source transfer for one Question Revision publication.
--
-- A publication coordinator copies source bytes to the Question Revision Object
-- Address before this transaction. This helper records the copied immutable
-- bytes and source relationship together; it never exposes a partial
-- published-source registration capability.

SET LOCAL ROLE ple_data_owner;
-- The private coordinator verifies only the target's answer-free backend;
-- private grading records and source bytes remain inaccessible through this grant.
GRANT SELECT ON TABLE ple_data.question_revision TO ple_private_owner;
CREATE POLICY question_revision_private_source_transfer_lookup ON ple_data.question_revision
    FOR SELECT TO ple_private_owner USING (true);
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

-- ASVS 2.1.1, 2.2.1-2.2.3, 2.3.1-2.3.3, 8.1.1-8.2.3, and 14.2.4:
-- publication preserves the exact immutable source bytes, verifies every
-- cross-record relationship at the trusted database boundary, and leaves no
-- public publication event without its revision-owned Question Source.
CREATE FUNCTION ple_private.transfer_draft_question_source_to_question_revision(
    p_question_source_uuid uuid,
    p_draft_question_uuid uuid,
    p_draft_question_revision_number integer,
    p_question_id text,
    p_revision_number integer,
    p_imathas_profile text,
    p_source_object_id uuid,
    p_source_object_address jsonb,
    p_source_object_sha256 bytea,
    p_source_object_size_bytes bigint,
    p_source_object_media_type text,
    p_source_object_created_at_millis bigint
)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    source_record ple_private.question_source%ROWTYPE;
    target_backend text;
    expected_address jsonb := jsonb_build_object(
        'kind', 'questionSource',
        'questionRevision', jsonb_build_object(
            'questionId', p_question_id,
            'revisionNumber', p_revision_number
        ),
        'object', p_source_object_id
    );
    expected_created_at timestamp with time zone :=
        pg_catalog.to_timestamp(p_source_object_created_at_millis::double precision / 1000.0);
    resolved_question_source_uuid uuid;
BEGIN
    SELECT source.*
      INTO source_record
      FROM ple_private.question_source AS source
      JOIN ple_private.draft_question_revision AS draft_revision
        ON draft_revision.draft_question_revision_uuid = source.draft_question_revision_uuid
     WHERE draft_revision.draft_question_uuid = p_draft_question_uuid
       AND draft_revision.revision_number = p_draft_question_revision_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision publication requires one Draft Question Source';
    END IF;

    SELECT revision.backend
      INTO target_backend
      FROM ple_data.question_revision AS revision
     WHERE revision.question_id = p_question_id
       AND revision.revision_number = p_revision_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Source transfer requires its exact Question Revision';
    END IF;
    IF target_backend <> source_record.backend THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision Backend must match its Draft Question Source Backend';
    END IF;
    IF (source_record.backend = 'imathas'
            AND NOT COALESCE(p_imathas_profile ~ '^[A-Za-z0-9._-]{1,160}$', false))
       OR (source_record.backend <> 'imathas' AND p_imathas_profile IS NOT NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Revision Source requires a pinned iMathAS Profile only for iMathAS';
    END IF;
    IF p_question_source_uuid = source_record.question_source_uuid
       OR p_source_object_id = source_record.source_object_id THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Revision publication requires distinct Question Source and Source Object identities';
    END IF;
    IF p_source_object_sha256 IS NULL
       OR pg_catalog.octet_length(p_source_object_sha256) <> 32
       OR p_source_object_size_bytes < 0
       OR char_length(btrim(p_source_object_media_type)) NOT BETWEEN 1 AND 255
       OR jsonb_typeof(p_source_object_address) <> 'object'
       OR p_source_object_address <> expected_address THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Revision Source Object Record requires its exact typed immutable facts';
    END IF;
    IF encode(p_source_object_sha256, 'hex') <> source_record.source_object_checksum THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision Source Object Checksum must match the Draft Question Source';
    END IF;

    INSERT INTO ple_private.object_record (
        object_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        p_source_object_id, expected_address, 'private-content', 'question-source',
        p_source_object_sha256, p_source_object_size_bytes, p_source_object_media_type,
        expected_created_at
    ) ON CONFLICT DO NOTHING;
    IF NOT FOUND AND NOT EXISTS (
        SELECT 1
          FROM ple_private.object_record AS record
         WHERE record.object_id = p_source_object_id
           AND record.object_address = expected_address
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = 'question-source'
           AND record.sha256 = p_source_object_sha256
           AND record.size_bytes = p_source_object_size_bytes
           AND record.media_type = p_source_object_media_type
           AND record.created_at = expected_created_at
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23505',
            MESSAGE = 'Question Revision Source Object identity or address already names different immutable bytes';
    END IF;

    INSERT INTO ple_private.question_source (
        question_source_uuid, question_id, revision_number, backend, question_format,
        question_type, webwork_pg_path, qti_package_item_identifier,
        imathas_deployment_reference, imathas_item_reference, imathas_profile,
        source_object_id, source_object_checksum,
        public_content_checksum, created_at, updated_at
    ) VALUES (
        p_question_source_uuid, p_question_id, p_revision_number, source_record.backend,
        source_record.question_format, source_record.question_type,
        source_record.webwork_pg_path, source_record.qti_package_item_identifier,
        source_record.imathas_deployment_reference, source_record.imathas_item_reference,
        p_imathas_profile,
        p_source_object_id, source_record.source_object_checksum,
        source_record.public_content_checksum, expected_created_at, expected_created_at
    ) ON CONFLICT DO NOTHING
    RETURNING question_source_uuid INTO resolved_question_source_uuid;
    IF FOUND THEN
        PERFORM ple_private.publish_question_fork_source(
            p_draft_question_uuid, p_question_id
        );
        RETURN resolved_question_source_uuid;
    END IF;

    SELECT source.question_source_uuid
      INTO resolved_question_source_uuid
      FROM ple_private.question_source AS source
     WHERE source.question_id = p_question_id
       AND source.revision_number = p_revision_number
       AND source.backend = source_record.backend
       AND source.question_format = source_record.question_format
       AND source.question_type = source_record.question_type
       AND source.webwork_pg_path IS NOT DISTINCT FROM source_record.webwork_pg_path
       AND source.qti_package_item_identifier IS NOT DISTINCT FROM source_record.qti_package_item_identifier
       AND source.workspace_import_id IS NULL
       AND source.imathas_deployment_reference IS NOT DISTINCT FROM source_record.imathas_deployment_reference
       AND source.imathas_item_reference IS NOT DISTINCT FROM source_record.imathas_item_reference
       AND source.imathas_profile IS NOT DISTINCT FROM p_imathas_profile
       AND source.source_object_id = p_source_object_id
       AND source.source_object_checksum = source_record.source_object_checksum
       AND source.public_content_checksum = source_record.public_content_checksum;
    IF FOUND THEN
        PERFORM ple_private.publish_question_fork_source(
            p_draft_question_uuid, p_question_id
        );
        RETURN resolved_question_source_uuid;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '23505',
        MESSAGE = 'Question Revision or Question Source identity already names different immutable facts';
END
$$;

CREATE FUNCTION ple_private.question_revision_has_question_source(
    p_question_id text,
    p_revision_number integer
)
RETURNS boolean LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ple_private.question_source AS source
         WHERE source.question_id = p_question_id
           AND source.revision_number = p_revision_number
    )
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.question_revision_has_question_source(text, integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.question_revision_has_question_source(text, integer) TO ple_data_owner;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.transfer_draft_question_source_to_question_revision(
    uuid, uuid, integer, text, integer, text, uuid, jsonb, bytea, bigint, text, bigint
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.transfer_draft_question_source_to_question_revision(
    uuid, uuid, integer, text, integer, text, uuid, jsonb, bytea, bigint, text, bigint
) TO ple_api_owner;

RESET ROLE;
SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.validate_question_publication_has_question_source()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT ple_private.question_revision_has_question_source(NEW.question_id, NEW.revision_number) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Publication requires an exact Question Revision Source Object';
    END IF;
    RETURN NULL;
END
$$;

CREATE CONSTRAINT TRIGGER question_publication_event_has_question_source
AFTER INSERT ON ple_data.question_publication_event
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_publication_has_question_source();

REVOKE ALL PRIVILEGES ON FUNCTION ple_data.validate_question_publication_has_question_source() FROM PUBLIC;

COMMENT ON FUNCTION ple_data.validate_question_publication_has_question_source() IS
    'Requires each Question Publication Event to commit with an exact Question Revision-owned Question Source.';

RESET ROLE;

SET LOCAL ROLE ple_private_owner;
COMMENT ON FUNCTION ple_private.transfer_draft_question_source_to_question_revision(
    uuid, uuid, integer, text, integer, text, uuid, jsonb, bytea, bigint, text, bigint
) IS 'Private publication-coordinator helper that resolves one exact Draft Question Revision, then records one copied immutable Question Source Object and one exact Question Revision Source relationship; no direct browser capability.';
COMMENT ON FUNCTION ple_private.question_revision_has_question_source(text, integer) IS
    'Private source-existence predicate used by the Question Publication Event integrity trigger.';
RESET ROLE;
