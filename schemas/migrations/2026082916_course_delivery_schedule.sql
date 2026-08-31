-- SD1 CourseInstance delivery schedule, release, and local-divergence roots.

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.assignment (
    assignment_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    source_blueprint_revision_id uuid NOT NULL REFERENCES ple_data.blueprint_course_revision (blueprint_revision_id),
    created_at timestamp with time zone NOT NULL,
    released_at timestamp with time zone,
    available_at timestamp with time zone,
    due_at timestamp with time zone,
    closes_at timestamp with time zone,
    local_override jsonb,
    CONSTRAINT assignment_course_is_unique UNIQUE (course_id, assignment_id),
    CONSTRAINT assignment_schedule_is_ordered CHECK (
        (available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
        AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)
    ),
    CONSTRAINT assignment_override_is_object CHECK (local_override IS NULL OR jsonb_typeof(local_override) = 'object')
);
-- ASVS 8.2.2: authored delivery meaning is immutable once referenced by
-- Student activity; later edits create a distinct complete revision.
CREATE TABLE ple_data.assignment_revision (
    assignment_revision_id uuid PRIMARY KEY,
    assignment_id uuid NOT NULL REFERENCES ple_data.assignment (assignment_id),
    revision_number integer NOT NULL CHECK (revision_number > 0),
    authored_definition jsonb NOT NULL CHECK (jsonb_typeof(authored_definition) = 'object'),
    created_at timestamp with time zone NOT NULL,
    published_at timestamp with time zone,
    UNIQUE (assignment_id, revision_number),
    UNIQUE (assignment_id, assignment_revision_id),
    CONSTRAINT assignment_revision_publication_is_ordered
        CHECK (published_at IS NULL OR published_at >= created_at)
);
CREATE FUNCTION ple_data.reject_assignment_revision_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Assignment Revision is immutable';
END
$$;
CREATE TRIGGER assignment_revision_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.assignment_revision
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_assignment_revision_change();
ALTER TABLE ple_data.assignment ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.assignment FROM PUBLIC;
REVOKE ALL PRIVILEGES ON TABLE ple_data.assignment_revision FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_assignment_revision_change() FROM PUBLIC;
COMMENT ON TABLE ple_data.assignment IS 'Course Instance-owned Assignment definition, release, schedule, and controlled local divergence.';
RESET ROLE;
