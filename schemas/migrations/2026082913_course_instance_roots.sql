-- SD1 CourseInstance roots bind immutable reusable Blueprint evidence to delivery.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.course_instance (
    course_id uuid PRIMARY KEY,
    blueprint_id uuid NOT NULL REFERENCES ple_data.blueprint_course (blueprint_id),
    blueprint_revision_id uuid NOT NULL REFERENCES ple_data.blueprint_course_revision (blueprint_revision_id),
    assigned_instructor_account_id uuid NOT NULL,
    assigned_instructor_role text NOT NULL DEFAULT 'instructor' CHECK (assigned_instructor_role = 'instructor'),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT course_instance_assigned_instructor_role_matches FOREIGN KEY (
        assigned_instructor_account_id, assigned_instructor_role
    ) REFERENCES ple_private.account (account_id, role),
    CONSTRAINT course_instance_blueprint_revision_is_unique UNIQUE (course_id, blueprint_revision_id)
);
CREATE TABLE ple_data.course_origin (
    course_origin_id uuid PRIMARY KEY,
    course_id uuid NOT NULL UNIQUE REFERENCES ple_data.course_instance (course_id),
    blueprint_revision_id uuid NOT NULL REFERENCES ple_data.blueprint_course_revision (blueprint_revision_id),
    source_course_id uuid REFERENCES ple_data.course_instance (course_id),
    idempotency_key bytea NOT NULL UNIQUE CHECK (pg_catalog.octet_length(idempotency_key) = 32),
    created_at timestamp with time zone NOT NULL,
    evidence jsonb NOT NULL CHECK (jsonb_typeof(evidence) = 'object')
);
CREATE FUNCTION ple_data.reject_course_origin_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'a Course Origin is immutable'; END
$$;
CREATE TRIGGER course_origin_is_immutable BEFORE UPDATE OR DELETE ON ple_data.course_origin FOR EACH ROW EXECUTE FUNCTION ple_data.reject_course_origin_change();
ALTER TABLE ple_data.course_instance ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_instance FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_origin ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_origin FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.course_instance, ple_data.course_origin FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_course_origin_change() FROM PUBLIC;
COMMENT ON TABLE ple_data.course_instance IS 'Private delivery aggregate bound to one immutable BlueprintCourse revision and accountable assigned Instructor; never blank.';
RESET ROLE;
