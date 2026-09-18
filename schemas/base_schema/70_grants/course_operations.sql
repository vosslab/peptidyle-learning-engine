-- Privileges from course_operations.sql.

SET LOCAL ROLE ple_data_owner;
GRANT SELECT ON TABLE ple_data.course_theme TO ple_api_owner, ple_app;
GRANT REFERENCES ON TABLE ple_data.course_theme TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.update_course_classification(text, uuid, uuid, uuid, uuid, uuid, text[]) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.update_course_classification(text, uuid, uuid, uuid, uuid, uuid, text[]) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.read_course_theme(text), ple_api.update_course_theme(text, text),
    ple_api.resolve_course_navigation(text), ple_api.read_course_summary(text),
    ple_api.create_course_instance(text, uuid, uuid, uuid, text, text, bigint, text, text, date, date, text, jsonb, uuid, uuid, uuid, uuid, text[]),
    ple_api.add_course_instructor(uuid, text, text),
    ple_api.list_course_instances(), ple_api.load_course_instance(text),
    ple_api.list_course_creation_instructors(), ple_api.list_course_roster(text),
    ple_api.read_course_roster_entry_repair_support(uuid, text, text),
    ple_api.list_live_student_course_landing(),
    ple_api.list_pending_student_course_invitations(),
    ple_api.export_pending_course_invitations(text),
    ple_api.load_invitation_export_course(text),
    ple_api.import_course_roster(text, text[], text[], text[], text[]),
    ple_api.claim_course_invitation(uuid, uuid, uuid, text),
    ple_api.revoke_course_roster_entry(uuid, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.read_course_theme(text), ple_api.update_course_theme(text, text),
    ple_api.resolve_course_navigation(text), ple_api.read_course_summary(text),
    ple_api.create_course_instance(text, uuid, uuid, uuid, text, text, bigint, text, text, date, date, text, jsonb, uuid, uuid, uuid, uuid, text[]),
    ple_api.add_course_instructor(uuid, text, text),
    ple_api.list_course_instances(), ple_api.load_course_instance(text),
    ple_api.list_course_creation_instructors(), ple_api.list_course_roster(text),
    ple_api.read_course_roster_entry_repair_support(uuid, text, text),
    ple_api.list_live_student_course_landing(),
    ple_api.list_pending_student_course_invitations(),
    ple_api.export_pending_course_invitations(text),
    ple_api.load_invitation_export_course(text),
    ple_api.import_course_roster(text, text[], text[], text[], text[]),
    ple_api.claim_course_invitation(uuid, uuid, uuid, text),
    ple_api.revoke_course_roster_entry(uuid, text, text) TO ple_app;

