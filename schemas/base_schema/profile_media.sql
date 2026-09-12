-- Instructor profile-thumbnail storage saga.  A thumbnail is private to its
-- instructor; only the service records completed external object operations.

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.profile_thumbnail_delivery (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL REFERENCES ple_private.object_record,
    profile_thumbnail_id uuid NOT NULL UNIQUE,
    UNIQUE (delivery_id, object_id),
    FOREIGN KEY (delivery_id, object_id) REFERENCES ple_data.object_delivery (delivery_id, object_id)
);
CREATE CONSTRAINT TRIGGER profile_thumbnail_delivery_preserves_available_owner
AFTER INSERT OR UPDATE OR DELETE ON ple_data.profile_thumbnail_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.require_exact_available_object_delivery_owner();
CREATE FUNCTION ple_data.validate_profile_thumbnail_delivery_object_record()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.object_delivery AS delivery
        JOIN ple_private.object_record AS record ON record.object_id = delivery.object_id
         WHERE delivery.delivery_id = NEW.delivery_id AND delivery.object_id = NEW.object_id
           AND record.object_address = jsonb_build_object(
               'kind', 'profileThumbnail', 'thumbnail', NEW.profile_thumbnail_id)
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = 'profile-thumbnail'
           AND record.sha256 = delivery.sha256 AND record.size_bytes = delivery.byte_length
           AND record.media_type = delivery.media_type
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Profile Thumbnail delivery requires its exact Object Record';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER profile_thumbnail_delivery_has_exact_object_record
BEFORE INSERT OR UPDATE ON ple_data.profile_thumbnail_delivery
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_profile_thumbnail_delivery_object_record();
ALTER TABLE ple_data.profile_thumbnail_delivery ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.profile_thumbnail_delivery FORCE ROW LEVEL SECURITY;
REVOKE ALL ON ple_data.profile_thumbnail_delivery FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.validate_profile_thumbnail_delivery_object_record() FROM PUBLIC;
CREATE POLICY profile_thumbnail_delivery_data_owner_access ON ple_data.profile_thumbnail_delivery
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
GRANT REFERENCES (delivery_id, object_id) ON ple_data.profile_thumbnail_delivery
    TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.instructor_profile_thumbnail (
    account_id uuid PRIMARY KEY REFERENCES ple_private.account,
    profile_thumbnail_id uuid NOT NULL UNIQUE,
    delivery_id uuid NOT NULL UNIQUE REFERENCES ple_data.profile_thumbnail_delivery
);
CREATE TABLE ple_private.profile_thumbnail_work (
    profile_thumbnail_work_id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES ple_private.account,
    profile_thumbnail_id uuid NOT NULL,
    delivery_id uuid NOT NULL,
    object_id uuid NOT NULL REFERENCES ple_private.object_record,
    operation_kind text NOT NULL CHECK (operation_kind IN ('put', 'delete')),
    state text NOT NULL CHECK (state IN ('pending', 'completed', 'repair-required', 'finalized')),
    created_at timestamptz NOT NULL,
    completed_at timestamptz,
    UNIQUE (profile_thumbnail_id, operation_kind),
    FOREIGN KEY (delivery_id, object_id)
        REFERENCES ple_data.profile_thumbnail_delivery (delivery_id, object_id)
);
ALTER TABLE ple_private.instructor_profile_thumbnail ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.instructor_profile_thumbnail FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.profile_thumbnail_work ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.profile_thumbnail_work FORCE ROW LEVEL SECURITY;
REVOKE ALL ON ple_private.instructor_profile_thumbnail, ple_private.profile_thumbnail_work FROM PUBLIC;
CREATE POLICY instructor_profile_thumbnail_private_owner_access ON ple_private.instructor_profile_thumbnail
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY profile_thumbnail_work_private_owner_access ON ple_private.profile_thumbnail_work
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_data_owner;
GRANT SELECT, INSERT, UPDATE ON ple_data.object_delivery, ple_data.profile_thumbnail_delivery TO ple_api_owner;
CREATE POLICY object_delivery_api_owner_profile_media ON ple_data.object_delivery
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY profile_thumbnail_delivery_api_owner_profile_media ON ple_data.profile_thumbnail_delivery
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
SET LOCAL ROLE ple_private_owner;
GRANT SELECT, INSERT, UPDATE ON ple_private.instructor_profile_thumbnail,
    ple_private.profile_thumbnail_work, ple_private.object_storage_check,
    ple_private.object_cleanup_manifest TO ple_api_owner;
