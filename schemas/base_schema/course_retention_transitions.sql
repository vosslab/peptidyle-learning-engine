-- Executor-owned, one-way Course retention transitions.  The preceding
-- course_retention.sql owns policy/due calculation; this late module owns the
-- exact destructive capability after every Course Student-Work child exists.

SET LOCAL ROLE ple_data_owner;

GRANT USAGE ON SCHEMA ple_data TO ple_course_retention_executor;
GRANT SELECT, UPDATE ON ple_data.course_instance TO ple_course_retention_executor;
GRANT SELECT ON ple_data.course_retention_policy TO ple_course_retention_executor;
GRANT SELECT, UPDATE (assessment_id) ON ple_data.assessment TO ple_course_retention_executor;
GRANT SELECT, DELETE ON ple_data.student_record, ple_data.course_membership,
    ple_data.course_membership_event TO ple_course_retention_executor;

CREATE POLICY course_instance_retention_executor_access
    ON ple_data.course_instance FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);
CREATE POLICY course_retention_policy_executor_read
    ON ple_data.course_retention_policy FOR SELECT TO ple_course_retention_executor
    USING (true);
CREATE POLICY assessment_retention_executor_read
    ON ple_data.assessment FOR SELECT TO ple_course_retention_executor
    USING (true);
CREATE POLICY assessment_retention_executor_student_work_root_lock
    ON ple_data.assessment FOR UPDATE TO ple_course_retention_executor
    USING (true) WITH CHECK (true);
CREATE POLICY student_record_retention_executor_access
    ON ple_data.student_record FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);
CREATE POLICY course_membership_retention_executor_access
    ON ple_data.course_membership FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);
CREATE POLICY course_membership_event_retention_executor_access
    ON ple_data.course_membership_event FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

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

RESET ROLE;

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_course_retention_executor;
GRANT SELECT, DELETE ON ple_private.assessment_attempt,
    ple_private.student_assessment_accommodation,
    ple_private.course_invitation,
    ple_private.course_invitation_event,
    ple_private.course_roster_profile TO ple_course_retention_executor;

CREATE POLICY assessment_attempt_retention_executor_access
    ON ple_private.assessment_attempt FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);
CREATE POLICY student_assessment_accommodation_retention_executor_access
    ON ple_private.student_assessment_accommodation FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);
CREATE POLICY course_invitation_retention_executor_access
    ON ple_private.course_invitation FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);
CREATE POLICY course_invitation_event_retention_executor_access
    ON ple_private.course_invitation_event FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);
CREATE POLICY course_roster_profile_retention_executor_access
    ON ple_private.course_roster_profile FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

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
       AND ROW(NEW.course_roster_profile_id, NEW.course_id, NEW.student_account_id,
               NEW.roster_id, NEW.created_at)
           IS NOT DISTINCT FROM ROW(OLD.course_roster_profile_id, OLD.course_id,
               OLD.student_account_id, OLD.roster_id, OLD.created_at) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'a Course Roster Profile is immutable';
END $$;

RESET ROLE;

SET LOCAL ROLE ple_audit_owner;

GRANT USAGE ON SCHEMA ple_audit TO ple_course_retention_executor;
GRANT SELECT, DELETE ON ple_audit.course_roster_event TO ple_course_retention_executor;
CREATE POLICY course_roster_event_retention_executor_access
    ON ple_audit.course_roster_event FOR ALL TO ple_course_retention_executor
    USING (true) WITH CHECK (true);

CREATE OR REPLACE FUNCTION ple_audit.reject_course_roster_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    IF TG_OP = 'DELETE' AND current_user = 'ple_course_retention_executor' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'a Course Roster Event is immutable';
END $$;

RESET ROLE;

-- The no-login capability owns these procedures.  C215 may grant execution
-- only to its worker profile; no session role receives table privileges.
SET LOCAL ROLE ple_data_owner;
GRANT EXECUTE ON FUNCTION ple_data.course_retention_due_actions(timestamp with time zone)
    TO ple_course_retention_executor;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
