-- Persisted Course Banner sources, fixed delivery renditions, and repair anchors.
--
-- This migration deliberately extends the existing object-delivery and cleanup
-- lineage.  Object storage is not transactional with PostgreSQL, so every
-- non-delivery banner object has a durable subject before a worker touches it.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.course_instance
    ADD COLUMN current_course_banner_id uuid,
    ADD COLUMN course_banner_alternative_kind text,
    ADD COLUMN course_banner_alternative_text text,
    ADD CONSTRAINT course_instance_banner_alternative_is_complete CHECK (
        (current_course_banner_id IS NULL
            AND course_banner_alternative_kind IS NULL
            AND course_banner_alternative_text IS NULL)
        OR
        (current_course_banner_id IS NOT NULL
            AND course_banner_alternative_kind = 'decorative'
            AND course_banner_alternative_text IS NULL)
        OR
        (current_course_banner_id IS NOT NULL
            AND course_banner_alternative_kind = 'informative'
            AND char_length(btrim(course_banner_alternative_text)) BETWEEN 1 AND 160)
    );

CREATE TABLE ple_data.course_banner_rendition (
    course_id uuid NOT NULL,
    course_banner_id uuid NOT NULL,
    rendition_kind text NOT NULL CHECK (rendition_kind IN ('hero', 'card')),
    object_id uuid NOT NULL,
    PRIMARY KEY (course_id, course_banner_id, rendition_kind),
    UNIQUE (course_id, course_banner_id, rendition_kind, object_id),
    FOREIGN KEY (course_id, course_banner_id)
        REFERENCES ple_data.course_banner (course_id, course_banner_id)
);

ALTER TABLE ple_data.course_banner_delivery
    ADD COLUMN rendition_kind text;
ALTER TABLE ple_data.course_banner_delivery
    ALTER COLUMN rendition_kind SET NOT NULL,
    ADD CONSTRAINT course_banner_delivery_rendition_kind_is_closed
        CHECK (rendition_kind IN ('hero', 'card'));
ALTER TABLE ple_data.course_banner_delivery
    ADD CONSTRAINT course_banner_delivery_exact_rendition_fkey
        FOREIGN KEY (course_id, course_banner_id, rendition_kind, object_id)
        REFERENCES ple_data.course_banner_rendition
            (course_id, course_banner_id, rendition_kind, object_id);
DO $$
DECLARE v_constraint text;
BEGIN
    SELECT conname INTO v_constraint
      FROM pg_constraint
     WHERE conrelid = 'ple_data.course_banner_delivery'::regclass
       AND contype = 'f'
       AND pg_get_constraintdef(oid) LIKE 'FOREIGN KEY (course_id, course_banner_id, object_id)%';
    IF v_constraint IS NULL THEN
        RAISE EXCEPTION 'course_banner_delivery must retain exactly one existing banner owner foreign key';
    END IF;
    EXECUTE pg_catalog.format('ALTER TABLE ple_data.course_banner_delivery DROP CONSTRAINT %I', v_constraint);
END
$$;

ALTER TABLE ple_data.course_instance
    ADD CONSTRAINT course_instance_current_banner_fkey
        FOREIGN KEY (course_id, current_course_banner_id)
        REFERENCES ple_data.course_banner (course_id, course_banner_id);

ALTER TABLE ple_data.course_banner_rendition ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_banner_rendition FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.course_banner_rendition FROM PUBLIC;
GRANT REFERENCES ON TABLE ple_data.course_banner, ple_data.course_banner_rendition
    TO ple_private_owner;
GRANT SELECT, INSERT, UPDATE ON TABLE ple_data.course_instance,
    ple_data.course_banner, ple_data.course_banner_rendition,
    ple_data.object_delivery, ple_data.course_banner_delivery TO ple_api_owner;
CREATE POLICY course_instance_api_owner_course_banner
    ON ple_data.course_instance FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_instance_api_owner_course_banner_update
    ON ple_data.course_instance FOR UPDATE TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_api_owner_course_banner
    ON ple_data.course_banner FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_banner_api_owner_course_banner_insert
    ON ple_data.course_banner FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY course_banner_rendition_api_owner_course_banner
    ON ple_data.course_banner_rendition FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_banner_rendition_api_owner_course_banner_insert
    ON ple_data.course_banner_rendition FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY object_delivery_api_owner_course_banner
    ON ple_data.object_delivery FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY object_delivery_api_owner_course_banner_insert
    ON ple_data.object_delivery FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY object_delivery_api_owner_course_banner_update
    ON ple_data.object_delivery FOR UPDATE TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_delivery_api_owner_course_banner
    ON ple_data.course_banner_delivery FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_banner_delivery_api_owner_course_banner_insert
    ON ple_data.course_banner_delivery FOR INSERT TO ple_api_owner WITH CHECK (true);

RESET ROLE;

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.course_banner_upload (
    course_banner_upload_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    object_id uuid NOT NULL UNIQUE,
    canonical_media_type text NOT NULL CHECK (canonical_media_type IN ('image/png', 'image/jpeg', 'image/webp')),
    byte_length bigint NOT NULL CHECK (byte_length > 0 AND byte_length <= 8388608),
    sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(sha256) = 32),
    width integer NOT NULL CHECK (width > 0),
    height integer NOT NULL CHECK (height > 0),
    expires_at timestamp with time zone NOT NULL,
    promoted_at timestamp with time zone,
    created_at timestamp with time zone NOT NULL,
    CHECK (expires_at > created_at)
);

