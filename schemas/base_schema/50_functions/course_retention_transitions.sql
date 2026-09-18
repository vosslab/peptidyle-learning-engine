-- Functions, triggers, and views from course_retention_transitions.sql.

SET LOCAL ROLE ple_data_owner;



-- The immutable membership history is itself identifiable Course Student
-- data.  Only the no-login retention procedure can remove its Student rows.
CREATE OR REPLACE FUNCTION ple_data.reject_course_membership_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF TG_OP = 'DELETE' AND current_user = 'ple_course_retention_executor' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Course Membership episodes are immutable';
END $$;

CREATE OR REPLACE FUNCTION ple_data.reject_course_membership_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF TG_OP = 'DELETE' AND current_user = 'ple_course_retention_executor' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Course Membership Events are immutable';
END $$;

SET LOCAL ROLE ple_private_owner;



-- Assessment Attempt is the sole private Student-Work root.  Its dependent
-- evidence follows audited foreign-key cascades; ordinary runtime roles still
-- have no direct delete path.  ASVS 2.3.1/2.3.3.
CREATE OR REPLACE FUNCTION ple_private.reject_student_work_delete()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF current_user NOT IN (
        'ple_unrelease_executor', 'ple_course_retention_executor'
    ) AND pg_catalog.pg_trigger_depth() <= 1 THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Student Work deletion requires its dedicated executor';
    END IF;
    RETURN OLD;
END $$;



-- Roster data contains direct identifiers.  Retention purges Student-targeted
-- invitations, their events, and roster profiles but leaves Accounts intact.
CREATE OR REPLACE FUNCTION ple_private.reject_course_invitation_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF TG_OP = 'DELETE' AND current_user = 'ple_course_retention_executor' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Course Invitations are immutable';
END $$;

CREATE OR REPLACE FUNCTION ple_private.reject_course_invitation_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF TG_OP = 'DELETE' AND current_user = 'ple_course_retention_executor' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Course Invitation Events are immutable';
END $$;

CREATE OR REPLACE FUNCTION ple_private.reject_course_roster_profile_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF TG_OP = 'DELETE' AND current_user = 'ple_course_retention_executor' THEN
        RETURN OLD;
    END IF;
    IF TG_OP = 'UPDATE' AND current_user = 'ple_api_owner'
       AND ROW(NEW.course_roster_profile_id, NEW.course_instance_id, NEW.student_account_id,
               NEW.roster_id, NEW.created_at)
           IS NOT DISTINCT FROM ROW(OLD.course_roster_profile_id, OLD.course_instance_id,
               OLD.student_account_id, OLD.roster_id, OLD.created_at) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'a Course Roster Profile is immutable';
END $$;

SET LOCAL ROLE ple_audit_owner;

CREATE OR REPLACE FUNCTION ple_audit.reject_course_roster_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    IF TG_OP = 'DELETE' AND current_user = 'ple_course_retention_executor' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'a Course Roster Event is immutable';
END $$;

SET LOCAL ROLE ple_course_retention_executor;

