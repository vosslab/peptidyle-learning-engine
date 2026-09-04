-- Blueprint Course roots and immutable revision evidence.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.blueprint_course (
    blueprint_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    blueprint_course_owner_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT blueprint_course_reference_is_bounded CHECK (
        reference_number > 0 AND reference_number <= 2147483647
    )
);
CREATE TABLE ple_data.blueprint_course_revision (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    blueprint_revision_number bigint NOT NULL CHECK (blueprint_revision_number > 0),
    title text NOT NULL CHECK (char_length(btrim(title)) BETWEEN 1 AND 500),
    blueprint_course_content jsonb NOT NULL CHECK (jsonb_typeof(blueprint_course_content) = 'object'),
    created_at timestamp with time zone NOT NULL,
    PRIMARY KEY (blueprint_course_reference_number, blueprint_revision_number)
);
CREATE FUNCTION ple_data.reject_blueprint_course_revision_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'a Blueprint Revision is immutable'; END
$$;
CREATE TRIGGER blueprint_course_revision_is_immutable BEFORE UPDATE OR DELETE ON ple_data.blueprint_course_revision FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_course_revision_change();
ALTER TABLE ple_data.blueprint_course ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_revision FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.blueprint_course, ple_data.blueprint_course_revision FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_blueprint_course_revision_change() FROM PUBLIC;
COMMENT ON TABLE ple_data.blueprint_course IS 'Reusable answer-free Blueprint Course aggregate; distinct from private Course Instance delivery.';
COMMENT ON TABLE ple_data.blueprint_course_revision IS 'Immutable Blueprint Revision identified by its exact Blueprint Course Reference and Blueprint Revision Number.';
RESET ROLE;