CREATE TABLE ple_private.course_banner_storage_subject (
    course_banner_storage_subject_id uuid PRIMARY KEY,
    subject_kind text NOT NULL CHECK (subject_kind IN ('upload', 'source')),
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    course_banner_upload_id uuid REFERENCES ple_private.course_banner_upload (course_banner_upload_id),
    course_banner_id uuid,
    object_id uuid NOT NULL UNIQUE,
    expected_sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(expected_sha256) = 32),
    storage_area text NOT NULL CHECK (storage_area IN ('temp-processing', 'private-content')),
    CHECK (
        (subject_kind = 'upload' AND course_banner_upload_id IS NOT NULL AND course_banner_id IS NULL AND storage_area = 'temp-processing')
        OR (subject_kind = 'source' AND course_banner_upload_id IS NULL AND course_banner_id IS NOT NULL AND storage_area = 'private-content')
    ),
    FOREIGN KEY (course_id, course_banner_id)
        REFERENCES ple_data.course_banner (course_id, course_banner_id)
);

CREATE TABLE ple_private.course_banner_work (
    course_banner_work_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    course_banner_id uuid,
    operation_kind text NOT NULL CHECK (operation_kind IN ('put-upload', 'put-source', 'put-rendition', 'delete-upload', 'delete-source', 'delete-rendition')),
    object_id uuid NOT NULL,
    state text NOT NULL CHECK (state IN ('pending', 'completed', 'repair-required')),
    course_banner_storage_subject_id uuid REFERENCES ple_private.course_banner_storage_subject (course_banner_storage_subject_id),
    delivery_id uuid REFERENCES ple_data.object_delivery (delivery_id),
    created_at timestamp with time zone NOT NULL,
    completed_at timestamp with time zone,
    CHECK ((course_banner_storage_subject_id IS NULL) <> (delivery_id IS NULL))
);
CREATE TABLE ple_private.course_banner_prepared_presentation (
    course_id uuid NOT NULL,
    course_banner_id uuid NOT NULL,
    alternative_kind text NOT NULL CHECK (alternative_kind IN ('decorative', 'informative')),
    alternative_text text,
    PRIMARY KEY (course_id, course_banner_id),
    FOREIGN KEY (course_id, course_banner_id)
        REFERENCES ple_data.course_banner (course_id, course_banner_id),
    CHECK ((alternative_kind = 'decorative' AND alternative_text IS NULL)
        OR (alternative_kind = 'informative'
            AND char_length(btrim(alternative_text)) BETWEEN 1 AND 160))
);

ALTER TABLE ple_private.course_banner_upload ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_upload FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_storage_subject ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_storage_subject FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_work ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_work FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_prepared_presentation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_banner_prepared_presentation FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.course_banner_upload,
    ple_private.course_banner_storage_subject, ple_private.course_banner_work,
    ple_private.course_banner_prepared_presentation FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON TABLE ple_private.course_banner_upload,
    ple_private.course_banner_storage_subject, ple_private.course_banner_work,
    ple_private.course_banner_prepared_presentation TO ple_api_owner;
CREATE POLICY course_banner_upload_api_owner ON ple_private.course_banner_upload
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_storage_subject_api_owner ON ple_private.course_banner_storage_subject
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_work_api_owner ON ple_private.course_banner_work
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_banner_prepared_presentation_api_owner
    ON ple_private.course_banner_prepared_presentation
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
GRANT SELECT, INSERT, UPDATE ON ple_private.job, ple_private.object_storage_check,
    ple_private.object_cleanup_manifest TO ple_api_owner;
CREATE POLICY job_api_owner_course_banner_cleanup ON ple_private.job
    FOR ALL TO ple_api_owner
    USING (job_kind = 'cleanup_course_banner_object' AND job_target_kind = 'course_banner_object')
    WITH CHECK (job_kind = 'cleanup_course_banner_object' AND job_target_kind = 'course_banner_object');

-- Extend, rather than bypass, the storage check anchor for source/upload
-- cleanup.  Existing delivery rows retain their one delivery anchor.
ALTER TABLE ple_private.object_storage_check
    ALTER COLUMN delivery_id DROP NOT NULL,
    ADD COLUMN course_banner_storage_subject_id uuid
        REFERENCES ple_private.course_banner_storage_subject (course_banner_storage_subject_id),
    ADD CONSTRAINT object_storage_check_exact_anchor CHECK (
        (delivery_id IS NOT NULL AND course_banner_storage_subject_id IS NULL)
        OR (delivery_id IS NULL AND course_banner_storage_subject_id IS NOT NULL)
    );
ALTER TABLE ple_private.object_storage_check DROP CONSTRAINT object_storage_check_delivery_id_key;
CREATE UNIQUE INDEX object_storage_check_delivery_id_unique
    ON ple_private.object_storage_check (delivery_id) WHERE delivery_id IS NOT NULL;
CREATE UNIQUE INDEX object_storage_check_banner_subject_unique
    ON ple_private.object_storage_check (course_banner_storage_subject_id)
    WHERE course_banner_storage_subject_id IS NOT NULL;
CREATE POLICY object_storage_check_api_owner_course_banner ON ple_private.object_storage_check
    FOR ALL TO ple_api_owner USING (course_banner_storage_subject_id IS NOT NULL
        OR delivery_id IN (SELECT delivery_id FROM ple_data.course_banner_delivery))
    WITH CHECK (course_banner_storage_subject_id IS NOT NULL
        OR delivery_id IN (SELECT delivery_id FROM ple_data.course_banner_delivery));
