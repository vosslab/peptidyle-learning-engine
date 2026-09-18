-- Privileges from authorization.sql.

SET LOCAL ROLE ple_api_owner;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_id(),
    ple_api.current_session_account_has_active_role(text),
    ple_api.current_session_account_is_instructor(),
    ple_api.current_session_account_has_platform_administration(),
    ple_api.current_session_account_is_sysadmin(),
    ple_api.current_session_account_is_course_instructor(text),
    ple_api.current_session_account_is_course_member(text),
    ple_api.current_session_account_owns_course_membership(text, uuid),
    ple_api.current_session_account_owns_student_record(text, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.current_session_account_id(),
    ple_api.current_session_account_has_active_role(text),
    ple_api.current_session_account_is_instructor(),
    ple_api.current_session_account_has_platform_administration(),
    ple_api.current_session_account_is_sysadmin(),
    ple_api.current_session_account_is_course_instructor(text),
    ple_api.current_session_account_is_course_member(text),
    ple_api.current_session_account_owns_course_membership(text, uuid),
    ple_api.current_session_account_owns_student_record(text, uuid)
    TO ple_app, ple_auth, ple_student, ple_data_owner, ple_private_owner;

