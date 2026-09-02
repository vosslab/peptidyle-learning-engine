-- SD1 shared answer-free published-question roots and immutable versions.

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.published_question (
    question_id text PRIMARY KEY,
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT published_question_id_is_crockford_shape CHECK (question_id ~ '^[0-9A-HJKMNP-TV-Z]{3,12}-[0-9A-HJKMNP-TV-Z]{4,12}$')
);
CREATE TABLE ple_data.question_revision (
    question_id text NOT NULL REFERENCES ple_data.published_question (question_id),
    revision_number integer NOT NULL CHECK (revision_number > 0),
    backend text NOT NULL CHECK (backend IN ('ple', 'webwork', 'qti', 'imathas')),
    published_at timestamp with time zone NOT NULL,
    public_metadata jsonb NOT NULL,
    question_description text GENERATED ALWAYS AS (public_metadata ->> 'questionDescription') STORED,
    PRIMARY KEY (question_id, revision_number),
    CONSTRAINT question_revision_metadata_is_object CHECK (jsonb_typeof(public_metadata) = 'object'),
    CONSTRAINT question_revision_metadata_has_question_description CHECK (
        jsonb_typeof(public_metadata -> 'questionDescription') = 'string'
        AND char_length(btrim(question_description)) BETWEEN 1 AND 4000
    )
);
CREATE FUNCTION ple_data.reject_question_revision_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514',
        MESSAGE = 'a published question revision is immutable';
END
$$;
CREATE TRIGGER question_revision_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_revision_change();
ALTER TABLE ple_data.published_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.published_question FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.published_question, ple_data.question_revision FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_question_revision_change() FROM PUBLIC;
COMMENT ON TABLE ple_data.published_question IS 'Shared stable human-facing QuestionId lineage root; no private draft or answer data.';
COMMENT ON COLUMN ple_data.question_revision.question_description IS 'Generated searchable Question Description derived from canonical answer-free public metadata; never Student-delivered by default.';
COMMENT ON TABLE ple_data.question_revision IS 'Immutable answer-free published version metadata including the required Instructor-facing Question Description; publication and current selection availability are separate event evidence.';
RESET ROLE;
