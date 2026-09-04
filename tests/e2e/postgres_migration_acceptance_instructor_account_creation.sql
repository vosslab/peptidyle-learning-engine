-- PostgreSQL Migration Acceptance Runtime Create Instructor Account authorization and integrity acceptance oracle.
-- Executed by the existing PostgreSQL Migration Acceptance Runtime lane.

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
    IF (SELECT count(*) FROM pg_proc AS proc
        JOIN pg_namespace AS namespace ON namespace.oid = proc.pronamespace
        JOIN pg_roles AS owner_role ON owner_role.oid = proc.proowner
        WHERE namespace.nspname = 'ple_audit'
          AND proc.proname = 'record_instructor_account_creation_event'
          AND pg_get_function_identity_arguments(proc.oid) =
              'p_created_instructor_account_id uuid, p_created_by_sysadmin_account_id uuid'
          AND owner_role.rolname = 'ple_audit_owner'
          AND proc.prosecdef
          AND has_function_privilege('ple_private_owner', proc.oid, 'EXECUTE')
          AND NOT has_function_privilege('ple_app', proc.oid, 'EXECUTE')
          AND NOT has_function_privilege('public', proc.oid, 'EXECUTE')) <> 1
       OR to_regclass('ple_audit.instructor_account_creation_event') IS NULL
       OR NOT (SELECT relation.relrowsecurity AND relation.relforcerowsecurity
                 FROM pg_class AS relation
                 WHERE relation.oid = 'ple_audit.instructor_account_creation_event'::regclass)
       OR (SELECT count(*)
             FROM pg_constraint AS foreign_key
            WHERE foreign_key.conrelid = 'ple_audit.instructor_account_creation_event'::regclass
              AND foreign_key.contype = 'f'
              AND foreign_key.confrelid = 'ple_private.account'::regclass) < 2
       OR NOT EXISTS (
           SELECT 1
             FROM pg_constraint AS foreign_key
            WHERE foreign_key.conrelid = 'ple_audit.instructor_account_creation_event'::regclass
              AND foreign_key.contype = 'f'
              AND foreign_key.confrelid = 'ple_private.account'::regclass
              AND foreign_key.conkey::smallint[] = ARRAY[
                  (SELECT attnum FROM pg_attribute
                    WHERE attrelid = foreign_key.conrelid
                      AND attname = 'created_instructor_account_id'),
                  (SELECT attnum FROM pg_attribute
                    WHERE attrelid = foreign_key.conrelid
                      AND attname = 'created_instructor_product_role')
              ]::smallint[]
              AND foreign_key.confkey::smallint[] = ARRAY[
                  (SELECT attnum FROM pg_attribute
                    WHERE attrelid = foreign_key.confrelid AND attname = 'account_id'),
                  (SELECT attnum FROM pg_attribute
                    WHERE attrelid = foreign_key.confrelid AND attname = 'product_role')
              ]::smallint[]
       )
       OR NOT EXISTS (
           SELECT 1
             FROM pg_constraint AS foreign_key
            WHERE foreign_key.conrelid = 'ple_audit.instructor_account_creation_event'::regclass
              AND foreign_key.contype = 'f'
              AND foreign_key.confrelid = 'ple_private.account'::regclass
              AND foreign_key.conkey::smallint[] = ARRAY[
                  (SELECT attnum FROM pg_attribute
                    WHERE attrelid = foreign_key.conrelid
                      AND attname = 'created_by_sysadmin_account_id'),
                  (SELECT attnum FROM pg_attribute
                    WHERE attrelid = foreign_key.conrelid
                      AND attname = 'created_by_sysadmin_product_role')
              ]::smallint[]
              AND foreign_key.confkey::smallint[] = ARRAY[
                  (SELECT attnum FROM pg_attribute
                    WHERE attrelid = foreign_key.confrelid AND attname = 'account_id'),
                  (SELECT attnum FROM pg_attribute
                    WHERE attrelid = foreign_key.confrelid AND attname = 'product_role')
              ]::smallint[]
       )
       OR has_table_privilege('ple_app', 'ple_audit.instructor_account_creation_event',
                              'SELECT, INSERT, UPDATE, DELETE')
       OR has_table_privilege('ple_private_owner', 'ple_audit.instructor_account_creation_event',
                              'SELECT, INSERT, UPDATE, DELETE')
       OR EXISTS (
           SELECT 1
             FROM pg_policy
            WHERE polrelid = 'ple_audit.instructor_account_creation_event'::regclass
              AND polcmd IN ('w', 'd')
       ) THEN
        RAISE EXCEPTION 'Create Instructor Account audit writer, forced-RLS, or least-privilege boundary is not exact';
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

