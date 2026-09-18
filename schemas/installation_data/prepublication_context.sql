-- Fixed fictional identities plus the temporary ordinary Instructor context
-- used by the Pilot publisher.  The caller supplies only a new opaque session
-- hash and UUID; browser credentials never enter this manifest.

SET LOCAL ROLE ple_private_owner;

-- Short-circuit existing roots before INSERT. The Account public-ID trigger
-- reserves an ID before conflict handling, so `ON CONFLICT DO NOTHING` would
-- leave an unused reservation on every replay.
INSERT INTO ple_private.account (account_id, product_role, created_at)
SELECT seed.account_id, seed.product_role, clock_timestamp()
  FROM (VALUES
      ('00000000-0000-0000-0000-000000000101'::uuid, 'instructor'::text),
      ('00000000-0000-0000-0000-000000000102'::uuid, 'student'::text),
      ('00000000-0000-0000-0000-000000000103'::uuid, 'student'::text),
      ('00000000-0000-0000-0000-000000000104'::uuid, 'student'::text),
      ('00000000-0000-0000-0000-000000000105'::uuid, 'sysadmin'::text),
      ('00000000-0000-0000-0000-000000000107'::uuid, 'instructor'::text)
  ) AS seed(account_id, product_role)
 WHERE NOT EXISTS (
     SELECT 1 FROM ple_private.account AS existing
      WHERE existing.account_id = seed.account_id
 );

INSERT INTO ple_private.account_authentication_email (
    account_id, normalized_email, delivery_email, verified_at, updated_at
) VALUES
    ('00000000-0000-0000-0000-000000000101', 'elena.martinez@live-demo.invalid', 'elena.martinez@live-demo.invalid', clock_timestamp(), clock_timestamp()),
    ('00000000-0000-0000-0000-000000000102', 'mary.okafor@biology.roosevelt.edu', 'mary.okafor@biology.roosevelt.edu', clock_timestamp(), clock_timestamp()),
    ('00000000-0000-0000-0000-000000000103', 'jack.nguyen@biology.roosevelt.edu', 'jack.nguyen@biology.roosevelt.edu', clock_timestamp(), clock_timestamp()),
    ('00000000-0000-0000-0000-000000000104', 'avery.thompson@biology.roosevelt.edu', 'avery.thompson@biology.roosevelt.edu', clock_timestamp(), clock_timestamp()),
    ('00000000-0000-0000-0000-000000000107', 'priya.shah@live-demo.invalid', 'priya.shah@live-demo.invalid', clock_timestamp(), clock_timestamp())
ON CONFLICT (account_id) DO NOTHING;

-- The canonical Instructor is represented by the same immutable vetting and
-- creation facts as an ordinary Sysadmin-created Instructor.  This is not an
-- Account/Profile display name; later Star projections may use it only through
-- their dedicated authorization procedure.
DO $$
DECLARE decision uuid;
BEGIN
    SELECT ple_audit.record_completed_instructor_identity_vetting_decision(
        'elena.martinez@live-demo.invalid', 'Elena Martinez',
        '00000000-0000-0000-0000-000000000105'::uuid
    ) INTO decision;
    PERFORM ple_audit.record_instructor_account_creation_event(
        '00000000-0000-0000-0000-000000000101'::uuid,
        '00000000-0000-0000-0000-000000000105'::uuid, decision
    );
    -- ASVS 8.3.1: Priya receives ordinary immutable vetting/creation evidence,
    -- not a selector-only role or academic authority grant.
    SELECT ple_audit.record_completed_instructor_identity_vetting_decision(
        'priya.shah@live-demo.invalid', 'Priya Shah',
        '00000000-0000-0000-0000-000000000105'::uuid
    ) INTO decision;
    PERFORM ple_audit.record_instructor_account_creation_event(
        '00000000-0000-0000-0000-000000000107'::uuid,
        '00000000-0000-0000-0000-000000000105'::uuid, decision
    );
END
$$;

-- This is Elena's ordinary private workspace.  It remains ordinary product
-- state after the temporary publication session expires.
INSERT INTO ple_private.authoring_workspace (authoring_workspace_id, owner_account_id, created_at)
VALUES ('00000000-0000-0000-0000-000000000201',
        '00000000-0000-0000-0000-000000000101', clock_timestamp())
ON CONFLICT (authoring_workspace_id) DO NOTHING;

INSERT INTO ple_private.authoring_workspace (authoring_workspace_id, owner_account_id, created_at)
VALUES ('00000000-0000-0000-0000-000000000206',
        '00000000-0000-0000-0000-000000000107', clock_timestamp())
ON CONFLICT (authoring_workspace_id) DO NOTHING;

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
