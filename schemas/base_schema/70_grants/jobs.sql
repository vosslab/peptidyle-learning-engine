-- Privileges from jobs.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.job FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.enforce_job_transition(),
    ple_private.notify_job_ready(),
    ple_private.enqueue_public_asset_publication(uuid, text, integer, jsonb, timestamptz, integer, timestamptz)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.enqueue_public_asset_publication(
    uuid, text, integer, jsonb, timestamptz, integer, timestamptz
)
    TO ple_api_owner;

