-- Functions, triggers, and views from profile_media.sql.

SET LOCAL ROLE ple_data_owner;

CREATE CONSTRAINT TRIGGER profile_image_delivery_preserves_available_owner
AFTER INSERT OR UPDATE OR DELETE ON ple_data.profile_image_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.require_exact_available_object_delivery_owner();

CREATE FUNCTION ple_data.validate_profile_image_delivery_object_record()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.object_delivery AS delivery
        JOIN ple_private.object_record AS record ON record.object_id = delivery.object_id
         WHERE delivery.delivery_id = NEW.delivery_id AND delivery.object_id = NEW.object_id
           AND record.object_address = jsonb_build_object(
               'kind', 'profileImage', 'image', NEW.profile_image_id,
               'object', NEW.object_id)
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = 'profile-image'
           AND record.sha256 = delivery.sha256 AND record.size_bytes = delivery.byte_length
           AND record.media_type = delivery.media_type
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Profile image delivery requires its exact Object Record';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER profile_image_delivery_has_exact_object_record
BEFORE INSERT OR UPDATE ON ple_data.profile_image_delivery
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_profile_image_delivery_object_record();

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.validate_account_avatar_profile_image()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.avatar_kind = 'profile-image' AND NOT EXISTS (
        SELECT 1 FROM ple_private.profile_image_work AS work
         WHERE work.account_id = NEW.account_id
           AND work.profile_image_id = NEW.profile_image_id
           AND work.delivery_id = NEW.profile_image_delivery_id
           AND work.operation_kind = 'put' AND work.state = 'finalized'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Profile image avatar requires the Account finalized image delivery';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_private.validate_account_avatar_provided_avatar()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NEW.avatar_kind = 'provided' AND NOT EXISTS (
        SELECT 1 FROM ple_data.provided_avatar AS avatar
         WHERE avatar.provided_avatar_id = NEW.provided_avatar_id
           AND avatar.is_selectable
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Provided avatar selection requires an active catalog asset';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER account_avatar_has_self_owned_profile_image
BEFORE INSERT OR UPDATE ON ple_private.account_avatar
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_account_avatar_profile_image();

CREATE TRIGGER account_avatar_has_selectable_provided_avatar
BEFORE INSERT OR UPDATE ON ple_private.account_avatar
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_account_avatar_provided_avatar();



-- ASVS 2.3.1/2.3.3: all Account creation paths persist the initial avatar
-- atomically; an unavailable gallery rolls back Account creation altogether.
CREATE FUNCTION ple_private.record_initial_account_avatar()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private, ple_data, pg_temp
AS $$
DECLARE v_provided_avatar_id text;
BEGIN
    SELECT avatar.provided_avatar_id INTO v_provided_avatar_id
      FROM ple_data.provided_avatar AS avatar
     WHERE avatar.is_selectable
     ORDER BY pg_catalog.random()
     LIMIT 1;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Account creation requires a selectable PLE gallery avatar';
    END IF;
    INSERT INTO ple_private.account_avatar (account_id, avatar_kind, provided_avatar_id)
    VALUES (NEW.account_id, 'provided', v_provided_avatar_id);
    RETURN NEW;
END
$$;

CREATE TRIGGER account_creation_records_initial_avatar
AFTER INSERT ON ple_private.account
FOR EACH ROW EXECUTE FUNCTION ple_private.record_initial_account_avatar();



-- C837's cross-Account projection is narrower than the self Avatar API: a
-- Sysadmin may learn only a selected provided-avatar ID. Generic and private
-- Profile-image choices both project as NULL, never as an image reference.
CREATE FUNCTION ple_private.list_instructor_account_avatar_summaries()
RETURNS TABLE (
    public_reference text,
    state text,
    last_successful_sign_in timestamp with time zone,
    provided_avatar_id text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    PERFORM ple_private.require_current_sysadmin_account();
    RETURN QUERY
    SELECT summary.public_reference,
           summary.state,
           summary.last_successful_sign_in,
           CASE WHEN avatar.avatar_kind = 'provided' THEN avatar.provided_avatar_id ELSE NULL END
      FROM ple_private.account AS account
      CROSS JOIN LATERAL ple_private.instructor_account_summary(account.account_id) AS summary
      LEFT JOIN ple_private.account_avatar AS avatar ON avatar.account_id = account.account_id
     WHERE account.product_role = 'instructor'
     ORDER BY summary.public_reference;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_instructor_account_avatar_summaries()
RETURNS TABLE (
    public_reference text,
    state text,
    last_successful_sign_in timestamp with time zone,
    provided_avatar_id text
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$ SELECT * FROM ple_private.list_instructor_account_avatar_summaries() $$;

CREATE FUNCTION ple_api.current_account_avatar()
RETURNS TABLE(avatar_kind text, provided_avatar_id text, profile_image_id uuid) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT avatar.avatar_kind, avatar.provided_avatar_id, avatar.profile_image_id
      FROM ple_private.account_avatar AS avatar
     WHERE avatar.account_id = ple_api.current_session_account_id()
$$;

CREATE FUNCTION ple_api.select_provided_account_avatar(p_provided_avatar_id text)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_account_id uuid := ple_api.current_session_account_id();
    old_delivery_id uuid; old_profile_image_id uuid; old_object_id uuid;
    completed_work ple_private.profile_image_work%ROWTYPE;
BEGIN
    IF v_account_id IS NULL OR NOT (
        ple_api.current_session_account_has_active_role('student')
        OR ple_api.current_session_account_is_instructor()
        OR ple_api.current_session_account_is_sysadmin()
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_data.provided_avatar
         WHERE provided_avatar_id = p_provided_avatar_id AND is_selectable
    ) THEN RETURN false; END IF;
    -- ASVS 2.3.1/2.3.3: serialize replacement with image finalization so a
    -- successful selection atomically removes self delivery and schedules its cleanup.
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'ple:profile-image-finalize:v1:' || v_account_id::text, 0));
    -- Lock completed work after the shared advisory lock. A later provided
    -- selection wins over an upload that has completed but not finalized.
    FOR completed_work IN
        SELECT * FROM ple_private.profile_image_work
         WHERE account_id = v_account_id AND operation_kind = 'put'
           AND state = 'completed'
         FOR UPDATE
    LOOP
        NULL;
    END LOOP;
    SELECT avatar.profile_image_delivery_id, avatar.profile_image_id
      INTO old_delivery_id, old_profile_image_id
      FROM ple_private.account_avatar AS avatar
     WHERE avatar.account_id = v_account_id AND avatar.avatar_kind = 'profile-image'
     FOR UPDATE;
    INSERT INTO ple_private.account_avatar (account_id, avatar_kind, provided_avatar_id)
    VALUES (v_account_id, 'provided', p_provided_avatar_id)
    ON CONFLICT (account_id) DO UPDATE SET avatar_kind = EXCLUDED.avatar_kind,
        provided_avatar_id = EXCLUDED.provided_avatar_id, profile_image_id = NULL,
        profile_image_delivery_id = NULL;
    IF old_delivery_id IS NOT NULL THEN
        UPDATE ple_data.object_delivery SET delivery_state = 'retired'
         WHERE delivery_id = old_delivery_id;
        SELECT object_id INTO old_object_id FROM ple_data.object_delivery
         WHERE delivery_id = old_delivery_id;
        INSERT INTO ple_private.profile_image_work
            (profile_image_work_id, account_id, profile_image_id, delivery_id,
             object_id, operation_kind, state, created_at)
        VALUES (gen_random_uuid(), v_account_id, old_profile_image_id,
            old_delivery_id, old_object_id, 'delete', 'pending', clock_timestamp())
        ON CONFLICT (profile_image_id, operation_kind) DO NOTHING;
    END IF;
    FOR completed_work IN
        SELECT * FROM ple_private.profile_image_work
         WHERE account_id = v_account_id AND operation_kind = 'put'
           AND state = 'completed'
    LOOP
        UPDATE ple_data.object_delivery SET delivery_state = 'retired'
         WHERE delivery_id = completed_work.delivery_id;
        INSERT INTO ple_private.profile_image_work
            (profile_image_work_id, account_id, profile_image_id, delivery_id,
             object_id, operation_kind, state, created_at)
        VALUES (gen_random_uuid(), v_account_id, completed_work.profile_image_id,
            completed_work.delivery_id, completed_work.object_id, 'delete',
            'pending', clock_timestamp())
        ON CONFLICT (profile_image_id, operation_kind) DO NOTHING;
    END LOOP;
    RETURN true;
END $$;

CREATE FUNCTION ple_api.prepare_account_profile_image(
    p_profile_image_id uuid, p_object_id uuid, p_sha256 bytea, p_byte_length bigint
) RETURNS TABLE(work_id uuid, profile_image_id uuid, object_id uuid) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_account_id uuid := ple_api.current_session_account_id(); v_delivery_id uuid := gen_random_uuid(); v_work_id uuid := gen_random_uuid();
    expected_address jsonb := jsonb_build_object(
        'kind', 'profileImage', 'image', p_profile_image_id, 'object', p_object_id);
BEGIN
    IF NOT (ple_api.current_session_account_is_instructor() OR ple_api.current_session_account_is_sysadmin())
       OR p_profile_image_id IS NULL OR p_object_id IS NULL OR p_sha256 IS NULL
       OR octet_length(p_sha256) <> 32 OR p_byte_length NOT BETWEEN 1 AND 2097152 THEN RETURN; END IF;
    INSERT INTO ple_private.object_record (object_id, object_address, object_storage_area, object_data_class, sha256, size_bytes, media_type, created_at)
    VALUES (p_object_id, expected_address, 'private-content', 'profile-image', p_sha256, p_byte_length, 'image/webp', clock_timestamp()) ON CONFLICT DO NOTHING;
    IF NOT EXISTS (SELECT 1 FROM ple_private.object_record AS record WHERE record.object_id = p_object_id
        AND record.object_address = expected_address AND record.object_storage_area = 'private-content' AND record.object_data_class = 'profile-image'
        AND record.sha256 = p_sha256 AND record.size_bytes = p_byte_length AND record.media_type = 'image/webp') THEN RETURN; END IF;
    INSERT INTO ple_data.object_delivery (delivery_id, object_id, sha256, media_type, byte_length, delivery_state, registered_at)
    VALUES (v_delivery_id, p_object_id, p_sha256, 'image/webp', p_byte_length, 'pending', clock_timestamp());
    INSERT INTO ple_data.profile_image_delivery VALUES (v_delivery_id, p_object_id, p_profile_image_id);
    INSERT INTO ple_private.profile_image_work (profile_image_work_id, account_id, profile_image_id, delivery_id, object_id, operation_kind, state, created_at)
    VALUES (v_work_id, v_account_id, p_profile_image_id, v_delivery_id, p_object_id, 'put', 'pending', clock_timestamp());
    work_id := v_work_id;
    profile_image_id := p_profile_image_id;
    object_id := p_object_id;
    RETURN NEXT;
EXCEPTION WHEN unique_violation OR foreign_key_violation OR check_violation THEN RETURN;
END $$;

CREATE FUNCTION ple_api.complete_account_profile_image_put(p_work_id uuid) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN UPDATE ple_private.profile_image_work SET state = 'completed', completed_at = clock_timestamp()
 WHERE profile_image_work_id = p_work_id AND operation_kind = 'put' AND state = 'pending' AND account_id = ple_api.current_session_account_id()
 AND (ple_api.current_session_account_is_instructor() OR ple_api.current_session_account_is_sysadmin()); RETURN FOUND; END $$;

CREATE FUNCTION ple_api.require_account_profile_image_repair(p_work_id uuid) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN UPDATE ple_private.profile_image_work SET state = 'repair-required'
 WHERE profile_image_work_id = p_work_id AND state = 'pending' AND account_id = ple_api.current_session_account_id()
 AND (ple_api.current_session_account_is_instructor() OR ple_api.current_session_account_is_sysadmin()); RETURN FOUND; END $$;

CREATE FUNCTION ple_api.prepare_account_profile_image_deletion(p_put_work_id uuid)
RETURNS TABLE(delete_work_id uuid, profile_image_id uuid, object_id uuid) LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE put_work ple_private.profile_image_work%ROWTYPE;
BEGIN
 SELECT * INTO put_work FROM ple_private.profile_image_work WHERE profile_image_work_id = p_put_work_id AND operation_kind = 'put' AND state = 'completed'
  AND account_id = ple_api.current_session_account_id() FOR UPDATE;
 IF NOT FOUND OR NOT (ple_api.current_session_account_is_instructor() OR ple_api.current_session_account_is_sysadmin()) THEN RETURN; END IF;
 SELECT existing.profile_image_work_id, existing.profile_image_id, existing.object_id
   INTO delete_work_id, profile_image_id, object_id FROM ple_private.profile_image_work existing
  WHERE existing.profile_image_id = put_work.profile_image_id AND existing.operation_kind = 'delete'; IF FOUND THEN RETURN NEXT; RETURN; END IF;
 delete_work_id := gen_random_uuid(); profile_image_id := put_work.profile_image_id; object_id := put_work.object_id;
 INSERT INTO ple_private.profile_image_work (profile_image_work_id, account_id, profile_image_id, delivery_id, object_id, operation_kind, state, created_at)
 VALUES (delete_work_id, put_work.account_id, put_work.profile_image_id, put_work.delivery_id, put_work.object_id, 'delete', 'pending', clock_timestamp()); RETURN NEXT;
EXCEPTION WHEN unique_violation THEN SELECT existing.profile_image_work_id, existing.profile_image_id, existing.object_id INTO delete_work_id, profile_image_id, object_id FROM ple_private.profile_image_work existing WHERE existing.profile_image_id = put_work.profile_image_id AND existing.operation_kind = 'delete'; IF FOUND THEN RETURN NEXT; END IF;
END $$;

CREATE FUNCTION ple_api.complete_account_profile_image_deletion(p_delete_work_id uuid) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN UPDATE ple_private.profile_image_work SET state = 'completed', completed_at = clock_timestamp() WHERE profile_image_work_id = p_delete_work_id AND operation_kind = 'delete' AND state = 'pending' AND account_id = ple_api.current_session_account_id() AND (ple_api.current_session_account_is_instructor() OR ple_api.current_session_account_is_sysadmin()); RETURN FOUND; END $$;

CREATE FUNCTION ple_api.require_account_profile_image_deletion_repair(p_delete_work_id uuid) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN UPDATE ple_private.profile_image_work SET state = 'repair-required' WHERE profile_image_work_id = p_delete_work_id AND operation_kind = 'delete' AND state = 'pending' AND account_id = ple_api.current_session_account_id() AND (ple_api.current_session_account_is_instructor() OR ple_api.current_session_account_is_sysadmin()); RETURN FOUND; END $$;

CREATE FUNCTION ple_api.record_account_profile_image_cleanup_check(p_delete_work_id uuid, p_object_present boolean, p_observed_checksum bytea)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE work ple_private.profile_image_work%ROWTYPE; expected bytea; check_id uuid := gen_random_uuid(); manifest_id uuid := gen_random_uuid(); result text; disposition text;
BEGIN
 SELECT * INTO work FROM ple_private.profile_image_work WHERE profile_image_work_id = p_delete_work_id AND operation_kind = 'delete' AND state = 'repair-required' FOR UPDATE;
 IF NOT FOUND OR work.account_id <> ple_api.current_session_account_id() OR NOT (ple_api.current_session_account_is_instructor() OR ple_api.current_session_account_is_sysadmin()) THEN RETURN false; END IF;
 SELECT sha256 INTO expected FROM ple_data.object_delivery WHERE delivery_id = work.delivery_id;
 IF NOT p_object_present AND p_observed_checksum IS NULL THEN result := 'missing'; ELSIF p_object_present AND p_observed_checksum = expected THEN result := 'verified'; ELSIF p_object_present AND octet_length(p_observed_checksum) = 32 THEN result := 'mismatched'; ELSE RETURN false; END IF;
 INSERT INTO ple_private.object_storage_check (object_storage_check_id, delivery_id, expected_sha256, check_result, checked_at) VALUES (check_id, work.delivery_id, expected, result, clock_timestamp());
 disposition := CASE result WHEN 'missing' THEN 'already_absent' ELSE 'retained' END; INSERT INTO ple_private.object_cleanup_manifest VALUES (manifest_id, check_id, clock_timestamp(), disposition);
 INSERT INTO ple_audit.object_storage_check_event VALUES (gen_random_uuid(), check_id, result, clock_timestamp(), sha256(convert_to('ple:profile-image-storage-check-event:v1', 'UTF8') || uuid_send(check_id) || convert_to(result, 'UTF8') || expected));
 INSERT INTO ple_audit.object_cleanup_receipt VALUES (gen_random_uuid(), manifest_id, disposition, clock_timestamp());
 UPDATE ple_private.profile_image_work SET state = 'completed', completed_at = clock_timestamp() WHERE profile_image_work_id = p_delete_work_id; RETURN true;
END $$;

CREATE FUNCTION ple_api.finalize_account_profile_image(p_work_id uuid)
RETURNS TABLE(profile_image_id uuid, object_id uuid, retired_delete_work_id uuid, retired_profile_image_id uuid, retired_object_id uuid) LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE work ple_private.profile_image_work%ROWTYPE; old_delivery uuid; old_image uuid; old_object uuid; expected_address jsonb;
BEGIN
 PERFORM pg_advisory_xact_lock(hashtextextended('ple:profile-image-finalize:v1:' || ple_api.current_session_account_id()::text, 0));
 SELECT * INTO work FROM ple_private.profile_image_work WHERE profile_image_work_id = p_work_id AND operation_kind = 'put' AND state = 'completed' AND account_id = ple_api.current_session_account_id() FOR UPDATE;
 IF NOT FOUND OR NOT (ple_api.current_session_account_is_instructor() OR ple_api.current_session_account_is_sysadmin()) OR EXISTS (SELECT 1 FROM ple_private.profile_image_work existing WHERE existing.profile_image_id = work.profile_image_id AND existing.operation_kind = 'delete') THEN RETURN; END IF;
 expected_address := jsonb_build_object(
     'kind', 'profileImage', 'image', work.profile_image_id, 'object', work.object_id);
 IF NOT EXISTS (SELECT 1 FROM ple_data.object_delivery delivery JOIN ple_private.object_record record ON record.object_id = delivery.object_id JOIN ple_data.profile_image_delivery owner ON owner.delivery_id = delivery.delivery_id AND owner.object_id = delivery.object_id AND owner.profile_image_id = work.profile_image_id WHERE delivery.delivery_id = work.delivery_id AND delivery.object_id = work.object_id AND record.object_address = expected_address AND record.object_storage_area = 'private-content' AND record.object_data_class = 'profile-image' AND record.sha256 = delivery.sha256 AND record.size_bytes = delivery.byte_length AND record.media_type = delivery.media_type) THEN RETURN; END IF;
 SELECT avatar.profile_image_delivery_id, avatar.profile_image_id INTO old_delivery, old_image FROM ple_private.account_avatar avatar WHERE avatar.account_id = work.account_id AND avatar.avatar_kind = 'profile-image' FOR UPDATE;
 UPDATE ple_private.profile_image_work SET state = 'finalized', completed_at = clock_timestamp() WHERE profile_image_work_id = work.profile_image_work_id;
 INSERT INTO ple_private.account_avatar (account_id, avatar_kind, profile_image_id, profile_image_delivery_id) VALUES (work.account_id, 'profile-image', work.profile_image_id, work.delivery_id) ON CONFLICT (account_id) DO UPDATE SET avatar_kind = EXCLUDED.avatar_kind, provided_avatar_id = NULL, profile_image_id = EXCLUDED.profile_image_id, profile_image_delivery_id = EXCLUDED.profile_image_delivery_id;
 UPDATE ple_data.object_delivery SET delivery_state = 'available' WHERE delivery_id = work.delivery_id;
 IF old_delivery IS NOT NULL THEN UPDATE ple_data.object_delivery SET delivery_state = 'retired' WHERE delivery_id = old_delivery; SELECT object_id INTO old_object FROM ple_data.object_delivery WHERE delivery_id = old_delivery; retired_delete_work_id := gen_random_uuid(); retired_profile_image_id := old_image; retired_object_id := old_object; INSERT INTO ple_private.profile_image_work (profile_image_work_id, account_id, profile_image_id, delivery_id, object_id, operation_kind, state, created_at) VALUES (retired_delete_work_id, work.account_id, old_image, old_delivery, old_object, 'delete', 'pending', clock_timestamp()); END IF;
 profile_image_id := work.profile_image_id; object_id := work.object_id; RETURN NEXT;
END $$;

CREATE FUNCTION ple_api.resolve_current_account_profile_image(p_profile_image_id uuid) RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
 SELECT delivery.object_id FROM ple_private.account_avatar avatar JOIN ple_data.object_delivery delivery ON delivery.delivery_id = avatar.profile_image_delivery_id WHERE avatar.profile_image_id = p_profile_image_id AND avatar.account_id = ple_api.current_session_account_id() AND avatar.avatar_kind = 'profile-image' AND (ple_api.current_session_account_is_instructor() OR ple_api.current_session_account_is_sysadmin()) AND delivery.delivery_state = 'available'
$$;

