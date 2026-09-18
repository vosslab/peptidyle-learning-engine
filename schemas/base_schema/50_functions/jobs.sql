-- Functions, triggers, and views from jobs.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.enforce_job_transition()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF ROW(
        NEW.job_id, NEW.job_kind, NEW.job_target_kind, NEW.published_question_id,
        NEW.revision_number, NEW.worker_kind,
        NEW.payload, NEW.max_attempts, NEW.created_at
    ) IS DISTINCT FROM ROW(
        OLD.job_id, OLD.job_kind, OLD.job_target_kind, OLD.published_question_id,
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

CREATE FUNCTION ple_private.enqueue_public_asset_publication(
    p_job_id uuid,
    p_published_question_id text,
    p_revision_number integer,
    p_payload jsonb,
    p_available_at timestamptz,
    p_max_attempts integer,
    p_created_at timestamptz
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF p_job_id IS NULL OR p_published_question_id IS NULL OR p_revision_number IS NULL
       OR jsonb_typeof(p_payload) <> 'object'
       OR p_available_at IS NULL OR p_created_at IS NULL
       OR p_max_attempts NOT BETWEEN 1 AND 20 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Public Asset Publication Job arguments are invalid';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.question_revision
         WHERE published_question_id = p_published_question_id AND revision_number = p_revision_number
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
               AND job.published_question_id = p_published_question_id
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
        job_id, job_kind, job_target_kind, published_question_id, revision_number, worker_kind,
        payload, available_at, max_attempts, created_at
    ) VALUES (
        p_job_id, 'publish_public_assets', 'public_asset_publication',
        p_published_question_id, p_revision_number, 'public_asset_publisher',
        p_payload, p_available_at, p_max_attempts, p_created_at
    );
END $$;

