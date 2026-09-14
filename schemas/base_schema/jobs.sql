-- Public-asset publication uses a short-lived execution lease, not product
-- history. Its immutable target is one pending public-asset publication.

SET LOCAL ROLE ple_private_owner;

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

CREATE FUNCTION ple_private.enforce_job_transition()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF ROW(
        NEW.job_id, NEW.job_kind, NEW.job_target_kind, NEW.question_id,
        NEW.revision_number, NEW.worker_kind,
        NEW.payload, NEW.max_attempts, NEW.created_at
    ) IS DISTINCT FROM ROW(
        OLD.job_id, OLD.job_kind, OLD.job_target_kind, OLD.question_id,
        OLD.revision_number, OLD.worker_kind,
        OLD.payload, OLD.max_attempts, OLD.created_at
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Job target and execution contract are immutable';
    END IF;

    -- ASVS 2.3.1, 2.3.3: only the closed public-asset lease edges below may
    -- mutate execution state.
    IF NOT (
        (OLD.state = 'ready' AND NEW.state = 'leased'
            AND NEW.attempt_count = OLD.attempt_count + 1
            AND NEW.attempt_count <= NEW.max_attempts
            AND NEW.available_at = OLD.available_at
            AND OLD.available_at <= pg_catalog.clock_timestamp())
        OR
        (OLD.state = 'leased' AND NEW.state = 'leased'
            AND NEW.attempt_count = OLD.attempt_count + 1
            AND NEW.attempt_count <= NEW.max_attempts
            AND NEW.available_at = OLD.available_at
            AND OLD.lease_expires_at <= pg_catalog.clock_timestamp())
        OR
        (OLD.state = 'leased' AND NEW.state = 'completed'
            AND NEW.attempt_count = OLD.attempt_count
            AND NEW.available_at = OLD.available_at)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Job transition is not allowed';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER job_state_is_controlled
BEFORE UPDATE ON ple_private.job
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_job_transition();

-- PostgreSQL delivers this transactional notification only after the Job's
-- ready state commits. The fixed channel and worker-kind payload disclose no
-- Student Work or lease capability (ASVS 8.3.1, 13.4.1).
CREATE FUNCTION ple_private.notify_job_ready()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.state = 'ready'
       AND (TG_OP = 'INSERT' OR OLD.state IS DISTINCT FROM 'ready') THEN
        PERFORM pg_catalog.pg_notify('ple_job_ready', NEW.worker_kind);
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER job_ready_notifies_workers
AFTER INSERT OR UPDATE OF state ON ple_private.job
FOR EACH ROW EXECUTE FUNCTION ple_private.notify_job_ready();

CREATE INDEX job_ready_claim_idx ON ple_private.job(available_at, job_id)
    WHERE state = 'ready';
CREATE INDEX job_expired_lease_idx ON ple_private.job(lease_expires_at, job_id)
    WHERE state = 'leased';

CREATE FUNCTION ple_private.enqueue_public_asset_publication(
    p_job_id uuid,
    p_question_id text,
    p_revision_number integer,
    p_payload jsonb,
    p_available_at timestamptz,
    p_max_attempts integer,
    p_created_at timestamptz
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF p_job_id IS NULL OR p_question_id IS NULL OR p_revision_number IS NULL
       OR jsonb_typeof(p_payload) <> 'object'
       OR p_available_at IS NULL OR p_created_at IS NULL
       OR p_max_attempts NOT BETWEEN 1 AND 20 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Public Asset Publication Job arguments are invalid';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_revision
         WHERE question_id = p_question_id AND revision_number = p_revision_number
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Public Asset Publication Job requires an exact Question Revision';
    END IF;
    IF EXISTS (SELECT 1 FROM ple_private.job WHERE job_id = p_job_id FOR UPDATE) THEN
        IF NOT EXISTS (
            SELECT 1 FROM ple_private.job AS job
             WHERE job.job_id = p_job_id
               AND job.job_kind = 'publish_public_assets'
               AND job.job_target_kind = 'public_asset_publication'
               AND job.question_id = p_question_id
               AND job.revision_number = p_revision_number
               AND job.worker_kind = 'public_asset_publisher'
               AND job.payload = p_payload
               AND job.available_at = p_available_at
               AND job.max_attempts = p_max_attempts
               AND job.created_at = p_created_at
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '23505',
                MESSAGE = 'Public Asset Publication Job replay differs';
        END IF;
        RETURN;
    END IF;

    INSERT INTO ple_private.job (
        job_id, job_kind, job_target_kind, question_id, revision_number, worker_kind,
        payload, available_at, max_attempts, created_at
    ) VALUES (
        p_job_id, 'publish_public_assets', 'public_asset_publication',
        p_question_id, p_revision_number, 'public_asset_publisher',
        p_payload, p_available_at, p_max_attempts, p_created_at
    );
END $$;

ALTER TABLE ple_private.job ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.job FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.job FROM PUBLIC;
CREATE POLICY job_private_owner_access ON ple_private.job
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
REVOKE ALL ON FUNCTION ple_private.enforce_job_transition(),
    ple_private.notify_job_ready(),
    ple_private.enqueue_public_asset_publication(uuid, text, integer, jsonb, timestamptz, integer, timestamptz)
    FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner, ple_public_asset_publisher;
GRANT EXECUTE ON FUNCTION ple_private.enqueue_public_asset_publication(
    uuid, text, integer, jsonb, timestamptz, integer, timestamptz
)
    TO ple_api_owner;

COMMENT ON TABLE ple_private.job IS
    'One immutable public Asset publication target with a bounded worker lease.';

RESET ROLE;
