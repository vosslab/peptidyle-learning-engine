-- Course Retention plans and immutable lifecycle evidence.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance
    TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
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
ALTER TABLE ple_private.course_retention_plan_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_retention_plan_revision FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.job
    ADD CONSTRAINT job_course_retention_plan_revision_matches
    FOREIGN KEY (course_retention_plan_revision_id, course_id)
    REFERENCES ple_private.course_retention_plan_revision (course_retention_plan_revision_id, course_id);
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.course_retention_plan_revision TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.job TO ple_audit_owner;
REVOKE ALL PRIVILEGES ON TABLE ple_private.course_retention_plan_revision FROM PUBLIC;
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
