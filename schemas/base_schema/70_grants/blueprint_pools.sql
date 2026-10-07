-- Privileges from blueprint_pools.sql.

SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.blueprint_pool_members(text, uuid, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.blueprint_pool_members(text, uuid, text) TO ple_app;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.validate_blueprint_pools() FROM PUBLIC;
