-- Functions, triggers, and views from question_watch_notifications.sql.

SET LOCAL ROLE ple_data_owner;





-- ASVS 8.2.1/8.3.1: this private snapshot derives recipients only from the
-- current active Instructor Watch relationship in the source transaction.
CREATE FUNCTION ple_data.snapshot_library_watch_event_recipients()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    INSERT INTO ple_data.library_watch_event_recipient(library_watch_event_id, recipient_account_id)
    SELECT NEW.event_id, watch.instructor_account_id
      FROM (
          SELECT question_watch.instructor_account_id
            FROM ple_data.question_watch AS question_watch
           WHERE NEW.target_kind = 'question'
             AND question_watch.published_question_id = NEW.target_public_id
          UNION ALL
          SELECT pool_watch.instructor_account_id
            FROM ple_data.question_pool_watch AS pool_watch
           WHERE NEW.target_kind = 'question_pool'
             AND pool_watch.question_pool_id = NEW.target_public_id
      ) AS watch
      JOIN ple_private.account AS account
        ON account.account_id = watch.instructor_account_id
       AND account.user_role = 'instructor'
      JOIN LATERAL (
          SELECT state FROM ple_private.account_state_event
           WHERE account_id = account.account_id
           ORDER BY occurred_at DESC, event_id DESC LIMIT 1
      ) AS state_event ON state_event.state = 'active'
     WHERE ple_private.instructor_display_name(account.account_id) IS NOT NULL;
    RETURN NEW;
END
$$;

CREATE TRIGGER library_watch_event_snapshots_recipients
AFTER INSERT ON ple_data.library_watch_event
FOR EACH ROW EXECUTE FUNCTION ple_data.snapshot_library_watch_event_recipients();

CREATE FUNCTION ple_data.enqueue_question_watch_revision_event()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    INSERT INTO ple_data.library_watch_event(
        target_kind, target_public_id, event_kind, question_revision_number, occurred_at
    ) VALUES ('question', NEW.published_question_id, 'revision', NEW.revision_number, NEW.occurred_at);
    RETURN NEW;
END
$$;

CREATE TRIGGER question_publication_enqueues_watch_revision
AFTER INSERT ON ple_data.question_publication_event
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_question_watch_revision_event();



-- The ordinary Published Question row is its own fork-parent event. A
-- private Draft fork cannot emit an event before Revision 1 is published.
CREATE FUNCTION ple_data.enqueue_question_watch_fork_event()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NEW.parent_published_question_id IS NULL THEN
        RETURN NEW;
    END IF;
    INSERT INTO ple_data.library_watch_event(
        target_kind, target_public_id, event_kind, question_revision_number,
        forked_public_id, occurred_at
    ) VALUES (
        'question', NEW.parent_published_question_id::text, 'fork', NEW.parent_revision_number,
        NEW.published_question_id::text, NEW.created_at
    );
    RETURN NEW;
END
$$;

CREATE TRIGGER question_fork_enqueues_watch_notification
AFTER INSERT ON ple_data.published_question
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_question_watch_fork_event();

CREATE FUNCTION ple_data.enqueue_question_pool_watch_members_changed_event()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NEW.question_pool_edit_number <= OLD.question_pool_edit_number THEN
        RETURN NEW;
    END IF;
    INSERT INTO ple_data.library_watch_event(
        target_kind, target_public_id, event_kind, question_pool_edit_number, occurred_at
    ) VALUES (
        'question_pool', NEW.question_pool_id, 'members_changed',
        NEW.question_pool_edit_number, clock_timestamp()
    );
    RETURN NEW;
END
$$;

CREATE TRIGGER question_pool_members_changed_enqueues_watch_notification
AFTER UPDATE OF question_pool_edit_number ON ple_data.question_pool
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_question_pool_watch_members_changed_event();

CREATE FUNCTION ple_data.enqueue_question_pool_watch_fork_event()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE source_public_id text;
BEGIN
    IF NEW.source_question_pool_id IS NULL THEN RETURN NEW; END IF;
    SELECT question_pool_id INTO source_public_id
      FROM ple_data.question_pool WHERE question_pool_id = NEW.source_question_pool_id;
    INSERT INTO ple_data.library_watch_event(
        target_kind, target_public_id, event_kind, question_pool_edit_number,
        forked_public_id, occurred_at
    ) VALUES (
        'question_pool', source_public_id, 'fork', (
            SELECT source_pool.question_pool_edit_number
              FROM ple_data.question_pool AS source_pool
             WHERE source_pool.question_pool_id = NEW.source_question_pool_id
        ),
        NEW.question_pool_id, NEW.created_at
    );
    RETURN NEW;
END
$$;

CREATE TRIGGER question_pool_fork_enqueues_watch_notification
AFTER INSERT ON ple_data.question_pool
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_question_pool_watch_fork_event();



