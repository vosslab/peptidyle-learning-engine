-- Instructor reset and restore of Student Course access.

SET LOCAL ROLE ple_api_owner;

-- Invalidates current Student Course access and every open signup invitation,
-- then records one fresh invitation for the same Account. The existing pending
-- invitation export is the signup-code delivery seam.
CREATE FUNCTION ple_api.reset_student_signup_access(
    p_course_instance_id text, p_roster_id text
)
RETURNS TABLE(roster_id text, roster_email text, course_name text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE
    course text; student text; actor text; membership uuid; open_invitation uuid;
    now_at timestamptz; issued_at timestamptz; revoked_invitation boolean := false;
BEGIN
    IF p_course_instance_id IS NULL OR p_roster_id IS NULL
       OR char_length(p_roster_id) NOT BETWEEN 1 AND 64
       OR p_roster_id !~ '^[A-Za-z0-9._-]+$' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Student signup reset arguments are invalid';
    END IF;
    SELECT course_instance_id INTO course FROM ple_data.course_instance
     WHERE course_instance_id = p_course_instance_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student signup reset is unavailable';
    END IF;
    PERFORM 1 FROM ple_data.course_instance WHERE course_instance_id = course FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student signup reset is unavailable';
    END IF;
    SELECT profile.student_account_id INTO student
      FROM ple_private.course_roster_profile AS profile
     WHERE profile.course_instance_id = course AND profile.roster_id = p_roster_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student signup reset is unavailable';
    END IF;
    actor := ple_api.current_session_account_id();
    now_at := pg_catalog.clock_timestamp();
    SELECT course_membership_id INTO membership FROM ple_data.course_membership
     WHERE course_instance_id = course AND account_id = student AND role = 'student'
       AND ple_data.course_membership_is_active(course_membership_id)
     LIMIT 1;
    IF FOUND THEN
        INSERT INTO ple_data.course_membership_event
        VALUES (pg_catalog.gen_random_uuid(), membership, 'ended', now_at,
                'Instructor reset Student signup access');
        PERFORM ple_audit.record_course_roster_event(
            course, student, actor, 'student_access_revoked');
    END IF;
    FOR open_invitation IN
        SELECT invitation.course_invitation_id
          FROM ple_private.course_invitation AS invitation
         WHERE invitation.course_instance_id = course
           AND invitation.target_account_id = student
           AND invitation.membership_role = 'student'
           AND invitation.expires_at > now_at
           AND NOT EXISTS (
               SELECT 1 FROM ple_private.course_invitation_event AS event
                WHERE event.course_invitation_id = invitation.course_invitation_id
           )
         FOR UPDATE
    LOOP
        INSERT INTO ple_private.course_invitation_event
        VALUES (pg_catalog.gen_random_uuid(), open_invitation, 'revoked', actor, now_at,
                'Instructor invalidated prior Student signup access');
        revoked_invitation := true;
    END LOOP;
    IF revoked_invitation AND membership IS NULL THEN
        PERFORM ple_audit.record_course_roster_event(
            course, student, actor, 'student_access_revoked');
    END IF;
    issued_at := pg_catalog.transaction_timestamp();
    INSERT INTO ple_private.course_invitation (
        course_invitation_id, course_instance_id, target_account_id, membership_role,
        inviting_instructor_account_id, inviting_instructor_role, issued_at, expires_at
    ) VALUES (
        pg_catalog.gen_random_uuid(), course, student, 'student', actor, 'instructor',
        issued_at, issued_at + interval '7 days'
    );
    PERFORM ple_audit.record_course_roster_event(course, student, actor, 'invitation_created');
    roster_id := p_roster_id;
    roster_email := ple_private.pending_course_invitation_delivery_email(course, student);
    SELECT instance.course_long_name INTO course_name
      FROM ple_data.course_instance AS instance
     WHERE instance.course_instance_id = course;
    IF roster_email IS NULL OR course_name IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student signup reset is unavailable';
    END IF;
    RETURN NEXT;
END
$$;

-- Reopens Course access on the existing Student Record. It does not create an
-- Account and does not delete Student Work.
CREATE FUNCTION ple_api.restore_student_course_access(
    p_membership uuid, p_course_instance_id text, p_roster_id text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE
    course text; student text; actor text; existing_record uuid; open_invitation uuid;
    now_at timestamptz;
BEGIN
    IF p_membership IS NULL OR p_course_instance_id IS NULL OR p_roster_id IS NULL
       OR char_length(p_roster_id) NOT BETWEEN 1 AND 64
       OR p_roster_id !~ '^[A-Za-z0-9._-]+$' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Student Course access restore arguments are invalid';
    END IF;
    SELECT course_instance_id INTO course FROM ple_data.course_instance
     WHERE course_instance_id = p_course_instance_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student Course access restore is unavailable';
    END IF;
    PERFORM 1 FROM ple_data.course_instance WHERE course_instance_id = course FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student Course access restore is unavailable';
    END IF;
    SELECT profile.student_account_id INTO student
      FROM ple_private.course_roster_profile AS profile
     WHERE profile.course_instance_id = course AND profile.roster_id = p_roster_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student Course access restore is unavailable';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_data.course_membership AS membership
         WHERE membership.course_instance_id = course AND membership.account_id = student
           AND membership.role = 'student'
           AND ple_data.course_membership_is_active(membership.course_membership_id)
    ) THEN
        RETURN;
    END IF;
    SELECT record.student_record_id INTO existing_record
      FROM ple_data.student_record AS record
     WHERE record.course_instance_id = course AND record.student_account_id = student;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student Course access restore is unavailable';
    END IF;
    actor := ple_api.current_session_account_id();
    now_at := pg_catalog.clock_timestamp();
    FOR open_invitation IN
        SELECT invitation.course_invitation_id
          FROM ple_private.course_invitation AS invitation
         WHERE invitation.course_instance_id = course
           AND invitation.target_account_id = student
           AND invitation.membership_role = 'student'
           AND invitation.expires_at > now_at
           AND NOT EXISTS (
               SELECT 1 FROM ple_private.course_invitation_event AS event
                WHERE event.course_invitation_id = invitation.course_invitation_id
           )
         FOR UPDATE
    LOOP
        INSERT INTO ple_private.course_invitation_event
        VALUES (pg_catalog.gen_random_uuid(), open_invitation, 'revoked', actor, now_at,
                'Instructor restored Student Course access');
    END LOOP;
    INSERT INTO ple_data.course_membership
    VALUES (p_membership, course, student, 'student', existing_record, now_at);
    PERFORM ple_audit.record_course_roster_event(course, student, actor, 'student_access_restored');
END
$$;