CREATE POLICY instructor_thumbnail_api_owner_profile_media ON ple_private.instructor_profile_thumbnail
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY thumbnail_work_api_owner_profile_media ON ple_private.profile_thumbnail_work
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY storage_check_api_owner_profile_media ON ple_private.object_storage_check
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY cleanup_manifest_api_owner_profile_media ON ple_private.object_cleanup_manifest
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
SET LOCAL ROLE ple_audit_owner;
GRANT INSERT ON ple_audit.object_storage_check_event, ple_audit.object_cleanup_receipt TO ple_api_owner;
CREATE POLICY storage_check_event_api_owner_profile_media ON ple_audit.object_storage_check_event
    FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY cleanup_receipt_api_owner_profile_media ON ple_audit.object_cleanup_receipt
    FOR INSERT TO ple_api_owner WITH CHECK (true);
SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.current_instructor_profile_thumbnail()
RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT thumbnail.profile_thumbnail_id
      FROM ple_private.instructor_profile_thumbnail thumbnail
      JOIN ple_private.account account ON account.account_id = thumbnail.account_id
     WHERE thumbnail.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'instructor'
       AND ple_api.current_session_account_is_instructor()
$$;
CREATE FUNCTION ple_api.prepare_instructor_profile_thumbnail(
    p_thumbnail_id uuid, p_object_id uuid, p_sha256 bytea, p_byte_length bigint
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_account_id uuid; v_delivery_id uuid := gen_random_uuid(); v_work_id uuid := gen_random_uuid();
    expected_address jsonb := jsonb_build_object('kind', 'profileThumbnail', 'thumbnail', p_thumbnail_id);
BEGIN
    v_account_id := ple_api.current_session_account_id();
    IF NOT ple_api.current_session_account_is_instructor()
       OR NOT EXISTS (
           SELECT 1 FROM ple_private.account AS account
            WHERE account.account_id = v_account_id AND account.product_role = 'instructor'
       )
       OR p_sha256 IS NULL OR octet_length(p_sha256) <> 32 OR p_byte_length NOT BETWEEN 1 AND 2097152 THEN
        RETURN NULL;
    END IF;
    INSERT INTO ple_private.object_record
        (object_id, object_address, object_storage_area, object_data_class,
         sha256, size_bytes, media_type, created_at)
    VALUES (p_object_id, expected_address, 'private-content', 'profile-thumbnail',
        p_sha256, p_byte_length, 'image/webp', clock_timestamp()) ON CONFLICT DO NOTHING;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.object_record WHERE object_id = p_object_id
          AND object_address = expected_address AND object_storage_area = 'private-content'
          AND object_data_class = 'profile-thumbnail' AND sha256 = p_sha256
          AND size_bytes = p_byte_length AND media_type = 'image/webp'
    ) THEN
        RETURN NULL;
    END IF;
    INSERT INTO ple_data.object_delivery
        (delivery_id, object_id, sha256, media_type, byte_length, delivery_state, registered_at)
    VALUES (v_delivery_id, p_object_id, p_sha256, 'image/webp', p_byte_length, 'pending', clock_timestamp());
    INSERT INTO ple_data.profile_thumbnail_delivery VALUES (v_delivery_id, p_object_id, p_thumbnail_id);
    INSERT INTO ple_private.profile_thumbnail_work
        (profile_thumbnail_work_id, account_id, profile_thumbnail_id, delivery_id, object_id, operation_kind, state, created_at)
    VALUES (v_work_id, v_account_id, p_thumbnail_id, v_delivery_id, p_object_id, 'put', 'pending', clock_timestamp());
    RETURN v_work_id;
