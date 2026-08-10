-- Forward migration: PLE-owned passwordless accounts and atomic course rosters.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_enrollment_broker') THEN
        CREATE ROLE ple_enrollment_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT BYPASSRLS;
    END IF;
END
$$;

GRANT USAGE ON SCHEMA public TO ple_enrollment_broker;

-- Global account and credential records are deliberately outside tenant RLS.
-- Only the dedicated application auth role can read or mutate them.
CREATE TABLE public.ple_account (
    user_id uuid PRIMARY KEY,
    normalized_email text NOT NULL UNIQUE,
    delivery_email text NOT NULL,
    display_name text NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT ple_account_email_check CHECK (
        octet_length(normalized_email) BETWEEN 3 AND 320
        AND normalized_email = lower(normalized_email)
        AND normalized_email = btrim(normalized_email)
        AND position('@' IN normalized_email) > 1
        AND octet_length(delivery_email) BETWEEN 3 AND 320
        AND delivery_email = btrim(delivery_email)
    ),
    CONSTRAINT ple_account_display_name_check CHECK (
        char_length(display_name) BETWEEN 1 AND 200
        AND display_name = btrim(display_name)
    )
);

CREATE TABLE public.email_authentication_challenge (
    challenge_id uuid PRIMARY KEY,
    token_hash bytea NOT NULL UNIQUE,
    browser_binding_hash bytea NOT NULL,
    normalized_email text NOT NULL,
    delivery_email text NOT NULL,
    purpose text NOT NULL,
    purpose_user_id uuid REFERENCES public.ple_account(user_id) ON DELETE CASCADE,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    CONSTRAINT email_authentication_challenge_subject_key UNIQUE NULLS NOT DISTINCT (
        normalized_email, purpose, purpose_user_id
    ),
    CONSTRAINT email_authentication_challenge_token_check CHECK (octet_length(token_hash) = 32),
    CONSTRAINT email_authentication_challenge_binding_check CHECK (
        octet_length(browser_binding_hash) = 32
    ),
    CONSTRAINT email_authentication_challenge_email_check CHECK (
        octet_length(normalized_email) BETWEEN 3 AND 320
        AND normalized_email = lower(normalized_email)
        AND normalized_email = btrim(normalized_email)
        AND octet_length(delivery_email) BETWEEN 3 AND 320
        AND delivery_email = btrim(delivery_email)
    ),
    CONSTRAINT email_authentication_challenge_purpose_check CHECK (
        (purpose = 'sign_in_or_register' AND purpose_user_id IS NULL)
        OR (purpose = 'change_email' AND purpose_user_id IS NOT NULL)
    ),
    CONSTRAINT email_authentication_challenge_expiry_check CHECK (
        expires_at > created_at AND expires_at <= created_at + interval '10 minutes'
    )
);

CREATE INDEX email_authentication_challenge_expiry_idx
    ON public.email_authentication_challenge (expires_at);
CREATE INDEX email_authentication_challenge_user_idx
    ON public.email_authentication_challenge (purpose_user_id)
    WHERE purpose_user_id IS NOT NULL;

CREATE TABLE public.authentication_rate_limit (
    limit_scope text NOT NULL,
    key_hash bytea NOT NULL,
    window_started_at timestamp with time zone NOT NULL,
    attempt_count integer NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    PRIMARY KEY (limit_scope, key_hash),
    CONSTRAINT authentication_rate_limit_scope_check CHECK (
        limit_scope IN ('email', 'network')
    ),
    CONSTRAINT authentication_rate_limit_key_check CHECK (octet_length(key_hash) = 32),
    CONSTRAINT authentication_rate_limit_count_check CHECK (
        attempt_count BETWEEN 1 AND 10001
    ),
    CONSTRAINT authentication_rate_limit_time_check CHECK (
        updated_at >= window_started_at
    )
);

CREATE INDEX authentication_rate_limit_updated_idx
    ON public.authentication_rate_limit (updated_at);

CREATE TABLE public.account_authentication_session (
    token_hash bytea PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES public.ple_account(user_id) ON DELETE CASCADE,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    CONSTRAINT account_authentication_session_token_check CHECK (octet_length(token_hash) = 32),
    CONSTRAINT account_authentication_session_expiry_check CHECK (
        expires_at > created_at AND expires_at <= created_at + interval '15 minutes'
    )
);

CREATE INDEX account_authentication_session_expiry_idx
    ON public.account_authentication_session (expires_at);
CREATE INDEX account_authentication_session_user_idx
    ON public.account_authentication_session (user_id);

