-- Functions, triggers, and views from course_media.sql.

SET LOCAL ROLE ple_data_owner;

CREATE CONSTRAINT TRIGGER course_banner_delivery_preserves_available_owner
AFTER INSERT OR UPDATE OR DELETE ON ple_data.course_banner_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.require_exact_available_object_delivery_owner();

CREATE FUNCTION ple_data.validate_course_banner_source_object_record()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.object_record AS record
         WHERE record.object_record_id = NEW.source_object_record_id
           AND record.object_address = jsonb_build_object(
               'kind', 'courseBannerSource', 'courseInstanceId', NEW.course_instance_id, 'banner', NEW.course_banner_id)
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = 'course-appearance'
           AND record.sha256 = NEW.source_object_checksum
           AND record.size_bytes = NEW.source_byte_length
           AND record.media_type = NEW.source_media_type
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Course Banner source requires its exact Object Record';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_data.validate_course_banner_rendition_object_record()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.object_record AS record
         WHERE record.object_record_id = NEW.object_record_id
           AND record.object_address = jsonb_build_object(
               'kind', 'courseBannerRendition', 'courseInstanceId', NEW.course_instance_id,
               'banner', NEW.course_banner_id, 'rendition', NEW.rendition_kind)
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = 'course-appearance'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Course Banner rendition requires its exact Object Record';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_data.validate_course_banner_delivery_object_record()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.object_delivery AS delivery
        JOIN ple_private.object_record AS record ON record.object_record_id = delivery.object_record_id
         WHERE delivery.object_delivery_id = NEW.object_delivery_id AND delivery.object_record_id = NEW.object_record_id
           AND record.object_address = jsonb_build_object(
               'kind', 'courseBannerRendition', 'courseInstanceId', NEW.course_instance_id,
               'banner', NEW.course_banner_id, 'rendition', NEW.rendition_kind)
           AND record.object_storage_area = 'private-content'
           AND record.object_data_class = 'course-appearance'
           AND record.sha256 = delivery.sha256 AND record.size_bytes = delivery.byte_length
           AND record.media_type = delivery.media_type
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Course Banner delivery requires its exact Object Record';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER course_banner_source_has_exact_object_record
BEFORE INSERT OR UPDATE ON ple_data.course_banner
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_course_banner_source_object_record();

CREATE TRIGGER course_banner_rendition_has_exact_object_record
BEFORE INSERT OR UPDATE ON ple_data.course_banner_rendition
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_course_banner_rendition_object_record();

CREATE TRIGGER course_banner_delivery_has_exact_object_record
BEFORE INSERT OR UPDATE ON ple_data.course_banner_delivery
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_course_banner_delivery_object_record();

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.read_course_banner(p_course_instance_id text)
RETURNS TABLE(course_banner_id uuid, alternative_kind text, alternative_text text)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.current_course_banner_id, course.course_banner_alternative_kind,
           course.course_banner_alternative_text
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id AND course.current_course_banner_id IS NOT NULL
       AND ple_api.current_session_account_is_course_member(p_course_instance_id)
$$;

CREATE FUNCTION ple_api.resolve_current_course_banner(p_banner_id uuid)
RETURNS TABLE(course_instance_id text) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.course_instance_id FROM ple_data.course_instance AS course
     WHERE course.current_course_banner_id = p_banner_id
       AND ple_api.current_session_account_is_course_member(course.course_instance_id)
$$;

