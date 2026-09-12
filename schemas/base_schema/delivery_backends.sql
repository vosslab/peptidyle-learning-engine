-- Private durable state for external Question-delivery backends.  The backend
-- keeps opaque state; an Issued Question and its Question Attempt remain the
-- authoritative reproducibility and Student Work evidence.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.imathas_render_cache_entry (
    imathas_render_cache_entry_id uuid PRIMARY KEY,
    imathas_deployment_reference text NOT NULL CHECK (imathas_deployment_reference ~ '^[A-Za-z0-9._-]{1,160}$'),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    imathas_normalized_question_seed integer NOT NULL CHECK (imathas_normalized_question_seed BETWEEN 1 AND 9999),
    imathas_profile text NOT NULL CHECK (imathas_profile ~ '^[A-Za-z0-9._-]{1,160}$'),
    source_payload_digest bytea NOT NULL CHECK (octet_length(source_payload_digest) = 32),
    encrypted_render_data bytea NOT NULL CHECK (octet_length(encrypted_render_data) BETWEEN 1 AND 1048576),
    fetched_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL CHECK (expires_at > fetched_at),
    FOREIGN KEY (question_id, revision_number) REFERENCES ple_data.question_revision(question_id, revision_number),
    UNIQUE (imathas_deployment_reference, question_id, revision_number, imathas_normalized_question_seed, imathas_profile, source_payload_digest)
);

CREATE TABLE ple_private.imathas_question_backend_session (
    imathas_question_backend_session_id uuid PRIMARY KEY,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    question_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_attempt(question_attempt_id) ON DELETE CASCADE,
    account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    imathas_deployment_reference text NOT NULL CHECK (imathas_deployment_reference ~ '^[A-Za-z0-9._-]{1,160}$'),
    imathas_item_reference text NOT NULL CHECK (octet_length(imathas_item_reference) BETWEEN 1 AND 128 AND imathas_item_reference ~ '^[A-Za-z0-9._-]+$'),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    source_object_id uuid NOT NULL,
    source_object_checksum bytea NOT NULL CHECK (octet_length(source_object_checksum) = 32),
    imathas_profile text NOT NULL CHECK (imathas_profile ~ '^[A-Za-z0-9._-]{1,160}$'),
    question_seed numeric(20, 0) NOT NULL CHECK (question_seed BETWEEN 0 AND 18446744073709551615),
    imathas_launch_binding_checksum text NOT NULL CHECK (imathas_launch_binding_checksum ~ '^[0-9a-f]{64}$'),
    imathas_response_sha256 bytea NOT NULL CHECK (octet_length(imathas_response_sha256) = 32),
    imathas_question_backend_session_challenge bytea NOT NULL CHECK (octet_length(imathas_question_backend_session_challenge) = 32 AND imathas_question_backend_session_challenge <> decode(repeat('00', 32), 'hex')),
    imathas_question_backend_session_authentication bytea NOT NULL CHECK (octet_length(imathas_question_backend_session_authentication) BETWEEN 3 AND 512 AND convert_from(imathas_question_backend_session_authentication, 'UTF8') ~ '^([0-9a-f]{2})+[.][0-9a-f]{64}$'),
    issued_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL CHECK (expires_at > issued_at),
    revoked_at timestamptz,
    consumed_at timestamptz,
    activity_lease_token_sha256 bytea CHECK (activity_lease_token_sha256 IS NULL OR octet_length(activity_lease_token_sha256) = 32),
    activity_lease_expires_at timestamptz,
    imathas_question_backend_state_key_id text NOT NULL CHECK (imathas_question_backend_state_key_id ~ '^[A-Za-z0-9._:-]{1,160}$'),
    imathas_question_backend_state_nonce bytea NOT NULL CHECK (octet_length(imathas_question_backend_state_nonce) = 24),
    imathas_question_backend_state_ciphertext bytea NOT NULL CHECK (octet_length(imathas_question_backend_state_ciphertext) BETWEEN 17 AND 65536),
    FOREIGN KEY (course_id, assignment_id) REFERENCES ple_data.assignment(course_id, assignment_id),
    FOREIGN KEY (question_id, revision_number) REFERENCES ple_data.question_revision(question_id, revision_number),
    CHECK (revoked_at IS NULL OR revoked_at >= issued_at),
    CHECK (consumed_at IS NULL OR consumed_at >= issued_at),
    CHECK ((activity_lease_token_sha256 IS NULL) = (activity_lease_expires_at IS NULL)),
    CHECK (activity_lease_expires_at IS NULL OR activity_lease_expires_at > issued_at AND activity_lease_expires_at <= expires_at),
    CHECK (revoked_at IS NULL OR consumed_at IS NULL),
    UNIQUE (imathas_question_backend_state_key_id, imathas_question_backend_state_nonce)
);

