-- Functions, triggers, and views from draft_question_assets.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.validate_draft_question_asset()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    -- ASVS 2.2.2, 5.3.2: persisted facts must use this exact server-owned semantic address.
    IF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Draft Question Asset is immutable';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_private.object_record AS record
        WHERE record.object_record_id = NEW.source_object_record_id
          AND record.object_address = jsonb_build_object('kind', 'draftQuestionAsset',
              'workspace', NEW.authoring_workspace_id, 'draftQuestionUuid', NEW.draft_question_id,
              'asset', NEW.asset_id, 'object', NEW.source_object_record_id)
          AND record.object_storage_area = 'private-content'
          AND record.object_data_class = 'authoring-content'
          AND record.media_type IN ('image/png', 'image/jpeg', 'image/webp')
          AND record.size_bytes BETWEEN 1 AND 8388608) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Draft Question Asset requires exact immutable raster evidence';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER draft_question_asset_has_exact_immutable_evidence
BEFORE INSERT OR UPDATE ON ple_private.draft_question_asset
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_draft_question_asset();

CREATE FUNCTION ple_private.register_draft_question_asset(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_asset_id uuid,
    p_object_record_id uuid, p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint,
    p_media_type text, p_created_at_millis bigint, p_width integer, p_height integer
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE draft ple_private.draft_question%ROWTYPE; expected_address jsonb;
BEGIN
    IF p_draft_question_uuid IS NULL OR p_asset_id IS NULL OR p_object_record_id IS NULL OR p_sha256 IS NULL OR octet_length(p_sha256) <> 32
       OR p_expected_draft_question_edit_number IS NULL OR p_expected_draft_question_edit_number <= 0
       OR p_size_bytes IS NULL OR p_size_bytes NOT BETWEEN 1 AND 8388608
       OR p_media_type IS NULL OR p_media_type NOT IN ('image/png', 'image/jpeg', 'image/webp')
       OR p_created_at_millis IS NULL OR p_width IS NULL OR p_height IS NULL
       OR p_width <= 0 OR p_height <= 0 OR p_width::bigint * p_height > 20000000 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Draft Question Asset arguments are invalid';
    END IF;
    -- ASVS 8.2.1-8.2.2, 8.3.1, 15.4.2: current owner and ordinary CAS under the same row lock.
    SELECT * INTO draft FROM ple_private.draft_question AS question
     WHERE question.draft_question_id = p_draft_question_uuid
       AND ple_api.current_session_account_is_instructor()
       AND ple_private.current_session_is_authoring_workspace_owner(question.authoring_workspace_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Draft Question is not available in the current Authoring Workspace';
    END IF;
    IF draft.draft_question_edit_number <> p_expected_draft_question_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Draft Question Edit Number is stale';
    END IF;
    expected_address := jsonb_build_object('kind', 'draftQuestionAsset', 'workspace', draft.authoring_workspace_id,
        'draftQuestionUuid', draft.draft_question_id, 'asset', p_asset_id, 'object', p_object_record_id);
    IF p_object_address IS DISTINCT FROM expected_address THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Draft Question Asset address is not its exact owner';
    END IF;
    INSERT INTO ple_private.object_record(object_record_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at)
    VALUES (p_object_record_id, expected_address, 'private-content', 'authoring-content', p_sha256,
        p_size_bytes, p_media_type, to_timestamp(p_created_at_millis::double precision / 1000.0));
    INSERT INTO ple_private.draft_question_asset VALUES
        (draft.draft_question_id, draft.authoring_workspace_id, p_asset_id, p_object_record_id, p_width, p_height);
END $$;

CREATE FUNCTION ple_private.load_draft_question_asset(p_draft_question_uuid uuid, p_asset_id uuid)
RETURNS TABLE(object_record_id uuid, object_address jsonb, sha256 bytea, size_bytes bigint,
    media_type text, created_at_millis bigint, intrinsic_width integer, intrinsic_height integer)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT record.object_record_id, record.object_address, record.sha256, record.size_bytes, record.media_type,
        round(extract(epoch FROM record.created_at) * 1000)::bigint, asset.intrinsic_width, asset.intrinsic_height
      FROM ple_private.draft_question AS draft
      JOIN ple_private.draft_question_asset AS asset USING (draft_question_id, authoring_workspace_id)
      JOIN ple_private.object_record AS record ON record.object_record_id = asset.source_object_record_id
     WHERE draft.draft_question_id = p_draft_question_uuid AND asset.asset_id = p_asset_id
       AND ple_api.current_session_account_is_instructor()
       AND ple_private.current_session_is_authoring_workspace_owner(draft.authoring_workspace_id)
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.register_draft_question_asset(
    p_draft_question_uuid uuid, p_expected_draft_question_edit_number bigint, p_asset_id uuid,
    p_object_record_id uuid, p_object_address jsonb, p_sha256 bytea, p_size_bytes bigint,
    p_media_type text, p_created_at_millis bigint, p_width integer, p_height integer
) RETURNS void LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private AS $$
    SELECT ple_private.register_draft_question_asset(p_draft_question_uuid,p_expected_draft_question_edit_number,p_asset_id,
        p_object_record_id,p_object_address,p_sha256,p_size_bytes,p_media_type,p_created_at_millis,p_width,p_height)
$$;

CREATE FUNCTION ple_api.load_draft_question_asset(p_draft_question_uuid uuid, p_asset_id uuid)
RETURNS TABLE(object_record_id uuid, object_address jsonb, sha256 bytea, size_bytes bigint,
    media_type text, created_at_millis bigint, intrinsic_width integer, intrinsic_height integer)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_private AS $$
    SELECT * FROM ple_private.load_draft_question_asset(p_draft_question_uuid,p_asset_id)
$$;

