-- SD1 authenticated Sysadmin account-provisioning broker.
--
-- Bootstrap creation of the first Sysadmin belongs to installation setup. Once
-- an authenticated Sysadmin exists, this is the sole application-facing path
-- that provisions another global Account. Authentication itself never does.

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
RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ple_private.account AS account
         WHERE account.account_id = ple_api.current_session_account_id()
           AND account.role = 'sysadmin'
    )
$$;

CREATE FUNCTION ple_api.provision_account(p_account_id uuid, p_role text)
RETURNS TABLE (account_id uuid, role text, provisioned_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_sysadmin() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'sysadmin account required';
    END IF;
    IF p_account_id IS NULL OR p_role NOT IN ('student', 'instructor', 'sysadmin') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'invalid account provisioning input';
    END IF;

    RETURN QUERY
    INSERT INTO ple_private.account (account_id, role, provisioned_at)
    VALUES (p_account_id, p_role, pg_catalog.transaction_timestamp())
    RETURNING account.account_id, account.role, account.provisioned_at;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_is_sysadmin() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.provision_account(uuid, text) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.current_session_account_is_sysadmin() TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.provision_account(uuid, text) TO ple_app;

COMMENT ON FUNCTION ple_api.provision_account(uuid, text) IS
    'Sysadmin-only provisioner for one global Account and immutable Product Role; authentication cannot create Accounts.';

RESET ROLE;
