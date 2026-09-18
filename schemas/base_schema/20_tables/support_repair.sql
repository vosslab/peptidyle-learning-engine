-- support_repair tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_private_owner;

-- Narrow, audited repair-capability registry.  A capability is a request for
-- one existing Course-local Student roster profile; it never confers Course
-- membership. Course/content repair authority remains future implementation.
CREATE TABLE ple_private.support_repair_capability (
    support_repair_capability_id uuid PRIMARY KEY,
    sysadmin_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    sysadmin_role ple_data.product_role NOT NULL DEFAULT 'sysadmin',
    issuer_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    resource_class text NOT NULL DEFAULT 'student',
    resource_reference text NOT NULL CHECK (
        resource_reference = btrim(resource_reference)
        AND char_length(resource_reference) BETWEEN 1 AND 512
        AND resource_reference !~ '[[:cntrl:]]'
    ),
    purpose text NOT NULL CHECK (purpose = btrim(purpose) AND char_length(purpose) BETWEEN 1 AND 1000
        AND purpose !~ '[[:cntrl:]]'),
    issued_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > issued_at),
    revoked_at timestamp with time zone,
    FOREIGN KEY (sysadmin_account_id, sysadmin_role)
        REFERENCES ple_private.account (account_id, product_role),
    CHECK (revoked_at IS NULL OR revoked_at >= issued_at),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.support_repair_capability_event (
    event_id uuid PRIMARY KEY,
    support_repair_capability_id uuid NOT NULL REFERENCES ple_private.support_repair_capability (support_repair_capability_id),
    sysadmin_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    issuer_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    resource_class text NOT NULL DEFAULT 'student',
    resource_reference text NOT NULL CHECK (resource_reference = btrim(resource_reference)
        AND char_length(resource_reference) BETWEEN 1 AND 512 AND resource_reference !~ '[[:cntrl:]]'),
    purpose text NOT NULL CHECK (purpose = btrim(purpose) AND char_length(purpose) BETWEEN 1 AND 1000
        AND purpose !~ '[[:cntrl:]]'),
    result ple_data.repair_result NOT NULL,
    occurred_at timestamp with time zone NOT NULL
);


SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.support_repair_capability IS 'role: current state, deleted by capability revoke. HUMAN_GUIDANCE.md Support repair.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.support_repair_capability_event IS 'role: event, deleted by capability revoke. HUMAN_GUIDANCE.md Support repair.';



SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.support_repair_capability IS 'role: current state, deleted by capability revoke. HUMAN_GUIDANCE.md Support repair.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.support_repair_capability_event IS 'role: event, deleted by capability revoke. HUMAN_GUIDANCE.md Support repair.';



SET LOCAL ROLE ple_audit_owner;


SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.support_repair_capability IS 'role: current state, deleted by capability revoke. HUMAN_GUIDANCE.md Support repair.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.support_repair_capability_event IS 'role: event, deleted by capability revoke. HUMAN_GUIDANCE.md Support repair.';



SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.support_repair_capability IS 'role: current state, deleted by capability revoke. HUMAN_GUIDANCE.md Support repair.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.support_repair_capability_event IS 'role: event, deleted by capability revoke. HUMAN_GUIDANCE.md Support repair.';




SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.support_repair_capability IS 'role: current state, deleted by capability revoke. HUMAN_GUIDANCE.md Support repair.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.support_repair_capability_event IS 'role: event, deleted by capability revoke. HUMAN_GUIDANCE.md Support repair.';



SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.support_repair_capability IS 'role: current state, deleted by capability revoke. HUMAN_GUIDANCE.md Support repair.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.support_repair_capability_event IS 'role: event, deleted by capability revoke. HUMAN_GUIDANCE.md Support repair.';

