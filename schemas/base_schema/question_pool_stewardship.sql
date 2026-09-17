-- Stable Question Pool Stars and actor-private Watches, across immutable Revisions.
-- ASVS 8.2.1/8.3.1: no caller-supplied Account or Watch directory capability.
SET LOCAL ROLE ple_data_owner;
CREATE POLICY question_pool_stewardship_data_read ON ple_data.question_pool
    FOR SELECT TO ple_data_owner USING (true);
GRANT SELECT ON ple_data.question_pool TO ple_data_owner;
CREATE TABLE ple_data.question_pool_star (
    public_question_pool_id text NOT NULL REFERENCES ple_data.question_pool(public_question_pool_id),
    instructor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    starred_at timestamptz NOT NULL,
    PRIMARY KEY (public_question_pool_id, instructor_account_id)
);
ALTER TABLE ple_data.question_pool_star ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_pool_star FORCE ROW LEVEL SECURITY;
CREATE POLICY question_pool_star_data_owner_access ON ple_data.question_pool_star
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
REVOKE ALL ON TABLE ple_data.question_pool_star FROM PUBLIC;
CREATE TABLE ple_data.question_pool_watch (
    public_question_pool_id text NOT NULL REFERENCES ple_data.question_pool(public_question_pool_id),
    instructor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    watched_at timestamptz NOT NULL,
    PRIMARY KEY (public_question_pool_id, instructor_account_id)
);
ALTER TABLE ple_data.question_pool_watch ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_pool_watch FORCE ROW LEVEL SECURITY;
CREATE POLICY question_pool_watch_data_owner_access ON ple_data.question_pool_watch
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
REVOKE ALL ON TABLE ple_data.question_pool_watch FROM PUBLIC;
CREATE FUNCTION ple_data.set_current_question_pool_star(
    p_public_question_pool_id text,
    p_starred boolean
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_public_question_pool_id IS NULL OR p_starred IS NULL
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor()
       OR ple_private.verified_instructor_display_name(actor_id) IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool Star requires an active Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.question_pool
     WHERE public_question_pool_id = p_public_question_pool_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool Star requires a published Question Pool';
    END IF;
    IF p_starred THEN
        INSERT INTO ple_data.question_pool_star(public_question_pool_id, instructor_account_id, starred_at)
        VALUES (p_public_question_pool_id, actor_id, pg_catalog.clock_timestamp())
        ON CONFLICT (public_question_pool_id, instructor_account_id) DO NOTHING;
    ELSE
        DELETE FROM ple_data.question_pool_star
         WHERE public_question_pool_id = p_public_question_pool_id AND instructor_account_id = actor_id;
    END IF;
END
$$;

-- This closed endorsement projection establishes every disclosure predicate
-- before it reaches the immutable vetting attribute.  It deliberately has no
-- Account, email, profile, Course, or Watch join in its result.  The caller
-- must be an active Instructor and the target must be a published Question Pool;
-- an archived or unavailable published Question Pool remains a published Question Pool
-- for its lineage-level stewardship facts.
-- ASVS 8.2.1 and 8.3.1: both viewer role and each disclosed endorser's active
-- Instructor status are derived in PostgreSQL, never from browser claims.
CREATE FUNCTION ple_data.read_current_question_pool_star(
    p_public_question_pool_id text
) RETURNS TABLE (
    viewer_has_starred boolean,
    star_count bigint,
    starred_instructor_display_names text[]
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_public_question_pool_id IS NULL
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor()
       OR ple_private.verified_instructor_display_name(actor_id) IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool Star requires an active Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.question_pool
     WHERE public_question_pool_id = p_public_question_pool_id;
    -- A missing public Pool identifier returns the Store's ordinary NotFound.
    -- HTTP concealment belongs to the separately reviewed server slice.
    IF NOT FOUND THEN RETURN; END IF;

    RETURN QUERY
    WITH active_endorsers AS (
        SELECT star.instructor_account_id,
               ple_private.verified_instructor_display_name(star.instructor_account_id)
                   AS display_name
          FROM ple_data.question_pool_star AS star
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
         WHERE star.public_question_pool_id = p_public_question_pool_id
    )
    SELECT EXISTS (
               SELECT 1 FROM active_endorsers
                WHERE instructor_account_id = actor_id
           ),
           count(*)::bigint,
           coalesce(
               array_agg(display_name ORDER BY display_name COLLATE "C")
                   FILTER (WHERE display_name IS NOT NULL),
               ARRAY[]::text[]
           )
      FROM active_endorsers WHERE display_name IS NOT NULL;
END
$$;

-- This private subscription operation derives the watcher from the
-- authenticated session.  It never accepts an Account identifier and grants
-- no read path for another Instructor's Watch state.  Retried requests are
-- idempotent; Only the owning Instructor's projection is exposed.
-- ASVS 8.2.1 and 8.3.1: authorization is enforced at the database boundary.
CREATE FUNCTION ple_data.set_current_question_pool_watch(
    p_public_question_pool_id text,
    p_watched boolean
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_public_question_pool_id IS NULL OR p_watched IS NULL
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor()
       OR ple_private.verified_instructor_display_name(actor_id) IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool Watch requires an active Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.question_pool
     WHERE public_question_pool_id = p_public_question_pool_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool Watch requires a published Question Pool';
    END IF;
    IF p_watched THEN
        INSERT INTO ple_data.question_pool_watch(public_question_pool_id, instructor_account_id, watched_at)
        VALUES (p_public_question_pool_id, actor_id, pg_catalog.clock_timestamp())
        ON CONFLICT (public_question_pool_id, instructor_account_id) DO NOTHING;
    ELSE
        DELETE FROM ple_data.question_pool_watch
         WHERE public_question_pool_id = p_public_question_pool_id AND instructor_account_id = actor_id;
    END IF;
END
$$;

-- The private read boundary returns only the current Instructor's state for
-- one published Question Pool.  It deliberately does not enumerate watches or
-- reveal any other watcher, even to another active Instructor.
CREATE FUNCTION ple_data.read_current_question_pool_watch(
    p_public_question_pool_id text
) RETURNS TABLE (watching boolean) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_public_question_pool_id IS NULL
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor()
       OR ple_private.verified_instructor_display_name(actor_id) IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool Watch requires an active Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.question_pool
     WHERE public_question_pool_id = p_public_question_pool_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Pool Watch requires a published Question Pool';
    END IF;
    RETURN QUERY SELECT EXISTS (
        SELECT 1 FROM ple_data.question_pool_watch
         WHERE public_question_pool_id = p_public_question_pool_id AND instructor_account_id = actor_id
    );
END
$$;

REVOKE ALL ON FUNCTION ple_data.set_current_question_pool_star(text, boolean),
    ple_data.read_current_question_pool_star(text),
    ple_data.set_current_question_pool_watch(text, boolean),
    ple_data.read_current_question_pool_watch(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.set_current_question_pool_star(text, boolean),
    ple_data.read_current_question_pool_star(text),
    ple_data.set_current_question_pool_watch(text, boolean),
    ple_data.read_current_question_pool_watch(text) TO ple_api_owner;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.set_current_question_pool_star(
    p_public_question_pool_id text,
    p_starred boolean
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT ple_data.set_current_question_pool_star($1, $2)
$$;
CREATE FUNCTION ple_api.read_current_question_pool_star(
    p_public_question_pool_id text
) RETURNS TABLE (
    viewer_has_starred boolean,
    star_count bigint,
    starred_instructor_display_names text[]
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT * FROM ple_data.read_current_question_pool_star($1)
$$;
CREATE FUNCTION ple_api.set_current_question_pool_watch(
    p_public_question_pool_id text,
    p_watched boolean
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT ple_data.set_current_question_pool_watch($1, $2)
$$;
CREATE FUNCTION ple_api.read_current_question_pool_watch(
    p_public_question_pool_id text
) RETURNS TABLE (watching boolean) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT * FROM ple_data.read_current_question_pool_watch($1)
$$;
REVOKE ALL ON FUNCTION ple_api.set_current_question_pool_star(text, boolean),
    ple_api.read_current_question_pool_star(text),
    ple_api.set_current_question_pool_watch(text, boolean),
    ple_api.read_current_question_pool_watch(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.set_current_question_pool_star(text, boolean),
    ple_api.read_current_question_pool_star(text),
    ple_api.set_current_question_pool_watch(text, boolean),
    ple_api.read_current_question_pool_watch(text) TO ple_app;
RESET ROLE;
