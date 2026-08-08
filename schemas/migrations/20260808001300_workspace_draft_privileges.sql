-- MOD-UI-EDITOR: a workspace draft contains private source locators and
-- authoring/evaluation configuration. Tenant RLS alone is insufficient: a
-- same-tenant student must not receive the full payload outside the dedicated
-- instructor workspace API.

REVOKE ALL ON workspace_draft
    FROM PUBLIC, ple_student, ple_grader, ple_qti_grader, ple_queue_broker, ple_auth;

-- Preserve the application role's existing server-side authoring operations.
GRANT SELECT, INSERT, UPDATE, DELETE ON workspace_draft TO ple_app;
