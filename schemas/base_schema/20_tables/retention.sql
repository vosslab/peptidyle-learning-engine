-- retention tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

-- Operational Course-retention policy.  Course-core owns Course facts; this
-- module only derives due work from those facts and the current policy.
CREATE TABLE ple_data.course_retention_policy (
    policy_key boolean PRIMARY KEY DEFAULT true CHECK (policy_key),
    inactive_warning_lead_time interval NOT NULL CHECK (inactive_warning_lead_time > INTERVAL '0'),
    archive_notice_lead_time interval NOT NULL CHECK (archive_notice_lead_time > INTERVAL '0'),
    archive_after_retention_start interval NOT NULL
        CHECK (archive_after_retention_start > INTERVAL '0'),
    delete_after_archive interval NOT NULL CHECK (delete_after_archive > INTERVAL '0'),
    CHECK (archive_notice_lead_time <= archive_after_retention_start)
);



-- These are operational defaults, not product-policy constants.  The retention
-- start is the latest Assessment deadline: archive at day 100, with a day-30
-- notice expressed as a 70-day lead (100 - 70), then delete at day 365 through
-- the 265-day interval after the same absolute archive cutoff (100 + 265).
-- The independent 14-day inactive-Course warning remains unchanged.
-- A deployment administrator can set its local FERPA periods before retention
-- processing.
INSERT INTO ple_data.course_retention_policy (
    inactive_warning_lead_time,
    archive_notice_lead_time,
    archive_after_retention_start,
    delete_after_archive
) VALUES (
    INTERVAL '14 days',
    INTERVAL '70 days',
    INTERVAL '100 days',
    INTERVAL '265 days'
);

SET LOCAL ROLE ple_private_owner;

-- Retention-notification receipts are a narrowly scoped, no-login capability.
-- This module consumes the retained Course schedule; it neither defines dates
-- nor exposes a general Account lookup or outbound-message queue.
CREATE TABLE ple_private.course_retention_notification (
    notification_id uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    action_kind text NOT NULL CHECK (action_kind IN ('warn_inactive', 'notify_archive')),
    due_at timestamp with time zone NOT NULL,
    recipient_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    recipient_product_role text NOT NULL DEFAULT 'instructor'
        CHECK (recipient_product_role = 'instructor'),
    provider_idempotency_key uuid NOT NULL DEFAULT pg_catalog.gen_random_uuid(),
    created_at timestamp with time zone NOT NULL,
    next_attempt_at timestamp with time zone NOT NULL,
    claimed_at timestamp with time zone,
    lease_expires_at timestamp with time zone,
    lease_token uuid,
    provider_accepted_at timestamp with time zone,
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
    CHECK ((last_failure_at IS NULL AND last_failure_kind IS NULL)
        OR (last_failure_at IS NOT NULL AND last_failure_kind IS NOT NULL)),
    CHECK (next_attempt_at >= due_at)
);

SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.course_retention_policy IS 'role: vocabulary, deleted by Course delete after the retention sweep. HUMAN_GUIDANCE.md Retention.';

SET LOCAL ROLE ple_private_owner;

COMMENT ON TABLE ple_private.course_retention_notification IS 'role: event, deleted by Course delete after the retention sweep. HUMAN_GUIDANCE.md Retention.';

