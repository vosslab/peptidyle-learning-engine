-- MOD-ADP-IMATHAS: replica-safe, tenant-owned external grade exchange.
-- This table deliberately has no browser grants and stores only opaque server
-- correlation/state.  Provider URLs, bearer values, and raw transcripts do
-- not belong here.
CREATE TABLE external_tool_exchange (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    provider text NOT NULL CHECK (octet_length(provider) BETWEEN 1 AND 160),
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    seed bigint NOT NULL,
    source_object_id uuid NOT NULL,
    source_sha256 character(64) NOT NULL,
    integration_profile text NOT NULL CHECK (octet_length(integration_profile) BETWEEN 1 AND 160),
    response_sha256 bytea NOT NULL CHECK (octet_length(response_sha256) = 32),
    idempotency_key text NOT NULL CHECK (octet_length(idempotency_key) BETWEEN 1 AND 200),
    correlation bytea NOT NULL CHECK (octet_length(correlation) BETWEEN 1 AND 512),
    state text NOT NULL CHECK (state IN ('verifying', 'verified_pending', 'committed')),
    lease_token bytea CHECK (lease_token IS NULL OR octet_length(lease_token) = 32),
    lease_expires_at timestamptz,
    result_payload jsonb,
    result_sha256 character(64),
    transcript_object_id uuid,
    transcript_sha256 character(64),
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    retention_at timestamptz,
    PRIMARY KEY (tenant_id, attempt_id),
    CHECK ((state = 'verifying') = (lease_token IS NOT NULL AND lease_expires_at IS NOT NULL)),
    CHECK ((state <> 'verified_pending') OR (result_payload IS NOT NULL AND result_sha256 IS NOT NULL))
);

CREATE INDEX external_tool_exchange_lease_idx
    ON external_tool_exchange (state, lease_expires_at)
    WHERE state = 'verifying';
CREATE INDEX external_tool_exchange_retention_idx
    ON external_tool_exchange (tenant_id, retention_at)
    WHERE transcript_object_id IS NOT NULL;

ALTER TABLE external_tool_exchange ENABLE ROW LEVEL SECURITY;
ALTER TABLE external_tool_exchange FORCE ROW LEVEL SECURITY;
CREATE POLICY external_tool_exchange_tenant ON external_tool_exchange
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
GRANT SELECT, INSERT, UPDATE ON external_tool_exchange TO ple_app;

-- Launch sessions are separate from exchanges: a browser frame restart cannot
-- mint a different provider grade identity. Cookie material is hash-only.
CREATE TABLE external_tool_launch_session (
    launch_session_id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    provider text NOT NULL CHECK (octet_length(provider) BETWEEN 1 AND 160),
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    seed bigint NOT NULL,
    source_object_id uuid NOT NULL,
    source_sha256 character(64) NOT NULL,
    integration_profile text NOT NULL CHECK (octet_length(integration_profile) BETWEEN 1 AND 160),
    response_sha256 bytea NOT NULL CHECK (octet_length(response_sha256) = 32),
    token_sha256 bytea NOT NULL CHECK (octet_length(token_sha256) = 32),
    encrypted_provider_state bytea,
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    UNIQUE (tenant_id, launch_session_id)
);
CREATE INDEX external_tool_launch_session_lookup_idx
    ON external_tool_launch_session (tenant_id, attempt_id, actor_id, expires_at);
ALTER TABLE external_tool_launch_session ENABLE ROW LEVEL SECURITY;
ALTER TABLE external_tool_launch_session FORCE ROW LEVEL SECURITY;
CREATE POLICY external_tool_launch_session_tenant ON external_tool_launch_session
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
GRANT SELECT, INSERT, UPDATE ON external_tool_launch_session TO ple_app;