GRANT USAGE, CREATE ON SCHEMA ple_api TO ple_course_retention_executor;
RESET ROLE;

SET LOCAL ROLE ple_course_retention_executor;

CREATE FUNCTION ple_api.archive_course_student_records(
    p_course_id uuid,
    p_evaluated_at timestamp with time zone
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
DECLARE
    course_row ple_data.course_instance%ROWTYPE;
    archive_due_at timestamp with time zone;
BEGIN
    IF p_course_id IS NULL OR p_evaluated_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Student-record archive arguments are invalid';
    END IF;

    SELECT * INTO course_row
      FROM ple_data.course_instance
     WHERE course_id = p_course_id
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
     WHERE course_id = course_row.course_id;
    RETURN true;
END
$$;

CREATE FUNCTION ple_api.delete_course_student_records(
    p_course_id uuid,
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
    IF p_course_id IS NULL OR p_evaluated_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Student-record deletion arguments are invalid';
    END IF;

    SELECT * INTO course_row
      FROM ple_data.course_instance
     WHERE course_id = p_course_id
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
     WHERE assessment.course_id = course_row.course_id
     FOR UPDATE;

    -- ASVS 2.3.3/14.2.4: capture only the existing anonymous Course/Account
    -- enrollment contribution, atomically with destruction of its evidence.
    SELECT count(DISTINCT membership.account_id)
      INTO students_ever_enrolled
      FROM ple_data.course_membership AS membership
     WHERE membership.course_id = course_row.course_id
       AND membership.role = 'student';

    -- Preserve Account, Course, Assessment, Question, configuration, and
    -- existing identity-free aggregate rows.  Do not rebuild statistics after
    -- their identifiable receipts cascade with the private Work roots.
    DELETE FROM ple_private.assessment_attempt AS assessment_attempt
     WHERE assessment_attempt.student_record_id IN (
         SELECT student.student_record_id
           FROM ple_data.student_record AS student
          WHERE student.course_id = course_row.course_id
     );
    DELETE FROM ple_private.student_assessment_accommodation AS accommodation
     WHERE accommodation.student_record_id IN (
         SELECT student.student_record_id
           FROM ple_data.student_record AS student
          WHERE student.course_id = course_row.course_id
     );

    DELETE FROM ple_audit.course_roster_event AS event
     WHERE event.course_id = course_row.course_id;
    DELETE FROM ple_private.course_invitation_event AS event
     USING ple_private.course_invitation AS invitation
     WHERE event.invitation_id = invitation.invitation_id
       AND invitation.course_id = course_row.course_id
       AND invitation.membership_role = 'student';
    DELETE FROM ple_private.course_invitation AS invitation
     WHERE invitation.course_id = course_row.course_id
       AND invitation.membership_role = 'student';
    DELETE FROM ple_private.course_roster_profile AS profile
     WHERE profile.course_id = course_row.course_id;

    DELETE FROM ple_data.course_membership_event AS event
     USING ple_data.course_membership AS membership
     WHERE event.membership_id = membership.membership_id
       AND membership.course_id = course_row.course_id
       AND membership.role = 'student';
    DELETE FROM ple_data.course_membership AS membership
     WHERE membership.course_id = course_row.course_id
       AND membership.role = 'student';
    DELETE FROM ple_data.student_record AS student
     WHERE student.course_id = course_row.course_id;

    UPDATE ple_data.course_instance
       SET retention_lifecycle_state = 'deleted',
           student_data_deleted_at = p_evaluated_at,
           purged_students_ever_enrolled = students_ever_enrolled
     WHERE course_id = course_row.course_id;
    RETURN true;
END
$$;

REVOKE ALL ON FUNCTION ple_api.archive_course_student_records(uuid, timestamp with time zone),
    ple_api.delete_course_student_records(uuid, timestamp with time zone) FROM PUBLIC;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_course_retention_executor;
RESET ROLE;