INSERT INTO ple_private.account (account_id, product_role, created_at) VALUES
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
INSERT INTO ple_private.authenticated_session (
    session_id, account_id, product_role, token_hash, created_at, expires_at
) VALUES (
    '00000000-0000-0000-0000-00000000d105',
    '00000000-0000-0000-0000-00000000d102', 'student', decode(repeat('a1', 32), 'hex'),
    '2026-09-02 00:00:00+00', '2026-09-03 00:00:00+00'
);
INSERT INTO ple_private.account_state_event (event_id, account_id, state, occurred_at, reason)
VALUES (
    '00000000-0000-0000-0000-00000000d106',
    '00000000-0000-0000-0000-00000000d102', 'deactivated',
    '2026-09-02 01:00:00+00', 'canonical-state oracle'
);
DO $$
BEGIN
    IF (SELECT revoked_at FROM ple_private.authenticated_session
        WHERE session_id = '00000000-0000-0000-0000-00000000d105')
       IS DISTINCT FROM '2026-09-02 01:00:00+00'::timestamp with time zone THEN
        RAISE EXCEPTION 'Deactivated Account State must revoke current Authenticated Sessions';
    END IF;
END
$$;
DO $$
BEGIN
    BEGIN
        INSERT INTO ple_private.account_state_event (
            event_id, account_id, state, occurred_at, reason
        ) VALUES (
            '00000000-0000-0000-0000-00000000d107',
            '00000000-0000-0000-0000-00000000d102', 'suspended',
            '2026-09-02 02:00:00+00', 'retired-state oracle'
        );
        RAISE EXCEPTION 'retired suspended Account State was accepted';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;
END
$$;
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
    v_event_count integer;
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
             AND account.created_at = v_created_at AND account.product_role = 'instructor'
             AND event.event_id = v_created_account_id AND event.state = 'active'
             AND event.occurred_at = v_created_at
             AND email.normalized_email = 'iaa1-success@example.test'
             AND email.delivery_email = 'iaa1-success@example.test'
             AND email.verified_at = v_created_at AND email.updated_at = v_created_at) <> 1 THEN
        RAISE EXCEPTION 'Create Instructor Account receipt does not bind the exact atomic records';
    END IF;
    SELECT count(*)::integer INTO v_event_count
      FROM ple_audit.instructor_account_creation_event AS event
     WHERE event.created_instructor_account_id = v_created_account_id
       AND event.created_by_sysadmin_account_id = '00000000-0000-0000-0000-00000000d101'
       AND event.occurred_at = v_created_at;
    IF v_event_count <> 1 THEN
        RAISE EXCEPTION 'successful Create Instructor Account must record exactly one Active Sysadmin audit event';
    END IF;
END
$$;
COMMIT;