CREATE TABLE public.webauthn_ceremony (
    ceremony_id uuid PRIMARY KEY,
    ceremony_kind text NOT NULL,
    user_id uuid REFERENCES public.ple_account(user_id) ON DELETE CASCADE,
    browser_binding_hash bytea NOT NULL,
    state jsonb NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT webauthn_ceremony_kind_check CHECK (
        (ceremony_kind = 'registration' AND user_id IS NOT NULL)
        OR ceremony_kind = 'authentication'
    ),
    CONSTRAINT webauthn_ceremony_binding_check CHECK (
        octet_length(browser_binding_hash) = 32
    ),
    CONSTRAINT webauthn_ceremony_state_check CHECK (
        jsonb_typeof(state) = 'object' AND octet_length(state::text) <= 65536
    ),
    CONSTRAINT webauthn_ceremony_expiry_check CHECK (
        expires_at > created_at AND expires_at <= created_at + interval '10 minutes'
    )
);

CREATE INDEX webauthn_ceremony_expiry_idx ON public.webauthn_ceremony (expires_at);
CREATE INDEX webauthn_ceremony_user_idx
    ON public.webauthn_ceremony (user_id) WHERE user_id IS NOT NULL;

CREATE TABLE public.account_passkey (
    passkey_id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES public.ple_account(user_id) ON DELETE CASCADE,
    credential_id_hash bytea NOT NULL UNIQUE,
    label text NOT NULL,
    credential jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    last_used_at timestamp with time zone,
    revoked_at timestamp with time zone,
    CONSTRAINT account_passkey_credential_hash_check CHECK (
        octet_length(credential_id_hash) = 32
    ),
    CONSTRAINT account_passkey_label_check CHECK (
        char_length(label) BETWEEN 1 AND 80 AND label = btrim(label)
    ),
    CONSTRAINT account_passkey_credential_check CHECK (
        jsonb_typeof(credential) = 'object' AND octet_length(credential::text) <= 65536
    ),
    CONSTRAINT account_passkey_timestamps_check CHECK (
        (last_used_at IS NULL OR last_used_at >= created_at)
        AND (revoked_at IS NULL OR revoked_at >= created_at)
    )
);

CREATE INDEX account_passkey_user_active_idx
    ON public.account_passkey (user_id, passkey_id) WHERE revoked_at IS NULL;
CREATE INDEX account_passkey_user_idx ON public.account_passkey (user_id);

ALTER TABLE public.ple_account ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.ple_account FORCE ROW LEVEL SECURITY;
ALTER TABLE public.email_authentication_challenge ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.email_authentication_challenge FORCE ROW LEVEL SECURITY;
ALTER TABLE public.authentication_rate_limit ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.authentication_rate_limit FORCE ROW LEVEL SECURITY;
ALTER TABLE public.account_authentication_session ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.account_authentication_session FORCE ROW LEVEL SECURITY;
ALTER TABLE public.webauthn_ceremony ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.webauthn_ceremony FORCE ROW LEVEL SECURITY;
ALTER TABLE public.account_passkey ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.account_passkey FORCE ROW LEVEL SECURITY;

CREATE POLICY ple_account_auth ON public.ple_account TO ple_auth
    USING (true) WITH CHECK (true);
CREATE POLICY email_authentication_challenge_auth ON public.email_authentication_challenge
    TO ple_auth USING (true) WITH CHECK (true);
CREATE POLICY authentication_rate_limit_auth ON public.authentication_rate_limit
    TO ple_auth USING (true) WITH CHECK (true);
CREATE POLICY account_authentication_session_auth ON public.account_authentication_session
    TO ple_auth USING (true) WITH CHECK (true);
CREATE POLICY webauthn_ceremony_auth ON public.webauthn_ceremony TO ple_auth
    USING (true) WITH CHECK (true);
CREATE POLICY account_passkey_auth ON public.account_passkey TO ple_auth
    USING (true) WITH CHECK (true);

GRANT SELECT, INSERT, UPDATE ON public.ple_account TO ple_auth;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.email_authentication_challenge TO ple_auth;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.authentication_rate_limit TO ple_auth;
GRANT SELECT, INSERT, DELETE ON public.account_authentication_session TO ple_auth;
GRANT SELECT, INSERT, DELETE ON public.webauthn_ceremony TO ple_auth;
GRANT SELECT, INSERT, UPDATE ON public.account_passkey TO ple_auth;

-- Roster commands reuse the accepted session/course authorization primitive
-- through a roster-named wrapper. This keeps the opaque session hash, tenant
-- administrator role, and direct course role inside the database decision.
CREATE FUNCTION public.ple_course_roster_actor(
    p_session character,
    p_course uuid,
    p_manager_only boolean DEFAULT true
) RETURNS uuid
    LANGUAGE sql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT public.ple_course_appearance_actor(p_session, p_course, p_manager_only)
