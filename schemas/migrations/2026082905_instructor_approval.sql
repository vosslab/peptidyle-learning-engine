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

CREATE TABLE ple_private.instructor_approval_event (
    instructor_approval_event_id uuid PRIMARY KEY,
    instructor_account_id uuid NOT NULL,
    instructor_role text NOT NULL DEFAULT 'instructor' CHECK (instructor_role = 'instructor'),
    authorizing_sysadmin_account_id uuid NOT NULL,
    authorizing_sysadmin_role text NOT NULL DEFAULT 'sysadmin'
        CHECK (authorizing_sysadmin_role = 'sysadmin'),
    event_kind text NOT NULL CHECK (event_kind IN ('approved', 'revoked')),
    occurred_at timestamp with time zone NOT NULL,
    reason text NOT NULL CHECK (char_length(btrim(reason)) BETWEEN 1 AND 1000),
    CONSTRAINT instructor_approval_event_subject_is_instructor
        FOREIGN KEY (instructor_account_id, instructor_role)
        REFERENCES ple_private.account (account_id, role),
    CONSTRAINT instructor_approval_event_authorizer_is_sysadmin
        FOREIGN KEY (authorizing_sysadmin_account_id, authorizing_sysadmin_role)
        REFERENCES ple_private.account (account_id, role),
    UNIQUE (instructor_account_id, occurred_at, instructor_approval_event_id)
);
CREATE FUNCTION ple_private.reject_instructor_approval_event_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Instructor Approval Events are immutable';
END
$$;
CREATE TRIGGER instructor_approval_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.instructor_approval_event
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_instructor_approval_event_change();
ALTER TABLE ple_private.instructor_approval_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.instructor_approval_event FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.instructor_approval_event FROM PUBLIC;
COMMENT ON TABLE ple_private.instructor_approval_event IS
    'Immutable Sysadmin-authorized approved or revoked global Instructor eligibility evidence; grants no course authority.';

RESET ROLE;
