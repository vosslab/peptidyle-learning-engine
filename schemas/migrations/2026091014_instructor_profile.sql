-- Authenticated Instructor self-profile time-zone preference API.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'migration 2026091014 must run as ple_migrator';
    END IF;
END
$$;

-- These SECURITY DEFINER functions touch the forced-RLS private preference
-- table, so the established private data owner owns them. Grant CREATE only
-- while declaring that tightly bounded definer boundary.
SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_api.read_instructor_profile()
RETURNS text
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT preference.time_zone
      FROM ple_private.account_time_zone AS preference
      JOIN ple_private.account AS account ON account.account_id = preference.account_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'instructor'
       AND ple_api.current_session_account_is_instructor()
$$;

CREATE FUNCTION ple_api.update_instructor_profile(p_time_zone text)
RETURNS text
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
DECLARE v_account_id uuid;
BEGIN
    IF p_time_zone IS NULL OR NOT ple_private.account_time_zone_is_exact_iana(p_time_zone) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Instructor Profile time zone is invalid';
    END IF;
    v_account_id := ple_api.current_session_account_id();
    IF NOT ple_api.current_session_account_is_instructor() OR NOT EXISTS (
        SELECT 1 FROM ple_private.account AS account
         WHERE account.account_id = v_account_id AND account.product_role = 'instructor'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Instructor Profile requires an active Instructor Account';
    END IF;
    UPDATE ple_private.account_time_zone
       SET time_zone = p_time_zone
     WHERE account_id = v_account_id
     RETURNING time_zone INTO p_time_zone;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Instructor Profile requires an Account time zone'; END IF;
    RETURN p_time_zone;
END
$$;

REVOKE ALL ON FUNCTION ple_api.read_instructor_profile() FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.update_instructor_profile(text) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.read_instructor_profile(), ple_api.update_instructor_profile(text) TO ple_app;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