CREATE POLICY object_cleanup_manifest_api_owner_course_banner ON ple_private.object_cleanup_manifest
    FOR ALL TO ple_api_owner USING (object_storage_check_id IN (
        SELECT object_storage_check_id FROM ple_private.object_storage_check
         WHERE course_banner_storage_subject_id IS NOT NULL
            OR delivery_id IN (SELECT delivery_id FROM ple_data.course_banner_delivery)))
    WITH CHECK (object_storage_check_id IN (
        SELECT object_storage_check_id FROM ple_private.object_storage_check
         WHERE course_banner_storage_subject_id IS NOT NULL
            OR delivery_id IN (SELECT delivery_id FROM ple_data.course_banner_delivery)));

-- Reuse the existing typed Job/manifest lineage for banner cleanup; the exact
-- delete work id is retained in the immutable job payload.
ALTER TABLE ple_private.job DROP CONSTRAINT job_kind_matches_target,
    DROP CONSTRAINT job_target_shape_is_exact;
ALTER TABLE ple_private.job DROP CONSTRAINT job_job_kind_check;
ALTER TABLE ple_private.job ADD CONSTRAINT job_kind_is_closed CHECK (job_kind IN (
    'grade_accepted_submission', 'recalculate_assignment', 'recalculate_assignment_question_analysis',
    'auto_submit_attempt', 'retention', 'render', 'import', 'qti_import', 'publish_public_assets',
    'cleanup_course_banner_object'
));
ALTER TABLE ple_private.job DROP CONSTRAINT job_job_target_kind_check;
ALTER TABLE ple_private.job ADD CONSTRAINT job_target_kind_is_closed CHECK (job_target_kind IN (
    'course_assignment', 'course_attempt', 'question_submission', 'course_retention',
    'question_revision', 'workspace_import', 'qti_import', 'public_asset_publication',
    'course_banner_object'
));
ALTER TABLE ple_private.job ADD CONSTRAINT job_kind_matches_target CHECK (
    (job_kind = 'grade_accepted_submission' AND job_target_kind = 'question_submission')
    OR (job_kind = 'recalculate_assignment_question_analysis' AND job_target_kind = 'course_assignment')
    OR (job_kind = 'cleanup_course_banner_object' AND job_target_kind = 'course_banner_object')
    OR (job_kind NOT IN ('grade_accepted_submission','recalculate_assignment_question_analysis','cleanup_course_banner_object')
        AND job_target_kind NOT IN ('course_assignment','question_submission','course_banner_object'))
);
ALTER TABLE ple_private.job ADD CONSTRAINT job_target_shape_is_exact CHECK (
    (job_target_kind = 'course_banner_object' AND course_id IS NOT NULL AND source_object_id IS NOT NULL
        AND assignment_id IS NULL AND attempt_id IS NULL AND question_submission_id IS NULL
        AND workspace_id IS NULL AND import_id IS NULL AND question_id IS NULL AND revision_number IS NULL
        AND course_retention_plan_revision_id IS NULL)
    OR (job_target_kind = 'course_assignment' AND course_id IS NOT NULL AND assignment_id IS NOT NULL
        AND attempt_id IS NULL AND question_submission_id IS NULL AND workspace_id IS NULL AND import_id IS NULL
        AND question_id IS NULL AND revision_number IS NULL AND source_object_id IS NULL)
    OR (job_target_kind = 'course_attempt' AND course_id IS NOT NULL AND attempt_id IS NOT NULL
        AND assignment_id IS NULL AND question_submission_id IS NULL AND workspace_id IS NULL AND import_id IS NULL
        AND question_id IS NULL AND revision_number IS NULL AND source_object_id IS NULL)
    OR (job_target_kind = 'question_submission' AND question_submission_id IS NOT NULL
        AND course_id IS NULL AND assignment_id IS NULL AND attempt_id IS NULL
        AND workspace_id IS NULL AND import_id IS NULL AND question_id IS NULL AND revision_number IS NULL AND source_object_id IS NULL)
    OR (job_target_kind = 'course_retention' AND course_id IS NOT NULL AND assignment_id IS NULL
        AND attempt_id IS NULL AND question_submission_id IS NULL AND workspace_id IS NULL AND import_id IS NULL
        AND question_id IS NULL AND revision_number IS NULL AND source_object_id IS NULL
        AND course_retention_plan_revision_id IS NOT NULL)
    OR (job_target_kind IN ('question_revision','public_asset_publication') AND question_id IS NOT NULL
        AND revision_number IS NOT NULL AND course_id IS NULL AND assignment_id IS NULL AND attempt_id IS NULL
        AND question_submission_id IS NULL AND workspace_id IS NULL AND import_id IS NULL AND source_object_id IS NULL)
    OR (job_target_kind = 'workspace_import' AND workspace_id IS NOT NULL AND source_object_id IS NOT NULL
        AND course_id IS NULL AND assignment_id IS NULL AND attempt_id IS NULL AND question_submission_id IS NULL
        AND import_id IS NULL AND question_id IS NULL AND revision_number IS NULL)
    OR (job_target_kind = 'qti_import' AND workspace_id IS NOT NULL AND import_id IS NOT NULL
        AND source_object_id IS NOT NULL AND course_id IS NULL AND assignment_id IS NULL AND attempt_id IS NULL
        AND question_submission_id IS NULL AND question_id IS NULL AND revision_number IS NULL)
);

RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
GRANT INSERT ON ple_audit.object_storage_check_event, ple_audit.object_cleanup_receipt TO ple_api_owner;
CREATE POLICY object_storage_check_event_api_owner_course_banner
    ON ple_audit.object_storage_check_event FOR INSERT TO ple_api_owner WITH CHECK (object_storage_check_id IN (
        SELECT object_storage_check_id FROM ple_private.object_storage_check
         WHERE course_banner_storage_subject_id IS NOT NULL
            OR delivery_id IN (SELECT delivery_id FROM ple_data.course_banner_delivery)
    ));
CREATE POLICY object_cleanup_receipt_api_owner_course_banner
    ON ple_audit.object_cleanup_receipt FOR INSERT TO ple_api_owner WITH CHECK (object_cleanup_manifest_id IN (
        SELECT manifest.object_cleanup_manifest_id FROM ple_private.object_cleanup_manifest AS manifest
         JOIN ple_private.object_storage_check AS check_row
           ON check_row.object_storage_check_id=manifest.object_storage_check_id
         WHERE check_row.course_banner_storage_subject_id IS NOT NULL
            OR check_row.delivery_id IN (SELECT delivery_id FROM ple_data.course_banner_delivery)
    ));
RESET ROLE;

-- These functions form the PostgreSQL side of the cross-store saga.  They
-- intentionally never claim an object put or deletion occurred: completion
-- is recorded only by the server after that exact external operation returns.
SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.read_course_banner(p_course_id uuid)
RETURNS TABLE(course_banner_id uuid, alternative_kind text, alternative_text text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.current_course_banner_id, course.course_banner_alternative_kind,
           course.course_banner_alternative_text
      FROM ple_data.course_instance AS course
     WHERE course.course_id = p_course_id
       AND course.current_course_banner_id IS NOT NULL
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
    p_byte_length bigint, p_sha256 bytea, p_width integer, p_height integer,
    p_expires_millis bigint)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_account uuid; v_subject uuid := gen_random_uuid(); v_work uuid := gen_random_uuid();
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_id) THEN RETURN NULL; END IF;
    v_account := ple_api.current_session_account_id();
    INSERT INTO ple_private.course_banner_upload
        (course_banner_upload_id, course_id, account_id, object_id, canonical_media_type,
         byte_length, sha256, width, height, expires_at, created_at)
    VALUES (p_upload_id, p_course_id, v_account, p_object_id, p_media_type,
            p_byte_length, p_sha256, p_width, p_height,
            to_timestamp(p_expires_millis / 1000.0), clock_timestamp());
    INSERT INTO ple_private.course_banner_storage_subject
        (course_banner_storage_subject_id, subject_kind, course_id,
         course_banner_upload_id, object_id, expected_sha256, storage_area)
    VALUES (v_subject, 'upload', p_course_id, p_upload_id, p_object_id, p_sha256,
            'temp-processing');
    INSERT INTO ple_private.course_banner_work
        (course_banner_work_id, course_id, operation_kind, object_id, state,
         course_banner_storage_subject_id, created_at)
    VALUES (v_work, p_course_id, 'put-upload', p_object_id, 'pending',
            v_subject, clock_timestamp());
    RETURN v_work;
EXCEPTION WHEN unique_violation OR check_violation THEN RETURN NULL;
END $$;

CREATE FUNCTION ple_api.finalize_course_banner_upload_stage(p_course_id uuid, p_upload_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_id) THEN RETURN false; END IF;
    UPDATE ple_private.course_banner_work AS work
       SET state = 'completed', completed_at = clock_timestamp()
      FROM ple_private.course_banner_upload AS upload
     WHERE upload.course_banner_upload_id = p_upload_id
       AND upload.course_id = p_course_id
       AND upload.account_id = ple_api.current_session_account_id()
       AND work.course_banner_storage_subject_id = (
           SELECT subject.course_banner_storage_subject_id
             FROM ple_private.course_banner_storage_subject AS subject
            WHERE subject.course_banner_upload_id = upload.course_banner_upload_id
              AND subject.subject_kind = 'upload')
       AND work.operation_kind = 'put-upload' AND work.state = 'pending';
    RETURN FOUND;
END $$;

CREATE FUNCTION ple_api.read_staged_course_banner_upload(p_course_id uuid, p_upload_id uuid)
RETURNS TABLE(object_id uuid, sha256 bytea, byte_length bigint, canonical_media_type text,
              width integer, height integer, put_work_id uuid)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT upload.object_id, upload.sha256, upload.byte_length,
           upload.canonical_media_type, upload.width, upload.height,
           work.course_banner_work_id
      FROM ple_private.course_banner_upload AS upload
      JOIN ple_private.course_banner_storage_subject AS subject
        ON subject.course_banner_upload_id=upload.course_banner_upload_id
      JOIN ple_private.course_banner_work AS work
        ON work.course_banner_storage_subject_id=subject.course_banner_storage_subject_id
     WHERE upload.course_id = p_course_id
       AND upload.course_banner_upload_id = p_upload_id
       AND upload.account_id = ple_api.current_session_account_id()
       AND upload.promoted_at IS NULL AND upload.expires_at > clock_timestamp()
       AND ple_api.current_session_account_is_course_instructor(p_course_id)
       AND work.operation_kind='put-upload' AND work.state='completed'
