-- SD1 authenticated Sysadmin Account Creation broker. ASVS 8.2.1 and 8.3.1.
-- Bootstrap creation of the first Sysadmin belongs to installation setup.

DO $$
BEGIN
    IF current_user <> 'ple_migrator'
       OR NOT pg_catalog.pg_has_role('ple_migrator', 'ple_api_owner', 'SET') THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026082934 requires the SD1 API migration principal';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.current_session_account_is_sysadmin()
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ple_private.account AS account
        WHERE account.account_id = ple_api.current_session_account_id()
          AND account.role = 'sysadmin'
    )
$$;

CREATE FUNCTION ple_api.create_account(p_account_id uuid, p_product_role text)
RETURNS TABLE (account_id uuid, product_role text, created_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_sysadmin() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Sysadmin Account required';
    END IF;
    IF p_account_id IS NULL OR p_product_role NOT IN ('student', 'instructor', 'sysadmin') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'invalid Account Creation input';
    END IF;
    RETURN QUERY
    INSERT INTO ple_private.account (account_id, role, created_at)
    VALUES (p_account_id, p_product_role, pg_catalog.transaction_timestamp())
    RETURNING account.account_id, account.role, account.created_at;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_is_sysadmin() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.create_account(uuid, text) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.current_session_account_is_sysadmin() TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.create_account(uuid, text) TO ple_app;
COMMENT ON FUNCTION ple_api.create_account(uuid, text) IS
    'Sysadmin-only Account Creation for one global Account and immutable Product Role.';

RESET ROLE;
