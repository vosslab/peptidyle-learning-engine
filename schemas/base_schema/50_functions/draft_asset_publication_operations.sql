-- Functions, triggers, and views from draft_asset_publication_operations.sql.

SET LOCAL ROLE ple_private_owner;

-- Late private-only helper: authoring publication calls this inside its final transaction.
-- Requires Assets and Jobs to exist; never a standalone ple_app command.
CREATE FUNCTION ple_private.bind_draft_asset_publication(
    p_draft_uuid uuid, p_workspace_id uuid, p_question_id text, p_revision integer,
    p_backend text, p_question_type text, p_asset jsonb, p_published_at timestamptz
) RETURNS void LANGUAGE plpgsql
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_asset_id uuid; source_id uuid; public_id uuid; v_delivery_id uuid; v_job_id uuid;
    width integer; height integer; checksum bytea; v_byte_length bigint;
    created_at_millis bigint; v_media_type text; expected_address jsonb;
    draft_asset ple_private.draft_question_asset%ROWTYPE;
    record ple_private.object_record%ROWTYPE;
BEGIN
    IF p_backend IS DISTINCT FROM 'ple' OR p_question_type IS DISTINCT FROM 'hotspot' THEN
        IF p_asset IS NOT NULL THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Only native HOTSPOT accepts a prepared Draft asset';
        END IF;
        RETURN;
    END IF;
    IF p_asset IS NULL OR jsonb_typeof(p_asset) IS DISTINCT FROM 'object'
       OR NOT p_asset ?& ARRAY['assetId','sourceObjectId','sourceObjectAddress','checksum','byteLength',
            'mediaType','createdAtMillis','publicObjectId','intrinsicWidth','intrinsicHeight','deliveryId','jobId']
       OR (p_asset - ARRAY['assetId','sourceObjectId','sourceObjectAddress','checksum','byteLength',
            'mediaType','createdAtMillis','publicObjectId','intrinsicWidth','intrinsicHeight','deliveryId','jobId']) <> '{}'
       OR (p_asset->>'checksum') IS NULL OR (p_asset->>'checksum') !~ '^[0-9a-f]{64}$' THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Native HOTSPOT requires complete prepared image facts';
    END IF;
    v_asset_id := (p_asset->>'assetId')::uuid; source_id := (p_asset->>'sourceObjectId')::uuid;
    public_id := (p_asset->>'publicObjectId')::uuid; v_delivery_id := (p_asset->>'deliveryId')::uuid;
    v_job_id := (p_asset->>'jobId')::uuid; width := (p_asset->>'intrinsicWidth')::integer;
    height := (p_asset->>'intrinsicHeight')::integer; checksum := decode(p_asset->>'checksum','hex');
    v_byte_length := (p_asset->>'byteLength')::bigint; v_media_type := p_asset->>'mediaType';
    created_at_millis := (p_asset->>'createdAtMillis')::bigint;
    IF v_asset_id IS NULL OR source_id IS NULL OR public_id IS NULL OR v_delivery_id IS NULL OR v_job_id IS NULL
       OR source_id = public_id OR width IS NULL OR height IS NULL OR width <= 0 OR height <= 0
       OR width::bigint * height > 20000000 OR v_byte_length IS NULL OR v_byte_length NOT BETWEEN 1 AND 8388608
       OR v_media_type IS NULL OR v_media_type NOT IN ('image/png','image/jpeg','image/webp')
       OR created_at_millis IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Prepared raster facts are invalid';
    END IF;
    -- ASVS 8.2.2, 8.3.1, 15.4.2: repeat owner and locked Draft checks, not caller checksum authority.
    PERFORM 1 FROM ple_private.draft_question AS draft
     WHERE draft.draft_question_uuid = p_draft_uuid AND draft.workspace_id = p_workspace_id
       AND ple_api.current_session_account_is_instructor()
       AND ple_private.current_session_is_authoring_workspace_owner(draft.workspace_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Prepared image requires current Draft ownership';
    END IF;
    SELECT * INTO draft_asset FROM ple_private.draft_question_asset AS asset
     WHERE asset.draft_question_uuid = p_draft_uuid AND asset.workspace_id = p_workspace_id
       AND asset.asset_id = v_asset_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Publication image is not owned by this Draft';
    END IF;
    SELECT * INTO STRICT record FROM ple_private.object_record WHERE object_id = draft_asset.source_object_id;
    expected_address := jsonb_build_object('kind','restrictedQuestionAsset',
        'questionRevision',jsonb_build_object('questionId',p_question_id,'revisionNumber',p_revision),
        'asset',v_asset_id,'object',source_id);
    IF p_asset->'sourceObjectAddress' IS DISTINCT FROM expected_address
       OR record.sha256 IS DISTINCT FROM checksum OR record.size_bytes IS DISTINCT FROM v_byte_length
       OR record.media_type IS DISTINCT FROM v_media_type
       OR draft_asset.intrinsic_width IS DISTINCT FROM width OR draft_asset.intrinsic_height IS DISTINCT FROM height THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Publication image must preserve exact Draft-owned raster evidence';
    END IF;
    -- ASVS 2.3.3: restricted record, Pending delivery, sole Job, and registry commit together.
    INSERT INTO ple_private.object_record(object_id,object_address,object_storage_area,object_data_class,
        sha256,size_bytes,media_type,created_at)
    VALUES(source_id,expected_address,'private-content','question-asset',checksum,v_byte_length,v_media_type,
        to_timestamp(created_at_millis::double precision / 1000.0));
    INSERT INTO ple_data.object_delivery(delivery_id,object_id,sha256,media_type,byte_length,delivery_state,registered_at)
    VALUES(v_delivery_id,public_id,checksum,v_media_type,v_byte_length,'pending',p_published_at);
    INSERT INTO ple_data.question_asset_delivery(delivery_id,object_id,question_id,revision_number,asset_id)
    VALUES(v_delivery_id,public_id,p_question_id,p_revision,v_asset_id);
    PERFORM ple_private.enqueue_public_asset_publication(v_job_id,p_question_id,p_revision,'{}',p_published_at,3,p_published_at);
    INSERT INTO ple_private.question_asset_publication(question_id,revision_number,asset_id,source_object_id,
        source_object_checksum,public_object_id,public_object_checksum,public_byte_length,verified_media_type,
        intrinsic_width,intrinsic_height,delivery_id,job_id,publication_state)
    VALUES(p_question_id,p_revision,v_asset_id,source_id,checksum,public_id,checksum,v_byte_length,v_media_type,
        width,height,v_delivery_id,v_job_id,'pending');
END $$;

