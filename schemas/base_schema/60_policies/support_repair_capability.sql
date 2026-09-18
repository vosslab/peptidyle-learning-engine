-- Row security policies from support_repair_capability.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.support_repair_capability ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.support_repair_capability FORCE ROW LEVEL SECURITY;

CREATE POLICY support_repair_capability_private_owner_access
    ON ple_private.support_repair_capability FOR ALL TO ple_private_owner
    USING (true) WITH CHECK (true);

CREATE POLICY support_repair_capability_api_owner_access
    ON ple_private.support_repair_capability FOR ALL TO ple_api_owner
    USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_audit_owner;

ALTER TABLE ple_audit.support_repair_capability_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.support_repair_capability_event FORCE ROW LEVEL SECURITY;

CREATE POLICY support_repair_capability_event_owner_write
    ON ple_audit.support_repair_capability_event FOR INSERT TO ple_audit_owner WITH CHECK (true);

