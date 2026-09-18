-- account tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_private_owner;



-- This append-only registry is the sole allocation authority for public IDs.
-- Its primary key covers every public object type, including the shared
-- Question/Question Pool namespace. Rows are deliberately never reclaimed:
-- deletion or archival of the object cannot make its public ID reusable.
CREATE TABLE ple_private.public_id_reservation (
    canonical_public_id text PRIMARY KEY,
    object_kind ple_data.public_id_object_kind NOT NULL,
    reserved_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


-- Global account identity, current state, profile preference, and account audit.
-- Credential material and sessions are owned by authentication.sql.
CREATE TABLE ple_private.account (
    account_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    public_reference text NOT NULL UNIQUE CHECK (
        ple_private.is_canonical_prefixed_public_id(public_reference, 'U')
    ),
    product_role ple_data.product_role NOT NULL,
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT account_product_role_is_unique UNIQUE (account_id, product_role),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);



CREATE TABLE ple_private.account_state_event (
    event_id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    state ple_data.account_state NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    reason text,
    CONSTRAINT account_state_event_reason_is_present_for_nonactive_state CHECK (
        state = 'active' OR char_length(btrim(reason)) BETWEEN 1 AND 1000
    )
);


CREATE TABLE ple_private.account_time_zone (
    account_id uuid PRIMARY KEY REFERENCES ple_private.account (account_id),
    time_zone text NOT NULL CHECK (ple_private.account_time_zone_is_exact_iana(time_zone)),
    student_invitation_default_pending boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.instructor_account_creation_event (
    event_id uuid PRIMARY KEY,
    created_instructor_account_id uuid NOT NULL,
    created_instructor_product_role ple_data.product_role NOT NULL DEFAULT 'instructor',
    created_by_sysadmin_account_id uuid NOT NULL,
    created_by_sysadmin_product_role ple_data.product_role NOT NULL DEFAULT 'sysadmin',
    instructor_identity_vetting_decision_id uuid NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    UNIQUE (created_instructor_account_id),
    FOREIGN KEY (created_instructor_account_id, created_instructor_product_role)
        REFERENCES ple_private.account (account_id, product_role),
    FOREIGN KEY (created_by_sysadmin_account_id, created_by_sysadmin_product_role)
        REFERENCES ple_private.account (account_id, product_role)
);



-- A completed identity check is evidence about a candidate identity, not an
-- Account state.  Keeping it separate means no caller can manufacture an
-- unapproved Instructor Account and later try to restrict its capabilities.
CREATE TABLE ple_audit.instructor_identity_vetting_decision (
    decision_id uuid PRIMARY KEY,
    normalized_email text NOT NULL UNIQUE CHECK (
        char_length(normalized_email) BETWEEN 3 AND 320
        AND normalized_email = lower(btrim(normalized_email))
    ),
    -- The vetted display name is an immutable audit fact.  It is deliberately
    -- not an Account/Profile/directory attribute and has no general projection.
    verified_instructor_display_name text NOT NULL CHECK (
        verified_instructor_display_name = btrim(verified_instructor_display_name)
        AND char_length(verified_instructor_display_name) BETWEEN 1 AND 200
        AND verified_instructor_display_name !~ '[[:cntrl:]]'
    ),
    completed_by_sysadmin_account_id uuid NOT NULL,
    completed_by_sysadmin_product_role ple_data.product_role NOT NULL DEFAULT 'sysadmin',
    completed_at timestamp with time zone NOT NULL,
    FOREIGN KEY (completed_by_sysadmin_account_id, completed_by_sysadmin_product_role)
        REFERENCES ple_private.account (account_id, product_role)
);

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.public_id_reservation IS 'role: vocabulary, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_state_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_time_zone IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.instructor_account_creation_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_audit.instructor_identity_vetting_decision IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.public_id_reservation IS 'role: vocabulary, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_state_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_time_zone IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.instructor_account_creation_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_audit.instructor_identity_vetting_decision IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';



SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.public_id_reservation IS 'role: vocabulary, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_state_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_time_zone IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.instructor_account_creation_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_audit.instructor_identity_vetting_decision IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';



SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.public_id_reservation IS 'role: vocabulary, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_state_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_time_zone IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.instructor_account_creation_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_audit.instructor_identity_vetting_decision IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.public_id_reservation IS 'role: vocabulary, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_state_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_time_zone IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.instructor_account_creation_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_audit.instructor_identity_vetting_decision IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';



SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.public_id_reservation IS 'role: vocabulary, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_state_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_time_zone IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.instructor_account_creation_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_audit.instructor_identity_vetting_decision IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';



SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.public_id_reservation IS 'role: vocabulary, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_state_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_time_zone IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.instructor_account_creation_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_audit.instructor_identity_vetting_decision IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.public_id_reservation IS 'role: vocabulary, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_state_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_time_zone IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.instructor_account_creation_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_audit.instructor_identity_vetting_decision IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';



SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.public_id_reservation IS 'role: vocabulary, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_state_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_private.account_time_zone IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.instructor_account_creation_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

COMMENT ON TABLE ple_audit.instructor_identity_vetting_decision IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and Product Roles.';

