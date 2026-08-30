-- SD1 export requests, retention plans, and immutable lifecycle evidence.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance, ple_data.course_instance_assignment_delivery
    TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.assignment_export_request (
    export_id uuid PRIMARY KEY,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    manifest_object_id uuid NOT NULL,
    requested_by_user_id uuid NOT NULL REFERENCES ple_private.account (user_id),
    requested_at timestamp with time zone NOT NULL,
    state text NOT NULL CHECK (state IN ('queued', 'ready', 'failed', 'cancelled')),
    CONSTRAINT assignment_export_request_assignment_matches FOREIGN KEY (course_id, assignment_id)
        REFERENCES ple_data.course_instance_assignment_delivery (course_id, assignment_id),
    CONSTRAINT assignment_export_request_manifest_matches FOREIGN KEY (manifest_object_id, course_id)
        REFERENCES ple_private.course_object_metadata (object_id, course_id)
);
CREATE TABLE ple_private.assignment_export_artifact (
    export_id uuid NOT NULL REFERENCES ple_private.assignment_export_request (export_id),
    artifact_kind text NOT NULL CHECK (artifact_kind IN ('docx', 'pdf', 'qti', 'answer_key')),
    object_id uuid NOT NULL,
    sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(sha256) = 32),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (export_id, artifact_kind),
    CONSTRAINT assignment_export_artifact_object_matches FOREIGN KEY (object_id)
        REFERENCES ple_private.course_object_metadata (object_id)
);
CREATE TABLE ple_private.course_retention_plan (
    retention_plan_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    stage text NOT NULL CHECK (stage IN ('archive', 'delete_private_artifacts', 'purge')),
    generation bigint NOT NULL CHECK (generation > 0),
    scheduled_at timestamp with time zone NOT NULL,
    state text NOT NULL CHECK (state IN ('scheduled', 'running', 'completed', 'cancelled')),
    UNIQUE (course_id, stage, generation)
);
CREATE FUNCTION ple_private.reject_export_request_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.course_id IS DISTINCT FROM OLD.course_id
        OR NEW.assignment_id IS DISTINCT FROM OLD.assignment_id
        OR NEW.manifest_object_id IS DISTINCT FROM OLD.manifest_object_id
        OR NEW.requested_by_user_id IS DISTINCT FROM OLD.requested_by_user_id
        OR NEW.requested_at IS DISTINCT FROM OLD.requested_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'an export request parent is immutable';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER assignment_export_request_parent_is_immutable
BEFORE UPDATE ON ple_private.assignment_export_request
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_export_request_change();
ALTER TABLE ple_private.assignment_export_request ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_export_request FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_export_artifact ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_export_artifact FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_retention_plan ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_retention_plan FORCE ROW LEVEL SECURITY;
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.course_retention_plan TO ple_audit_owner;
REVOKE ALL PRIVILEGES ON TABLE ple_private.assignment_export_request,
    ple_private.assignment_export_artifact, ple_private.course_retention_plan FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_export_request_change() FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.retention_lifecycle_event (
    event_id uuid PRIMARY KEY,
    retention_plan_id uuid NOT NULL REFERENCES ple_private.course_retention_plan (retention_plan_id),
    recorded_at timestamp with time zone NOT NULL,
    event_kind text NOT NULL CHECK (event_kind IN ('scheduled', 'claimed', 'completed', 'cancelled', 'failed')),
    digest bytea NOT NULL CHECK (pg_catalog.octet_length(digest) = 32),
    UNIQUE (retention_plan_id, event_kind, digest)
);
ALTER TABLE ple_audit.retention_lifecycle_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.retention_lifecycle_event FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.retention_lifecycle_event FROM PUBLIC;
RESET ROLE;