CREATE FUNCTION ple_api.stage_course_banner_upload(
    p_course_instance_id text, p_upload_id uuid, p_object_record_id uuid, p_media_type text,
    p_byte_length bigint, p_sha256 bytea, p_width integer, p_height integer, p_expires_millis bigint
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE account_id text; subject_id uuid := gen_random_uuid(); work_id uuid := gen_random_uuid();
    expected_address jsonb := jsonb_build_object('kind', 'courseBannerUpload', 'courseInstanceId', p_course_instance_id, 'upload', p_upload_id);
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_instance_id) THEN RETURN NULL; END IF;
    account_id := ple_api.current_session_account_id();
    INSERT INTO ple_private.object_record
        (object_record_id, object_address, object_storage_area, object_data_class,
         sha256, size_bytes, media_type, created_at)
    VALUES (p_object_record_id, expected_address, 'temp-processing', 'course-appearance',
        p_sha256, p_byte_length, p_media_type, clock_timestamp()) ON CONFLICT DO NOTHING;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.object_record WHERE object_record_id = p_object_record_id
          AND object_address = expected_address AND object_storage_area = 'temp-processing'
          AND object_data_class = 'course-appearance' AND sha256 = p_sha256
          AND size_bytes = p_byte_length AND media_type = p_media_type
    ) THEN
        RETURN NULL;
    END IF;
    INSERT INTO ple_private.course_banner_upload
        (course_banner_upload_id, course_instance_id, account_id, object_record_id, canonical_media_type, byte_length,
         sha256, width, height, expires_at, created_at)
    VALUES (p_upload_id, p_course_instance_id, account_id, p_object_record_id, p_media_type, p_byte_length,
        p_sha256, p_width, p_height, to_timestamp(p_expires_millis / 1000.0), clock_timestamp());
    INSERT INTO ple_private.course_banner_storage_subject
        (course_banner_storage_subject_id, subject_kind, course_instance_id, course_banner_upload_id,
         object_record_id, expected_sha256, expected_size_bytes, expected_media_type, storage_area)
    VALUES (subject_id, 'upload', p_course_instance_id, p_upload_id, p_object_record_id, p_sha256,
        p_byte_length, p_media_type, 'temp-processing');
    INSERT INTO ple_private.course_banner_work
        (course_banner_work_id, course_instance_id, operation_kind, object_record_id, state,
         course_banner_storage_subject_id, created_at)
    VALUES (work_id, p_course_instance_id, 'put-upload', p_object_record_id, 'pending', subject_id, clock_timestamp());
    RETURN work_id;
EXCEPTION WHEN unique_violation OR check_violation THEN RETURN NULL;
END $$;

