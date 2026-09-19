-- Privileges from question_watch_notifications.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.library_watch_event FROM PUBLIC;

REVOKE ALL ON TABLE ple_data.library_watch_event_recipient FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_api.materialize_library_watch_notifications(integer),
    ple_data.enqueue_question_watch_revision_event(),
    ple_data.enqueue_question_watch_fork_event(),
    ple_data.enqueue_question_pool_watch_members_changed_event(),
    ple_data.enqueue_question_pool_watch_fork_event(),
    ple_data.enqueue_library_watch_thread_event(),
    ple_data.enqueue_library_watch_impact_event(),
    ple_data.snapshot_library_watch_event_recipients(),
    ple_data.read_current_library_watch_notifications(integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.materialize_library_watch_notifications(integer)
    TO ple_assessment_attempt_expiry_worker;

GRANT EXECUTE ON FUNCTION ple_data.read_current_library_watch_notifications(integer)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.read_current_library_watch_notifications(integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_current_library_watch_notifications(integer) TO ple_app;

REVOKE CREATE ON SCHEMA ple_api FROM ple_data_owner;
