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
