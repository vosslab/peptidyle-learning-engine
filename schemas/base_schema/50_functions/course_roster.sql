-- Functions, triggers, and views from course_roster.sql.

SET LOCAL ROLE ple_private_owner;

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

SET LOCAL ROLE ple_audit_owner;

CREATE FUNCTION ple_audit.reject_course_roster_event_change() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog,ple_audit AS $$ BEGIN RAISE EXCEPTION USING ERRCODE='55000',MESSAGE='a Course Roster Event is immutable'; END $$;

CREATE TRIGGER course_roster_event_is_immutable BEFORE UPDATE OR DELETE ON ple_audit.course_roster_event FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_course_roster_event_change();

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

