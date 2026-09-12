-- Current Course Instance truth and immutable source history.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.course_instance (
    course_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE
        CHECK (reference_number BETWEEN 1 AND 2147483647),
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL CHECK (blueprint_revision_number > 0),
    assigned_instructor_account_id uuid NOT NULL,
    assigned_instructor_role text NOT NULL DEFAULT 'instructor'
        CHECK (assigned_instructor_role = 'instructor'),
    course_short_name text NOT NULL CHECK (
        course_short_name = btrim(course_short_name)
        AND char_length(course_short_name) BETWEEN 1 AND 200
    ),
    course_long_name text NOT NULL CHECK (
        course_long_name = btrim(course_long_name)
        AND char_length(course_long_name) BETWEEN 1 AND 200
    ),
    term_starts_on date NOT NULL,
    term_ends_on date NOT NULL CHECK (term_ends_on >= term_starts_on),
    course_theme text NOT NULL DEFAULT 'grass' CHECK (course_theme IN (
        'tundra', 'forest', 'desert', 'grass', 'arctic', 'ocean', 'tropical',
        'coral-reef', 'swamp', 'underground', 'salt-marsh', 'wetland',
        'sea-floor', 'magma', 'beach'
    )),
    created_at timestamp with time zone NOT NULL,
    FOREIGN KEY (assigned_instructor_account_id, assigned_instructor_role)
        REFERENCES ple_private.account (account_id, product_role),
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number)
);

CREATE TABLE ple_data.course_origin (
    course_origin_id uuid PRIMARY KEY,
    course_id uuid NOT NULL UNIQUE REFERENCES ple_data.course_instance (course_id),
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL CHECK (blueprint_revision_number > 0),
    source_course_id uuid REFERENCES ple_data.course_instance (course_id),
    created_at timestamp with time zone NOT NULL,
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number)
);

CREATE FUNCTION ple_data.reject_course_origin_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'a Course Origin is immutable';
END
$$;

CREATE TRIGGER course_origin_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.course_origin
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_course_origin_change();

ALTER TABLE ple_data.course_instance ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_instance FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_origin ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_origin FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_data.course_instance, ple_data.course_origin FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.reject_course_origin_change() FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;
GRANT SELECT, INSERT ON ple_data.course_instance, ple_data.course_origin TO ple_api_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner, ple_audit_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance TO ple_private_owner, ple_audit_owner;
CREATE POLICY course_instance_api_owner_access ON ple_data.course_instance
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_origin_api_owner_access ON ple_data.course_origin
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

RESET ROLE;

SET LOCAL ROLE ple_data_owner;
GRANT REFERENCES ON TABLE ple_data.blueprint_course_revision TO ple_audit_owner;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.course_instance_creation_event (
    course_instance_creation_event_id uuid PRIMARY KEY,
    course_id uuid NOT NULL UNIQUE REFERENCES ple_data.course_instance (course_id),
    course_reference_number bigint NOT NULL UNIQUE,
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL,
    assigned_instructor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    created_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    occurred_at timestamp with time zone NOT NULL,
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number)
);

CREATE FUNCTION ple_audit.reject_course_instance_creation_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_audit
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'a Course Instance Creation Event is immutable';
END
$$;

CREATE TRIGGER course_instance_creation_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.course_instance_creation_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_course_instance_creation_event_change();

ALTER TABLE ple_audit.course_instance_creation_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.course_instance_creation_event FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_audit.course_instance_creation_event FROM PUBLIC;
CREATE POLICY course_instance_creation_event_owner_write
    ON ple_audit.course_instance_creation_event FOR INSERT TO ple_audit_owner WITH CHECK (true);

CREATE FUNCTION ple_audit.record_course_instance_creation_event(
    p_event_id uuid, p_course_id uuid, p_reference bigint, p_blueprint_reference bigint,
    p_blueprint_revision bigint, p_assigned_instructor uuid, p_creator uuid,
    p_occurred_at timestamp with time zone
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_audit
AS $$
BEGIN
    IF p_event_id IS NULL OR p_course_id IS NULL OR p_reference NOT BETWEEN 1 AND 2147483647
       OR p_blueprint_reference NOT BETWEEN 1 AND 2147483647 OR p_blueprint_revision <= 0
       OR p_assigned_instructor IS NULL OR p_creator IS NULL OR p_occurred_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Instance Creation Event arguments are invalid';
    END IF;
    INSERT INTO ple_audit.course_instance_creation_event
    VALUES (p_event_id, p_course_id, p_reference, p_blueprint_reference, p_blueprint_revision,
            p_assigned_instructor, p_creator, p_occurred_at);
END
$$;

REVOKE ALL ON FUNCTION ple_audit.reject_course_instance_creation_event_change(),
    ple_audit.record_course_instance_creation_event(
        uuid, uuid, bigint, bigint, bigint, uuid, uuid, timestamp with time zone
    ) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_audit.record_course_instance_creation_event(
    uuid, uuid, bigint, bigint, bigint, uuid, uuid, timestamp with time zone
) TO ple_api_owner;

RESET ROLE;
