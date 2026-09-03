-- SD1 Create Instructor Account authorization and integrity acceptance oracle.
-- Executed by the existing staged PostgreSQL 17 acceptance lane.

-- Create Instructor Account has the exact API/private authority and forced-RLS shape.
DO $$
BEGIN
    IF (SELECT count(*) FROM pg_proc AS proc
        JOIN pg_namespace AS namespace ON namespace.oid = proc.pronamespace
        JOIN pg_roles AS owner_role ON owner_role.oid = proc.proowner
        WHERE namespace.nspname = 'ple_api'
          AND proc.proname = 'create_instructor_account'
          AND pg_get_function_identity_arguments(proc.oid) = 'p_normalized_email text, p_delivery_email text'
          AND owner_role.rolname = 'ple_api_owner' AND proc.prosecdef
          AND array_to_string(proc.proconfig, ',') = 'search_path=pg_catalog, ple_api, ple_private'
          AND has_function_privilege('ple_app', proc.oid, 'EXECUTE')
          AND NOT has_function_privilege('public', proc.oid, 'EXECUTE')) <> 1
       OR (SELECT count(*) FROM pg_proc AS proc
        JOIN pg_namespace AS namespace ON namespace.oid = proc.pronamespace
        JOIN pg_roles AS owner_role ON owner_role.oid = proc.proowner
        WHERE namespace.nspname = 'ple_private'
          AND proc.proname = 'create_instructor_account'
          AND pg_get_function_identity_arguments(proc.oid) = 'p_normalized_email text, p_delivery_email text'
          AND owner_role.rolname = 'ple_private_owner' AND proc.prosecdef
          AND array_to_string(proc.proconfig, ',') = 'search_path=pg_catalog, ple_private'
          AND has_function_privilege('ple_api_owner', proc.oid, 'EXECUTE')
          AND NOT has_function_privilege('ple_app', proc.oid, 'EXECUTE')
          AND NOT has_function_privilege('public', proc.oid, 'EXECUTE')) <> 1
       OR to_regprocedure('ple_api.create_account(uuid,text)') IS NOT NULL
       OR to_regprocedure('ple_api.current_session_account_is_sysadmin()') IS NOT NULL
       OR has_schema_privilege('ple_app', 'ple_private', 'USAGE')
       OR has_table_privilege('ple_app', 'ple_private.account', 'SELECT, INSERT, UPDATE, DELETE')
       OR has_table_privilege('ple_app', 'ple_private.account_state_event', 'SELECT, INSERT, UPDATE, DELETE')
       OR has_table_privilege('ple_app', 'ple_private.account_authentication_email', 'SELECT, INSERT, UPDATE, DELETE') THEN
        RAISE EXCEPTION 'Create Instructor Account function ownership or ACL split is not exact';
    END IF;
    IF (SELECT count(*) FROM pg_class AS relation
        JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'ple_private'
          AND relation.relname IN ('account', 'account_state_event', 'account_authentication_email')
          AND relation.relrowsecurity AND relation.relforcerowsecurity) <> 3
       OR (SELECT count(*) FROM pg_policy WHERE polrelid = 'ple_private.account'::regclass
           AND polname = 'account_private_owner_create') <> 1
       OR (SELECT count(*) FROM pg_policy WHERE polrelid = 'ple_private.account_state_event'::regclass
           AND polname IN ('account_state_event_private_owner_read', 'account_state_event_private_owner_create')) <> 2
       OR (SELECT count(*) FROM pg_policy WHERE polrelid = 'ple_private.account_authentication_email'::regclass
           AND polname = 'account_authentication_email_private_owner_create') <> 1
       OR EXISTS (SELECT 1 FROM pg_policy WHERE polrelid = 'ple_private.account_authentication_email'::regclass
                  AND polcmd = 'w') THEN
        RAISE EXCEPTION 'Create Instructor Account forced-RLS policies are not exact';
    END IF;
END
$$;

