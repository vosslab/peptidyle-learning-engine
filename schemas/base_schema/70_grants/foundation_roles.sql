-- Privileges from foundation_roles.sql.

RESET ROLE;
REVOKE ALL PRIVILEGES ON TABLE ple_migration._sqlx_migrations FROM PUBLIC;

GRANT SELECT ON TABLE ple_migration._sqlx_migrations TO ple_api_owner;

