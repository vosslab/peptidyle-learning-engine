-- SD1 private external-tool launches, External Question Provider Cache Entries, and passback state.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.assignment,
    ple_data.question_revision TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.external_question_provider_cache_entry (
    external_question_provider_cache_entry_id uuid PRIMARY KEY,
    provider_reference text NOT NULL CHECK (provider_reference ~ '^[A-Za-z0-9._-]{1,160}$'),
    resource_digest bytea NOT NULL CHECK (pg_catalog.octet_length(resource_digest) = 32),
    encrypted_payload bytea NOT NULL CHECK (pg_catalog.octet_length(encrypted_payload) BETWEEN 1 AND 1048576),
    payload_digest bytea NOT NULL CHECK (pg_catalog.octet_length(payload_digest) = 32),
    fetched_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > fetched_at),
    UNIQUE (provider_reference, resource_digest)
);
CREATE TABLE ple_private.external_tool_launch_session (
    launch_session_id uuid PRIMARY KEY,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    attempt_id uuid NOT NULL REFERENCES ple_private.question_attempt (question_attempt_id),
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    provider_reference text NOT NULL CHECK (provider_reference ~ '^[A-Za-z0-9._-]{1,160}$'),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    source_object_id uuid NOT NULL,
    source_object_checksum bytea NOT NULL CHECK (pg_catalog.octet_length(source_object_checksum) = 32),
    integration_profile text NOT NULL CHECK (integration_profile ~ '^[A-Za-z0-9._-]{1,160}$'),
    response_sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(response_sha256) = 32),
    token_sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(token_sha256) = 32),
    external_tool_launch_challenge bytea NOT NULL
        CHECK (
            pg_catalog.octet_length(external_tool_launch_challenge) = 32
            AND external_tool_launch_challenge <> pg_catalog.decode(pg_catalog.repeat('00', 32), 'hex')
        ),
    external_tool_launch_session_authentication bytea NOT NULL
        CONSTRAINT external_tool_launch_session_authentication_is_canonical CHECK (
            pg_catalog.octet_length(external_tool_launch_session_authentication) BETWEEN 3 AND 512
            AND pg_catalog.convert_from(external_tool_launch_session_authentication, 'UTF8')
                ~ '^([0-9a-f]{2})+[.][0-9a-f]{64}$'
        ),
    encrypted_provider_state bytea CHECK (pg_catalog.octet_length(encrypted_provider_state) BETWEEN 1 AND 65536),
    issued_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > issued_at),
    revoked_at timestamp with time zone,
    consumed_at timestamp with time zone,
    activity_lease_token_sha256 bytea CHECK (pg_catalog.octet_length(activity_lease_token_sha256) = 32),
    activity_lease_expires_at timestamp with time zone,
    CONSTRAINT external_tool_launch_assignment_matches FOREIGN KEY (course_id, assignment_id)
        REFERENCES ple_data.assignment (course_id, assignment_id),
    CONSTRAINT external_tool_launch_version_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT external_tool_launch_revocation_is_ordered CHECK (revoked_at IS NULL OR revoked_at >= issued_at),
    CONSTRAINT external_tool_launch_consumption_is_ordered CHECK (consumed_at IS NULL OR consumed_at >= issued_at),
    CONSTRAINT external_tool_launch_activity_lease_matches CHECK (
        (activity_lease_token_sha256 IS NULL AND activity_lease_expires_at IS NULL)
        OR (activity_lease_token_sha256 IS NOT NULL AND activity_lease_expires_at IS NOT NULL)
    )
);
CREATE TABLE ple_private.external_tool_exchange (
    launch_session_id uuid PRIMARY KEY,
    idempotency_key text NOT NULL CHECK (pg_catalog.octet_length(idempotency_key) BETWEEN 1 AND 200),
    state text NOT NULL,
    lease_token_sha256 bytea CHECK (pg_catalog.octet_length(lease_token_sha256) = 32),
    lease_expires_at timestamp with time zone,
    verification_token_sha256 bytea CHECK (pg_catalog.octet_length(verification_token_sha256) = 32),
    external_tool_result jsonb,
    external_tool_result_checksum bytea CHECK (pg_catalog.octet_length(external_tool_result_checksum) = 32),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    committed_at timestamp with time zone,
    failed_at timestamp with time zone,
    cancelled_at timestamp with time zone,
    failure_code text CHECK (failure_code IS NULL OR char_length(btrim(failure_code)) BETWEEN 1 AND 160),
    CONSTRAINT external_tool_exchange_launch_session_matches FOREIGN KEY (launch_session_id)
        REFERENCES ple_private.external_tool_launch_session (launch_session_id),
    CONSTRAINT external_tool_exchange_state_is_closed
        CHECK (state IN ('verifying', 'ready_to_commit', 'committed', 'failed', 'cancelled')),
    CONSTRAINT external_tool_exchange_state_matches CHECK (
        (state = 'verifying' AND lease_token_sha256 IS NOT NULL AND lease_expires_at IS NOT NULL
            AND verification_token_sha256 IS NULL AND external_tool_result IS NULL AND external_tool_result_checksum IS NULL
            AND committed_at IS NULL AND failed_at IS NULL AND cancelled_at IS NULL AND failure_code IS NULL)
        OR (state = 'ready_to_commit' AND lease_token_sha256 IS NULL AND lease_expires_at IS NULL
            AND verification_token_sha256 IS NOT NULL AND external_tool_result IS NOT NULL AND external_tool_result_checksum IS NOT NULL
            AND committed_at IS NULL AND failed_at IS NULL AND cancelled_at IS NULL AND failure_code IS NULL)
        OR (state = 'committed' AND lease_token_sha256 IS NULL AND lease_expires_at IS NULL
            AND verification_token_sha256 IS NOT NULL AND external_tool_result IS NOT NULL AND external_tool_result_checksum IS NOT NULL
            AND committed_at IS NOT NULL AND failed_at IS NULL AND cancelled_at IS NULL AND failure_code IS NULL)
        OR (state = 'failed' AND lease_token_sha256 IS NULL AND lease_expires_at IS NULL
            AND verification_token_sha256 IS NULL AND external_tool_result IS NULL AND external_tool_result_checksum IS NULL
            AND committed_at IS NULL AND failed_at IS NOT NULL AND cancelled_at IS NULL AND failure_code IS NOT NULL)
        OR (state = 'cancelled' AND lease_token_sha256 IS NULL AND lease_expires_at IS NULL
            AND verification_token_sha256 IS NULL AND external_tool_result IS NULL AND external_tool_result_checksum IS NULL
            AND committed_at IS NULL AND failed_at IS NULL AND cancelled_at IS NOT NULL AND failure_code IS NULL)
    ),
    CONSTRAINT external_tool_exchange_transition_times_are_ordered CHECK (
        updated_at >= created_at
        AND (committed_at IS NULL OR committed_at BETWEEN created_at AND updated_at)
        AND (failed_at IS NULL OR failed_at BETWEEN created_at AND updated_at)
        AND (cancelled_at IS NULL OR cancelled_at BETWEEN created_at AND updated_at)
    )
);
CREATE TABLE ple_private.lti_grade_return (
    lti_grade_return_id uuid PRIMARY KEY,
    launch_session_id uuid NOT NULL,
    assignment_grade_id uuid NOT NULL REFERENCES ple_private.assignment_grade (assignment_grade_id),
    delivery_state text NOT NULL,
    lti_grade_return_payload_digest bytea NOT NULL CHECK (pg_catalog.octet_length(lti_grade_return_payload_digest) = 32),
    created_at timestamp with time zone NOT NULL,
    delivered_at timestamp with time zone,
    failed_at timestamp with time zone,
    cancelled_at timestamp with time zone,
    failure_code text CHECK (failure_code IS NULL OR char_length(btrim(failure_code)) BETWEEN 1 AND 160),
    UNIQUE (launch_session_id, lti_grade_return_payload_digest),
    CONSTRAINT lti_grade_return_launch_session_matches FOREIGN KEY (launch_session_id)
        REFERENCES ple_private.external_tool_launch_session (launch_session_id),
    CONSTRAINT lti_grade_return_state_is_closed
        CHECK (delivery_state IN ('requested', 'delivered', 'failed', 'cancelled')),
    CONSTRAINT lti_grade_return_state_matches CHECK (
        (delivery_state = 'requested' AND delivered_at IS NULL AND failed_at IS NULL AND cancelled_at IS NULL AND failure_code IS NULL)
        OR (delivery_state = 'delivered' AND delivered_at IS NOT NULL AND failed_at IS NULL AND cancelled_at IS NULL AND failure_code IS NULL)
        OR (delivery_state = 'failed' AND delivered_at IS NULL AND failed_at IS NOT NULL AND cancelled_at IS NULL AND failure_code IS NOT NULL)
        OR (delivery_state = 'cancelled' AND delivered_at IS NULL AND failed_at IS NULL AND cancelled_at IS NOT NULL AND failure_code IS NULL)
    ),
    CONSTRAINT lti_grade_return_transition_times_are_ordered CHECK (
        (delivered_at IS NULL OR delivered_at >= created_at)
        AND (failed_at IS NULL OR failed_at >= created_at)
        AND (cancelled_at IS NULL OR cancelled_at >= created_at)
    )
);
-- ASVS 2.3.1 and 2.3.4: launch state is a one-use server capability. A
-- successful provider-result transition consumes it under the same row lock.
CREATE FUNCTION ple_private.enforce_external_tool_launch_session_transition()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.external_tool_launch_challenge IS DISTINCT FROM OLD.external_tool_launch_challenge
       OR NEW.external_tool_launch_session_authentication IS DISTINCT FROM OLD.external_tool_launch_session_authentication THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'External Tool Launch Session authentication is immutable';
    END IF;
    IF OLD.consumed_at IS NOT NULL AND NEW.consumed_at IS DISTINCT FROM OLD.consumed_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'External Tool Launch Session cannot be consumed twice';
    END IF;
    IF NEW.consumed_at IS NOT NULL AND NEW.consumed_at < OLD.issued_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'External Tool Launch Session consumption predates issue';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER external_tool_launch_session_transition_is_forward_only
