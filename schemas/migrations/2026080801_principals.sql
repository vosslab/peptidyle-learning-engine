-- Initial database epoch: principals.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_app') THEN
        CREATE ROLE ple_app NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOBYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_auth') THEN
        CREATE ROLE ple_auth NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOBYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_student') THEN
        CREATE ROLE ple_student NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOBYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_catalog_ownership_broker') THEN
        CREATE ROLE ple_catalog_ownership_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT BYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_grader') THEN
        CREATE ROLE ple_grader NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOBYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_grading_reader') THEN
        CREATE ROLE ple_grading_reader LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOBYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_qti_staging_broker') THEN
        CREATE ROLE ple_qti_staging_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT BYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_qti_provenance_broker') THEN
        CREATE ROLE ple_qti_provenance_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOBYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_queue_broker') THEN
        CREATE ROLE ple_queue_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT BYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_retention_broker') THEN
        CREATE ROLE ple_retention_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOBYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_statistics_broker') THEN
        CREATE ROLE ple_statistics_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOBYPASSRLS;
    END IF;
END
$$;

REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO
    ple_app,
    ple_auth,
    ple_catalog_ownership_broker,
    ple_student,
    ple_grader,
    ple_grading_reader,
    ple_qti_staging_broker,
    ple_qti_provenance_broker,
    ple_queue_broker,
    ple_retention_broker,
    ple_statistics_broker;

CREATE FUNCTION public.ple_current_tenant() RETURNS uuid
    LANGUAGE sql STABLE PARALLEL SAFE
    AS $$
    SELECT NULLIF(current_setting('ple.tenant_id', true), '')::uuid
$$;

CREATE TABLE public.auth_session (
    session_hash character(64) NOT NULL,
    tenant_id uuid NOT NULL,
    user_id uuid NOT NULL,
    display_name text NOT NULL,
    roles jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    CONSTRAINT auth_session_check CHECK ((expires_at > created_at)),
    CONSTRAINT auth_session_display_name_check CHECK (((char_length(btrim(display_name)) >= 1) AND (char_length(btrim(display_name)) <= 200))),
    CONSTRAINT auth_session_roles_check CHECK (((jsonb_typeof(roles) = 'array'::text) AND (jsonb_array_length(roles) > 0) AND (roles <@ '["student", "instructor", "administrator"]'::jsonb))),
    CONSTRAINT auth_session_session_hash_check CHECK ((session_hash ~ '^[0-9a-f]{64}$'::text))
);

ALTER TABLE ONLY public.auth_session FORCE ROW LEVEL SECURITY;

ALTER TABLE ONLY public.auth_session
    ADD CONSTRAINT auth_session_pkey PRIMARY KEY (session_hash);

CREATE INDEX auth_session_expiry_idx ON public.auth_session USING btree (expires_at) WHERE (revoked_at IS NULL);

ALTER TABLE public.auth_session ENABLE ROW LEVEL SECURITY;

CREATE POLICY auth_session_presented_token ON public.auth_session USING ((session_hash = (NULLIF(current_setting('ple.session_hash'::text, true), ''::text))::character(64))) WITH CHECK ((session_hash = (NULLIF(current_setting('ple.session_hash'::text, true), ''::text))::character(64)));

GRANT SELECT,INSERT,UPDATE ON TABLE public.auth_session TO ple_auth;
GRANT SELECT ON TABLE public.auth_session TO ple_retention_broker;

REVOKE ALL ON FUNCTION public.ple_current_tenant() FROM PUBLIC;
GRANT ALL ON FUNCTION public.ple_current_tenant() TO ple_app;
GRANT ALL ON FUNCTION public.ple_current_tenant() TO ple_catalog_ownership_broker;
GRANT ALL ON FUNCTION public.ple_current_tenant() TO ple_student;
GRANT ALL ON FUNCTION public.ple_current_tenant() TO ple_grader;
GRANT ALL ON FUNCTION public.ple_current_tenant() TO ple_grading_reader;
GRANT ALL ON FUNCTION public.ple_current_tenant() TO ple_qti_staging_broker;
GRANT ALL ON FUNCTION public.ple_current_tenant() TO ple_qti_provenance_broker;
GRANT ALL ON FUNCTION public.ple_current_tenant() TO ple_statistics_broker;
GRANT ALL ON FUNCTION public.ple_current_tenant() TO ple_retention_broker;

CREATE VIEW public.ple_migration_state AS
SELECT version, success, checksum
  FROM public._sqlx_migrations;

REVOKE ALL ON public.ple_migration_state FROM PUBLIC;
GRANT SELECT ON public.ple_migration_state TO ple_app;
