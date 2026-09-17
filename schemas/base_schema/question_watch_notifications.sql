-- Private in-app Watch notifications for Question Library Objects.
-- This dedicated Watch outbox has exactly two target kinds and the four Human
-- Guidance event kinds. It is not a generic event bus and never delivers email.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.library_watch_event (
    event_id uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
    target_kind text NOT NULL CHECK (target_kind IN ('question', 'question_pool')),
    target_public_id text NOT NULL CHECK (
        target_public_id ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
        AND substr(target_public_id, 6, 1) = ple_private.crockford_checksum_character(
            substr(target_public_id, 1, 4) || substr(target_public_id, 7, 3)
        )
    ),
    event_kind text NOT NULL CHECK (event_kind IN (
        'revision', 'fork', 'improvement_thread', 'impact_notice'
    )),
    revision_number bigint CHECK (revision_number > 0),
    forked_public_id text CHECK (forked_public_id IS NULL
        OR (forked_public_id ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
            AND substr(forked_public_id, 6, 1) = ple_private.crockford_checksum_character(
                substr(forked_public_id, 1, 4) || substr(forked_public_id, 7, 3)
            ))),
    activity_id uuid,
    occurred_at timestamptz NOT NULL,
    processed_at timestamptz,
    CHECK (
        (event_kind = 'revision' AND revision_number IS NOT NULL
            AND forked_public_id IS NULL AND activity_id IS NULL)
        OR (event_kind = 'fork' AND revision_number IS NOT NULL
            AND forked_public_id IS NOT NULL AND activity_id IS NULL)
        OR (event_kind = 'improvement_thread' AND revision_number IS NOT NULL
            AND forked_public_id IS NULL AND activity_id IS NOT NULL)
        OR (event_kind = 'impact_notice' AND forked_public_id IS NULL
            AND activity_id IS NOT NULL)
    )
);
CREATE INDEX library_watch_event_pending_idx
    ON ple_data.library_watch_event(occurred_at, event_id) WHERE processed_at IS NULL;
ALTER TABLE ple_data.library_watch_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.library_watch_event FORCE ROW LEVEL SECURITY;
CREATE POLICY library_watch_event_data_owner_access
    ON ple_data.library_watch_event FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);
REVOKE ALL ON TABLE ple_data.library_watch_event FROM PUBLIC;
-- The private notification projection has a foreign key to the immutable
-- outbox row, so grant this only after its table exists.
GRANT REFERENCES ON ple_data.library_watch_event TO ple_private_owner;

-- Recipient snapshots preserve who was actively subscribed when the source
-- event INSERT statement ran in its transaction. That transaction commits or
-- rolls back the event and frozen snapshot together; a later Watch/unwatch
-- cannot rewrite that event.
CREATE TABLE ple_data.library_watch_event_recipient (
    event_id uuid NOT NULL REFERENCES ple_data.library_watch_event(event_id),
    recipient_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    PRIMARY KEY (event_id, recipient_account_id)
);
ALTER TABLE ple_data.library_watch_event_recipient ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.library_watch_event_recipient FORCE ROW LEVEL SECURITY;
CREATE POLICY library_watch_event_recipient_data_owner_access
    ON ple_data.library_watch_event_recipient FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);
REVOKE ALL ON TABLE ple_data.library_watch_event_recipient FROM PUBLIC;

