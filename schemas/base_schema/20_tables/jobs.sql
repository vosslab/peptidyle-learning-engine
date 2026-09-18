-- jobs tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_private_owner;

-- Public-asset publication uses a short-lived execution lease, not product
-- history. Its immutable target is one pending public-asset publication.
CREATE TABLE ple_private.job (
    job_id uuid PRIMARY KEY,
    job_kind text NOT NULL CHECK (job_kind = 'publish_public_assets'),
    job_target_kind text NOT NULL CHECK (job_target_kind = 'public_asset_publication'),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    worker_kind text NOT NULL CHECK (worker_kind = 'public_asset_publisher'),
    payload jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(payload) = 'object'),
    state text NOT NULL DEFAULT 'ready' CHECK (state IN ('ready', 'leased', 'completed')),
    available_at timestamptz NOT NULL,
    lease_token uuid,
    lease_expires_at timestamptz,
    attempt_count integer NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    max_attempts integer NOT NULL DEFAULT 3 CHECK (max_attempts BETWEEN 1 AND 20),
    completed_at timestamptz,
    created_at timestamptz NOT NULL,
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    CHECK (
        (state = 'ready' AND lease_token IS NULL AND lease_expires_at IS NULL
            AND completed_at IS NULL)
        OR (state = 'leased' AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL
            AND completed_at IS NULL)
        OR (state = 'completed' AND lease_token IS NULL AND lease_expires_at IS NULL
            AND completed_at IS NOT NULL)
    )
);

COMMENT ON TABLE ple_private.job IS 'role: current state, One immutable public Asset publication target with a bounded worker lease.';

