-- MOD-STO: immutable, server-only source binding for every source-backed
-- published version. The row is inserted in the same transaction as the
-- problem version and payload; source bytes remain object-store authoritative.

CREATE TABLE published_source_artifact (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    backend text NOT NULL
        CHECK (backend IN ('webwork', 'qti', 'h5p', 'imathas')),
    object_id uuid NOT NULL UNIQUE,
    payload jsonb NOT NULL
        CHECK (jsonb_typeof(payload) = 'object'),
    payload_sha256 character(64) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (problem_id, version_id),
    FOREIGN KEY (problem_id, version_id)
        REFERENCES problem_version(problem_id, version_id)
        ON DELETE RESTRICT
        DEFERRABLE INITIALLY DEFERRED
);

ALTER TABLE published_source_artifact ENABLE ROW LEVEL SECURITY;
ALTER TABLE published_source_artifact FORCE ROW LEVEL SECURITY;
CREATE POLICY published_source_artifact_visible_select ON published_source_artifact
    FOR SELECT TO ple_app
    USING (
        EXISTS (
            SELECT 1 FROM problem_version AS visible_version
            WHERE visible_version.problem_id = published_source_artifact.problem_id
              AND visible_version.version_id = published_source_artifact.version_id
        )
    );
CREATE POLICY published_source_artifact_app_insert ON published_source_artifact
    FOR INSERT TO ple_app
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM problem_version AS visible_version
            WHERE visible_version.problem_id = published_source_artifact.problem_id
              AND visible_version.version_id = published_source_artifact.version_id
        )
    );

GRANT SELECT, INSERT ON published_source_artifact TO ple_app;
