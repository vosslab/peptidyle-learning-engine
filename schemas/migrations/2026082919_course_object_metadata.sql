-- SD1 private course-object metadata scopes; object keys are server-only.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance, ple_data.course_student TO ple_private_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.course_object_metadata (
    object_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    scope text NOT NULL CHECK (scope IN ('student_upload', 'student_artifact', 'course_export', 'protected_feedback')),
    owner_student_id uuid REFERENCES ple_data.course_student (student_id),
    sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(sha256) = 32),
    media_type text NOT NULL CHECK (char_length(btrim(media_type)) BETWEEN 1 AND 200),
    byte_length bigint NOT NULL CHECK (byte_length >= 0),
    created_at timestamp with time zone NOT NULL,
    deleted_at timestamp with time zone,
    CONSTRAINT course_object_metadata_object_course_is_unique UNIQUE (object_id, course_id),
    CONSTRAINT course_object_deletion_is_ordered CHECK (deleted_at IS NULL OR deleted_at >= created_at)
);
ALTER TABLE ple_private.course_object_metadata ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_object_metadata FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.course_object_metadata FROM PUBLIC;
COMMENT ON TABLE ple_private.course_object_metadata IS 'Private object metadata; storage location and delivery credentials are never stored in browser-visible fields.';
RESET ROLE;