$$;

REVOKE ALL ON FUNCTION public.ple_course_roster_actor(character, uuid, boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_course_roster_actor(character, uuid, boolean) TO ple_app;

-- The learner identity is global to one PLE account, while StudentId remains
-- a tenant-scoped pedagogical identifier used by educational records.
CREATE TABLE public.tenant_learner_identity (
    tenant_id uuid NOT NULL,
    user_id uuid NOT NULL,
    student_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    PRIMARY KEY (tenant_id, user_id),
    UNIQUE (tenant_id, student_id),
    UNIQUE (tenant_id, user_id, student_id)
);

CREATE TABLE public.course_roster_state (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    signup_posture text DEFAULT 'invitation_only' NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    PRIMARY KEY (tenant_id, course_id),
    FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course(tenant_id, course_id) ON DELETE CASCADE,
    CONSTRAINT course_roster_state_revision_check CHECK (revision > 0),
    CONSTRAINT course_roster_state_posture_check CHECK (
        signup_posture IN ('invitation_only', 'permitted_domains')
    )
);

CREATE TABLE public.course_allowed_email_domain (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    normalized_domain text NOT NULL,
    include_subdomains boolean DEFAULT false NOT NULL,
    PRIMARY KEY (tenant_id, course_id, normalized_domain),
    FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course_roster_state(tenant_id, course_id) ON DELETE CASCADE,
    CONSTRAINT course_allowed_email_domain_check CHECK (
        octet_length(normalized_domain) BETWEEN 3 AND 253
        AND normalized_domain = lower(normalized_domain)
        AND normalized_domain = btrim(normalized_domain)
        AND position('.' IN normalized_domain) > 1
    )
);

CREATE TABLE public.course_roster_member (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    course_member_id uuid NOT NULL,
    user_id uuid NOT NULL,
    student_id uuid NOT NULL,
    display_name text NOT NULL,
    roster_email_normalized text,
    roster_email_delivery text,
    roster_id text,
    source text NOT NULL,
    status text NOT NULL,
    joined_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    PRIMARY KEY (tenant_id, course_id, course_member_id),
    UNIQUE (tenant_id, course_id, user_id),
    FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course(tenant_id, course_id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, user_id, student_id)
        REFERENCES public.tenant_learner_identity(tenant_id, user_id, student_id),
    CONSTRAINT course_roster_member_email_check CHECK (
        (roster_email_normalized IS NULL AND roster_email_delivery IS NULL)
        OR (
            octet_length(roster_email_normalized) BETWEEN 3 AND 320
            AND roster_email_normalized = lower(roster_email_normalized)
            AND roster_email_normalized = btrim(roster_email_normalized)
            AND octet_length(roster_email_delivery) BETWEEN 3 AND 320
            AND roster_email_delivery = btrim(roster_email_delivery)
        )
    ),
    CONSTRAINT course_roster_member_roster_id_check CHECK (
        roster_id IS NULL OR roster_id ~ '^[A-Za-z0-9._-]{1,64}$'
    ),
    CONSTRAINT course_roster_member_display_name_check CHECK (
        char_length(display_name) BETWEEN 1 AND 200 AND display_name = btrim(display_name)
    ),
    CONSTRAINT course_roster_member_source_check CHECK (source IN ('invitation', 'legacy')),
    CONSTRAINT course_roster_member_status_check CHECK (status IN ('active', 'revoked')),
    CONSTRAINT course_roster_member_revocation_check CHECK (
        (status = 'active' AND revoked_at IS NULL)
        OR (status = 'revoked' AND revoked_at IS NOT NULL AND revoked_at >= joined_at)
    ),
    CONSTRAINT course_roster_member_managed_fields_check CHECK (
        source = 'legacy'
        OR (roster_email_normalized IS NOT NULL AND roster_id IS NOT NULL)
    )
);

CREATE UNIQUE INDEX course_roster_member_roster_id_key
    ON public.course_roster_member (tenant_id, course_id, roster_id)
    WHERE roster_id IS NOT NULL;
CREATE INDEX course_roster_member_learner_fk_idx
    ON public.course_roster_member (tenant_id, user_id, student_id);

CREATE TABLE public.course_invitation (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    invitation_id uuid NOT NULL,
    token_hash bytea NOT NULL UNIQUE,
    normalized_email text NOT NULL,
    delivery_email text NOT NULL,
    roster_id text NOT NULL,
    invited_by uuid NOT NULL,
    idempotency_key text NOT NULL,
    status text DEFAULT 'pending' NOT NULL,
    claimed_user_id uuid,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    claimed_at timestamp with time zone,
    PRIMARY KEY (tenant_id, course_id, invitation_id),
    UNIQUE (tenant_id, course_id, idempotency_key),
    FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course_roster_state(tenant_id, course_id) ON DELETE CASCADE,
    CONSTRAINT course_invitation_token_check CHECK (octet_length(token_hash) = 32),
    CONSTRAINT course_invitation_email_check CHECK (
        octet_length(normalized_email) BETWEEN 3 AND 320
        AND normalized_email = lower(normalized_email)
        AND normalized_email = btrim(normalized_email)
        AND octet_length(delivery_email) BETWEEN 3 AND 320
        AND delivery_email = btrim(delivery_email)
    ),
    CONSTRAINT course_invitation_roster_id_check CHECK (
        roster_id ~ '^[A-Za-z0-9._-]{1,64}$'
    ),
    CONSTRAINT course_invitation_idempotency_check CHECK (
        octet_length(idempotency_key) BETWEEN 1 AND 128
    ),
    CONSTRAINT course_invitation_status_check CHECK (
        status IN ('pending', 'claimed', 'expired', 'revoked')
    ),
    CONSTRAINT course_invitation_expiry_check CHECK (
        expires_at > created_at AND expires_at <= created_at + interval '30 days'
    ),
    CONSTRAINT course_invitation_claim_check CHECK (
        (status = 'claimed' AND claimed_user_id IS NOT NULL AND claimed_at IS NOT NULL)
        OR (status <> 'claimed' AND claimed_user_id IS NULL AND claimed_at IS NULL)
    )
);

CREATE UNIQUE INDEX course_invitation_pending_roster_id_key
    ON public.course_invitation (tenant_id, course_id, roster_id)
    WHERE status = 'pending';
CREATE UNIQUE INDEX course_invitation_pending_email_key
    ON public.course_invitation (tenant_id, course_id, normalized_email)
    WHERE status = 'pending';
CREATE INDEX course_invitation_expiry_idx
    ON public.course_invitation (expires_at) WHERE status = 'pending';

CREATE TABLE public.course_roster_import (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    roster_import_id uuid NOT NULL,
    normalized_digest bytea NOT NULL,
    stage_idempotency_key text NOT NULL,
    commit_idempotency_key text,
    roster_revision bigint NOT NULL,
    committed_roster_revision bigint,
    revision bigint DEFAULT 1 NOT NULL,
    status text DEFAULT 'preview' NOT NULL,
    created_by uuid NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    committed_at timestamp with time zone,
    PRIMARY KEY (tenant_id, course_id, roster_import_id),
    UNIQUE (tenant_id, course_id, stage_idempotency_key),
    FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course_roster_state(tenant_id, course_id) ON DELETE CASCADE,
    CONSTRAINT course_roster_import_digest_check CHECK (octet_length(normalized_digest) = 32),
    CONSTRAINT course_roster_import_idempotency_check CHECK (
        octet_length(stage_idempotency_key) BETWEEN 1 AND 128
        AND (commit_idempotency_key IS NULL
             OR octet_length(commit_idempotency_key) BETWEEN 1 AND 128)
    ),
    CONSTRAINT course_roster_import_revision_check CHECK (
        roster_revision > 0 AND revision > 0
        AND (committed_roster_revision IS NULL OR committed_roster_revision > roster_revision)
    ),
    CONSTRAINT course_roster_import_status_check CHECK (status IN ('preview', 'committed')),
    CONSTRAINT course_roster_import_expiry_check CHECK (
        expires_at > created_at AND expires_at <= created_at + interval '24 hours'
    ),
    CONSTRAINT course_roster_import_commit_check CHECK (
        (status = 'preview' AND revision = 1
         AND commit_idempotency_key IS NULL AND committed_roster_revision IS NULL
         AND committed_at IS NULL)
        OR (status = 'committed' AND revision = 2
            AND commit_idempotency_key IS NOT NULL
            AND committed_roster_revision IS NOT NULL AND committed_at IS NOT NULL)
    )
);

CREATE INDEX course_roster_import_expiry_idx
    ON public.course_roster_import (expires_at) WHERE status = 'preview';

CREATE TABLE public.course_roster_import_row (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    roster_import_id uuid NOT NULL,
    row_number integer NOT NULL,
    normalized_email text,
    delivery_email text,
    roster_id text,
    row_status text NOT NULL,
    PRIMARY KEY (tenant_id, course_id, roster_import_id, row_number),
    FOREIGN KEY (tenant_id, course_id, roster_import_id)
        REFERENCES public.course_roster_import(tenant_id, course_id, roster_import_id)
        ON DELETE CASCADE,
    CONSTRAINT course_roster_import_row_number_check CHECK (row_number BETWEEN 2 AND 501),
    CONSTRAINT course_roster_import_row_email_check CHECK (
        (normalized_email IS NULL AND delivery_email IS NULL AND roster_id IS NULL)
        OR (
            octet_length(normalized_email) BETWEEN 3 AND 320
            AND normalized_email = lower(normalized_email)
            AND normalized_email = btrim(normalized_email)
            AND octet_length(delivery_email) BETWEEN 3 AND 320
            AND delivery_email = btrim(delivery_email)
            AND roster_id ~ '^[A-Za-z0-9._-]{1,64}$'
        )
    ),
    CONSTRAINT course_roster_import_row_status_check CHECK (
        row_status IN ('ready_to_invite', 'already_member', 'already_pending', 'duplicate', 'invalid')
    ),
    CONSTRAINT course_roster_import_row_invalid_shape_check CHECK (
        row_status <> 'invalid' OR normalized_email IS NULL OR roster_id IS NOT NULL
    )
);

ALTER TABLE public.course_invitation
    ADD COLUMN roster_import_id uuid,
    ADD COLUMN roster_import_row_number integer,
    ADD CONSTRAINT course_invitation_import_shape_check CHECK (
        (roster_import_id IS NULL AND roster_import_row_number IS NULL)
        OR (roster_import_id IS NOT NULL AND roster_import_row_number IS NOT NULL)
    ),
    ADD CONSTRAINT course_invitation_import_row_fk FOREIGN KEY (
        tenant_id, course_id, roster_import_id, roster_import_row_number
    ) REFERENCES public.course_roster_import_row(
        tenant_id, course_id, roster_import_id, row_number
    ) ON DELETE CASCADE;

CREATE UNIQUE INDEX course_invitation_import_row_key
    ON public.course_invitation (
        tenant_id, course_id, roster_import_id, roster_import_row_number
    ) WHERE roster_import_id IS NOT NULL;

CREATE TABLE public.course_grade_export_audit (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    export_id uuid NOT NULL,
    requested_by uuid NOT NULL,
    row_count integer NOT NULL,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    PRIMARY KEY (tenant_id, export_id),
    FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment(tenant_id, course_id, assignment_id) ON DELETE CASCADE,
    CONSTRAINT course_grade_export_audit_row_count_check CHECK (row_count BETWEEN 0 AND 500)
);

INSERT INTO public.course_roster_state (tenant_id, course_id)
SELECT tenant_id, course_id FROM public.course;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM public.enrollment
         GROUP BY tenant_id, user_id
        HAVING COUNT(DISTINCT student_id) > 1
    ) THEN
        RAISE EXCEPTION 'existing learner has multiple StudentId values inside one tenant';
    END IF;
