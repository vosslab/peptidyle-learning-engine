-- Privileges from api_compatibility.sql.

SET LOCAL ROLE ple_api_owner;

REVOKE ALL PRIVILEGES ON TABLE ple_api.ple_schema_state FROM PUBLIC;

GRANT SELECT ON TABLE ple_api.ple_schema_state TO ple_app;

GRANT SELECT ON TABLE ple_api.ple_schema_state TO ple_migrator;

