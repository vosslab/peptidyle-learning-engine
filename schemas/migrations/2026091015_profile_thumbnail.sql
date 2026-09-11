-- Self-only Instructor Profile thumbnail delivery and external-write work.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'migration 2026091015 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.profile_thumbnail_delivery (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL,
    profile_thumbnail_id uuid NOT NULL UNIQUE,
    FOREIGN KEY (delivery_id, object_id) REFERENCES ple_data.object_delivery (delivery_id, object_id)
);
ALTER TABLE ple_data.profile_thumbnail_delivery ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.profile_thumbnail_delivery FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.profile_thumbnail_delivery FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON ple_data.object_delivery, ple_data.profile_thumbnail_delivery TO ple_api_owner;
CREATE POLICY profile_thumbnail_delivery_api_owner ON ple_data.profile_thumbnail_delivery
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
-- The deferred exact-owner trigger runs as ple_data_owner after an API
-- definer returns.  Give that owner only the read policy it needs to count
-- this relation, matching the established delivery-owner pattern.
CREATE POLICY profile_thumbnail_delivery_owner_trigger_data_owner_read
    ON ple_data.profile_thumbnail_delivery
    FOR SELECT TO ple_data_owner USING (true);

CREATE OR REPLACE FUNCTION ple_data.require_exact_available_object_delivery_owner()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE
    target_delivery_id uuid := COALESCE(NEW.delivery_id, OLD.delivery_id);
    owner_count integer;
BEGIN
    IF (SELECT delivery_state FROM ple_data.object_delivery WHERE delivery_id = target_delivery_id) = 'available' THEN
        SELECT (SELECT count(*) FROM ple_data.question_asset_delivery WHERE delivery_id = target_delivery_id)
             + (SELECT count(*) FROM ple_data.course_banner_delivery WHERE delivery_id = target_delivery_id)
             + (SELECT count(*) FROM ple_data.course_object_delivery WHERE delivery_id = target_delivery_id)
             + (SELECT count(*) FROM ple_data.profile_thumbnail_delivery WHERE delivery_id = target_delivery_id)
          INTO owner_count;
        IF owner_count <> 1 THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'available object delivery must have exactly one owner';
        END IF;
    END IF;
    RETURN NULL;
END
$$;
CREATE CONSTRAINT TRIGGER profile_thumbnail_delivery_preserves_available_owner
    AFTER INSERT OR UPDATE OR DELETE ON ple_data.profile_thumbnail_delivery DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION ple_data.require_exact_available_object_delivery_owner();

RESET ROLE;
SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.instructor_profile_thumbnail (
    account_id uuid PRIMARY KEY REFERENCES ple_private.account (account_id),
    profile_thumbnail_id uuid NOT NULL UNIQUE,
    delivery_id uuid NOT NULL UNIQUE REFERENCES ple_data.object_delivery (delivery_id)
);
CREATE TABLE ple_private.profile_thumbnail_work (
    profile_thumbnail_work_id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    profile_thumbnail_id uuid NOT NULL,
    delivery_id uuid NOT NULL REFERENCES ple_data.object_delivery (delivery_id),
    object_id uuid NOT NULL,
    operation_kind text NOT NULL CHECK (operation_kind IN ('put', 'delete')),
    state text NOT NULL CHECK (state IN ('pending', 'completed', 'finalized', 'repair-required')),
    created_at timestamp with time zone NOT NULL,
    completed_at timestamp with time zone
);
ALTER TABLE ple_private.profile_thumbnail_work
    ADD CONSTRAINT profile_thumbnail_work_one_operation_per_thumbnail
    UNIQUE (profile_thumbnail_id, operation_kind);
ALTER TABLE ple_private.instructor_profile_thumbnail ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.instructor_profile_thumbnail FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.profile_thumbnail_work ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.profile_thumbnail_work FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.instructor_profile_thumbnail, ple_private.profile_thumbnail_work FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON ple_private.instructor_profile_thumbnail, ple_private.profile_thumbnail_work TO ple_api_owner;
CREATE POLICY instructor_profile_thumbnail_api_owner ON ple_private.instructor_profile_thumbnail
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY profile_thumbnail_work_api_owner ON ple_private.profile_thumbnail_work
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

