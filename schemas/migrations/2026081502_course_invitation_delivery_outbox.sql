-- Durable invitation-delivery intent. SMTP correlation is not provider
-- idempotency: ambiguous submission outcomes intentionally never auto-retry.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_invitation_delivery_broker') THEN
        CREATE ROLE ple_invitation_delivery_broker
            NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
    END IF;
END
$$;

ALTER ROLE ple_invitation_delivery_broker
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_auth_members AS membership
        WHERE membership.member = (SELECT oid FROM pg_roles WHERE rolname = 'ple_invitation_delivery_broker')
           OR membership.roleid = (SELECT oid FROM pg_roles WHERE rolname = 'ple_invitation_delivery_broker')
    ) THEN
        RAISE EXCEPTION 'ple_invitation_delivery_broker must not have role memberships';
    END IF;
END
$$;
REVOKE ALL ON SCHEMA public FROM ple_invitation_delivery_broker;
GRANT USAGE ON SCHEMA public TO ple_invitation_delivery_broker;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_invitation_delivery_worker') THEN
        CREATE ROLE ple_invitation_delivery_worker
            NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
    END IF;
END
$$;
ALTER ROLE ple_invitation_delivery_worker
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
REVOKE ALL ON SCHEMA public FROM ple_invitation_delivery_worker;
GRANT USAGE ON SCHEMA public TO ple_invitation_delivery_worker;

CREATE TABLE public.course_invitation_delivery (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    invitation_id uuid NOT NULL,
    delivery_id uuid NOT NULL,
    state text NOT NULL DEFAULT 'pending',
    attempt_count integer NOT NULL DEFAULT 0,
    next_attempt_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    last_attempt_at timestamptz,
    lease_id uuid,
    lease_expires_at timestamptz,
    dispatch_started_at timestamptz,
    outcome_code text,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    accepted_at timestamptz,
    terminal_at timestamptz,
    PRIMARY KEY (tenant_id, course_id, invitation_id),
    UNIQUE (delivery_id),
    FOREIGN KEY (tenant_id, course_id, invitation_id)
        REFERENCES public.course_invitation(tenant_id, course_id, invitation_id) ON DELETE CASCADE,
    CONSTRAINT course_invitation_delivery_state_check CHECK (state IN (
        'pending', 'accepted_by_provider', 'retryable_failed', 'ambiguous',
        'permanent_failed', 'cancelled'
    )),
    CONSTRAINT course_invitation_delivery_attempt_check CHECK (attempt_count >= 0),
    CONSTRAINT course_invitation_delivery_lease_check CHECK (
        (lease_id IS NULL AND lease_expires_at IS NULL)
        OR (lease_id IS NOT NULL AND lease_expires_at IS NOT NULL)
    ),
    CONSTRAINT course_invitation_delivery_dispatch_check CHECK (
        dispatch_started_at IS NULL OR lease_id IS NOT NULL
    ),
    CONSTRAINT course_invitation_delivery_outcome_check CHECK (outcome_code IS NULL OR outcome_code IN (
        'accepted', 'temporary_failure', 'permanent_failure', 'ambiguous_transport', 'cancelled'
    ))
);

REVOKE ALL ON TABLE public.course_invitation_delivery,
                    public.course_invitation,
                    public.course_roster_import
    FROM ple_invitation_delivery_worker;

INSERT INTO public.course_invitation_delivery (
    tenant_id, course_id, invitation_id, delivery_id, state, outcome_code, terminal_at
)
SELECT invitation.tenant_id, invitation.course_id, invitation.invitation_id, gen_random_uuid(),
       CASE WHEN invitation.status = 'pending' AND invitation.expires_at > transaction_timestamp()
            THEN 'pending' ELSE 'cancelled' END,
       CASE WHEN invitation.status = 'pending' AND invitation.expires_at > transaction_timestamp()
            THEN NULL ELSE 'cancelled' END,
       CASE WHEN invitation.status = 'pending' AND invitation.expires_at > transaction_timestamp()
            THEN NULL ELSE transaction_timestamp() END
  FROM public.course_invitation AS invitation
ON CONFLICT (tenant_id, course_id, invitation_id) DO NOTHING;

CREATE INDEX course_invitation_delivery_due_idx
    ON public.course_invitation_delivery (next_attempt_at, delivery_id)
    WHERE state IN ('pending', 'retryable_failed');