END
$$;

INSERT INTO public.tenant_learner_identity (tenant_id, user_id, student_id)
SELECT cm.tenant_id,
       cm.user_id,
       COALESCE((array_agg(DISTINCT e.student_id ORDER BY e.student_id))[1], cm.user_id)
  FROM public.course_member cm
  LEFT JOIN public.enrollment e
    ON e.tenant_id = cm.tenant_id AND e.user_id = cm.user_id
 WHERE cm.role = 'student'
 GROUP BY cm.tenant_id, cm.user_id;

INSERT INTO public.course_roster_member (
    tenant_id, course_id, course_member_id, user_id, student_id,
    display_name, roster_email_normalized, roster_email_delivery, roster_id,
    source, status, joined_at
)
SELECT cm.tenant_id,
       cm.course_id,
       gen_random_uuid(),
       cm.user_id,
       learner.student_id,
       'Legacy learner',
       NULL,
       NULL,
       NULL,
       'legacy',
       'active',
       transaction_timestamp()
  FROM public.course_member cm
  JOIN public.tenant_learner_identity learner
    ON learner.tenant_id = cm.tenant_id AND learner.user_id = cm.user_id
 WHERE cm.role = 'student';

ALTER TABLE public.tenant_learner_identity ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.tenant_learner_identity FORCE ROW LEVEL SECURITY;
ALTER TABLE public.course_roster_state ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.course_roster_state FORCE ROW LEVEL SECURITY;
ALTER TABLE public.course_allowed_email_domain ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.course_allowed_email_domain FORCE ROW LEVEL SECURITY;
ALTER TABLE public.course_roster_member ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.course_roster_member FORCE ROW LEVEL SECURITY;
ALTER TABLE public.course_invitation ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.course_invitation FORCE ROW LEVEL SECURITY;
ALTER TABLE public.course_roster_import ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.course_roster_import FORCE ROW LEVEL SECURITY;
ALTER TABLE public.course_roster_import_row ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.course_roster_import_row FORCE ROW LEVEL SECURITY;
ALTER TABLE public.course_grade_export_audit ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.course_grade_export_audit FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_learner_identity_app ON public.tenant_learner_identity TO ple_app
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY course_roster_state_app ON public.course_roster_state TO ple_app
    USING (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    )
    WITH CHECK (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    );
