-- Functions, triggers, and views from question_stewardship.sql.

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.reject_question_stewardship_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Question stewardship evidence is immutable';
END
$$;

CREATE FUNCTION ple_data.validate_question_revision_acceptance()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    published_at timestamptz;
BEGIN
    SELECT revision.published_at INTO published_at
      FROM ple_data.question_revision AS revision
     WHERE revision.question_id = NEW.question_id
       AND revision.revision_number = NEW.revision_number;
    IF published_at IS DISTINCT FROM NEW.accepted_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision acceptance time must match published time';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_private.account AS account
        JOIN LATERAL (SELECT state FROM ple_private.account_state_event
            WHERE account_id = account.account_id ORDER BY occurred_at DESC, event_id DESC LIMIT 1
        ) AS state_event ON state_event.state = 'active'
        WHERE account.account_id = NEW.editor_account_id AND account.product_role = 'instructor')
       OR NOT EXISTS (SELECT 1 FROM ple_private.account AS account
        JOIN LATERAL (SELECT state FROM ple_private.account_state_event
            WHERE account_id = account.account_id ORDER BY occurred_at DESC, event_id DESC LIMIT 1
        ) AS state_event ON state_event.state = 'active'
        WHERE account.account_id = NEW.accepted_by_account_id AND account.product_role = 'instructor') THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision editor and accepter must be Active Instructor Accounts';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_data.validate_question_ownership_event()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    prior ple_data.question_ownership_event%ROWTYPE;
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(NEW.question_id, 0));
    SELECT * INTO prior FROM ple_data.question_ownership_event
     WHERE question_id = NEW.question_id
     ORDER BY occurred_at DESC, question_ownership_event_id DESC LIMIT 1;
    IF NEW.event_kind = 'initial' AND FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Owner already exists';
    END IF;
    IF NEW.event_kind = 'transferred' AND (
        NOT FOUND OR NEW.recorded_by_account_id <> prior.owner_account_id
        OR NEW.occurred_at <= prior.occurred_at
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question ownership transfer requires the current owner and later time';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_private.account AS account
        JOIN LATERAL (SELECT state FROM ple_private.account_state_event
            WHERE account_id = account.account_id ORDER BY occurred_at DESC, event_id DESC LIMIT 1
        ) AS state_event ON state_event.state = 'active'
        WHERE account.account_id = NEW.owner_account_id AND account.product_role = 'instructor') THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Owner must be an Active Instructor Account';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_data.validate_question_publication()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE
    author_count integer;
    final_position integer;
