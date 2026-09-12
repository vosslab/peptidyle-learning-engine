-- The two durable asynchronous targets.  A Job is a short-lived execution
-- lease, not product history: its immutable target is either one accepted
-- Student Submission or one pending public-asset publication.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.job (
    job_id uuid PRIMARY KEY,
    job_kind text NOT NULL CHECK (job_kind IN (
        'grade_accepted_submission', 'publish_public_assets'
    )),
    job_target_kind text NOT NULL CHECK (job_target_kind IN (
        'question_submission', 'public_asset_publication'
    )),
    question_submission_id uuid UNIQUE REFERENCES ple_private.question_submission(submission_id)
        ON DELETE CASCADE,
    question_id text,
    revision_number integer,
    worker_kind text NOT NULL CHECK (worker_kind IN (
        'native_ple_grading', 'webwork_grading',
        'imathas_question_backend_grading', 'public_asset_publisher'
    )),
    generation bigint NOT NULL DEFAULT 1 CHECK (generation = 1),
    payload jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(payload) = 'object'),
    state text NOT NULL DEFAULT 'ready' CHECK (state IN ('ready', 'leased', 'completed', 'failed')),
    available_at timestamptz NOT NULL,
    lease_token uuid,
    lease_expires_at timestamptz,
    attempt_count integer NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    max_attempts integer NOT NULL DEFAULT 3 CHECK (max_attempts BETWEEN 1 AND 20),
    completed_at timestamptz,
    failure_kind text CHECK (failure_kind IN ('retryable', 'final', 'timed_out')),
    created_at timestamptz NOT NULL,
    UNIQUE (job_id, question_submission_id),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    CHECK (
        (job_kind = 'grade_accepted_submission'
            AND job_target_kind = 'question_submission'
            AND question_submission_id IS NOT NULL
            AND question_id IS NULL AND revision_number IS NULL
            AND worker_kind IN (
                'native_ple_grading', 'webwork_grading',
                'imathas_question_backend_grading'
            ))
        OR
        (job_kind = 'publish_public_assets'
            AND job_target_kind = 'public_asset_publication'
            AND question_submission_id IS NULL
            AND question_id IS NOT NULL AND revision_number IS NOT NULL
            AND worker_kind = 'public_asset_publisher')
    ),
    CHECK (
        (state = 'ready' AND lease_token IS NULL AND lease_expires_at IS NULL
            AND completed_at IS NULL AND failure_kind IS NULL)
        OR (state = 'leased' AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL
            AND completed_at IS NULL AND failure_kind IS NULL)
        OR (state = 'completed' AND lease_token IS NULL AND lease_expires_at IS NULL
            AND completed_at IS NOT NULL AND failure_kind IS NULL)
        OR (state = 'failed' AND lease_token IS NULL AND lease_expires_at IS NULL
            AND completed_at IS NOT NULL AND failure_kind IS NOT NULL)
    )
);