-- Profile Thumbnail deletion uses the already-established exact delivery,
-- manifest, and audit lineage.  It gains no independent cleanup platform.
GRANT SELECT, INSERT, UPDATE ON ple_private.job, ple_private.object_storage_check,
    ple_private.object_cleanup_manifest TO ple_api_owner;
CREATE POLICY object_storage_check_api_owner_profile_thumbnail
    ON ple_private.object_storage_check FOR ALL TO ple_api_owner
    USING (delivery_id IN (SELECT delivery_id FROM ple_data.profile_thumbnail_delivery))
    WITH CHECK (delivery_id IN (SELECT delivery_id FROM ple_data.profile_thumbnail_delivery));
CREATE POLICY object_cleanup_manifest_api_owner_profile_thumbnail
    ON ple_private.object_cleanup_manifest FOR ALL TO ple_api_owner
    USING (object_storage_check_id IN (
        SELECT object_storage_check_id FROM ple_private.object_storage_check
         WHERE delivery_id IN (SELECT delivery_id FROM ple_data.profile_thumbnail_delivery)
    ))
    WITH CHECK (object_storage_check_id IN (
        SELECT object_storage_check_id FROM ple_private.object_storage_check
         WHERE delivery_id IN (SELECT delivery_id FROM ple_data.profile_thumbnail_delivery)
    ));
CREATE POLICY job_api_owner_profile_thumbnail_cleanup ON ple_private.job
    FOR ALL TO ple_api_owner
    USING (job_kind = 'cleanup_profile_thumbnail' AND job_target_kind = 'profile_thumbnail')
    WITH CHECK (job_kind = 'cleanup_profile_thumbnail' AND job_target_kind = 'profile_thumbnail');

RESET ROLE;
SET LOCAL ROLE ple_private_owner;
ALTER TABLE ple_private.job DROP CONSTRAINT job_kind_matches_target,
    DROP CONSTRAINT job_target_shape_is_exact;
