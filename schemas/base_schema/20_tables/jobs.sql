-- jobs tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_private_owner;

-- Public-asset publication uses a short-lived execution lease, not product
-- history. Its immutable target is one pending public-asset publication.
CREATE TABLE ple_private.job (
    job_id uuid PRIMARY KEY,
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL,
    payload jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(payload) = 'object'),
    state ple_data.job_state NOT NULL DEFAULT 'ready',
    available_at timestamptz NOT NULL,
    lease_token uuid,
    lease_expires_at timestamptz,
    attempt_count integer NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    max_attempts integer NOT NULL DEFAULT 3 CHECK (max_attempts BETWEEN 1 AND 20),
    completed_at timestamptz,
    created_at timestamptz NOT NULL,
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    CHECK (
        (state = 'ready' AND lease_token IS NULL AND lease_expires_at IS NULL
            AND completed_at IS NULL)
        OR (state = 'leased' AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL
            AND completed_at IS NULL)
        OR (state = 'completed' AND lease_token IS NULL AND lease_expires_at IS NULL
            AND completed_at IS NOT NULL)
    ),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);



SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.job IS 'role: current state, One immutable public Asset publication target with a bounded worker lease.';

SET LOCAL ROLE ple_private_owner;
COMMENT ON COLUMN ple_private.job.lease_token IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.job.lease_expires_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.job.completed_at IS 'NULL means this optional fact is absent.';

