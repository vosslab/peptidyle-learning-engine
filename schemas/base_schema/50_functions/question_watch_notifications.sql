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
CREATE FUNCTION ple_data.read_current_library_watch_notifications(p_limit integer)
RETURNS TABLE(
    target_kind text, target_public_id text, event_kind text,
    question_revision_number bigint, question_pool_edit_number bigint,
    forked_public_id text, occurred_at_millis bigint
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE actor_id text;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_limit NOT BETWEEN 1 AND 100 OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Library Watch inbox requires an active Instructor Account';
    END IF;
    RETURN QUERY
    SELECT event.target_kind::text, event.target_public_id, event.event_kind::text,
           event.question_revision_number::bigint, event.question_pool_edit_number,
           event.forked_public_id,
           (EXTRACT(EPOCH FROM event.occurred_at) * 1000)::bigint
      FROM ple_data.library_watch_event_recipient AS recipient
      JOIN ple_data.library_watch_event AS event
        ON event.event_id = recipient.library_watch_event_id
     WHERE recipient.recipient_account_id = actor_id
     ORDER BY event.occurred_at DESC, event.event_id DESC
     LIMIT p_limit;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.read_current_library_watch_notifications(p_limit integer)
RETURNS TABLE(
    target_kind text, target_public_id text, event_kind text,
    question_revision_number bigint, question_pool_edit_number bigint,
    forked_public_id text, occurred_at_millis bigint
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.read_current_library_watch_notifications($1)
$$;
