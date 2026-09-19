-- Fictional identities plus the temporary ordinary Instructor context
-- used by the Pilot publisher.  The caller supplies only a new opaque session
-- hash and UUID; browser credentials never enter this manifest.
-- Account primary keys are server-minted public IDs. Email is the stable
-- seed lookup.

SET LOCAL ROLE ple_private_owner;

DO $$
DECLARE
    elena ple_data.account_id;
    mary ple_data.account_id;
    jack ple_data.account_id;
    avery ple_data.account_id;
    sysadmin_id ple_data.account_id;
    priya ple_data.account_id;
    decision uuid;
    account_placeholder constant text := 'U00000009';
BEGIN
    SELECT email.account_id INTO elena
      FROM ple_private.account_authentication_email AS email
     WHERE email.normalized_email = 'elena.martinez@live-demo.invalid';
    IF elena IS NULL THEN
        INSERT INTO ple_private.account (account_id, product_role, created_at)
        VALUES (account_placeholder, 'instructor', pg_catalog.transaction_timestamp())
        RETURNING account_id INTO elena;
        INSERT INTO ple_private.account_authentication_email (
            account_id, normalized_email, delivery_email, verified_at, updated_at
        ) VALUES (
            elena, 'elena.martinez@live-demo.invalid',
            'elena.martinez@live-demo.invalid', clock_timestamp(), clock_timestamp()
        );
    END IF;

    SELECT email.account_id INTO mary
      FROM ple_private.account_authentication_email AS email
     WHERE email.normalized_email = 'mary.okafor@biology.roosevelt.edu';
    IF mary IS NULL THEN
        INSERT INTO ple_private.account (account_id, product_role, created_at)
        VALUES (account_placeholder, 'student', pg_catalog.transaction_timestamp())
        RETURNING account_id INTO mary;
        INSERT INTO ple_private.account_authentication_email (
            account_id, normalized_email, delivery_email, verified_at, updated_at
        ) VALUES (
            mary, 'mary.okafor@biology.roosevelt.edu',
            'mary.okafor@biology.roosevelt.edu', clock_timestamp(), clock_timestamp()
        );
    END IF;

    SELECT email.account_id INTO jack
      FROM ple_private.account_authentication_email AS email
     WHERE email.normalized_email = 'jack.nguyen@biology.roosevelt.edu';
    IF jack IS NULL THEN
        INSERT INTO ple_private.account (account_id, product_role, created_at)
        VALUES (account_placeholder, 'student', pg_catalog.transaction_timestamp())
        RETURNING account_id INTO jack;
        INSERT INTO ple_private.account_authentication_email (
            account_id, normalized_email, delivery_email, verified_at, updated_at
        ) VALUES (
            jack, 'jack.nguyen@biology.roosevelt.edu',
            'jack.nguyen@biology.roosevelt.edu', clock_timestamp(), clock_timestamp()
        );
    END IF;

    SELECT email.account_id INTO avery
      FROM ple_private.account_authentication_email AS email
     WHERE email.normalized_email = 'avery.thompson@biology.roosevelt.edu';
    IF avery IS NULL THEN
        INSERT INTO ple_private.account (account_id, product_role, created_at)
        VALUES (account_placeholder, 'student', pg_catalog.transaction_timestamp())
        RETURNING account_id INTO avery;
        INSERT INTO ple_private.account_authentication_email (
            account_id, normalized_email, delivery_email, verified_at, updated_at
        ) VALUES (
            avery, 'avery.thompson@biology.roosevelt.edu',
            'avery.thompson@biology.roosevelt.edu', clock_timestamp(), clock_timestamp()
        );
    END IF;

    SELECT email.account_id INTO sysadmin_id
      FROM ple_private.account_authentication_email AS email
     WHERE email.normalized_email = 'morgan.delgado@live-demo.invalid';
    IF sysadmin_id IS NULL THEN
        SELECT account.account_id INTO sysadmin_id
          FROM ple_private.account AS account
         WHERE account.product_role = 'sysadmin'
         ORDER BY account.created_at, account.account_id
         LIMIT 1;
    END IF;
    IF sysadmin_id IS NULL THEN
        INSERT INTO ple_private.account (account_id, product_role, created_at)
        VALUES (account_placeholder, 'sysadmin', pg_catalog.transaction_timestamp())
        RETURNING account_id INTO sysadmin_id;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.account_authentication_email AS email
         WHERE email.normalized_email = 'morgan.delgado@live-demo.invalid'
    ) THEN
        INSERT INTO ple_private.account_authentication_email (
            account_id, normalized_email, delivery_email, verified_at, updated_at
        ) VALUES (
            sysadmin_id, 'morgan.delgado@live-demo.invalid',
            'morgan.delgado@live-demo.invalid', clock_timestamp(), clock_timestamp()
        );
    END IF;

    SELECT email.account_id INTO priya
      FROM ple_private.account_authentication_email AS email
     WHERE email.normalized_email = 'priya.shah@live-demo.invalid';
    IF priya IS NULL THEN
        INSERT INTO ple_private.account (account_id, product_role, created_at)
        VALUES (account_placeholder, 'instructor', pg_catalog.transaction_timestamp())
        RETURNING account_id INTO priya;
        INSERT INTO ple_private.account_authentication_email (
            account_id, normalized_email, delivery_email, verified_at, updated_at
        ) VALUES (
            priya, 'priya.shah@live-demo.invalid',
            'priya.shah@live-demo.invalid', clock_timestamp(), clock_timestamp()
        );
    END IF;

    SELECT ple_audit.record_completed_instructor_identity_vetting_decision(
        'elena.martinez@live-demo.invalid', 'Elena Martinez', sysadmin_id
    ) INTO decision;
    PERFORM ple_audit.record_instructor_account_creation_event(
        elena, sysadmin_id, decision
    );
    SELECT ple_audit.record_completed_instructor_identity_vetting_decision(
        'priya.shah@live-demo.invalid', 'Priya Shah', sysadmin_id
    ) INTO decision;
    PERFORM ple_audit.record_instructor_account_creation_event(
        priya, sysadmin_id, decision
    );

    INSERT INTO ple_private.authoring_workspace (
        authoring_workspace_id, owner_account_id, created_at
    ) VALUES (
        '00000000-0000-0000-0000-000000000201', elena, pg_catalog.transaction_timestamp()
    ) ON CONFLICT (authoring_workspace_id) DO NOTHING;

    INSERT INTO ple_private.authoring_workspace (
        authoring_workspace_id, owner_account_id, created_at
    ) VALUES (
        '00000000-0000-0000-0000-000000000206', priya, pg_catalog.transaction_timestamp()
    ) ON CONFLICT (authoring_workspace_id) DO NOTHING;
END
$$;

SELECT session_id
  FROM ple_private.create_authenticated_session(
      :'pilot_publication_session_id'::uuid,
      (
          SELECT email.account_id
            FROM ple_private.account_authentication_email AS email
           WHERE email.normalized_email = 'elena.martinez@live-demo.invalid'
      ),
      decode(:'pilot_publication_session_token_hash', 'hex'),
      900
  );

RESET ROLE;