INSERT INTO ple_private.account (account_id, role, created_at) VALUES
    ('00000000-0000-0000-0000-00000000d101', 'sysadmin', '2026-09-02 00:00:00+00'),
    ('00000000-0000-0000-0000-00000000d102', 'student', '2026-09-02 00:00:00+00'),
    ('00000000-0000-0000-0000-00000000d103', 'sysadmin', '2026-09-02 00:00:00+00');
INSERT INTO ple_private.account_state_event (event_id, account_id, state, occurred_at, reason)
VALUES ('00000000-0000-0000-0000-00000000d104',
    '00000000-0000-0000-0000-00000000d103', 'closed',
    '2026-09-02 00:00:00+00', 'oracle deactivation');
INSERT INTO ple_private.account_authentication_email (
    account_id, normalized_email, delivery_email, verified_at, updated_at
) VALUES ('00000000-0000-0000-0000-00000000d102',
    'iaa1-student@example.test', 'iaa1-student@example.test',
    '2026-09-02 00:00:00+00', '2026-09-02 00:00:00+00');
-- The controlled privileged oracle writer supplies the unsupported Sysadmin
-- fixture without creating a product write path.
BEGIN;
SET LOCAL session_replication_role = replica;
INSERT INTO ple_private.account_authentication_email (
    account_id, normalized_email, delivery_email, verified_at, updated_at
) VALUES ('00000000-0000-0000-0000-00000000d101',
    'iaa1-sysadmin@example.test', 'iaa1-sysadmin@example.test',
    '2026-09-02 00:00:00+00', '2026-09-02 00:00:00+00');
COMMIT;

BEGIN;
DO $$
DECLARE
    v_created_account_id uuid;
    v_created_at timestamp with time zone;
BEGIN
    SET LOCAL ROLE ple_app;
    PERFORM pg_catalog.set_config(
        'ple.session_account_id', '00000000-0000-0000-0000-00000000d101', true
    );
    SELECT account_id, created_at INTO v_created_account_id, v_created_at
      FROM ple_api.create_instructor_account(
          'iaa1-success@example.test', 'iaa1-success@example.test'
      );
    RESET ROLE;
    IF v_created_account_id IS NULL OR v_created_at IS NULL
       OR (SELECT count(*) FROM ple_private.account AS account
           JOIN ple_private.account_state_event AS event ON event.account_id = account.account_id
           JOIN ple_private.account_authentication_email AS email ON email.account_id = account.account_id
           WHERE account.account_id = v_created_account_id
             AND account.created_at = v_created_at AND account.role = 'instructor'
             AND event.event_id = v_created_account_id AND event.state = 'active'
             AND event.occurred_at = v_created_at
             AND email.normalized_email = 'iaa1-success@example.test'
             AND email.delivery_email = 'iaa1-success@example.test'
             AND email.verified_at = v_created_at AND email.updated_at = v_created_at) <> 1 THEN
        RAISE EXCEPTION 'Create Instructor Account receipt does not bind the exact atomic records';
    END IF;
END
$$;
COMMIT;

DO $$
DECLARE
    v_before_count integer;
    v_after_count integer;
    v_email text;
    v_delivery text;
    v_expected_sqlstate text;
BEGIN
    FOR v_email, v_delivery, v_expected_sqlstate IN VALUES
        ('iaa1-student-denied@example.test', 'student@example.test', '42501'),
        ('iaa1-inactive-denied@example.test', 'inactive@example.test', '42501'),
        ('iaa1-success@example.test', 'duplicate@example.test', '23505'),
        (NULL, 'null@example.test', '22023'),
        (' IAA1-noncanonical@example.test ', 'noncanonical@example.test', '22023'),
        ('iaa1-invalid-delivery@example.test', ' ', '22023')
    LOOP
        SELECT count(*)::integer INTO v_before_count FROM ple_private.account;
        BEGIN
            SET LOCAL ROLE ple_app;
            PERFORM pg_catalog.set_config(
                'ple.session_account_id', CASE v_email
                    WHEN 'iaa1-student-denied@example.test' THEN '00000000-0000-0000-0000-00000000d102'
                    WHEN 'iaa1-inactive-denied@example.test' THEN '00000000-0000-0000-0000-00000000d103'
                    ELSE '00000000-0000-0000-0000-00000000d101'
                END, true
            );
            PERFORM ple_api.create_instructor_account(v_email, v_delivery);
            RAISE EXCEPTION 'Create Instructor Account denial unexpectedly succeeded';
        EXCEPTION WHEN OTHERS THEN
            IF SQLSTATE <> v_expected_sqlstate THEN
                RAISE;
            END IF;
        END;
        SELECT count(*)::integer INTO v_after_count FROM ple_private.account;
        IF v_after_count <> v_before_count
           OR (v_email IS NOT NULL AND v_email <> 'iaa1-success@example.test'
               AND EXISTS (SELECT 1 FROM ple_private.account_authentication_email
                           WHERE normalized_email = v_email)) THEN
            RAISE EXCEPTION 'failed Create Instructor Account left an orphan record';
        END IF;
    END LOOP;
