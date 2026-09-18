-- Functions, triggers, and views from blueprint_stewardship.sql.

SET LOCAL ROLE ple_data_owner;



-- ASVS 8.2.1 and 8.3.1: this definer boundary derives the actor from the
-- attested session and allows only an active Instructor to set their own
-- relationship with a Public or Archived Blueprint Course.
CREATE FUNCTION ple_data.set_current_blueprint_course_star(
    p_reference_number bigint,
    p_starred boolean
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_starred IS NULL
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Blueprint Course Star requires an active Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.blueprint_course
     WHERE reference_number = p_reference_number
       AND availability IN ('public', 'archived');
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Blueprint Course Star requires a Public or Archived Blueprint Course';
    END IF;
    IF p_starred THEN
        INSERT INTO ple_data.blueprint_course_star (
            blueprint_course_reference_number, instructor_account_id, starred_at
        ) VALUES (p_reference_number, actor_id, pg_catalog.clock_timestamp())
        ON CONFLICT (blueprint_course_reference_number, instructor_account_id) DO NOTHING;
    ELSE
        DELETE FROM ple_data.blueprint_course_star
         WHERE blueprint_course_reference_number = p_reference_number
           AND instructor_account_id = actor_id;
    END IF;
END
$$;



-- ASVS 8.2.1 and 8.3.1: private subscription changes are self-only and
-- idempotent. No Account identifier or role claim is accepted from callers.
CREATE FUNCTION ple_data.set_current_blueprint_course_watch(
    p_reference_number bigint,
    p_watching boolean
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_watching IS NULL
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Blueprint Course Watch requires an active Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.blueprint_course
     WHERE reference_number = p_reference_number
       AND availability IN ('public', 'archived');
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Blueprint Course Watch requires a Public or Archived Blueprint Course';
    END IF;
    IF p_watching THEN
        INSERT INTO ple_data.blueprint_course_watch (
            blueprint_course_reference_number, instructor_account_id, watched_at
        ) VALUES (p_reference_number, actor_id, pg_catalog.clock_timestamp())
        ON CONFLICT (blueprint_course_reference_number, instructor_account_id) DO NOTHING;
    ELSE
        DELETE FROM ple_data.blueprint_course_watch
         WHERE blueprint_course_reference_number = p_reference_number
           AND instructor_account_id = actor_id;
    END IF;
END
$$;





-- ASVS 8.2.1 and 8.3.1: the actor and recipients are derived in PostgreSQL;
-- this is never an API accepting an Account identifier or client role claim.
CREATE FUNCTION ple_data.fan_out_blueprint_course_watch_notifications(
    p_reference_number bigint,
    p_event_kind text,
    p_source_event_id bigint,
    p_occurred_at timestamptz
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF p_reference_number IS NULL
       OR p_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_event_kind IS NULL
       OR p_event_kind NOT IN ('revision', 'published', 'archived', 'restored')
       OR p_source_event_id IS NULL OR p_source_event_id <= 0 OR p_occurred_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint Course Watch event is invalid';
    END IF;
    INSERT INTO ple_private.blueprint_course_watch_notification (
        recipient_account_id, blueprint_course_reference_number, event_kind,
        source_event_id, occurred_at
    )
    SELECT watch.instructor_account_id, p_reference_number, p_event_kind,
           p_source_event_id, p_occurred_at
      FROM ple_data.blueprint_course_watch AS watch
      JOIN ple_private.account AS account
        ON account.account_id = watch.instructor_account_id
      JOIN LATERAL (
          SELECT state FROM ple_private.account_state_event
           WHERE account_id = account.account_id
           ORDER BY occurred_at DESC, event_id DESC LIMIT 1
      ) AS state_event ON state_event.state = 'active'
     WHERE watch.blueprint_course_reference_number = p_reference_number
       AND account.product_role = 'instructor'
    ON CONFLICT (recipient_account_id, event_kind, source_event_id) DO NOTHING;
END
$$;

CREATE FUNCTION ple_data.enqueue_blueprint_course_watch_revision()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    PERFORM ple_data.fan_out_blueprint_course_watch_notifications(
        NEW.blueprint_course_reference_number, 'revision',
        NEW.blueprint_revision_event_id, NEW.occurred_at
    );
    RETURN NEW;
END
$$;

CREATE TRIGGER blueprint_revision_enqueues_watch_notifications
AFTER INSERT ON ple_data.blueprint_revision_event
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_blueprint_course_watch_revision();

CREATE FUNCTION ple_data.enqueue_blueprint_course_watch_lifecycle_change()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE previous_availability text; event_kind text;
BEGIN
    SELECT availability INTO previous_availability
      FROM ple_data.blueprint_metadata_event
     WHERE blueprint_course_reference_number = NEW.blueprint_course_reference_number
       AND blueprint_metadata_event_id < NEW.blueprint_metadata_event_id
     ORDER BY blueprint_metadata_event_id DESC LIMIT 1;
    event_kind := CASE
        WHEN previous_availability = 'private' AND NEW.availability = 'public' THEN 'published'
        WHEN previous_availability = 'public' AND NEW.availability = 'archived' THEN 'archived'
        WHEN previous_availability = 'archived' AND NEW.availability = 'public' THEN 'restored'
        ELSE NULL
    END;
    IF event_kind IS NOT NULL THEN
        PERFORM ple_data.fan_out_blueprint_course_watch_notifications(
            NEW.blueprint_course_reference_number, event_kind,
            NEW.blueprint_metadata_event_id, NEW.occurred_at
        );
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER blueprint_lifecycle_enqueues_watch_notifications
AFTER INSERT ON ple_data.blueprint_metadata_event
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_blueprint_course_watch_lifecycle_change();



-- ASVS 1.2.4: this fixed query accepts only a typed Blueprint reference and
-- derives both the viewer and the current active-Instructor endorser set from
-- trusted database state; it constructs no dynamic SQL.
CREATE FUNCTION ple_data.read_current_blueprint_course_star(
    p_reference_number bigint
) RETURNS TABLE (viewer_has_starred boolean, star_count bigint)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_reference_number IS NULL
       OR p_reference_number NOT BETWEEN 1 AND 2147483647
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Blueprint Course Star requires an active Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.blueprint_course
     WHERE reference_number = p_reference_number
       AND availability IN ('public', 'archived');
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Blueprint Course Star requires a Public or Archived Blueprint Course';
    END IF;
    RETURN QUERY
    WITH active_endorsers AS (
        SELECT star.instructor_account_id
          FROM ple_data.blueprint_course_star AS star
          JOIN ple_private.account AS account
            ON account.account_id = star.instructor_account_id
           AND account.product_role = 'instructor'
          JOIN LATERAL (
              SELECT event.state
                FROM ple_private.account_state_event AS event
               WHERE event.account_id = account.account_id
               ORDER BY event.occurred_at DESC, event.event_id DESC
               LIMIT 1
          ) AS state_event ON state_event.state = 'active'
         WHERE star.blueprint_course_reference_number = p_reference_number
    )
    SELECT EXISTS (
               SELECT 1 FROM active_endorsers
                WHERE instructor_account_id = actor_id
           ),
           count(*)::bigint
      FROM active_endorsers;
END
$$;



-- This is a distinct, closed identity projection rather than an expansion of
-- C409's Star aggregate. It establishes the viewer and Blueprint visibility
-- predicates before consulting immutable vetting evidence, and returns only
-- a display name for each currently active Instructor endorser. In particular
-- it contains no Account/email/avatar/Course/substitute/Profile/Watch join.
-- ASVS 8.2.1 and 8.3.1: viewer and disclosed endorser authorization derive
-- only from PostgreSQL state, never from browser role or identity claims.
CREATE FUNCTION ple_data.read_current_blueprint_course_starred_instructors(
    p_reference_number bigint
) RETURNS TABLE (display_name text)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_reference_number IS NULL
       OR p_reference_number NOT BETWEEN 1 AND 2147483647
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Blueprint Course Star identities require an active Instructor Account';
    END IF;
    IF ple_private.verified_instructor_display_name(actor_id) IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Blueprint Course Star identities require a vetted Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.blueprint_course
     WHERE reference_number = p_reference_number
       AND availability IN ('public', 'archived');
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Blueprint Course Star identities require a Public or Archived Blueprint Course';
    END IF;

    RETURN QUERY
    SELECT ple_private.verified_instructor_display_name(star.instructor_account_id)
      FROM ple_data.blueprint_course_star AS star
      JOIN ple_private.account AS account
        ON account.account_id = star.instructor_account_id
       AND account.product_role = 'instructor'
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS state_event ON state_event.state = 'active'
     WHERE star.blueprint_course_reference_number = p_reference_number
       AND ple_private.verified_instructor_display_name(star.instructor_account_id) IS NOT NULL
     ORDER BY ple_private.verified_instructor_display_name(star.instructor_account_id) COLLATE "C";
END
$$;

CREATE FUNCTION ple_data.read_current_blueprint_course_watch(
    p_reference_number bigint
) RETURNS TABLE (watching boolean)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_reference_number IS NULL
       OR p_reference_number NOT BETWEEN 1 AND 2147483647
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Blueprint Course Watch requires an active Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.blueprint_course
     WHERE reference_number = p_reference_number
       AND availability IN ('public', 'archived');
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Blueprint Course Watch requires a Public or Archived Blueprint Course';
    END IF;
    RETURN QUERY SELECT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course_watch
         WHERE blueprint_course_reference_number = p_reference_number
           AND instructor_account_id = actor_id
    );
END
$$;

CREATE FUNCTION ple_data.read_current_blueprint_course_watch_events(
    p_reference_number bigint, p_limit integer
) RETURNS TABLE (event_kind text, occurred_at_millis bigint)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_reference_number IS NULL
       OR p_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_limit IS NULL OR p_limit NOT BETWEEN 1 AND 100
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Blueprint Course Watch events require an active Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.blueprint_course
     WHERE reference_number = p_reference_number
       AND availability IN ('public', 'archived');
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Blueprint Course Watch events require a Public or Archived Blueprint Course';
    END IF;
    RETURN QUERY SELECT notification.event_kind,
        (EXTRACT(EPOCH FROM notification.occurred_at) * 1000)::bigint
      FROM ple_private.blueprint_course_watch_notification AS notification
     WHERE notification.recipient_account_id = actor_id
       AND notification.blueprint_course_reference_number = p_reference_number
     ORDER BY notification.occurred_at DESC, notification.notification_id DESC
     LIMIT p_limit;
END
$$;

CREATE FUNCTION ple_data.reject_blueprint_course_watch_notification_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Blueprint Course Watch notification is immutable';
END
$$;

SET LOCAL ROLE ple_private_owner;

CREATE TRIGGER blueprint_course_watch_notification_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.blueprint_course_watch_notification
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_course_watch_notification_change();

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.read_current_blueprint_course_star(
    p_reference text
) RETURNS TABLE (viewer_has_starred boolean, star_count bigint)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.read_current_blueprint_course_star(
        (SELECT reference_number FROM ple_data.blueprint_course WHERE public_reference = $1)
    )
$$;

CREATE FUNCTION ple_api.read_current_blueprint_course_starred_instructors(
    p_reference text
) RETURNS TABLE (display_name text)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.read_current_blueprint_course_starred_instructors(
        (SELECT reference_number FROM ple_data.blueprint_course WHERE public_reference = $1)
    )
$$;

CREATE FUNCTION ple_api.read_current_blueprint_course_watch(
    p_reference text
) RETURNS TABLE (watching boolean)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.read_current_blueprint_course_watch(
        (SELECT reference_number FROM ple_data.blueprint_course WHERE public_reference = $1)
    )
$$;

CREATE FUNCTION ple_api.read_current_blueprint_course_watch_events(
    p_reference text, p_limit integer
) RETURNS TABLE (event_kind text, occurred_at_millis bigint)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.read_current_blueprint_course_watch_events(
        (SELECT reference_number FROM ple_data.blueprint_course WHERE public_reference = $1), $2
    )
$$;

CREATE FUNCTION ple_api.set_current_blueprint_course_star(
    p_reference text, p_starred boolean
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT ple_data.set_current_blueprint_course_star(
        (SELECT reference_number FROM ple_data.blueprint_course WHERE public_reference = $1), $2
    )
$$;

CREATE FUNCTION ple_api.set_current_blueprint_course_watch(
    p_reference text, p_watching boolean
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT ple_data.set_current_blueprint_course_watch(
        (SELECT reference_number FROM ple_data.blueprint_course WHERE public_reference = $1), $2
    )
$$;

