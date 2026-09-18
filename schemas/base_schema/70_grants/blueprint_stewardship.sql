-- Privileges from blueprint_stewardship.sql.

SET LOCAL ROLE ple_data_owner;

GRANT SELECT ON ple_data.blueprint_course TO ple_data_owner;

GRANT REFERENCES ON ple_data.blueprint_course TO ple_private_owner;

GRANT SELECT ON ple_data.blueprint_metadata_event TO ple_data_owner;

REVOKE ALL ON TABLE ple_data.blueprint_course_star,
    ple_data.blueprint_course_watch FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.set_current_blueprint_course_star(bigint, boolean),
    ple_data.set_current_blueprint_course_watch(bigint, boolean) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.set_current_blueprint_course_star(bigint, boolean),
    ple_data.set_current_blueprint_course_watch(bigint, boolean) TO ple_api_owner;

SET LOCAL ROLE ple_private_owner;




-- C409's recipient rows are the only Watch-event projection.  They are made
-- at the immutable source-event insertion, rather than by a generic course
-- hook, a fork/adoption path, or a later mutable lookup.  A row names only
-- its recipient and never becomes a Watch directory or aggregate.
GRANT SELECT ON ple_private.account, ple_private.account_state_event TO ple_data_owner;

GRANT INSERT, SELECT ON ple_private.blueprint_course_watch_notification TO ple_data_owner;

REVOKE ALL ON TABLE ple_private.blueprint_course_watch_notification FROM PUBLIC;

SET LOCAL ROLE ple_data_owner;


-- PostgreSQL checks trigger-function EXECUTE for the table owner at CREATE
-- TRIGGER time.  This is the sole cross-owner execution grant: it is neither
-- public nor application-principal authority.
GRANT EXECUTE ON FUNCTION ple_data.reject_blueprint_course_watch_notification_change()
    TO ple_private_owner;

REVOKE ALL ON FUNCTION ple_data.fan_out_blueprint_course_watch_notifications(
    bigint, text, bigint, timestamptz),
    ple_data.enqueue_blueprint_course_watch_revision(),
    ple_data.enqueue_blueprint_course_watch_lifecycle_change(),
    ple_data.read_current_blueprint_course_star(bigint),
    ple_data.read_current_blueprint_course_starred_instructors(bigint),
    ple_data.read_current_blueprint_course_watch(bigint),
    ple_data.read_current_blueprint_course_watch_events(bigint, integer),
    ple_data.reject_blueprint_course_watch_notification_change() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.read_current_blueprint_course_star(bigint),
    ple_data.read_current_blueprint_course_starred_instructors(bigint),
    ple_data.read_current_blueprint_course_watch(bigint),
    ple_data.read_current_blueprint_course_watch_events(bigint, integer) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.read_current_blueprint_course_star(text),
    ple_api.read_current_blueprint_course_starred_instructors(text),
    ple_api.read_current_blueprint_course_watch(text),
    ple_api.read_current_blueprint_course_watch_events(text, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_current_blueprint_course_star(text),
    ple_api.read_current_blueprint_course_starred_instructors(text),
    ple_api.read_current_blueprint_course_watch(text),
    ple_api.read_current_blueprint_course_watch_events(text, integer) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.set_current_blueprint_course_star(text, boolean),
    ple_api.set_current_blueprint_course_watch(text, boolean) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.set_current_blueprint_course_star(text, boolean),
    ple_api.set_current_blueprint_course_watch(text, boolean) TO ple_app;

