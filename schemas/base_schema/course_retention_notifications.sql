-- Retention-notification receipts are a narrowly scoped, no-login capability.
-- This module consumes the retained Course schedule; it neither defines dates
-- nor exposes a general Account lookup or outbound-message queue.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.course_retention_notification (
    notification_id uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    action_kind text NOT NULL CHECK (action_kind IN ('warn_inactive', 'notify_archive')),
    due_at timestamp with time zone NOT NULL,
    recipient_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    recipient_product_role text NOT NULL DEFAULT 'instructor'
        CHECK (recipient_product_role = 'instructor'),
    verified_destination text NOT NULL CHECK (
        char_length(btrim(verified_destination)) BETWEEN 3 AND 320
    ),
    provider_idempotency_key uuid NOT NULL DEFAULT pg_catalog.gen_random_uuid(),
    created_at timestamp with time zone NOT NULL,
    next_attempt_at timestamp with time zone NOT NULL,
    claimed_at timestamp with time zone,
    lease_expires_at timestamp with time zone,
    lease_token uuid,
    provider_accepted_at timestamp with time zone,
    delivered_at timestamp with time zone,
    last_failure_at timestamp with time zone,
    last_failure_kind text CHECK (last_failure_kind IN (
        'not_configured', 'provider_transient', 'provider_rejected'
    )),
    attempt_count integer NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    CONSTRAINT course_retention_notification_identity
        UNIQUE (course_id, action_kind, due_at, recipient_account_id),
    UNIQUE (provider_idempotency_key),
    FOREIGN KEY (recipient_account_id, recipient_product_role)
        REFERENCES ple_private.account (account_id, product_role),
    CHECK ((claimed_at IS NULL AND lease_expires_at IS NULL AND lease_token IS NULL)
        OR (claimed_at IS NOT NULL AND lease_expires_at > claimed_at AND lease_token IS NOT NULL)),
    CHECK (provider_accepted_at IS NULL OR provider_accepted_at >= created_at),
    CHECK (delivered_at IS NULL OR (
        provider_accepted_at IS NOT NULL AND delivered_at >= provider_accepted_at
    )),
    CHECK ((last_failure_at IS NULL AND last_failure_kind IS NULL)
        OR (last_failure_at IS NOT NULL AND last_failure_kind IS NOT NULL)),
    CHECK (next_attempt_at >= due_at)
);

CREATE INDEX course_retention_notification_claim_idx
    ON ple_private.course_retention_notification (due_at, notification_id)
    WHERE provider_accepted_at IS NULL;

ALTER TABLE ple_private.course_retention_notification ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.course_retention_notification FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.course_retention_notification FROM PUBLIC;
CREATE POLICY course_retention_notification_private_owner_access
    ON ple_private.course_retention_notification FOR ALL TO ple_private_owner
    USING (true) WITH CHECK (true);
REVOKE ALL ON TABLE ple_private.course_retention_notification,
    ple_private.account, ple_private.account_state_event,
    ple_private.account_authentication_email FROM ple_course_retention_notifier;
REVOKE USAGE ON SCHEMA ple_private FROM ple_course_retention_notifier;

RESET ROLE;

SET LOCAL ROLE ple_data_owner;

-- A dedicated non-login owner, not the notifier capability, owns the trusted
-- lookup/receipt operation.  The notifier receives only typed functions.
GRANT USAGE ON SCHEMA ple_data TO ple_course_retention_notification_owner;
GRANT SELECT ON ple_data.course_instance, ple_data.course_retention_policy,
    ple_data.course_membership, ple_data.course_membership_event
    TO ple_course_retention_notification_owner;
CREATE POLICY course_instance_retention_notification_owner_read
    ON ple_data.course_instance FOR SELECT
    TO ple_course_retention_notification_owner USING (true);
CREATE POLICY course_retention_policy_notification_owner_read
    ON ple_data.course_retention_policy FOR SELECT
    TO ple_course_retention_notification_owner USING (true);
CREATE POLICY course_membership_retention_notification_owner_read
    ON ple_data.course_membership FOR SELECT
    TO ple_course_retention_notification_owner USING (true);
CREATE POLICY course_membership_event_retention_notification_owner_read
    ON ple_data.course_membership_event FOR SELECT
    TO ple_course_retention_notification_owner USING (true);
GRANT EXECUTE ON FUNCTION ple_data.course_retention_due_actions(timestamp with time zone),
    ple_data.course_membership_is_active(uuid) TO ple_course_retention_notification_owner;
REVOKE ALL ON TABLE ple_data.course_instance, ple_data.course_retention_policy,
    ple_data.course_membership, ple_data.course_membership_event FROM ple_course_retention_notifier;
REVOKE USAGE ON SCHEMA ple_data FROM ple_course_retention_notifier;

RESET ROLE;