CREATE FUNCTION ple_api.mark_course_instance_inactive(
    p_course_instance_id uuid,
    p_evaluated_at timestamp with time zone
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
DECLARE
    course_row ple_data.course_instance%ROWTYPE;
BEGIN
    IF p_course_instance_id IS NULL OR p_evaluated_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course inactivity arguments are invalid';
    END IF;

    -- ASVS 2.3.3/15.4.2: check and change the lifecycle under one row lock.
    SELECT * INTO course_row
      FROM ple_data.course_instance
     WHERE course_instance_id = p_course_instance_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course is unavailable';
    END IF;
    IF course_row.course_lifecycle_state = 'inactive' THEN
        RETURN false;
    END IF;
    -- ASVS 2.3.2: the immutable Active cutoff governs timely and late execution.
    IF p_evaluated_at < course_row.active_until_at THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course inactivity is not due';
    END IF;

    -- ASVS 8.2.3: inactivity changes only these Course lifecycle fields.
    UPDATE ple_data.course_instance
       SET course_lifecycle_state = 'inactive',
           course_became_inactive_at = active_until_at
     WHERE course_instance_id = course_row.course_instance_id;
    RETURN true;
END
$$;

CREATE FUNCTION ple_api.archive_course_student_records(
    p_course_instance_id uuid,
    p_evaluated_at timestamp with time zone
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
DECLARE
    course_row ple_data.course_instance%ROWTYPE;
    archive_due_at timestamp with time zone;
BEGIN
    IF p_course_instance_id IS NULL OR p_evaluated_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Student-record archive arguments are invalid';
    END IF;

    SELECT * INTO course_row
      FROM ple_data.course_instance
     WHERE course_instance_id = p_course_instance_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Student records are unavailable';
    END IF;
    IF course_row.retention_lifecycle_state IN ('archived', 'deleted') THEN
        RETURN false;
    END IF;
    IF course_row.retention_lifecycle_state <> 'active'
       OR course_row.retention_starts_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Course Student-record archive has not started retention';
    END IF;

    SELECT course_row.retention_starts_at + policy.archive_after_retention_start
      INTO archive_due_at
      FROM ple_data.course_retention_policy AS policy
     WHERE policy.policy_key;
    IF archive_due_at IS NULL OR p_evaluated_at < archive_due_at THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Student-record archive is not due';
    END IF;

    UPDATE ple_data.course_instance
       SET retention_lifecycle_state = 'archived',
           student_data_archived_at = p_evaluated_at
     WHERE course_instance_id = course_row.course_instance_id;
    RETURN true;
END
$$;

CREATE FUNCTION ple_api.delete_course_student_records(
    p_course_instance_id uuid,
    p_evaluated_at timestamp with time zone
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_audit, ple_data, ple_private
AS $$
DECLARE
    course_row ple_data.course_instance%ROWTYPE;
    delete_due_at timestamp with time zone;
    students_ever_enrolled bigint;
BEGIN
    IF p_course_instance_id IS NULL OR p_evaluated_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Student-record deletion arguments are invalid';
    END IF;

    SELECT * INTO course_row
      FROM ple_data.course_instance
     WHERE course_instance_id = p_course_instance_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Student records are unavailable';
    END IF;
    IF course_row.retention_lifecycle_state = 'deleted' THEN
        RETURN false;
    END IF;
    IF course_row.retention_lifecycle_state <> 'archived'
       OR course_row.student_data_archived_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Course Student-record deletion requires archive';
    END IF;

    -- ASVS 14.2.4/14.2.7: enforce the configured absolute deletion deadline;
    -- student_data_archived_at remains immutable transition evidence only.
    SELECT course_row.retention_starts_at + policy.archive_after_retention_start
               + policy.delete_after_archive
      INTO delete_due_at
      FROM ple_data.course_retention_policy AS policy
     WHERE policy.policy_key;
    IF delete_due_at IS NULL OR p_evaluated_at < delete_due_at THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Student-record deletion is not due';
    END IF;

    -- Student Work creation locks its Assessment root.  Take the same roots
    -- before deleting so an in-flight start/submit cannot slip a new Assessment Attempt
    -- between this purge's scan and its Student-record removal.
    PERFORM 1
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_instance_id = course_row.course_instance_id
     FOR UPDATE;

    -- ASVS 2.3.3/14.2.4: capture only the existing anonymous Course/Account
    -- enrollment contribution, atomically with destruction of its evidence.
    SELECT count(DISTINCT membership.account_id)
      INTO students_ever_enrolled
      FROM ple_data.course_membership AS membership
     WHERE membership.course_instance_id = course_row.course_instance_id
       AND membership.role = 'student';

    -- Preserve Account, Course, Assessment, Question, configuration, and
    -- existing identity-free aggregate rows.  Do not rebuild statistics after
    -- their identifiable receipts cascade with the private Work roots.
    DELETE FROM ple_private.assessment_attempt AS assessment_attempt
     WHERE assessment_attempt.student_record_id IN (
         SELECT student.student_record_id
           FROM ple_data.student_record AS student
          WHERE student.course_instance_id = course_row.course_instance_id
     );
    DELETE FROM ple_private.student_assessment_accommodation AS accommodation
     WHERE accommodation.student_record_id IN (
         SELECT student.student_record_id
           FROM ple_data.student_record AS student
          WHERE student.course_instance_id = course_row.course_instance_id
     );

    DELETE FROM ple_audit.course_roster_event AS event
     WHERE event.course_instance_id = course_row.course_instance_id;
    DELETE FROM ple_private.course_invitation_event AS event
     USING ple_private.course_invitation AS invitation
     WHERE event.course_invitation_id = invitation.course_invitation_id
       AND invitation.course_instance_id = course_row.course_instance_id
       AND invitation.membership_role = 'student';
    DELETE FROM ple_private.course_invitation AS invitation
     WHERE invitation.course_instance_id = course_row.course_instance_id
       AND invitation.membership_role = 'student';
    DELETE FROM ple_private.course_roster_profile AS profile
     WHERE profile.course_instance_id = course_row.course_instance_id;

    DELETE FROM ple_data.course_membership_event AS event
     USING ple_data.course_membership AS membership
     WHERE event.course_membership_id = membership.course_membership_id
       AND membership.course_instance_id = course_row.course_instance_id
       AND membership.role = 'student';
    DELETE FROM ple_data.course_membership AS membership
     WHERE membership.course_instance_id = course_row.course_instance_id
       AND membership.role = 'student';
    DELETE FROM ple_data.student_record AS student
     WHERE student.course_instance_id = course_row.course_instance_id;

    UPDATE ple_data.course_instance
       SET retention_lifecycle_state = 'deleted',
           student_data_deleted_at = p_evaluated_at,
           purged_students_ever_enrolled = students_ever_enrolled
     WHERE course_instance_id = course_row.course_instance_id;
    RETURN true;
END
$$;