CREATE FUNCTION ple_api.finalize_course_banner_upload_stage(p_course_instance_id text, p_upload_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.course_banner_work AS work SET state = 'completed', completed_at = clock_timestamp()
      FROM ple_private.course_banner_upload AS upload
     WHERE upload.course_banner_upload_id = p_upload_id AND upload.course_instance_id = p_course_instance_id
       AND upload.account_id = ple_api.current_session_account_id()
       AND work.course_banner_storage_subject_id IN (SELECT course_banner_storage_subject_id
           FROM ple_private.course_banner_storage_subject WHERE course_banner_upload_id = upload.course_banner_upload_id)
       AND work.operation_kind = 'put-upload' AND work.state = 'pending'
       AND ple_api.current_session_account_is_course_instructor(p_course_instance_id);
    RETURN FOUND;
END $$;

CREATE FUNCTION ple_api.read_staged_course_banner_upload(p_course_instance_id text, p_upload_id uuid)
RETURNS TABLE(object_record_id uuid, sha256 bytea, byte_length bigint, canonical_media_type text,
              width integer, height integer, put_work_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT upload.object_record_id, upload.sha256, upload.byte_length, upload.canonical_media_type,
           upload.width, upload.height, work.course_banner_work_id
      FROM ple_private.course_banner_upload upload
      JOIN ple_private.course_banner_storage_subject subject ON subject.course_banner_upload_id = upload.course_banner_upload_id
      JOIN ple_private.course_banner_work work ON work.course_banner_storage_subject_id = subject.course_banner_storage_subject_id
     WHERE upload.course_instance_id = p_course_instance_id AND upload.course_banner_upload_id = p_upload_id
       AND upload.account_id = ple_api.current_session_account_id() AND upload.promoted_at IS NULL
       AND upload.expires_at > clock_timestamp() AND work.operation_kind = 'put-upload'
       AND work.state = 'completed' AND ple_api.current_session_account_is_course_instructor(p_course_instance_id)
$$;

CREATE FUNCTION ple_api.prepare_course_banner_promotion(
    p_course_instance_id text, p_upload_id uuid, p_banner_id uuid, p_kind text, p_text text,
    p_source_object uuid, p_source_sha256 bytea, p_source_size bigint, p_source_media text,
    p_source_width integer, p_source_height integer,
    p_banner_object uuid, p_banner_sha256 bytea, p_banner_size bigint, p_banner_media text,
    p_banner_width integer, p_banner_height integer
) RETURNS TABLE(source_put_work_id uuid, banner_put_work_id uuid)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE upload ple_private.course_banner_upload%ROWTYPE; source_subject uuid := gen_random_uuid();
    banner_delivery uuid := gen_random_uuid();
    source_work uuid := gen_random_uuid(); banner_work uuid := gen_random_uuid();
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_instance_id) THEN RETURN; END IF;
    SELECT * INTO upload FROM ple_private.course_banner_upload
     WHERE course_banner_upload_id = p_upload_id AND course_instance_id = p_course_instance_id
       AND account_id = ple_api.current_session_account_id() AND promoted_at IS NULL
       AND expires_at > clock_timestamp() FOR UPDATE;
    IF NOT FOUND OR p_source_sha256 <> upload.sha256 OR p_source_size <> upload.byte_length
       OR p_source_media <> upload.canonical_media_type
       OR p_source_width IS NULL OR p_source_height IS NULL
       OR p_source_width <> upload.width OR p_source_height <> upload.height
       OR p_source_width::bigint <> p_source_height::bigint * 5
       OR p_banner_size NOT BETWEEN 1 AND 2097152 OR p_banner_media <> 'image/webp'
       OR p_banner_width IS NULL OR p_banner_height IS NULL
       OR p_banner_width <> 1280 OR p_banner_height <> 256 THEN RETURN; END IF;
    INSERT INTO ple_private.object_record
        (object_record_id, object_address, object_storage_area, object_data_class,
         sha256, size_bytes, media_type, created_at)
    VALUES
        (p_source_object, jsonb_build_object('kind', 'courseBannerSource', 'courseInstanceId', p_course_instance_id, 'banner', p_banner_id),
         'private-content', 'course-appearance', p_source_sha256, p_source_size, p_source_media, clock_timestamp()),
        (p_banner_object, jsonb_build_object('kind', 'courseBannerRendition', 'courseInstanceId', p_course_instance_id, 'banner', p_banner_id, 'rendition', 'banner'),
         'private-content', 'course-appearance', p_banner_sha256, p_banner_size, p_banner_media, clock_timestamp())
    ON CONFLICT DO NOTHING;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.object_record WHERE object_record_id = p_source_object
          AND object_address = jsonb_build_object('kind', 'courseBannerSource', 'courseInstanceId', p_course_instance_id, 'banner', p_banner_id)
          AND object_storage_area = 'private-content' AND object_data_class = 'course-appearance'
          AND sha256 = p_source_sha256 AND size_bytes = p_source_size AND media_type = p_source_media
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_private.object_record WHERE object_record_id = p_banner_object
          AND object_address = jsonb_build_object('kind', 'courseBannerRendition', 'courseInstanceId', p_course_instance_id, 'banner', p_banner_id, 'rendition', 'banner')
          AND object_storage_area = 'private-content' AND object_data_class = 'course-appearance'
          AND sha256 = p_banner_sha256 AND size_bytes = p_banner_size AND media_type = p_banner_media
    ) THEN
        RETURN;
    END IF;
    INSERT INTO ple_data.course_banner
        (course_instance_id, course_banner_id, source_object_record_id, source_object_checksum, source_byte_length, source_media_type,
         source_width, source_height)
    VALUES (p_course_instance_id, p_banner_id, p_source_object, p_source_sha256, p_source_size, p_source_media,
        p_source_width, p_source_height);
    INSERT INTO ple_private.course_banner_prepared_presentation VALUES (p_course_instance_id, p_banner_id, p_kind, p_text);
    INSERT INTO ple_private.course_banner_storage_subject VALUES
        (source_subject, 'source', p_course_instance_id, NULL, p_banner_id, p_source_object, p_source_sha256,
         p_source_size, p_source_media, 'private-content');
    INSERT INTO ple_data.course_banner_rendition VALUES
        (p_course_instance_id, p_banner_id, 'banner', p_banner_object, p_banner_width, p_banner_height);
    INSERT INTO ple_data.object_delivery VALUES
        (banner_delivery, p_banner_object, p_banner_sha256, p_banner_media, p_banner_size, 'pending', clock_timestamp());
    INSERT INTO ple_data.course_banner_delivery VALUES
        (banner_delivery, p_banner_object, p_course_instance_id, p_banner_id, 'banner');
    INSERT INTO ple_private.course_banner_work VALUES
        (source_work, p_course_instance_id, p_banner_id, 'put-source', p_source_object, 'pending', source_subject, NULL, clock_timestamp(), NULL),
        (banner_work, p_course_instance_id, p_banner_id, 'put-rendition', p_banner_object, 'pending', NULL, banner_delivery, clock_timestamp(), NULL);
    RETURN QUERY SELECT source_work, banner_work;
