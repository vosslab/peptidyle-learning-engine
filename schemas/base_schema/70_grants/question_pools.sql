-- Privileges from question_pools.sql.

SET LOCAL ROLE ple_private_owner;

GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.question_pool, ple_data.question_pool_member FROM PUBLIC;

GRANT SELECT ON ple_data.question_pool, ple_data.question_pool_member
    TO ple_private_owner, ple_api_owner;

-- ASVS 8.2.1: search metadata may change; established Discipline and Subject may not.
GRANT UPDATE (question_pool_metadata_edit_number, title, description, content_topic_id, content_subtopic_id,
    tags, bloom_cognitive_process, bloom_knowledge_dimension, updated_on)
    ON ple_data.question_pool TO ple_private_owner;

-- ASVS 8.2.3: Pool support text may change. Member Question columns are not in this grant.
GRANT UPDATE (question_pool_metadata_edit_number, hint, general_feedback, worked_solution, updated_on)
    ON ple_data.question_pool TO ple_private_owner;

REVOKE ALL ON FUNCTION ple_data.reject_question_pool_immutable_change() FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.calculate_question_pool_license(text[], integer[]),
    ple_data.current_question_pool_license(text) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.validate_question_pool_lineage_update(),
    ple_data.validate_question_pool_members(),
    ple_data.validate_question_pool_member_insert() FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.create_question_pool(text, text[], integer[], text, text, text[]) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.save_question_pool_members(text, bigint, text[], integer[]) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.construct_question_pool_fork(text, text, text) FROM PUBLIC;


REVOKE ALL ON FUNCTION ple_data.fork_question_pool(text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.create_question_pool(text, text[], integer[], text, text, text[]) TO ple_api_owner;

GRANT EXECUTE ON FUNCTION ple_data.save_question_pool_members(text, bigint, text[], integer[])
    TO ple_api_owner;

GRANT EXECUTE ON FUNCTION ple_data.fork_question_pool(text, text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.fork_question_pool(text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.fork_question_pool(text, text) TO ple_app;


SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.create_question_pool(text, text[], integer[], text, text, text[]) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.create_question_pool(text, text[], integer[], text, text, text[]) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.save_question_pool_members(text, bigint, text[], integer[]) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.save_question_pool_members(text, bigint, text[], integer[])
    TO ple_app;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.list_published_content_identities() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.list_published_content_identities() TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_published_content_identities() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_published_content_identities() TO ple_app;

REVOKE ALL ON FUNCTION ple_api.resolve_current_published_question_pool(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.resolve_current_published_question_pool(text) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.read_current_published_question_pool(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_current_published_question_pool(text) TO ple_app;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.replace_question_pool_metadata(
    text, bigint, text, text, uuid, uuid, text[],
    ple_data.bloom_cognitive_process, ple_data.bloom_knowledge_dimension
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.replace_question_pool_metadata(
    text, bigint, text, text, uuid, uuid, text[],
    ple_data.bloom_cognitive_process, ple_data.bloom_knowledge_dimension
) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.read_current_question_pool_metadata(text),
    ple_api.replace_question_pool_metadata(
        text, bigint, text, text, uuid, uuid, text[], text, text
    ) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_current_question_pool_metadata(text),
    ple_api.replace_question_pool_metadata(
        text, bigint, text, text, uuid, uuid, text[], text, text
    ) TO ple_app;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.bulk_replace_question_pool_search_metadata(jsonb, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.bulk_replace_question_pool_search_metadata(jsonb, jsonb)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.bulk_replace_question_pool_search_metadata(jsonb, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.bulk_replace_question_pool_search_metadata(jsonb, jsonb)
    TO ple_app;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.stored_question_pool_support_text(text),
    ple_private.require_question_pool_support_caller(text),
    ple_private.read_question_pool_ple_managed_support(text),
    ple_private.save_question_pool_ple_managed_support(text, bigint, text, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.read_question_pool_ple_managed_support(text),
    ple_private.save_question_pool_ple_managed_support(text, bigint, text, text, text)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.read_question_pool_ple_managed_support(text),
    ple_api.save_question_pool_ple_managed_support(text, bigint, text, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_question_pool_ple_managed_support(text),
    ple_api.save_question_pool_ple_managed_support(text, bigint, text, text, text)
    TO ple_app;
