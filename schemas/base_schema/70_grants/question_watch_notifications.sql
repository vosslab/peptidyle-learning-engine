-- Privileges from question_watch_notifications.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.library_watch_event FROM PUBLIC;

REVOKE ALL ON TABLE ple_data.library_watch_event_recipient FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.enqueue_question_watch_revision_event(),
    ple_data.enqueue_question_watch_fork_event(),
    ple_data.enqueue_question_pool_watch_members_changed_event(),
    ple_data.enqueue_question_pool_watch_fork_event(),
    ple_data.snapshot_library_watch_event_recipients(),
    ple_data.match_current_library_watch_events(text, text[], text[]),
    ple_data.read_current_library_watch_notifications(text, text[], text[], boolean, bigint, bigint),
    ple_data.read_current_library_watch_facets(text, text[], text[]) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.read_current_library_watch_notifications(text, text[], text[], boolean, bigint, bigint),
    ple_data.read_current_library_watch_facets(text, text[], text[])
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.read_current_library_watch_notifications(text, text[], text[], boolean, bigint, bigint),
    ple_api.read_current_library_watch_facets(text, text[], text[]) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_current_library_watch_notifications(text, text[], text[], boolean, bigint, bigint),
    ple_api.read_current_library_watch_facets(text, text[], text[]) TO ple_app;

REVOKE CREATE ON SCHEMA ple_api FROM ple_data_owner;
