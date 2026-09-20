-- Functions, triggers, and views from object_records.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.reject_object_record_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Object Records are immutable';
END
$$;

CREATE TRIGGER object_record_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.object_record
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_object_record_change();



-- The source-binding tables are authored by question-authoring state.  Keep the
-- exact Object Record FKs there, after both module families have been loaded.
CREATE FUNCTION ple_private.register_workspace_question_source_object(
    p_authoring_workspace_id uuid, p_object_record_id uuid, p_object_address jsonb,
    p_sha256 bytea, p_size_bytes bigint, p_media_type text, p_created_at_millis bigint
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    expected_address jsonb := jsonb_build_object(
        'kind', 'workspaceQuestionSource', 'workspaceId', p_authoring_workspace_id,
        'objectId', p_object_record_id);
    expected_created_at timestamptz := to_timestamp(p_created_at_millis::double precision / 1000.0);
BEGIN
    IF NOT ple_api.current_session_account_can_access_authoring_workspace(p_authoring_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Workspace Question Source Object registration requires current Authoring Workspace access';
    END IF;
    IF jsonb_typeof(p_object_address) <> 'object' OR p_object_address <> expected_address THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Workspace Question Source Object registration requires its exact typed Object Address';
    END IF;
    INSERT INTO ple_private.object_record (
        object_record_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        p_object_record_id, expected_address, 'private-content', 'authoring-content',
        p_sha256, p_size_bytes, p_media_type, expected_created_at
    ) ON CONFLICT DO NOTHING;
    IF FOUND OR EXISTS (
        SELECT 1 FROM ple_private.object_record AS record
         WHERE record.object_record_id = p_object_record_id AND record.object_address = expected_address
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = 'authoring-content'
           AND record.sha256 = p_sha256 AND record.size_bytes = p_size_bytes
           AND record.media_type = p_media_type AND record.created_at = expected_created_at
    ) THEN
        RETURN;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '23505',
        MESSAGE = 'Object Record identity or address already names different immutable bytes';
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.register_workspace_question_source_object(
    p_authoring_workspace_id uuid, p_object_record_id uuid, p_object_address jsonb,
    p_sha256 bytea, p_size_bytes bigint, p_media_type text, p_created_at_millis bigint
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.register_workspace_question_source_object(
        p_authoring_workspace_id, p_object_record_id, p_object_address, p_sha256, p_size_bytes,
        p_media_type, p_created_at_millis)
$$;

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.require_exact_available_object_delivery_owner()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE target_delivery_id uuid := COALESCE(NEW.object_delivery_id, OLD.object_delivery_id); owner_count integer;
BEGIN
    IF (SELECT delivery_state FROM ple_data.object_delivery WHERE object_delivery_id = target_delivery_id) = 'available' THEN
        SELECT (SELECT count(*) FROM ple_data.question_image_delivery WHERE object_delivery_id = target_delivery_id)
             + (SELECT count(*) FROM ple_data.course_banner_delivery WHERE object_delivery_id = target_delivery_id)
             + (SELECT count(*) FROM ple_data.course_object_delivery WHERE object_delivery_id = target_delivery_id)
             + (SELECT count(*) FROM ple_data.profile_image_delivery WHERE object_delivery_id = target_delivery_id)
          INTO owner_count;
        IF owner_count <> 1 THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'an available Object Delivery requires exactly one owner relationship';
        END IF;
    END IF;
    RETURN NULL;
END
$$;

CREATE CONSTRAINT TRIGGER object_delivery_has_exact_available_owner
AFTER INSERT OR UPDATE OR DELETE ON ple_data.object_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.require_exact_available_object_delivery_owner();