BEGIN
    SELECT count(*), max(author_position) INTO author_count, final_position
      FROM ple_data.question_revision_authorship
     WHERE question_id = NEW.question_id AND revision_number = NEW.revision_number;
    IF author_count = 0 OR author_count <> final_position
       OR NOT EXISTS (SELECT 1 FROM ple_data.question_revision_acceptance
           WHERE question_id = NEW.question_id AND revision_number = NEW.revision_number)
       OR NOT EXISTS (SELECT 1 FROM ple_data.question_revision_license
           WHERE question_id = NEW.question_id AND revision_number = NEW.revision_number)
       OR NOT ple_private.question_revision_has_source_binding(
           NEW.question_id, NEW.revision_number)
       OR NOT EXISTS (SELECT 1 FROM ple_data.question_current_owner
           WHERE question_id = NEW.question_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question publication requires acceptance, contiguous authorship, license, exact source, and owner';
    END IF;
    RETURN NEW;
END
$$;



-- This internal store operation deliberately derives the endorser from the
-- authenticated session. It never accepts an Account identifier, and it does
-- not disclose a count or another Instructor's identity. Repeating the
-- requested state is a no-op so retries cannot create duplicate endorsements.
-- ASVS 8.2.1 and 8.3.1: the database authorization boundary, rather than a
-- browser assertion, permits only the current active Instructor to mutate it.
CREATE FUNCTION ple_data.set_current_question_star(
    p_question_id text,
    p_starred boolean
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_question_id IS NULL OR p_starred IS NULL
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Star requires an active Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.published_question
     WHERE question_id = p_question_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Star requires a Published Question';
    END IF;
    IF p_starred THEN
        INSERT INTO ple_data.question_star(question_id, instructor_account_id, starred_at)
        VALUES (p_question_id, actor_id, pg_catalog.clock_timestamp())
        ON CONFLICT (question_id, instructor_account_id) DO NOTHING;
    ELSE
        DELETE FROM ple_data.question_star
         WHERE question_id = p_question_id AND instructor_account_id = actor_id;
    END IF;
END
$$;



-- This closed endorsement projection establishes every disclosure predicate
-- before it reaches the immutable vetting attribute.  It deliberately has no
-- Account, email, profile, Course, or Watch join in its result.  The caller
-- must be an active Instructor and the target must be a Published Question;
-- an archived or unavailable Published Question remains a Published Question
-- for its lineage-level stewardship facts.
-- ASVS 8.2.1 and 8.3.1: both viewer role and each disclosed endorser's active
-- Instructor status are derived in PostgreSQL, never from browser claims.
CREATE FUNCTION ple_data.read_current_question_star(
    p_question_id text
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
    IF p_question_id IS NULL
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Star requires an active Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.published_question
     WHERE question_id = p_question_id;
    -- A valid-looking Question identifier that is not Published must not
    -- disclose availability through this read surface.  No row becomes the
    -- Store's ordinary NotFound result and the HTTP route conceals it.
    IF NOT FOUND THEN RETURN; END IF;

    RETURN QUERY
    WITH active_endorsers AS (
        SELECT star.instructor_account_id,
               ple_private.verified_instructor_display_name(star.instructor_account_id)
                   AS display_name
          FROM ple_data.question_star AS star
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
         WHERE star.question_id = p_question_id
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
      FROM active_endorsers;
END
$$;



-- This private subscription operation derives the watcher from the
-- authenticated session.  It never accepts an Account identifier and grants
-- no read path for another Instructor's Watch state.  Retried requests are
-- idempotent; C373 may later provide the owning Instructor's projection.
-- ASVS 8.2.1 and 8.3.1: authorization is enforced at the database boundary.
CREATE FUNCTION ple_data.set_current_question_watch(
    p_question_id text,
    p_watched boolean
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_question_id IS NULL OR p_watched IS NULL
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Watch requires an active Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.published_question
     WHERE question_id = p_question_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Watch requires a Published Question';
    END IF;
    IF p_watched THEN
        INSERT INTO ple_data.question_watch(question_id, instructor_account_id, watched_at)
        VALUES (p_question_id, actor_id, pg_catalog.clock_timestamp())
        ON CONFLICT (question_id, instructor_account_id) DO NOTHING;
    ELSE
        DELETE FROM ple_data.question_watch
         WHERE question_id = p_question_id AND instructor_account_id = actor_id;
    END IF;
END
$$;



-- The private read boundary returns only the current Instructor's state for
-- one Published Question.  It deliberately does not enumerate watches or
-- reveal any other watcher, even to another active Instructor.
CREATE FUNCTION ple_data.read_current_question_watch(
    p_question_id text
) RETURNS TABLE (watching boolean) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_question_id IS NULL
       OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Watch requires an active Instructor Account';
    END IF;
    PERFORM 1 FROM ple_data.published_question
     WHERE question_id = p_question_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Watch requires a Published Question';
    END IF;
    RETURN QUERY SELECT EXISTS (
        SELECT 1 FROM ple_data.question_watch
         WHERE question_id = p_question_id AND instructor_account_id = actor_id
    );
END
$$;

CREATE VIEW ple_data.question_current_owner
WITH (security_barrier = true, security_invoker = true) AS
SELECT DISTINCT ON (question_id) question_id, owner_account_id,
    question_ownership_event_id, occurred_at
FROM ple_data.question_ownership_event
ORDER BY question_id, occurred_at DESC, question_ownership_event_id DESC;

CREATE TRIGGER question_revision_acceptance_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision_acceptance
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();

CREATE TRIGGER question_revision_authorship_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision_authorship
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();

CREATE TRIGGER question_revision_license_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision_license
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();

CREATE TRIGGER question_revision_citation_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision_citation
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();

CREATE TRIGGER question_ownership_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_ownership_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();

CREATE TRIGGER question_fork_source_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_fork_source
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();

CREATE TRIGGER question_revision_acceptance_is_valid
BEFORE INSERT ON ple_data.question_revision_acceptance
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_revision_acceptance();

CREATE TRIGGER question_ownership_event_is_valid
BEFORE INSERT ON ple_data.question_ownership_event
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_ownership_event();

CREATE CONSTRAINT TRIGGER question_publication_is_complete
AFTER INSERT ON ple_data.question_publication_event
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
EXECUTE FUNCTION ple_data.validate_question_publication();

SET LOCAL ROLE ple_api_owner;




-- The application principal gets only the closed self-service surface.  The
-- database principals that own the Watch table remain inaccessible to it.
CREATE FUNCTION ple_api.set_current_question_star(
    p_question_id text,
    p_starred boolean
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT ple_data.set_current_question_star($1, $2)
$$;

CREATE FUNCTION ple_api.read_current_question_star(
    p_question_id text
) RETURNS TABLE (
    viewer_has_starred boolean,
    star_count bigint,
    starred_instructor_display_names text[]
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.read_current_question_star($1)
$$;

CREATE FUNCTION ple_api.set_current_question_watch(
    p_question_id text,
    p_watched boolean
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT ple_data.set_current_question_watch($1, $2)
$$;

CREATE FUNCTION ple_api.read_current_question_watch(
    p_question_id text
) RETURNS TABLE (watching boolean) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.read_current_question_watch($1)
$$;

