-- Invitation and roster evidence.  Email/persona resolution is owned by accounts.sql.

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.course_invitation (
    invitation_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    target_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    membership_role text NOT NULL CHECK (membership_role IN ('student','instructor')),
    inviting_instructor_account_id uuid NOT NULL,
    inviting_instructor_role text NOT NULL DEFAULT 'instructor'
        CHECK (inviting_instructor_role = 'instructor'),
    issued_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > issued_at),
    FOREIGN KEY (target_account_id,membership_role)
        REFERENCES ple_private.account(account_id,product_role),
    FOREIGN KEY (inviting_instructor_account_id,inviting_instructor_role)
        REFERENCES ple_private.account(account_id,product_role)
);
CREATE TABLE ple_private.course_invitation_event (
    course_invitation_event_id uuid PRIMARY KEY,
    invitation_id uuid NOT NULL UNIQUE REFERENCES ple_private.course_invitation (invitation_id),
    event_kind text NOT NULL CHECK (event_kind IN ('accepted','declined','revoked')),
    performed_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    occurred_at timestamp with time zone NOT NULL,
    reason text NOT NULL CHECK (char_length(btrim(reason)) BETWEEN 1 AND 1000)
);
CREATE TABLE ple_private.course_roster_profile (
    course_roster_profile_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    student_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    roster_id text NOT NULL CHECK (char_length(roster_id) BETWEEN 1 AND 64 AND roster_id ~ '^[A-Za-z0-9._-]+$'),
    roster_name text NOT NULL CHECK (char_length(roster_name) BETWEEN 1 AND 200
        AND roster_name = btrim(roster_name,
            U&'\0009\000A\000B\000C\000D\0020\0085\00A0\1680\2000\2001\2002\2003\2004\2005\2006\2007\2008\2009\200A\2028\2029\202F\205F\3000')
        AND roster_name !~ U&'[\0001-\001F\007F-\009F]'),
    created_at timestamp with time zone NOT NULL,
    UNIQUE(course_id,student_account_id), UNIQUE(course_id,roster_id)
);
CREATE FUNCTION ple_private.reject_course_invitation_change() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,ple_private AS $$ BEGIN RAISE EXCEPTION USING ERRCODE='55000',MESSAGE='Course Invitations are immutable'; END $$;
CREATE FUNCTION ple_private.reject_course_invitation_event_change() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,ple_private AS $$ BEGIN RAISE EXCEPTION USING ERRCODE='55000',MESSAGE='Course Invitation Events are immutable'; END $$;
CREATE FUNCTION ple_private.reject_course_roster_profile_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    -- Column-level grants and immutable identity checks constrain name correction.
    IF TG_OP = 'UPDATE' AND current_user = 'ple_api_owner'
       AND ROW(NEW.course_roster_profile_id, NEW.course_id, NEW.student_account_id,
               NEW.roster_id, NEW.created_at)
           IS NOT DISTINCT FROM ROW(OLD.course_roster_profile_id, OLD.course_id,
               OLD.student_account_id, OLD.roster_id, OLD.created_at) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'a Course Roster Profile identity is immutable';
END $$;
-- ASVS 8.2.3/8.3.1: the normal roster has no email projection.  This private
-- helper is the sole lower-level email read for the distinct, already-pending
-- invitation delivery operation; callers cannot use it for active, revoked,
-- expired, or unrelated Course records.
CREATE FUNCTION ple_private.pending_course_invitation_delivery_email(
    p_course_id uuid, p_student_account_id uuid
) RETURNS text
LANGUAGE sql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
    SELECT email.delivery_email
      FROM ple_private.account_authentication_email AS email
     WHERE email.account_id = $2
       AND EXISTS (
           SELECT 1
             FROM ple_private.course_invitation AS invitation
            WHERE invitation.course_id = $1
              AND invitation.target_account_id = $2
              AND invitation.membership_role = 'student'
              AND invitation.expires_at > pg_catalog.clock_timestamp()
              AND NOT EXISTS (
                  SELECT 1
                    FROM ple_private.course_invitation_event AS event
                   WHERE event.invitation_id = invitation.invitation_id
              )
       )
$$;
CREATE TRIGGER course_invitation_is_immutable BEFORE UPDATE OR DELETE ON ple_private.course_invitation FOR EACH ROW EXECUTE FUNCTION ple_private.reject_course_invitation_change();
CREATE TRIGGER course_invitation_event_is_immutable BEFORE UPDATE OR DELETE ON ple_private.course_invitation_event FOR EACH ROW EXECUTE FUNCTION ple_private.reject_course_invitation_event_change();
CREATE TRIGGER course_roster_profile_is_immutable BEFORE UPDATE OR DELETE ON ple_private.course_roster_profile FOR EACH ROW EXECUTE FUNCTION ple_private.reject_course_roster_profile_change();
CREATE FUNCTION ple_private.assert_course_invitation_event_is_valid() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,ple_private AS $$
DECLARE invite ple_private.course_invitation%ROWTYPE;
BEGIN SELECT * INTO invite FROM ple_private.course_invitation WHERE invitation_id=NEW.invitation_id;
 IF NOT FOUND OR NEW.occurred_at<invite.issued_at OR NEW.occurred_at>=invite.expires_at OR (NEW.event_kind IN ('accepted','declined') AND NEW.performed_by_account_id<>invite.target_account_id) THEN RAISE EXCEPTION USING ERRCODE='23514',MESSAGE='Course Invitation Event is outside its exact transition boundary'; END IF; RETURN NEW; END $$;