$$;

CREATE FUNCTION ple_api.prepare_course_banner_promotion(
    p_course_id uuid, p_upload_id uuid, p_banner_id uuid, p_kind text, p_text text,
    p_source_object uuid, p_source_sha256 bytea, p_source_size bigint, p_source_media text,
    p_hero_object uuid, p_hero_sha256 bytea, p_hero_size bigint, p_hero_media text,
    p_card_object uuid, p_card_sha256 bytea, p_card_size bigint, p_card_media text)
RETURNS TABLE(source_put_work_id uuid, hero_put_work_id uuid, card_put_work_id uuid)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_upload ple_private.course_banner_upload%ROWTYPE;
    v_source_subject uuid := gen_random_uuid();
    v_hero_delivery uuid := gen_random_uuid(); v_card_delivery uuid := gen_random_uuid();
    v_source_work uuid := gen_random_uuid(); v_hero_work uuid := gen_random_uuid(); v_card_work uuid := gen_random_uuid();
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_id) THEN RETURN; END IF;
    SELECT * INTO v_upload FROM ple_private.course_banner_upload
     WHERE course_banner_upload_id = p_upload_id AND course_id = p_course_id
       AND account_id = ple_api.current_session_account_id() AND promoted_at IS NULL
       AND expires_at > clock_timestamp() FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.course_banner_work AS work
         JOIN ple_private.course_banner_storage_subject AS subject
           ON subject.course_banner_storage_subject_id = work.course_banner_storage_subject_id
         WHERE subject.course_banner_upload_id = p_upload_id
           AND work.operation_kind = 'put-upload' AND work.state = 'completed'
    ) THEN RETURN; END IF;
    IF p_source_sha256 <> v_upload.sha256 OR p_source_size <> v_upload.byte_length
       OR p_source_media <> v_upload.canonical_media_type THEN RETURN; END IF;
    IF p_source_size <= 0 OR p_hero_size NOT BETWEEN 1 AND 2097152
       OR p_card_size NOT BETWEEN 1 AND 2097152
       OR p_hero_media <> 'image/webp' OR p_card_media <> 'image/webp' THEN RETURN; END IF;
    INSERT INTO ple_data.course_banner (course_id, course_banner_id, object_id)
      VALUES (p_course_id, p_banner_id, p_source_object);
    INSERT INTO ple_private.course_banner_prepared_presentation
      (course_id, course_banner_id, alternative_kind, alternative_text)
    VALUES (p_course_id, p_banner_id, p_kind, p_text);
    INSERT INTO ple_private.course_banner_storage_subject
      (course_banner_storage_subject_id, subject_kind, course_id, course_banner_id,
       object_id, expected_sha256, storage_area)
    VALUES (v_source_subject, 'source', p_course_id, p_banner_id, p_source_object,
            p_source_sha256, 'private-content');
    INSERT INTO ple_data.course_banner_rendition
      (course_id, course_banner_id, rendition_kind, object_id)
    VALUES (p_course_id,p_banner_id,'hero',p_hero_object),
           (p_course_id,p_banner_id,'card',p_card_object);
    INSERT INTO ple_data.object_delivery
      (delivery_id,object_id,sha256,media_type,byte_length,delivery_state,registered_at)
    VALUES (v_hero_delivery,p_hero_object,p_hero_sha256,p_hero_media,p_hero_size,'pending',clock_timestamp()),
           (v_card_delivery,p_card_object,p_card_sha256,p_card_media,p_card_size,'pending',clock_timestamp());
    INSERT INTO ple_data.course_banner_delivery
      (delivery_id,object_id,course_id,course_banner_id,rendition_kind)
    VALUES (v_hero_delivery,p_hero_object,p_course_id,p_banner_id,'hero'),
           (v_card_delivery,p_card_object,p_course_id,p_banner_id,'card');
    INSERT INTO ple_private.course_banner_work
      (course_banner_work_id,course_id,course_banner_id,operation_kind,object_id,state,course_banner_storage_subject_id,created_at)
    VALUES (v_source_work,p_course_id,p_banner_id,'put-source',p_source_object,'pending',v_source_subject,clock_timestamp());
    INSERT INTO ple_private.course_banner_work
      (course_banner_work_id,course_id,course_banner_id,operation_kind,object_id,state,delivery_id,created_at)
    VALUES (v_hero_work,p_course_id,p_banner_id,'put-rendition',p_hero_object,'pending',v_hero_delivery,clock_timestamp()),
           (v_card_work,p_course_id,p_banner_id,'put-rendition',p_card_object,'pending',v_card_delivery,clock_timestamp());
    RETURN QUERY SELECT v_source_work, v_hero_work, v_card_work;
EXCEPTION WHEN unique_violation OR foreign_key_violation OR check_violation THEN RETURN;
END $$;

CREATE FUNCTION ple_api.complete_prepared_course_banner_object(
    p_course_id uuid, p_banner_id uuid, p_object_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_id) THEN RETURN false; END IF;
    UPDATE ple_private.course_banner_work SET state = 'completed', completed_at = clock_timestamp()
     WHERE course_id = p_course_id AND course_banner_id = p_banner_id
       AND object_id = p_object_id AND operation_kind IN ('put-source','put-rendition')
       AND state = 'pending';
    RETURN FOUND;
END $$;

