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
    encrypted_provider_state bytea CHECK (pg_catalog.octet_length(encrypted_provider_state) BETWEEN 1 AND 65536),
    issued_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > issued_at),
    revoked_at timestamp with time zone,
    activity_lease_token_sha256 bytea CHECK (pg_catalog.octet_length(activity_lease_token_sha256) = 32),
    activity_lease_expires_at timestamp with time zone,
    CONSTRAINT external_tool_launch_assignment_matches FOREIGN KEY (course_id, assignment_id)
        REFERENCES ple_data.assignment (course_id, assignment_id),
    CONSTRAINT external_tool_launch_version_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT external_tool_launch_revocation_is_ordered CHECK (revoked_at IS NULL OR revoked_at >= issued_at),
    CONSTRAINT external_tool_launch_activity_lease_matches CHECK (
        (activity_lease_token_sha256 IS NULL AND activity_lease_expires_at IS NULL)
        OR (activity_lease_token_sha256 IS NOT NULL AND activity_lease_expires_at IS NOT NULL)
    )
);
CREATE TABLE ple_private.external_tool_exchange (
    attempt_id uuid PRIMARY KEY REFERENCES ple_private.question_attempt (question_attempt_id),
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    provider_reference text NOT NULL CHECK (provider_reference ~ '^[A-Za-z0-9._-]{1,160}$'),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    source_object_id uuid NOT NULL,
    source_object_checksum bytea NOT NULL CHECK (pg_catalog.octet_length(source_object_checksum) = 32),
    integration_profile text NOT NULL CHECK (integration_profile ~ '^[A-Za-z0-9._-]{1,160}$'),
    response_sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(response_sha256) = 32),
    idempotency_key text NOT NULL CHECK (pg_catalog.octet_length(idempotency_key) BETWEEN 1 AND 200),
    correlation bytea NOT NULL CHECK (pg_catalog.octet_length(correlation) BETWEEN 1 AND 512),
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
    CONSTRAINT external_tool_exchange_assignment_matches FOREIGN KEY (course_id, assignment_id)
        REFERENCES ple_data.assignment (course_id, assignment_id),
    CONSTRAINT external_tool_exchange_version_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
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
        AND (committed_at IS NULL OR committed_at >= created_at)
        AND (failed_at IS NULL OR failed_at >= created_at)
        AND (cancelled_at IS NULL OR cancelled_at >= created_at)
    )
);
CREATE TABLE ple_private.lti_grade_return (
    lti_grade_return_id uuid PRIMARY KEY,
    question_attempt_id uuid NOT NULL REFERENCES ple_private.external_tool_exchange (attempt_id),
    assignment_grade_id uuid NOT NULL REFERENCES ple_private.assignment_grade (assignment_grade_id),
    delivery_state text NOT NULL,
    lti_grade_return_payload_digest bytea NOT NULL CHECK (pg_catalog.octet_length(lti_grade_return_payload_digest) = 32),
    created_at timestamp with time zone NOT NULL,
    delivered_at timestamp with time zone,
    failed_at timestamp with time zone,
    cancelled_at timestamp with time zone,
    failure_code text CHECK (failure_code IS NULL OR char_length(btrim(failure_code)) BETWEEN 1 AND 160),
    UNIQUE (question_attempt_id, lti_grade_return_payload_digest),
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
RESET ROLE;