EXCEPTION WHEN unique_violation OR foreign_key_violation OR check_violation THEN RETURN;
END $$;

CREATE FUNCTION ple_api.complete_prepared_course_banner_object(p_course_instance_id text, p_banner_id uuid, p_object_record_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.course_banner_work SET state = 'completed', completed_at = clock_timestamp()
     WHERE course_instance_id = p_course_instance_id AND course_banner_id = p_banner_id AND object_record_id = p_object_record_id
       AND operation_kind IN ('put-source', 'put-rendition') AND state = 'pending'
       AND ple_api.current_session_account_is_course_instructor(p_course_instance_id);
    RETURN FOUND;
END $$;

CREATE FUNCTION ple_api.prepare_course_banner_object_deletion(p_put_work_id uuid)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE put_work ple_private.course_banner_work%ROWTYPE; delete_id uuid := gen_random_uuid();
BEGIN
    SELECT * INTO put_work FROM ple_private.course_banner_work WHERE course_banner_work_id = p_put_work_id FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(put_work.course_instance_id)
       OR put_work.operation_kind NOT IN ('put-upload','put-source','put-rendition') THEN RETURN NULL; END IF;
    INSERT INTO ple_private.course_banner_work
        (course_banner_work_id, course_instance_id, course_banner_id, operation_kind, object_record_id, state,
         course_banner_storage_subject_id, object_delivery_id, created_at)
    VALUES (delete_id, put_work.course_instance_id, put_work.course_banner_id,
        CASE put_work.operation_kind WHEN 'put-upload' THEN 'delete-upload' WHEN 'put-source' THEN 'delete-source' ELSE 'delete-rendition' END,
        put_work.object_record_id, 'pending', put_work.course_banner_storage_subject_id, put_work.object_delivery_id, clock_timestamp());
    RETURN delete_id;
EXCEPTION WHEN unique_violation THEN RETURN NULL;
END $$;

CREATE FUNCTION ple_api.complete_course_banner_object_deletion(p_delete_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.course_banner_work SET state = 'completed', completed_at = clock_timestamp()
     WHERE course_banner_work_id = p_delete_work_id AND operation_kind LIKE 'delete-%' AND state = 'pending'
       AND ple_api.current_session_account_is_course_instructor(course_instance_id);
    RETURN FOUND;
END $$;

CREATE FUNCTION ple_api.require_course_banner_object_repair(p_course_instance_id text, p_banner_id uuid, p_object_record_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.course_banner_work SET state = 'repair-required'
     WHERE course_instance_id = p_course_instance_id AND course_banner_id IS NOT DISTINCT FROM p_banner_id AND object_record_id = p_object_record_id
       AND state = 'pending' AND ple_api.current_session_account_is_course_instructor(p_course_instance_id);
    RETURN FOUND;
END $$;

CREATE FUNCTION ple_api.require_course_banner_deletion_repair(p_delete_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.course_banner_work SET state = 'repair-required'
     WHERE course_banner_work_id = p_delete_work_id AND operation_kind LIKE 'delete-%' AND state = 'pending'
       AND ple_api.current_session_account_is_course_instructor(course_instance_id);
    RETURN FOUND;
END $$;

CREATE FUNCTION ple_api.record_course_banner_cleanup_check(p_delete_work_id uuid, p_object_present boolean, p_observed_checksum bytea)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE work ple_private.course_banner_work%ROWTYPE; expected bytea; check_id uuid := gen_random_uuid(); manifest_id uuid := gen_random_uuid(); result text; disposition text;
BEGIN
    SELECT * INTO work FROM ple_private.course_banner_work WHERE course_banner_work_id = p_delete_work_id
      AND operation_kind LIKE 'delete-%' AND state = 'repair-required' FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(work.course_instance_id) THEN RETURN false; END IF;
    SELECT COALESCE(subject.expected_sha256, delivery.sha256) INTO expected FROM ple_private.course_banner_work w
      LEFT JOIN ple_private.course_banner_storage_subject subject ON subject.course_banner_storage_subject_id = w.course_banner_storage_subject_id
      LEFT JOIN ple_data.object_delivery delivery ON delivery.object_delivery_id = w.object_delivery_id WHERE w.course_banner_work_id = p_delete_work_id;
    IF NOT p_object_present AND p_observed_checksum IS NULL THEN result := 'missing';
    ELSIF p_object_present AND p_observed_checksum = expected THEN result := 'verified';
    ELSIF p_object_present AND octet_length(p_observed_checksum) = 32 THEN result := 'mismatched';
    ELSE RETURN false; END IF;
    INSERT INTO ple_private.object_storage_check (object_storage_check_id, object_delivery_id, course_banner_storage_subject_id, expected_sha256, check_result, checked_at)
    VALUES (check_id, work.object_delivery_id, work.course_banner_storage_subject_id, expected, result, clock_timestamp());
    disposition := CASE result WHEN 'missing' THEN 'already_absent' ELSE 'retained' END;
    INSERT INTO ple_private.object_cleanup_manifest VALUES (manifest_id, check_id, clock_timestamp(), disposition);
    INSERT INTO ple_audit.object_storage_check_event VALUES (gen_random_uuid(), check_id, result, clock_timestamp(),
        sha256(convert_to('ple:course-banner-storage-check-event:v1', 'UTF8') || uuid_send(check_id) || convert_to(result, 'UTF8') || expected));
    INSERT INTO ple_audit.object_cleanup_receipt VALUES (gen_random_uuid(), manifest_id, disposition, clock_timestamp());
    UPDATE ple_private.course_banner_work SET state = 'completed', completed_at = clock_timestamp() WHERE course_banner_work_id = p_delete_work_id;
    RETURN true;
END $$;

CREATE FUNCTION ple_api.finalize_course_banner_promotion(p_course_instance_id text, p_upload_id uuid, p_banner_id uuid)
RETURNS TABLE(alternative_kind text, alternative_text text, retired_course_banner_id uuid, upload_put_work_id uuid, retired_source_put_work_id uuid, retired_banner_put_work_id uuid)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE old_banner uuid; kind text; alt_text text;
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_instance_id) THEN RETURN; END IF;
    IF (SELECT count(*) FROM ple_private.course_banner_work WHERE course_instance_id = p_course_instance_id AND course_banner_id = p_banner_id
          AND operation_kind IN ('put-source','put-rendition') AND state = 'completed') <> 2 THEN RETURN; END IF;
    SELECT current_course_banner_id INTO old_banner FROM ple_data.course_instance WHERE course_instance_id = p_course_instance_id FOR UPDATE;
    SELECT presentation.alternative_kind, presentation.alternative_text INTO kind, alt_text
      FROM ple_private.course_banner_prepared_presentation AS presentation
     WHERE presentation.course_instance_id = p_course_instance_id
       AND presentation.course_banner_id = p_banner_id;
    UPDATE ple_data.object_delivery SET delivery_state = 'available' WHERE object_delivery_id IN (SELECT object_delivery_id FROM ple_data.course_banner_delivery WHERE course_instance_id = p_course_instance_id AND course_banner_id = p_banner_id);
    UPDATE ple_data.course_instance SET current_course_banner_id = p_banner_id, course_banner_alternative_kind = kind, course_banner_alternative_text = alt_text WHERE course_instance_id = p_course_instance_id;
    UPDATE ple_private.course_banner_upload SET promoted_at = clock_timestamp() WHERE course_banner_upload_id = p_upload_id AND promoted_at IS NULL;
    IF old_banner IS NOT NULL THEN UPDATE ple_data.object_delivery SET delivery_state = 'retired' WHERE object_delivery_id IN (SELECT object_delivery_id FROM ple_data.course_banner_delivery WHERE course_instance_id = p_course_instance_id AND course_banner_id = old_banner); END IF;
    RETURN QUERY SELECT kind, alt_text, old_banner,
      (SELECT work.course_banner_work_id
         FROM ple_private.course_banner_work AS work
         JOIN ple_private.course_banner_storage_subject AS subject
           ON subject.course_banner_storage_subject_id = work.course_banner_storage_subject_id
        WHERE subject.course_banner_upload_id = p_upload_id
          AND work.operation_kind = 'put-upload'),
      (SELECT work.course_banner_work_id
         FROM ple_private.course_banner_work AS work
        WHERE work.course_instance_id = p_course_instance_id AND work.course_banner_id = old_banner
          AND work.operation_kind = 'put-source'),
      (SELECT work.course_banner_work_id
         FROM ple_private.course_banner_work AS work
         JOIN ple_data.course_banner_delivery AS delivery
           ON delivery.object_delivery_id = work.object_delivery_id
        WHERE work.course_instance_id = p_course_instance_id AND work.course_banner_id = old_banner
          AND delivery.rendition_kind = 'banner');
END $$;

CREATE FUNCTION ple_api.prepare_course_banner_removal(p_course_instance_id text)
RETURNS TABLE(course_banner_id uuid, source_put_work_id uuid, banner_put_work_id uuid)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE banner uuid;
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_instance_id) THEN RETURN; END IF;
    SELECT current_course_banner_id INTO banner FROM ple_data.course_instance WHERE course_instance_id = p_course_instance_id FOR UPDATE;
    IF banner IS NULL THEN RETURN; END IF;
    UPDATE ple_data.course_instance SET current_course_banner_id=NULL, course_banner_alternative_kind=NULL, course_banner_alternative_text=NULL WHERE course_instance_id=p_course_instance_id;
    UPDATE ple_data.object_delivery SET delivery_state='retired'
     WHERE object_delivery_id IN (
         SELECT delivery.object_delivery_id
           FROM ple_data.course_banner_delivery AS delivery
          WHERE delivery.course_instance_id=p_course_instance_id AND delivery.course_banner_id=banner
     );
    RETURN QUERY SELECT banner,
      (SELECT work.course_banner_work_id FROM ple_private.course_banner_work AS work
        WHERE work.course_instance_id=p_course_instance_id AND work.course_banner_id=banner AND work.operation_kind='put-source'),
      (SELECT work.course_banner_work_id FROM ple_private.course_banner_work work JOIN ple_data.course_banner_delivery delivery ON delivery.object_delivery_id=work.object_delivery_id WHERE work.course_instance_id=p_course_instance_id AND work.course_banner_id=banner AND delivery.rendition_kind='banner');
END $$;

