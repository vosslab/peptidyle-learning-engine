-- Row security policies from question_watch_notifications.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.library_watch_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.library_watch_event FORCE ROW LEVEL SECURITY;

CREATE POLICY library_watch_event_data_owner_access
    ON ple_data.library_watch_event FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);

ALTER TABLE ple_data.library_watch_event_recipient ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.library_watch_event_recipient FORCE ROW LEVEL SECURITY;

CREATE POLICY library_watch_event_recipient_data_owner_access
    ON ple_data.library_watch_event_recipient FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_private_owner;

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

