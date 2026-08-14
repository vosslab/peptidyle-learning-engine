-- Live oracle for account-presentation authorization.  This intentionally
-- creates its own accounts and opaque session hashes: no pytest fixture or
-- shared seeded identity is authoritative here.

BEGIN;

SET LOCAL ROLE ple_auth;

INSERT INTO public.ple_account (
    user_id, normalized_email, delivery_email, display_name
) VALUES
    ('00000000-0000-4000-8000-000000000936', 'presentation-a@example.edu',
     'presentation-a@example.edu', 'Presentation A'),
    ('00000000-0000-4000-8000-000000000937', 'presentation-b@example.edu',
     'presentation-b@example.edu', 'Presentation B');

INSERT INTO public.account_authentication_session (
    token_hash, user_id, created_at, expires_at
) VALUES
    (decode(repeat('a1', 32), 'hex'), '00000000-0000-4000-8000-000000000936',
     transaction_timestamp(), transaction_timestamp() + interval '10 minutes'),
    (decode(repeat('b2', 32), 'hex'), '00000000-0000-4000-8000-000000000937',
     transaction_timestamp(), transaction_timestamp() + interval '10 minutes'),
    (decode(repeat('c3', 32), 'hex'), '00000000-0000-4000-8000-000000000936',
     transaction_timestamp() - interval '2 minutes', transaction_timestamp() - interval '1 minute');

DO $$
DECLARE
    first_hash bytea := decode(repeat('a1', 32), 'hex');
    second_hash bytea := decode(repeat('b2', 32), 'hex');
    expired_hash bytea := decode(repeat('c3', 32), 'hex');
    unknown_hash bytea := decode(repeat('d4', 32), 'hex');
BEGIN
    IF public.ple_account_presentation_get(first_hash) <> 'standard' THEN
        RAISE EXCEPTION 'new account must default to standard contrast';
    END IF;
    IF public.ple_account_presentation_save(first_hash, 'increased') <> 'increased'
       OR public.ple_account_presentation_get(first_hash) <> 'increased' THEN
        RAISE EXCEPTION 'live session must save and read its contrast';
    END IF;
    IF public.ple_account_presentation_get(second_hash) <> 'standard'
       OR public.ple_account_presentation_get(first_hash) <> 'increased' THEN
        RAISE EXCEPTION 'account presentation must remain session-derived and isolated';
    END IF;
    IF public.ple_account_presentation_get(expired_hash) IS NOT NULL
       OR public.ple_account_presentation_get(unknown_hash) IS NOT NULL
       OR public.ple_account_presentation_save(expired_hash, 'standard') IS NOT NULL
       OR public.ple_account_presentation_save(unknown_hash, 'standard') IS NOT NULL
       OR public.ple_account_presentation_save(first_hash, NULL) IS NOT NULL
       OR public.ple_account_presentation_get(first_hash) <> 'increased' THEN
        RAISE EXCEPTION 'expired or unknown session hashes must fail closed';
    END IF;
    IF has_table_privilege(
        'ple_auth', 'public.account_presentation_preference', 'SELECT'
    )
       OR has_table_privilege(
        'ple_auth', 'public.account_presentation_preference', 'INSERT'
    )
       OR has_table_privilege(
        'ple_auth', 'public.account_presentation_preference', 'UPDATE'
    ) THEN
        RAISE EXCEPTION 'ple_auth must have no direct presentation-table privileges';
    END IF;
    BEGIN
        PERFORM 1 FROM public.account_presentation_preference;
        RAISE EXCEPTION 'ple_auth must not read presentation rows directly';
    EXCEPTION WHEN insufficient_privilege THEN
        NULL;
    END;
END
$$;

RESET ROLE;

DO $$
DECLARE
    broker oid := 'ple_account_presentation_broker'::regrole;
    preference_table oid := 'public.account_presentation_preference'::regclass;
    session_table oid := 'public.account_authentication_session'::regclass;
    function_row record;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_roles
         WHERE oid = broker
           AND NOT rolcanlogin
           AND NOT rolsuper
           AND NOT rolcreatedb
           AND NOT rolcreaterole
           AND NOT rolinherit
           AND NOT rolreplication
           AND NOT rolbypassrls
    ) THEN
        RAISE EXCEPTION 'presentation broker role attributes are not least authority';
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_auth_members
         WHERE roleid = broker OR member = broker
    ) THEN
        RAISE EXCEPTION 'presentation broker must have no role memberships';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_class
         WHERE oid = preference_table AND relrowsecurity AND relforcerowsecurity
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_class
         WHERE oid = session_table AND relrowsecurity AND relforcerowsecurity
    ) THEN
        RAISE EXCEPTION 'presentation authority tables must enforce RLS even for owners';
    END IF;
    FOR function_row IN
        SELECT proc.oid, proc.proowner, proc.proacl
          FROM pg_proc AS proc
         WHERE proc.oid IN (
            'public.ple_account_presentation_get(bytea)'::regprocedure,
            'public.ple_account_presentation_save(bytea,text)'::regprocedure
         )
    LOOP
        IF function_row.proowner <> broker
           OR NOT has_function_privilege('ple_auth', function_row.oid, 'EXECUTE')
           OR EXISTS (
                SELECT 1
                  FROM aclexplode(
                    coalesce(function_row.proacl, acldefault('f', function_row.proowner))
                  ) AS privilege
                 WHERE privilege.grantee = 0
                   AND privilege.privilege_type = 'EXECUTE'
            ) THEN
            RAISE EXCEPTION 'presentation function owner or execute scope is incorrect';
        END IF;
    END LOOP;
END
$$;

ROLLBACK;

\echo 'PASS: account-presentation broker authorization oracle passed.'
