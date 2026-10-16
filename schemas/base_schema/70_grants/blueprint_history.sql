-- Privileges from blueprint_history.sql.

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_blueprint_history(text, text, text, text, text, text[], text, bigint, integer) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.blueprint_history_availability_facets(text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_blueprint_history(text, text, text, text, text, text[], text, bigint, integer) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.blueprint_history_availability_facets(text, text) TO ple_app;