END
$$;

DO $$
DECLARE
    v_normalized_email text;
    v_delivery_email text;
    v_verified_at timestamp with time zone;
    v_updated_at timestamp with time zone;
BEGIN
    SELECT normalized_email, delivery_email, verified_at, updated_at
      INTO v_normalized_email, v_delivery_email, v_verified_at, v_updated_at
      FROM ple_private.account_authentication_email
     WHERE account_id = '00000000-0000-0000-0000-00000000d102';
    BEGIN
        UPDATE ple_private.account_authentication_email
           SET normalized_email = 'iaa1-student-update@example.test'
         WHERE account_id = '00000000-0000-0000-0000-00000000d102';
        RAISE EXCEPTION 'Student Authentication Email normalized update succeeded';
    EXCEPTION WHEN OTHERS THEN IF SQLSTATE <> '23514' THEN RAISE; END IF;
    END;
    BEGIN
        UPDATE ple_private.account_authentication_email
           SET delivery_email = 'iaa1-student-update@example.test',
               updated_at = updated_at + interval '1 second'
         WHERE account_id = '00000000-0000-0000-0000-00000000d102';
        RAISE EXCEPTION 'Student Authentication Email delivery or timestamp update succeeded';
    EXCEPTION WHEN OTHERS THEN IF SQLSTATE <> '23514' THEN RAISE; END IF;
    END;
    IF NOT EXISTS (SELECT 1 FROM ple_private.account_authentication_email
        WHERE account_id = '00000000-0000-0000-0000-00000000d102'
          AND normalized_email = v_normalized_email AND delivery_email = v_delivery_email
          AND verified_at = v_verified_at AND updated_at = v_updated_at) THEN
        RAISE EXCEPTION 'Student Authentication Email changed after rejected updates';
    END IF;
    BEGIN
        INSERT INTO ple_private.account_authentication_email (
            account_id, normalized_email, delivery_email, verified_at, updated_at
        ) VALUES ('00000000-0000-0000-0000-00000000d103',
            'iaa1-sysadmin-new@example.test', 'iaa1-sysadmin-new@example.test',
            '2026-09-02 00:00:00+00', '2026-09-02 00:00:00+00');
        RAISE EXCEPTION 'Sysadmin Authentication Email insert succeeded';
    EXCEPTION WHEN OTHERS THEN IF SQLSTATE <> '23514' THEN RAISE; END IF;
    END;
    BEGIN
        UPDATE ple_private.account_authentication_email
           SET updated_at = updated_at + interval '1 second'
         WHERE account_id = '00000000-0000-0000-0000-00000000d101';
        RAISE EXCEPTION 'Sysadmin Authentication Email update succeeded';
    EXCEPTION WHEN OTHERS THEN IF SQLSTATE <> '23514' THEN RAISE; END IF;
    END;
    BEGIN
        SET LOCAL ROLE ple_app;
        PERFORM 1 FROM ple_private.account_authentication_email;
        RAISE EXCEPTION 'ple_app read a private Authentication Email';
    EXCEPTION WHEN OTHERS THEN IF SQLSTATE <> '42501' THEN RAISE; END IF;
    END;
END
$$;
