-- Question publication and availability events.
-- ASVS 2.1.1 and 2.3.1: the stored event shape and transition order express
-- the complete Question Revision availability workflow.

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.question_publication_event (
    event_id uuid PRIMARY KEY,
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    published_at timestamp with time zone NOT NULL,
    CONSTRAINT question_publication_event_version_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT question_publication_event_version_is_unique UNIQUE (question_id, revision_number)
);
CREATE TABLE ple_data.question_revision_availability_event (
    event_id uuid PRIMARY KEY,
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    availability text NOT NULL CHECK (availability IN ('available', 'archived')),
    reason text,
    occurred_at timestamp with time zone NOT NULL,
    CONSTRAINT question_revision_availability_reason_matches_event CHECK (
        (availability = 'available' AND reason IS NULL)
        OR (availability = 'archived' AND char_length(btrim(reason)) BETWEEN 1 AND 1000)
    ),
    CONSTRAINT question_revision_availability_event_version_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT question_revision_availability_event_kind_is_unique UNIQUE (question_id, revision_number, availability)
);
CREATE FUNCTION ple_data.reject_question_publication_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'a Question Publication Event is immutable';
END
$$;
CREATE TRIGGER question_publication_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_publication_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_publication_event_change();
CREATE TRIGGER question_revision_availability_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision_availability_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_publication_event_change();
CREATE FUNCTION ple_data.validate_question_revision_availability_event()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(NEW.question_id || ':' || NEW.revision_number::text, 0)
    );
    IF NEW.availability = 'archived' AND NOT EXISTS (
        SELECT 1 FROM ple_data.question_revision_availability_event AS available_event
        WHERE available_event.question_id = NEW.question_id
          AND available_event.revision_number = NEW.revision_number
          AND available_event.availability = 'available'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'a Question Revision must be Available before it is Archived';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER question_revision_availability_event_has_valid_transition
BEFORE INSERT ON ple_data.question_revision_availability_event
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_revision_availability_event();
CREATE INDEX question_revision_availability_current_idx
    ON ple_data.question_revision_availability_event (question_id, revision_number, occurred_at DESC);
ALTER TABLE ple_data.question_publication_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_publication_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_availability_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_availability_event FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.question_publication_event, ple_data.question_revision_availability_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_question_publication_event_change(),
    ple_data.validate_question_revision_availability_event() FROM PUBLIC;
COMMENT ON TABLE ple_data.question_publication_event IS 'One immutable publication event for one Question Revision.';
COMMENT ON TABLE ple_data.question_revision_availability_event IS 'Append-only Available or Archived selection evidence for one published Question Revision.';
RESET ROLE;