CREATE POLICY course_allowed_email_domain_app ON public.course_allowed_email_domain TO ple_app
    USING (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    )
    WITH CHECK (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    );
CREATE POLICY course_roster_member_app ON public.course_roster_member TO ple_app
    USING (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    )
    WITH CHECK (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    );
CREATE POLICY course_invitation_app ON public.course_invitation TO ple_app
    USING (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    )
    WITH CHECK (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    );
CREATE POLICY course_roster_import_app ON public.course_roster_import TO ple_app
    USING (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    )
    WITH CHECK (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    );
CREATE POLICY course_roster_import_row_app ON public.course_roster_import_row TO ple_app
    USING (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    )
    WITH CHECK (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    );
CREATE POLICY course_grade_export_audit_app ON public.course_grade_export_audit TO ple_app
    USING (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    )
    WITH CHECK (
        tenant_id = public.ple_current_tenant()
        AND public.ple_course_records_accessible(tenant_id, course_id)
    );

CREATE POLICY tenant_learner_identity_retention ON public.tenant_learner_identity
    TO ple_retention_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_roster_member_retention ON public.course_roster_member
    TO ple_retention_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_invitation_retention ON public.course_invitation
    TO ple_retention_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_roster_import_retention ON public.course_roster_import
    TO ple_retention_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_roster_import_row_retention ON public.course_roster_import_row
    TO ple_retention_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_grade_export_audit_retention ON public.course_grade_export_audit
    TO ple_retention_broker USING (tenant_id = public.ple_current_tenant());

