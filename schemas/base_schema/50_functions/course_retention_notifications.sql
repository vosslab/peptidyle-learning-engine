-- Functions, triggers, and views from course_retention_notifications.sql.

SET LOCAL ROLE ple_course_retention_notification_owner;

CREATE FUNCTION ple_api.claim_course_retention_notification(
    p_evaluated_at timestamp with time zone,
    p_lease_seconds integer
) RETURNS TABLE (
    notification_id uuid,
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
        course_instance_id, action_kind, due_at, recipient_account_id,
        created_at, next_attempt_at
    )
    WITH due AS (
        SELECT action.course_instance_id, action.due_action AS action_kind, action.due_at
          FROM ple_data.course_retention_due_actions(p_evaluated_at) AS action
         WHERE action.due_action IN ('warn_inactive', 'notify_archive')
    ), recipient AS (
        SELECT due.course_instance_id, due.action_kind, due.due_at, membership.account_id
          FROM due
          JOIN ple_data.course_membership AS membership
            ON membership.course_instance_id = due.course_instance_id
           AND membership.role = 'instructor'
           AND ple_data.course_membership_is_active(membership.course_membership_id)
    )
    SELECT recipient.course_instance_id, recipient.action_kind::ple_data.retention_action_kind, recipient.due_at,
           recipient.account_id, p_evaluated_at, recipient.due_at
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

    -- ASVS 8.3.2/14.2.6: every claim rechecks current Instructor eligibility
    -- and returns only that Account's current verified delivery address.
    -- ASVS 2.3.4: row locking plus SKIP LOCKED prevents two notifier workers
    -- from taking the same receipt. An expired unaccepted lease is retryable;
    -- provider acceptance is terminal for sending.
    RETURN QUERY
    WITH candidate AS (
        SELECT receipt.notification_id, email.delivery_email AS verified_destination
          FROM ple_private.course_retention_notification AS receipt
          JOIN ple_private.account AS account
            ON account.account_id = receipt.recipient_account_id
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
         WHERE receipt.next_attempt_at <= p_evaluated_at
           AND receipt.provider_accepted_at IS NULL
           AND (receipt.lease_expires_at IS NULL OR receipt.lease_expires_at <= p_evaluated_at)
           AND EXISTS (
               SELECT 1
                 FROM ple_data.course_retention_due_actions(p_evaluated_at) AS due
                WHERE due.course_instance_id = receipt.course_instance_id
                  AND due.due_action::ple_data.retention_action_kind = receipt.action_kind
                  AND due.due_at = receipt.due_at
           )
           AND EXISTS (
               SELECT 1
                 FROM ple_data.course_membership AS membership
                WHERE membership.course_instance_id = receipt.course_instance_id
                  AND membership.account_id = receipt.recipient_account_id
                  AND membership.role = 'instructor'
                  AND ple_data.course_membership_is_active(
                      membership.course_membership_id
                  )
           )
         ORDER BY receipt.due_at, receipt.notification_id
         FOR UPDATE OF receipt SKIP LOCKED
         LIMIT 1
    ), claimed AS (
        UPDATE ple_private.course_retention_notification AS receipt
           SET claimed_at = p_evaluated_at,
               lease_expires_at = p_evaluated_at
                   + pg_catalog.make_interval(secs => p_lease_seconds),
               next_attempt_at = p_evaluated_at
                   + pg_catalog.make_interval(secs => p_lease_seconds),
               lease_token = pg_catalog.gen_random_uuid(),
               attempt_count = receipt.attempt_count + 1
          FROM candidate
         WHERE receipt.notification_id = candidate.notification_id
         RETURNING receipt.notification_id, candidate.verified_destination,
                   receipt.provider_idempotency_key, receipt.lease_token
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
           last_failure_kind = p_failure_kind::ple_data.retention_failure_kind,
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
