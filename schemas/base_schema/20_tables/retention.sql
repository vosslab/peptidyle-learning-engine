-- retention tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.
-- FERPA notice/archive/deletion intervals are deployment configuration
-- (ple.retention_* settings via ple_data.retention_schedule()), not a table.

SET LOCAL ROLE ple_private_owner;

-- Retention-notification receipts are a narrowly scoped, no-login capability.
-- This module consumes the retained Course schedule; it neither defines dates
-- nor exposes a general Account lookup or outbound-message queue.
CREATE TABLE ple_private.course_retention_notification (
    notification_id uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
    course_instance_id ple_data.course_instance_id NOT NULL REFERENCES ple_data.course_instance (course_instance_id),
    action_kind ple_data.retention_action_kind NOT NULL,
    due_at timestamp with time zone NOT NULL,
    recipient_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account (account_id),
    recipient_product_role ple_data.product_role NOT NULL DEFAULT 'instructor',
    provider_idempotency_key uuid NOT NULL DEFAULT pg_catalog.gen_random_uuid(),
    created_at timestamp with time zone NOT NULL,
    next_attempt_at timestamp with time zone NOT NULL,
    claimed_at timestamp with time zone,
    lease_expires_at timestamp with time zone,
    lease_token uuid,
    provider_accepted_at timestamp with time zone,
    last_failure_at timestamp with time zone,
    last_failure_kind ple_data.retention_failure_kind,
    attempt_count integer NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    CONSTRAINT course_retention_notification_identity
        UNIQUE (course_instance_id, action_kind, due_at, recipient_account_id),
    UNIQUE (provider_idempotency_key),
    FOREIGN KEY (recipient_account_id, recipient_product_role)
        REFERENCES ple_private.account (account_id, product_role),
    CHECK (ple_private.work_lease_pair_is_valid(lease_token, lease_expires_at, created_at)),
    CHECK ((claimed_at IS NULL) = (lease_token IS NULL)),
    CHECK (claimed_at IS NULL OR claimed_at >= created_at),
    CHECK (lease_expires_at IS NULL OR claimed_at IS NULL OR lease_expires_at > claimed_at),
    CHECK (provider_accepted_at IS NULL OR provider_accepted_at >= created_at),
    CHECK ((last_failure_at IS NULL AND last_failure_kind IS NULL)
        OR (last_failure_at IS NOT NULL AND last_failure_kind IS NOT NULL)),
    CHECK (next_attempt_at >= due_at)
);

COMMENT ON TABLE ple_private.course_retention_notification IS 'role: event, deleted by Course delete after the retention sweep. HUMAN_GUIDANCE.md Retention.';
COMMENT ON COLUMN ple_private.course_retention_notification.claimed_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.course_retention_notification.lease_expires_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.course_retention_notification.lease_token IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.course_retention_notification.provider_accepted_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.course_retention_notification.last_failure_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.course_retention_notification.last_failure_kind IS 'NULL means this optional fact is absent.';