SET LOCAL ROLE ple_api_owner;
GRANT USAGE, CREATE ON SCHEMA ple_api TO ple_data_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.library_watch_notification (
    notification_id uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
    recipient_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    event_id uuid NOT NULL REFERENCES ple_data.library_watch_event(event_id),
    target_kind text NOT NULL CHECK (target_kind IN ('question', 'question_pool')),
    target_public_id text NOT NULL CHECK (
        target_public_id ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
        AND substr(target_public_id, 6, 1) = ple_private.crockford_checksum_character(
            substr(target_public_id, 1, 4) || substr(target_public_id, 7, 3)
        )
    ),
    event_kind text NOT NULL CHECK (event_kind IN (
        'revision', 'fork', 'improvement_thread', 'impact_notice'
    )),
    revision_number bigint CHECK (revision_number > 0),
    forked_public_id text CHECK (forked_public_id IS NULL
        OR (forked_public_id ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
            AND substr(forked_public_id, 6, 1) = ple_private.crockford_checksum_character(
                substr(forked_public_id, 1, 4) || substr(forked_public_id, 7, 3)
            ))),
    activity_id uuid,
    occurred_at timestamptz NOT NULL,
    UNIQUE (recipient_account_id, event_id),
    CHECK (
        (event_kind = 'revision' AND revision_number IS NOT NULL
            AND forked_public_id IS NULL AND activity_id IS NULL)
        OR (event_kind = 'fork' AND revision_number IS NOT NULL
            AND forked_public_id IS NOT NULL AND activity_id IS NULL)
        OR (event_kind = 'improvement_thread' AND revision_number IS NOT NULL
            AND forked_public_id IS NULL AND activity_id IS NOT NULL)
        OR (event_kind = 'impact_notice' AND forked_public_id IS NULL
            AND activity_id IS NOT NULL)
    )
);
CREATE INDEX library_watch_notification_recipient_idx
    ON ple_private.library_watch_notification(
        recipient_account_id, occurred_at DESC, notification_id DESC
    );
ALTER TABLE ple_private.library_watch_notification ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.library_watch_notification FORCE ROW LEVEL SECURITY;
CREATE POLICY library_watch_notification_private_owner_access
    ON ple_private.library_watch_notification FOR ALL TO ple_private_owner
    USING (true) WITH CHECK (true);
CREATE POLICY library_watch_notification_data_materialization_insert
    ON ple_private.library_watch_notification FOR INSERT TO ple_data_owner
    WITH CHECK (true);
CREATE POLICY library_watch_notification_data_materialization_read
    ON ple_private.library_watch_notification FOR SELECT TO ple_data_owner USING (true);
GRANT INSERT, SELECT ON ple_private.library_watch_notification TO ple_data_owner;
REVOKE ALL ON TABLE ple_private.library_watch_notification FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
-- ASVS 2.3.3/15.4.2: PostgreSQL atomically claims, fans out, and completes
-- each event. SKIP LOCKED prevents competing worker iterations from overlap.
CREATE FUNCTION ple_api.materialize_library_watch_notifications(p_limit integer)
RETURNS integer LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE materialized integer;
BEGIN
    IF p_limit NOT BETWEEN 1 AND 500 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Library Watch notification limit is invalid';
    END IF;
    WITH candidate AS (
        SELECT event_id, target_kind, target_public_id, event_kind,
               revision_number, forked_public_id, activity_id, occurred_at
          FROM ple_data.library_watch_event
         WHERE processed_at IS NULL
         ORDER BY occurred_at, event_id
         FOR UPDATE SKIP LOCKED LIMIT p_limit
    ), recipients AS (
        SELECT candidate.*, snapshot.recipient_account_id
          FROM candidate
          JOIN ple_data.library_watch_event_recipient AS snapshot
            ON snapshot.event_id = candidate.event_id
    ), inserted AS (
        INSERT INTO ple_private.library_watch_notification(
            recipient_account_id, event_id, target_kind, target_public_id, event_kind,
            revision_number, forked_public_id, activity_id, occurred_at
        )
        SELECT recipient_account_id, event_id, target_kind, target_public_id, event_kind,
               revision_number, forked_public_id, activity_id, occurred_at
          FROM recipients
        ON CONFLICT (recipient_account_id, event_id) DO NOTHING
        RETURNING notification_id
    ), completed AS (
        UPDATE ple_data.library_watch_event AS event
           SET processed_at = pg_catalog.clock_timestamp()
          FROM candidate WHERE event.event_id = candidate.event_id
    )
    SELECT count(*) INTO materialized FROM inserted;
    RETURN materialized;
