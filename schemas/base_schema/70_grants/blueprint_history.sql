-- Privileges from blueprint_history.sql.

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_blueprint_history(text, text, text, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_blueprint_history(text, text, text, integer) TO ple_app;

