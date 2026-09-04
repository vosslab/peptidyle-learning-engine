-- Immutable, role-qualified Create Instructor Account evidence.

DO $$
BEGIN
    IF current_user <> 'ple_migrator'
       OR NOT pg_catalog.pg_has_role('ple_migrator', 'ple_private_owner', 'SET')
       OR NOT pg_catalog.pg_has_role('ple_migrator', 'ple_audit_owner', 'SET') THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026090401 requires the account-creation audit migration principal';
    END IF;
END
$$;

-- ASVS 8.2.2 and 16.2.5: this audit relation holds only the subject, the
-- authorized actor, and database-generated event metadata. It never records an
-- Authentication Email, passkey material, browser data, or a session token.
SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_audit_owner;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.instructor_account_creation_event (
    event_id uuid PRIMARY KEY,
    created_instructor_account_id uuid NOT NULL,
    created_instructor_product_role text NOT NULL DEFAULT 'instructor'
        CHECK (created_instructor_product_role = 'instructor'),
    created_by_sysadmin_account_id uuid NOT NULL,
    created_by_sysadmin_product_role text NOT NULL DEFAULT 'sysadmin'
        CHECK (created_by_sysadmin_product_role = 'sysadmin'),
    occurred_at timestamp with time zone NOT NULL,
    CONSTRAINT instructor_account_creation_event_subject_is_unique
        UNIQUE (created_instructor_account_id),
    CONSTRAINT instructor_account_creation_event_subject_product_role_matches
        FOREIGN KEY (created_instructor_account_id, created_instructor_product_role)
        REFERENCES ple_private.account (account_id, product_role),
    CONSTRAINT instructor_account_creation_event_actor_product_role_matches
        FOREIGN KEY (created_by_sysadmin_account_id, created_by_sysadmin_product_role)
        REFERENCES ple_private.account (account_id, product_role)
);

-- ASVS 16.4.2: reject any attempted rewrite even if a future privileged path
-- accidentally receives table-modification access.
CREATE FUNCTION ple_audit.reject_instructor_account_creation_event_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_audit
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514',
        MESSAGE = 'Create Instructor Account audit evidence is immutable';
END
$$;

CREATE TRIGGER instructor_account_creation_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.instructor_account_creation_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_instructor_account_creation_event_change();

ALTER TABLE ple_audit.instructor_account_creation_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.instructor_account_creation_event FORCE ROW LEVEL SECURITY;

-- The writer has no caller-supplied identity or timestamp. Its only inputs are
-- the already-authorized Account subject and actor; it creates who/what/when
-- evidence locally. ASVS 2.2.1, 2.2.2, 8.2.1, 8.3.1, 16.2.1, 16.2.2, and 16.3.3.
CREATE FUNCTION ple_audit.record_instructor_account_creation_event(
    p_created_instructor_account_id uuid,
    p_created_by_sysadmin_account_id uuid
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_audit
AS $$
BEGIN
    IF p_created_instructor_account_id IS NULL
       OR p_created_by_sysadmin_account_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22004',
            MESSAGE = 'Create Instructor Account audit evidence requires an Account subject and actor';
    END IF;

    INSERT INTO ple_audit.instructor_account_creation_event (
        event_id,
        created_instructor_account_id,
        created_by_sysadmin_account_id,
        occurred_at
    ) VALUES (
        pg_catalog.gen_random_uuid(),
        p_created_instructor_account_id,
        p_created_by_sysadmin_account_id,
        pg_catalog.transaction_timestamp()
    );
END
$$;

CREATE POLICY instructor_account_creation_event_audit_owner_insert
    ON ple_audit.instructor_account_creation_event
    FOR INSERT TO ple_audit_owner WITH CHECK (true);

REVOKE ALL PRIVILEGES ON TABLE ple_audit.instructor_account_creation_event
    FROM PUBLIC, ple_private_owner, ple_api_owner, ple_app, ple_auth, ple_student;
REVOKE ALL PRIVILEGES ON FUNCTION ple_audit.reject_instructor_account_creation_event_change()
    FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_audit.record_instructor_account_creation_event(uuid, uuid)
    FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_audit TO ple_private_owner;
GRANT EXECUTE ON FUNCTION ple_audit.record_instructor_account_creation_event(uuid, uuid)
    TO ple_private_owner;

RESET ROLE;

SET LOCAL ROLE ple_private_owner;

-- ASVS 2.3.3 and 8.3.1: validate the one captured current Account at the
-- trusted database boundary, then create the Account, its initial Account
-- State, Authentication Email, and immutable actor evidence atomically.
CREATE OR REPLACE FUNCTION ple_private.create_instructor_account(
    p_normalized_email text,
    p_delivery_email text
)
RETURNS TABLE (account_id uuid, created_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE
    v_account_id uuid;
    v_actor_account_id uuid;
    v_actor_product_role text;
    v_created_at timestamp with time zone;
BEGIN
    IF p_normalized_email IS NULL
       OR char_length(p_normalized_email) NOT BETWEEN 3 AND 320
       OR p_normalized_email IS DISTINCT FROM lower(btrim(p_normalized_email))
       OR p_delivery_email IS NULL
       OR char_length(btrim(p_delivery_email)) NOT BETWEEN 3 AND 320 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'invalid Create Instructor Account input';
    END IF;

    v_actor_account_id := ple_api.current_session_account_id();
    SELECT account.product_role
      INTO v_actor_product_role
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT state_event.state
            FROM ple_private.account_state_event AS state_event
           WHERE state_event.account_id = account.account_id
           ORDER BY state_event.occurred_at DESC, state_event.event_id DESC
           LIMIT 1
      ) AS current_state ON current_state.state = 'active'
     WHERE account.account_id = v_actor_account_id;
    IF v_actor_product_role IS DISTINCT FROM 'sysadmin' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Active Sysadmin Account required';
    END IF;

    v_account_id := pg_catalog.gen_random_uuid();
    v_created_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_private.account (account_id, product_role, created_at)
    VALUES (v_account_id, 'instructor', v_created_at);
    INSERT INTO ple_private.account_authentication_email (
        account_id, normalized_email, delivery_email, verified_at, updated_at
    ) VALUES (
        v_account_id, p_normalized_email, p_delivery_email, v_created_at, v_created_at
    );
    PERFORM ple_audit.record_instructor_account_creation_event(
        v_account_id, v_actor_account_id
    );
    RETURN QUERY SELECT v_account_id, v_created_at;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.create_instructor_account(text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.create_instructor_account(text, text) TO ple_api_owner;

RESET ROLE;
