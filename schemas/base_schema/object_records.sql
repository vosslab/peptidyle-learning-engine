-- Immutable object identities, delivery ownership, and factual storage receipts.
-- External storage remains outside PostgreSQL.  These rows retain only the
-- exact facts that PostgreSQL can authorize and later reconcile.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.object_record (
    object_id uuid PRIMARY KEY,
    object_address jsonb NOT NULL CHECK (jsonb_typeof(object_address) = 'object'),
    object_storage_area text NOT NULL CHECK (object_storage_area IN
        ('public-assets', 'private-content', 'student-records', 'temp-processing')),
    object_data_class text NOT NULL CHECK (object_data_class IN
        ('authoring-content', 'question-source', 'question-asset', 'question-render',
         'course-appearance', 'profile-thumbnail', 'student-record',
         'temporary-processing')),
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
    size_bytes bigint NOT NULL CHECK (size_bytes >= 0),
    media_type text NOT NULL CHECK (char_length(btrim(media_type)) BETWEEN 1 AND 255),
    created_at timestamptz NOT NULL,
    UNIQUE (object_address)
);

CREATE FUNCTION ple_private.reject_object_record_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Object Records are immutable';
END
$$;

CREATE TRIGGER object_record_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.object_record
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_object_record_change();

ALTER TABLE ple_private.object_record ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.object_record FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.object_record FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_private.reject_object_record_change() FROM PUBLIC;
CREATE POLICY object_record_private_owner_access ON ple_private.object_record
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
-- Object-owned relations use this immutable identity as their foreign-key
-- target.  A delivery may remain pending while an external put is in flight,
-- so it deliberately does not impose a global Object Record FK.
GRANT REFERENCES ON ple_private.object_record TO ple_data_owner, ple_private_owner;
GRANT SELECT ON ple_private.object_record TO ple_data_owner;
CREATE POLICY object_record_data_owner_read_access ON ple_private.object_record
    FOR SELECT TO ple_data_owner USING (true);
GRANT SELECT, INSERT ON ple_private.object_record TO ple_api_owner;
CREATE POLICY object_record_api_owner_insert_access ON ple_private.object_record
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

-- The source-binding tables are authored by question-authoring state.  Keep the
-- exact Object Record FKs there, after both module families have been loaded.

