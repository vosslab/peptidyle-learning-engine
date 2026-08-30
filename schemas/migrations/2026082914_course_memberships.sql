-- SD1 direct Student/Instructor course relationships; Sysadmin is never a course member.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance TO ple_private_owner;
CREATE TABLE ple_data.course_membership (
    membership_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    role text NOT NULL CHECK (role IN ('student', 'instructor')),
    joined_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    CONSTRAINT course_membership_account_role_matches FOREIGN KEY (account_id, role)
        REFERENCES ple_private.account (account_id, role),
    CONSTRAINT course_membership_revocation_is_ordered CHECK (revoked_at IS NULL OR revoked_at >= joined_at)
);
CREATE UNIQUE INDEX course_membership_current_role_idx
    ON ple_data.course_membership (course_id, account_id, role) WHERE revoked_at IS NULL;
CREATE FUNCTION ple_data.assert_assigned_instructor_membership()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
DECLARE
    checked_course_id uuid;
BEGIN
    checked_course_id := CASE WHEN TG_OP = 'DELETE' THEN OLD.course_id ELSE NEW.course_id END;
    IF NOT EXISTS (
        SELECT 1
        FROM ple_data.course_instance AS course
        JOIN ple_data.course_membership AS membership
          ON membership.course_id = course.course_id
         AND membership.account_id = course.assigned_instructor_account_id
         AND membership.role = 'instructor'
         AND membership.revoked_at IS NULL
        WHERE course.course_id = checked_course_id
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'a CourseInstance requires one current assigned Instructor membership';
    END IF;
    RETURN NULL;
END
$$;
CREATE CONSTRAINT TRIGGER course_instance_assigned_instructor_is_current
AFTER INSERT OR UPDATE OF assigned_instructor_account_id, assigned_instructor_role ON ple_data.course_instance
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
EXECUTE FUNCTION ple_data.assert_assigned_instructor_membership();
CREATE CONSTRAINT TRIGGER course_membership_preserves_assigned_instructor
AFTER UPDATE OF account_id, role, revoked_at OR DELETE ON ple_data.course_membership
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
EXECUTE FUNCTION ple_data.assert_assigned_instructor_membership();
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.course_invitation (
    invitation_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    target_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    membership_role text NOT NULL CHECK (membership_role IN ('student', 'instructor')),
    issued_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > issued_at),
    accepted_at timestamp with time zone,
    revoked_at timestamp with time zone,
    CONSTRAINT course_invitation_target_role_matches FOREIGN KEY (target_account_id, membership_role)
        REFERENCES ple_private.account (account_id, role),
    CONSTRAINT course_invitation_terminal_state_is_ordered CHECK (
        (accepted_at IS NULL OR accepted_at >= issued_at) AND (revoked_at IS NULL OR revoked_at >= issued_at)
    )
);
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
ALTER TABLE ple_data.course_membership ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_membership FORCE ROW LEVEL SECURITY;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
ALTER TABLE ple_private.course_invitation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_invitation FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.course_invitation FROM PUBLIC;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
REVOKE ALL PRIVILEGES ON TABLE ple_data.course_membership FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.assert_assigned_instructor_membership() FROM PUBLIC;
RESET ROLE;