-- The dedicated notification owner owns the SECURITY DEFINER entry points.
-- It may create them during this migration only; the notifier has no table ACLs.
SET LOCAL ROLE ple_api_owner;
GRANT USAGE, CREATE ON SCHEMA ple_api TO ple_course_retention_notification_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON SCHEMA ple_private TO ple_course_retention_notification_owner;
GRANT SELECT, INSERT, UPDATE ON ple_private.course_retention_notification
    TO ple_course_retention_notification_owner;
GRANT SELECT ON ple_private.account, ple_private.account_state_event,
    ple_private.account_authentication_email TO ple_course_retention_notification_owner;
CREATE POLICY course_retention_notification_owner_access
    ON ple_private.course_retention_notification FOR ALL
    TO ple_course_retention_notification_owner USING (true) WITH CHECK (true);
CREATE POLICY account_retention_notification_owner_read ON ple_private.account
    FOR SELECT TO ple_course_retention_notification_owner USING (true);
CREATE POLICY account_state_event_retention_notification_owner_read
    ON ple_private.account_state_event FOR SELECT
    TO ple_course_retention_notification_owner USING (true);
CREATE POLICY account_authentication_email_retention_notification_owner_read
    ON ple_private.account_authentication_email FOR SELECT
    TO ple_course_retention_notification_owner USING (true);

RESET ROLE;

SET LOCAL ROLE ple_course_retention_notification_owner;

