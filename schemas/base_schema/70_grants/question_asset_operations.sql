-- Privileges from question_asset_operations.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.validate_question_asset_publication_job(),
    ple_private.claim_question_asset_publication_job(uuid, timestamptz),
    ple_private.activate_question_asset_publication(uuid, uuid),
    ple_private.resolve_ready_question_asset_delivery(text, integer, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.claim_question_asset_publication_job(uuid, timestamptz),
    ple_private.activate_question_asset_publication(uuid, uuid) TO ple_public_asset_publisher;

GRANT EXECUTE ON FUNCTION ple_private.resolve_ready_question_asset_delivery(text, integer, uuid) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.resolve_ready_question_asset(text, integer, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.resolve_ready_question_asset(text, integer, uuid) TO ple_app;