DO $$
DECLARE
    v_before_count integer;
    v_after_count integer;
    v_before_email_count integer;
    v_after_email_count integer;
    v_before_event_count integer;
    v_after_event_count integer;
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
        SELECT count(*)::integer INTO v_before_email_count
          FROM ple_private.account_authentication_email;
        SELECT count(*)::integer INTO v_before_event_count
          FROM ple_audit.instructor_account_creation_event;
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
        SELECT count(*)::integer INTO v_after_email_count
          FROM ple_private.account_authentication_email;
        SELECT count(*)::integer INTO v_after_event_count
          FROM ple_audit.instructor_account_creation_event;
        IF v_after_count <> v_before_count OR v_after_email_count <> v_before_email_count
           OR v_after_event_count <> v_before_event_count THEN
            RAISE EXCEPTION 'failed Create Instructor Account left account, Authentication Email, or audit residue';
        END IF;
    END LOOP;
END
$$;

-- An audit-write failure is induced after the Account and Authentication Email
-- writes.  The rejected call must still leave no part of the operation committed.
CREATE FUNCTION public.iaa1_reject_instructor_account_audit_event()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = 'P0001',
        MESSAGE = 'induced Create Instructor Account audit failure';
END
$$;
CREATE TRIGGER iaa1_reject_instructor_account_audit_event
AFTER INSERT ON ple_audit.instructor_account_creation_event
FOR EACH ROW EXECUTE FUNCTION public.iaa1_reject_instructor_account_audit_event();

DO $$
DECLARE
    v_before_account_count integer;
    v_before_email_count integer;
    v_before_event_count integer;
BEGIN
    SELECT count(*)::integer INTO v_before_account_count FROM ple_private.account;
    SELECT count(*)::integer INTO v_before_email_count
      FROM ple_private.account_authentication_email;
    SELECT count(*)::integer INTO v_before_event_count
      FROM ple_audit.instructor_account_creation_event;
    BEGIN
        SET LOCAL ROLE ple_app;
        PERFORM pg_catalog.set_config(
            'ple.session_account_id', '00000000-0000-0000-0000-00000000d101', true
        );
        PERFORM ple_api.create_instructor_account(
            'iaa1-induced-audit-failure@example.test',
            'iaa1-induced-audit-failure@example.test'
        );
        RAISE EXCEPTION 'induced Create Instructor Account audit failure unexpectedly succeeded';
    EXCEPTION WHEN OTHERS THEN
        IF SQLSTATE <> 'P0001' THEN
            RAISE;
        END IF;
    END;
    IF (SELECT count(*) FROM ple_private.account) <> v_before_account_count
       OR (SELECT count(*) FROM ple_private.account_authentication_email) <> v_before_email_count
       OR (SELECT count(*) FROM ple_audit.instructor_account_creation_event) <> v_before_event_count THEN
        RAISE EXCEPTION 'induced Create Instructor Account failure was not atomic';
    END IF;
END
$$;
DROP TRIGGER iaa1_reject_instructor_account_audit_event
    ON ple_audit.instructor_account_creation_event;
DROP FUNCTION public.iaa1_reject_instructor_account_audit_event();

DO $$
DECLARE
    v_event_id uuid;
BEGIN
    SELECT event_id INTO v_event_id
      FROM ple_audit.instructor_account_creation_event
     WHERE created_instructor_account_id = (
         SELECT account_id
           FROM ple_private.account_authentication_email
          WHERE normalized_email = 'iaa1-success@example.test'
     );
    BEGIN
        SET LOCAL ROLE ple_private_owner;
        UPDATE ple_audit.instructor_account_creation_event
           SET occurred_at = occurred_at + interval '1 second'
         WHERE event_id = v_event_id;
        RAISE EXCEPTION 'private Create Instructor Account writer changed immutable audit evidence';
    EXCEPTION WHEN OTHERS THEN
        IF SQLSTATE <> '42501' THEN RAISE; END IF;
    END;
    BEGIN
        SET LOCAL ROLE ple_private_owner;
        DELETE FROM ple_audit.instructor_account_creation_event
         WHERE event_id = v_event_id;
        RAISE EXCEPTION 'private Create Instructor Account writer deleted immutable audit evidence';
    EXCEPTION WHEN OTHERS THEN
        IF SQLSTATE <> '42501' THEN RAISE; END IF;
    END;
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