CREATE FUNCTION ple_api.claim_course_retention_notification(
    p_evaluated_at timestamp with time zone,
    p_lease_seconds integer
) RETURNS TABLE (
    notification_id uuid,
    action_kind text,
    due_at timestamp with time zone,
    verified_destination text,
    provider_idempotency_key uuid,
    lease_token uuid
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
BEGIN
    IF p_evaluated_at IS NULL OR p_lease_seconds NOT BETWEEN 30 AND 900 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course-retention notification claim arguments are invalid';
    END IF;

    -- ASVS 2.3.1/2.3.3: materialize only the two stored due notice actions,
    -- with the exact recipient identity deduplicated before the unique receipt
    -- insert.  Archive and delete are not notification actions.
    INSERT INTO ple_private.course_retention_notification (
        course_id, action_kind, due_at, recipient_account_id,
           verified_destination, created_at, next_attempt_at
    )
    WITH due AS (
        SELECT action.course_id, action.due_action AS action_kind, action.due_at
          FROM ple_data.course_retention_due_actions(p_evaluated_at) AS action
         WHERE action.due_action IN ('warn_inactive', 'notify_archive')
    ), recipient AS (
        SELECT due.course_id, due.action_kind, due.due_at,
               course.assigned_instructor_account_id AS account_id
          FROM due
          JOIN ple_data.course_instance AS course ON course.course_id = due.course_id
        UNION
        SELECT due.course_id, due.action_kind, due.due_at, membership.account_id
          FROM due
          JOIN ple_data.course_membership AS membership
            ON membership.course_id = due.course_id
           AND membership.role = 'instructor'
           AND ple_data.course_membership_is_active(membership.membership_id)
    )
    SELECT recipient.course_id, recipient.action_kind, recipient.due_at,
           recipient.account_id, email.delivery_email, p_evaluated_at, recipient.due_at
      FROM recipient
      JOIN ple_private.account AS account
        ON account.account_id = recipient.account_id
       AND account.product_role = 'instructor'
      JOIN LATERAL (
          SELECT state.state
            FROM ple_private.account_state_event AS state
           WHERE state.account_id = account.account_id
           ORDER BY state.occurred_at DESC, state.event_id DESC
           LIMIT 1
      ) AS current_state ON current_state.state = 'active'
      JOIN ple_private.account_authentication_email AS email
        ON email.account_id = account.account_id
       AND email.verified_at IS NOT NULL
    ON CONFLICT ON CONSTRAINT course_retention_notification_identity DO NOTHING;

    -- ASVS 2.3.4: row locking plus SKIP LOCKED prevents two notifier workers
    -- from taking the same receipt.  An expired unaccepted lease is retryable;
    -- provider acceptance is terminal for sending.
    RETURN QUERY
    WITH candidate AS (
        SELECT receipt.notification_id
          FROM ple_private.course_retention_notification AS receipt
         WHERE receipt.next_attempt_at <= p_evaluated_at
           AND receipt.provider_accepted_at IS NULL
           AND (receipt.lease_expires_at IS NULL OR receipt.lease_expires_at <= p_evaluated_at)
           AND EXISTS (
               SELECT 1
                 FROM ple_data.course_retention_due_actions(p_evaluated_at) AS due
                WHERE due.course_id = receipt.course_id
                  AND due.due_action = receipt.action_kind
                  AND due.due_at = receipt.due_at
           )
         ORDER BY receipt.due_at, receipt.notification_id
         FOR UPDATE SKIP LOCKED
         LIMIT 1
    ), claimed AS (
        UPDATE ple_private.course_retention_notification AS receipt
           SET claimed_at = p_evaluated_at,
               lease_expires_at = p_evaluated_at
                   + pg_catalog.make_interval(secs => p_lease_seconds),
               lease_token = pg_catalog.gen_random_uuid(),
               attempt_count = receipt.attempt_count + 1
          FROM candidate
         WHERE receipt.notification_id = candidate.notification_id
         RETURNING receipt.notification_id, receipt.action_kind, receipt.due_at,
                   receipt.verified_destination, receipt.provider_idempotency_key,
                   receipt.lease_token
    )
    SELECT * FROM claimed;
END
$$;

CREATE FUNCTION ple_api.record_course_retention_notification_provider_acceptance(
    p_notification_id uuid,
    p_lease_token uuid,
    p_provider_idempotency_key uuid,
    p_accepted_at timestamp with time zone
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
BEGIN
    IF p_notification_id IS NULL OR p_lease_token IS NULL
       OR p_provider_idempotency_key IS NULL OR p_accepted_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course-retention provider acceptance arguments are invalid';
    END IF;
    UPDATE ple_private.course_retention_notification AS receipt
       SET provider_accepted_at = p_accepted_at,
           claimed_at = NULL,
           lease_expires_at = NULL,
           lease_token = NULL
     WHERE receipt.notification_id = p_notification_id
       AND receipt.lease_token = p_lease_token
       AND receipt.provider_idempotency_key = p_provider_idempotency_key
       AND receipt.provider_accepted_at IS NULL
       AND p_accepted_at >= receipt.claimed_at
       AND receipt.lease_expires_at > p_accepted_at;
    RETURN FOUND;
END
$$;

CREATE FUNCTION ple_api.record_course_retention_notification_delivered(
    p_notification_id uuid,
    p_provider_idempotency_key uuid,
    p_delivered_at timestamp with time zone
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
BEGIN
    IF p_notification_id IS NULL OR p_provider_idempotency_key IS NULL
       OR p_delivered_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course-retention delivered receipt arguments are invalid';
    END IF;
    UPDATE ple_private.course_retention_notification AS receipt
       SET delivered_at = COALESCE(receipt.delivered_at, p_delivered_at)
     WHERE receipt.notification_id = p_notification_id
       AND receipt.provider_idempotency_key = p_provider_idempotency_key
       AND receipt.provider_accepted_at IS NOT NULL
       AND p_delivered_at >= receipt.provider_accepted_at;
    RETURN FOUND;
END
$$;

CREATE FUNCTION ple_api.fail_course_retention_notification_before_acceptance(
    p_notification_id uuid,
    p_lease_token uuid,
    p_failed_at timestamp with time zone,
    p_failure_kind text
) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
BEGIN
    IF p_notification_id IS NULL OR p_lease_token IS NULL OR p_failed_at IS NULL
       OR p_failure_kind NOT IN ('not_configured', 'provider_transient', 'provider_rejected') THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course-retention notification failure arguments are invalid';
    END IF;
    UPDATE ple_private.course_retention_notification AS receipt
       SET claimed_at = NULL,
           lease_expires_at = NULL,
           lease_token = NULL,
           last_failure_at = p_failed_at,
           last_failure_kind = p_failure_kind,
           next_attempt_at = p_failed_at + pg_catalog.make_interval(secs => LEAST(
               3600,
               60 * pg_catalog.power(2::numeric, LEAST(receipt.attempt_count - 1, 10))::integer
           ))
     WHERE receipt.notification_id = p_notification_id
       AND receipt.lease_token = p_lease_token
       AND p_failed_at >= receipt.claimed_at
       AND receipt.provider_accepted_at IS NULL;
    RETURN FOUND;
END
$$;

REVOKE ALL ON FUNCTION ple_api.claim_course_retention_notification(
        timestamp with time zone, integer
    ),
    ple_api.record_course_retention_notification_provider_acceptance(
        uuid, uuid, uuid, timestamp with time zone
    ),
    ple_api.record_course_retention_notification_delivered(uuid, uuid, timestamp with time zone),
    ple_api.fail_course_retention_notification_before_acceptance(
        uuid, uuid, timestamp with time zone, text
    )
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.claim_course_retention_notification(
        timestamp with time zone, integer
    ),
    ple_api.record_course_retention_notification_provider_acceptance(
        uuid, uuid, uuid, timestamp with time zone
    ),
    ple_api.record_course_retention_notification_delivered(uuid, uuid, timestamp with time zone),
    ple_api.fail_course_retention_notification_before_acceptance(
        uuid, uuid, timestamp with time zone, text
    )
    TO ple_course_retention_notifier;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_course_retention_notification_owner;
GRANT USAGE ON SCHEMA ple_api TO ple_course_retention_notifier;
RESET ROLE;