BEFORE UPDATE ON ple_private.external_tool_launch_session
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_external_tool_launch_session_transition();

-- ASVS 2.3.1, 2.3.3, and 2.3.4: only the ordered verification workflow may
-- advance an exchange, and its ready-to-commit transition atomically consumes
-- one active launch session with its exact lease.
CREATE FUNCTION ple_private.enforce_external_tool_exchange_transition()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.state <> 'verifying' OR NOT EXISTS (
            SELECT 1
              FROM ple_private.external_tool_launch_session AS launch_session
             WHERE launch_session.launch_session_id = NEW.launch_session_id
               AND launch_session.revoked_at IS NULL
               AND launch_session.consumed_at IS NULL
               AND launch_session.expires_at > NEW.updated_at
               AND launch_session.activity_lease_token_sha256 = NEW.lease_token_sha256
               AND launch_session.activity_lease_expires_at > NEW.updated_at
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'External Tool Exchange requires one active leased Launch Session';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.updated_at < OLD.updated_at OR NEW.state = OLD.state THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'External Tool Exchange transition is not forward';
    END IF;
    IF NOT (
        (OLD.state = 'verifying' AND NEW.state IN ('ready_to_commit', 'failed', 'cancelled'))
        OR (OLD.state = 'ready_to_commit' AND NEW.state IN ('committed', 'failed', 'cancelled'))
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'External Tool Exchange transition is not allowed';
    END IF;
    IF OLD.state = 'verifying' AND NEW.state = 'ready_to_commit' THEN
        UPDATE ple_private.external_tool_launch_session AS launch_session
           SET consumed_at = NEW.updated_at
         WHERE launch_session.launch_session_id = NEW.launch_session_id
           AND launch_session.revoked_at IS NULL
           AND launch_session.consumed_at IS NULL
           AND launch_session.expires_at > NEW.updated_at
           AND launch_session.activity_lease_token_sha256 = OLD.lease_token_sha256
           AND launch_session.activity_lease_expires_at > NEW.updated_at;
        IF NOT FOUND THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'External Tool Launch Session is expired, revoked, consumed, or not leased by this exchange';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER external_tool_exchange_transition_is_forward_only
BEFORE INSERT OR UPDATE ON ple_private.external_tool_exchange
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_external_tool_exchange_transition();

-- ASVS 2.2.3: an LTI Grade Return joins directly to its Launch Session and
-- its selected Assignment Grade must belong to that exact Student/Course/Assignment.
CREATE FUNCTION ple_private.enforce_lti_grade_return_context_and_transition()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private, ple_data AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.external_tool_launch_session AS launch_session
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.question_attempt_id = launch_session.attempt_id
          JOIN ple_private.issued_question AS issued_question
            ON issued_question.issued_question_id = question_attempt.issued_question_id
          JOIN ple_private.assignment_attempt AS assignment_attempt
            ON assignment_attempt.assignment_attempt_id = issued_question.assignment_attempt_id
          JOIN ple_data.student_record AS student_record
            ON student_record.student_record_id = assignment_attempt.student_record_id
          JOIN ple_private.assignment_grade AS assignment_grade
            ON assignment_grade.assignment_grade_id = NEW.assignment_grade_id
         WHERE launch_session.launch_session_id = NEW.launch_session_id
           AND assignment_grade.student_record_id = assignment_attempt.student_record_id
           AND assignment_grade.assignment_id = assignment_attempt.assignment_id
           AND launch_session.course_id = student_record.course_id
           AND launch_session.assignment_id = assignment_attempt.assignment_id
           AND launch_session.account_id = student_record.student_account_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'LTI Grade Return Assignment Grade does not match its Launch Session Student/Course/Assignment';
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.delivery_state <> 'requested' THEN
            RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'LTI Grade Return must begin requested';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.delivery_state = OLD.delivery_state OR NOT (
        OLD.delivery_state = 'requested' AND NEW.delivery_state IN ('delivered', 'failed', 'cancelled')
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'LTI Grade Return transition is not forward';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER lti_grade_return_context_and_transition_is_forward_only
BEFORE INSERT OR UPDATE ON ple_private.lti_grade_return
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_lti_grade_return_context_and_transition();
CREATE INDEX external_tool_launch_active_idx
    ON ple_private.external_tool_launch_session (attempt_id, account_id, expires_at)
    WHERE revoked_at IS NULL;
CREATE INDEX external_tool_exchange_active_lease_idx
    ON ple_private.external_tool_exchange (lease_expires_at) WHERE state = 'verifying';
ALTER TABLE ple_private.external_question_provider_cache_entry ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.external_question_provider_cache_entry FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.external_tool_launch_session ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.external_tool_launch_session FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.external_tool_exchange ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.external_tool_exchange FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.lti_grade_return ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.lti_grade_return FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.external_question_provider_cache_entry,
    ple_private.external_tool_launch_session, ple_private.external_tool_exchange,
    ple_private.lti_grade_return FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.enforce_external_tool_launch_session_transition(),
    ple_private.enforce_external_tool_exchange_transition(),
    ple_private.enforce_lti_grade_return_context_and_transition() FROM PUBLIC;
RESET ROLE;