-- The references into grading are installed by cross_domain_constraints.sql
-- after the grading family has created its immutable tables.
CREATE TABLE ple_private.imathas_result_exchange (
    imathas_question_backend_session_id uuid PRIMARY KEY REFERENCES ple_private.imathas_question_backend_session(imathas_question_backend_session_id) ON DELETE CASCADE,
    state text NOT NULL CHECK (state IN ('verifying', 'ready_to_commit', 'committed', 'failed', 'cancelled')),
    lease_token_sha256 bytea CHECK (lease_token_sha256 IS NULL OR octet_length(lease_token_sha256) = 32),
    lease_expires_at timestamptz,
    imathas_result_token_sha256 bytea CHECK (imathas_result_token_sha256 IS NULL OR octet_length(imathas_result_token_sha256) = 32),
    imathas_result_normalized_score double precision,
    imathas_result_checksum bytea CHECK (imathas_result_checksum IS NULL OR octet_length(imathas_result_checksum) = 32),
    submission_id uuid UNIQUE,
    question_submission_grading_id uuid UNIQUE,
    grading_result_id uuid UNIQUE,
    committed_job_lease_token_sha256 bytea CHECK (committed_job_lease_token_sha256 IS NULL OR octet_length(committed_job_lease_token_sha256) = 32),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    committed_at timestamptz,
    failed_at timestamptz,
    cancelled_at timestamptz,
    failure_code text CHECK (failure_code IS NULL OR char_length(btrim(failure_code)) BETWEEN 1 AND 160),
    CHECK (imathas_result_normalized_score IS NULL OR (imathas_result_normalized_score >= 0 AND imathas_result_normalized_score <= 1 AND imathas_result_normalized_score NOT IN ('NaN'::double precision, 'Infinity'::double precision, '-Infinity'::double precision))),
    CHECK (updated_at >= created_at),
    CHECK ((state = 'verifying' AND lease_token_sha256 IS NOT NULL AND lease_expires_at IS NOT NULL AND submission_id IS NULL AND question_submission_grading_id IS NULL AND grading_result_id IS NULL AND committed_at IS NULL AND failed_at IS NULL AND cancelled_at IS NULL)
        OR (state = 'ready_to_commit' AND lease_token_sha256 IS NULL AND lease_expires_at IS NULL AND imathas_result_token_sha256 IS NOT NULL AND imathas_result_normalized_score IS NOT NULL AND imathas_result_checksum IS NOT NULL AND submission_id IS NOT NULL AND question_submission_grading_id IS NOT NULL AND grading_result_id IS NULL AND committed_at IS NULL AND failed_at IS NULL AND cancelled_at IS NULL)
        OR (state = 'committed' AND lease_token_sha256 IS NULL AND lease_expires_at IS NULL AND imathas_result_token_sha256 IS NOT NULL AND imathas_result_normalized_score IS NOT NULL AND imathas_result_checksum IS NOT NULL AND submission_id IS NOT NULL AND question_submission_grading_id IS NOT NULL AND grading_result_id IS NOT NULL AND committed_job_lease_token_sha256 IS NOT NULL AND committed_at IS NOT NULL AND failed_at IS NULL AND cancelled_at IS NULL)
        OR (state = 'failed' AND failed_at IS NOT NULL AND failure_code IS NOT NULL)
        OR (state = 'cancelled' AND cancelled_at IS NOT NULL))
);

