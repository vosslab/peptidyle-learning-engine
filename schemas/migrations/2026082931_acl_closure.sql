-- SD1 final schema, relation, sequence, and function ACL closure.

SET LOCAL ROLE ple_data_owner;
REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA ple_data FROM PUBLIC;
REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA ple_data FROM PUBLIC;
REVOKE ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA ple_data FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_data TO ple_app;
GRANT SELECT ON TABLE ple_data.course_instance, ple_data.course_membership,
    ple_data.course_student, ple_data.assignment_enrollment TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA ple_private FROM PUBLIC;
REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA ple_private FROM PUBLIC;
REVOKE ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA ple_private FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_app, ple_worker;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE ple_private.authoring_workspace,
    ple_private.authoring_workspace_collaborator,
    ple_private.workspace_draft_question, ple_private.workspace_flat_question_source TO ple_app;
GRANT INSERT, UPDATE, DELETE ON TABLE ple_private.workspace_flat_question_grading TO ple_app;
GRANT SELECT ON TABLE ple_private.published_flat_question_grading TO ple_grader;
GRANT SELECT ON TABLE ple_private.published_qti_question_grading TO ple_grader;
GRANT SELECT, INSERT, UPDATE ON TABLE ple_private.workspace_qti_import TO ple_app;
GRANT INSERT, UPDATE ON TABLE ple_private.workspace_qti_import_grading TO ple_app;
GRANT SELECT, INSERT, DELETE ON TABLE ple_private.worker_job TO ple_app;
GRANT SELECT ON TABLE ple_private.worker_job TO ple_worker;
GRANT UPDATE ON TABLE ple_private.workspace_qti_import, ple_private.worker_job TO ple_worker;
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
GRANT USAGE ON SCHEMA ple_api TO ple_auth, ple_app, ple_student, ple_grader, ple_worker;
GRANT EXECUTE ON FUNCTION ple_api.resolve_and_install_actor(bytea) TO ple_auth;
GRANT EXECUTE ON FUNCTION ple_api.current_actor_user_id(),
    ple_api.current_actor_is_course_instructor(uuid),
    ple_api.current_actor_is_course_student(uuid, uuid),
    ple_api.current_actor_owns_workspace(uuid),
    ple_api.current_actor_can_access_workspace(uuid),
    ple_api.current_actor_has_course_observer_grant(uuid),
    ple_api.current_actor_has_student_observer_grant(uuid, uuid),
    ple_api.current_actor_has_support_capability(uuid, uuid, uuid, text)
    TO ple_app, ple_auth, ple_student, ple_grader, ple_worker;
GRANT EXECUTE ON FUNCTION ple_api.current_worker_has_job_lease(uuid) TO ple_worker;
GRANT EXECUTE ON FUNCTION ple_api.qti_import_is_committed(uuid, uuid) TO ple_grader;
RESET ROLE;