CREATE FUNCTION ple_api.prepare_course_banner_object_deletion(p_put_work_id uuid)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE v_work ple_private.course_banner_work%ROWTYPE; v_delete_work uuid := gen_random_uuid();
BEGIN
    SELECT * INTO v_work FROM ple_private.course_banner_work
     WHERE course_banner_work_id=p_put_work_id FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_work.course_id)
       OR v_work.operation_kind NOT IN ('put-upload','put-source','put-rendition') THEN RETURN NULL; END IF;
    INSERT INTO ple_private.course_banner_work
      (course_banner_work_id,course_id,course_banner_id,operation_kind,object_id,state,
       course_banner_storage_subject_id,delivery_id,created_at)
    VALUES (v_delete_work,v_work.course_id,v_work.course_banner_id,
       CASE v_work.operation_kind WHEN 'put-upload' THEN 'delete-upload'
          WHEN 'put-source' THEN 'delete-source' ELSE 'delete-rendition' END,
       v_work.object_id,'pending',v_work.course_banner_storage_subject_id,v_work.delivery_id,clock_timestamp());
    RETURN v_delete_work;
EXCEPTION WHEN unique_violation THEN RETURN NULL;
END $$;

CREATE FUNCTION ple_api.require_course_banner_object_repair(
    p_course_id uuid, p_banner_id uuid, p_object_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_id) THEN RETURN false; END IF;
    IF p_banner_id IS NULL THEN
        UPDATE ple_private.course_banner_work AS work SET state = 'repair-required'
          FROM ple_private.course_banner_storage_subject AS subject
          JOIN ple_private.course_banner_upload AS upload
            ON upload.course_banner_upload_id = subject.course_banner_upload_id
         WHERE work.course_id = p_course_id AND work.course_banner_id IS NULL
           AND work.object_id = p_object_id AND work.state = 'pending'
           AND work.course_banner_storage_subject_id = subject.course_banner_storage_subject_id
           AND subject.subject_kind = 'upload' AND upload.course_id = p_course_id
           AND upload.account_id = ple_api.current_session_account_id();
    ELSE
        UPDATE ple_private.course_banner_work SET state = 'repair-required'
         WHERE course_id = p_course_id AND course_banner_id = p_banner_id
           AND object_id = p_object_id AND state = 'pending';
    END IF;
    RETURN FOUND;
END $$;

