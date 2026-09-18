-- Privileges from blueprint_revision_integrity.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_blueprint_revision_change(),
    ple_data.reject_blueprint_event_change(),
    ple_data.reject_sealed_blueprint_revision_child_insert() FROM PUBLIC;

