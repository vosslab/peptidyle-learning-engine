-- Functions, triggers, and views from course_core.sql.

SET LOCAL ROLE ple_data_owner;

CREATE TRIGGER course_instance_human_reference_is_minted
BEFORE INSERT ON ple_data.course_instance
FOR EACH ROW EXECUTE FUNCTION ple_private.assign_human_reference('CI');

CREATE FUNCTION ple_data.enforce_course_instance_retention_schedule()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.purged_students_ever_enrolled IS NOT NULL THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'a Course enrollment count is captured only at Student-data deletion';
        END IF;
        NEW.active_until_at := COALESCE(
            NEW.active_until_at,
            ((NEW.created_at AT TIME ZONE 'UTC') + INTERVAL '6 months') AT TIME ZONE 'UTC'
        );
        NEW.retention_lifecycle_state := 'active';
        NEW.retention_starts_at := COALESCE(
            NEW.latest_assessment_due_at,
            NEW.active_until_at
        );
    ELSIF NEW.created_at IS DISTINCT FROM OLD.created_at
       OR NEW.active_until_at IS DISTINCT FROM OLD.active_until_at THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'a Course Instance creation time and Active lifetime are immutable';
    END IF;
    IF NEW.active_until_at IS DISTINCT FROM (
        ((NEW.created_at AT TIME ZONE 'UTC') + INTERVAL '6 months') AT TIME ZONE 'UTC'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'a Course Instance Active lifetime is six months from creation';
    END IF;
    IF TG_OP = 'UPDATE' THEN
        -- ASVS 8.2.3/14.2.4: only the owning deletion transition may capture
        -- anonymous enrollment evidence; metadata callers cannot rewrite it.
        IF NEW.purged_students_ever_enrolled IS DISTINCT FROM OLD.purged_students_ever_enrolled
           AND NOT (
               current_user = 'ple_course_retention_executor'
               AND OLD.retention_lifecycle_state = 'archived'
               AND NEW.retention_lifecycle_state = 'deleted'
               AND OLD.purged_students_ever_enrolled IS NULL
               AND NEW.purged_students_ever_enrolled IS NOT NULL
           ) THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'a Course purged enrollment count is immutable';
        END IF;
        IF OLD.course_lifecycle_state = 'inactive' AND (
            NEW.course_lifecycle_state IS DISTINCT FROM OLD.course_lifecycle_state
            OR NEW.course_became_inactive_at IS DISTINCT FROM OLD.course_became_inactive_at
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'a Course Instance cannot return from Inactive';
        END IF;
        IF (OLD.retention_lifecycle_state = 'active'
               AND NEW.retention_lifecycle_state NOT IN ('active', 'archived'))
           OR (OLD.retention_lifecycle_state = 'archived'
               AND NEW.retention_lifecycle_state NOT IN ('archived', 'deleted'))
           OR (OLD.retention_lifecycle_state = 'deleted'
               AND NEW.retention_lifecycle_state <> 'deleted') THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'a Course retention lifecycle cannot regress';
        END IF;
        IF OLD.retention_lifecycle_state IN ('archived', 'deleted')
           AND NEW.retention_starts_at IS DISTINCT FROM OLD.retention_starts_at THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'a Course retention start is immutable';
        END IF;
        IF OLD.student_data_archived_at IS NOT NULL
           AND NEW.student_data_archived_at IS DISTINCT FROM OLD.student_data_archived_at THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'a Course Student-data archive time is immutable';
        END IF;
        IF OLD.student_data_deleted_at IS NOT NULL
           AND NEW.student_data_deleted_at IS DISTINCT FROM OLD.student_data_deleted_at THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'a Course Student-data deletion time is immutable';
        END IF;
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER course_instance_retention_schedule_is_consistent
BEFORE INSERT OR UPDATE ON ple_data.course_instance
FOR EACH ROW EXECUTE FUNCTION ple_data.enforce_course_instance_retention_schedule();

CREATE FUNCTION ple_data.reject_course_instance_source_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    IF NEW.source_kind IS DISTINCT FROM OLD.source_kind
       OR NEW.blueprint_course_id
          IS DISTINCT FROM OLD.blueprint_course_id
       OR NEW.blueprint_revision_number IS DISTINCT FROM OLD.blueprint_revision_number THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'a Course Instance source is immutable';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER course_instance_source_is_immutable
BEFORE UPDATE OF source_kind, blueprint_course_id, blueprint_revision_number
ON ple_data.course_instance
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_course_instance_source_change();

CREATE FUNCTION ple_data.reject_course_origin_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'a Course Origin is immutable';
END
$$;

CREATE TRIGGER course_origin_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.course_origin
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_course_origin_change();

SET LOCAL ROLE ple_audit_owner;

CREATE FUNCTION ple_audit.reject_course_instance_creation_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_audit
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'a Course Instance Creation Event is immutable';
END
$$;

CREATE TRIGGER course_instance_creation_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.course_instance_creation_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_course_instance_creation_event_change();

CREATE FUNCTION ple_audit.record_course_instance_creation_event(
    p_event_id uuid, p_course_instance_id text, p_source_kind text,
    p_blueprint_reference text, p_blueprint_revision integer,
    p_assigned_instructor text, p_creator text,
    p_occurred_at timestamp with time zone
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_audit, ple_data
AS $$
BEGIN
    IF p_event_id IS NULL OR p_course_instance_id IS NULL
       OR p_source_kind IS NULL OR p_source_kind NOT IN ('empty', 'adopted')
       OR (p_source_kind = 'empty'
           AND (p_blueprint_reference IS NOT NULL OR p_blueprint_revision IS NOT NULL))
       OR (p_source_kind = 'adopted'
           AND (p_blueprint_reference IS NULL OR p_blueprint_revision IS NULL
                OR p_blueprint_revision <= 0))
       OR p_assigned_instructor IS NULL OR p_creator IS NULL OR p_occurred_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Instance Creation Event arguments are invalid';
    END IF;
    INSERT INTO ple_audit.course_instance_creation_event (
        course_instance_creation_event_id, course_instance_id, source_kind,
        blueprint_course_id, blueprint_revision_number,
        assigned_instructor_account_id, created_by_account_id, occurred_at
    ) VALUES (
        p_event_id, p_course_instance_id, p_source_kind::ple_data.course_source_kind,
        p_blueprint_reference, p_blueprint_revision,
        p_assigned_instructor, p_creator, p_occurred_at
    );
END
$$;