CREATE FUNCTION ple_private.enforce_imathas_question_backend_session_transition()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF ROW(NEW.imathas_question_backend_session_id, NEW.course_id, NEW.assignment_id, NEW.question_attempt_id, NEW.account_id, NEW.imathas_deployment_reference, NEW.imathas_item_reference, NEW.question_id, NEW.revision_number, NEW.source_object_id, NEW.source_object_checksum, NEW.imathas_profile, NEW.question_seed, NEW.imathas_launch_binding_checksum, NEW.imathas_response_sha256, NEW.imathas_question_backend_session_challenge, NEW.imathas_question_backend_session_authentication, NEW.issued_at, NEW.expires_at, NEW.imathas_question_backend_state_key_id, NEW.imathas_question_backend_state_nonce, NEW.imathas_question_backend_state_ciphertext)
       IS DISTINCT FROM ROW(OLD.imathas_question_backend_session_id, OLD.course_id, OLD.assignment_id, OLD.question_attempt_id, OLD.account_id, OLD.imathas_deployment_reference, OLD.imathas_item_reference, OLD.question_id, OLD.revision_number, OLD.source_object_id, OLD.source_object_checksum, OLD.imathas_profile, OLD.question_seed, OLD.imathas_launch_binding_checksum, OLD.imathas_response_sha256, OLD.imathas_question_backend_session_challenge, OLD.imathas_question_backend_session_authentication, OLD.issued_at, OLD.expires_at, OLD.imathas_question_backend_state_key_id, OLD.imathas_question_backend_state_nonce, OLD.imathas_question_backend_state_ciphertext) THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'iMathAS Question Backend Session binding is immutable';
    END IF;
    IF OLD.revoked_at IS NOT NULL OR OLD.consumed_at IS NOT NULL THEN
        IF NEW IS DISTINCT FROM OLD THEN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'iMathAS Question Backend Session is terminal'; END IF;
    ELSIF NEW.revoked_at IS NOT NULL AND (NEW.revoked_at < OLD.issued_at OR NEW.consumed_at IS NOT NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Session revocation is invalid';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.enforce_imathas_result_exchange_transition()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.state <> 'verifying' THEN RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Result Exchange starts verifying'; END IF;
        RETURN NEW;
    END IF;
    IF NEW.updated_at < OLD.updated_at OR NOT ((OLD.state = 'verifying' AND NEW.state IN ('ready_to_commit', 'failed', 'cancelled')) OR (OLD.state = 'ready_to_commit' AND NEW.state = 'committed')) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Result Exchange transition is not forward';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER imathas_question_backend_session_transition_is_forward_only BEFORE UPDATE ON ple_private.imathas_question_backend_session FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_imathas_question_backend_session_transition();
CREATE TRIGGER imathas_result_exchange_transition_is_forward_only BEFORE INSERT OR UPDATE ON ple_private.imathas_result_exchange FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_imathas_result_exchange_transition();
CREATE INDEX imathas_question_backend_session_active_lookup_idx ON ple_private.imathas_question_backend_session(imathas_question_backend_session_id, account_id, expires_at) WHERE revoked_at IS NULL AND consumed_at IS NULL;

ALTER TABLE ple_private.imathas_render_cache_entry ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.imathas_render_cache_entry FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.imathas_question_backend_session ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.imathas_question_backend_session FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.imathas_result_exchange ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.imathas_result_exchange FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.imathas_render_cache_entry, ple_private.imathas_question_backend_session, ple_private.imathas_result_exchange FROM PUBLIC;
CREATE POLICY imathas_render_cache_private_owner_access ON ple_private.imathas_render_cache_entry FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY imathas_session_private_owner_access ON ple_private.imathas_question_backend_session FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY imathas_exchange_private_owner_access ON ple_private.imathas_result_exchange FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
REVOKE ALL ON FUNCTION ple_private.enforce_imathas_question_backend_session_transition(), ple_private.enforce_imathas_result_exchange_transition() FROM PUBLIC;

RESET ROLE;
