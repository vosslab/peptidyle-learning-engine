-- SD1 shared answer-free published-question roots and immutable versions.

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.published_question (
    question_id text PRIMARY KEY,
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT published_question_id_is_crockford_shape CHECK (question_id ~ '^[0-9A-HJKMNP-TV-Z]{3,12}-[0-9A-HJKMNP-TV-Z]{4,12}$')
);
CREATE TABLE ple_data.published_question_version (
    problem_id uuid PRIMARY KEY,
    version_id uuid NOT NULL UNIQUE,
    question_id text NOT NULL REFERENCES ple_data.published_question (question_id),
    backend text NOT NULL CHECK (backend IN ('native', 'webwork', 'qti', 'h5p', 'imathas')),
    lifecycle text NOT NULL CHECK (lifecycle IN ('published', 'deprecated', 'archived')),
    published_at timestamp with time zone NOT NULL,
    public_metadata jsonb NOT NULL,
    CONSTRAINT published_question_version_problem_version_is_unique UNIQUE (problem_id, version_id),
    CONSTRAINT published_question_version_metadata_is_object CHECK (jsonb_typeof(public_metadata) = 'object')
);
CREATE FUNCTION ple_data.reject_published_question_version_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514',
        MESSAGE = 'a published question version is immutable';
END
$$;
CREATE TRIGGER published_question_version_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.published_question_version
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_published_question_version_change();
ALTER TABLE ple_data.published_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.published_question FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.published_question_version ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.published_question_version FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.published_question, ple_data.published_question_version FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_published_question_version_change() FROM PUBLIC;
COMMENT ON TABLE ple_data.published_question IS 'Shared stable human-facing QuestionId lineage root; no private draft or answer data.';
COMMENT ON TABLE ple_data.published_question_version IS 'Immutable answer-free published version metadata; grading/source payloads remain private.';
RESET ROLE;
