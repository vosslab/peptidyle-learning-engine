-- Row security policies from accounts.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.account ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.account FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.account_state_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.account_state_event FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.account_time_zone ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.account_time_zone FORCE ROW LEVEL SECURITY;

CREATE POLICY account_private_owner_access ON ple_private.account
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY account_state_event_private_owner_access ON ple_private.account_state_event
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY account_time_zone_private_owner_access ON ple_private.account_time_zone
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY account_api_owner_access ON ple_private.account
    FOR SELECT TO ple_api_owner USING (true);

CREATE POLICY account_state_event_api_owner_access ON ple_private.account_state_event
    FOR SELECT TO ple_api_owner USING (true);

SET LOCAL ROLE ple_audit_owner;

ALTER TABLE ple_audit.instructor_account_creation_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.instructor_account_creation_event FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.instructor_identity_vetting_decision ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.instructor_identity_vetting_decision FORCE ROW LEVEL SECURITY;

CREATE POLICY instructor_account_creation_event_audit_owner_insert
    ON ple_audit.instructor_account_creation_event FOR INSERT TO ple_audit_owner WITH CHECK (true);

CREATE POLICY instructor_account_creation_event_audit_owner_read
    ON ple_audit.instructor_account_creation_event FOR SELECT TO ple_audit_owner USING (true);

CREATE POLICY instructor_identity_vetting_decision_audit_owner_insert
    ON ple_audit.instructor_identity_vetting_decision FOR INSERT TO ple_audit_owner WITH CHECK (true);

CREATE POLICY instructor_identity_vetting_decision_audit_owner_read
    ON ple_audit.instructor_identity_vetting_decision FOR SELECT TO ple_audit_owner USING (true);

