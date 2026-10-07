-- Privileges from published_question_metadata_operations.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.replace_published_question_metadata(
    text, integer, bigint, text, text, text, text[], uuid, uuid, uuid, uuid, ple_data.question_type,
    ple_data.bloom_cognitive_process, ple_data.bloom_knowledge_dimension
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.replace_published_question_metadata(
    text, integer, bigint, text, text, text, text[], uuid, uuid, uuid, uuid, ple_data.question_type,
    ple_data.bloom_cognitive_process, ple_data.bloom_knowledge_dimension
) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.replace_published_question_metadata(
    text, integer, bigint, text, text, text, text[], uuid, uuid, uuid, uuid, text, text, text
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.replace_published_question_metadata(
    text, integer, bigint, text, text, text, text[], uuid, uuid, uuid, uuid, text, text, text
) TO ple_app;

REVOKE ALL ON FUNCTION ple_private.bulk_replace_published_question_metadata(jsonb, jsonb)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.bulk_replace_published_question_metadata(jsonb, jsonb)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.bulk_replace_published_question_metadata(jsonb, jsonb)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.bulk_replace_published_question_metadata(jsonb, jsonb)
    TO ple_app;
