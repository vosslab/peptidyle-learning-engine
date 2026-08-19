-- A RETURNS TABLE function exposes its output names as PL/pgSQL variables.
-- Keep every outbox-table reference qualified so those names cannot shadow
-- columns during lease reconciliation.

CREATE OR REPLACE FUNCTION public.ple_claim_course_invitation_deliveries(
    p_limit integer, p_lease_seconds integer
) RETURNS TABLE(
    tenant_id uuid, course_id uuid, invitation_id uuid, delivery_id uuid,
    state text, attempt_count integer, next_attempt_at timestamptz, last_attempt_at timestamptz,
    lease_id uuid, lease_expires_at timestamptz, dispatch_started_at timestamptz,
    outcome_code text, created_at timestamptz, updated_at timestamptz, accepted_at timestamptz,
    terminal_at timestamptz
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
BEGIN
    IF p_limit NOT BETWEEN 1 AND 100 OR p_lease_seconds NOT BETWEEN 1 AND 900 THEN
        RAISE EXCEPTION 'invalid invitation delivery claim arguments' USING ERRCODE = '22023';
    END IF;

    UPDATE public.course_invitation_delivery AS delivery
       SET state = CASE WHEN delivery.dispatch_started_at IS NULL THEN 'cancelled' ELSE 'ambiguous' END,
           outcome_code = CASE WHEN delivery.dispatch_started_at IS NULL THEN 'cancelled' ELSE 'ambiguous_transport' END,
           lease_id = NULL, lease_expires_at = NULL, dispatch_started_at = NULL,
           updated_at = transaction_timestamp(), terminal_at = transaction_timestamp()
      FROM public.course_invitation AS invitation
     WHERE (invitation.tenant_id, invitation.course_id, invitation.invitation_id)
         = (delivery.tenant_id, delivery.course_id, delivery.invitation_id)
       AND invitation.expires_at <= transaction_timestamp()
       AND delivery.state IN ('pending', 'retryable_failed');

    UPDATE public.course_invitation_delivery AS delivery
       SET state = CASE WHEN delivery.dispatch_started_at IS NULL THEN delivery.state ELSE 'ambiguous' END,
           outcome_code = CASE WHEN delivery.dispatch_started_at IS NULL THEN delivery.outcome_code ELSE 'ambiguous_transport' END,
           lease_id = NULL, lease_expires_at = NULL, dispatch_started_at = NULL,
           updated_at = transaction_timestamp(),
           terminal_at = CASE WHEN delivery.dispatch_started_at IS NULL THEN delivery.terminal_at ELSE transaction_timestamp() END
     WHERE delivery.state IN ('pending', 'retryable_failed')
       AND delivery.lease_expires_at <= transaction_timestamp();

    UPDATE public.course_invitation_delivery AS delivery
       SET state = 'permanent_failed', outcome_code = 'permanent_failure',
           lease_id = NULL, lease_expires_at = NULL, dispatch_started_at = NULL,
           updated_at = transaction_timestamp(), terminal_at = transaction_timestamp()
     WHERE delivery.state IN ('pending', 'retryable_failed') AND delivery.attempt_count >= 3
       AND (delivery.lease_expires_at IS NULL OR delivery.lease_expires_at <= transaction_timestamp());

    RETURN QUERY
    WITH candidates AS (
        SELECT delivery.tenant_id, delivery.course_id, delivery.invitation_id
          FROM public.course_invitation_delivery AS delivery
          JOIN public.course_invitation AS invitation
            ON (invitation.tenant_id, invitation.course_id, invitation.invitation_id)
             = (delivery.tenant_id, delivery.course_id, delivery.invitation_id)
         WHERE delivery.state IN ('pending', 'retryable_failed')
           AND delivery.next_attempt_at <= transaction_timestamp()
           AND delivery.attempt_count < 3
           AND (delivery.lease_expires_at IS NULL OR delivery.lease_expires_at <= transaction_timestamp())
           AND invitation.status = 'pending' AND invitation.expires_at > transaction_timestamp()
         ORDER BY delivery.next_attempt_at, delivery.delivery_id
         FOR UPDATE OF delivery SKIP LOCKED
         LIMIT p_limit
    ), claimed AS (
        UPDATE public.course_invitation_delivery AS delivery
           SET lease_id = gen_random_uuid(), dispatch_started_at = NULL,
               lease_expires_at = transaction_timestamp() + make_interval(secs => p_lease_seconds),
               attempt_count = delivery.attempt_count + 1,
               last_attempt_at = transaction_timestamp(), updated_at = transaction_timestamp()
          FROM candidates
         WHERE (delivery.tenant_id, delivery.course_id, delivery.invitation_id)
             = (candidates.tenant_id, candidates.course_id, candidates.invitation_id)
        RETURNING delivery.tenant_id, delivery.course_id, delivery.invitation_id, delivery.delivery_id,
                  delivery.state, delivery.attempt_count, delivery.next_attempt_at, delivery.last_attempt_at,
                  delivery.lease_id, delivery.lease_expires_at, delivery.dispatch_started_at,
                  delivery.outcome_code, delivery.created_at, delivery.updated_at, delivery.accepted_at,
                  delivery.terminal_at
    ) SELECT * FROM claimed;
END
$$;

ALTER FUNCTION public.ple_claim_course_invitation_deliveries(integer, integer)
    OWNER TO ple_invitation_delivery_broker;
