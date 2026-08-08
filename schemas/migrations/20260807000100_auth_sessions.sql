-- MOD-API-AUTH: opaque, revocable, replica-safe database sessions.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_auth') THEN
        CREATE ROLE ple_auth NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOBYPASSRLS;
    END IF;
END
$$;

CREATE TABLE auth_session (
    session_hash character(64) PRIMARY KEY
        CHECK (session_hash ~ '^[0-9a-f]{64}$'),
    tenant_id uuid NOT NULL,
    user_id uuid NOT NULL,
    display_name text NOT NULL
        CHECK (char_length(btrim(display_name)) BETWEEN 1 AND 200),
    roles jsonb NOT NULL
        CHECK (
            jsonb_typeof(roles) = 'array'
            AND jsonb_array_length(roles) > 0
            AND roles <@ '["student", "instructor", "administrator"]'::jsonb
        ),
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    CHECK (expires_at > created_at)
);

CREATE INDEX auth_session_expiry_idx
    ON auth_session (expires_at)
    WHERE revoked_at IS NULL;

ALTER TABLE auth_session ENABLE ROW LEVEL SECURITY;
ALTER TABLE auth_session FORCE ROW LEVEL SECURITY;

-- Authentication happens before a tenant can be trusted. Scope this narrowly
-- privileged role by the one-way hash of the presented opaque credential; the
-- resolved row is the only source from which TenantContext may be constructed.
CREATE POLICY auth_session_presented_token ON auth_session
    USING (
        session_hash = NULLIF(current_setting('ple.session_hash', true), '')::character(64)
    )
    WITH CHECK (
        session_hash = NULLIF(current_setting('ple.session_hash', true), '')::character(64)
    );

GRANT USAGE ON SCHEMA public TO ple_auth;
GRANT SELECT, INSERT, UPDATE ON auth_session TO ple_auth;
REVOKE ALL ON auth_session FROM PUBLIC, ple_app, ple_student, ple_grader;
