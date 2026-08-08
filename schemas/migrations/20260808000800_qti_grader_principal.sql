-- MOD-ADP-QTI: dedicated QTI grader login. Deployment configures its
-- credential externally; this migration never embeds a password or grants it
-- membership in the application role.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_qti_grader') THEN
        CREATE ROLE ple_qti_grader LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOBYPASSRLS;
    END IF;
END
$$;

REVOKE ple_app FROM ple_qti_grader;
REVOKE ple_grader FROM ple_qti_grader;
REVOKE ALL ON workspace_qti_import_grading
    FROM PUBLIC, ple_app, ple_student, ple_grader, ple_qti_grader;
GRANT INSERT ON workspace_qti_import_grading TO ple_app;
GRANT USAGE ON SCHEMA public TO ple_qti_grader;
GRANT EXECUTE ON FUNCTION ple_current_tenant() TO ple_qti_grader;
GRANT SELECT ON workspace_qti_import_grading TO ple_qti_grader;
ALTER POLICY workspace_qti_import_grading_grader_select
    ON workspace_qti_import_grading TO ple_qti_grader;
