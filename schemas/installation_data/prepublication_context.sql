-- Fixed fictional identities plus the temporary ordinary Instructor context
-- used by the Pilot publisher.  The caller supplies only a new opaque session
-- hash and UUID; browser credentials never enter this manifest.

SET LOCAL ROLE ple_private_owner;

INSERT INTO ple_private.account (account_id, product_role, created_at) VALUES
    ('00000000-0000-0000-0000-000000000101', 'instructor', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000102', 'student', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000103', 'student', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000104', 'student', clock_timestamp()),
    ('00000000-0000-0000-0000-000000000105', 'sysadmin', clock_timestamp())
ON CONFLICT (account_id) DO NOTHING;

INSERT INTO ple_private.account_authentication_email (
    account_id, normalized_email, delivery_email, verified_at, updated_at
) VALUES
    ('00000000-0000-0000-0000-000000000101', 'elena.martinez@live-demo.invalid', 'elena.martinez@live-demo.invalid', clock_timestamp(), clock_timestamp()),
    ('00000000-0000-0000-0000-000000000102', 'mary.okafor@live-demo.invalid', 'mary.okafor@live-demo.invalid', clock_timestamp(), clock_timestamp()),
    ('00000000-0000-0000-0000-000000000103', 'jack.nguyen@live-demo.invalid', 'jack.nguyen@live-demo.invalid', clock_timestamp(), clock_timestamp()),
    ('00000000-0000-0000-0000-000000000104', 'avery.thompson@live-demo.invalid', 'avery.thompson@live-demo.invalid', clock_timestamp(), clock_timestamp())
ON CONFLICT (account_id) DO NOTHING;

-- This is Elena's ordinary private workspace.  It remains ordinary product
-- state after the temporary publication session expires.
INSERT INTO ple_private.authoring_workspace (workspace_id, owner_account_id, created_at)
VALUES ('00000000-0000-0000-0000-000000000201',
        '00000000-0000-0000-0000-000000000101', clock_timestamp())
ON CONFLICT (workspace_id) DO NOTHING;

-- A new session is supplied on every publisher invocation.  Use the ordinary
-- session boundary so it validates the active Account and derives its Product
-- Role from the Account.  The coordinator generates a fresh opaque session
-- identity for each invocation; a collision is rejected rather than changing
-- an established session.
SELECT session_id
  FROM ple_private.create_authenticated_session(
      :'pilot_publication_session_id'::uuid,
      '00000000-0000-0000-0000-000000000101'::uuid,
      decode(:'pilot_publication_session_token_hash', 'hex'),
      900
  );

RESET ROLE;