CREATE FUNCTION ple_private.register_workspace_question_source_object(
    p_workspace_id uuid, p_object_id uuid, p_object_address jsonb,
    p_sha256 bytea, p_size_bytes bigint, p_media_type text, p_created_at_millis bigint
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE
    expected_address jsonb := jsonb_build_object(
        'kind', 'workspaceQuestionSource', 'workspace', p_workspace_id,
        'object', p_object_id);
    expected_created_at timestamptz := to_timestamp(p_created_at_millis::double precision / 1000.0);
BEGIN
    IF NOT ple_api.current_session_account_can_access_authoring_workspace(p_workspace_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Workspace Question Source Object registration requires current Authoring Workspace access';
    END IF;
    IF jsonb_typeof(p_object_address) <> 'object' OR p_object_address <> expected_address THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Workspace Question Source Object registration requires its exact typed Object Address';
    END IF;
    INSERT INTO ple_private.object_record (
        object_id, object_address, object_storage_area, object_data_class,
        sha256, size_bytes, media_type, created_at
    ) VALUES (
        p_object_id, expected_address, 'private-content', 'authoring-content',
        p_sha256, p_size_bytes, p_media_type, expected_created_at
    ) ON CONFLICT DO NOTHING;
    IF FOUND OR EXISTS (
        SELECT 1 FROM ple_private.object_record AS record
         WHERE record.object_id = p_object_id AND record.object_address = expected_address
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

GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;
REVOKE ALL ON FUNCTION ple_private.register_workspace_question_source_object(
    uuid, uuid, jsonb, bytea, bigint, text, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.register_workspace_question_source_object(
    uuid, uuid, jsonb, bytea, bigint, text, bigint) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.register_workspace_question_source_object(
    p_workspace_id uuid, p_object_id uuid, p_object_address jsonb,
    p_sha256 bytea, p_size_bytes bigint, p_media_type text, p_created_at_millis bigint
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.register_workspace_question_source_object(
        p_workspace_id, p_object_id, p_object_address, p_sha256, p_size_bytes,
        p_media_type, p_created_at_millis)
$$;
REVOKE ALL ON FUNCTION ple_api.register_workspace_question_source_object(
    uuid, uuid, jsonb, bytea, bigint, text, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.register_workspace_question_source_object(
    uuid, uuid, jsonb, bytea, bigint, text, bigint) TO ple_app;

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.object_delivery (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL,
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
    media_type text NOT NULL CHECK (char_length(btrim(media_type)) BETWEEN 1 AND 200),
    byte_length bigint NOT NULL CHECK (byte_length >= 0),
    delivery_state text NOT NULL CHECK (delivery_state IN ('pending', 'available', 'retired')),
    registered_at timestamptz NOT NULL,
    UNIQUE (delivery_id, object_id)
);
CREATE TABLE ple_data.course_object_delivery (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL,
    course_id uuid NOT NULL,
    FOREIGN KEY (delivery_id, object_id) REFERENCES ple_data.object_delivery (delivery_id, object_id)
);

CREATE FUNCTION ple_data.require_exact_available_object_delivery_owner()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE target_delivery_id uuid := COALESCE(NEW.delivery_id, OLD.delivery_id); owner_count integer;
BEGIN
    IF (SELECT delivery_state FROM ple_data.object_delivery WHERE delivery_id = target_delivery_id) = 'available' THEN
        SELECT (SELECT count(*) FROM ple_data.question_asset_delivery WHERE delivery_id = target_delivery_id)
             + (SELECT count(*) FROM ple_data.course_banner_delivery WHERE delivery_id = target_delivery_id)
             + (SELECT count(*) FROM ple_data.course_object_delivery WHERE delivery_id = target_delivery_id)
             + (SELECT count(*) FROM ple_data.profile_thumbnail_delivery WHERE delivery_id = target_delivery_id)
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
ALTER TABLE ple_data.object_delivery ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.object_delivery FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_object_delivery ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_object_delivery FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_data.object_delivery, ple_data.course_object_delivery FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.require_exact_available_object_delivery_owner() FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner, ple_audit_owner;
GRANT REFERENCES ON ple_data.object_delivery TO ple_private_owner, ple_audit_owner;
CREATE POLICY object_delivery_data_owner_access ON ple_data.object_delivery
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY course_object_delivery_data_owner_access ON ple_data.course_object_delivery
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.object_storage_check (
    object_storage_check_id uuid PRIMARY KEY,
    delivery_id uuid REFERENCES ple_data.object_delivery (delivery_id),
    -- A check applies to one immutable storage anchor.  Course Media owns the
    -- banner subject relation and the late cross-domain foreign key; this
    -- generic record owns the complete anchor shape from its initial DDL.
    course_banner_storage_subject_id uuid,
    expected_sha256 bytea NOT NULL CHECK (octet_length(expected_sha256) = 32),
    check_result text NOT NULL CHECK (check_result IN ('verified', 'missing', 'mismatched')),
    checked_at timestamptz NOT NULL,
    CHECK ((delivery_id IS NOT NULL) <> (course_banner_storage_subject_id IS NOT NULL))
);
CREATE UNIQUE INDEX object_storage_check_delivery_once
    ON ple_private.object_storage_check (delivery_id) WHERE delivery_id IS NOT NULL;
CREATE UNIQUE INDEX object_storage_check_banner_subject_once
    ON ple_private.object_storage_check (course_banner_storage_subject_id)
    WHERE course_banner_storage_subject_id IS NOT NULL;
CREATE TABLE ple_private.object_cleanup_manifest (
    object_cleanup_manifest_id uuid PRIMARY KEY,
    object_storage_check_id uuid NOT NULL REFERENCES ple_private.object_storage_check,
    authorized_at timestamptz NOT NULL,
    permitted_disposition text NOT NULL CHECK (permitted_disposition IN ('deleted', 'already_absent', 'retained')),
    UNIQUE (object_cleanup_manifest_id, permitted_disposition)
);
ALTER TABLE ple_private.object_storage_check ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.object_storage_check FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.object_cleanup_manifest ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.object_cleanup_manifest FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.object_storage_check, ple_private.object_cleanup_manifest FROM PUBLIC;
CREATE POLICY object_storage_check_private_owner_access ON ple_private.object_storage_check
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY object_cleanup_manifest_private_owner_access ON ple_private.object_cleanup_manifest
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON ple_private.object_storage_check, ple_private.object_cleanup_manifest TO ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.object_delivery_access_event (
    event_id uuid PRIMARY KEY,
    delivery_id uuid NOT NULL REFERENCES ple_data.object_delivery,
    account_id uuid NOT NULL REFERENCES ple_private.account,
    access_decision text NOT NULL CHECK (access_decision IN ('allowed', 'denied')),
    accessed_at timestamptz NOT NULL,
    UNIQUE (event_id, delivery_id)
);
CREATE TABLE ple_audit.object_storage_check_event (
    event_id uuid PRIMARY KEY,
    object_storage_check_id uuid NOT NULL REFERENCES ple_private.object_storage_check,
    check_result text NOT NULL CHECK (check_result IN ('verified', 'missing', 'mismatched')),
    recorded_at timestamptz NOT NULL,
    object_storage_check_event_checksum bytea NOT NULL CHECK (octet_length(object_storage_check_event_checksum) = 32),
    UNIQUE (object_storage_check_id, check_result, object_storage_check_event_checksum)
);
CREATE TABLE ple_audit.object_cleanup_receipt (
    object_cleanup_receipt_id uuid PRIMARY KEY,
    object_cleanup_manifest_id uuid NOT NULL,
    disposition text NOT NULL,
    recorded_at timestamptz NOT NULL,
    FOREIGN KEY (object_cleanup_manifest_id, disposition)
        REFERENCES ple_private.object_cleanup_manifest (object_cleanup_manifest_id, permitted_disposition)
);
ALTER TABLE ple_audit.object_delivery_access_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.object_delivery_access_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.object_storage_check_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.object_storage_check_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.object_cleanup_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.object_cleanup_receipt FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_audit.object_delivery_access_event, ple_audit.object_storage_check_event,
    ple_audit.object_cleanup_receipt FROM PUBLIC;
CREATE POLICY object_audit_owner_access ON ple_audit.object_delivery_access_event
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);
CREATE POLICY object_storage_check_event_audit_owner_access ON ple_audit.object_storage_check_event
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);
CREATE POLICY object_cleanup_receipt_audit_owner_access ON ple_audit.object_cleanup_receipt
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);

RESET ROLE;
