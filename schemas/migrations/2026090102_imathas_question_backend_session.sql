-- Durable iMathAS Question Backend Session Store boundary.
-- This is a direct pre-production cutover.  The iMathAS Question Backend State is an AEAD
-- ciphertext triple, not a second serialized Session representation.

CREATE ROLE ple_imathas_question_backend_grading_worker
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
REVOKE ple_imathas_question_backend_grading_worker FROM ple_migrator;

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner, ple_api_owner;
GRANT REFERENCES ON TABLE ple_data.question_revision TO ple_private_owner;
GRANT SELECT ON TABLE ple_data.assignment, ple_data.question_revision,
    ple_data.student_record TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

-- Submission-state validation reads the paired immutable records while a
-- security-definer API procedure creates the marker Submission.
ALTER FUNCTION ple_private.enforce_question_attempt_submission_state()
    SECURITY DEFINER
    SET search_path = pg_catalog, ple_private;
ALTER FUNCTION ple_private.enforce_question_submission_attempt_state()
    SECURITY DEFINER
    SET search_path = pg_catalog, ple_private;
CREATE POLICY question_submission_attempt_state_private_owner_read
    ON ple_private.question_submission FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_attempt_submission_state_private_owner_read
    ON ple_private.question_attempt FOR SELECT TO ple_private_owner USING (true);

-- These records begin at their final pre-production shape.  Earlier migration
-- revisions were consolidated, so this migration owns the durable baseline.
CREATE TABLE ple_private.imathas_render_cache_entry (
    imathas_render_cache_entry_id uuid PRIMARY KEY,
    imathas_deployment_reference text NOT NULL
        CHECK (imathas_deployment_reference ~ '^[A-Za-z0-9._-]{1,160}$'),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    imathas_normalized_question_seed integer NOT NULL
        CHECK (imathas_normalized_question_seed BETWEEN 1 AND 9999),
    imathas_profile text NOT NULL CHECK (imathas_profile ~ '^[A-Za-z0-9._-]{1,160}$'),
    source_payload_digest bytea NOT NULL
        CHECK (pg_catalog.octet_length(source_payload_digest) = 32),
    encrypted_render_data bytea NOT NULL
        CHECK (pg_catalog.octet_length(encrypted_render_data) BETWEEN 1 AND 1048576),
    fetched_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > fetched_at),
    CONSTRAINT imathas_render_cache_entry_question_revision_matches
        FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    UNIQUE (
        imathas_deployment_reference, question_id, revision_number,
        imathas_normalized_question_seed, imathas_profile, source_payload_digest
    )
);

CREATE TABLE ple_private.imathas_question_backend_session (
    imathas_question_backend_session_id uuid PRIMARY KEY,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    question_attempt_id uuid NOT NULL
        REFERENCES ple_private.question_attempt (question_attempt_id),
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    imathas_deployment_reference text NOT NULL
        CHECK (imathas_deployment_reference ~ '^[A-Za-z0-9._-]{1,160}$'),
    imathas_item_reference text NOT NULL
        CHECK (pg_catalog.octet_length(imathas_item_reference) BETWEEN 1 AND 128
            AND imathas_item_reference ~ '^[A-Za-z0-9._-]+$'),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    source_object_id uuid NOT NULL,
    source_object_checksum bytea NOT NULL
        CHECK (pg_catalog.octet_length(source_object_checksum) = 32),
    imathas_profile text NOT NULL
        CHECK (imathas_profile ~ '^[A-Za-z0-9._-]{1,160}$'),
    question_seed numeric(20, 0) NOT NULL
        CHECK (question_seed >= 0 AND question_seed <= 18446744073709551615),
    question_grading_rule text NOT NULL
        CHECK (question_grading_rule IN ('all_or_nothing', 'partial_credit', 'ungraded')),
    question_points_possible numeric
        CHECK (question_points_possible IS NULL OR (question_points_possible >= 0
            AND question_points_possible NOT IN ('NaN'::numeric, 'Infinity'::numeric, '-Infinity'::numeric))),
    qualified_launch_binding_digest text NOT NULL
        CHECK (qualified_launch_binding_digest ~ '^[0-9a-f]{64}$'),
    imathas_response_sha256 bytea NOT NULL
        CHECK (pg_catalog.octet_length(imathas_response_sha256) = 32),
    imathas_question_backend_session_challenge bytea NOT NULL
        CHECK (pg_catalog.octet_length(imathas_question_backend_session_challenge) = 32
            AND imathas_question_backend_session_challenge
                <> pg_catalog.decode(pg_catalog.repeat('00', 32), 'hex')),
    imathas_question_backend_session_authentication bytea NOT NULL
        CONSTRAINT imathas_question_backend_session_authentication_is_canonical CHECK (
            pg_catalog.octet_length(imathas_question_backend_session_authentication) BETWEEN 3 AND 512
            AND pg_catalog.convert_from(imathas_question_backend_session_authentication, 'UTF8')
                ~ '^([0-9a-f]{2})+[.][0-9a-f]{64}$'
        ),
    issued_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > issued_at),
    revoked_at timestamp with time zone,
    consumed_at timestamp with time zone,
    activity_lease_token_sha256 bytea
        CHECK (activity_lease_token_sha256 IS NULL
            OR pg_catalog.octet_length(activity_lease_token_sha256) = 32),
    activity_lease_expires_at timestamp with time zone,
    imathas_question_backend_state_key_id text NOT NULL
        CHECK (imathas_question_backend_state_key_id ~ '^[A-Za-z0-9._:-]{1,160}$'),
    imathas_question_backend_state_nonce bytea NOT NULL
        CHECK (pg_catalog.octet_length(imathas_question_backend_state_nonce) = 24),
    imathas_question_backend_state_ciphertext bytea NOT NULL
        CHECK (pg_catalog.octet_length(imathas_question_backend_state_ciphertext) BETWEEN 17 AND 65536),
    CONSTRAINT imathas_question_backend_session_assignment_matches FOREIGN KEY (course_id, assignment_id)
        REFERENCES ple_data.assignment (course_id, assignment_id),
    CONSTRAINT imathas_question_backend_session_version_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT imathas_question_backend_session_revocation_is_ordered
        CHECK (revoked_at IS NULL OR revoked_at >= issued_at),
    CONSTRAINT imathas_question_backend_session_consumption_is_ordered
        CHECK (consumed_at IS NULL OR consumed_at >= issued_at),
    CONSTRAINT imathas_question_backend_session_activity_lease_matches CHECK (
        (activity_lease_token_sha256 IS NULL AND activity_lease_expires_at IS NULL)
        OR (activity_lease_token_sha256 IS NOT NULL AND activity_lease_expires_at IS NOT NULL)
    ),
    CONSTRAINT imathas_question_backend_session_activity_lease_is_within_session CHECK (
        activity_lease_expires_at IS NULL
        OR (activity_lease_expires_at > issued_at AND activity_lease_expires_at <= expires_at)
    ),
    CONSTRAINT imathas_question_backend_session_terminal_state_is_exclusive CHECK (
        revoked_at IS NULL OR consumed_at IS NULL
    ),
    CONSTRAINT imathas_question_backend_session_question_grading_rule_is_exact CHECK (
        (question_grading_rule IN ('all_or_nothing', 'partial_credit')
            AND question_points_possible IS NOT NULL)
        OR (question_grading_rule = 'ungraded' AND question_points_possible IS NULL)
    ),
    CONSTRAINT imathas_question_backend_session_state_key_nonce_is_unique
        UNIQUE (imathas_question_backend_state_key_id, imathas_question_backend_state_nonce)
);

