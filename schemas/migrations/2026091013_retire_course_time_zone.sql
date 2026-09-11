-- Course terms retain inclusive dates; Account preferences are the sole wall-clock authority.
DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091013 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

DROP FUNCTION ple_api.create_live_demo_course_instance(
    uuid, uuid, uuid, uuid, uuid, bigint, bigint, text, date, date, text, bigint
);
DROP FUNCTION ple_api.list_live_demo_course_instances();
DROP FUNCTION ple_api.load_live_demo_course_instance(bigint);
DROP FUNCTION ple_api.read_course_summary(uuid);
DROP FUNCTION ple_api.load_live_demo_assignment_course_term(bigint);

CREATE FUNCTION ple_api.create_live_demo_course_instance(
    p_course_id uuid, p_course_origin_id uuid, p_course_membership_id uuid,
    p_course_schedule_revision_id uuid, p_creation_event_id uuid,
    p_blueprint_course_reference_number bigint, p_blueprint_revision_number bigint,
    p_course_title text, p_term_starts_on date, p_term_ends_on date,
    p_assigned_instructor_reference_number bigint
)
RETURNS TABLE (
    reference_number bigint, title text, term_starts_on date, term_ends_on date,
    creator_is_assigned_instructor boolean
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit
AS $$
DECLARE
    v_creator_account_id uuid;
    v_creator_role text;
    v_assigned_instructor_account_id uuid;
    v_course_reference_number bigint;
    v_occurred_at timestamp with time zone;
BEGIN
    IF p_course_id IS NULL OR p_course_origin_id IS NULL OR p_course_membership_id IS NULL
       OR p_course_schedule_revision_id IS NULL OR p_creation_event_id IS NULL
       OR p_blueprint_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_blueprint_revision_number IS NULL OR p_blueprint_revision_number <= 0
       OR p_course_title IS NULL OR p_course_title <> btrim(p_course_title)
       OR char_length(p_course_title) NOT BETWEEN 1 AND 200
       OR p_term_starts_on IS NULL OR p_term_ends_on IS NULL OR p_term_starts_on > p_term_ends_on
    THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Instance creation arguments are invalid';
    END IF;
    v_creator_account_id := ple_api.current_session_account_id();
    SELECT account.product_role INTO v_creator_role
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
      ) AS current_state ON current_state.state = 'active'
     WHERE account.account_id = v_creator_account_id;
    IF v_creator_role NOT IN ('instructor', 'sysadmin') THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Instance creation requires an active Instructor or Sysadmin Account';
    END IF;
    IF v_creator_role = 'instructor' THEN
        IF p_assigned_instructor_reference_number IS NOT NULL THEN
            SELECT account.account_id INTO v_assigned_instructor_account_id
              FROM ple_private.account AS account
             WHERE account.reference_number = p_assigned_instructor_reference_number;
            IF v_assigned_instructor_account_id IS DISTINCT FROM v_creator_account_id THEN
                RAISE EXCEPTION USING ERRCODE = '42501',
                    MESSAGE = 'an Instructor may create a Course Instance only for self';
            END IF;
        ELSE
            v_assigned_instructor_account_id := v_creator_account_id;
        END IF;
    ELSE
        IF p_assigned_instructor_reference_number NOT BETWEEN 1 AND 2147483647 THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'a Sysadmin must select one active Instructor Account';
        END IF;
        SELECT account.account_id INTO v_assigned_instructor_account_id
          FROM ple_private.account AS account
          JOIN LATERAL (
              SELECT event.state FROM ple_private.account_state_event AS event
               WHERE event.account_id = account.account_id
               ORDER BY event.occurred_at DESC, event.event_id DESC LIMIT 1
          ) AS current_state ON current_state.state = 'active'
         WHERE account.reference_number = p_assigned_instructor_reference_number
           AND account.product_role = 'instructor';
        IF NOT FOUND THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'the selected Assigned Instructor is unavailable';
        END IF;
    END IF;
    PERFORM 1 FROM ple_data.blueprint_course_revision AS revision
      JOIN ple_data.blueprint_publication_event AS publication
        ON publication.blueprint_course_reference_number = revision.blueprint_course_reference_number
       AND publication.blueprint_revision_number = revision.blueprint_revision_number
      JOIN LATERAL (
          SELECT availability.event_kind FROM ple_data.blueprint_revision_availability_event AS availability
           WHERE availability.blueprint_course_reference_number = revision.blueprint_course_reference_number
             AND availability.blueprint_revision_number = revision.blueprint_revision_number
           ORDER BY availability.occurred_at DESC, availability.blueprint_revision_availability_event_id DESC LIMIT 1
      ) AS current_availability ON current_availability.event_kind = 'available'
     WHERE revision.blueprint_course_reference_number = p_blueprint_course_reference_number
       AND revision.blueprint_revision_number = p_blueprint_revision_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Instance source must be one available published Blueprint Revision';
    END IF;
    v_occurred_at := pg_catalog.clock_timestamp();
    INSERT INTO ple_data.course_instance (course_id, blueprint_course_reference_number, blueprint_revision_number,
        assigned_instructor_account_id, course_title, created_at)
    VALUES (p_course_id, p_blueprint_course_reference_number, p_blueprint_revision_number,
        v_assigned_instructor_account_id, p_course_title, v_occurred_at)
    RETURNING course_instance.reference_number INTO v_course_reference_number;
    INSERT INTO ple_data.course_origin (course_origin_id, course_id, blueprint_course_reference_number,
        blueprint_revision_number, source_course_id, created_at, evidence)
    VALUES (p_course_origin_id, p_course_id, p_blueprint_course_reference_number,
        p_blueprint_revision_number, NULL, v_occurred_at, '{}'::jsonb);
    INSERT INTO ple_data.course_schedule_revision (course_schedule_revision_id, course_id, revision_number,
        term_starts_on, term_ends_on, created_at)
    VALUES (p_course_schedule_revision_id, p_course_id, 1, p_term_starts_on, p_term_ends_on, v_occurred_at);
    INSERT INTO ple_data.course_membership (membership_id, course_id, account_id, role, joined_at)
    VALUES (p_course_membership_id, p_course_id, v_assigned_instructor_account_id, 'instructor', v_occurred_at);
    PERFORM ple_audit.record_course_instance_creation_event(p_creation_event_id, p_course_id,
        v_course_reference_number, p_blueprint_course_reference_number, p_blueprint_revision_number,
        v_assigned_instructor_account_id, v_creator_account_id, v_occurred_at);
    RETURN QUERY SELECT v_course_reference_number, p_course_title, p_term_starts_on, p_term_ends_on,
        v_creator_account_id = v_assigned_instructor_account_id;