CREATE FUNCTION ple_api.complete_course_banner_object_deletion(p_delete_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.course_banner_work SET state = 'completed', completed_at = clock_timestamp()
     WHERE course_banner_work_id = p_delete_work_id
       AND operation_kind IN ('delete-upload','delete-source','delete-rendition')
       AND state = 'pending'
       AND ple_api.current_session_account_is_course_instructor(course_id);
    RETURN FOUND;
END $$;

CREATE FUNCTION ple_api.require_course_banner_deletion_repair(p_delete_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.course_banner_work SET state = 'repair-required'
     WHERE course_banner_work_id=p_delete_work_id
       AND operation_kind IN ('delete-upload','delete-source','delete-rendition')
       AND state='pending' AND ple_api.current_session_account_is_course_instructor(course_id);
    RETURN FOUND;
END $$;

-- A repair worker calls this only after it has checked the exact typed object
-- address.  Unknown object-store outcomes never enter this procedure.
CREATE FUNCTION ple_api.record_course_banner_cleanup_check(
    p_delete_work_id uuid, p_object_present boolean, p_observed_checksum bytea)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE
    v_work ple_private.course_banner_work%ROWTYPE; v_expected bytea;
    v_check uuid := gen_random_uuid(); v_job uuid := gen_random_uuid(); v_manifest uuid := gen_random_uuid();
    v_disposition text; v_object uuid; v_check_result text;
BEGIN
    SELECT * INTO v_work FROM ple_private.course_banner_work
     WHERE course_banner_work_id=p_delete_work_id
       AND operation_kind IN ('delete-upload','delete-source','delete-rendition')
       AND state='repair-required' FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_work.course_id)
       THEN RETURN false; END IF;
    IF v_work.course_banner_storage_subject_id IS NOT NULL THEN
        SELECT expected_sha256, object_id INTO v_expected, v_object
          FROM ple_private.course_banner_storage_subject
         WHERE course_banner_storage_subject_id=v_work.course_banner_storage_subject_id;
    ELSE
        SELECT sha256, object_id INTO v_expected, v_object FROM ple_data.object_delivery
         WHERE delivery_id=v_work.delivery_id;
    END IF;
    IF NOT p_object_present THEN
        IF p_observed_checksum IS NOT NULL THEN RETURN false; END IF;
        v_check_result := 'missing';
    ELSIF p_observed_checksum IS NULL OR pg_catalog.octet_length(p_observed_checksum) <> 32 THEN
        RETURN false;
    ELSIF p_observed_checksum = v_expected THEN v_check_result := 'verified';
    ELSE v_check_result := 'mismatched'; END IF;
    IF v_work.course_banner_storage_subject_id IS NOT NULL THEN
        INSERT INTO ple_private.object_storage_check
          (object_storage_check_id,course_banner_storage_subject_id,expected_sha256,check_result,checked_at)
        VALUES (v_check,v_work.course_banner_storage_subject_id,v_expected,v_check_result,clock_timestamp());
    ELSE
        INSERT INTO ple_private.object_storage_check
          (object_storage_check_id,delivery_id,expected_sha256,check_result,checked_at)
        VALUES (v_check,v_work.delivery_id,v_expected,v_check_result,clock_timestamp());
    END IF;
    v_disposition := CASE v_check_result WHEN 'verified' THEN 'deleted'
        WHEN 'missing' THEN 'already_absent' ELSE 'retained' END;
    INSERT INTO ple_private.job
      (job_id,job_kind,job_target_kind,course_id,source_object_id,generation,payload,state,
       available_at,max_attempts,created_at,completed_at)
    VALUES (v_job,'cleanup_course_banner_object','course_banner_object',v_work.course_id,v_object,1,
       jsonb_build_object('deleteWorkId',p_delete_work_id::text),
       CASE WHEN v_check_result='verified' THEN 'ready' ELSE 'completed' END,
       clock_timestamp(),1,clock_timestamp(),
       CASE WHEN v_check_result='verified' THEN NULL ELSE clock_timestamp() END);
    INSERT INTO ple_private.object_cleanup_manifest
      (object_cleanup_manifest_id,object_storage_check_id,job_id,authorized_at,permitted_disposition)
    VALUES (v_manifest,v_check,v_job,clock_timestamp(),v_disposition);
    INSERT INTO ple_audit.object_storage_check_event
      (event_id,object_storage_check_id,check_result,recorded_at,object_storage_check_event_checksum)
    VALUES (gen_random_uuid(),v_check,v_check_result,clock_timestamp(),
       pg_catalog.sha256(
           pg_catalog.convert_to('ple:course-banner-storage-check-event:v1', 'UTF8')
           || pg_catalog.uuid_send(v_check)
           || pg_catalog.convert_to(v_check_result, 'UTF8')
           || v_expected
       ));
    IF v_check_result <> 'verified' THEN
        INSERT INTO ple_audit.object_cleanup_receipt
          (object_cleanup_receipt_id,object_cleanup_manifest_id,disposition,recorded_at)
        VALUES (gen_random_uuid(),v_manifest,v_disposition,clock_timestamp());
        UPDATE ple_private.course_banner_work
           SET state='completed', completed_at=clock_timestamp()
         WHERE course_banner_work_id=p_delete_work_id AND state='repair-required';
    END IF;
    RETURN true;
END $$;

CREATE FUNCTION ple_api.finalize_course_banner_promotion(
    p_course_id uuid, p_upload_id uuid, p_banner_id uuid)
RETURNS TABLE(alternative_kind text, alternative_text text, retired_course_banner_id uuid,
    upload_put_work_id uuid, retired_source_put_work_id uuid,
    retired_hero_put_work_id uuid, retired_card_put_work_id uuid)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_kind text; v_text text; v_old_banner uuid;
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_id) THEN RETURN; END IF;
    PERFORM 1 FROM ple_private.course_banner_upload
     WHERE course_banner_upload_id=p_upload_id AND course_id=p_course_id
       AND account_id=ple_api.current_session_account_id() AND promoted_at IS NULL
       AND expires_at > clock_timestamp() FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_private.course_banner_work
        WHERE course_id=p_course_id AND course_banner_id=p_banner_id
          AND operation_kind='put-source' AND state='completed')
       OR (SELECT count(*) FROM ple_private.course_banner_work
            WHERE course_id=p_course_id AND course_banner_id=p_banner_id
              AND operation_kind='put-rendition' AND state='completed') <> 2
       OR EXISTS (SELECT 1 FROM ple_private.course_banner_work
            WHERE course_id=p_course_id AND course_banner_id=p_banner_id
              AND operation_kind IN ('put-source','put-rendition') AND state <> 'completed')
    THEN RETURN; END IF;
    SELECT current_course_banner_id INTO v_old_banner
      FROM ple_data.course_instance WHERE course_id=p_course_id FOR UPDATE;
    -- The alternative text is held in a hidden prepared banner row by the
    -- original promotion function's arguments.  Store it in the work payload
    -- is deliberately avoided; derive it from the source banner below.
    SELECT prepared.alternative_kind, prepared.alternative_text INTO v_kind, v_text
      FROM ple_private.course_banner_prepared_presentation AS prepared
     WHERE prepared.course_id=p_course_id AND prepared.course_banner_id=p_banner_id;
    IF NOT FOUND THEN RETURN; END IF;
    UPDATE ple_data.object_delivery SET delivery_state='available'
     WHERE delivery_id IN (SELECT delivery_id FROM ple_data.course_banner_delivery
                            WHERE course_id=p_course_id AND course_banner_id=p_banner_id);
    UPDATE ple_data.course_instance SET current_course_banner_id=p_banner_id,
       course_banner_alternative_kind=v_kind, course_banner_alternative_text=v_text
     WHERE course_id=p_course_id;
    UPDATE ple_private.course_banner_upload SET promoted_at=clock_timestamp()
     WHERE course_banner_upload_id=p_upload_id;
    IF v_old_banner IS NOT NULL THEN
        UPDATE ple_data.object_delivery SET delivery_state='retired'
         WHERE delivery_id IN (SELECT delivery_id FROM ple_data.course_banner_delivery
                                WHERE course_id=p_course_id AND course_banner_id=v_old_banner);
    END IF;
    RETURN QUERY SELECT v_kind, v_text, v_old_banner,
      (SELECT work.course_banner_work_id FROM ple_private.course_banner_work AS work
        JOIN ple_private.course_banner_storage_subject AS subject ON subject.course_banner_storage_subject_id=work.course_banner_storage_subject_id
       WHERE subject.course_banner_upload_id=p_upload_id AND work.operation_kind='put-upload'),
      (SELECT work.course_banner_work_id FROM ple_private.course_banner_work AS work
       WHERE work.course_id=p_course_id AND work.course_banner_id=v_old_banner AND work.operation_kind='put-source'),
      (SELECT work.course_banner_work_id FROM ple_private.course_banner_work AS work
        JOIN ple_data.course_banner_delivery AS delivery ON delivery.delivery_id=work.delivery_id
       WHERE work.course_id=p_course_id AND work.course_banner_id=v_old_banner AND work.operation_kind='put-rendition' AND delivery.rendition_kind='hero'),
      (SELECT work.course_banner_work_id FROM ple_private.course_banner_work AS work
        JOIN ple_data.course_banner_delivery AS delivery ON delivery.delivery_id=work.delivery_id
       WHERE work.course_id=p_course_id AND work.course_banner_id=v_old_banner AND work.operation_kind='put-rendition' AND delivery.rendition_kind='card');
