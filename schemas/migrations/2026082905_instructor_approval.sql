-- SD1 global Instructor vetting; approval is eligibility, never course authority.

DO $$
BEGIN
    IF current_user <> 'ple_migrator'
       OR NOT pg_catalog.pg_has_role('ple_migrator', 'ple_private_owner', 'SET') THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026082905 requires the SD1 private migration principal';
    END IF;
END
$$;

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.instructor_approval (
    instructor_account_id uuid PRIMARY KEY,
    approved_by_account_id uuid NOT NULL,
    approved_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    CONSTRAINT instructor_approval_subject_is_instructor
        FOREIGN KEY (instructor_account_id, role) REFERENCES ple_private.account (account_id, role),
    CONSTRAINT instructor_approval_operator_is_sysadmin
        FOREIGN KEY (approved_by_account_id, approved_by_role)
        REFERENCES ple_private.account (account_id, role),
    role text NOT NULL DEFAULT 'instructor' CHECK (role = 'instructor'),
    approved_by_role text NOT NULL DEFAULT 'sysadmin' CHECK (approved_by_role = 'sysadmin'),
    CONSTRAINT instructor_approval_revocation_is_ordered CHECK (revoked_at IS NULL OR revoked_at >= approved_at)
);
ALTER TABLE ple_private.instructor_approval ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.instructor_approval FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.instructor_approval FROM PUBLIC;
COMMENT ON TABLE ple_private.instructor_approval IS
    'Revocable Sysadmin-recorded global Instructor eligibility; grants no course authority.';

RESET ROLE;
