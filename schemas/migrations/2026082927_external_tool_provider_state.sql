-- SD1 private external-tool launches, provider cache, and passback state.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance_assignment_delivery,
    ple_data.published_question_version TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.external_tool_provider_cache (
    cache_id uuid PRIMARY KEY,
    provider_key text NOT NULL CHECK (provider_key ~ '^[A-Za-z0-9._-]{1,160}$'),
    resource_key_digest bytea NOT NULL CHECK (pg_catalog.octet_length(resource_key_digest) = 32),
    encrypted_payload bytea NOT NULL CHECK (pg_catalog.octet_length(encrypted_payload) BETWEEN 1 AND 1048576),
    payload_digest bytea NOT NULL CHECK (pg_catalog.octet_length(payload_digest) = 32),
    fetched_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > fetched_at),
    UNIQUE (provider_key, resource_key_digest)
);
CREATE TABLE ple_private.external_tool_launch_session (
    launch_session_id uuid PRIMARY KEY,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    attempt_id uuid NOT NULL REFERENCES ple_private.question_attempt (attempt_id),
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    provider_key text NOT NULL CHECK (provider_key ~ '^[A-Za-z0-9._-]{1,160}$'),
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    source_object_id uuid NOT NULL,
    source_sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(source_sha256) = 32),
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
        REFERENCES ple_data.course_instance_assignment_delivery (course_id, assignment_id),
    CONSTRAINT external_tool_launch_version_matches FOREIGN KEY (problem_id, version_id)
        REFERENCES ple_data.published_question_version (problem_id, version_id),
    CONSTRAINT external_tool_launch_revocation_is_ordered CHECK (revoked_at IS NULL OR revoked_at >= issued_at),
    CONSTRAINT external_tool_launch_activity_lease_matches CHECK (
        (activity_lease_token_sha256 IS NULL AND activity_lease_expires_at IS NULL)
        OR (activity_lease_token_sha256 IS NOT NULL AND activity_lease_expires_at IS NOT NULL)
    )
);
CREATE TABLE ple_private.external_tool_exchange (
    attempt_id uuid PRIMARY KEY REFERENCES ple_private.question_attempt (attempt_id),
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    provider_key text NOT NULL CHECK (provider_key ~ '^[A-Za-z0-9._-]{1,160}$'),
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    source_object_id uuid NOT NULL,
    source_sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(source_sha256) = 32),
    integration_profile text NOT NULL CHECK (integration_profile ~ '^[A-Za-z0-9._-]{1,160}$'),
    response_sha256 bytea NOT NULL CHECK (pg_catalog.octet_length(response_sha256) = 32),
    idempotency_key text NOT NULL CHECK (pg_catalog.octet_length(idempotency_key) BETWEEN 1 AND 200),
    correlation bytea NOT NULL CHECK (pg_catalog.octet_length(correlation) BETWEEN 1 AND 512),
    state text NOT NULL CHECK (state IN ('verifying', 'verified_pending', 'committed')),
    lease_token_sha256 bytea CHECK (pg_catalog.octet_length(lease_token_sha256) = 32),
    lease_expires_at timestamp with time zone,
    verification_token_sha256 bytea CHECK (pg_catalog.octet_length(verification_token_sha256) = 32),
    result_payload jsonb,
    result_sha256 bytea CHECK (pg_catalog.octet_length(result_sha256) = 32),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT external_tool_exchange_assignment_matches FOREIGN KEY (course_id, assignment_id)
        REFERENCES ple_data.course_instance_assignment_delivery (course_id, assignment_id),
    CONSTRAINT external_tool_exchange_version_matches FOREIGN KEY (problem_id, version_id)
        REFERENCES ple_data.published_question_version (problem_id, version_id),
    CONSTRAINT external_tool_exchange_lease_matches CHECK (
        (state = 'verifying' AND lease_token_sha256 IS NOT NULL AND lease_expires_at IS NOT NULL
            AND verification_token_sha256 IS NULL AND result_payload IS NULL AND result_sha256 IS NULL)
        OR (state = 'verified_pending' AND lease_token_sha256 IS NULL AND lease_expires_at IS NULL
            AND verification_token_sha256 IS NOT NULL AND result_payload IS NOT NULL AND result_sha256 IS NOT NULL)
        OR (state = 'committed' AND lease_token_sha256 IS NULL AND lease_expires_at IS NULL
            AND verification_token_sha256 IS NOT NULL AND result_payload IS NOT NULL AND result_sha256 IS NOT NULL)
    )
);
CREATE TABLE ple_private.external_tool_passback_state (
    passback_id uuid PRIMARY KEY,
    attempt_id uuid NOT NULL REFERENCES ple_private.external_tool_exchange (attempt_id),
    provider_key text NOT NULL CHECK (provider_key ~ '^[A-Za-z0-9._-]{1,160}$'),
    state text NOT NULL CHECK (state IN ('pending', 'delivered', 'failed', 'cancelled')),
    outbound_digest bytea NOT NULL CHECK (pg_catalog.octet_length(outbound_digest) = 32),
    created_at timestamp with time zone NOT NULL,
    delivered_at timestamp with time zone,
    UNIQUE (attempt_id, outbound_digest),
    CONSTRAINT external_tool_passback_delivery_is_ordered CHECK (delivered_at IS NULL OR delivered_at >= created_at)
);
CREATE INDEX external_tool_launch_active_idx
    ON ple_private.external_tool_launch_session (attempt_id, account_id, expires_at)
    WHERE revoked_at IS NULL;
CREATE INDEX external_tool_exchange_active_lease_idx
    ON ple_private.external_tool_exchange (lease_expires_at) WHERE state = 'verifying';
ALTER TABLE ple_private.external_tool_provider_cache ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.external_tool_provider_cache FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.external_tool_launch_session ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.external_tool_launch_session FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.external_tool_exchange ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.external_tool_exchange FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.external_tool_passback_state ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.external_tool_passback_state FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.external_tool_provider_cache,
    ple_private.external_tool_launch_session, ple_private.external_tool_exchange,
    ple_private.external_tool_passback_state FROM PUBLIC;
RESET ROLE;
