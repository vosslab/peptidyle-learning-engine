-- Private in-app Watch notification outbox. It is unrelated to Course
-- retention notices: no email address, provider, Student fact, or outbound
-- delivery state belongs here.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.question_watch_event (
    event_id uuid PRIMARY KEY,
    question_id text NOT NULL REFERENCES ple_data.published_question(question_id),
    event_kind text NOT NULL CHECK (event_kind IN ('revision', 'fork')),
    occurred_at timestamptz NOT NULL,
    processed_at timestamptz
);
CREATE INDEX question_watch_event_pending_idx
    ON ple_data.question_watch_event(occurred_at, event_id) WHERE processed_at IS NULL;
GRANT REFERENCES ON ple_data.question_watch_event TO ple_private_owner;
GRANT REFERENCES ON ple_data.published_question TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.question_watch_notification (
    notification_id uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
    recipient_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    event_id uuid NOT NULL REFERENCES ple_data.question_watch_event(event_id),
    question_id text NOT NULL REFERENCES ple_data.published_question(question_id),
    event_kind text NOT NULL CHECK (event_kind IN ('revision', 'fork')),
    created_at timestamptz NOT NULL,
    UNIQUE (recipient_account_id, event_id)
);
CREATE INDEX question_watch_notification_recipient_idx
    ON ple_private.question_watch_notification(recipient_account_id, created_at DESC);
ALTER TABLE ple_private.question_watch_notification ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_watch_notification FORCE ROW LEVEL SECURITY;
CREATE POLICY question_watch_notification_private_owner_access
    ON ple_private.question_watch_notification FOR ALL TO ple_private_owner
    USING (true) WITH CHECK (true);
CREATE POLICY question_watch_notification_data_materialization_insert
    ON ple_private.question_watch_notification FOR INSERT TO ple_data_owner
    WITH CHECK (true);
CREATE POLICY question_watch_notification_data_materialization_read
    ON ple_private.question_watch_notification FOR SELECT TO ple_data_owner
    USING (true);
GRANT INSERT, SELECT ON ple_private.question_watch_notification TO ple_data_owner;
REVOKE ALL ON TABLE ple_private.question_watch_notification FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
ALTER TABLE ple_data.question_watch_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_watch_event FORCE ROW LEVEL SECURITY;
CREATE POLICY question_watch_event_data_owner_access ON ple_data.question_watch_event
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_watch_event_private_owner_insert ON ple_data.question_watch_event
    FOR INSERT TO ple_private_owner WITH CHECK (true);
REVOKE ALL ON TABLE ple_data.question_watch_event FROM PUBLIC;

CREATE FUNCTION ple_data.enqueue_question_watch_event()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    INSERT INTO ple_data.question_watch_event(event_id, question_id, event_kind, occurred_at)
    VALUES (pg_catalog.gen_random_uuid(), NEW.question_id, 'revision', NEW.occurred_at);
    RETURN NEW;
END $$;
CREATE TRIGGER question_publication_enqueues_watch_revision
AFTER INSERT ON ple_data.question_publication_event
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_question_watch_event();

CREATE FUNCTION ple_data.enqueue_question_watch_fork_event()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    INSERT INTO ple_data.question_watch_event(event_id, question_id, event_kind, occurred_at)
    VALUES (pg_catalog.gen_random_uuid(), NEW.source_question_id, 'fork', pg_catalog.clock_timestamp());
    RETURN NEW;
END $$;
CREATE TRIGGER question_fork_enqueues_watch_notification
AFTER INSERT ON ple_data.question_fork_source
FOR EACH ROW EXECUTE FUNCTION ple_data.enqueue_question_watch_fork_event();

-- The existing private worker receives only this bounded, idempotent operation.
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
GRANT USAGE, CREATE ON SCHEMA ple_api TO ple_data_owner;
RESET ROLE;
SET LOCAL ROLE ple_data_owner;
CREATE FUNCTION ple_api.materialize_question_watch_notifications(p_limit integer)
RETURNS integer LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE materialized integer;
BEGIN
    IF p_limit NOT BETWEEN 1 AND 500 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Watch notification limit is invalid';
    END IF;
    WITH candidate AS (
        SELECT event_id, question_id, event_kind, occurred_at
          FROM ple_data.question_watch_event
         WHERE processed_at IS NULL
         ORDER BY occurred_at, event_id
         FOR UPDATE SKIP LOCKED LIMIT p_limit
    ), inserted AS (
        INSERT INTO ple_private.question_watch_notification(
            recipient_account_id, event_id, question_id, event_kind, created_at
        )
        SELECT watch.instructor_account_id, candidate.event_id, candidate.question_id,
               candidate.event_kind, pg_catalog.clock_timestamp()
          FROM candidate
          JOIN ple_data.question_watch AS watch ON watch.question_id = candidate.question_id
          JOIN ple_private.account AS account ON account.account_id = watch.instructor_account_id
          JOIN LATERAL (
              SELECT state FROM ple_private.account_state_event
               WHERE account_id = account.account_id ORDER BY occurred_at DESC, event_id DESC LIMIT 1
          ) AS state_event ON state_event.state = 'active'
         WHERE account.product_role = 'instructor'
        ON CONFLICT (recipient_account_id, event_id) DO NOTHING
        RETURNING notification_id
    ), completed AS (
        UPDATE ple_data.question_watch_event AS event SET processed_at = pg_catalog.clock_timestamp()
         FROM candidate WHERE event.event_id = candidate.event_id
    )
    SELECT count(*) INTO materialized FROM inserted;
    RETURN materialized;
END $$;

REVOKE ALL ON FUNCTION ple_data.enqueue_question_watch_event(),
    ple_data.enqueue_question_watch_fork_event(),
    ple_api.materialize_question_watch_notifications(integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.materialize_question_watch_notifications(integer)
    TO ple_assessment_attempt_expiry_worker;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_data_owner;
RESET ROLE;
