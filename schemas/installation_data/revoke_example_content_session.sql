-- ASVS 7.4.1: revoke the exact transient capability even if the ordinary
-- publisher failed.

SET LOCAL ROLE ple_private_owner;

SELECT set_config(
    'ple.installation_example_content_session_id',
    :'example_content_session_id', true
);

DO $$
DECLARE v_session_id uuid := current_setting('ple.installation_example_content_session_id')::uuid;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.authenticated_session
         WHERE authenticated_session.session_id = v_session_id
           AND authenticated_session.account_id = (
                   SELECT workspace.owner_account_id
                     FROM ple_private.authoring_workspace AS workspace
                    WHERE workspace.authoring_workspace_id
                          = '00000000-0000-0000-0000-000000000202'::uuid
               )
           AND authenticated_session.product_role = 'instructor'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'PLE Example Content session is not the supplied temporary Instructor session';
    END IF;
    UPDATE ple_private.authenticated_session
       SET revoked_at = coalesce(revoked_at, clock_timestamp())
     WHERE authenticated_session.session_id = v_session_id;
END
$$;

RESET ROLE;