CREATE INDEX course_invitation_delivery_lease_idx
    ON public.course_invitation_delivery (lease_expires_at, delivery_id)
    WHERE lease_id IS NOT NULL;

ALTER TABLE public.course_invitation_delivery ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.course_invitation_delivery FORCE ROW LEVEL SECURITY;

CREATE POLICY course_invitation_delivery_app_tenant_read
    ON public.course_invitation_delivery FOR SELECT TO ple_app
    USING (tenant_id = public.ple_current_tenant()
       AND public.ple_course_records_accessible(tenant_id, course_id));
CREATE POLICY course_invitation_delivery_app_tenant_insert
    ON public.course_invitation_delivery FOR INSERT TO ple_app
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY course_invitation_delivery_broker_all
    ON public.course_invitation_delivery FOR ALL TO ple_invitation_delivery_broker
    USING (true) WITH CHECK (true);
CREATE POLICY course_invitation_delivery_retention_delete
    ON public.course_invitation_delivery FOR DELETE TO ple_retention_broker
    USING (true);
CREATE POLICY course_invitation_delivery_broker_invitation_read ON public.course_invitation
    FOR SELECT TO ple_invitation_delivery_broker USING (true);
CREATE POLICY course_invitation_delivery_broker_import_read ON public.course_roster_import
    FOR SELECT TO ple_invitation_delivery_broker USING (true);

CREATE OR REPLACE FUNCTION public.ple_cancel_ineligible_invitation_deliveries()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
BEGIN
    IF NEW.status IN ('claimed', 'expired', 'revoked') THEN
        UPDATE public.course_invitation_delivery
           SET state = CASE WHEN dispatch_started_at IS NULL THEN 'cancelled' ELSE 'ambiguous' END,
               outcome_code = CASE WHEN dispatch_started_at IS NULL THEN 'cancelled' ELSE 'ambiguous_transport' END,
               lease_id = NULL,
               lease_expires_at = NULL, dispatch_started_at = NULL, updated_at = transaction_timestamp(),
               terminal_at = COALESCE(terminal_at, transaction_timestamp())
         WHERE tenant_id = NEW.tenant_id AND course_id = NEW.course_id
           AND invitation_id = NEW.invitation_id
           AND state IN ('pending', 'retryable_failed');
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER course_invitation_delivery_cancel_on_invitation_close
    AFTER UPDATE OF status ON public.course_invitation
    FOR EACH ROW EXECUTE FUNCTION public.ple_cancel_ineligible_invitation_deliveries();

