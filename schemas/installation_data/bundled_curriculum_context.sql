-- PLE Example Content is the installation-owned, non-login Instructor used
-- solely to publish bundled reusable curriculum through ordinary product paths.
-- The account schema intentionally has no display-name field: source authorship
-- and attribution remain in the curriculum manifest.

SET LOCAL ROLE ple_private_owner;

DO $$
DECLARE
    publisher ple_data.account_id;
    account_placeholder constant text := 'U00000009';
    example_workspace constant uuid := '00000000-0000-0000-0000-000000000202';
BEGIN
    SELECT workspace.owner_account_id INTO publisher
      FROM ple_private.authoring_workspace AS workspace
     WHERE workspace.authoring_workspace_id = example_workspace;
    IF publisher IS NULL THEN
        INSERT INTO ple_private.account (account_id, product_role, created_at)
        VALUES (account_placeholder, 'instructor', pg_catalog.transaction_timestamp())
        RETURNING account_id INTO publisher;
        INSERT INTO ple_private.authoring_workspace (
            authoring_workspace_id, owner_account_id, created_at
        ) VALUES (example_workspace, publisher, clock_timestamp());
    END IF;
END
$$;

-- No authentication email or passkey is inserted. This account cannot be used
-- to sign in; its fresh session is a narrow, transient publisher capability.
SELECT session_id
  FROM ple_private.create_authenticated_session(
      :'example_content_session_id'::uuid,
      (
          SELECT workspace.owner_account_id
            FROM ple_private.authoring_workspace AS workspace
           WHERE workspace.authoring_workspace_id
                 = '00000000-0000-0000-0000-000000000202'::uuid
      ),
      decode(:'example_content_session_token_hash', 'hex'),
      900
  );

RESET ROLE;