END
$$;

-- ASVS 8.2.1/8.3.1: this private snapshot derives recipients only from the
-- current active Instructor Watch relationship in the source transaction.
CREATE FUNCTION ple_data.snapshot_library_watch_event_recipients()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    INSERT INTO ple_data.library_watch_event_recipient(event_id, recipient_account_id)
    SELECT NEW.event_id, watch.instructor_account_id
      FROM (
          SELECT question_watch.instructor_account_id
            FROM ple_data.question_watch AS question_watch
           WHERE NEW.target_kind = 'question'
             AND question_watch.question_id = NEW.target_public_id
          UNION ALL
          SELECT pool_watch.instructor_account_id
            FROM ple_data.question_pool_watch AS pool_watch
           WHERE NEW.target_kind = 'question_pool'
             AND pool_watch.public_question_pool_id = NEW.target_public_id
      ) AS watch
      JOIN ple_private.account AS account
        ON account.account_id = watch.instructor_account_id
       AND account.product_role = 'instructor'
      JOIN LATERAL (
          SELECT state FROM ple_private.account_state_event
           WHERE account_id = account.account_id
           ORDER BY occurred_at DESC, event_id DESC LIMIT 1
      ) AS state_event ON state_event.state = 'active'
     WHERE ple_private.verified_instructor_display_name(account.account_id) IS NOT NULL;
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
        target_kind, target_public_id, event_kind, revision_number, occurred_at
    ) VALUES ('question', NEW.question_id, 'revision', NEW.revision_number, NEW.occurred_at);
    RETURN NEW;
END
$$;
CREATE TRIGGER question_publication_enqueues_watch_revision
AFTER INSERT ON ple_data.question_publication_event
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_question_watch_revision_event();

-- The public provenance row is inserted only in the fork publication
-- transaction; a private Draft fork cannot emit an event.
CREATE FUNCTION ple_data.enqueue_question_watch_fork_event()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    INSERT INTO ple_data.library_watch_event(
        target_kind, target_public_id, event_kind, revision_number,
        forked_public_id, occurred_at
    ) VALUES (
        'question', NEW.source_question_id, 'fork', NEW.source_revision_number,
        NEW.forked_question_id, NEW.recorded_at
    );
    RETURN NEW;
END
$$;
CREATE TRIGGER question_fork_enqueues_watch_notification
AFTER INSERT ON ple_data.question_fork_source
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_question_watch_fork_event();

CREATE FUNCTION ple_data.enqueue_question_pool_watch_revision_event()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE public_id text;
BEGIN
    SELECT public_question_pool_id INTO public_id
      FROM ple_data.question_pool WHERE question_pool_id = NEW.question_pool_id;
    INSERT INTO ple_data.library_watch_event(
        target_kind, target_public_id, event_kind, revision_number, occurred_at
    ) VALUES ('question_pool', public_id, 'revision', NEW.revision_number, NEW.created_at);
    RETURN NEW;
END
$$;
CREATE TRIGGER question_pool_revision_enqueues_watch_notification
AFTER INSERT ON ple_data.question_pool_revision
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_question_pool_watch_revision_event();

CREATE FUNCTION ple_data.enqueue_question_pool_watch_fork_event()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE source_public_id text;
BEGIN
    IF NEW.source_question_pool_id IS NULL THEN RETURN NEW; END IF;
    SELECT public_question_pool_id INTO source_public_id
      FROM ple_data.question_pool WHERE question_pool_id = NEW.source_question_pool_id;
    INSERT INTO ple_data.library_watch_event(
        target_kind, target_public_id, event_kind, revision_number,
        forked_public_id, occurred_at
    ) VALUES (
        'question_pool', source_public_id, 'fork', NEW.source_question_pool_revision_number,
        NEW.public_question_pool_id, NEW.created_at
    );
    RETURN NEW;