EXCEPTION WHEN unique_violation OR foreign_key_violation OR check_violation THEN RETURN NULL;
END $$;
CREATE FUNCTION ple_api.complete_instructor_profile_thumbnail_put(p_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.profile_thumbnail_work SET state = 'completed', completed_at = clock_timestamp()
     WHERE profile_thumbnail_work_id = p_work_id AND operation_kind = 'put' AND state = 'pending'
       AND account_id = ple_api.current_session_account_id() AND ple_api.current_session_account_is_instructor();
    RETURN FOUND;
END $$;
CREATE FUNCTION ple_api.require_instructor_profile_thumbnail_repair(p_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.profile_thumbnail_work SET state = 'repair-required'
     WHERE profile_thumbnail_work_id = p_work_id AND state = 'pending'
       AND account_id = ple_api.current_session_account_id() AND ple_api.current_session_account_is_instructor();
    RETURN FOUND;
END $$;
CREATE FUNCTION ple_api.prepare_instructor_profile_thumbnail_deletion(p_put_work_id uuid)
RETURNS TABLE(delete_work_id uuid, profile_thumbnail_id uuid) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE put_work ple_private.profile_thumbnail_work%ROWTYPE;
BEGIN
    SELECT * INTO put_work FROM ple_private.profile_thumbnail_work
     WHERE profile_thumbnail_work_id = p_put_work_id AND operation_kind = 'put' AND state = 'completed'
       AND account_id = ple_api.current_session_account_id() FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_instructor() THEN RETURN; END IF;
    SELECT existing.profile_thumbnail_work_id, existing.profile_thumbnail_id
      INTO delete_work_id, profile_thumbnail_id
      FROM ple_private.profile_thumbnail_work AS existing
     WHERE existing.profile_thumbnail_id = put_work.profile_thumbnail_id
       AND existing.operation_kind = 'delete';
    IF FOUND THEN RETURN NEXT; RETURN; END IF;
    delete_work_id := gen_random_uuid(); profile_thumbnail_id := put_work.profile_thumbnail_id;
    INSERT INTO ple_private.profile_thumbnail_work
        (profile_thumbnail_work_id, account_id, profile_thumbnail_id, delivery_id, object_id, operation_kind, state, created_at)
    VALUES (delete_work_id, put_work.account_id, put_work.profile_thumbnail_id, put_work.delivery_id,
        put_work.object_id, 'delete', 'pending', clock_timestamp());
    RETURN NEXT;
EXCEPTION WHEN unique_violation THEN
    SELECT existing.profile_thumbnail_work_id, existing.profile_thumbnail_id
      INTO delete_work_id, profile_thumbnail_id
      FROM ple_private.profile_thumbnail_work AS existing
     WHERE existing.profile_thumbnail_id = put_work.profile_thumbnail_id
       AND existing.operation_kind = 'delete';
    IF FOUND THEN RETURN NEXT; END IF;
END $$;
CREATE FUNCTION ple_api.complete_instructor_profile_thumbnail_deletion(p_delete_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.profile_thumbnail_work SET state = 'completed', completed_at = clock_timestamp()
     WHERE profile_thumbnail_work_id = p_delete_work_id AND operation_kind = 'delete' AND state = 'pending'
       AND account_id = ple_api.current_session_account_id() AND ple_api.current_session_account_is_instructor();
    RETURN FOUND;
END $$;
CREATE FUNCTION ple_api.require_instructor_profile_thumbnail_deletion_repair(p_delete_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.profile_thumbnail_work SET state = 'repair-required'
     WHERE profile_thumbnail_work_id = p_delete_work_id AND operation_kind = 'delete' AND state = 'pending'
       AND account_id = ple_api.current_session_account_id() AND ple_api.current_session_account_is_instructor();
    RETURN FOUND;
END $$;
CREATE FUNCTION ple_api.record_instructor_profile_thumbnail_cleanup_check(
    p_delete_work_id uuid, p_object_present boolean, p_observed_checksum bytea
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE work ple_private.profile_thumbnail_work%ROWTYPE; expected bytea; check_id uuid := gen_random_uuid();
    manifest_id uuid := gen_random_uuid(); result text; disposition text;
BEGIN
    SELECT * INTO work FROM ple_private.profile_thumbnail_work WHERE profile_thumbnail_work_id = p_delete_work_id
       AND operation_kind = 'delete' AND state = 'repair-required' FOR UPDATE;
    IF NOT FOUND OR work.account_id <> ple_api.current_session_account_id()
       OR NOT ple_api.current_session_account_is_instructor() THEN RETURN false; END IF;
    SELECT sha256 INTO expected FROM ple_data.object_delivery WHERE delivery_id = work.delivery_id;
    IF NOT p_object_present AND p_observed_checksum IS NULL THEN result := 'missing';
    ELSIF p_object_present AND p_observed_checksum = expected THEN result := 'verified';
    ELSIF p_object_present AND octet_length(p_observed_checksum) = 32 THEN result := 'mismatched';
    ELSE RETURN false; END IF;
    INSERT INTO ple_private.object_storage_check
        (object_storage_check_id, delivery_id, expected_sha256, check_result, checked_at)
    VALUES (check_id, work.delivery_id, expected, result, clock_timestamp());
    disposition := CASE result WHEN 'missing' THEN 'already_absent' ELSE 'retained' END;
    INSERT INTO ple_private.object_cleanup_manifest VALUES (manifest_id, check_id, clock_timestamp(), disposition);
    INSERT INTO ple_audit.object_storage_check_event VALUES (gen_random_uuid(), check_id, result, clock_timestamp(),
      sha256(convert_to('ple:profile-thumbnail-storage-check-event:v1', 'UTF8') || uuid_send(check_id) || convert_to(result, 'UTF8') || expected));
    INSERT INTO ple_audit.object_cleanup_receipt VALUES (gen_random_uuid(), manifest_id, disposition, clock_timestamp());
    UPDATE ple_private.profile_thumbnail_work SET state = 'completed', completed_at = clock_timestamp()
     WHERE profile_thumbnail_work_id = p_delete_work_id;
    RETURN true;
END $$;
CREATE FUNCTION ple_api.finalize_instructor_profile_thumbnail(p_work_id uuid)
RETURNS TABLE(profile_thumbnail_id uuid, retired_delete_work_id uuid, retired_profile_thumbnail_id uuid)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE work ple_private.profile_thumbnail_work%ROWTYPE; old_delivery uuid; old_thumbnail uuid; old_object uuid;
    expected_address jsonb;
BEGIN
    SELECT * INTO work FROM ple_private.profile_thumbnail_work WHERE profile_thumbnail_work_id = p_work_id
      AND operation_kind = 'put' AND state = 'completed' AND account_id = ple_api.current_session_account_id() FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_instructor() THEN RETURN; END IF;
    IF EXISTS (
        SELECT 1 FROM ple_private.profile_thumbnail_work AS existing
         WHERE existing.profile_thumbnail_id = work.profile_thumbnail_id
           AND existing.operation_kind = 'delete'
    ) THEN RETURN; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended('ple:profile-thumbnail-finalize:v1:' || work.account_id::text, 0));
    expected_address := jsonb_build_object('kind', 'profileThumbnail', 'thumbnail', work.profile_thumbnail_id);
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.object_delivery AS delivery
        JOIN ple_private.object_record AS record ON record.object_id = delivery.object_id
        JOIN ple_data.profile_thumbnail_delivery AS owner ON owner.delivery_id = delivery.delivery_id
         AND owner.object_id = delivery.object_id AND owner.profile_thumbnail_id = work.profile_thumbnail_id
        WHERE delivery.delivery_id = work.delivery_id AND delivery.object_id = work.object_id
          AND record.object_address = expected_address AND record.object_storage_area = 'private-content'
          AND record.object_data_class = 'profile-thumbnail' AND record.sha256 = delivery.sha256
          AND record.size_bytes = delivery.byte_length AND record.media_type = delivery.media_type
    ) THEN
        RETURN;
    END IF;
    SELECT current_thumbnail.delivery_id, current_thumbnail.profile_thumbnail_id
      INTO old_delivery, old_thumbnail
      FROM ple_private.instructor_profile_thumbnail AS current_thumbnail
     WHERE current_thumbnail.account_id = work.account_id FOR UPDATE;
    INSERT INTO ple_private.instructor_profile_thumbnail VALUES (work.account_id, work.profile_thumbnail_id, work.delivery_id)
    ON CONFLICT (account_id) DO UPDATE SET profile_thumbnail_id = EXCLUDED.profile_thumbnail_id, delivery_id = EXCLUDED.delivery_id;
    UPDATE ple_data.object_delivery SET delivery_state = 'available' WHERE delivery_id = work.delivery_id;
    IF old_delivery IS NOT NULL THEN
        UPDATE ple_data.object_delivery SET delivery_state = 'retired' WHERE delivery_id = old_delivery;
        SELECT object_id INTO old_object FROM ple_data.object_delivery WHERE delivery_id = old_delivery;
        retired_delete_work_id := gen_random_uuid(); retired_profile_thumbnail_id := old_thumbnail;
        INSERT INTO ple_private.profile_thumbnail_work
            (profile_thumbnail_work_id, account_id, profile_thumbnail_id, delivery_id, object_id, operation_kind, state, created_at)
        VALUES (retired_delete_work_id, work.account_id, old_thumbnail, old_delivery, old_object, 'delete', 'pending', clock_timestamp());
    END IF;
    UPDATE ple_private.profile_thumbnail_work SET state = 'finalized', completed_at = clock_timestamp() WHERE profile_thumbnail_work_id = work.profile_thumbnail_work_id;
    profile_thumbnail_id := work.profile_thumbnail_id; RETURN NEXT;
END $$;
CREATE FUNCTION ple_api.resolve_current_instructor_profile_thumbnail(p_thumbnail_id uuid)
RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT delivery.object_id FROM ple_private.instructor_profile_thumbnail current_thumbnail
      JOIN ple_data.object_delivery delivery ON delivery.delivery_id = current_thumbnail.delivery_id
      JOIN ple_private.account account ON account.account_id = current_thumbnail.account_id
     WHERE current_thumbnail.profile_thumbnail_id = p_thumbnail_id
       AND current_thumbnail.account_id = ple_api.current_session_account_id() AND account.product_role = 'instructor'
       AND ple_api.current_session_account_is_instructor() AND delivery.delivery_state = 'available'
$$;
REVOKE ALL ON FUNCTION ple_api.current_instructor_profile_thumbnail(),
    ple_api.prepare_instructor_profile_thumbnail(uuid,uuid,bytea,bigint), ple_api.complete_instructor_profile_thumbnail_put(uuid),
    ple_api.require_instructor_profile_thumbnail_repair(uuid), ple_api.prepare_instructor_profile_thumbnail_deletion(uuid),
    ple_api.complete_instructor_profile_thumbnail_deletion(uuid), ple_api.require_instructor_profile_thumbnail_deletion_repair(uuid),
    ple_api.record_instructor_profile_thumbnail_cleanup_check(uuid,boolean,bytea), ple_api.finalize_instructor_profile_thumbnail(uuid),
    ple_api.resolve_current_instructor_profile_thumbnail(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.current_instructor_profile_thumbnail(),
    ple_api.prepare_instructor_profile_thumbnail(uuid,uuid,bytea,bigint), ple_api.complete_instructor_profile_thumbnail_put(uuid),
    ple_api.require_instructor_profile_thumbnail_repair(uuid), ple_api.prepare_instructor_profile_thumbnail_deletion(uuid),
    ple_api.complete_instructor_profile_thumbnail_deletion(uuid), ple_api.require_instructor_profile_thumbnail_deletion_repair(uuid),
    ple_api.record_instructor_profile_thumbnail_cleanup_check(uuid,boolean,bytea), ple_api.finalize_instructor_profile_thumbnail(uuid),
    ple_api.resolve_current_instructor_profile_thumbnail(uuid) TO ple_app;

RESET ROLE;