CREATE TABLE ple_private.imathas_result_exchange (
    imathas_question_backend_session_id uuid PRIMARY KEY,
    idempotency_key text NOT NULL CHECK (pg_catalog.octet_length(idempotency_key) BETWEEN 1 AND 200),
    state text NOT NULL CHECK (state IN ('verifying', 'ready_to_commit', 'committed', 'failed', 'cancelled')),
    lease_token_sha256 bytea CHECK (lease_token_sha256 IS NULL OR pg_catalog.octet_length(lease_token_sha256) = 32),
    lease_expires_at timestamp with time zone,
    imathas_result_token_sha256 bytea CHECK (imathas_result_token_sha256 IS NULL OR pg_catalog.octet_length(imathas_result_token_sha256) = 32),
    imathas_result_normalized_score double precision,
    imathas_result_checksum bytea CHECK (imathas_result_checksum IS NULL OR pg_catalog.octet_length(imathas_result_checksum) = 32),
    submission_id uuid UNIQUE REFERENCES ple_private.question_submission (submission_id),
    question_submission_grading_id uuid UNIQUE REFERENCES ple_private.question_submission_grading (question_submission_grading_id),
    grading_result_id uuid UNIQUE REFERENCES ple_private.grading_result (grading_result_id),
    committed_job_lease_token_sha256 bytea CHECK (committed_job_lease_token_sha256 IS NULL OR pg_catalog.octet_length(committed_job_lease_token_sha256) = 32),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    committed_at timestamp with time zone,
    failed_at timestamp with time zone,
    cancelled_at timestamp with time zone,
    failure_code text CHECK (failure_code IS NULL OR char_length(btrim(failure_code)) BETWEEN 1 AND 160),
    CONSTRAINT imathas_result_exchange_session_matches
        FOREIGN KEY (imathas_question_backend_session_id)
        REFERENCES ple_private.imathas_question_backend_session (imathas_question_backend_session_id),
    CONSTRAINT imathas_result_exchange_normalized_score_is_valid CHECK (
        imathas_result_normalized_score IS NULL OR (
            imathas_result_normalized_score NOT IN ('NaN'::double precision, 'Infinity'::double precision, '-Infinity'::double precision)
            AND imathas_result_normalized_score >= 0 AND imathas_result_normalized_score <= 1
            AND pg_catalog.float8send(imathas_result_normalized_score) <> pg_catalog.decode('8000000000000000', 'hex')
        )
    ),
    CONSTRAINT imathas_result_exchange_transition_times_are_ordered CHECK (
        updated_at >= created_at
        AND (committed_at IS NULL OR committed_at BETWEEN created_at AND updated_at)
        AND (failed_at IS NULL OR failed_at BETWEEN created_at AND updated_at)
        AND (cancelled_at IS NULL OR cancelled_at BETWEEN created_at AND updated_at)
    ),
    CONSTRAINT imathas_result_exchange_state_matches CHECK (
        (state = 'verifying' AND lease_token_sha256 IS NOT NULL AND lease_expires_at IS NOT NULL
            AND imathas_result_token_sha256 IS NULL AND imathas_result_normalized_score IS NULL
            AND imathas_result_checksum IS NULL AND submission_id IS NULL
            AND question_submission_grading_id IS NULL AND grading_result_id IS NULL
            AND committed_job_lease_token_sha256 IS NULL
            AND committed_at IS NULL AND failed_at IS NULL AND cancelled_at IS NULL AND failure_code IS NULL)
        OR (state = 'ready_to_commit' AND lease_token_sha256 IS NULL AND lease_expires_at IS NULL
            AND imathas_result_token_sha256 IS NOT NULL AND imathas_result_normalized_score IS NOT NULL
            AND imathas_result_checksum IS NOT NULL AND submission_id IS NOT NULL
            AND question_submission_grading_id IS NOT NULL AND grading_result_id IS NULL
            AND committed_job_lease_token_sha256 IS NULL
            AND committed_at IS NULL AND failed_at IS NULL AND cancelled_at IS NULL AND failure_code IS NULL)
        OR (state = 'committed' AND lease_token_sha256 IS NULL AND lease_expires_at IS NULL
            AND imathas_result_token_sha256 IS NOT NULL AND imathas_result_normalized_score IS NOT NULL
            AND imathas_result_checksum IS NOT NULL AND submission_id IS NOT NULL
            AND question_submission_grading_id IS NOT NULL AND grading_result_id IS NOT NULL
            AND committed_job_lease_token_sha256 IS NOT NULL
            AND committed_at IS NOT NULL AND failed_at IS NULL AND cancelled_at IS NULL AND failure_code IS NULL)
        OR (state = 'failed' AND lease_token_sha256 IS NULL AND lease_expires_at IS NULL
            AND imathas_result_token_sha256 IS NULL AND imathas_result_normalized_score IS NULL
            AND imathas_result_checksum IS NULL AND submission_id IS NULL
            AND question_submission_grading_id IS NULL AND grading_result_id IS NULL
            AND committed_job_lease_token_sha256 IS NULL
            AND committed_at IS NULL AND failed_at IS NOT NULL AND cancelled_at IS NULL AND failure_code IS NOT NULL)
        OR (state = 'cancelled' AND lease_token_sha256 IS NULL AND lease_expires_at IS NULL
            AND imathas_result_token_sha256 IS NULL AND imathas_result_normalized_score IS NULL
            AND imathas_result_checksum IS NULL AND submission_id IS NULL
            AND question_submission_grading_id IS NULL AND grading_result_id IS NULL
            AND committed_job_lease_token_sha256 IS NULL
            AND committed_at IS NULL AND failed_at IS NULL AND cancelled_at IS NOT NULL AND failure_code IS NULL)
    )
);
GRANT SELECT ON TABLE ple_private.account, ple_private.assignment_attempt,
    ple_private.question_source TO ple_api_owner;
GRANT UPDATE ON TABLE ple_private.question_attempt TO ple_api_owner;
GRANT INSERT ON TABLE ple_private.question_submission TO ple_api_owner;
GRANT INSERT, UPDATE ON TABLE ple_private.question_submission_grading,
    ple_private.job TO ple_api_owner;
GRANT SELECT, INSERT ON TABLE ple_private.grading_result TO ple_api_owner;
GRANT SELECT, INSERT, UPDATE ON TABLE ple_private.imathas_question_backend_session,
    ple_private.imathas_result_exchange TO ple_api_owner;

CREATE INDEX imathas_question_backend_session_store_lookup_idx
    ON ple_private.imathas_question_backend_session (imathas_question_backend_session_id, account_id, expires_at)
    WHERE revoked_at IS NULL AND consumed_at IS NULL;