END
$$;
CREATE TRIGGER question_pool_fork_enqueues_watch_notification
AFTER INSERT ON ple_data.question_pool
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_question_pool_watch_fork_event();

-- A Watch event records the birth of the retained thread. Replies, post edits,
-- and thread lifecycle changes remain visible in the activity view.
CREATE FUNCTION ple_data.enqueue_library_watch_thread_event()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    INSERT INTO ple_data.library_watch_event(
        target_kind, target_public_id, event_kind, revision_number, activity_id, occurred_at
    ) VALUES (
        NEW.object_kind, NEW.public_object_id, 'improvement_thread',
        NEW.creation_revision_number, NEW.thread_id, NEW.created_at
    );
    RETURN NEW;
END
$$;
CREATE TRIGGER library_improvement_thread_enqueues_watch_notification
AFTER INSERT ON ple_data.library_improvement_thread
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_library_watch_thread_event();

CREATE FUNCTION ple_data.enqueue_library_watch_impact_event()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    INSERT INTO ple_data.library_watch_event(
        target_kind, target_public_id, event_kind, revision_number, activity_id, occurred_at
    ) VALUES (
        NEW.object_kind, NEW.public_object_id, 'impact_notice',
        NEW.affected_revision_number, NEW.impact_notice_id, NEW.created_at
    );
    RETURN NEW;
END
$$;
CREATE TRIGGER library_impact_notice_enqueues_watch_notification
AFTER INSERT ON ple_data.library_impact_notice
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_library_watch_impact_event();

-- ASVS 8.2.1/8.3.1: this self-only Inbox derives the Account solely from the
-- installed server session and reveals neither recipient nor Watch facts.
CREATE FUNCTION ple_data.read_current_library_watch_notifications(p_limit integer)
RETURNS TABLE(
    target_kind text, target_public_id text, event_kind text,
    revision_number bigint, forked_public_id text, activity_id uuid, occurred_at_millis bigint
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE actor_id uuid;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF p_limit NOT BETWEEN 1 AND 100 OR actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Library Watch inbox requires an active Instructor Account';
    END IF;
    RETURN QUERY
    SELECT notification.target_kind, notification.target_public_id, notification.event_kind,
           notification.revision_number, notification.forked_public_id,
           notification.activity_id,
           (EXTRACT(EPOCH FROM notification.occurred_at) * 1000)::bigint
      FROM ple_private.library_watch_notification AS notification
     WHERE notification.recipient_account_id = actor_id
     ORDER BY notification.occurred_at DESC, notification.notification_id DESC
     LIMIT p_limit;
END
$$;

REVOKE ALL ON FUNCTION ple_api.materialize_library_watch_notifications(integer),
    ple_data.enqueue_question_watch_revision_event(),
    ple_data.enqueue_question_watch_fork_event(),
    ple_data.enqueue_question_pool_watch_revision_event(),
    ple_data.enqueue_question_pool_watch_fork_event(),
    ple_data.enqueue_library_watch_thread_event(),
    ple_data.enqueue_library_watch_impact_event(),
    ple_data.snapshot_library_watch_event_recipients(),
    ple_data.read_current_library_watch_notifications(integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.materialize_library_watch_notifications(integer)
    TO ple_assessment_attempt_expiry_worker;
GRANT EXECUTE ON FUNCTION ple_data.read_current_library_watch_notifications(integer)
    TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.read_current_library_watch_notifications(p_limit integer)
RETURNS TABLE(
    target_kind text, target_public_id text, event_kind text,
    revision_number bigint, forked_public_id text, activity_id uuid, occurred_at_millis bigint
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.read_current_library_watch_notifications($1)
$$;
REVOKE ALL ON FUNCTION ple_api.read_current_library_watch_notifications(integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_current_library_watch_notifications(integer) TO ple_app;
REVOKE CREATE ON SCHEMA ple_api FROM ple_data_owner;
RESET ROLE;
