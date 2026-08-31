-- SD1 Object Delivery registry, Object Storage Checks, and Object Cleanup authority.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner, ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.course_object_metadata TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_audit_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.object_delivery_record (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL,
    delivery_kind text NOT NULL CHECK (delivery_kind IN ('catalog_asset', 'course_banner', 'course_record')),
    course_id uuid,
    question_id text,
    version_number integer,
    asset_id uuid,
    sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(sha256) = 32),
    media_type text NOT NULL CHECK (char_length(btrim(media_type)) BETWEEN 1 AND 200),
    byte_length bigint NOT NULL CHECK (byte_length >= 0),
    publication_state text NOT NULL CHECK (publication_state IN ('pending', 'active', 'retired')),
    registered_at timestamp with time zone NOT NULL,
    CONSTRAINT object_delivery_catalog_parent_matches FOREIGN KEY (question_id, version_number)
        REFERENCES ple_data.published_question_version (question_id, version_number),
    CONSTRAINT object_delivery_course_parent_matches FOREIGN KEY (object_id, course_id)
        REFERENCES ple_private.course_object_metadata (object_id, course_id),
    CONSTRAINT object_delivery_parent_shape_is_exact CHECK (
        (delivery_kind = 'catalog_asset' AND question_id IS NOT NULL AND version_number IS NOT NULL
            AND asset_id IS NOT NULL AND course_id IS NULL)
        OR (delivery_kind = 'course_banner' AND course_id IS NOT NULL AND question_id IS NULL
            AND version_number IS NULL AND asset_id IS NULL)
        OR (delivery_kind = 'course_record' AND course_id IS NOT NULL AND question_id IS NULL
            AND version_number IS NULL AND asset_id IS NULL)
    )
);
CREATE FUNCTION ple_data.reject_object_delivery_parent_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NEW.object_id IS DISTINCT FROM OLD.object_id
        OR NEW.delivery_kind IS DISTINCT FROM OLD.delivery_kind
        OR NEW.course_id IS DISTINCT FROM OLD.course_id
        OR NEW.question_id IS DISTINCT FROM OLD.question_id
        OR NEW.version_number IS DISTINCT FROM OLD.version_number
        OR NEW.asset_id IS DISTINCT FROM OLD.asset_id
        OR NEW.sha256 IS DISTINCT FROM OLD.sha256
        OR NEW.media_type IS DISTINCT FROM OLD.media_type
        OR NEW.byte_length IS DISTINCT FROM OLD.byte_length
        OR NEW.registered_at IS DISTINCT FROM OLD.registered_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'an object delivery parent is immutable';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER object_delivery_parent_is_immutable
BEFORE UPDATE ON ple_data.object_delivery_record
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_object_delivery_parent_change();
ALTER TABLE ple_data.object_delivery_record ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.object_delivery_record FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.object_delivery_record FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_object_delivery_parent_change() FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner, ple_audit_owner;
GRANT REFERENCES ON TABLE ple_data.object_delivery_record TO ple_private_owner, ple_audit_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.object_storage_check (
    object_storage_check_id uuid PRIMARY KEY,
    delivery_id uuid NOT NULL UNIQUE REFERENCES ple_data.object_delivery_record (delivery_id),
    expected_sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(expected_sha256) = 32),
    check_result text NOT NULL CHECK (check_result IN ('verified', 'missing', 'mismatched')),
    checked_at timestamp with time zone NOT NULL
);
CREATE TABLE ple_private.object_cleanup_manifest (
    object_cleanup_manifest_id uuid PRIMARY KEY,
    object_storage_check_id uuid NOT NULL REFERENCES ple_private.object_storage_check (object_storage_check_id),
    job_id uuid NOT NULL UNIQUE REFERENCES ple_private.worker_job (job_id),
    authorized_at timestamp with time zone NOT NULL,
    permitted_disposition text NOT NULL CHECK (permitted_disposition IN ('deleted', 'already_absent', 'retained')),
    UNIQUE (object_cleanup_manifest_id, permitted_disposition)
);
ALTER TABLE ple_private.object_storage_check ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.object_storage_check FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.object_cleanup_manifest ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.object_cleanup_manifest FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.object_storage_check,
    ple_private.object_cleanup_manifest FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.object_storage_check TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.object_cleanup_manifest TO ple_audit_owner;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.object_delivery_access_event (
    event_id uuid PRIMARY KEY,
    delivery_id uuid NOT NULL REFERENCES ple_data.object_delivery_record (delivery_id),
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    course_id uuid,
    authorized_at timestamp with time zone NOT NULL,
    UNIQUE (event_id, delivery_id)
);
CREATE TABLE ple_audit.object_storage_check_event (
    event_id uuid PRIMARY KEY,
    object_storage_check_id uuid NOT NULL REFERENCES ple_private.object_storage_check (object_storage_check_id),
    check_result text NOT NULL CHECK (check_result IN ('verified', 'missing', 'mismatched')),
    recorded_at timestamp with time zone NOT NULL,
    digest bytea NOT NULL CHECK (pg_catalog.octet_length(digest) = 32),
    UNIQUE (object_storage_check_id, check_result, digest)
);
CREATE TABLE ple_audit.object_cleanup_receipt (
    object_cleanup_receipt_id uuid PRIMARY KEY,
    object_cleanup_manifest_id uuid NOT NULL,
    disposition text NOT NULL,
    recorded_at timestamp with time zone NOT NULL,
    FOREIGN KEY (object_cleanup_manifest_id, disposition)
        REFERENCES ple_private.object_cleanup_manifest (object_cleanup_manifest_id, permitted_disposition)
);
ALTER TABLE ple_audit.object_delivery_access_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.object_delivery_access_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.object_storage_check_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.object_storage_check_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.object_cleanup_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.object_cleanup_receipt FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.object_delivery_access_event,
    ple_audit.object_storage_check_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.object_cleanup_receipt FROM PUBLIC;
RESET ROLE;
