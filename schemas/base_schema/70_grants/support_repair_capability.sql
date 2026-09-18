-- Privileges from support_repair_capability.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.support_repair_capability FROM PUBLIC;

GRANT SELECT, INSERT, UPDATE (revoked_at) ON ple_private.support_repair_capability TO ple_api_owner;

GRANT REFERENCES ON TABLE ple_private.support_repair_capability TO ple_audit_owner;

REVOKE ALL ON FUNCTION ple_private.reject_support_repair_capability_change() FROM PUBLIC;

SET LOCAL ROLE ple_audit_owner;

REVOKE ALL ON TABLE ple_audit.support_repair_capability_event FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_audit.reject_support_repair_capability_event_change(),
    ple_audit.record_support_repair_capability_event(uuid, text, text, text, text, text, text)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_audit.record_support_repair_capability_event(uuid, text, text, text, text, text, text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.support_repair_roster_course(text) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_api.issue_support_repair_capability(text, text, text, text, uuid),
    ple_api.revoke_support_repair_capability(uuid),
    ple_api.record_support_repair_capability_use(uuid, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.issue_support_repair_capability(text, text, text, text, uuid),
    ple_api.revoke_support_repair_capability(uuid),
    ple_api.record_support_repair_capability_use(uuid, text, text) TO ple_app;

