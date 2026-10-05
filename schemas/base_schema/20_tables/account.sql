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
    account_id ple_data.account_id PRIMARY KEY,
    user_role ple_data.user_role NOT NULL,
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT account_user_role_is_unique UNIQUE (account_id, user_role),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


CREATE TABLE ple_private.account_state_event (
    event_id uuid PRIMARY KEY,
    account_id ple_data.account_id NOT NULL REFERENCES ple_private.account (account_id),
    state ple_data.account_state NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    reason text,
    CONSTRAINT account_state_event_reason_is_present_for_nonactive_state CHECK (
        state = 'active' OR char_length(btrim(reason)) BETWEEN 1 AND 1000
    )
);


CREATE TABLE ple_private.account_time_zone (
    account_id ple_data.account_id PRIMARY KEY REFERENCES ple_private.account (account_id),
    time_zone text NOT NULL CHECK (ple_private.account_time_zone_is_exact_iana(time_zone)),
    student_invitation_default_pending boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);

-- Viewer-owned display choice. NULL means the browser's current preference.
CREATE TABLE ple_private.account_appearance (
    account_id ple_data.account_id PRIMARY KEY REFERENCES ple_private.account (account_id),
    display_mode_preference ple_data.display_mode,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);

-- This row can exist only for an Instructor Account. The role carrier keeps
-- that invariant declarative and lets the FK reject a non-Instructor subject.
CREATE TABLE ple_private.instructor_personal_theme (
    account_id ple_data.account_id PRIMARY KEY,
    instructor_user_role ple_data.user_role NOT NULL DEFAULT 'instructor',
    theme_id text NOT NULL DEFAULT 'grass',
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CONSTRAINT instructor_personal_theme_instructor_user_role_check
        CHECK (instructor_user_role = 'instructor'),
    CHECK (updated_at >= created_at),
    FOREIGN KEY (account_id, instructor_user_role)
        REFERENCES ple_private.account (account_id, user_role)
);

-- The initial public identity supplied by the Sysadmin after outside vetting.
-- Authentication Email remains private and is deliberately not a Profile field.
CREATE TABLE ple_private.instructor_profile (
    account_id ple_data.account_id PRIMARY KEY,
    instructor_user_role ple_data.user_role NOT NULL DEFAULT 'instructor',
    first_name text NOT NULL CHECK (
        first_name = btrim(first_name)
        AND char_length(first_name) BETWEEN 1 AND 100
        AND first_name !~ '[[:cntrl:]]'
    ),
    last_name text NOT NULL CHECK (
        last_name = btrim(last_name)
        AND char_length(last_name) BETWEEN 1 AND 100
        AND last_name !~ '[[:cntrl:]]'
    ),
    affiliation text NOT NULL CHECK (
        affiliation = btrim(affiliation)
        AND char_length(affiliation) BETWEEN 1 AND 300
        AND affiliation !~ '[[:cntrl:]]'
    ),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CONSTRAINT instructor_profile_instructor_user_role_check
        CHECK (instructor_user_role = 'instructor'),
    CHECK (updated_at >= created_at),
    FOREIGN KEY (account_id, instructor_user_role)
        REFERENCES ple_private.account (account_id, user_role)
);


SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.instructor_account_creation_event (
    event_id uuid PRIMARY KEY,
    created_instructor_account_id ple_data.account_id NOT NULL,
    created_instructor_user_role ple_data.user_role NOT NULL DEFAULT 'instructor',
    created_by_sysadmin_account_id ple_data.account_id NOT NULL,
    created_by_sysadmin_user_role ple_data.user_role NOT NULL DEFAULT 'sysadmin',
    occurred_at timestamp with time zone NOT NULL,
    UNIQUE (created_instructor_account_id),
    FOREIGN KEY (created_instructor_account_id, created_instructor_user_role)
        REFERENCES ple_private.account (account_id, user_role),
    FOREIGN KEY (created_by_sysadmin_account_id, created_by_sysadmin_user_role)
        REFERENCES ple_private.account (account_id, user_role)
);

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.public_id_reservation IS 'role: vocabulary, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and User Roles.';

COMMENT ON TABLE ple_private.account IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and User Roles.';

COMMENT ON TABLE ple_private.account_state_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and User Roles.';

COMMENT ON TABLE ple_private.account_time_zone IS 'role: current state, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and User Roles.';
COMMENT ON TABLE ple_private.account_appearance IS 'role: current state, Account-owned display-mode preference; NULL follows the browser setting.';
COMMENT ON TABLE ple_private.instructor_personal_theme IS 'role: current state, Instructor-owned global-page theme preference.';
COMMENT ON TABLE ple_private.instructor_profile IS 'role: current state, Instructor identity created after outside vetting. HUMAN_GUIDANCE.md Instructor role.';
COMMENT ON COLUMN ple_private.account_appearance.display_mode_preference IS
    'NULL follows the browser setting until the Account explicitly chooses Light or Dark.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.instructor_account_creation_event IS 'role: event, deleted by Account deactivation and closure; public IDs are never reclaimed. HUMAN_GUIDANCE.md Accounts and User Roles.';


SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;


SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;


SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;


SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;


SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;


SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;


SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;


SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;


SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;


SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;


SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;


SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;


SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;


SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;


SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;


SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;


SET LOCAL ROLE ple_private_owner;
COMMENT ON COLUMN ple_private.account_state_event.reason IS 'NULL means this optional fact is absent.';
