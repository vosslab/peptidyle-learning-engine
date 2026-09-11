-- M8 live Course Instance creation.  One atomic operation binds an exact
-- published Blueprint Revision, immutable Course Origin, initial Course Term,
-- and required Assigned Instructor Course Membership.  It intentionally does
-- not create Student Records, enrollment, Assignments, or delivery state.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.account
    ADD COLUMN reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE
        CHECK (reference_number BETWEEN 1 AND 2147483647);

GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_audit_owner;

RESET ROLE;

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.course_instance
    ADD COLUMN reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE
        CHECK (reference_number BETWEEN 1 AND 2147483647),
    ADD COLUMN course_short_name text NOT NULL
        CHECK (course_short_name = btrim(course_short_name)
            AND char_length(course_short_name) BETWEEN 1 AND 200),
    ADD COLUMN course_long_name text NOT NULL
        CHECK (course_long_name = btrim(course_long_name)
            AND char_length(course_long_name) BETWEEN 1 AND 200);

GRANT USAGE ON SCHEMA ple_data TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance, ple_data.blueprint_course_revision
    TO ple_audit_owner;

GRANT SELECT, INSERT ON TABLE
    ple_data.course_instance,
    ple_data.course_origin,
    ple_data.course_membership,
    ple_data.course_membership_event,
    ple_data.course_schedule_revision
TO ple_api_owner;

CREATE POLICY course_instance_api_owner_live_demo_read
    ON ple_data.course_instance FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_instance_api_owner_live_demo_create
    ON ple_data.course_instance FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY course_origin_api_owner_live_demo_read
    ON ple_data.course_origin FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_origin_api_owner_live_demo_create
    ON ple_data.course_origin FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY course_membership_api_owner_live_demo_create
    ON ple_data.course_membership FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY course_membership_event_api_owner_live_demo_create
    ON ple_data.course_membership_event FOR INSERT TO ple_api_owner WITH CHECK (true);
CREATE POLICY course_schedule_revision_api_owner_live_demo_read
    ON ple_data.course_schedule_revision FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_schedule_revision_api_owner_live_demo_create
    ON ple_data.course_schedule_revision FOR INSERT TO ple_api_owner WITH CHECK (true);

-- The pre-existing deferred Assigned Instructor invariant fires at commit,
-- after the application transaction has returned to `ple_app`.  It therefore
-- runs under its non-login data owner with the narrow read policies required
-- to verify that invariant, never under a browser-selected role.
ALTER FUNCTION ple_data.assert_assigned_instructor_membership() SECURITY DEFINER;
CREATE POLICY course_instance_data_owner_assigned_instructor_check
    ON ple_data.course_instance FOR SELECT TO ple_data_owner USING (true);
CREATE POLICY course_membership_data_owner_assigned_instructor_check
    ON ple_data.course_membership FOR SELECT TO ple_data_owner USING (true);
CREATE POLICY course_membership_event_data_owner_assigned_instructor_check
    ON ple_data.course_membership_event FOR SELECT TO ple_data_owner USING (true);

RESET ROLE;

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.course_instance_creation_event (
    course_instance_creation_event_id uuid PRIMARY KEY,
    course_id uuid NOT NULL UNIQUE REFERENCES ple_data.course_instance (course_id),
    course_reference_number bigint NOT NULL UNIQUE,
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL,
    assigned_instructor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    created_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    occurred_at timestamp with time zone NOT NULL,
    CONSTRAINT course_instance_creation_event_source_is_valid FOREIGN KEY (
        blueprint_course_reference_number, blueprint_revision_number
    ) REFERENCES ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number
    )
);
CREATE FUNCTION ple_audit.reject_course_instance_creation_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514',
        MESSAGE = 'a Course Instance Creation Event is immutable';
END
$$;
CREATE TRIGGER course_instance_creation_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.course_instance_creation_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_course_instance_creation_event_change();
ALTER TABLE ple_audit.course_instance_creation_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.course_instance_creation_event FORCE ROW LEVEL SECURITY;
CREATE FUNCTION ple_audit.record_course_instance_creation_event(
    p_creation_event_id uuid,
    p_course_id uuid,
    p_course_reference_number bigint,
    p_blueprint_course_reference_number bigint,
    p_blueprint_revision_number bigint,
    p_assigned_instructor_account_id uuid,
    p_created_by_account_id uuid,
    p_occurred_at timestamp with time zone
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_audit
AS $$
BEGIN
    IF p_creation_event_id IS NULL OR p_course_id IS NULL
       OR p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_blueprint_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_blueprint_revision_number IS NULL OR p_blueprint_revision_number <= 0
       OR p_assigned_instructor_account_id IS NULL OR p_created_by_account_id IS NULL
       OR p_occurred_at IS NULL
    THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Instance Creation Event arguments are invalid';
    END IF;
    INSERT INTO ple_audit.course_instance_creation_event (
        course_instance_creation_event_id, course_id, course_reference_number,
        blueprint_course_reference_number, blueprint_revision_number,
        assigned_instructor_account_id, created_by_account_id, occurred_at
    ) VALUES (
        p_creation_event_id, p_course_id, p_course_reference_number,
        p_blueprint_course_reference_number, p_blueprint_revision_number,
        p_assigned_instructor_account_id, p_created_by_account_id, p_occurred_at
    );
END
$$;
CREATE POLICY course_instance_creation_event_audit_owner_create
    ON ple_audit.course_instance_creation_event FOR INSERT TO ple_audit_owner WITH CHECK (true);
REVOKE ALL PRIVILEGES ON TABLE ple_audit.course_instance_creation_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_audit.reject_course_instance_creation_event_change()
    FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_audit.record_course_instance_creation_event(
    uuid, uuid, bigint, bigint, bigint, uuid, uuid, timestamp with time zone
) FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_audit TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_audit.record_course_instance_creation_event(
    uuid, uuid, bigint, bigint, bigint, uuid, uuid, timestamp with time zone
) TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.current_session_account_is_sysadmin()
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ple_private.account AS account
          JOIN LATERAL (
              SELECT state_event.state
                FROM ple_private.account_state_event AS state_event
               WHERE state_event.account_id = account.account_id
               ORDER BY state_event.occurred_at DESC, state_event.event_id DESC
               LIMIT 1
          ) AS current_state ON current_state.state = 'active'
         WHERE account.account_id = ple_api.current_session_account_id()
           AND account.product_role = 'sysadmin'
    )
