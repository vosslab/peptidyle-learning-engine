-- Course banner metadata and the database half of its object-store saga.

SET LOCAL ROLE ple_data_owner;

-- A generic Course-owned delivery is still ordinary Course media.  The
-- Object Delivery module cannot name this foreign key because Course does not
-- exist until this point in the manifest.
ALTER TABLE ple_data.course_object_delivery
    ADD CONSTRAINT course_object_delivery_course_fkey
    FOREIGN KEY (course_id) REFERENCES ple_data.course_instance(course_id);

ALTER TABLE ple_data.course_instance
    ADD COLUMN current_course_banner_id uuid,
    ADD COLUMN course_banner_alternative_kind text,
    ADD COLUMN course_banner_alternative_text text,
    ADD CONSTRAINT course_instance_banner_alternative_is_complete CHECK (
        (current_course_banner_id IS NULL AND course_banner_alternative_kind IS NULL
            AND course_banner_alternative_text IS NULL)
        OR (current_course_banner_id IS NOT NULL AND course_banner_alternative_kind = 'decorative'
            AND course_banner_alternative_text IS NULL)
        OR (current_course_banner_id IS NOT NULL AND course_banner_alternative_kind = 'informative'
            AND char_length(btrim(course_banner_alternative_text)) BETWEEN 1 AND 160)
    );

