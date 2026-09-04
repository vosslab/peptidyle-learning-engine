-- Durable Question Publication Event completeness invariant. A future atomic
-- publication operation supplies immutable Question Revision records directly;
-- this migration intentionally supplies no draft-to-published copy workflow.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.question_revision_has_question_source_binding(
    p_question_id text,
    p_revision_number integer
)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ple_private.question_revision_source_binding AS binding
         WHERE binding.question_id = p_question_id
           AND binding.revision_number = p_revision_number
    )
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.question_revision_has_question_source_binding(text, integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.question_revision_has_question_source_binding(text, integer) TO ple_data_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
CREATE FUNCTION ple_data.validate_question_publication_has_question_source_binding()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT ple_private.question_revision_has_question_source_binding(NEW.question_id, NEW.revision_number) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Publication requires an exact Question Revision Question Source Binding';
    END IF;
    RETURN NEW;
END
$$;
CREATE CONSTRAINT TRIGGER question_publication_event_has_question_source_binding
AFTER INSERT ON ple_data.question_publication_event
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_publication_has_question_source_binding();
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.validate_question_publication_has_question_source_binding() FROM PUBLIC;
COMMENT ON FUNCTION ple_data.validate_question_publication_has_question_source_binding() IS
    'Requires each Question Publication Event to commit with an exact Question Revision-owned Question Source Binding.';
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
COMMENT ON FUNCTION ple_private.question_revision_has_question_source_binding(text, integer) IS
    'Question Revision-owned Question Source Binding-existence predicate used by the Question Publication Event integrity trigger.';
RESET ROLE;
