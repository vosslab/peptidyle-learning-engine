-- Privileges from blueprint_pools.sql.

SET LOCAL ROLE ple_data_owner;

-- Blueprint-owned Pools use exact current Assessment membership, not a second ownership table.
GRANT EXECUTE ON FUNCTION ple_data.fork_question_pool_revision(text, text),
    ple_data.save_question_pool_members(text, bigint, text[], integer[], boolean) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.blueprint_pool_write_receipt(text, bytea) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.blueprint_pool_write_receipt(text, bytea) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.fork_blueprint_question_pool(text, text, text),
    ple_api.blueprint_pool_members(text, uuid, text, boolean, bigint, bigint),
    ple_api.append_blueprint_pool_revision(text, uuid, bigint, text, bigint, text[], integer[], boolean) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.fork_blueprint_question_pool(text, text, text),
    ple_api.blueprint_pool_members(text, uuid, text, boolean, bigint, bigint),
    ple_api.append_blueprint_pool_revision(text, uuid, bigint, text, bigint, text[], integer[], boolean) TO ple_app;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.validate_blueprint_owned_pools() FROM PUBLIC;

