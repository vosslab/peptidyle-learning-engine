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
    CONSTRAINT course_membership_account_role_matches FOREIGN KEY (account_id, role)
        REFERENCES ple_private.account (account_id, role)
);
CREATE TABLE ple_data.course_membership_event (
    course_membership_event_id uuid PRIMARY KEY,
    membership_id uuid NOT NULL REFERENCES ple_data.course_membership (membership_id),
    event_kind text NOT NULL CHECK (event_kind IN ('started', 'ended')),
    occurred_at timestamp with time zone NOT NULL,
    reason text NOT NULL CHECK (char_length(btrim(reason)) BETWEEN 1 AND 1000),
    UNIQUE (membership_id, occurred_at, course_membership_event_id)
);
CREATE FUNCTION ple_data.reject_course_membership_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Course Membership episodes are immutable';
END
$$;
CREATE TRIGGER course_membership_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.course_membership
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_course_membership_change();
CREATE FUNCTION ple_data.reject_course_membership_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Course Membership Events are immutable';
END
$$;
CREATE TRIGGER course_membership_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.course_membership_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_course_membership_event_change();
CREATE FUNCTION ple_data.record_course_membership_start()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    INSERT INTO ple_data.course_membership_event (
        course_membership_event_id, membership_id, event_kind, occurred_at, reason
    ) VALUES (NEW.membership_id, NEW.membership_id, 'started', NEW.joined_at, 'membership created');
    RETURN NEW;
END
$$;
CREATE TRIGGER course_membership_creation_records_started_event
AFTER INSERT ON ple_data.course_membership
FOR EACH ROW EXECUTE FUNCTION ple_data.record_course_membership_start();
CREATE FUNCTION ple_data.course_membership_is_active(p_membership_id uuid)
RETURNS boolean LANGUAGE sql STABLE
SET search_path = pg_catalog, ple_data AS $$
    SELECT event.event_kind = 'started'
    FROM ple_data.course_membership_event AS event
    WHERE event.membership_id = p_membership_id
    ORDER BY event.occurred_at DESC, event.course_membership_event_id DESC
    LIMIT 1
$$;
CREATE FUNCTION ple_data.assert_course_membership_event_transition()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
DECLARE
    membership ple_data.course_membership%ROWTYPE;
    current_event_kind text;
BEGIN
    SELECT * INTO membership
    FROM ple_data.course_membership
    WHERE membership_id = NEW.membership_id;
    IF NOT FOUND OR NEW.occurred_at < membership.joined_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Course Membership Event is outside its exact membership episode';
    END IF;
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        pg_catalog.format('%s:%s:%s', membership.course_id, membership.account_id, membership.role), 0
    ));
    SELECT event.event_kind INTO current_event_kind
    FROM ple_data.course_membership_event AS event
    WHERE event.membership_id = NEW.membership_id
    ORDER BY event.occurred_at DESC, event.course_membership_event_id DESC
    LIMIT 1;
    IF (NEW.event_kind = 'started' AND current_event_kind IS NOT NULL)
       OR (NEW.event_kind = 'ended' AND current_event_kind <> 'started') THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Course Membership Event does not follow a valid state transition';
    END IF;
    IF NEW.event_kind = 'started' AND EXISTS (
        SELECT 1
        FROM ple_data.course_membership AS other_membership
        WHERE other_membership.course_id = membership.course_id
          AND other_membership.account_id = membership.account_id
          AND other_membership.role = membership.role
          AND other_membership.membership_id <> membership.membership_id
          AND ple_data.course_membership_is_active(other_membership.membership_id)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23505',
            MESSAGE = 'an Account can have only one active Course Membership for one Course Instance and role';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER course_membership_event_has_valid_transition
BEFORE INSERT ON ple_data.course_membership_event
FOR EACH ROW EXECUTE FUNCTION ple_data.assert_course_membership_event_transition();
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
         AND ple_data.course_membership_is_active(membership.membership_id)
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
AFTER INSERT ON ple_data.course_membership_event
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
    CONSTRAINT course_invitation_target_role_matches FOREIGN KEY (target_account_id, membership_role)
        REFERENCES ple_private.account (account_id, role)
);
CREATE TABLE ple_private.course_invitation_event (
    course_invitation_event_id uuid PRIMARY KEY,
    invitation_id uuid NOT NULL UNIQUE REFERENCES ple_private.course_invitation (invitation_id),
    event_kind text NOT NULL CHECK (event_kind IN ('accepted', 'declined', 'revoked')),
    performed_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    occurred_at timestamp with time zone NOT NULL,
    reason text NOT NULL CHECK (char_length(btrim(reason)) BETWEEN 1 AND 1000)
);
CREATE FUNCTION ple_private.assert_course_invitation_event_is_valid()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
DECLARE
    invitation ple_private.course_invitation%ROWTYPE;
BEGIN
    SELECT * INTO invitation
    FROM ple_private.course_invitation
    WHERE invitation_id = NEW.invitation_id;
    IF NOT FOUND
       OR NEW.occurred_at < invitation.issued_at
       OR NEW.occurred_at >= invitation.expires_at
       OR (NEW.event_kind IN ('accepted', 'declined')
           AND NEW.performed_by_account_id <> invitation.target_account_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Course Invitation Event is outside its exact invitation transition boundary';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER course_invitation_event_has_valid_transition
BEFORE INSERT ON ple_private.course_invitation_event
FOR EACH ROW EXECUTE FUNCTION ple_private.assert_course_invitation_event_is_valid();
CREATE FUNCTION ple_private.reject_course_invitation_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Course Invitation Events are immutable';
END
$$;
CREATE TRIGGER course_invitation_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.course_invitation_event
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_course_invitation_event_change();
CREATE FUNCTION ple_private.reject_course_invitation_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Course Invitations are immutable';
END
$$;
CREATE TRIGGER course_invitation_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.course_invitation
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_course_invitation_change();
COMMENT ON TABLE ple_private.course_invitation_event IS
    'One immutable accepted, declined, or revoked transition for an exact Course Invitation; no event means Pending or Expired by time.';
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
ALTER TABLE ple_data.course_membership ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_membership FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_membership_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_membership_event FORCE ROW LEVEL SECURITY;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
ALTER TABLE ple_private.course_invitation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_invitation FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_invitation_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_invitation_event FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.course_invitation,
    ple_private.course_invitation_event FROM PUBLIC;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
REVOKE ALL PRIVILEGES ON TABLE ple_data.course_membership FROM PUBLIC;
REVOKE ALL PRIVILEGES ON TABLE ple_data.course_membership_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.assert_assigned_instructor_membership() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.assert_course_membership_event_transition() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.course_membership_is_active(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.course_membership_is_active(uuid) TO ple_api_owner;
RESET ROLE;