CREATE OR REPLACE FUNCTION public.ple_claim_course_invitation_deliveries(
    p_limit integer, p_lease_seconds integer
) RETURNS TABLE(
    tenant_id uuid, course_id uuid, invitation_id uuid, delivery_id uuid,
    state text, attempt_count integer, next_attempt_at timestamptz, last_attempt_at timestamptz,
    lease_id uuid, lease_expires_at timestamptz, dispatch_started_at timestamptz,
    outcome_code text, created_at timestamptz, updated_at timestamptz, accepted_at timestamptz,
    terminal_at timestamptz
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
BEGIN
    IF p_limit NOT BETWEEN 1 AND 100 OR p_lease_seconds NOT BETWEEN 1 AND 900 THEN
        RAISE EXCEPTION 'invalid invitation delivery claim arguments' USING ERRCODE = '22023';
    END IF;
    UPDATE public.course_invitation_delivery AS delivery
       SET state = CASE WHEN delivery.dispatch_started_at IS NULL THEN 'cancelled' ELSE 'ambiguous' END,
           outcome_code = CASE WHEN delivery.dispatch_started_at IS NULL THEN 'cancelled' ELSE 'ambiguous_transport' END,
           lease_id = NULL, lease_expires_at = NULL, dispatch_started_at = NULL,
           updated_at = transaction_timestamp(), terminal_at = transaction_timestamp()
      FROM public.course_invitation AS invitation
     WHERE (invitation.tenant_id, invitation.course_id, invitation.invitation_id)
         = (delivery.tenant_id, delivery.course_id, delivery.invitation_id)
       AND invitation.expires_at <= transaction_timestamp()
       AND delivery.state IN ('pending', 'retryable_failed');
    UPDATE public.course_invitation_delivery
       SET state = CASE WHEN dispatch_started_at IS NULL THEN state ELSE 'ambiguous' END,
           outcome_code = CASE WHEN dispatch_started_at IS NULL THEN outcome_code ELSE 'ambiguous_transport' END,
           lease_id = NULL, lease_expires_at = NULL, dispatch_started_at = NULL,
           updated_at = transaction_timestamp(),
           terminal_at = CASE WHEN dispatch_started_at IS NULL THEN terminal_at ELSE transaction_timestamp() END
     WHERE state IN ('pending', 'retryable_failed')
       AND lease_expires_at <= transaction_timestamp();
    UPDATE public.course_invitation_delivery
       SET state = 'permanent_failed', outcome_code = 'permanent_failure',
           lease_id = NULL, lease_expires_at = NULL, dispatch_started_at = NULL,
           updated_at = transaction_timestamp(), terminal_at = transaction_timestamp()
     WHERE state IN ('pending', 'retryable_failed') AND attempt_count >= 3
       AND (lease_expires_at IS NULL OR lease_expires_at <= transaction_timestamp());
    RETURN QUERY
    WITH candidates AS (
        SELECT delivery.tenant_id, delivery.course_id, delivery.invitation_id
          FROM public.course_invitation_delivery AS delivery
          JOIN public.course_invitation AS invitation
            ON (invitation.tenant_id, invitation.course_id, invitation.invitation_id)
             = (delivery.tenant_id, delivery.course_id, delivery.invitation_id)
         WHERE delivery.state IN ('pending', 'retryable_failed')
           AND delivery.next_attempt_at <= transaction_timestamp()
           AND delivery.attempt_count < 3
           AND (delivery.lease_expires_at IS NULL OR delivery.lease_expires_at <= transaction_timestamp())
           AND invitation.status = 'pending' AND invitation.expires_at > transaction_timestamp()
         ORDER BY delivery.next_attempt_at, delivery.delivery_id
         FOR UPDATE OF delivery SKIP LOCKED
         LIMIT p_limit
    ), claimed AS (
        UPDATE public.course_invitation_delivery AS delivery
           SET lease_id = gen_random_uuid(), dispatch_started_at = NULL,
               lease_expires_at = transaction_timestamp() + make_interval(secs => p_lease_seconds),
               attempt_count = delivery.attempt_count + 1,
               last_attempt_at = transaction_timestamp(), updated_at = transaction_timestamp()
          FROM candidates
         WHERE (delivery.tenant_id, delivery.course_id, delivery.invitation_id)
             = (candidates.tenant_id, candidates.course_id, candidates.invitation_id)
        RETURNING delivery.tenant_id, delivery.course_id, delivery.invitation_id, delivery.delivery_id,
                  delivery.state, delivery.attempt_count, delivery.next_attempt_at, delivery.last_attempt_at,
                  delivery.lease_id, delivery.lease_expires_at, delivery.dispatch_started_at,
                  delivery.outcome_code, delivery.created_at, delivery.updated_at, delivery.accepted_at,
                  delivery.terminal_at
    ) SELECT * FROM claimed;
END
$$;

CREATE OR REPLACE FUNCTION public.ple_prepare_course_invitation_delivery(
    p_delivery_id uuid, p_lease_id uuid
) RETURNS TABLE(
    tenant_id uuid, course_id uuid, delivery_id uuid, lease_id uuid, delivery_email text, token_hash bytea,
    roster_id text, idempotency_key text, roster_import_id uuid,
    roster_import_row_number integer, commit_idempotency_key text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
DECLARE
    v_tenant_id uuid;
    v_course_id uuid;
    v_delivery_id uuid;
    v_lease_id uuid;
    v_delivery_email text;
    v_token_hash bytea;
    v_roster_id text;
    v_idempotency_key text;
    v_roster_import_id uuid;
    v_roster_import_row_number integer;
    v_commit_idempotency_key text;
BEGIN
    UPDATE public.course_invitation_delivery AS delivery
       SET dispatch_started_at = transaction_timestamp(), updated_at = transaction_timestamp()
      FROM public.course_invitation AS invitation
     WHERE delivery.delivery_id = p_delivery_id AND delivery.lease_id = p_lease_id
       AND delivery.lease_expires_at > transaction_timestamp()
       AND delivery.dispatch_started_at IS NULL
       AND delivery.state IN ('pending', 'retryable_failed')
       AND invitation.status = 'pending' AND invitation.expires_at > transaction_timestamp()
       AND (invitation.tenant_id, invitation.course_id, invitation.invitation_id)
         = (delivery.tenant_id, delivery.course_id, delivery.invitation_id)
    RETURNING delivery.tenant_id, delivery.course_id, delivery.delivery_id, delivery.lease_id, invitation.delivery_email, invitation.token_hash,
              invitation.roster_id, invitation.idempotency_key, invitation.roster_import_id,
              invitation.roster_import_row_number, NULL::text
      INTO v_tenant_id, v_course_id, v_delivery_id, v_lease_id, v_delivery_email, v_token_hash, v_roster_id, v_idempotency_key,
           v_roster_import_id, v_roster_import_row_number, v_commit_idempotency_key;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    IF v_roster_import_id IS NOT NULL THEN
        SELECT import.commit_idempotency_key INTO v_commit_idempotency_key
          FROM public.course_roster_import AS import
         WHERE import.tenant_id = v_tenant_id AND import.course_id = v_course_id
           AND import.roster_import_id = v_roster_import_id;
        IF v_commit_idempotency_key IS NULL THEN
            RAISE EXCEPTION 'invitation delivery import provenance is unavailable' USING ERRCODE = '55000';
        END IF;
    END IF;
    RETURN QUERY SELECT v_tenant_id, v_course_id, v_delivery_id, v_lease_id, v_delivery_email, v_token_hash,
                        v_roster_id, v_idempotency_key, v_roster_import_id,
                        v_roster_import_row_number, v_commit_idempotency_key;
END
$$;

CREATE OR REPLACE FUNCTION public.ple_complete_course_invitation_delivery(
    p_delivery_id uuid, p_lease_id uuid, p_state text, p_next_attempt_at timestamptz
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
DECLARE
    final_outcome text;
BEGIN
    IF p_delivery_id IS NULL OR p_lease_id IS NULL
       OR p_state NOT IN ('accepted_by_provider', 'retryable_failed', 'ambiguous', 'permanent_failed')
       OR (p_state = 'retryable_failed' AND p_next_attempt_at IS NULL)
       OR (p_state <> 'retryable_failed' AND p_next_attempt_at IS NOT NULL) THEN
        RAISE EXCEPTION 'invalid invitation delivery completion arguments' USING ERRCODE = '22023';
    END IF;
    final_outcome := CASE p_state
        WHEN 'accepted_by_provider' THEN 'accepted'
        WHEN 'retryable_failed' THEN 'temporary_failure'
        WHEN 'ambiguous' THEN 'ambiguous_transport'
        ELSE 'permanent_failure'
    END;
    UPDATE public.course_invitation_delivery AS delivery
           SET state = CASE
               WHEN (invitation.status <> 'pending' OR invitation.expires_at <= transaction_timestamp())
                    AND delivery.dispatch_started_at IS NOT NULL
                   THEN 'ambiguous'
               WHEN invitation.status <> 'pending' OR invitation.expires_at <= transaction_timestamp()
                   THEN 'cancelled'
               WHEN p_state = 'retryable_failed' AND delivery.attempt_count >= 3
                   THEN 'permanent_failed'
               ELSE p_state END,
           outcome_code = CASE
               WHEN (invitation.status <> 'pending' OR invitation.expires_at <= transaction_timestamp())
                    AND delivery.dispatch_started_at IS NOT NULL
                   THEN 'ambiguous_transport'
               WHEN invitation.status <> 'pending' OR invitation.expires_at <= transaction_timestamp()
                   THEN 'cancelled'
               WHEN p_state = 'retryable_failed' AND delivery.attempt_count >= 3
                   THEN 'permanent_failure'
               ELSE final_outcome END,
           next_attempt_at = COALESCE(p_next_attempt_at, delivery.next_attempt_at),
           lease_id = NULL, lease_expires_at = NULL, dispatch_started_at = NULL, updated_at = transaction_timestamp(),
           accepted_at = CASE
               WHEN p_state = 'accepted_by_provider' AND invitation.status = 'pending'
                    AND invitation.expires_at > transaction_timestamp()
                   THEN transaction_timestamp()
               ELSE NULL END,
           terminal_at = CASE WHEN p_state IN ('accepted_by_provider', 'ambiguous', 'permanent_failed')
                               OR (p_state = 'retryable_failed' AND delivery.attempt_count >= 3)
                               OR invitation.status <> 'pending' OR invitation.expires_at <= transaction_timestamp()
                              THEN transaction_timestamp() ELSE NULL END
      FROM public.course_invitation AS invitation
     WHERE delivery.delivery_id = p_delivery_id AND delivery.lease_id = p_lease_id
       AND delivery.lease_expires_at > transaction_timestamp()
       AND delivery.dispatch_started_at IS NOT NULL
       AND (invitation.tenant_id, invitation.course_id, invitation.invitation_id)
         = (delivery.tenant_id, delivery.course_id, delivery.invitation_id);
    RETURN FOUND;
END
$$;

CREATE OR REPLACE FUNCTION public.ple_revalidate_course_invitation_delivery_lease(
    p_delivery_id uuid, p_lease_id uuid
) RETURNS boolean LANGUAGE sql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
    SELECT EXISTS (
        SELECT 1
          FROM public.course_invitation_delivery AS delivery
          JOIN public.course_invitation AS invitation
            USING (tenant_id, course_id, invitation_id)
         WHERE delivery.delivery_id = p_delivery_id AND delivery.lease_id = p_lease_id
           AND delivery.lease_expires_at > transaction_timestamp()
           AND delivery.dispatch_started_at IS NOT NULL
           AND invitation.status = 'pending' AND invitation.expires_at > transaction_timestamp()
    )
$$;

CREATE OR REPLACE FUNCTION public.ple_invitation_delivery_worker_migration_state()
RETURNS TABLE(version bigint, success boolean, checksum bytea)
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT version, success, checksum FROM public.ple_migration_state ORDER BY version
$$;

ALTER FUNCTION public.ple_cancel_ineligible_invitation_deliveries() OWNER TO ple_invitation_delivery_broker;
ALTER FUNCTION public.ple_claim_course_invitation_deliveries(integer, integer)
    OWNER TO ple_invitation_delivery_broker;
ALTER FUNCTION public.ple_complete_course_invitation_delivery(uuid, uuid, text, timestamptz)
    OWNER TO ple_invitation_delivery_broker;
ALTER FUNCTION public.ple_prepare_course_invitation_delivery(uuid, uuid)
    OWNER TO ple_invitation_delivery_broker;
ALTER FUNCTION public.ple_revalidate_course_invitation_delivery_lease(uuid, uuid)
    OWNER TO ple_invitation_delivery_broker;
ALTER FUNCTION public.ple_invitation_delivery_worker_migration_state()
    OWNER TO ple_invitation_delivery_broker;
REVOKE ALL ON FUNCTION public.ple_cancel_ineligible_invitation_deliveries() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_claim_course_invitation_deliveries(integer, integer) FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_complete_course_invitation_delivery(uuid, uuid, text, timestamptz) FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_prepare_course_invitation_delivery(uuid, uuid) FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_revalidate_course_invitation_delivery_lease(uuid, uuid) FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_invitation_delivery_worker_migration_state() FROM PUBLIC, ple_app;
GRANT SELECT (tenant_id, course_id, invitation_id, state)
    ON public.course_invitation_delivery TO ple_app;
GRANT INSERT (tenant_id, course_id, invitation_id, delivery_id)
    ON public.course_invitation_delivery TO ple_app;
GRANT SELECT, UPDATE ON public.course_invitation_delivery TO ple_invitation_delivery_broker;
GRANT SELECT (tenant_id, course_id, invitation_id, status, expires_at, delivery_email, token_hash, roster_id, idempotency_key, roster_import_id, roster_import_row_number)
    ON public.course_invitation TO ple_invitation_delivery_broker;
GRANT SELECT (tenant_id, course_id, roster_import_id, commit_idempotency_key)
    ON public.course_roster_import TO ple_invitation_delivery_broker;
GRANT SELECT ON public.ple_migration_state TO ple_invitation_delivery_broker;
GRANT EXECUTE ON FUNCTION public.ple_claim_course_invitation_deliveries(integer, integer)
    TO ple_invitation_delivery_worker;
GRANT EXECUTE ON FUNCTION public.ple_complete_course_invitation_delivery(uuid, uuid, text, timestamptz)
    TO ple_invitation_delivery_worker;
GRANT EXECUTE ON FUNCTION public.ple_prepare_course_invitation_delivery(uuid, uuid)
    TO ple_invitation_delivery_worker;
GRANT EXECUTE ON FUNCTION public.ple_revalidate_course_invitation_delivery_lease(uuid, uuid)
    TO ple_invitation_delivery_worker;
GRANT EXECUTE ON FUNCTION public.ple_invitation_delivery_worker_migration_state()
    TO ple_invitation_delivery_worker;
GRANT DELETE ON public.course_invitation_delivery TO ple_retention_broker;
