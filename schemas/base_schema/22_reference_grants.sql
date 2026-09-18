-- Cross-schema REFERENCES privileges required before 30_constraints.sql
-- adds foreign keys that the table owner cannot declare inline.

SET LOCAL ROLE ple_private_owner;
GRANT REFERENCES ON ALL TABLES IN SCHEMA ple_private
    TO ple_data_owner, ple_audit_owner, ple_api_owner;

SET LOCAL ROLE ple_data_owner;
GRANT REFERENCES ON ALL TABLES IN SCHEMA ple_data
    TO ple_private_owner, ple_audit_owner, ple_api_owner;

SET LOCAL ROLE ple_audit_owner;
GRANT REFERENCES ON ALL TABLES IN SCHEMA ple_audit
    TO ple_private_owner, ple_data_owner, ple_api_owner;
