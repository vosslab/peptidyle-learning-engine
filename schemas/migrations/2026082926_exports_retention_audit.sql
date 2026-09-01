-- SD1 export requests, retention plans, and immutable lifecycle evidence.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance, ple_data.assignment
    TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.assignment_export_request (
    assignment_export_id uuid PRIMARY KEY,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    manifest_object_id uuid NOT NULL,
    requested_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    requested_at timestamp with time zone NOT NULL,
    assignment_export_state text NOT NULL CHECK (assignment_export_state IN ('requested', 'completed', 'failed', 'cancelled')),
    CONSTRAINT assignment_export_request_assignment_matches FOREIGN KEY (course_id, assignment_id)
        REFERENCES ple_data.assignment (course_id, assignment_id),
    CONSTRAINT assignment_export_request_manifest_matches FOREIGN KEY (manifest_object_id, course_id)
        REFERENCES ple_private.course_object_reference (object_id, course_id),
    UNIQUE (assignment_export_id, course_id, assignment_id)
);
CREATE TABLE ple_private.assignment_export_artifact (
    assignment_export_id uuid NOT NULL REFERENCES ple_private.assignment_export_request (assignment_export_id),
    assignment_export_format text NOT NULL CHECK (assignment_export_format IN ('docx', 'pdf', 'qti', 'answer_key_package')),
    object_id uuid NOT NULL,
    sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(sha256) = 32),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (assignment_export_id, assignment_export_format),
    CONSTRAINT assignment_export_artifact_object_matches FOREIGN KEY (object_id)
        REFERENCES ple_private.course_object_reference (object_id)
);
CREATE TABLE ple_private.course_retention_plan_revision (
    course_retention_plan_revision_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    revision_number bigint NOT NULL CHECK (revision_number > 0),
    retention_action text NOT NULL CHECK (retention_action IN (
        'archive_student_records', 'delete_private_artifacts', 'purge_student_records'
    )),
    scheduled_for timestamp with time zone NOT NULL,
    retention_manifest_checksum bytea NOT NULL CHECK (pg_catalog.octet_length(retention_manifest_checksum) = 32),
    created_at timestamp with time zone NOT NULL,
    UNIQUE (course_id, revision_number),
    UNIQUE (course_retention_plan_revision_id, course_id),
    UNIQUE (course_retention_plan_revision_id, retention_action)
);
CREATE FUNCTION ple_private.reject_export_request_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.assignment_export_id IS DISTINCT FROM OLD.assignment_export_id
        OR NEW.course_id IS DISTINCT FROM OLD.course_id
        OR NEW.assignment_id IS DISTINCT FROM OLD.assignment_id
        OR NEW.manifest_object_id IS DISTINCT FROM OLD.manifest_object_id
        OR NEW.requested_by_account_id IS DISTINCT FROM OLD.requested_by_account_id
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
ALTER TABLE ple_private.course_retention_plan_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_retention_plan_revision FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.job
    ADD CONSTRAINT job_course_retention_plan_revision_matches
    FOREIGN KEY (course_retention_plan_revision_id, course_id)
    REFERENCES ple_private.course_retention_plan_revision (course_retention_plan_revision_id, course_id);
ALTER TABLE ple_private.job
    ADD CONSTRAINT job_assignment_export_matches
    FOREIGN KEY (assignment_export_id, course_id, assignment_id)
    REFERENCES ple_private.assignment_export_request (assignment_export_id, course_id, assignment_id);
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.course_retention_plan_revision TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.job TO ple_audit_owner;
REVOKE ALL PRIVILEGES ON TABLE ple_private.assignment_export_request,
    ple_private.assignment_export_artifact, ple_private.course_retention_plan_revision FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_export_request_change() FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.course_retention_event (
    course_retention_event_id uuid PRIMARY KEY,
    course_retention_plan_revision_id uuid NOT NULL,
    job_id uuid NOT NULL,
    retention_action text NOT NULL,
    job_result text NOT NULL CHECK (job_result IN ('completed', 'failed')),
    recorded_at timestamp with time zone NOT NULL,
    course_retention_event_checksum bytea NOT NULL CHECK (pg_catalog.octet_length(course_retention_event_checksum) = 32),
    FOREIGN KEY (course_retention_plan_revision_id, retention_action)
        REFERENCES ple_private.course_retention_plan_revision (course_retention_plan_revision_id, retention_action),
    FOREIGN KEY (job_id, course_retention_plan_revision_id)
        REFERENCES ple_private.job (job_id, course_retention_plan_revision_id),
    UNIQUE (course_retention_plan_revision_id, job_id, course_retention_event_checksum)
);
ALTER TABLE ple_audit.course_retention_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.course_retention_event FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.course_retention_event FROM PUBLIC;
RESET ROLE;
