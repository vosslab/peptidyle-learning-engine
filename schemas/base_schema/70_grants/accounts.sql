-- Privileges from accounts.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL PRIVILEGES ON TABLE ple_private.account, ple_private.account_state_event,
    ple_private.account_time_zone, ple_private.account_appearance,
    ple_private.instructor_personal_theme, ple_private.instructor_profile FROM PUBLIC;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_account_identity_change(),
    ple_private.record_initial_account_state(), ple_private.account_time_zone_is_exact_iana(text),
    ple_private.reject_invalid_account_time_zone(), ple_private.record_default_account_time_zone(),
    ple_private.record_default_account_appearance()
    FROM PUBLIC;

GRANT SELECT ON ple_private.account, ple_private.account_state_event TO ple_api_owner;

GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;

GRANT REFERENCES ON TABLE ple_private.account TO ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;

REVOKE ALL PRIVILEGES ON TABLE ple_audit.instructor_account_creation_event FROM PUBLIC;

REVOKE ALL PRIVILEGES ON FUNCTION ple_audit.reject_instructor_account_creation_event_change(),
    ple_audit.record_instructor_account_creation_event(text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_audit.record_instructor_account_creation_event(text, text)
    TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.require_current_sysadmin_account(),
    ple_private.current_authenticated_account_time_zone(),
    ple_private.update_current_authenticated_account_time_zone(text),
    ple_private.current_authenticated_account_appearance(),
    ple_private.update_current_authenticated_account_display_mode_preference(text),
    ple_private.update_current_authenticated_instructor_personal_theme(text),
    ple_private.apply_student_invitation_time_zone_default(text, text),
    ple_private.resolve_or_create_student_account(text, text),
    ple_private.create_instructor_account(text, text, text, text, text),
    ple_private.instructor_display_name(text), ple_private.instructor_affiliation(text),
    ple_private.instructor_setup_email_destination(text),
    ple_private.instructor_account_summary(text),
    ple_private.list_instructor_accounts(),
    ple_private.create_instructor_account_summary(text, text, text, text),
    ple_private.change_instructor_account_state(text, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.create_instructor_account(text, text, text, text, text),
    ple_private.instructor_display_name(text), ple_private.instructor_affiliation(text),
    ple_private.instructor_setup_email_destination(text),
    ple_private.create_instructor_account_summary(text, text, text, text),
    ple_private.list_instructor_accounts(),
    ple_private.change_instructor_account_state(text, text, text),
    ple_private.current_authenticated_account_time_zone(),
    ple_private.update_current_authenticated_account_time_zone(text),
    ple_private.current_authenticated_account_appearance(),
    ple_private.update_current_authenticated_account_display_mode_preference(text),
    ple_private.update_current_authenticated_instructor_personal_theme(text),
    ple_private.apply_student_invitation_time_zone_default(text, text),
    ple_private.resolve_or_create_student_account(text, text) TO ple_api_owner;


-- C853's closed Question Star projection runs as ple_data_owner and may call
-- this private helper only after its own active-Instructor and Published
-- Question predicates have succeeded.  ple_app receives no direct grant.
GRANT EXECUTE ON FUNCTION ple_private.instructor_display_name(text),
    ple_private.instructor_affiliation(text)
    TO ple_data_owner, ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_account_time_zone(),
    ple_api.update_current_account_time_zone(text),
    ple_api.current_account_appearance(),
    ple_api.update_current_account_display_mode_preference(text),
    ple_api.update_current_instructor_personal_theme(text),
    ple_api.list_instructor_accounts(), ple_api.create_instructor_account(text, text, text, text),
    ple_api.instructor_setup_email_destination(text),
    ple_api.change_instructor_account_state(text, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.current_account_time_zone(),
    ple_api.update_current_account_time_zone(text),
    ple_api.current_account_appearance(),
    ple_api.update_current_account_display_mode_preference(text),
    ple_api.update_current_instructor_personal_theme(text),
    ple_api.list_instructor_accounts(), ple_api.create_instructor_account(text, text, text, text),
    ple_api.instructor_setup_email_destination(text),
    ple_api.change_instructor_account_state(text, text, text) TO ple_app;
