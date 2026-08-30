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
    assigned_instructor_user_id uuid NOT NULL,
    assigned_instructor_role text NOT NULL DEFAULT 'instructor' CHECK (assigned_instructor_role = 'instructor'),
    created_at timestamp with time zone NOT NULL,
    delivery_time_zone text NOT NULL CHECK (char_length(btrim(delivery_time_zone)) BETWEEN 1 AND 100),
    CONSTRAINT course_instance_assigned_instructor_role_matches FOREIGN KEY (
        assigned_instructor_user_id, assigned_instructor_role
    ) REFERENCES ple_private.account (user_id, role),
    CONSTRAINT course_instance_blueprint_revision_is_unique UNIQUE (course_id, blueprint_revision_id)
);
CREATE TABLE ple_data.course_instance_blueprint_adoption (
    adoption_id uuid PRIMARY KEY,
    course_id uuid NOT NULL UNIQUE REFERENCES ple_data.course_instance (course_id),
    blueprint_revision_id uuid NOT NULL REFERENCES ple_data.blueprint_course_revision (blueprint_revision_id),
    idempotency_key bytea NOT NULL UNIQUE CHECK (pg_catalog.octet_length(idempotency_key) = 32),
    adopted_at timestamp with time zone NOT NULL,
    evidence jsonb NOT NULL CHECK (jsonb_typeof(evidence) = 'object')
);
CREATE FUNCTION ple_data.reject_course_instance_adoption_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'a CourseInstance Blueprint adoption is immutable'; END
$$;
CREATE TRIGGER course_instance_adoption_is_immutable BEFORE UPDATE OR DELETE ON ple_data.course_instance_blueprint_adoption FOR EACH ROW EXECUTE FUNCTION ple_data.reject_course_instance_adoption_change();
ALTER TABLE ple_data.course_instance ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_instance FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_instance_blueprint_adoption ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_instance_blueprint_adoption FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.course_instance, ple_data.course_instance_blueprint_adoption FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_course_instance_adoption_change() FROM PUBLIC;
COMMENT ON TABLE ple_data.course_instance IS 'Private delivery aggregate bound to one immutable BlueprintCourse revision and accountable assigned Instructor; never blank.';
RESET ROLE;