CREATE TRIGGER course_invitation_event_has_valid_transition BEFORE INSERT ON ple_private.course_invitation_event FOR EACH ROW EXECUTE FUNCTION ple_private.assert_course_invitation_event_is_valid();
ALTER TABLE ple_private.course_invitation ENABLE ROW LEVEL SECURITY; ALTER TABLE ple_private.course_invitation FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_invitation_event ENABLE ROW LEVEL SECURITY; ALTER TABLE ple_private.course_invitation_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_roster_profile ENABLE ROW LEVEL SECURITY; ALTER TABLE ple_private.course_roster_profile FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.course_invitation,ple_private.course_invitation_event,ple_private.course_roster_profile FROM PUBLIC;
-- Invitation acceptance locks its immutable invitation row so concurrent
-- claims serialize before the unique acceptance event is inserted.  UPDATE is
-- used only for that row lock; the immutability trigger rejects data changes.
GRANT SELECT, INSERT, UPDATE (invitation_id) ON ple_private.course_invitation TO ple_api_owner;
GRANT SELECT, INSERT ON ple_private.course_invitation_event,
    ple_private.course_roster_profile TO ple_api_owner;
GRANT UPDATE (roster_name) ON ple_private.course_roster_profile TO ple_api_owner;
CREATE POLICY course_invitation_api_owner_access ON ple_private.course_invitation FOR ALL TO ple_api_owner USING(true) WITH CHECK(true);
CREATE POLICY course_invitation_event_api_owner_access ON ple_private.course_invitation_event FOR ALL TO ple_api_owner USING(true) WITH CHECK(true);
CREATE POLICY course_roster_profile_api_owner_access ON ple_private.course_roster_profile FOR ALL TO ple_api_owner USING(true) WITH CHECK(true);
REVOKE ALL ON FUNCTION ple_private.reject_course_invitation_change(),ple_private.reject_course_invitation_event_change(),ple_private.reject_course_roster_profile_change(),ple_private.assert_course_invitation_event_is_valid(),ple_private.pending_course_invitation_delivery_email(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.pending_course_invitation_delivery_email(uuid, uuid)
    TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.course_roster_event (
 course_roster_event_id uuid PRIMARY KEY, course_id uuid NOT NULL REFERENCES ple_data.course_instance(course_id),
 student_account_id uuid NOT NULL REFERENCES ple_private.account(account_id), acting_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
 event_kind text NOT NULL CHECK(event_kind IN ('invitation_created','invitation_claimed','student_access_revoked')), occurred_at timestamp with time zone NOT NULL);
CREATE FUNCTION ple_audit.reject_course_roster_event_change() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,ple_audit AS $$ BEGIN RAISE EXCEPTION USING ERRCODE='55000',MESSAGE='a Course Roster Event is immutable'; END $$;
CREATE TRIGGER course_roster_event_is_immutable BEFORE UPDATE OR DELETE ON ple_audit.course_roster_event FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_course_roster_event_change();
ALTER TABLE ple_audit.course_roster_event ENABLE ROW LEVEL SECURITY; ALTER TABLE ple_audit.course_roster_event FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_audit.course_roster_event FROM PUBLIC;
CREATE POLICY course_roster_event_owner_write ON ple_audit.course_roster_event FOR INSERT TO ple_audit_owner WITH CHECK(true);
CREATE POLICY course_roster_event_owner_read ON ple_audit.course_roster_event FOR SELECT TO ple_audit_owner USING(true);
CREATE FUNCTION ple_audit.record_course_roster_event(
    p_course uuid, p_student uuid, p_actor uuid, p_kind text
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    IF p_course IS NULL OR p_student IS NULL OR p_actor IS NULL
       OR p_kind NOT IN ('invitation_created', 'invitation_claimed', 'student_access_revoked') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Roster Event arguments are invalid';
    END IF;
    INSERT INTO ple_audit.course_roster_event
    VALUES (pg_catalog.gen_random_uuid(), p_course, p_student, p_actor, p_kind,
            pg_catalog.transaction_timestamp());
END
$$;
REVOKE ALL ON FUNCTION ple_audit.reject_course_roster_event_change(),
    ple_audit.record_course_roster_event(uuid, uuid, uuid, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_audit.record_course_roster_event(uuid, uuid, uuid, text)
    TO ple_api_owner;
RESET ROLE;