$$;

CREATE FUNCTION ple_api.create_live_demo_course_instance(
    p_course_id uuid,
    p_course_origin_id uuid,
    p_course_membership_id uuid,
    p_course_schedule_revision_id uuid,
    p_creation_event_id uuid,
    p_blueprint_course_reference_number bigint,
    p_blueprint_revision_number bigint,
    p_course_short_name text,
    p_course_long_name text,
    p_term_starts_on date,
    p_term_ends_on date,
    p_course_time_zone text,
    p_assigned_instructor_reference_number bigint
)
RETURNS TABLE (
    reference_number bigint,
    short_name text,
    long_name text,
    term_starts_on date,
    term_ends_on date,
    course_time_zone text,
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
       OR p_course_short_name IS NULL OR p_course_short_name <> btrim(p_course_short_name)
       OR char_length(p_course_short_name) NOT BETWEEN 1 AND 200
       OR p_course_long_name IS NULL OR p_course_long_name <> btrim(p_course_long_name)
       OR char_length(p_course_long_name) NOT BETWEEN 1 AND 200
       OR p_term_starts_on IS NULL OR p_term_ends_on IS NULL OR p_term_starts_on > p_term_ends_on
       OR p_course_time_zone IS NULL OR char_length(btrim(p_course_time_zone)) NOT BETWEEN 1 AND 100
    THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Instance creation arguments are invalid';
    END IF;

    v_creator_account_id := ple_api.current_session_account_id();
    SELECT account.product_role
      INTO v_creator_role
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS current_state ON current_state.state = 'active'
     WHERE account.account_id = v_creator_account_id;
    IF v_creator_role NOT IN ('instructor', 'sysadmin') THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Instance creation requires an active Instructor or Sysadmin Account';
    END IF;

    IF v_creator_role = 'instructor' THEN
        IF p_assigned_instructor_reference_number IS NOT NULL THEN
            SELECT account.account_id
              INTO v_assigned_instructor_account_id
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
        SELECT account.account_id
          INTO v_assigned_instructor_account_id
          FROM ple_private.account AS account
          JOIN LATERAL (
              SELECT event.state
                FROM ple_private.account_state_event AS event
               WHERE event.account_id = account.account_id
               ORDER BY event.occurred_at DESC, event.event_id DESC
               LIMIT 1
          ) AS current_state ON current_state.state = 'active'
         WHERE account.reference_number = p_assigned_instructor_reference_number
           AND account.product_role = 'instructor';
        IF NOT FOUND THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'the selected Assigned Instructor is unavailable';
        END IF;
    END IF;

    PERFORM 1
      FROM ple_data.blueprint_course_revision AS revision
      JOIN ple_data.blueprint_publication_event AS publication
        ON publication.blueprint_course_reference_number = revision.blueprint_course_reference_number
       AND publication.blueprint_revision_number = revision.blueprint_revision_number
      JOIN LATERAL (
          SELECT availability.event_kind
            FROM ple_data.blueprint_revision_availability_event AS availability
           WHERE availability.blueprint_course_reference_number = revision.blueprint_course_reference_number
             AND availability.blueprint_revision_number = revision.blueprint_revision_number
           ORDER BY availability.occurred_at DESC, availability.blueprint_revision_availability_event_id DESC
           LIMIT 1
      ) AS current_availability ON current_availability.event_kind = 'available'
     WHERE revision.blueprint_course_reference_number = p_blueprint_course_reference_number
       AND revision.blueprint_revision_number = p_blueprint_revision_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Instance source must be one available published Blueprint Revision';
    END IF;

    v_occurred_at := pg_catalog.clock_timestamp();
    INSERT INTO ple_data.course_instance (
        course_id, blueprint_course_reference_number, blueprint_revision_number,
        assigned_instructor_account_id, course_short_name, course_long_name, created_at
    ) VALUES (
        p_course_id, p_blueprint_course_reference_number, p_blueprint_revision_number,
        v_assigned_instructor_account_id, p_course_short_name, p_course_long_name, v_occurred_at
    ) RETURNING course_instance.reference_number INTO v_course_reference_number;
    INSERT INTO ple_data.course_origin (
        course_origin_id, course_id, blueprint_course_reference_number,
        blueprint_revision_number, source_course_id, created_at, evidence
    ) VALUES (
        p_course_origin_id, p_course_id, p_blueprint_course_reference_number,
        p_blueprint_revision_number, NULL, v_occurred_at, '{}'::jsonb
    );
    INSERT INTO ple_data.course_schedule_revision (
        course_schedule_revision_id, course_id, revision_number,
        term_starts_on, term_ends_on, course_time_zone, created_at
    ) VALUES (
        p_course_schedule_revision_id, p_course_id, 1,
        p_term_starts_on, p_term_ends_on, p_course_time_zone, v_occurred_at
    );
    INSERT INTO ple_data.course_membership (
        membership_id, course_id, account_id, role, joined_at
    ) VALUES (
        p_course_membership_id, p_course_id, v_assigned_instructor_account_id,
        'instructor', v_occurred_at
    );
    PERFORM ple_audit.record_course_instance_creation_event(
        p_creation_event_id, p_course_id, v_course_reference_number,
        p_blueprint_course_reference_number, p_blueprint_revision_number,
        v_assigned_instructor_account_id, v_creator_account_id, v_occurred_at
    );

    RETURN QUERY SELECT v_course_reference_number, p_course_short_name, p_course_long_name,
        p_term_starts_on, p_term_ends_on, p_course_time_zone,
        v_creator_account_id = v_assigned_instructor_account_id;
END
$$;

CREATE FUNCTION ple_api.list_live_demo_course_instances()
RETURNS TABLE (
    reference_number bigint,
    short_name text,
    long_name text,
    term_starts_on date,
    term_ends_on date,
    course_time_zone text
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course.reference_number, course.course_short_name, course.course_long_name,
           schedule.term_starts_on, schedule.term_ends_on, schedule.course_time_zone
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership
        ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_schedule_revision AS schedule
        ON schedule.course_id = course.course_id
       AND schedule.revision_number = 1
     WHERE ple_api.current_session_account_is_instructor()
     ORDER BY course.course_long_name, course.reference_number
$$;

CREATE FUNCTION ple_api.load_live_demo_course_instance(p_reference_number bigint)
RETURNS TABLE (
    reference_number bigint,
    short_name text,
    long_name text,
    term_starts_on date,
    term_ends_on date,
    course_time_zone text,
    is_assigned_instructor boolean,
    active_instructor_count bigint
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course.reference_number, course.course_short_name, course.course_long_name,
           schedule.term_starts_on, schedule.term_ends_on, schedule.course_time_zone,
           course.assigned_instructor_account_id = ple_api.current_session_account_id(),
           (
               SELECT count(*)
                 FROM ple_data.course_membership AS team_member
                WHERE team_member.course_id = course.course_id
                  AND team_member.role = 'instructor'
                  AND ple_data.course_membership_is_active(team_member.membership_id)
           )
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership
        ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_schedule_revision AS schedule
        ON schedule.course_id = course.course_id
       AND schedule.revision_number = 1
     WHERE p_reference_number BETWEEN 1 AND 2147483647
       AND course.reference_number = p_reference_number
       AND ple_api.current_session_account_is_instructor()
$$;

CREATE FUNCTION ple_api.list_live_demo_course_creation_instructors()
RETURNS TABLE (reference_number bigint)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT account.reference_number
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS current_state ON current_state.state = 'active'
     WHERE ple_api.current_session_account_is_sysadmin()
       AND account.product_role = 'instructor'
     ORDER BY account.reference_number
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_is_sysadmin() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.create_live_demo_course_instance(
    uuid, uuid, uuid, uuid, uuid, bigint, bigint, text, text, date, date, text, bigint
) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_live_demo_course_instances() FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.load_live_demo_course_instance(bigint) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_live_demo_course_creation_instructors() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.create_live_demo_course_instance(
    uuid, uuid, uuid, uuid, uuid, bigint, bigint, text, text, date, date, text, bigint
) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.list_live_demo_course_instances() TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.load_live_demo_course_instance(bigint) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.list_live_demo_course_creation_instructors() TO ple_app;

COMMENT ON FUNCTION ple_api.create_live_demo_course_instance(
    uuid, uuid, uuid, uuid, uuid, bigint, bigint, text, text, date, date, text, bigint
) IS 'Atomically creates one Course Instance from an exact available published Blueprint Revision with immutable Course Origin, Course Term, initial Assigned Instructor Course Membership, and audit event.';

RESET ROLE;

SET LOCAL ROLE ple_audit_owner;

COMMENT ON TABLE ple_audit.course_instance_creation_event IS
    'Immutable evidence of one completed Course Instance Creation; it contains no Student or Assignment delivery state.';

RESET ROLE;
