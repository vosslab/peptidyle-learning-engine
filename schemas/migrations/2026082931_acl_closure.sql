-- Schema and ACL closure.

SET LOCAL ROLE ple_data_owner;
REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA ple_data FROM PUBLIC;
REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA ple_data FROM PUBLIC;
REVOKE ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA ple_data FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA ple_private FROM PUBLIC;
REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA ple_private FROM PUBLIC;
REVOKE ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA ple_private FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA ple_audit FROM PUBLIC;
REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA ple_audit FROM PUBLIC;
REVOKE ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA ple_audit FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA ple_api FROM PUBLIC;
REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA ple_api FROM PUBLIC;
REVOKE ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA ple_api FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_auth, ple_app, ple_student;
GRANT EXECUTE ON FUNCTION ple_api.resolve_and_install_session(bytea) TO ple_auth;
GRANT EXECUTE ON FUNCTION ple_api.current_session_account_id(),
    ple_api.current_session_account_is_course_instructor(uuid),
    ple_api.current_session_account_owns_student_record(uuid, uuid),
    ple_api.current_session_account_owns_workspace(uuid),
    ple_api.current_session_account_can_access_workspace(uuid),
    ple_api.current_session_account_has_course_observer_relationship(uuid),
    ple_api.current_session_account_has_support_capability(uuid, uuid, uuid, text)
    TO ple_app, ple_auth, ple_student;
RESET ROLE;
