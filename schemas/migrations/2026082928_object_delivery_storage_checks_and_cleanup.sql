-- SD1 Object Delivery registry, Object Storage Checks, and Object Cleanup authority.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner, ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.course_object_reference TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_audit_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
-- ASVS 2.1.1/2.1.2: delivery state and exact owner relationships are database-validated.
CREATE TABLE ple_data.object_delivery (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL,
    sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(sha256) = 32),
    media_type text NOT NULL CHECK (char_length(btrim(media_type)) BETWEEN 1 AND 200),
    byte_length bigint NOT NULL CHECK (byte_length >= 0),
    delivery_state text NOT NULL CHECK (delivery_state IN ('pending', 'available', 'retired')),
    registered_at timestamp with time zone NOT NULL,
    UNIQUE (delivery_id, object_id)
);
CREATE TABLE ple_data.course_banner (
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    course_banner_id uuid NOT NULL,
    object_id uuid NOT NULL,
    PRIMARY KEY (course_id, course_banner_id),
    UNIQUE (course_id, course_banner_id, object_id)
);
CREATE TABLE ple_data.question_asset_delivery (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL,
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    asset_id uuid NOT NULL,
    FOREIGN KEY (delivery_id, object_id) REFERENCES ple_data.object_delivery (delivery_id, object_id),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number)
);
CREATE TABLE ple_data.course_banner_delivery (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL,
    course_id uuid NOT NULL,
    course_banner_id uuid NOT NULL,
    FOREIGN KEY (delivery_id, object_id) REFERENCES ple_data.object_delivery (delivery_id, object_id),
    FOREIGN KEY (course_id, course_banner_id, object_id)
        REFERENCES ple_data.course_banner (course_id, course_banner_id, object_id)
);
CREATE TABLE ple_data.course_object_delivery (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL,
    course_id uuid NOT NULL,
    FOREIGN KEY (delivery_id, object_id) REFERENCES ple_data.object_delivery (delivery_id, object_id),
    FOREIGN KEY (object_id, course_id)
        REFERENCES ple_private.course_object_reference (object_id, course_id)
);
-- ASVS 2.3.1/2.3.3: only one exact owner may make a delivery available.
CREATE FUNCTION ple_data.require_exact_available_object_delivery_owner()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
DECLARE
    target_delivery_id uuid := COALESCE(NEW.delivery_id, OLD.delivery_id);
    owner_count integer;
BEGIN
    IF (SELECT delivery_state FROM ple_data.object_delivery WHERE delivery_id = target_delivery_id) = 'available' THEN
        SELECT (SELECT count(*) FROM ple_data.question_asset_delivery WHERE delivery_id = target_delivery_id)
             + (SELECT count(*) FROM ple_data.course_banner_delivery WHERE delivery_id = target_delivery_id)
             + (SELECT count(*) FROM ple_data.course_object_delivery WHERE delivery_id = target_delivery_id)
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
CREATE CONSTRAINT TRIGGER question_asset_delivery_preserves_available_owner
AFTER INSERT OR UPDATE OR DELETE ON ple_data.question_asset_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.require_exact_available_object_delivery_owner();
CREATE CONSTRAINT TRIGGER course_banner_delivery_preserves_available_owner
AFTER INSERT OR UPDATE OR DELETE ON ple_data.course_banner_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.require_exact_available_object_delivery_owner();
CREATE CONSTRAINT TRIGGER course_object_delivery_preserves_available_owner
AFTER INSERT OR UPDATE OR DELETE ON ple_data.course_object_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.require_exact_available_object_delivery_owner();
ALTER TABLE ple_data.object_delivery ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.object_delivery FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_banner ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_banner FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_asset_delivery ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_asset_delivery FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_banner_delivery ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_banner_delivery FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_object_delivery ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_object_delivery FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.object_delivery, ple_data.course_banner,
    ple_data.question_asset_delivery, ple_data.course_banner_delivery,
    ple_data.course_object_delivery FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.require_exact_available_object_delivery_owner() FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner, ple_audit_owner;
GRANT REFERENCES ON TABLE ple_data.object_delivery TO ple_private_owner, ple_audit_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.object_storage_check (
    object_storage_check_id uuid PRIMARY KEY,
    delivery_id uuid NOT NULL UNIQUE REFERENCES ple_data.object_delivery (delivery_id),
    expected_sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(expected_sha256) = 32),
    check_result text NOT NULL CHECK (check_result IN ('verified', 'missing', 'mismatched')),
    checked_at timestamp with time zone NOT NULL
);
CREATE TABLE ple_private.object_cleanup_manifest (
    object_cleanup_manifest_id uuid PRIMARY KEY,
    object_storage_check_id uuid NOT NULL REFERENCES ple_private.object_storage_check (object_storage_check_id),
    job_id uuid NOT NULL UNIQUE REFERENCES ple_private.job (job_id),
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
    delivery_id uuid NOT NULL REFERENCES ple_data.object_delivery (delivery_id),
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    access_decision text NOT NULL,
    accessed_at timestamp with time zone NOT NULL,
    CONSTRAINT object_delivery_access_event_decision_is_closed
        CHECK (access_decision IN ('allowed', 'denied')),
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
