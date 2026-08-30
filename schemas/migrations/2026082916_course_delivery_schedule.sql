-- SD1 CourseInstance delivery schedule, release, and local-divergence roots.

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.course_instance_assignment_delivery (
    delivery_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    assignment_id uuid NOT NULL,
    source_blueprint_revision_id uuid NOT NULL REFERENCES ple_data.blueprint_course_revision (blueprint_revision_id),
    released_at timestamp with time zone,
    available_at timestamp with time zone,
    due_at timestamp with time zone,
    closes_at timestamp with time zone,
    local_override jsonb,
    CONSTRAINT course_delivery_is_unique UNIQUE (course_id, assignment_id),
    CONSTRAINT course_delivery_schedule_is_ordered CHECK (
        (available_at IS NULL OR due_at IS NULL OR available_at <= due_at)
        AND (due_at IS NULL OR closes_at IS NULL OR due_at <= closes_at)
    ),
    CONSTRAINT course_delivery_override_is_object CHECK (local_override IS NULL OR jsonb_typeof(local_override) = 'object')
);
ALTER TABLE ple_data.course_instance_assignment_delivery ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_instance_assignment_delivery FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.course_instance_assignment_delivery FROM PUBLIC;
COMMENT ON TABLE ple_data.course_instance_assignment_delivery IS 'Private CourseInstance release, resolved schedule, and explicit local-delivery divergence.';
RESET ROLE;