-- The Session binding is write-once.  Only a forward revocation, lease rotation,
-- and the Exchange trigger's verified consumption transition may change it.
CREATE OR REPLACE FUNCTION ple_private.enforce_imathas_question_backend_session_transition()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.imathas_question_backend_session_id IS DISTINCT FROM OLD.imathas_question_backend_session_id
       OR NEW.course_id IS DISTINCT FROM OLD.course_id
       OR NEW.assignment_id IS DISTINCT FROM OLD.assignment_id
       OR NEW.question_attempt_id IS DISTINCT FROM OLD.question_attempt_id
       OR NEW.account_id IS DISTINCT FROM OLD.account_id
       OR NEW.imathas_deployment_reference IS DISTINCT FROM OLD.imathas_deployment_reference
       OR NEW.imathas_item_reference IS DISTINCT FROM OLD.imathas_item_reference
       OR NEW.question_id IS DISTINCT FROM OLD.question_id
       OR NEW.revision_number IS DISTINCT FROM OLD.revision_number
       OR NEW.source_object_id IS DISTINCT FROM OLD.source_object_id
       OR NEW.source_object_checksum IS DISTINCT FROM OLD.source_object_checksum
       OR NEW.imathas_profile IS DISTINCT FROM OLD.imathas_profile
       OR NEW.question_seed IS DISTINCT FROM OLD.question_seed
       OR NEW.question_grading_rule IS DISTINCT FROM OLD.question_grading_rule
       OR NEW.question_points_possible IS DISTINCT FROM OLD.question_points_possible
       OR NEW.qualified_launch_binding_digest IS DISTINCT FROM OLD.qualified_launch_binding_digest
       OR NEW.imathas_response_sha256 IS DISTINCT FROM OLD.imathas_response_sha256
       OR NEW.imathas_question_backend_session_challenge IS DISTINCT FROM OLD.imathas_question_backend_session_challenge
       OR NEW.imathas_question_backend_session_authentication IS DISTINCT FROM OLD.imathas_question_backend_session_authentication
       OR NEW.issued_at IS DISTINCT FROM OLD.issued_at
       OR NEW.expires_at IS DISTINCT FROM OLD.expires_at
       OR NEW.imathas_question_backend_state_key_id IS DISTINCT FROM OLD.imathas_question_backend_state_key_id
       OR NEW.imathas_question_backend_state_nonce IS DISTINCT FROM OLD.imathas_question_backend_state_nonce
       OR NEW.imathas_question_backend_state_ciphertext IS DISTINCT FROM OLD.imathas_question_backend_state_ciphertext THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Question Backend Session binding is immutable';
    END IF;
    IF NEW.revoked_at IS DISTINCT FROM OLD.revoked_at THEN
        IF OLD.revoked_at IS NOT NULL OR NEW.revoked_at IS NULL
           OR NEW.revoked_at < OLD.issued_at OR NEW.consumed_at IS DISTINCT FROM OLD.consumed_at
           OR NEW.activity_lease_token_sha256 IS NOT NULL OR NEW.activity_lease_expires_at IS NOT NULL THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Question Backend Session revocation is forward-only';
        END IF;
    END IF;
    IF NEW.consumed_at IS DISTINCT FROM OLD.consumed_at THEN
        IF OLD.consumed_at IS NOT NULL OR NEW.consumed_at IS NULL
           OR NEW.consumed_at < OLD.issued_at OR OLD.revoked_at IS NOT NULL
           OR NEW.revoked_at IS DISTINCT FROM OLD.revoked_at
           OR pg_catalog.pg_trigger_depth() < 2
           OR pg_catalog.current_setting('ple_private.imathas_result_exchange_consumption', true) IS DISTINCT FROM 'verified'
           OR NEW.activity_lease_token_sha256 IS NOT NULL OR NEW.activity_lease_expires_at IS NOT NULL THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Question Backend Session consumption requires a verified iMathAS Result Exchange transition';
        END IF;
    END IF;
    IF (OLD.revoked_at IS NOT NULL OR OLD.consumed_at IS NOT NULL)
       AND (NEW.activity_lease_token_sha256 IS DISTINCT FROM OLD.activity_lease_token_sha256
            OR NEW.activity_lease_expires_at IS DISTINCT FROM OLD.activity_lease_expires_at) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Question Backend Session terminal state cannot receive a lease';
    END IF;
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION ple_private.enforce_imathas_result_exchange_transition()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.state <> 'verifying' OR NOT EXISTS (
            SELECT 1 FROM ple_private.imathas_question_backend_session AS imathas_question_backend_session
            WHERE imathas_question_backend_session.imathas_question_backend_session_id = NEW.imathas_question_backend_session_id
              AND imathas_question_backend_session.revoked_at IS NULL AND imathas_question_backend_session.consumed_at IS NULL
              AND imathas_question_backend_session.issued_at <= NEW.updated_at
              AND imathas_question_backend_session.expires_at > NEW.updated_at
              AND imathas_question_backend_session.activity_lease_token_sha256 = NEW.lease_token_sha256
              AND imathas_question_backend_session.activity_lease_expires_at > NEW.updated_at
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Result Exchange requires one active leased Session';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.updated_at < OLD.updated_at OR NEW.state = OLD.state THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Result Exchange transition is not forward';
    END IF;
    IF NOT (
        (OLD.state = 'verifying' AND NEW.state IN ('ready_to_commit', 'failed', 'cancelled'))
        OR (OLD.state = 'ready_to_commit' AND NEW.state = 'committed')
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Result Exchange transition is not allowed';
    END IF;
    IF OLD.state = 'verifying' AND NEW.state = 'ready_to_commit' THEN
        IF NOT EXISTS (
            SELECT 1
            FROM ple_private.question_submission AS submission
            JOIN ple_private.question_submission_grading AS grading
              ON grading.question_submission_grading_id = NEW.question_submission_grading_id
             AND grading.submission_id = submission.submission_id
            JOIN ple_private.job AS job
              ON job.job_id = grading.job_id
             AND job.question_submission_id = submission.submission_id
             AND job.job_kind = 'grade_accepted_submission'
             AND job.job_target_kind = 'question_submission'
            JOIN ple_private.imathas_question_backend_session AS session
              ON session.imathas_question_backend_session_id = NEW.imathas_question_backend_session_id
             AND session.question_attempt_id = submission.question_attempt_id
            WHERE submission.submission_id = NEW.submission_id
              AND submission.student_response = pg_catalog.jsonb_build_object('kind', 'imathasQuestionBackend')
              AND grading.grading_state = 'pending'
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Result Exchange ready lineage is not exact';
        END IF;
        PERFORM pg_catalog.set_config(
            'ple_private.imathas_result_exchange_consumption', 'verified', true
        );
        UPDATE ple_private.imathas_question_backend_session AS imathas_question_backend_session
           SET consumed_at = NEW.updated_at,
               activity_lease_token_sha256 = NULL,
               activity_lease_expires_at = NULL
         WHERE imathas_question_backend_session.imathas_question_backend_session_id = NEW.imathas_question_backend_session_id
           AND imathas_question_backend_session.revoked_at IS NULL AND imathas_question_backend_session.consumed_at IS NULL
           AND imathas_question_backend_session.issued_at <= NEW.updated_at
           AND imathas_question_backend_session.expires_at > NEW.updated_at
           AND imathas_question_backend_session.activity_lease_token_sha256 = OLD.lease_token_sha256
           AND imathas_question_backend_session.activity_lease_expires_at > NEW.updated_at;
        IF NOT FOUND THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Question Backend Session is expired, revoked, consumed, or not leased by this exchange';
        END IF;
    ELSIF OLD.state = 'ready_to_commit' AND NEW.state = 'committed' THEN
        IF NOT EXISTS (
            SELECT 1
            FROM ple_private.grading_result AS result
            JOIN ple_private.question_submission_grading AS grading
              ON grading.question_submission_grading_id = result.question_submission_grading_id
             AND grading.submission_id = result.submission_id
            JOIN ple_private.job AS job ON job.job_id = grading.job_id
            JOIN ple_private.question_submission AS submission ON submission.submission_id = result.submission_id
            JOIN ple_private.imathas_question_backend_session AS session
              ON session.imathas_question_backend_session_id = NEW.imathas_question_backend_session_id
             AND session.question_attempt_id = result.question_attempt_id
            WHERE result.grading_result_id = NEW.grading_result_id
              AND result.submission_id = NEW.submission_id
              AND result.question_submission_grading_id = NEW.question_submission_grading_id
              AND grading.grading_state = 'graded' AND job.state = 'completed'
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Result Exchange committed lineage is not exact';
        END IF;
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER imathas_question_backend_session_transition_is_forward_only
BEFORE UPDATE ON ple_private.imathas_question_backend_session
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_imathas_question_backend_session_transition();

CREATE TRIGGER imathas_result_exchange_transition_is_forward_only
BEFORE INSERT OR UPDATE ON ple_private.imathas_result_exchange
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_imathas_result_exchange_transition();

ALTER TABLE ple_private.imathas_render_cache_entry ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.imathas_render_cache_entry FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.imathas_question_backend_session ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.imathas_question_backend_session FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.imathas_result_exchange ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.imathas_result_exchange FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.imathas_render_cache_entry,
    ple_private.imathas_question_backend_session,
    ple_private.imathas_result_exchange FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION
    ple_private.enforce_imathas_question_backend_session_transition(),
    ple_private.enforce_imathas_result_exchange_transition() FROM PUBLIC;

CREATE POLICY imathas_question_backend_session_api_owner_select
    ON ple_private.imathas_question_backend_session FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY imathas_question_backend_session_api_owner_insert
    ON ple_private.imathas_question_backend_session FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY imathas_question_backend_session_api_owner_update
    ON ple_private.imathas_question_backend_session FOR UPDATE TO ple_api_owner
    USING (true) WITH CHECK (true);
CREATE POLICY imathas_result_exchange_api_owner_select
    ON ple_private.imathas_result_exchange FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY imathas_result_exchange_api_owner_insert
    ON ple_private.imathas_result_exchange FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY imathas_result_exchange_api_owner_update
    ON ple_private.imathas_result_exchange FOR UPDATE TO ple_api_owner
    USING (true) WITH CHECK (true);
CREATE POLICY question_attempt_api_owner_update
    ON ple_private.question_attempt FOR UPDATE TO ple_api_owner
    USING (true) WITH CHECK (true);
CREATE POLICY question_submission_api_owner_insert
    ON ple_private.question_submission FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY question_submission_grading_api_owner_insert
    ON ple_private.question_submission_grading FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY question_submission_grading_api_owner_update
    ON ple_private.question_submission_grading FOR UPDATE TO ple_api_owner
    USING (true) WITH CHECK (true);
CREATE POLICY job_api_owner_insert
    ON ple_private.job FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY job_api_owner_update
    ON ple_private.job FOR UPDATE TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY grading_result_api_owner_read
    ON ple_private.grading_result FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY grading_result_api_owner_insert
    ON ple_private.grading_result FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY assignment_attempt_api_owner_access
    ON ple_private.assignment_attempt FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY question_source_api_owner_access
    ON ple_private.question_source FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY account_api_owner_access
    ON ple_private.account FOR SELECT TO ple_api_owner USING (true);
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
GRANT INSERT ON TABLE ple_audit.automated_grading_receipt TO ple_api_owner;
CREATE POLICY automated_grading_receipt_api_owner_insert
    ON ple_audit.automated_grading_receipt FOR INSERT TO ple_api_owner WITH CHECK (true);
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
CREATE POLICY assignment_api_owner_access ON ple_data.assignment
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY question_revision_api_owner_access ON ple_data.question_revision
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY student_record_api_owner_access ON ple_data.student_record
    FOR SELECT TO ple_api_owner USING (true);
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.create_imathas_question_backend_session(
    p_imathas_question_backend_session_id uuid, p_course_id uuid, p_assignment_id uuid, p_question_attempt_id uuid,
    p_imathas_deployment_reference text, p_imathas_item_reference text, p_question_id text,
    p_revision_number integer, p_source_object_id uuid, p_source_object_checksum bytea,
    p_imathas_profile text, p_question_seed numeric, p_question_grading_rule text,
    p_question_points_possible numeric, p_qualified_launch_binding_digest text,
    p_imathas_response_sha256 bytea, p_imathas_question_backend_session_challenge bytea,
    p_imathas_question_backend_session_authentication bytea, p_issued_at timestamp with time zone,
    p_expires_at timestamp with time zone, p_imathas_question_backend_state_key_id text,
    p_imathas_question_backend_state_nonce bytea, p_imathas_question_backend_state_ciphertext bytea
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_account_id uuid;
BEGIN
    v_account_id := ple_api.current_session_account_id();
    IF p_imathas_question_backend_session_id IS NULL OR p_issued_at IS NULL
       OR p_issued_at > pg_catalog.clock_timestamp()
       OR p_expires_at <= pg_catalog.clock_timestamp() OR p_expires_at <= p_issued_at
       OR p_imathas_question_backend_state_key_id IS NULL
       OR p_imathas_question_backend_state_nonce IS NULL
       OR p_imathas_question_backend_state_ciphertext IS NULL
       OR p_imathas_item_reference IS NULL OR p_question_seed IS NULL
       OR p_qualified_launch_binding_digest IS NULL OR p_question_grading_rule NOT IN ('all_or_nothing', 'partial_credit', 'ungraded')
       OR (p_question_grading_rule IN ('all_or_nothing', 'partial_credit')
           AND (p_question_points_possible IS NULL OR p_question_points_possible < 0
                OR p_question_points_possible IN ('NaN'::numeric, 'Infinity'::numeric, '-Infinity'::numeric)))
       OR (p_question_grading_rule = 'ungraded' AND p_question_points_possible IS NOT NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'iMathAS Question Backend Session requires a future expiry';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.question_attempt qa
        JOIN ple_private.issued_question iq ON iq.issued_question_id = qa.issued_question_id
        JOIN ple_private.assignment_attempt aa ON aa.assignment_attempt_id = iq.assignment_attempt_id
        JOIN ple_data.student_record sr ON sr.student_record_id = aa.student_record_id
        JOIN ple_data.assignment a ON a.assignment_id = aa.assignment_id
        JOIN ple_private.question_source qs ON qs.question_id = iq.question_id
            AND qs.revision_number = iq.revision_number
            AND qs.source_object_id = p_source_object_id
            AND qs.source_object_checksum = pg_catalog.encode(p_source_object_checksum, 'hex')
        WHERE qa.question_attempt_id = p_question_attempt_id AND sr.student_account_id = v_account_id
          AND sr.course_id = p_course_id AND a.course_id = p_course_id
          AND aa.assignment_id = p_assignment_id AND iq.question_id = p_question_id
          AND iq.revision_number = p_revision_number AND qa.question_seed = p_question_seed
          AND (p_question_grading_rule = 'ungraded' OR iq.point_value = p_question_points_possible)
          AND ple_api.current_session_account_owns_student_record(p_course_id, aa.student_record_id)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Question Backend Session context is not owned by the installed Account';
    END IF;
    INSERT INTO ple_private.imathas_question_backend_session (
        imathas_question_backend_session_id, course_id, assignment_id, question_attempt_id, account_id, imathas_deployment_reference,
        imathas_item_reference, question_id, revision_number, source_object_id,
        source_object_checksum, imathas_profile, question_seed, question_grading_rule,
        question_points_possible, qualified_launch_binding_digest,
        imathas_response_sha256, imathas_question_backend_session_challenge,
        imathas_question_backend_session_authentication, issued_at, expires_at,
        imathas_question_backend_state_key_id, imathas_question_backend_state_nonce,
        imathas_question_backend_state_ciphertext
    ) VALUES (
        p_imathas_question_backend_session_id, p_course_id, p_assignment_id, p_question_attempt_id, v_account_id,
        p_imathas_deployment_reference, p_imathas_item_reference, p_question_id, p_revision_number,
        p_source_object_id, p_source_object_checksum, p_imathas_profile, p_question_seed,
        p_question_grading_rule, p_question_points_possible,
        p_qualified_launch_binding_digest, p_imathas_response_sha256,
        p_imathas_question_backend_session_challenge, p_imathas_question_backend_session_authentication,
        p_issued_at, p_expires_at, p_imathas_question_backend_state_key_id,
        p_imathas_question_backend_state_nonce, p_imathas_question_backend_state_ciphertext
    );
    RETURN p_imathas_question_backend_session_id;
END $$;

CREATE FUNCTION ple_api.load_imathas_question_backend_session(
    p_imathas_question_backend_session_id uuid, p_account_id uuid, p_course_id uuid, p_assignment_id uuid, p_question_attempt_id uuid,
    p_imathas_deployment_reference text, p_imathas_item_reference text, p_question_id text, p_revision_number integer,
    p_source_object_id uuid, p_source_object_checksum bytea, p_imathas_profile text,
    p_question_seed numeric, p_question_grading_rule text, p_question_points_possible numeric,
    p_qualified_launch_binding_digest text
) RETURNS TABLE (
    imathas_question_backend_session_id uuid, imathas_item_reference text, question_seed numeric,
    imathas_profile text, question_grading_rule text, question_points_possible numeric,
    qualified_launch_binding_digest text, imathas_response_sha256 bytea,
    imathas_question_backend_session_challenge bytea,
    imathas_question_backend_session_authentication bytea, issued_at timestamp with time zone,
    expires_at timestamp with time zone, imathas_question_backend_state_key_id text,
    imathas_question_backend_state_nonce bytea, imathas_question_backend_state_ciphertext bytea
) LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    RETURN QUERY SELECT s.imathas_question_backend_session_id, s.imathas_item_reference, s.question_seed,
        s.imathas_profile, s.question_grading_rule, s.question_points_possible,
        s.qualified_launch_binding_digest, s.imathas_response_sha256,
        s.imathas_question_backend_session_challenge,
        s.imathas_question_backend_session_authentication, s.issued_at, s.expires_at,
        s.imathas_question_backend_state_key_id, s.imathas_question_backend_state_nonce,
        s.imathas_question_backend_state_ciphertext
    FROM ple_private.imathas_question_backend_session s
    WHERE s.imathas_question_backend_session_id = p_imathas_question_backend_session_id
      AND s.account_id = ple_api.current_session_account_id() AND s.account_id = p_account_id
      AND s.course_id = p_course_id AND s.assignment_id = p_assignment_id
      AND s.question_attempt_id = p_question_attempt_id AND s.imathas_deployment_reference = p_imathas_deployment_reference AND s.imathas_item_reference = p_imathas_item_reference
      AND s.question_id = p_question_id AND s.revision_number = p_revision_number
      AND s.source_object_id = p_source_object_id AND s.source_object_checksum = p_source_object_checksum
      AND s.imathas_profile = p_imathas_profile AND s.question_seed = p_question_seed
      AND s.question_grading_rule = p_question_grading_rule
      AND s.question_points_possible IS NOT DISTINCT FROM p_question_points_possible
      AND s.qualified_launch_binding_digest = p_qualified_launch_binding_digest
      AND EXISTS (SELECT 1 FROM ple_private.question_attempt qa
          JOIN ple_private.issued_question iq ON iq.issued_question_id = qa.issued_question_id
          JOIN ple_private.assignment_attempt aa ON aa.assignment_attempt_id = iq.assignment_attempt_id
          WHERE qa.question_attempt_id = s.question_attempt_id
            AND ple_api.current_session_account_owns_student_record(s.course_id, aa.student_record_id))
      AND s.revoked_at IS NULL AND s.consumed_at IS NULL
      AND s.issued_at <= pg_catalog.clock_timestamp()
      AND s.expires_at > pg_catalog.clock_timestamp();
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Question Backend Session is unavailable'; END IF;
END $$;

CREATE FUNCTION ple_api.lease_imathas_question_backend_session(
    p_imathas_question_backend_session_id uuid, p_course_id uuid, p_assignment_id uuid, p_question_attempt_id uuid,
    p_imathas_deployment_reference text, p_imathas_item_reference text, p_question_id text,
    p_revision_number integer, p_source_object_id uuid, p_source_object_checksum bytea,
    p_imathas_profile text, p_question_seed numeric, p_question_grading_rule text,
    p_question_points_possible numeric,
    p_qualified_launch_binding_digest text, p_lease_token_sha256 bytea,
    p_lease_expires_at timestamp with time zone
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE s ple_private.imathas_question_backend_session%ROWTYPE;
BEGIN
    IF pg_catalog.octet_length(p_lease_token_sha256) <> 32
       OR p_lease_expires_at <= pg_catalog.clock_timestamp() THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'iMathAS Question Backend Session lease is invalid';
    END IF;
    SELECT * INTO s FROM ple_private.imathas_question_backend_session
     WHERE imathas_question_backend_session_id = p_imathas_question_backend_session_id FOR UPDATE;
    IF NOT FOUND OR s.account_id <> ple_api.current_session_account_id()
       OR s.course_id <> p_course_id OR s.assignment_id <> p_assignment_id OR s.question_attempt_id <> p_question_attempt_id
       OR s.imathas_deployment_reference <> p_imathas_deployment_reference
       OR s.imathas_item_reference <> p_imathas_item_reference OR s.question_id <> p_question_id
       OR s.revision_number <> p_revision_number OR s.source_object_id <> p_source_object_id
       OR s.source_object_checksum <> p_source_object_checksum
       OR s.imathas_profile <> p_imathas_profile OR s.question_seed <> p_question_seed
       OR s.question_grading_rule <> p_question_grading_rule
       OR s.question_points_possible IS DISTINCT FROM p_question_points_possible
       OR s.qualified_launch_binding_digest <> p_qualified_launch_binding_digest
       OR s.revoked_at IS NOT NULL OR s.consumed_at IS NOT NULL
       OR s.issued_at > pg_catalog.clock_timestamp()
       OR s.expires_at <= pg_catalog.clock_timestamp() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Question Backend Session is unavailable';
    END IF;
    IF p_lease_expires_at > s.expires_at THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'iMathAS Question Backend Session lease exceeds session expiry';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_private.question_attempt qa JOIN ple_private.issued_question iq ON iq.issued_question_id = qa.issued_question_id JOIN ple_private.assignment_attempt aa ON aa.assignment_attempt_id = iq.assignment_attempt_id WHERE qa.question_attempt_id = s.question_attempt_id AND ple_api.current_session_account_owns_student_record(s.course_id, aa.student_record_id)) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Question Backend Session is unavailable';
    END IF;
    IF s.activity_lease_expires_at > pg_catalog.clock_timestamp() THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'iMathAS Question Backend Session already has an active lease';
    END IF;
    UPDATE ple_private.imathas_question_backend_session SET activity_lease_token_sha256 = p_lease_token_sha256,
        activity_lease_expires_at = p_lease_expires_at WHERE imathas_question_backend_session_id = p_imathas_question_backend_session_id;
END $$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.stage_verified_imathas_result(
    p_imathas_question_backend_session_id uuid, p_course_id uuid, p_assignment_id uuid, p_question_attempt_id uuid,
    p_imathas_deployment_reference text, p_imathas_item_reference text, p_question_id text,
    p_revision_number integer, p_source_object_id uuid, p_source_object_checksum bytea,
    p_imathas_profile text, p_question_seed numeric, p_question_grading_rule text,
    p_question_points_possible numeric,
    p_qualified_launch_binding_digest text, p_lease_token_sha256 bytea,
    p_idempotency_key text, p_imathas_result_token_sha256 bytea,
    p_imathas_result_normalized_score double precision, p_imathas_result_checksum bytea,
    p_submission_id uuid, p_job_id uuid, p_question_submission_grading_id uuid,
    p_updated_at timestamp with time zone
) RETURNS TABLE (submission_id uuid, question_submission_grading_id uuid, job_id uuid)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE s ple_private.imathas_question_backend_session%ROWTYPE;
DECLARE e ple_private.imathas_result_exchange%ROWTYPE;
BEGIN
    IF pg_catalog.octet_length(p_lease_token_sha256) <> 32
       OR pg_catalog.octet_length(p_imathas_result_token_sha256) <> 32
       OR pg_catalog.octet_length(p_imathas_result_checksum) <> 32
       OR p_idempotency_key IS NULL OR pg_catalog.octet_length(p_idempotency_key) NOT BETWEEN 1 AND 200
       OR p_imathas_result_normalized_score IS NULL
       OR p_imathas_result_normalized_score IN ('NaN'::double precision, 'Infinity'::double precision, '-Infinity'::double precision)
       OR p_imathas_result_normalized_score < 0 OR p_imathas_result_normalized_score > 1
       OR pg_catalog.float8send(p_imathas_result_normalized_score) = pg_catalog.decode('8000000000000000', 'hex')
       OR p_submission_id IS NULL OR p_job_id IS NULL OR p_question_submission_grading_id IS NULL
       OR p_updated_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'iMathAS Result Exchange verification facts are invalid';
    END IF;
    SELECT * INTO s FROM ple_private.imathas_question_backend_session
     WHERE imathas_question_backend_session_id = p_imathas_question_backend_session_id FOR UPDATE;
    IF NOT FOUND OR s.account_id IS DISTINCT FROM ple_api.current_session_account_id()
       OR s.course_id IS DISTINCT FROM p_course_id OR s.assignment_id IS DISTINCT FROM p_assignment_id
       OR s.question_attempt_id IS DISTINCT FROM p_question_attempt_id
       OR s.imathas_deployment_reference IS DISTINCT FROM p_imathas_deployment_reference OR s.imathas_item_reference IS DISTINCT FROM p_imathas_item_reference
       OR s.question_id IS DISTINCT FROM p_question_id OR s.revision_number IS DISTINCT FROM p_revision_number
       OR s.source_object_id IS DISTINCT FROM p_source_object_id OR s.source_object_checksum IS DISTINCT FROM p_source_object_checksum
       OR s.imathas_profile IS DISTINCT FROM p_imathas_profile OR s.question_seed IS DISTINCT FROM p_question_seed
       OR s.question_grading_rule IS DISTINCT FROM p_question_grading_rule
       OR s.question_points_possible IS DISTINCT FROM p_question_points_possible
       OR s.qualified_launch_binding_digest <> p_qualified_launch_binding_digest THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Question Backend Session context is unavailable';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_private.question_attempt qa JOIN ple_private.issued_question iq ON iq.issued_question_id = qa.issued_question_id JOIN ple_private.assignment_attempt aa ON aa.assignment_attempt_id = iq.assignment_attempt_id WHERE qa.question_attempt_id = s.question_attempt_id AND ple_api.current_session_account_owns_student_record(s.course_id, aa.student_record_id)) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Question Backend Session cannot be consumed';
    END IF;
    SELECT * INTO e FROM ple_private.imathas_result_exchange
     WHERE imathas_question_backend_session_id = p_imathas_question_backend_session_id;
    IF e.imathas_question_backend_session_id IS NOT NULL AND e.state IN ('ready_to_commit', 'committed') THEN
        IF e.idempotency_key = p_idempotency_key
           AND e.imathas_result_token_sha256 = p_imathas_result_token_sha256
           AND e.imathas_result_normalized_score = p_imathas_result_normalized_score
           AND e.imathas_result_checksum = p_imathas_result_checksum THEN
            RETURN QUERY SELECT e.submission_id, e.question_submission_grading_id,
                (SELECT grading.job_id FROM ple_private.question_submission_grading AS grading
                  WHERE grading.question_submission_grading_id = e.question_submission_grading_id);
            RETURN;
        END IF;
        RAISE EXCEPTION USING ERRCODE = '23505', MESSAGE = 'iMathAS Result stage replay does not match ready lineage';
    END IF;
    IF s.account_id IS DISTINCT FROM ple_api.current_session_account_id()
       OR s.course_id IS DISTINCT FROM p_course_id OR s.assignment_id IS DISTINCT FROM p_assignment_id OR s.question_attempt_id IS DISTINCT FROM p_question_attempt_id
       OR s.imathas_deployment_reference IS DISTINCT FROM p_imathas_deployment_reference
       OR s.imathas_item_reference IS DISTINCT FROM p_imathas_item_reference OR s.question_id IS DISTINCT FROM p_question_id
       OR s.revision_number IS DISTINCT FROM p_revision_number OR s.source_object_id IS DISTINCT FROM p_source_object_id
       OR s.source_object_checksum IS DISTINCT FROM p_source_object_checksum
       OR s.imathas_profile IS DISTINCT FROM p_imathas_profile OR s.question_seed IS DISTINCT FROM p_question_seed
       OR s.question_grading_rule IS DISTINCT FROM p_question_grading_rule
       OR s.question_points_possible IS DISTINCT FROM p_question_points_possible
       OR s.qualified_launch_binding_digest <> p_qualified_launch_binding_digest
       OR s.revoked_at IS NOT NULL OR s.consumed_at IS NOT NULL
       OR s.issued_at > pg_catalog.clock_timestamp() OR s.issued_at > p_updated_at
       OR s.expires_at <= pg_catalog.clock_timestamp() OR s.expires_at <= p_updated_at
       OR s.activity_lease_token_sha256 <> p_lease_token_sha256
       OR s.activity_lease_expires_at <= p_updated_at
       OR s.activity_lease_expires_at <= pg_catalog.clock_timestamp()
       OR p_updated_at > pg_catalog.clock_timestamp() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Question Backend Session cannot be consumed';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_private.question_attempt qa JOIN ple_private.issued_question iq ON iq.issued_question_id = qa.issued_question_id JOIN ple_private.assignment_attempt aa ON aa.assignment_attempt_id = iq.assignment_attempt_id WHERE qa.question_attempt_id = s.question_attempt_id AND ple_api.current_session_account_owns_student_record(s.course_id, aa.student_record_id)) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Question Backend Session cannot be consumed';
    END IF;
    IF s.question_grading_rule NOT IN ('all_or_nothing', 'partial_credit') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Ungraded Question Grading Rule cannot stage an iMathAS Result';
    END IF;
    INSERT INTO ple_private.imathas_result_exchange (
        imathas_question_backend_session_id, idempotency_key, state, lease_token_sha256, lease_expires_at, created_at, updated_at
    ) VALUES (p_imathas_question_backend_session_id, p_idempotency_key, 'verifying', p_lease_token_sha256,
        s.activity_lease_expires_at, p_updated_at, p_updated_at)
    ON CONFLICT (imathas_question_backend_session_id) DO NOTHING;
    IF NOT FOUND THEN
        SELECT * INTO e FROM ple_private.imathas_result_exchange
         WHERE imathas_question_backend_session_id = p_imathas_question_backend_session_id;
        IF e.state IN ('ready_to_commit', 'committed')
           AND e.idempotency_key = p_idempotency_key
           AND e.imathas_result_token_sha256 = p_imathas_result_token_sha256
           AND e.imathas_result_normalized_score = p_imathas_result_normalized_score
           AND e.imathas_result_checksum = p_imathas_result_checksum THEN
            RETURN QUERY SELECT e.submission_id, e.question_submission_grading_id,
                (SELECT grading.job_id FROM ple_private.question_submission_grading AS grading
                  WHERE grading.question_submission_grading_id = e.question_submission_grading_id);
            RETURN;
        END IF;
        RAISE EXCEPTION USING ERRCODE = '23505', MESSAGE = 'iMathAS Result stage replay does not match ready lineage';
    END IF;
    BEGIN
        INSERT INTO ple_private.question_submission (
            submission_id, question_attempt_id, submitted_at, student_response
        ) VALUES (
            p_submission_id, s.question_attempt_id, p_updated_at,
            pg_catalog.jsonb_build_object('kind', 'imathasQuestionBackend')
        );
    EXCEPTION WHEN unique_violation THEN
        SELECT * INTO e FROM ple_private.imathas_result_exchange
         WHERE imathas_question_backend_session_id = p_imathas_question_backend_session_id;
        IF e.state IN ('ready_to_commit', 'committed')
           AND e.idempotency_key = p_idempotency_key
           AND e.imathas_result_token_sha256 = p_imathas_result_token_sha256
           AND e.imathas_result_normalized_score = p_imathas_result_normalized_score
           AND e.imathas_result_checksum = p_imathas_result_checksum THEN
            RETURN QUERY SELECT e.submission_id, e.question_submission_grading_id,
                (SELECT job_id FROM ple_private.question_submission_grading
                  WHERE question_submission_grading_id = e.question_submission_grading_id);
            RETURN;
        END IF;
        RAISE;
    END;
    UPDATE ple_private.question_attempt
       SET question_attempt_state = 'submission_accepted'
     WHERE question_attempt_id = s.question_attempt_id AND question_attempt_state = 'open';
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Question Backend Session Question Attempt is not open';
    END IF;
    INSERT INTO ple_private.job (
        job_id, job_kind, job_target_kind, question_submission_id, generation, target_digest,
        payload, state, available_at, max_attempts, created_at
    ) VALUES (
        p_job_id, 'grade_accepted_submission', 'question_submission', p_submission_id, 1,
        p_imathas_result_checksum, '{}'::jsonb,
        'ready', p_updated_at, 3, p_updated_at
    );
    INSERT INTO ple_private.question_submission_grading (
        question_submission_grading_id, submission_id, job_id, grading_state, created_at
    ) VALUES (p_question_submission_grading_id, p_submission_id, p_job_id, 'pending', p_updated_at);
    UPDATE ple_private.imathas_result_exchange SET state = 'ready_to_commit', lease_token_sha256 = NULL,
        lease_expires_at = NULL,
        imathas_result_token_sha256 = p_imathas_result_token_sha256,
        imathas_result_normalized_score = p_imathas_result_normalized_score,
        imathas_result_checksum = p_imathas_result_checksum,
        submission_id = p_submission_id, question_submission_grading_id = p_question_submission_grading_id,
        updated_at = p_updated_at WHERE imathas_question_backend_session_id = p_imathas_question_backend_session_id;
    RETURN QUERY SELECT p_submission_id, p_question_submission_grading_id, p_job_id;
END $$;

CREATE FUNCTION ple_api.claim_imathas_result_grading_job(
    p_job_id uuid, p_lease_token uuid, p_lease_expires_at timestamp with time zone
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    IF p_lease_token IS NULL OR p_lease_expires_at <= pg_catalog.clock_timestamp() THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'iMathAS Question Backend grading Job lease is invalid';
    END IF;
    IF p_lease_expires_at > pg_catalog.clock_timestamp() + interval '300 seconds' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'iMathAS Question Backend grading Job lease exceeds maximum duration';
    END IF;
    UPDATE ple_private.job SET state = 'leased',
        lease_token = p_lease_token,
        lease_expires_at = p_lease_expires_at, attempt_count = attempt_count + 1
     WHERE job_id = p_job_id AND job_kind = 'grade_accepted_submission'
       AND job_target_kind = 'question_submission' AND state = 'ready'
       AND available_at <= pg_catalog.clock_timestamp() AND attempt_count < max_attempts;
    IF NOT FOUND THEN
        UPDATE ple_private.job SET state = 'leased',
            lease_token = p_lease_token, lease_expires_at = p_lease_expires_at,
            attempt_count = attempt_count + 1
         WHERE job_id = p_job_id AND job_kind = 'grade_accepted_submission'
           AND job_target_kind = 'question_submission' AND state = 'leased'
           AND lease_expires_at <= pg_catalog.clock_timestamp() AND attempt_count < max_attempts;
    END IF;
    IF NOT FOUND THEN
        UPDATE ple_private.question_submission_grading AS grading
           SET grading_state = 'instructor_attention'
          FROM ple_private.imathas_result_exchange AS exchange
          JOIN ple_private.job AS job ON job.job_id = p_job_id
         WHERE exchange.question_submission_grading_id = grading.question_submission_grading_id
           AND job.job_id = p_job_id AND job.state = 'leased'
           AND job.lease_expires_at <= pg_catalog.clock_timestamp()
           AND job.attempt_count >= job.max_attempts
           AND exchange.state = 'ready_to_commit' AND grading.grading_state = 'pending';
        UPDATE ple_private.job SET state = 'failed', lease_token = NULL, lease_expires_at = NULL,
            completed_at = pg_catalog.clock_timestamp(), job_failure_kind = 'timed_out'
         WHERE job_id = p_job_id AND state = 'leased'
           AND lease_expires_at <= pg_catalog.clock_timestamp()
           AND attempt_count >= max_attempts;
        RETURN false;
    END IF;
    RETURN true;
END $$;

CREATE FUNCTION ple_api.commit_imathas_result_grading(
    p_job_id uuid, p_job_lease_token uuid, p_committed_at timestamp with time zone
) RETURNS TABLE (
    automated_grading_receipt_id uuid, correct boolean,
    points_earned double precision, points_possible double precision,
    automated_grading_receipt_checksum bytea
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private, ple_audit AS $$
DECLARE
    e ple_private.imathas_result_exchange%ROWTYPE;
    s ple_private.imathas_question_backend_session%ROWTYPE;
    j ple_private.job%ROWTYPE;
    g ple_private.question_submission_grading%ROWTYPE;
    v_correct boolean;
    v_points_earned double precision;
    v_points_possible double precision;
    v_grading_result_id uuid;
    v_automated_grading_receipt_id uuid;
    v_automated_grading_receipt_checksum bytea;
BEGIN
    IF p_job_id IS NULL OR p_job_lease_token IS NULL
       OR p_committed_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'iMathAS Question Backend grading commit facts are invalid';
    END IF;
    SELECT exchange.* INTO e
      FROM ple_private.imathas_result_exchange AS exchange
      JOIN ple_private.question_submission_grading AS submission_grading
        ON submission_grading.question_submission_grading_id = exchange.question_submission_grading_id
     WHERE submission_grading.job_id = p_job_id FOR UPDATE OF exchange;
    IF e.state = 'committed' AND e.question_submission_grading_id IS NOT NULL
       AND e.committed_job_lease_token_sha256 = pg_catalog.sha256(pg_catalog.uuid_send(p_job_lease_token)) THEN
        RETURN QUERY SELECT r.automated_grading_receipt_id, result.correct,
                result.points_earned::double precision, result.points_possible::double precision,
                r.automated_grading_receipt_checksum
            FROM ple_audit.automated_grading_receipt r
            JOIN ple_private.grading_result result ON result.grading_result_id = r.grading_result_id
            WHERE result.grading_result_id = e.grading_result_id;
        IF FOUND THEN RETURN; END IF;
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Question Backend committed lineage has no receipt';
    END IF;
    SELECT * INTO s FROM ple_private.imathas_question_backend_session WHERE imathas_question_backend_session_id = e.imathas_question_backend_session_id FOR UPDATE;
    SELECT * INTO j FROM ple_private.job WHERE job_id = p_job_id FOR UPDATE;
    SELECT * INTO g FROM ple_private.question_submission_grading
      WHERE question_submission_grading_id = e.question_submission_grading_id FOR UPDATE;
    IF e.state <> 'ready_to_commit' OR j.job_kind <> 'grade_accepted_submission'
       OR j.job_target_kind <> 'question_submission' OR j.question_submission_id <> e.submission_id
       OR j.state <> 'leased' OR j.lease_token <> p_job_lease_token
       OR j.lease_expires_at <= p_committed_at OR j.lease_expires_at <= pg_catalog.clock_timestamp()
       OR p_committed_at > pg_catalog.clock_timestamp() OR p_committed_at < e.updated_at
       OR p_committed_at < j.created_at OR p_committed_at < g.created_at OR g.job_id <> j.job_id
       OR g.submission_id <> e.submission_id OR g.grading_state <> 'pending'
       OR s.question_attempt_id <> (SELECT question_attempt_id FROM ple_private.question_submission WHERE submission_id = e.submission_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Question Backend grading commit does not own the live typed Job lease';
    END IF;
    v_points_possible := s.question_points_possible::double precision;
    IF v_points_possible IS NULL OR v_points_possible IN ('NaN'::double precision, 'Infinity'::double precision, '-Infinity'::double precision)
       OR v_points_possible < 0 OR pg_catalog.float8send(v_points_possible) = pg_catalog.decode('8000000000000000', 'hex') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Session Question Points Possible is invalid for iMathAS Question Backend grading';
    END IF;
    v_correct := e.imathas_result_normalized_score = 1.0;
    v_points_earned := CASE s.question_grading_rule
        WHEN 'all_or_nothing' THEN CASE WHEN v_correct THEN v_points_possible ELSE 0 END
        WHEN 'partial_credit' THEN v_points_possible * e.imathas_result_normalized_score
        ELSE NULL END;
    IF v_points_earned IS NULL OR v_points_earned IN ('NaN'::double precision, 'Infinity'::double precision, '-Infinity'::double precision)
       OR v_points_earned < 0 OR pg_catalog.float8send(v_points_earned) = pg_catalog.decode('8000000000000000', 'hex') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'iMathAS Question Backend grading result is invalid';
    END IF;
    v_grading_result_id := pg_catalog.gen_random_uuid();
    v_automated_grading_receipt_id := pg_catalog.gen_random_uuid();
    INSERT INTO ple_private.grading_result (
        grading_result_id, submission_id, question_submission_grading_id, question_attempt_id,
        correct, points_earned, points_possible, recorded_at
    ) VALUES (v_grading_result_id, e.submission_id, g.question_submission_grading_id,
        s.question_attempt_id, v_correct, v_points_earned, v_points_possible, p_committed_at);
    v_automated_grading_receipt_checksum := pg_catalog.sha256(
        pg_catalog.convert_to('ple:automated-grading-receipt:v1', 'UTF8')
        || pg_catalog.decode('00', 'hex')
        || pg_catalog.uuid_send(v_automated_grading_receipt_id)
        || pg_catalog.uuid_send(v_grading_result_id)
        || pg_catalog.uuid_send(g.question_submission_grading_id)
        || pg_catalog.uuid_send(e.submission_id)
        || pg_catalog.uuid_send(s.question_attempt_id)
        || pg_catalog.uuid_send(j.job_id)
        || pg_catalog.uuid_send(s.imathas_question_backend_session_id)
        || e.imathas_result_token_sha256 || e.imathas_result_checksum
        || CASE WHEN v_correct THEN pg_catalog.decode('01', 'hex') ELSE pg_catalog.decode('00', 'hex') END
        || pg_catalog.float8send(v_points_earned) || pg_catalog.float8send(v_points_possible)
        || pg_catalog.int8send((pg_catalog.date_part('epoch', p_committed_at) * 1000)::bigint)
    );
    INSERT INTO ple_audit.automated_grading_receipt (
        automated_grading_receipt_id, question_submission_grading_id, grading_result_id,
        committed_at, automated_grading_receipt_checksum
    ) VALUES (v_automated_grading_receipt_id, g.question_submission_grading_id, v_grading_result_id,
        p_committed_at, v_automated_grading_receipt_checksum);
    UPDATE ple_private.question_submission_grading SET grading_state = 'graded', completed_at = p_committed_at
      WHERE question_submission_grading_id = g.question_submission_grading_id;
    UPDATE ple_private.job SET state = 'completed', lease_token = NULL, lease_expires_at = NULL,
        completed_at = p_committed_at WHERE job_id = j.job_id;
    -- ASVS 2.3.1, 2.3.3, 2.3.4, 8.2.1-8.2.3, 8.3.1, 14.2.4, and 15.4.2:
    -- the trusted grading commit records the identity-free observation after
    -- terminal grading state and before the Result Exchange becomes replayable.
    -- Current iMathAS marker responses have no eligible choice IDs.
    PERFORM ple_api.record_question_statistics_observation(
        v_automated_grading_receipt_id,
        ARRAY[]::text[]
    );
    UPDATE ple_private.imathas_result_exchange SET state = 'committed', grading_result_id = v_grading_result_id,
        committed_job_lease_token_sha256 = pg_catalog.sha256(pg_catalog.uuid_send(p_job_lease_token)),
        committed_at = p_committed_at, updated_at = p_committed_at WHERE imathas_question_backend_session_id = e.imathas_question_backend_session_id;
    RETURN QUERY SELECT v_automated_grading_receipt_id, v_correct,
        v_points_earned, v_points_possible, v_automated_grading_receipt_checksum;
END $$;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.create_imathas_question_backend_session(uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, numeric, text, bytea, bytea, bytea, timestamp with time zone, timestamp with time zone, text, bytea, bytea),
    ple_api.load_imathas_question_backend_session(uuid, uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, numeric, text),
    ple_api.lease_imathas_question_backend_session(uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, numeric, text, bytea, timestamp with time zone),
    ple_api.stage_verified_imathas_result(uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, numeric, text, bytea, text, bytea, double precision, bytea, uuid, uuid, uuid, timestamp with time zone),
    ple_api.claim_imathas_result_grading_job(uuid, uuid, timestamp with time zone),
    ple_api.commit_imathas_result_grading(uuid, uuid, timestamp with time zone)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.create_imathas_question_backend_session(uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, numeric, text, bytea, bytea, bytea, timestamp with time zone, timestamp with time zone, text, bytea, bytea),
    ple_api.load_imathas_question_backend_session(uuid, uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, numeric, text),
    ple_api.lease_imathas_question_backend_session(uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, numeric, text, bytea, timestamp with time zone),
    ple_api.stage_verified_imathas_result(uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, numeric, text, bytea, text, bytea, double precision, bytea, uuid, uuid, uuid, timestamp with time zone)
TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.claim_imathas_result_grading_job(uuid, uuid, timestamp with time zone),
    ple_api.commit_imathas_result_grading(uuid, uuid, timestamp with time zone)
TO ple_api_owner;
GRANT USAGE ON SCHEMA ple_api TO ple_imathas_question_backend_grading_worker;
GRANT EXECUTE ON FUNCTION ple_api.claim_imathas_result_grading_job(uuid, uuid, timestamp with time zone),
    ple_api.commit_imathas_result_grading(uuid, uuid, timestamp with time zone)
TO ple_imathas_question_backend_grading_worker;
RESET ROLE;
