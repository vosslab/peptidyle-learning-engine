-- Blueprint Course lineage stewardship.
--
-- Stars and Watches deliberately reference the stable Blueprint Course
-- identity, not an immutable Revision. A later Save therefore preserves an
-- Instructor's chosen relationship without copying or rewriting records.

SET LOCAL ROLE ple_data_owner;

-- The stewardship definer functions must validate the stable Blueprint
-- identity and lifecycle without granting the application principal a table
-- read. This is deliberately narrower than C409's future Star projection.
CREATE POLICY blueprint_stewardship_data_read ON ple_data.blueprint_course
    FOR SELECT TO ple_data_owner USING (true);
GRANT SELECT ON ple_data.blueprint_course TO ple_data_owner;
GRANT REFERENCES ON ple_data.blueprint_course TO ple_private_owner;
-- The lifecycle trigger reads only the preceding immutable metadata event to
-- classify the three documented transitions. It has no application caller.
CREATE POLICY blueprint_stewardship_metadata_event_data_read
    ON ple_data.blueprint_metadata_event FOR SELECT TO ple_data_owner USING (true);
GRANT SELECT ON ple_data.blueprint_metadata_event TO ple_data_owner;

CREATE TABLE ple_data.blueprint_course_star (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    instructor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    starred_at timestamptz NOT NULL,
    PRIMARY KEY (blueprint_course_reference_number, instructor_account_id)
);
CREATE INDEX blueprint_course_star_instructor_collection_idx
    ON ple_data.blueprint_course_star (
        instructor_account_id, starred_at DESC, blueprint_course_reference_number
    );

-- A Watch is private: there is deliberately no watcher count, list, or
-- identity projection. C409/C423 own the separately authorized Star
-- projection and the in-app notification projection.
CREATE TABLE ple_data.blueprint_course_watch (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    instructor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    watched_at timestamptz NOT NULL,
    PRIMARY KEY (blueprint_course_reference_number, instructor_account_id)
);
CREATE INDEX blueprint_course_watch_instructor_collection_idx
    ON ple_data.blueprint_course_watch (
        instructor_account_id, watched_at DESC, blueprint_course_reference_number
    );

ALTER TABLE ple_data.blueprint_course_star ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_star FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_watch ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_watch FORCE ROW LEVEL SECURITY;
CREATE POLICY blueprint_course_star_data_owner_access
    ON ple_data.blueprint_course_star FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);
CREATE POLICY blueprint_course_watch_data_owner_access
    ON ple_data.blueprint_course_watch FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);

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

REVOKE ALL ON TABLE ple_data.blueprint_course_star,
    ple_data.blueprint_course_watch FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.set_current_blueprint_course_star(bigint, boolean),
    ple_data.set_current_blueprint_course_watch(bigint, boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.set_current_blueprint_course_star(bigint, boolean),
    ple_data.set_current_blueprint_course_watch(bigint, boolean) TO ple_api_owner;
RESET ROLE;

-- C409's recipient rows are the only Watch-event projection.  They are made
-- at the immutable source-event insertion, rather than by a generic course
-- hook, a fork/adoption path, or a later mutable lookup.  A row names only
-- its recipient and never becomes a Watch directory or aggregate.
SET LOCAL ROLE ple_private_owner;
GRANT SELECT ON ple_private.account, ple_private.account_state_event TO ple_data_owner;
CREATE POLICY account_blueprint_watch_notification_data_read ON ple_private.account
    FOR SELECT TO ple_data_owner USING (true);
CREATE POLICY account_state_blueprint_watch_notification_data_read
    ON ple_private.account_state_event FOR SELECT TO ple_data_owner USING (true);
CREATE TABLE ple_private.blueprint_course_watch_notification (
    notification_id uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
    recipient_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course(reference_number),
    event_kind text NOT NULL CHECK (event_kind IN (
        'revision', 'published', 'archived', 'restored'
    )),
    source_event_id bigint NOT NULL CHECK (source_event_id > 0),
    occurred_at timestamptz NOT NULL,
    UNIQUE (recipient_account_id, event_kind, source_event_id)
);
CREATE INDEX blueprint_course_watch_notification_recipient_idx
    ON ple_private.blueprint_course_watch_notification (
        recipient_account_id, occurred_at DESC, notification_id DESC
    );
ALTER TABLE ple_private.blueprint_course_watch_notification ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.blueprint_course_watch_notification FORCE ROW LEVEL SECURITY;
CREATE POLICY blueprint_course_watch_notification_private_owner_access
    ON ple_private.blueprint_course_watch_notification FOR ALL TO ple_private_owner
    USING (true) WITH CHECK (true);
CREATE POLICY blueprint_course_watch_notification_data_materialization_insert
    ON ple_private.blueprint_course_watch_notification FOR INSERT TO ple_data_owner
    WITH CHECK (true);
CREATE POLICY blueprint_course_watch_notification_data_read
    ON ple_private.blueprint_course_watch_notification FOR SELECT TO ple_data_owner
    USING (true);
GRANT INSERT, SELECT ON ple_private.blueprint_course_watch_notification TO ple_data_owner;
REVOKE ALL ON TABLE ple_private.blueprint_course_watch_notification FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
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
-- PostgreSQL checks trigger-function EXECUTE for the table owner at CREATE
-- TRIGGER time.  This is the sole cross-owner execution grant: it is neither
-- public nor application-principal authority.
GRANT EXECUTE ON FUNCTION ple_data.reject_blueprint_course_watch_notification_change()
    TO ple_private_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
CREATE TRIGGER blueprint_course_watch_notification_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.blueprint_course_watch_notification
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_course_watch_notification_change();
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
REVOKE ALL ON FUNCTION ple_data.fan_out_blueprint_course_watch_notifications(
    bigint, text, bigint, timestamptz),
    ple_data.enqueue_blueprint_course_watch_revision(),
    ple_data.enqueue_blueprint_course_watch_lifecycle_change(),
    ple_data.read_current_blueprint_course_star(bigint),
    ple_data.read_current_blueprint_course_starred_instructors(bigint),
    ple_data.read_current_blueprint_course_watch(bigint),
    ple_data.read_current_blueprint_course_watch_events(bigint, integer),
    ple_data.reject_blueprint_course_watch_notification_change() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.read_current_blueprint_course_star(bigint),
    ple_data.read_current_blueprint_course_starred_instructors(bigint),
    ple_data.read_current_blueprint_course_watch(bigint),
    ple_data.read_current_blueprint_course_watch_events(bigint, integer) TO ple_api_owner;
RESET ROLE;

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
REVOKE ALL ON FUNCTION ple_api.read_current_blueprint_course_star(text),
    ple_api.read_current_blueprint_course_starred_instructors(text),
    ple_api.read_current_blueprint_course_watch(text),
    ple_api.read_current_blueprint_course_watch_events(text, integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_current_blueprint_course_star(text),
    ple_api.read_current_blueprint_course_starred_instructors(text),
    ple_api.read_current_blueprint_course_watch(text),
    ple_api.read_current_blueprint_course_watch_events(text, integer) TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
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
REVOKE ALL ON FUNCTION ple_api.set_current_blueprint_course_star(text, boolean),
    ple_api.set_current_blueprint_course_watch(text, boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.set_current_blueprint_course_star(text, boolean),
    ple_api.set_current_blueprint_course_watch(text, boolean) TO ple_app;
RESET ROLE;