CREATE FUNCTION ple_private.enforce_job_transition()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF ROW(
        NEW.job_id, NEW.job_kind, NEW.job_target_kind, NEW.question_submission_id,
        NEW.question_id, NEW.revision_number, NEW.worker_kind, NEW.generation,
        NEW.payload, NEW.max_attempts, NEW.created_at
    ) IS DISTINCT FROM ROW(
        OLD.job_id, OLD.job_kind, OLD.job_target_kind, OLD.question_submission_id,
        OLD.question_id, OLD.revision_number, OLD.worker_kind, OLD.generation,
        OLD.payload, OLD.max_attempts, OLD.created_at
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Job target and execution contract are immutable';
    END IF;

    IF OLD.state IN ('completed', 'failed')
       OR NEW.attempt_count < OLD.attempt_count
       OR NEW.attempt_count > OLD.attempt_count + 1 THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Job state is forward-only';
    END IF;

    IF NEW.state = 'leased'
       AND (NEW.attempt_count <> OLD.attempt_count + 1
            OR NEW.attempt_count > NEW.max_attempts
            OR OLD.state NOT IN ('ready', 'leased')
            OR (OLD.state = 'ready' AND OLD.available_at > pg_catalog.clock_timestamp())
            OR (OLD.state = 'leased' AND OLD.lease_expires_at > pg_catalog.clock_timestamp())) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Job lease is not claimable';
    END IF;

    IF NEW.state = 'completed'
       AND (OLD.state <> 'leased' OR NEW.attempt_count <> OLD.attempt_count) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Only a leased Job may complete';
    END IF;

    IF NEW.state = 'failed'
       AND (OLD.state <> 'leased' OR NEW.attempt_count <> OLD.attempt_count) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Only a leased Job may fail';
    END IF;

    IF NEW.state = 'ready' AND OLD.state = 'leased'
       AND NEW.attempt_count <> OLD.attempt_count THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'A retry keeps its recorded claim count';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER job_state_is_controlled
BEFORE UPDATE ON ple_private.job
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_job_transition();

CREATE INDEX job_ready_claim_idx ON ple_private.job(available_at, job_id)
    WHERE state = 'ready';
CREATE INDEX job_expired_lease_idx ON ple_private.job(lease_expires_at, job_id)
    WHERE state = 'leased';

-- Delivery and publication procedures call these owner-only helpers after
-- they validate their own backend or object side effects.  Keeping target
-- selection here prevents a new generic Job branch from appearing by accident.
CREATE FUNCTION ple_private.enqueue_grade_accepted_submission(
    p_job_id uuid,
    p_question_submission_grading_id uuid,
    p_submission_id uuid,
    p_worker_kind text,
    p_payload jsonb,
    p_available_at timestamptz,
    p_max_attempts integer,
    p_created_at timestamptz
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE assignment_id uuid;
DECLARE locked_assignment_attempt_id uuid;
BEGIN
    IF p_job_id IS NULL OR p_question_submission_grading_id IS NULL OR p_submission_id IS NULL
       OR p_worker_kind NOT IN (
           'native_ple_grading', 'webwork_grading',
           'imathas_question_backend_grading'
       )
       OR jsonb_typeof(p_payload) <> 'object'
       OR p_available_at IS NULL OR p_created_at IS NULL
       OR p_max_attempts NOT BETWEEN 1 AND 20 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Grade Job arguments are invalid';
    END IF;

    -- Assignment, Attempt, Question Attempt, Submission: this is the same
    -- root-first order used by start, save, submit, grading, and Unrelease.
    SELECT assignment.assignment_id, assignment_attempt.assignment_attempt_id
      INTO assignment_id, locked_assignment_attempt_id
      FROM ple_private.question_submission AS submission
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_attempt_id = issued.assignment_attempt_id
      JOIN ple_data.assignment AS assignment
        ON assignment.assignment_id = assignment_attempt.assignment_id
     WHERE submission.submission_id = p_submission_id
     FOR UPDATE OF assignment;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Grade Job requires an accepted Submission';
    END IF;
    PERFORM 1 FROM ple_private.assignment_attempt
     WHERE assignment_attempt_id = locked_assignment_attempt_id FOR UPDATE;
    PERFORM 1 FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.question_submission AS submission
        ON submission.question_attempt_id = question_attempt.question_attempt_id
     WHERE submission.submission_id = p_submission_id
     FOR UPDATE OF question_attempt;
    PERFORM 1 FROM ple_private.question_submission
     WHERE submission_id = p_submission_id FOR UPDATE;

    -- Submission acceptance can be retried after its response is lost.  The
    -- immutable IDs are the idempotency key, so a replay either returns the
    -- same pending contract or fails before it can alter Student Work.
    IF EXISTS (
        SELECT 1 FROM ple_private.job
         WHERE question_submission_id = p_submission_id
         FOR UPDATE
    ) THEN
        IF NOT EXISTS (
            SELECT 1 FROM ple_private.job AS job
             WHERE job.question_submission_id = p_submission_id
               AND job.job_id = p_job_id
               AND job.job_kind = 'grade_accepted_submission'
               AND job.job_target_kind = 'question_submission'
               AND job.worker_kind = p_worker_kind
               AND job.payload = p_payload
               AND job.available_at = p_available_at
               AND job.max_attempts = p_max_attempts
               AND job.created_at = p_created_at
        ) OR NOT EXISTS (
            SELECT 1 FROM ple_private.question_submission_grading AS grading
             WHERE grading.submission_id = p_submission_id
               AND grading.question_submission_grading_id = p_question_submission_grading_id
               AND grading.job_id = p_job_id
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '23505',
                MESSAGE = 'Grade Job replay differs from the accepted Submission';
        END IF;
        RETURN;
    END IF;

    INSERT INTO ple_private.job (
        job_id, job_kind, job_target_kind, question_submission_id, worker_kind,
        payload, available_at, max_attempts, created_at
    ) VALUES (
        p_job_id, 'grade_accepted_submission', 'question_submission', p_submission_id,
        p_worker_kind, p_payload, p_available_at, p_max_attempts, p_created_at
    );
    INSERT INTO ple_private.question_submission_grading (
        question_submission_grading_id, submission_id, job_id, grading_state, created_at
    ) VALUES (
        p_question_submission_grading_id, p_submission_id, p_job_id, 'pending', p_created_at
    );
END $$;

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
    ple_private.enqueue_grade_accepted_submission(uuid, uuid, uuid, text, jsonb, timestamptz, integer, timestamptz),
    ple_private.enqueue_public_asset_publication(uuid, text, integer, jsonb, timestamptz, integer, timestamptz)
    FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner, ple_public_asset_publisher,
    ple_native_ple_grading_worker, ple_webwork_grading_worker,
    ple_imathas_question_backend_grading_worker;
GRANT EXECUTE ON FUNCTION
    ple_private.enqueue_grade_accepted_submission(uuid, uuid, uuid, text, jsonb, timestamptz, integer, timestamptz),
    ple_private.enqueue_public_asset_publication(uuid, text, integer, jsonb, timestamptz, integer, timestamptz)
    TO ple_api_owner;

COMMENT ON TABLE ple_private.job IS
    'Exactly two typed, immutable Job targets: accepted Submission grading and public Asset publication.';

RESET ROLE;
