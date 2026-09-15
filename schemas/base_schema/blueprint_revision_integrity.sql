-- Immutable Blueprint Revision and lifecycle-event guards. Installed after
-- blueprints.sql so every guarded table already exists.

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.reject_blueprint_revision_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'a Blueprint Revision is immutable';
END
$$;

CREATE FUNCTION ple_data.reject_blueprint_event_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Blueprint lifecycle evidence is immutable';
END
$$;

CREATE FUNCTION ple_data.reject_sealed_blueprint_revision_child_insert()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    PERFORM 1 FROM ple_data.blueprint_course_revision AS revision
     WHERE revision.blueprint_course_reference_number = NEW.blueprint_course_reference_number
       AND revision.blueprint_revision_number = NEW.blueprint_revision_number
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23503',
            MESSAGE = 'Blueprint Revision parent is unavailable for child insertion';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_data.blueprint_revision_event AS event
         WHERE event.blueprint_course_reference_number = NEW.blueprint_course_reference_number
           AND event.blueprint_revision_number = NEW.blueprint_revision_number
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'a Blueprint Revision is immutable';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER blueprint_course_revision_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_course_revision
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_revision_change();
CREATE TRIGGER blueprint_revision_question_pin_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_revision_question_pin
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_revision_change();
CREATE TRIGGER blueprint_revision_question_pin_is_sealed
BEFORE INSERT ON ple_data.blueprint_revision_question_pin
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_sealed_blueprint_revision_child_insert();
CREATE TRIGGER blueprint_revision_assessment_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_revision_assessment
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_revision_change();
CREATE TRIGGER blueprint_revision_assessment_is_sealed
BEFORE INSERT ON ple_data.blueprint_revision_assessment
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_sealed_blueprint_revision_child_insert();
CREATE TRIGGER blueprint_revision_module_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_revision_module
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_revision_change();
CREATE TRIGGER blueprint_revision_module_is_sealed
BEFORE INSERT ON ple_data.blueprint_revision_module
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_sealed_blueprint_revision_child_insert();
CREATE TRIGGER blueprint_revision_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_revision_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_event_change();
CREATE TRIGGER blueprint_metadata_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_metadata_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_event_change();

REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_blueprint_revision_change(),
    ple_data.reject_blueprint_event_change(),
    ple_data.reject_sealed_blueprint_revision_child_insert() FROM PUBLIC;

RESET ROLE;
