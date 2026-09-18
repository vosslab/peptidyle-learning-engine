-- Privileges from published_question_metadata_operations.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.bulk_replace_published_question_metadata(jsonb, jsonb)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.bulk_replace_published_question_metadata(jsonb, jsonb)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.bulk_replace_published_question_metadata(jsonb, jsonb)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.bulk_replace_published_question_metadata(jsonb, jsonb)
    TO ple_app;

