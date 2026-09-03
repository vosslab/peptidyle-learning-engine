-- Course Object References and server-only Object Addresses.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance TO ple_private_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.course_object_reference (
    object_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    object_checksum bytea NOT NULL CHECK (pg_catalog.octet_length(object_checksum) = 32),
    media_type text NOT NULL CHECK (char_length(btrim(media_type)) BETWEEN 1 AND 200),
    byte_length bigint NOT NULL CHECK (byte_length >= 0),
    created_at timestamp with time zone NOT NULL,
    deleted_at timestamp with time zone,
    CONSTRAINT course_object_reference_object_course_is_unique UNIQUE (object_id, course_id),
    CONSTRAINT course_object_deletion_is_ordered CHECK (deleted_at IS NULL OR deleted_at >= created_at)
);
ALTER TABLE ple_private.course_object_reference ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_object_reference FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.course_object_reference FROM PUBLIC;
COMMENT ON TABLE ple_private.course_object_reference IS 'One private Course Instance relationship to immutable object bytes; Object Address and delivery credentials are never browser-visible.';
RESET ROLE;
