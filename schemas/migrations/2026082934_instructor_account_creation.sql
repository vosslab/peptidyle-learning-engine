-- SD1 authenticated Create Instructor Account operation. ASVS 8.2.1 and 8.3.1.
-- Bootstrap creation of the first Sysadmin belongs to installation setup.

DO $$
BEGIN
    IF current_user <> 'ple_migrator'
       OR NOT pg_catalog.pg_has_role('ple_migrator', 'ple_private_owner', 'SET')
       OR NOT pg_catalog.pg_has_role('ple_migrator', 'ple_api_owner', 'SET') THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026082934 requires the SD1 Instructor Account API migration principal';
    END IF;
END
$$;

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.account_state_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.account_state_event FORCE ROW LEVEL SECURITY;

CREATE POLICY account_private_owner_create ON ple_private.account
    FOR INSERT TO ple_private_owner
    WITH CHECK (role = 'instructor');

CREATE POLICY account_state_event_private_owner_read
    ON ple_private.account_state_event
    FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY account_state_event_private_owner_create
    ON ple_private.account_state_event
    FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY account_authentication_email_private_owner_create
    ON ple_private.account_authentication_email
    FOR INSERT TO ple_private_owner
    WITH CHECK (
        EXISTS (
            SELECT 1
            FROM ple_private.account AS account
            WHERE account.account_id = account_authentication_email.account_id
              AND account.role = 'instructor'
        )
    );

CREATE FUNCTION ple_private.create_instructor_account(
    p_normalized_email text,
    p_delivery_email text
)
RETURNS TABLE (account_id uuid, created_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
DECLARE
    v_account_id uuid;
    v_caller_role text;
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

    SELECT account.role
      INTO v_caller_role
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
          FROM ple_private.account_state_event AS event
          WHERE event.account_id = account.account_id
          ORDER BY event.occurred_at DESC, event.event_id DESC
          LIMIT 1
      ) AS current_state ON current_state.state = 'active'
     WHERE account.account_id = ple_api.current_session_account_id();
    IF v_caller_role IS DISTINCT FROM 'sysadmin' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Active Sysadmin Account required';
    END IF;

    v_account_id := pg_catalog.gen_random_uuid();
    v_created_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_private.account (account_id, role, created_at)
    VALUES (v_account_id, 'instructor', v_created_at);
    INSERT INTO ple_private.account_authentication_email (
        account_id, normalized_email, delivery_email, verified_at, updated_at
    ) VALUES (
        v_account_id, p_normalized_email, p_delivery_email, v_created_at, v_created_at
    );
    RETURN QUERY SELECT v_account_id, v_created_at;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.create_instructor_account(text, text) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.create_instructor_account(text, text) TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.create_instructor_account(
    p_normalized_email text,
    p_delivery_email text
)
RETURNS TABLE (account_id uuid, created_at timestamp with time zone)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT * FROM ple_private.create_instructor_account(p_normalized_email, p_delivery_email)
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.create_instructor_account(text, text) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.create_instructor_account(text, text) TO ple_app;
COMMENT ON FUNCTION ple_api.create_instructor_account(text, text) IS
    'Active Sysadmin-only Create Instructor Account operation with a private Authentication Email.';

RESET ROLE;