END
$$;

CREATE FUNCTION ple_api.list_live_demo_course_instances()
RETURNS TABLE (reference_number bigint, title text, term_starts_on date, term_ends_on date, course_theme text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course.reference_number, course.course_title, schedule.term_starts_on, schedule.term_ends_on, course.course_theme
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id() AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_schedule_revision AS schedule ON schedule.course_id = course.course_id AND schedule.revision_number = 1
     WHERE ple_api.current_session_account_is_instructor()
     ORDER BY course.course_title, course.reference_number
$$;

CREATE FUNCTION ple_api.load_live_demo_course_instance(p_reference_number bigint)
RETURNS TABLE (reference_number bigint, title text, term_starts_on date, term_ends_on date, course_theme text,
    is_assigned_instructor boolean, active_instructor_count bigint)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course.reference_number, course.course_title, schedule.term_starts_on, schedule.term_ends_on,
           course.course_theme, course.assigned_instructor_account_id = ple_api.current_session_account_id(),
           (SELECT count(*) FROM ple_data.course_membership AS team_member WHERE team_member.course_id = course.course_id
             AND team_member.role = 'instructor' AND ple_data.course_membership_is_active(team_member.membership_id))
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id() AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_schedule_revision AS schedule ON schedule.course_id = course.course_id AND schedule.revision_number = 1
     WHERE p_reference_number BETWEEN 1 AND 2147483647 AND course.reference_number = p_reference_number
       AND ple_api.current_session_account_is_instructor()
$$;

CREATE FUNCTION ple_api.read_course_summary(p_course_id uuid)
RETURNS TABLE (course_id uuid, reference_number bigint, title text, term_starts_on date, term_ends_on date,
    membership_role text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course.course_id, course.reference_number, course.course_title, schedule.term_starts_on,
           schedule.term_ends_on, membership.role
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_schedule_revision AS schedule ON schedule.course_id = course.course_id AND schedule.revision_number = 1
     WHERE course.course_id = p_course_id AND ple_api.current_session_account_is_course_member(p_course_id)
$$;

REVOKE ALL ON FUNCTION ple_api.create_live_demo_course_instance(
    uuid, uuid, uuid, uuid, uuid, bigint, bigint, text, date, date, bigint
) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.list_live_demo_course_instances() FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.load_live_demo_course_instance(bigint) FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.read_course_summary(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.create_live_demo_course_instance(
    uuid, uuid, uuid, uuid, uuid, bigint, bigint, text, date, date, bigint
) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.list_live_demo_course_instances(),
    ple_api.load_live_demo_course_instance(bigint), ple_api.read_course_summary(uuid) TO ple_app;

SET LOCAL ROLE ple_data_owner;
ALTER TABLE ple_data.course_schedule_revision DROP COLUMN course_time_zone;
RESET ROLE;