END $$;

CREATE FUNCTION ple_api.prepare_course_banner_removal(p_course_id uuid)
RETURNS TABLE(course_banner_id uuid, source_put_work_id uuid, hero_put_work_id uuid, card_put_work_id uuid) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_banner uuid;
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_id) THEN RETURN; END IF;
    SELECT course.current_course_banner_id INTO v_banner FROM ple_data.course_instance AS course
     WHERE course.course_id=p_course_id FOR UPDATE;
    IF v_banner IS NULL THEN RETURN; END IF;
    UPDATE ple_data.course_instance AS course_instance SET current_course_banner_id=NULL,
       course_banner_alternative_kind=NULL, course_banner_alternative_text=NULL
     WHERE course_instance.course_id=p_course_id;
    UPDATE ple_data.object_delivery SET delivery_state='retired'
     WHERE delivery_id IN (SELECT delivery.delivery_id FROM ple_data.course_banner_delivery AS delivery
                            WHERE delivery.course_id=p_course_id AND delivery.course_banner_id=v_banner);
    RETURN QUERY SELECT v_banner,
      (SELECT work.course_banner_work_id FROM ple_private.course_banner_work AS work WHERE work.course_id=p_course_id AND work.course_banner_id=v_banner AND work.operation_kind='put-source'),
      (SELECT work.course_banner_work_id FROM ple_private.course_banner_work AS work JOIN ple_data.course_banner_delivery AS delivery ON delivery.delivery_id=work.delivery_id WHERE work.course_id=p_course_id AND work.course_banner_id=v_banner AND work.operation_kind='put-rendition' AND delivery.rendition_kind='hero'),
      (SELECT work.course_banner_work_id FROM ple_private.course_banner_work AS work JOIN ple_data.course_banner_delivery AS delivery ON delivery.delivery_id=work.delivery_id WHERE work.course_id=p_course_id AND work.course_banner_id=v_banner AND work.operation_kind='put-rendition' AND delivery.rendition_kind='card');
END $$;

REVOKE ALL ON FUNCTION ple_api.stage_course_banner_upload(uuid,uuid,uuid,text,bigint,bytea,integer,integer,bigint),
    ple_api.finalize_course_banner_upload_stage(uuid,uuid),
    ple_api.read_staged_course_banner_upload(uuid,uuid),
    ple_api.prepare_course_banner_promotion(uuid,uuid,uuid,text,text,uuid,bytea,bigint,text,uuid,bytea,bigint,text,uuid,bytea,bigint,text),
    ple_api.complete_prepared_course_banner_object(uuid,uuid,uuid),
    ple_api.require_course_banner_object_repair(uuid,uuid,uuid),
    ple_api.prepare_course_banner_object_deletion(uuid),
    ple_api.complete_course_banner_object_deletion(uuid),
    ple_api.require_course_banner_deletion_repair(uuid),
    ple_api.record_course_banner_cleanup_check(uuid,boolean,bytea),
    ple_api.finalize_course_banner_promotion(uuid,uuid,uuid),
    ple_api.read_course_banner(uuid), ple_api.resolve_current_course_banner(uuid),
    ple_api.prepare_course_banner_removal(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.stage_course_banner_upload(uuid,uuid,uuid,text,bigint,bytea,integer,integer,bigint),
    ple_api.finalize_course_banner_upload_stage(uuid,uuid),
    ple_api.read_staged_course_banner_upload(uuid,uuid),
    ple_api.prepare_course_banner_promotion(uuid,uuid,uuid,text,text,uuid,bytea,bigint,text,uuid,bytea,bigint,text,uuid,bytea,bigint,text),
    ple_api.complete_prepared_course_banner_object(uuid,uuid,uuid),
    ple_api.require_course_banner_object_repair(uuid,uuid,uuid),
    ple_api.prepare_course_banner_object_deletion(uuid),
    ple_api.complete_course_banner_object_deletion(uuid),
    ple_api.require_course_banner_deletion_repair(uuid),
    ple_api.record_course_banner_cleanup_check(uuid,boolean,bytea),
    ple_api.finalize_course_banner_promotion(uuid,uuid,uuid),
    ple_api.read_course_banner(uuid), ple_api.resolve_current_course_banner(uuid),
    ple_api.prepare_course_banner_removal(uuid) TO ple_app;
RESET ROLE;