GRANT SELECT, INSERT, UPDATE ON public.tenant_learner_identity TO ple_app;
GRANT SELECT, INSERT, UPDATE ON public.course_roster_state TO ple_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.course_allowed_email_domain TO ple_app;
GRANT SELECT, INSERT, UPDATE ON public.course_roster_member TO ple_app;
GRANT SELECT, INSERT, UPDATE ON public.course_invitation TO ple_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.course_roster_import TO ple_app;
GRANT SELECT, INSERT, DELETE ON public.course_roster_import_row TO ple_app;
GRANT SELECT, INSERT ON public.course_grade_export_audit TO ple_app;
GRANT SELECT, DELETE ON public.tenant_learner_identity TO ple_retention_broker;
GRANT SELECT, DELETE ON public.course_roster_member TO ple_retention_broker;
GRANT SELECT, DELETE ON public.course_invitation TO ple_retention_broker;
GRANT SELECT, DELETE ON public.course_roster_import, public.course_roster_import_row
    TO ple_retention_broker;
GRANT SELECT, DELETE ON public.course_grade_export_audit TO ple_retention_broker;

-- The capability lookup resolves tenant/course exclusively from a hashed,
-- single-use invitation. It locks both invitation and course, then installs
-- the transaction-local tenant context for the remaining atomic Rust write.
CREATE FUNCTION public.ple_claim_course_invitation_context(p_token_hash bytea)
RETURNS TABLE (
    tenant_id uuid,
    course_id uuid,
    invitation_id uuid,
    normalized_email text,
    delivery_email text,
    roster_id text,
    status text,
    claimed_user_id uuid,
    expires_at timestamp with time zone
)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    located record;
BEGIN
    IF p_token_hash IS NULL OR octet_length(p_token_hash) <> 32 THEN
        RETURN;
    END IF;

    SELECT invitation.tenant_id,
           invitation.course_id,
           invitation.invitation_id,
           invitation.normalized_email,
           invitation.delivery_email,
           invitation.roster_id,
           invitation.status,
           invitation.claimed_user_id,
           invitation.expires_at
      INTO located
      FROM public.course_invitation invitation
     WHERE invitation.token_hash = p_token_hash;
    IF NOT FOUND THEN
        RETURN;
    END IF;

    PERFORM set_config('ple.tenant_id', located.tenant_id::text, true);
    PERFORM 1
      FROM public.course
     WHERE course.tenant_id = located.tenant_id
       AND course.course_id = located.course_id
     FOR UPDATE;
    IF NOT FOUND
       OR NOT public.ple_course_records_accessible(located.tenant_id, located.course_id)
    THEN
        PERFORM set_config('ple.tenant_id', '', true);
        RETURN;
    END IF;

    PERFORM 1
      FROM public.course_roster_state
     WHERE course_roster_state.tenant_id = located.tenant_id
       AND course_roster_state.course_id = located.course_id
     FOR UPDATE;
    IF NOT FOUND THEN
        PERFORM set_config('ple.tenant_id', '', true);
        RETURN;
    END IF;

    -- Every roster writer locks course, roster state, then invitation/member.
    -- Re-read under the invitation lock so the returned state cannot predate
    -- a concurrent revocation or claim.
    SELECT invitation.tenant_id,
           invitation.course_id,
           invitation.invitation_id,
           invitation.normalized_email,
           invitation.delivery_email,
           invitation.roster_id,
           invitation.status,
           invitation.claimed_user_id,
           invitation.expires_at
      INTO located
      FROM public.course_invitation invitation
     WHERE invitation.token_hash = p_token_hash
       AND invitation.tenant_id = located.tenant_id
       AND invitation.course_id = located.course_id
     FOR UPDATE;
    IF NOT FOUND THEN
        PERFORM set_config('ple.tenant_id', '', true);
        RETURN;
    END IF;

    RETURN QUERY SELECT located.tenant_id,
                        located.course_id,
                        located.invitation_id,
                        located.normalized_email,
                        located.delivery_email,
                        located.roster_id,
                        located.status,
                        located.claimed_user_id,
                        located.expires_at;
