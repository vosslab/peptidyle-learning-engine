-- PLE Example Content is the installation-owned, non-login Instructor used
-- solely to publish bundled reusable curriculum through ordinary product paths.
-- The account schema intentionally has no display-name field: source authorship
-- and attribution remain in the curriculum manifest.

SET LOCAL ROLE ple_private_owner;

-- Avoid invoking the public-ID trigger on an already seeded Account. A real
-- concurrent root conflict aborts instead, rolling back its reservation.
INSERT INTO ple_private.account (account_id, product_role, created_at)
SELECT '00000000-0000-0000-0000-000000000106'::uuid, 'instructor', clock_timestamp()
 WHERE NOT EXISTS (
     SELECT 1 FROM ple_private.account
      WHERE account_id = '00000000-0000-0000-0000-000000000106'::uuid
 );

-- No authentication email or passkey is inserted. This account cannot be used
-- to sign in; its fresh session is a narrow, transient publisher capability.
INSERT INTO ple_private.authoring_workspace (workspace_id, owner_account_id, created_at)
VALUES ('00000000-0000-0000-0000-000000000202',
        '00000000-0000-0000-0000-000000000106', clock_timestamp())
ON CONFLICT (workspace_id) DO NOTHING;

SELECT session_id
  FROM ple_private.create_authenticated_session(
      :'example_content_session_id'::uuid,
      '00000000-0000-0000-0000-000000000106'::uuid,
      decode(:'example_content_session_token_hash', 'hex'),
      900
  );

RESET ROLE;
