-- Privileges from question_pools.sql.

SET LOCAL ROLE ple_private_owner;

GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.question_pool, ple_data.question_pool_member FROM PUBLIC;

GRANT SELECT ON ple_data.question_pool, ple_data.question_pool_member
    TO ple_private_owner, ple_api_owner;

REVOKE ALL ON FUNCTION ple_data.reject_question_pool_immutable_change() FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.validate_question_pool_lineage_update(),
    ple_data.validate_question_pool_members(),
    ple_data.validate_question_pool_member_insert() FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.create_question_pool(text, text[], integer[], boolean, text, text) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.save_question_pool_members(text, bigint, text[], integer[], boolean) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.construct_question_pool_fork(text, text) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.fork_question_pool_for_course_adoption(text, text) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.fork_question_pool(text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.create_question_pool(text, text[], integer[], boolean, text, text) TO ple_api_owner;

GRANT EXECUTE ON FUNCTION ple_data.save_question_pool_members(text, bigint, text[], integer[], boolean)
    TO ple_api_owner;

GRANT EXECUTE ON FUNCTION ple_data.fork_question_pool(text, text) TO ple_api_owner;

GRANT EXECUTE ON FUNCTION ple_data.fork_question_pool_for_course_adoption(text, text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.create_question_pool(text, text[], integer[], boolean, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.create_question_pool(text, text[], integer[], boolean, text, text) TO ple_app;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.list_published_content_identities() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.list_published_content_identities() TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_published_content_identities() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_published_content_identities() TO ple_app;

REVOKE ALL ON FUNCTION ple_api.resolve_current_published_question_pool(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.resolve_current_published_question_pool(text) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.list_published_question_pools(text, integer, uuid, uuid, uuid, uuid, boolean, jsonb, text[], text, text),
    ple_api.read_current_published_question_pool(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_published_question_pools(text, integer, uuid, uuid, uuid, uuid, boolean, jsonb, text[], text, text),
    ple_api.read_current_published_question_pool(text) TO ple_app;