END
$$;

ALTER FUNCTION public.ple_claim_course_invitation_context(bytea)
    OWNER TO ple_enrollment_broker;
REVOKE ALL ON FUNCTION public.ple_claim_course_invitation_context(bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_claim_course_invitation_context(bytea) TO ple_app;
GRANT SELECT ON public.course_invitation, public.course, public.course_roster_state
    TO ple_enrollment_broker;
-- PostgreSQL requires UPDATE privilege for SELECT ... FOR UPDATE. Grant only
-- one identity column that the function never modifies on each row-lock
-- target to this NOLOGIN owner rather than broad table mutation privileges.
GRANT UPDATE (course_id) ON public.course TO ple_enrollment_broker;
GRANT UPDATE (revision) ON public.course_roster_state TO ple_enrollment_broker;
GRANT UPDATE (invitation_id) ON public.course_invitation TO ple_enrollment_broker;
GRANT EXECUTE ON FUNCTION public.ple_course_records_accessible(uuid, uuid)
    TO ple_enrollment_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant() TO ple_enrollment_broker;

-- A global PLE account may select only a course relationship proven by the
-- enrollment broker. ple_auth receives no direct course or roster grants.
-- Student contexts disappear when the course learner-record boundary closes.
CREATE FUNCTION public.ple_account_course_context_page(
    p_user uuid,
    p_after_tenant uuid,
    p_after_course uuid,
    p_limit integer
) RETURNS TABLE (
    tenant_id uuid,
    course_id uuid,
    title text,
    role text
)
    LANGUAGE sql
    STABLE
    SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT member.tenant_id,
           member.course_id,
           course.title,
           member.role
      FROM public.course_member AS member
      JOIN public.course AS course
        ON course.tenant_id = member.tenant_id
       AND course.course_id = member.course_id
     WHERE member.user_id = p_user
       AND (
            member.role <> 'student'
            OR public.ple_course_records_accessible(member.tenant_id, member.course_id)
       )
       AND (
            p_after_tenant IS NULL
            OR (member.tenant_id, member.course_id) > (p_after_tenant, p_after_course)
       )
     ORDER BY member.tenant_id, member.course_id
     LIMIT least(greatest(p_limit, 1), 101)
$$;

CREATE FUNCTION public.ple_account_course_context(
    p_user uuid,
    p_course uuid
) RETURNS TABLE (
    tenant_id uuid,
    course_id uuid,
    title text,
    role text
)
    LANGUAGE sql
    STABLE
    SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT member.tenant_id,
           member.course_id,
           course.title,
           member.role
      FROM public.course_member AS member
      JOIN public.course AS course
        ON course.tenant_id = member.tenant_id
       AND course.course_id = member.course_id
     WHERE member.user_id = p_user
       AND member.course_id = p_course
       AND (
            member.role <> 'student'
            OR public.ple_course_records_accessible(member.tenant_id, member.course_id)
       )
     ORDER BY member.tenant_id
     LIMIT 2
$$;

ALTER FUNCTION public.ple_account_course_context_page(uuid, uuid, uuid, integer)
    OWNER TO ple_enrollment_broker;
ALTER FUNCTION public.ple_account_course_context(uuid, uuid)
    OWNER TO ple_enrollment_broker;
REVOKE ALL ON FUNCTION public.ple_account_course_context_page(uuid, uuid, uuid, integer)
    FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_account_course_context(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_account_course_context_page(uuid, uuid, uuid, integer)
    TO ple_auth;
GRANT EXECUTE ON FUNCTION public.ple_account_course_context(uuid, uuid) TO ple_auth;
GRANT SELECT ON public.course_member TO ple_enrollment_broker;

-- Extend the accepted retention deletion transaction without rewriting the
-- frozen 2026080806 migration. The previous function remains the authority
-- for the established learner graph; this wrapper removes the new roster PII
-- before reporting the same delete stage committed.
ALTER FUNCTION public.ple_commit_delete_retention_work(
    uuid, uuid, uuid, uuid, text, bigint
) RENAME TO ple_commit_delete_retention_work_before_passwordless_identity;

CREATE FUNCTION public.ple_commit_delete_retention_work(
    p_tenant uuid,
    p_job uuid,
    p_token uuid,
    p_course uuid,
    p_stage text,
    p_generation bigint
) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    committed boolean;
BEGIN
    committed := public.ple_commit_delete_retention_work_before_passwordless_identity(
        p_tenant, p_job, p_token, p_course, p_stage, p_generation
    );
    IF NOT committed THEN
        RETURN false;
    END IF;

    PERFORM set_config('ple.tenant_id', p_tenant::text, true);
    DELETE FROM public.course_grade_export_audit
     WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_roster_import
     WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_invitation
     WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_roster_member
     WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.tenant_learner_identity learner
     WHERE learner.tenant_id = p_tenant
       AND NOT EXISTS (
           SELECT 1 FROM public.course_roster_member member
            WHERE member.tenant_id = learner.tenant_id
              AND member.user_id = learner.user_id
       )
       AND NOT EXISTS (
           SELECT 1 FROM public.course_member member
            WHERE member.tenant_id = learner.tenant_id
              AND member.user_id = learner.user_id
              AND member.role = 'student'
       )
       AND NOT EXISTS (
           SELECT 1 FROM public.enrollment enrollment
            WHERE enrollment.tenant_id = learner.tenant_id
              AND enrollment.user_id = learner.user_id
       );

    RETURN NOT EXISTS (
        SELECT 1 FROM public.course_invitation
         WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL
        SELECT 1 FROM public.course_roster_member
         WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL
        SELECT 1 FROM public.course_grade_export_audit
         WHERE tenant_id = p_tenant AND course_id = p_course
    );
END
$$;

ALTER FUNCTION public.ple_commit_delete_retention_work_before_passwordless_identity(
    uuid, uuid, uuid, uuid, text, bigint
) OWNER TO ple_retention_broker;
ALTER FUNCTION public.ple_commit_delete_retention_work(
    uuid, uuid, uuid, uuid, text, bigint
) OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_commit_delete_retention_work_before_passwordless_identity(
    uuid, uuid, uuid, uuid, text, bigint
) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_commit_delete_retention_work(
    uuid, uuid, uuid, uuid, text, bigint
) FROM PUBLIC;

-- Course-owned roster writes use the same course lock/retention fence as the
-- rest of the learner record graph.
CREATE FUNCTION public.ple_fence_course_roster_write() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF NEW.tenant_id IS NULL
       OR NEW.course_id IS NULL
       OR NOT public.ple_lock_course_write(NEW.tenant_id, NEW.course_id, true)
    THEN
        RAISE EXCEPTION 'course roster is unavailable' USING ERRCODE = '23503';
    END IF;
    IF TG_OP = 'UPDATE'
       AND (OLD.tenant_id, OLD.course_id) IS DISTINCT FROM (NEW.tenant_id, NEW.course_id)
    THEN
        RAISE EXCEPTION 'course roster ownership is immutable' USING ERRCODE = '22023';
    END IF;
    RETURN NEW;
END
$$;

ALTER FUNCTION public.ple_fence_course_roster_write() OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_fence_course_roster_write() FROM PUBLIC;

CREATE TRIGGER course_roster_member_retention_fence
    BEFORE INSERT OR UPDATE ON public.course_roster_member
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_course_roster_write();
CREATE TRIGGER course_invitation_retention_fence
    BEFORE INSERT OR UPDATE ON public.course_invitation
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_course_roster_write();

CREATE TRIGGER course_roster_import_retention_fence
    BEFORE INSERT OR UPDATE ON public.course_roster_import
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_course_roster_write();

CREATE TRIGGER course_roster_import_row_retention_fence
    BEFORE INSERT ON public.course_roster_import_row
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_course_roster_write();

CREATE TRIGGER course_grade_export_audit_retention_fence
    BEFORE INSERT ON public.course_grade_export_audit
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_course_roster_write();