ALTER TABLE ple_private.job DROP CONSTRAINT job_kind_is_closed;
ALTER TABLE ple_private.job ADD CONSTRAINT job_kind_is_closed CHECK (job_kind IN (
    'grade_accepted_submission', 'recalculate_assignment', 'recalculate_assignment_question_analysis',
    'auto_submit_attempt', 'retention', 'render', 'import', 'qti_import', 'publish_public_assets',
    'cleanup_course_banner_object', 'cleanup_profile_thumbnail'
));
ALTER TABLE ple_private.job DROP CONSTRAINT job_target_kind_is_closed;
ALTER TABLE ple_private.job ADD CONSTRAINT job_target_kind_is_closed CHECK (job_target_kind IN (
    'course_assignment', 'course_attempt', 'question_submission', 'course_retention',
    'question_revision', 'workspace_import', 'qti_import', 'public_asset_publication',
    'course_banner_object', 'profile_thumbnail'
));
ALTER TABLE ple_private.job ADD CONSTRAINT job_kind_matches_target CHECK (
    (job_kind = 'grade_accepted_submission' AND job_target_kind = 'question_submission')
    OR (job_kind = 'recalculate_assignment_question_analysis' AND job_target_kind = 'course_assignment')
    OR (job_kind = 'cleanup_course_banner_object' AND job_target_kind = 'course_banner_object')
    OR (job_kind = 'cleanup_profile_thumbnail' AND job_target_kind = 'profile_thumbnail')
    OR (job_kind NOT IN ('grade_accepted_submission','recalculate_assignment_question_analysis',
        'cleanup_course_banner_object','cleanup_profile_thumbnail')
        AND job_target_kind NOT IN ('course_assignment','question_submission','course_banner_object','profile_thumbnail'))
);
ALTER TABLE ple_private.job ADD CONSTRAINT job_target_shape_is_exact CHECK (
    (job_target_kind = 'course_banner_object' AND course_id IS NOT NULL AND source_object_id IS NOT NULL
        AND assignment_id IS NULL AND attempt_id IS NULL AND question_submission_id IS NULL
        AND workspace_id IS NULL AND import_id IS NULL AND question_id IS NULL AND revision_number IS NULL
        AND course_retention_plan_revision_id IS NULL)
    OR (job_target_kind = 'profile_thumbnail' AND source_object_id IS NOT NULL
        AND course_id IS NULL AND assignment_id IS NULL AND attempt_id IS NULL
        AND question_submission_id IS NULL AND workspace_id IS NULL AND import_id IS NULL
        AND question_id IS NULL AND revision_number IS NULL AND course_retention_plan_revision_id IS NULL)
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
CREATE POLICY object_storage_check_event_api_owner_profile_thumbnail
    ON ple_audit.object_storage_check_event FOR INSERT TO ple_api_owner WITH CHECK (object_storage_check_id IN (
        SELECT object_storage_check_id FROM ple_private.object_storage_check
         WHERE delivery_id IN (SELECT delivery_id FROM ple_data.profile_thumbnail_delivery)
    ));
CREATE POLICY object_cleanup_receipt_api_owner_profile_thumbnail
    ON ple_audit.object_cleanup_receipt FOR INSERT TO ple_api_owner WITH CHECK (object_cleanup_manifest_id IN (
        SELECT manifest.object_cleanup_manifest_id FROM ple_private.object_cleanup_manifest AS manifest
        JOIN ple_private.object_storage_check AS check_row
          ON check_row.object_storage_check_id = manifest.object_storage_check_id
         WHERE check_row.delivery_id IN (SELECT delivery_id FROM ple_data.profile_thumbnail_delivery)
    ));
RESET ROLE;
SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.current_instructor_profile_thumbnail()
RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT thumbnail.profile_thumbnail_id
      FROM ple_private.instructor_profile_thumbnail AS thumbnail
      JOIN ple_private.account AS account ON account.account_id = thumbnail.account_id
     WHERE thumbnail.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'instructor'
       AND ple_api.current_session_account_is_instructor()
$$;

CREATE FUNCTION ple_api.prepare_instructor_profile_thumbnail(
    p_thumbnail_id uuid, p_object_id uuid, p_sha256 bytea, p_byte_length bigint)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_account_id uuid; v_delivery_id uuid := gen_random_uuid(); v_work_id uuid := gen_random_uuid();
BEGIN
    v_account_id := ple_api.current_session_account_id();
    IF NOT ple_api.current_session_account_is_instructor() OR NOT EXISTS (
        SELECT 1 FROM ple_private.account WHERE account_id=v_account_id AND product_role='instructor'
    ) OR p_sha256 IS NULL OR octet_length(p_sha256) <> 32 OR p_byte_length NOT BETWEEN 1 AND 2097152 THEN
        RETURN NULL;
    END IF;
    INSERT INTO ple_data.object_delivery
        (delivery_id, object_id, sha256, media_type, byte_length, delivery_state, registered_at)
    VALUES (v_delivery_id, p_object_id, p_sha256, 'image/webp', p_byte_length, 'pending', clock_timestamp());
    INSERT INTO ple_data.profile_thumbnail_delivery (delivery_id, object_id, profile_thumbnail_id)
    VALUES (v_delivery_id, p_object_id, p_thumbnail_id);
    INSERT INTO ple_private.profile_thumbnail_work
        (profile_thumbnail_work_id, account_id, profile_thumbnail_id, delivery_id, object_id, operation_kind, state, created_at)
    VALUES (v_work_id, v_account_id, p_thumbnail_id, v_delivery_id, p_object_id, 'put', 'pending', clock_timestamp());
    RETURN v_work_id;
EXCEPTION WHEN unique_violation OR foreign_key_violation OR check_violation THEN RETURN NULL;
END
$$;

CREATE FUNCTION ple_api.complete_instructor_profile_thumbnail_put(p_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.profile_thumbnail_work
       SET state='completed', completed_at=clock_timestamp()
     WHERE profile_thumbnail_work_id=p_work_id AND operation_kind='put' AND state='pending'
       AND account_id=ple_api.current_session_account_id()
       AND ple_api.current_session_account_is_instructor();
    RETURN FOUND;
END
$$;

CREATE FUNCTION ple_api.require_instructor_profile_thumbnail_repair(p_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.profile_thumbnail_work
       SET state='repair-required'
     WHERE profile_thumbnail_work_id=p_work_id AND state='pending'
       AND account_id=ple_api.current_session_account_id()
       AND ple_api.current_session_account_is_instructor();
    RETURN FOUND;
END
$$;

CREATE FUNCTION ple_api.prepare_instructor_profile_thumbnail_deletion(p_put_work_id uuid)
RETURNS TABLE(delete_work_id uuid, profile_thumbnail_id uuid) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_put ple_private.profile_thumbnail_work%ROWTYPE;
BEGIN
    SELECT * INTO v_put FROM ple_private.profile_thumbnail_work
     WHERE profile_thumbnail_work_id=p_put_work_id AND operation_kind='put'
       AND state='completed'
       AND account_id=ple_api.current_session_account_id() FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_instructor() THEN RETURN; END IF;
    SELECT work.profile_thumbnail_work_id, work.profile_thumbnail_id
      INTO delete_work_id, profile_thumbnail_id
      FROM ple_private.profile_thumbnail_work AS work
     WHERE work.profile_thumbnail_id=v_put.profile_thumbnail_id AND work.operation_kind='delete';
    IF FOUND THEN RETURN NEXT; RETURN; END IF;
    delete_work_id := gen_random_uuid();
    profile_thumbnail_id := v_put.profile_thumbnail_id;
    INSERT INTO ple_private.profile_thumbnail_work
      (profile_thumbnail_work_id,account_id,profile_thumbnail_id,delivery_id,object_id,operation_kind,state,created_at)
    VALUES (delete_work_id,v_put.account_id,v_put.profile_thumbnail_id,v_put.delivery_id,v_put.object_id,
       'delete','pending',clock_timestamp());
    RETURN NEXT;
EXCEPTION WHEN unique_violation THEN
    SELECT work.profile_thumbnail_work_id, work.profile_thumbnail_id
      INTO delete_work_id, profile_thumbnail_id
      FROM ple_private.profile_thumbnail_work AS work
     WHERE work.profile_thumbnail_id=v_put.profile_thumbnail_id AND work.operation_kind='delete';
    IF FOUND THEN RETURN NEXT; END IF;
END
$$;

CREATE FUNCTION ple_api.complete_instructor_profile_thumbnail_deletion(p_delete_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.profile_thumbnail_work SET state='completed', completed_at=clock_timestamp()
     WHERE profile_thumbnail_work_id=p_delete_work_id AND operation_kind='delete' AND state='pending'
       AND account_id=ple_api.current_session_account_id()
       AND ple_api.current_session_account_is_instructor();
    RETURN FOUND;
END
$$;

CREATE FUNCTION ple_api.require_instructor_profile_thumbnail_deletion_repair(p_delete_work_id uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    UPDATE ple_private.profile_thumbnail_work SET state='repair-required'
     WHERE profile_thumbnail_work_id=p_delete_work_id AND operation_kind='delete' AND state='pending'
       AND account_id=ple_api.current_session_account_id()
       AND ple_api.current_session_account_is_instructor();
    RETURN FOUND;
END
$$;

-- A repair actor reports only an exact observed check.  The established
-- manifest/audit bridge records the durable disposition; it does not invent
-- a deletion result when storage could not be inspected.
CREATE FUNCTION ple_api.record_instructor_profile_thumbnail_cleanup_check(
    p_delete_work_id uuid, p_object_present boolean, p_observed_checksum bytea)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE v_work ple_private.profile_thumbnail_work%ROWTYPE; v_expected bytea;
    v_check uuid := gen_random_uuid(); v_job uuid := gen_random_uuid(); v_manifest uuid := gen_random_uuid();
    v_result text; v_disposition text;
BEGIN
    SELECT * INTO v_work FROM ple_private.profile_thumbnail_work
     WHERE profile_thumbnail_work_id=p_delete_work_id AND operation_kind='delete'
       AND state='repair-required' FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_instructor()
       OR v_work.account_id <> ple_api.current_session_account_id() THEN RETURN false; END IF;
    SELECT sha256 INTO v_expected FROM ple_data.object_delivery WHERE delivery_id=v_work.delivery_id;
    IF NOT FOUND THEN RETURN false; END IF;
    IF NOT p_object_present THEN
        IF p_observed_checksum IS NOT NULL THEN RETURN false; END IF;
        v_result := 'missing';
    ELSIF p_observed_checksum IS NULL OR pg_catalog.octet_length(p_observed_checksum) <> 32 THEN
        RETURN false;
    ELSIF p_observed_checksum = v_expected THEN v_result := 'verified';
    ELSE v_result := 'mismatched'; END IF;
    INSERT INTO ple_private.object_storage_check
      (object_storage_check_id,delivery_id,expected_sha256,check_result,checked_at)
    VALUES (v_check,v_work.delivery_id,v_expected,v_result,clock_timestamp());
    v_disposition := CASE v_result WHEN 'verified' THEN 'deleted'
        WHEN 'missing' THEN 'already_absent' ELSE 'retained' END;
    INSERT INTO ple_private.job
      (job_id,job_kind,job_target_kind,source_object_id,generation,payload,state,
       available_at,max_attempts,created_at,completed_at)
    VALUES (v_job,'cleanup_profile_thumbnail','profile_thumbnail',v_work.object_id,1,
       jsonb_build_object('deleteWorkId',p_delete_work_id::text),
       CASE WHEN v_result='verified' THEN 'ready' ELSE 'completed' END,
       clock_timestamp(),1,clock_timestamp(),
       CASE WHEN v_result='verified' THEN NULL ELSE clock_timestamp() END);
    INSERT INTO ple_private.object_cleanup_manifest
      (object_cleanup_manifest_id,object_storage_check_id,job_id,authorized_at,permitted_disposition)
    VALUES (v_manifest,v_check,v_job,clock_timestamp(),v_disposition);
    INSERT INTO ple_audit.object_storage_check_event
      (event_id,object_storage_check_id,check_result,recorded_at,object_storage_check_event_checksum)
    VALUES (gen_random_uuid(),v_check,v_result,clock_timestamp(),
       pg_catalog.sha256(pg_catalog.convert_to('ple:profile-thumbnail-storage-check-event:v1', 'UTF8')
           || pg_catalog.uuid_send(v_check) || pg_catalog.convert_to(v_result, 'UTF8') || v_expected));
    IF v_result <> 'verified' THEN
        INSERT INTO ple_audit.object_cleanup_receipt
          (object_cleanup_receipt_id,object_cleanup_manifest_id,disposition,recorded_at)
        VALUES (gen_random_uuid(),v_manifest,v_disposition,clock_timestamp());
        UPDATE ple_private.profile_thumbnail_work SET state='completed', completed_at=clock_timestamp()
         WHERE profile_thumbnail_work_id=p_delete_work_id AND state='repair-required';
    END IF;
    RETURN true;
END
$$;

CREATE FUNCTION ple_api.finalize_instructor_profile_thumbnail(p_work_id uuid)
RETURNS TABLE(profile_thumbnail_id uuid, retired_delete_work_id uuid,
    retired_profile_thumbnail_id uuid) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_work ple_private.profile_thumbnail_work%ROWTYPE;
    v_old_delivery uuid; v_old_thumbnail uuid; v_old_object uuid;
BEGIN
    SELECT * INTO v_work FROM ple_private.profile_thumbnail_work
     WHERE profile_thumbnail_work_id=p_work_id AND operation_kind='put' AND state='completed'
       AND account_id=ple_api.current_session_account_id() FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_instructor() THEN RETURN; END IF;
    -- prepare deletion locks this same put row.  Once compensation has
    -- created delete work, this completed put can never become current.
    IF EXISTS (
        SELECT 1 FROM ple_private.profile_thumbnail_work AS delete_work
         WHERE delete_work.profile_thumbnail_id=v_work.profile_thumbnail_id
           AND delete_work.operation_kind='delete'
    ) THEN
        RETURN;
    END IF;
    -- The pointer may be absent.  This account-scoped transaction lock
    -- serializes first-thumbnail finalization with later replacements.
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        'ple:profile-thumbnail-finalize:v1:' || v_work.account_id::text, 0
    ));
    SELECT current_thumbnail.delivery_id, current_thumbnail.profile_thumbnail_id
      INTO v_old_delivery, v_old_thumbnail
      FROM ple_private.instructor_profile_thumbnail AS current_thumbnail
     WHERE current_thumbnail.account_id=v_work.account_id FOR UPDATE;
    INSERT INTO ple_private.instructor_profile_thumbnail (account_id, profile_thumbnail_id, delivery_id)
    VALUES (v_work.account_id, v_work.profile_thumbnail_id, v_work.delivery_id)
    ON CONFLICT (account_id) DO UPDATE SET profile_thumbnail_id=EXCLUDED.profile_thumbnail_id,
        delivery_id=EXCLUDED.delivery_id;
    UPDATE ple_data.object_delivery SET delivery_state='available' WHERE delivery_id=v_work.delivery_id;
    IF v_old_delivery IS NOT NULL THEN
        UPDATE ple_data.object_delivery SET delivery_state='retired' WHERE delivery_id=v_old_delivery;
        SELECT object_id INTO v_old_object FROM ple_data.object_delivery WHERE delivery_id=v_old_delivery;
        retired_delete_work_id := gen_random_uuid();
        retired_profile_thumbnail_id := v_old_thumbnail;
        INSERT INTO ple_private.profile_thumbnail_work
          (profile_thumbnail_work_id,account_id,profile_thumbnail_id,delivery_id,object_id,operation_kind,state,created_at)
        VALUES (retired_delete_work_id,v_work.account_id,v_old_thumbnail,v_old_delivery,v_old_object,
           'delete','pending',clock_timestamp());
    END IF;
    UPDATE ple_private.profile_thumbnail_work SET state='finalized', completed_at=clock_timestamp()
     WHERE profile_thumbnail_work_id=v_work.profile_thumbnail_work_id AND state='completed';
    profile_thumbnail_id := v_work.profile_thumbnail_id;
    RETURN NEXT;
END
$$;

CREATE FUNCTION ple_api.resolve_current_instructor_profile_thumbnail(p_thumbnail_id uuid)
RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT delivery.object_id
      FROM ple_private.instructor_profile_thumbnail AS current_thumbnail
      JOIN ple_data.object_delivery AS delivery ON delivery.delivery_id=current_thumbnail.delivery_id
      JOIN ple_private.account AS account ON account.account_id=current_thumbnail.account_id
     WHERE current_thumbnail.profile_thumbnail_id=p_thumbnail_id
       AND current_thumbnail.account_id=ple_api.current_session_account_id()
       AND account.product_role='instructor'
       AND ple_api.current_session_account_is_instructor()
       AND delivery.delivery_state='available'
$$;

REVOKE ALL ON FUNCTION ple_api.current_instructor_profile_thumbnail(),
    ple_api.prepare_instructor_profile_thumbnail(uuid,uuid,bytea,bigint),
    ple_api.complete_instructor_profile_thumbnail_put(uuid),
    ple_api.require_instructor_profile_thumbnail_repair(uuid),
    ple_api.prepare_instructor_profile_thumbnail_deletion(uuid),
    ple_api.complete_instructor_profile_thumbnail_deletion(uuid),
    ple_api.require_instructor_profile_thumbnail_deletion_repair(uuid),
    ple_api.record_instructor_profile_thumbnail_cleanup_check(uuid,boolean,bytea),
    ple_api.finalize_instructor_profile_thumbnail(uuid),
    ple_api.resolve_current_instructor_profile_thumbnail(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.current_instructor_profile_thumbnail(),
    ple_api.prepare_instructor_profile_thumbnail(uuid,uuid,bytea,bigint),
    ple_api.complete_instructor_profile_thumbnail_put(uuid),
    ple_api.require_instructor_profile_thumbnail_repair(uuid),
    ple_api.prepare_instructor_profile_thumbnail_deletion(uuid),
    ple_api.complete_instructor_profile_thumbnail_deletion(uuid),
    ple_api.require_instructor_profile_thumbnail_deletion_repair(uuid),
    ple_api.record_instructor_profile_thumbnail_cleanup_check(uuid,boolean,bytea),
    ple_api.finalize_instructor_profile_thumbnail(uuid),
    ple_api.resolve_current_instructor_profile_thumbnail(uuid) TO ple_app;
RESET ROLE;