CREATE TABLE ple_data.course_banner (
    course_id uuid NOT NULL REFERENCES ple_data.course_instance,
    course_banner_id uuid NOT NULL,
    source_object_id uuid NOT NULL REFERENCES ple_private.object_record,
    source_object_checksum bytea NOT NULL CHECK (octet_length(source_object_checksum) = 32),
    source_byte_length bigint NOT NULL CHECK (source_byte_length BETWEEN 1 AND 8388608),
    source_media_type text NOT NULL CHECK (source_media_type IN ('image/png', 'image/jpeg', 'image/webp')),
    PRIMARY KEY (course_id, course_banner_id),
    UNIQUE (course_id, course_banner_id, source_object_id)
);
CREATE TABLE ple_data.course_banner_rendition (
    course_id uuid NOT NULL,
    course_banner_id uuid NOT NULL,
    rendition_kind text NOT NULL CHECK (rendition_kind IN ('hero', 'card')),
    object_id uuid NOT NULL REFERENCES ple_private.object_record,
    PRIMARY KEY (course_id, course_banner_id, rendition_kind),
    UNIQUE (course_id, course_banner_id, rendition_kind, object_id),
    FOREIGN KEY (course_id, course_banner_id) REFERENCES ple_data.course_banner
);
CREATE TABLE ple_data.course_banner_delivery (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL,
    course_id uuid NOT NULL,
    course_banner_id uuid NOT NULL,
    rendition_kind text NOT NULL CHECK (rendition_kind IN ('hero', 'card')),
    FOREIGN KEY (delivery_id, object_id) REFERENCES ple_data.object_delivery (delivery_id, object_id),
    FOREIGN KEY (course_id, course_banner_id, rendition_kind, object_id)
        REFERENCES ple_data.course_banner_rendition
            (course_id, course_banner_id, rendition_kind, object_id)
);
CREATE CONSTRAINT TRIGGER course_banner_delivery_preserves_available_owner
AFTER INSERT OR UPDATE OR DELETE ON ple_data.course_banner_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.require_exact_available_object_delivery_owner();
CREATE FUNCTION ple_data.validate_course_banner_source_object_record()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.object_record AS record
         WHERE record.object_id = NEW.source_object_id
           AND record.object_address = jsonb_build_object(
               'kind', 'courseBannerSource', 'course', NEW.course_id, 'banner', NEW.course_banner_id)
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
         WHERE record.object_id = NEW.object_id
           AND record.object_address = jsonb_build_object(
               'kind', 'courseBannerRendition', 'course', NEW.course_id,
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
        JOIN ple_private.object_record AS record ON record.object_id = delivery.object_id
         WHERE delivery.delivery_id = NEW.delivery_id AND delivery.object_id = NEW.object_id
           AND record.object_address = jsonb_build_object(
               'kind', 'courseBannerRendition', 'course', NEW.course_id,
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
ALTER TABLE ple_data.course_banner ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_banner FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_banner_rendition ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_banner_rendition FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_banner_delivery ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_banner_delivery FORCE ROW LEVEL SECURITY;
REVOKE ALL ON ple_data.course_banner, ple_data.course_banner_rendition,
    ple_data.course_banner_delivery FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.validate_course_banner_source_object_record(),
    ple_data.validate_course_banner_rendition_object_record(),
    ple_data.validate_course_banner_delivery_object_record() FROM PUBLIC;
CREATE POLICY course_banner_data_owner_access ON ple_data.course_banner
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_rendition_data_owner_access ON ple_data.course_banner_rendition
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_delivery_data_owner_access ON ple_data.course_banner_delivery
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
GRANT REFERENCES ON ple_data.course_banner, ple_data.course_banner_rendition TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.course_banner_upload (
    course_banner_upload_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance,
    account_id uuid NOT NULL REFERENCES ple_private.account,
    object_id uuid NOT NULL UNIQUE REFERENCES ple_private.object_record,
    canonical_media_type text NOT NULL CHECK (canonical_media_type IN ('image/png', 'image/jpeg', 'image/webp')),
    byte_length bigint NOT NULL CHECK (byte_length BETWEEN 1 AND 8388608),
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
    width integer NOT NULL CHECK (width > 0),
    height integer NOT NULL CHECK (height > 0),
    expires_at timestamptz NOT NULL,
    promoted_at timestamptz,
    created_at timestamptz NOT NULL,
    CHECK (expires_at > created_at)
);
CREATE TABLE ple_private.course_banner_storage_subject (
    course_banner_storage_subject_id uuid PRIMARY KEY,
    subject_kind text NOT NULL CHECK (subject_kind IN ('upload', 'source')),
    course_id uuid NOT NULL REFERENCES ple_data.course_instance,
    course_banner_upload_id uuid REFERENCES ple_private.course_banner_upload,
    course_banner_id uuid,
    object_id uuid NOT NULL UNIQUE REFERENCES ple_private.object_record,
    expected_sha256 bytea NOT NULL CHECK (octet_length(expected_sha256) = 32),
    expected_size_bytes bigint NOT NULL CHECK (expected_size_bytes BETWEEN 1 AND 8388608),
    expected_media_type text NOT NULL CHECK (expected_media_type IN ('image/png', 'image/jpeg', 'image/webp')),
    storage_area text NOT NULL CHECK (storage_area IN ('temp-processing', 'private-content')),
    UNIQUE (course_banner_storage_subject_id, object_id),
    CHECK ((subject_kind = 'upload' AND course_banner_upload_id IS NOT NULL
            AND course_banner_id IS NULL AND storage_area = 'temp-processing')
        OR (subject_kind = 'source' AND course_banner_upload_id IS NULL
            AND course_banner_id IS NOT NULL AND storage_area = 'private-content')),
    FOREIGN KEY (course_id, course_banner_id) REFERENCES ple_data.course_banner
);
CREATE TABLE ple_private.course_banner_work (
    course_banner_work_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance,
    course_banner_id uuid,
    operation_kind text NOT NULL CHECK (operation_kind IN
        ('put-upload', 'put-source', 'put-rendition', 'delete-upload', 'delete-source', 'delete-rendition')),
    object_id uuid NOT NULL,
    state text NOT NULL CHECK (state IN ('pending', 'completed', 'repair-required')),
    course_banner_storage_subject_id uuid REFERENCES ple_private.course_banner_storage_subject,
    delivery_id uuid REFERENCES ple_data.object_delivery,
    created_at timestamptz NOT NULL,
    completed_at timestamptz,
    CHECK ((course_banner_storage_subject_id IS NULL) <> (delivery_id IS NULL)),
    FOREIGN KEY (course_banner_storage_subject_id, object_id)
        REFERENCES ple_private.course_banner_storage_subject (course_banner_storage_subject_id, object_id),
    FOREIGN KEY (delivery_id, object_id)
        REFERENCES ple_data.object_delivery (delivery_id, object_id)
);
CREATE TABLE ple_private.course_banner_prepared_presentation (
    course_id uuid NOT NULL,
    course_banner_id uuid NOT NULL,
    alternative_kind text NOT NULL CHECK (alternative_kind IN ('decorative', 'informative')),
    alternative_text text,
    PRIMARY KEY (course_id, course_banner_id),
    FOREIGN KEY (course_id, course_banner_id) REFERENCES ple_data.course_banner,
    CHECK ((alternative_kind = 'decorative' AND alternative_text IS NULL)
        OR (alternative_kind = 'informative' AND char_length(btrim(alternative_text)) BETWEEN 1 AND 160))
);
ALTER TABLE ple_private.course_banner_upload ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_upload FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_storage_subject ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_storage_subject FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_work ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_work FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_prepared_presentation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_prepared_presentation FORCE ROW LEVEL SECURITY;
REVOKE ALL ON ple_private.course_banner_upload, ple_private.course_banner_storage_subject,
    ple_private.course_banner_work, ple_private.course_banner_prepared_presentation FROM PUBLIC;
CREATE POLICY course_banner_upload_private_owner_access ON ple_private.course_banner_upload
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_subject_private_owner_access ON ple_private.course_banner_storage_subject
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_work_private_owner_access ON ple_private.course_banner_work
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_presentation_private_owner_access ON ple_private.course_banner_prepared_presentation
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_data_owner;
GRANT SELECT, INSERT, UPDATE ON ple_data.course_instance, ple_data.course_banner,
    ple_data.course_banner_rendition, ple_data.course_banner_delivery,
    ple_data.object_delivery TO ple_api_owner;
CREATE POLICY course_instance_api_owner_course_media ON ple_data.course_instance
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_api_owner_course_media ON ple_data.course_banner
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_rendition_api_owner_course_media ON ple_data.course_banner_rendition
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_delivery_api_owner_course_media ON ple_data.course_banner_delivery
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY object_delivery_api_owner_course_media ON ple_data.object_delivery
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
SET LOCAL ROLE ple_private_owner;
GRANT SELECT, INSERT, UPDATE ON ple_private.course_banner_upload,
    ple_private.course_banner_storage_subject, ple_private.course_banner_work,
    ple_private.course_banner_prepared_presentation, ple_private.object_storage_check,
    ple_private.object_cleanup_manifest TO ple_api_owner;
CREATE POLICY course_banner_upload_api_owner_course_media ON ple_private.course_banner_upload
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_subject_api_owner_course_media ON ple_private.course_banner_storage_subject
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_work_api_owner_course_media ON ple_private.course_banner_work
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_presentation_api_owner_course_media ON ple_private.course_banner_prepared_presentation
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY object_storage_check_api_owner_course_media ON ple_private.object_storage_check
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY object_cleanup_manifest_api_owner_course_media ON ple_private.object_cleanup_manifest
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
SET LOCAL ROLE ple_audit_owner;
GRANT INSERT ON ple_audit.object_storage_check_event, ple_audit.object_cleanup_receipt TO ple_api_owner;
CREATE POLICY object_storage_check_event_api_owner_course_media ON ple_audit.object_storage_check_event
    FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY object_cleanup_receipt_api_owner_course_media ON ple_audit.object_cleanup_receipt
    FOR INSERT TO ple_api_owner WITH CHECK (true);
SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.read_course_banner(p_course_id uuid)
RETURNS TABLE(course_banner_id uuid, alternative_kind text, alternative_text text)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.current_course_banner_id, course.course_banner_alternative_kind,
           course.course_banner_alternative_text
      FROM ple_data.course_instance AS course
     WHERE course.course_id = p_course_id AND course.current_course_banner_id IS NOT NULL
       AND ple_api.current_session_account_is_course_member(p_course_id)
$$;
CREATE FUNCTION ple_api.resolve_current_course_banner(p_banner_id uuid)
RETURNS TABLE(course_id uuid) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.course_id FROM ple_data.course_instance AS course
     WHERE course.current_course_banner_id = p_banner_id
       AND ple_api.current_session_account_is_course_member(course.course_id)
$$;
CREATE FUNCTION ple_api.stage_course_banner_upload(
    p_course_id uuid, p_upload_id uuid, p_object_id uuid, p_media_type text,
    p_byte_length bigint, p_sha256 bytea, p_width integer, p_height integer, p_expires_millis bigint
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE account_id uuid; subject_id uuid := gen_random_uuid(); work_id uuid := gen_random_uuid();
    expected_address jsonb := jsonb_build_object('kind', 'courseBannerUpload', 'course', p_course_id, 'upload', p_upload_id);
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_id) THEN RETURN NULL; END IF;
    account_id := ple_api.current_session_account_id();
    INSERT INTO ple_private.object_record
        (object_id, object_address, object_storage_area, object_data_class,
         sha256, size_bytes, media_type, created_at)
    VALUES (p_object_id, expected_address, 'temp-processing', 'course-appearance',
        p_sha256, p_byte_length, p_media_type, clock_timestamp()) ON CONFLICT DO NOTHING;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.object_record WHERE object_id = p_object_id
          AND object_address = expected_address AND object_storage_area = 'temp-processing'
          AND object_data_class = 'course-appearance' AND sha256 = p_sha256
          AND size_bytes = p_byte_length AND media_type = p_media_type
    ) THEN
        RETURN NULL;
    END IF;
    INSERT INTO ple_private.course_banner_upload
        (course_banner_upload_id, course_id, account_id, object_id, canonical_media_type, byte_length,
         sha256, width, height, expires_at, created_at)
    VALUES (p_upload_id, p_course_id, account_id, p_object_id, p_media_type, p_byte_length,
        p_sha256, p_width, p_height, to_timestamp(p_expires_millis / 1000.0), clock_timestamp());
    INSERT INTO ple_private.course_banner_storage_subject
        (course_banner_storage_subject_id, subject_kind, course_id, course_banner_upload_id,
         object_id, expected_sha256, expected_size_bytes, expected_media_type, storage_area)
    VALUES (subject_id, 'upload', p_course_id, p_upload_id, p_object_id, p_sha256,
        p_byte_length, p_media_type, 'temp-processing');
    INSERT INTO ple_private.course_banner_work
        (course_banner_work_id, course_id, operation_kind, object_id, state,
         course_banner_storage_subject_id, created_at)
    VALUES (work_id, p_course_id, 'put-upload', p_object_id, 'pending', subject_id, clock_timestamp());
    RETURN work_id;
EXCEPTION WHEN unique_violation OR check_violation THEN RETURN NULL;
END $$;
CREATE FUNCTION ple_api.finalize_course_banner_upload_stage(p_course_id uuid, p_upload_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.course_banner_work AS work SET state = 'completed', completed_at = clock_timestamp()
      FROM ple_private.course_banner_upload AS upload
     WHERE upload.course_banner_upload_id = p_upload_id AND upload.course_id = p_course_id
       AND upload.account_id = ple_api.current_session_account_id()
       AND work.course_banner_storage_subject_id IN (SELECT course_banner_storage_subject_id
           FROM ple_private.course_banner_storage_subject WHERE course_banner_upload_id = upload.course_banner_upload_id)
       AND work.operation_kind = 'put-upload' AND work.state = 'pending'
       AND ple_api.current_session_account_is_course_instructor(p_course_id);
    RETURN FOUND;
END $$;
CREATE FUNCTION ple_api.read_staged_course_banner_upload(p_course_id uuid, p_upload_id uuid)
RETURNS TABLE(object_id uuid, sha256 bytea, byte_length bigint, canonical_media_type text,
              width integer, height integer, put_work_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT upload.object_id, upload.sha256, upload.byte_length, upload.canonical_media_type,
           upload.width, upload.height, work.course_banner_work_id
      FROM ple_private.course_banner_upload upload
      JOIN ple_private.course_banner_storage_subject subject ON subject.course_banner_upload_id = upload.course_banner_upload_id
      JOIN ple_private.course_banner_work work ON work.course_banner_storage_subject_id = subject.course_banner_storage_subject_id
     WHERE upload.course_id = p_course_id AND upload.course_banner_upload_id = p_upload_id
       AND upload.account_id = ple_api.current_session_account_id() AND upload.promoted_at IS NULL
       AND upload.expires_at > clock_timestamp() AND work.operation_kind = 'put-upload'
       AND work.state = 'completed' AND ple_api.current_session_account_is_course_instructor(p_course_id)
$$;
CREATE FUNCTION ple_api.prepare_course_banner_promotion(
    p_course_id uuid, p_upload_id uuid, p_banner_id uuid, p_kind text, p_text text,
    p_source_object uuid, p_source_sha256 bytea, p_source_size bigint, p_source_media text,
    p_hero_object uuid, p_hero_sha256 bytea, p_hero_size bigint, p_hero_media text,
    p_card_object uuid, p_card_sha256 bytea, p_card_size bigint, p_card_media text
) RETURNS TABLE(source_put_work_id uuid, hero_put_work_id uuid, card_put_work_id uuid)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE upload ple_private.course_banner_upload%ROWTYPE; source_subject uuid := gen_random_uuid();
    hero_delivery uuid := gen_random_uuid(); card_delivery uuid := gen_random_uuid();
    source_work uuid := gen_random_uuid(); hero_work uuid := gen_random_uuid(); card_work uuid := gen_random_uuid();
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_id) THEN RETURN; END IF;
    SELECT * INTO upload FROM ple_private.course_banner_upload
     WHERE course_banner_upload_id = p_upload_id AND course_id = p_course_id
       AND account_id = ple_api.current_session_account_id() AND promoted_at IS NULL
       AND expires_at > clock_timestamp() FOR UPDATE;
    IF NOT FOUND OR p_source_sha256 <> upload.sha256 OR p_source_size <> upload.byte_length
       OR p_source_media <> upload.canonical_media_type OR p_hero_size NOT BETWEEN 1 AND 2097152
       OR p_card_size NOT BETWEEN 1 AND 2097152 OR p_hero_media <> 'image/webp' OR p_card_media <> 'image/webp' THEN RETURN; END IF;
    INSERT INTO ple_private.object_record
        (object_id, object_address, object_storage_area, object_data_class,
         sha256, size_bytes, media_type, created_at)
    VALUES
        (p_source_object, jsonb_build_object('kind', 'courseBannerSource', 'course', p_course_id, 'banner', p_banner_id),
         'private-content', 'course-appearance', p_source_sha256, p_source_size, p_source_media, clock_timestamp()),
        (p_hero_object, jsonb_build_object('kind', 'courseBannerRendition', 'course', p_course_id, 'banner', p_banner_id, 'rendition', 'hero'),
         'private-content', 'course-appearance', p_hero_sha256, p_hero_size, p_hero_media, clock_timestamp()),
        (p_card_object, jsonb_build_object('kind', 'courseBannerRendition', 'course', p_course_id, 'banner', p_banner_id, 'rendition', 'card'),
         'private-content', 'course-appearance', p_card_sha256, p_card_size, p_card_media, clock_timestamp())
    ON CONFLICT DO NOTHING;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.object_record WHERE object_id = p_source_object
          AND object_address = jsonb_build_object('kind', 'courseBannerSource', 'course', p_course_id, 'banner', p_banner_id)
          AND object_storage_area = 'private-content' AND object_data_class = 'course-appearance'
          AND sha256 = p_source_sha256 AND size_bytes = p_source_size AND media_type = p_source_media
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_private.object_record WHERE object_id = p_hero_object
          AND object_address = jsonb_build_object('kind', 'courseBannerRendition', 'course', p_course_id, 'banner', p_banner_id, 'rendition', 'hero')
          AND object_storage_area = 'private-content' AND object_data_class = 'course-appearance'
          AND sha256 = p_hero_sha256 AND size_bytes = p_hero_size AND media_type = p_hero_media
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_private.object_record WHERE object_id = p_card_object
          AND object_address = jsonb_build_object('kind', 'courseBannerRendition', 'course', p_course_id, 'banner', p_banner_id, 'rendition', 'card')
          AND object_storage_area = 'private-content' AND object_data_class = 'course-appearance'
          AND sha256 = p_card_sha256 AND size_bytes = p_card_size AND media_type = p_card_media
    ) THEN
        RETURN;
    END IF;
    INSERT INTO ple_data.course_banner
        (course_id, course_banner_id, source_object_id, source_object_checksum, source_byte_length, source_media_type)
    VALUES (p_course_id, p_banner_id, p_source_object, p_source_sha256, p_source_size, p_source_media);
    INSERT INTO ple_private.course_banner_prepared_presentation VALUES (p_course_id, p_banner_id, p_kind, p_text);
    INSERT INTO ple_private.course_banner_storage_subject VALUES
        (source_subject, 'source', p_course_id, NULL, p_banner_id, p_source_object, p_source_sha256,
         p_source_size, p_source_media, 'private-content');
    INSERT INTO ple_data.course_banner_rendition VALUES
        (p_course_id, p_banner_id, 'hero', p_hero_object), (p_course_id, p_banner_id, 'card', p_card_object);
    INSERT INTO ple_data.object_delivery VALUES
        (hero_delivery, p_hero_object, p_hero_sha256, p_hero_media, p_hero_size, 'pending', clock_timestamp()),
        (card_delivery, p_card_object, p_card_sha256, p_card_media, p_card_size, 'pending', clock_timestamp());
    INSERT INTO ple_data.course_banner_delivery VALUES
        (hero_delivery, p_hero_object, p_course_id, p_banner_id, 'hero'),
        (card_delivery, p_card_object, p_course_id, p_banner_id, 'card');
    INSERT INTO ple_private.course_banner_work VALUES
        (source_work, p_course_id, p_banner_id, 'put-source', p_source_object, 'pending', source_subject, NULL, clock_timestamp(), NULL),
        (hero_work, p_course_id, p_banner_id, 'put-rendition', p_hero_object, 'pending', NULL, hero_delivery, clock_timestamp(), NULL),
        (card_work, p_course_id, p_banner_id, 'put-rendition', p_card_object, 'pending', NULL, card_delivery, clock_timestamp(), NULL);
    RETURN QUERY SELECT source_work, hero_work, card_work;
EXCEPTION WHEN unique_violation OR foreign_key_violation OR check_violation THEN RETURN;
END $$;
CREATE FUNCTION ple_api.complete_prepared_course_banner_object(p_course_id uuid, p_banner_id uuid, p_object_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.course_banner_work SET state = 'completed', completed_at = clock_timestamp()
     WHERE course_id = p_course_id AND course_banner_id = p_banner_id AND object_id = p_object_id
       AND operation_kind IN ('put-source', 'put-rendition') AND state = 'pending'
       AND ple_api.current_session_account_is_course_instructor(p_course_id);
    RETURN FOUND;
END $$;
CREATE FUNCTION ple_api.prepare_course_banner_object_deletion(p_put_work_id uuid)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE put_work ple_private.course_banner_work%ROWTYPE; delete_id uuid := gen_random_uuid();
BEGIN
    SELECT * INTO put_work FROM ple_private.course_banner_work WHERE course_banner_work_id = p_put_work_id FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(put_work.course_id)
       OR put_work.operation_kind NOT IN ('put-upload','put-source','put-rendition') THEN RETURN NULL; END IF;
    INSERT INTO ple_private.course_banner_work
        (course_banner_work_id, course_id, course_banner_id, operation_kind, object_id, state,
         course_banner_storage_subject_id, delivery_id, created_at)
    VALUES (delete_id, put_work.course_id, put_work.course_banner_id,
        CASE put_work.operation_kind WHEN 'put-upload' THEN 'delete-upload' WHEN 'put-source' THEN 'delete-source' ELSE 'delete-rendition' END,
        put_work.object_id, 'pending', put_work.course_banner_storage_subject_id, put_work.delivery_id, clock_timestamp());
    RETURN delete_id;
EXCEPTION WHEN unique_violation THEN RETURN NULL;
END $$;
CREATE FUNCTION ple_api.complete_course_banner_object_deletion(p_delete_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.course_banner_work SET state = 'completed', completed_at = clock_timestamp()
     WHERE course_banner_work_id = p_delete_work_id AND operation_kind LIKE 'delete-%' AND state = 'pending'
       AND ple_api.current_session_account_is_course_instructor(course_id);
    RETURN FOUND;
END $$;
CREATE FUNCTION ple_api.require_course_banner_object_repair(p_course_id uuid, p_banner_id uuid, p_object_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.course_banner_work SET state = 'repair-required'
     WHERE course_id = p_course_id AND course_banner_id IS NOT DISTINCT FROM p_banner_id AND object_id = p_object_id
       AND state = 'pending' AND ple_api.current_session_account_is_course_instructor(p_course_id);
    RETURN FOUND;
END $$;
CREATE FUNCTION ple_api.require_course_banner_deletion_repair(p_delete_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.course_banner_work SET state = 'repair-required'
     WHERE course_banner_work_id = p_delete_work_id AND operation_kind LIKE 'delete-%' AND state = 'pending'
       AND ple_api.current_session_account_is_course_instructor(course_id);
    RETURN FOUND;
END $$;
CREATE FUNCTION ple_api.record_course_banner_cleanup_check(p_delete_work_id uuid, p_object_present boolean, p_observed_checksum bytea)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE work ple_private.course_banner_work%ROWTYPE; expected bytea; check_id uuid := gen_random_uuid(); manifest_id uuid := gen_random_uuid(); result text; disposition text;
BEGIN
    SELECT * INTO work FROM ple_private.course_banner_work WHERE course_banner_work_id = p_delete_work_id
      AND operation_kind LIKE 'delete-%' AND state = 'repair-required' FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(work.course_id) THEN RETURN false; END IF;
    SELECT COALESCE(subject.expected_sha256, delivery.sha256) INTO expected FROM ple_private.course_banner_work w
      LEFT JOIN ple_private.course_banner_storage_subject subject ON subject.course_banner_storage_subject_id = w.course_banner_storage_subject_id
      LEFT JOIN ple_data.object_delivery delivery ON delivery.delivery_id = w.delivery_id WHERE w.course_banner_work_id = p_delete_work_id;
    IF NOT p_object_present AND p_observed_checksum IS NULL THEN result := 'missing';
    ELSIF p_object_present AND p_observed_checksum = expected THEN result := 'verified';
    ELSIF p_object_present AND octet_length(p_observed_checksum) = 32 THEN result := 'mismatched';
    ELSE RETURN false; END IF;
    INSERT INTO ple_private.object_storage_check (object_storage_check_id, delivery_id, course_banner_storage_subject_id, expected_sha256, check_result, checked_at)
    VALUES (check_id, work.delivery_id, work.course_banner_storage_subject_id, expected, result, clock_timestamp());
    disposition := CASE result WHEN 'missing' THEN 'already_absent' ELSE 'retained' END;
    INSERT INTO ple_private.object_cleanup_manifest VALUES (manifest_id, check_id, clock_timestamp(), disposition);
    INSERT INTO ple_audit.object_storage_check_event VALUES (gen_random_uuid(), check_id, result, clock_timestamp(),
        sha256(convert_to('ple:course-banner-storage-check-event:v1', 'UTF8') || uuid_send(check_id) || convert_to(result, 'UTF8') || expected));
    INSERT INTO ple_audit.object_cleanup_receipt VALUES (gen_random_uuid(), manifest_id, disposition, clock_timestamp());
    UPDATE ple_private.course_banner_work SET state = 'completed', completed_at = clock_timestamp() WHERE course_banner_work_id = p_delete_work_id;
    RETURN true;
END $$;
CREATE FUNCTION ple_api.finalize_course_banner_promotion(p_course_id uuid, p_upload_id uuid, p_banner_id uuid)
RETURNS TABLE(alternative_kind text, alternative_text text, retired_course_banner_id uuid, upload_put_work_id uuid, retired_source_put_work_id uuid, retired_hero_put_work_id uuid, retired_card_put_work_id uuid)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE old_banner uuid; kind text; alt_text text;
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_id) THEN RETURN; END IF;
    IF (SELECT count(*) FROM ple_private.course_banner_work WHERE course_id = p_course_id AND course_banner_id = p_banner_id
          AND operation_kind IN ('put-source','put-rendition') AND state = 'completed') <> 3 THEN RETURN; END IF;
    SELECT current_course_banner_id INTO old_banner FROM ple_data.course_instance WHERE course_id = p_course_id FOR UPDATE;
    SELECT presentation.alternative_kind, presentation.alternative_text INTO kind, alt_text
      FROM ple_private.course_banner_prepared_presentation AS presentation
     WHERE presentation.course_id = p_course_id
       AND presentation.course_banner_id = p_banner_id;
    UPDATE ple_data.object_delivery SET delivery_state = 'available' WHERE delivery_id IN (SELECT delivery_id FROM ple_data.course_banner_delivery WHERE course_id = p_course_id AND course_banner_id = p_banner_id);
    UPDATE ple_data.course_instance SET current_course_banner_id = p_banner_id, course_banner_alternative_kind = kind, course_banner_alternative_text = alt_text WHERE course_id = p_course_id;
    UPDATE ple_private.course_banner_upload SET promoted_at = clock_timestamp() WHERE course_banner_upload_id = p_upload_id AND promoted_at IS NULL;
    IF old_banner IS NOT NULL THEN UPDATE ple_data.object_delivery SET delivery_state = 'retired' WHERE delivery_id IN (SELECT delivery_id FROM ple_data.course_banner_delivery WHERE course_id = p_course_id AND course_banner_id = old_banner); END IF;
    RETURN QUERY SELECT kind, alt_text, old_banner,
      (SELECT work.course_banner_work_id
         FROM ple_private.course_banner_work AS work
         JOIN ple_private.course_banner_storage_subject AS subject
           ON subject.course_banner_storage_subject_id = work.course_banner_storage_subject_id
        WHERE subject.course_banner_upload_id = p_upload_id
          AND work.operation_kind = 'put-upload'),
      (SELECT work.course_banner_work_id
         FROM ple_private.course_banner_work AS work
        WHERE work.course_id = p_course_id AND work.course_banner_id = old_banner
          AND work.operation_kind = 'put-source'),
      (SELECT work.course_banner_work_id
         FROM ple_private.course_banner_work AS work
         JOIN ple_data.course_banner_delivery AS delivery
           ON delivery.delivery_id = work.delivery_id
        WHERE work.course_id = p_course_id AND work.course_banner_id = old_banner
          AND delivery.rendition_kind = 'hero'),
      (SELECT work.course_banner_work_id
         FROM ple_private.course_banner_work AS work
         JOIN ple_data.course_banner_delivery AS delivery
           ON delivery.delivery_id = work.delivery_id
        WHERE work.course_id = p_course_id AND work.course_banner_id = old_banner
          AND delivery.rendition_kind = 'card');
END $$;
CREATE FUNCTION ple_api.prepare_course_banner_removal(p_course_id uuid)
RETURNS TABLE(course_banner_id uuid, source_put_work_id uuid, hero_put_work_id uuid, card_put_work_id uuid)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE banner uuid;
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_id) THEN RETURN; END IF;
    SELECT current_course_banner_id INTO banner FROM ple_data.course_instance WHERE course_id = p_course_id FOR UPDATE;
    IF banner IS NULL THEN RETURN; END IF;
    UPDATE ple_data.course_instance SET current_course_banner_id=NULL, course_banner_alternative_kind=NULL, course_banner_alternative_text=NULL WHERE course_id=p_course_id;
    UPDATE ple_data.object_delivery SET delivery_state='retired'
     WHERE delivery_id IN (
         SELECT delivery.delivery_id
           FROM ple_data.course_banner_delivery AS delivery
          WHERE delivery.course_id=p_course_id AND delivery.course_banner_id=banner
     );
    RETURN QUERY SELECT banner,
      (SELECT work.course_banner_work_id FROM ple_private.course_banner_work AS work
        WHERE work.course_id=p_course_id AND work.course_banner_id=banner AND work.operation_kind='put-source'),
      (SELECT work.course_banner_work_id FROM ple_private.course_banner_work work JOIN ple_data.course_banner_delivery delivery ON delivery.delivery_id=work.delivery_id WHERE work.course_id=p_course_id AND work.course_banner_id=banner AND delivery.rendition_kind='hero'),
      (SELECT work.course_banner_work_id FROM ple_private.course_banner_work work JOIN ple_data.course_banner_delivery delivery ON delivery.delivery_id=work.delivery_id WHERE work.course_id=p_course_id AND work.course_banner_id=banner AND delivery.rendition_kind='card');
END $$;
REVOKE ALL ON FUNCTION ple_api.read_course_banner(uuid), ple_api.resolve_current_course_banner(uuid),
    ple_api.stage_course_banner_upload(uuid,uuid,uuid,text,bigint,bytea,integer,integer,bigint),
    ple_api.finalize_course_banner_upload_stage(uuid,uuid), ple_api.read_staged_course_banner_upload(uuid,uuid),
    ple_api.prepare_course_banner_promotion(uuid,uuid,uuid,text,text,uuid,bytea,bigint,text,uuid,bytea,bigint,text,uuid,bytea,bigint,text),
    ple_api.complete_prepared_course_banner_object(uuid,uuid,uuid), ple_api.prepare_course_banner_object_deletion(uuid),
    ple_api.complete_course_banner_object_deletion(uuid), ple_api.require_course_banner_object_repair(uuid,uuid,uuid),
    ple_api.require_course_banner_deletion_repair(uuid), ple_api.record_course_banner_cleanup_check(uuid,boolean,bytea),
    ple_api.finalize_course_banner_promotion(uuid,uuid,uuid), ple_api.prepare_course_banner_removal(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_course_banner(uuid), ple_api.resolve_current_course_banner(uuid),
    ple_api.stage_course_banner_upload(uuid,uuid,uuid,text,bigint,bytea,integer,integer,bigint),
    ple_api.finalize_course_banner_upload_stage(uuid,uuid), ple_api.read_staged_course_banner_upload(uuid,uuid),
    ple_api.prepare_course_banner_promotion(uuid,uuid,uuid,text,text,uuid,bytea,bigint,text,uuid,bytea,bigint,text,uuid,bytea,bigint,text),
    ple_api.complete_prepared_course_banner_object(uuid,uuid,uuid), ple_api.prepare_course_banner_object_deletion(uuid),
    ple_api.complete_course_banner_object_deletion(uuid), ple_api.require_course_banner_object_repair(uuid,uuid,uuid),
    ple_api.require_course_banner_deletion_repair(uuid), ple_api.record_course_banner_cleanup_check(uuid,boolean,bytea),
    ple_api.finalize_course_banner_promotion(uuid,uuid,uuid), ple_api.prepare_course_banner_removal(uuid) TO ple_app;

RESET ROLE;
