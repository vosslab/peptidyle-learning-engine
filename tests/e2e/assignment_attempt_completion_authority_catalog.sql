-- Assignment Attempt completion trigger authorization acceptance oracle.
-- Executed by the existing PostgreSQL Migration Acceptance Runtime lane.

DO $$
BEGIN
    IF (
        SELECT count(*)
        FROM pg_proc AS procedure
        JOIN pg_namespace AS namespace ON namespace.oid = procedure.pronamespace
        JOIN pg_roles AS owner_role ON owner_role.oid = procedure.proowner
        WHERE namespace.nspname = 'ple_private'
          AND procedure.proname = 'complete_assignment_attempt_after_grading'
          AND pg_get_function_identity_arguments(procedure.oid) = ''
          AND owner_role.rolname = 'ple_private_owner'
          AND procedure.prosecdef
          AND array_to_string(procedure.proconfig, ',') =
              'search_path=pg_catalog, ple_data, ple_private'
          AND has_function_privilege('ple_private_owner', procedure.oid, 'EXECUTE')
          AND NOT has_function_privilege('ple_api_owner', procedure.oid, 'EXECUTE')
          AND NOT has_function_privilege('ple_app', procedure.oid, 'EXECUTE')
          AND NOT has_function_privilege(
              'ple_imathas_question_backend_grading_worker', procedure.oid, 'EXECUTE'
          )
          AND NOT has_function_privilege('public', procedure.oid, 'EXECUTE')
    ) <> 1 OR has_column_privilege(
        'ple_api_owner', 'ple_private.assignment_attempt', 'completed_at', 'UPDATE'
    ) THEN
        RAISE EXCEPTION 'Assignment Attempt completion trigger authority is not exact';
    END IF;
END
$$;
