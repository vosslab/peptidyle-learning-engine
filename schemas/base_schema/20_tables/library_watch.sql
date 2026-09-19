-- library_watch tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

-- Private in-app Watch notifications for Question Library Objects.
-- This dedicated Watch outbox has exactly two target kinds and the four Human
-- Guidance event kinds. It is not a generic event bus and never delivers email.
-- One recipient row is the notification; reads JOIN recipient to event.
CREATE TABLE ple_data.library_watch_event (
    event_id uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
    target_kind ple_data.library_object_kind NOT NULL,
    target_public_id text NOT NULL CHECK (
        target_public_id ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
        AND substr(target_public_id, 6, 1) = ple_private.crockford_checksum_character(
            substr(target_public_id, 1, 4) || substr(target_public_id, 7, 3)
        )
    ),
    event_kind ple_data.library_watch_event_kind NOT NULL,
    revision_number integer CHECK (revision_number > 0),
    forked_public_id text CHECK (forked_public_id IS NULL
        OR (forked_public_id ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
            AND substr(forked_public_id, 6, 1) = ple_private.crockford_checksum_character(
                substr(forked_public_id, 1, 4) || substr(forked_public_id, 7, 3)
            ))),
    activity_id uuid,
    occurred_at timestamptz NOT NULL,
    processed_at timestamptz,
    CHECK (
        (event_kind IN ('revision', 'members_changed') AND revision_number IS NOT NULL
            AND forked_public_id IS NULL AND activity_id IS NULL)
        OR (event_kind = 'fork' AND revision_number IS NOT NULL
            AND forked_public_id IS NOT NULL AND activity_id IS NULL)
        OR (event_kind = 'improvement_thread' AND revision_number IS NOT NULL
            AND forked_public_id IS NULL AND activity_id IS NOT NULL)
        OR (event_kind = 'impact_notice' AND forked_public_id IS NULL
            AND activity_id IS NOT NULL)
    )
);

-- Recipient snapshots preserve who was actively subscribed when the source
-- event INSERT statement ran in its transaction. That transaction commits or
-- rolls back the event and frozen snapshot together; a later Watch/unwatch
-- cannot rewrite that event. Each recipient row is the in-app notification.
CREATE TABLE ple_data.library_watch_event_recipient (
    library_watch_event_id uuid NOT NULL REFERENCES ple_data.library_watch_event(event_id),
    recipient_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    PRIMARY KEY (library_watch_event_id, recipient_account_id),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);

COMMENT ON TABLE ple_data.library_watch_event IS 'role: event, deleted by Watch unsubscribe and event retention. HUMAN_GUIDANCE.md Library Watch.';
COMMENT ON TABLE ple_data.library_watch_event_recipient IS 'role: event, deleted by Watch unsubscribe and event retention. HUMAN_GUIDANCE.md Library Watch.';
COMMENT ON COLUMN ple_data.library_watch_event.revision_number IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.library_watch_event.forked_public_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.library_watch_event.activity_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.library_watch_event.processed_at IS 'NULL means this optional fact is absent.';
