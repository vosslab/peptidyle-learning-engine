-- SD1 append-only lifecycle evidence for immutable published question versions.

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.published_question_lifecycle_event (
    event_id uuid PRIMARY KEY,
    problem_id uuid NOT NULL REFERENCES ple_data.published_question_version (problem_id),
    lifecycle text NOT NULL CHECK (lifecycle IN ('published', 'deprecated', 'archived')),
    reason text,
    occurred_at timestamp with time zone NOT NULL,
    CONSTRAINT catalog_lifecycle_reason_matches_state CHECK (
        (lifecycle = 'published' AND reason IS NULL)
        OR (lifecycle IN ('deprecated', 'archived') AND char_length(btrim(reason)) BETWEEN 1 AND 1000)
    ),
    CONSTRAINT catalog_lifecycle_event_is_unique UNIQUE (problem_id, occurred_at)
);
CREATE FUNCTION ple_data.reject_published_question_lifecycle_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'a catalog lifecycle event is immutable';
END
$$;
CREATE TRIGGER published_question_lifecycle_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.published_question_lifecycle_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_published_question_lifecycle_event_change();
CREATE INDEX published_question_lifecycle_current_idx
    ON ple_data.published_question_lifecycle_event (problem_id, occurred_at DESC);
ALTER TABLE ple_data.published_question_lifecycle_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.published_question_lifecycle_event FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.published_question_lifecycle_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_published_question_lifecycle_event_change() FROM PUBLIC;
COMMENT ON TABLE ple_data.published_question_lifecycle_event IS
    'Append-only published/deprecated/archived evidence; immutable versions are never rewritten.';
RESET ROLE;