-- ASVS 8.2.1/8.3.1: this self-only Inbox derives the Account solely from the
-- installed server session and reveals neither recipient nor Watch facts.
CREATE FUNCTION ple_data.match_current_library_watch_events(
    p_search text, p_target_kinds text[], p_event_kinds text[]
)
RETURNS SETOF ple_data.library_watch_event LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE actor_id text;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF actor_id IS NULL OR NOT ple_api.current_session_account_is_instructor()
       OR p_search IS NULL OR length(p_search) > 120
       OR p_target_kinds IS NULL OR p_event_kinds IS NULL
       OR array_position(p_target_kinds, NULL) IS NOT NULL
       OR array_position(p_event_kinds, NULL) IS NOT NULL
       OR NOT (p_target_kinds <@ ARRAY['question', 'question_pool']::text[])
       OR NOT (p_event_kinds <@ ARRAY['revision', 'members_changed', 'fork']::text[]) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Library Watch inbox requires a valid active Instructor query';
    END IF;
    RETURN QUERY
    SELECT event.*
      FROM ple_data.library_watch_event_recipient AS recipient
      JOIN ple_data.library_watch_event AS event
        ON event.event_id = recipient.library_watch_event_id
     WHERE recipient.recipient_account_id = actor_id
       AND (cardinality(p_target_kinds) = 0 OR event.target_kind::text = ANY(p_target_kinds))
       AND (cardinality(p_event_kinds) = 0 OR event.event_kind::text = ANY(p_event_kinds))
       AND (p_search = '' OR strpos(lower(event.target_public_id), p_search) > 0
            OR strpos(lower(coalesce(event.forked_public_id, '')), p_search) > 0
            OR strpos(lower(CASE event.event_kind::text
                WHEN 'revision' THEN 'New Revision'
                WHEN 'members_changed' THEN 'Questions changed'
                ELSE 'New fork' END), p_search) > 0);
END
$$;

CREATE FUNCTION ple_data.read_current_library_watch_notifications(
    p_search text, p_target_kinds text[], p_event_kinds text[],
    p_ascending boolean, p_after_at timestamptz, p_after_id uuid, p_limit integer
)
RETURNS TABLE(
    event_id uuid, target_kind text, target_public_id text, event_kind text,
    question_revision_number bigint, question_pool_edit_number bigint,
    forked_public_id text, occurred_at_millis bigint, occurred_at_cursor text
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_limit IS NULL OR p_limit NOT BETWEEN 1 AND 251 OR p_ascending IS NULL
       OR (p_after_at IS NULL) != (p_after_id IS NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Library Watch page is invalid';
    END IF;
    RETURN QUERY
    SELECT event.event_id, event.target_kind::text, event.target_public_id, event.event_kind::text,
           event.question_revision_number::bigint, event.question_pool_edit_number,
           event.forked_public_id,
           (EXTRACT(EPOCH FROM event.occurred_at) * 1000)::bigint,
           to_char(event.occurred_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
      FROM ple_data.match_current_library_watch_events(
          p_search, p_target_kinds, p_event_kinds
      ) AS event
     WHERE p_after_at IS NULL
        OR (p_ascending AND (event.occurred_at, event.event_id) > (p_after_at, p_after_id))
        OR (NOT p_ascending AND (event.occurred_at, event.event_id) < (p_after_at, p_after_id))
     ORDER BY CASE WHEN p_ascending THEN event.occurred_at END ASC,
              CASE WHEN p_ascending THEN event.event_id END ASC,
              CASE WHEN NOT p_ascending THEN event.occurred_at END DESC,
              CASE WHEN NOT p_ascending THEN event.event_id END DESC
     LIMIT p_limit;
END
$$;

CREATE FUNCTION ple_data.read_current_library_watch_facets(
    p_search text, p_target_kinds text[], p_event_kinds text[]
)
RETURNS TABLE(facet_field text, facet_value text, facet_count bigint)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT 'target_kind'::text, event.target_kind::text, count(*)
      FROM ple_data.match_current_library_watch_events($1, ARRAY[]::text[], $3) AS event
     GROUP BY event.target_kind
    UNION ALL
    SELECT 'event_kind'::text, event.event_kind::text, count(*)
      FROM ple_data.match_current_library_watch_events($1, $2, ARRAY[]::text[]) AS event
     GROUP BY event.event_kind
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.read_current_library_watch_notifications(
    p_search text, p_target_kinds text[], p_event_kinds text[],
    p_ascending boolean, p_after_at timestamptz, p_after_id uuid, p_limit integer
)
RETURNS TABLE(
    event_id uuid, target_kind text, target_public_id text, event_kind text,
    question_revision_number bigint, question_pool_edit_number bigint,
    forked_public_id text, occurred_at_millis bigint, occurred_at_cursor text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.read_current_library_watch_notifications($1, $2, $3, $4, $5, $6, $7)
$$;

CREATE FUNCTION ple_api.read_current_library_watch_facets(
    p_search text, p_target_kinds text[], p_event_kinds text[]
)
RETURNS TABLE(facet_field text, facet_value text, facet_count bigint)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.read_current_library_watch_facets($1, $2, $3)
$$;
