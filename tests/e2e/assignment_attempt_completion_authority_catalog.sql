-- Assignment Attempt submission-boundary authorization acceptance oracle.
-- Executed by the existing PostgreSQL Migration Acceptance Runtime lane.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM pg_trigger AS trigger
         WHERE trigger.tgrelid = 'ple_private.grading_result'::regclass
           AND trigger.tgname = 'grading_result_completes_assignment_attempt'
           AND NOT trigger.tgisinternal
    ) OR (
        SELECT count(*)
        FROM pg_proc AS procedure
        JOIN pg_namespace AS namespace ON namespace.oid = procedure.pronamespace
        JOIN pg_roles AS owner_role ON owner_role.oid = procedure.proowner
        WHERE namespace.nspname = 'ple_api'
          AND procedure.proname = 'finalize_student_assignment_attempt'
          AND pg_get_function_identity_arguments(procedure.oid) = 'p_assignment_attempt_reference_number bigint'
          AND owner_role.rolname = 'ple_private_owner'
          AND procedure.prosecdef
          AND array_to_string(procedure.proconfig, ',') =
              'search_path=pg_catalog, ple_api, ple_data, ple_private'
          AND has_function_privilege('ple_private_owner', procedure.oid, 'EXECUTE')
          AND NOT has_function_privilege('ple_api_owner', procedure.oid, 'EXECUTE')
          AND NOT has_function_privilege('public', procedure.oid, 'EXECUTE')
    ) <> 1 OR NOT has_function_privilege(
        'ple_app', 'ple_api.finalize_student_assignment_attempt(bigint)', 'EXECUTE'
    ) OR has_function_privilege(
        'public', 'ple_api.finalize_student_assignment_attempt(bigint)', 'EXECUTE'
    ) OR has_column_privilege(
        'ple_api_owner', 'ple_private.assignment_attempt', 'completed_at', 'UPDATE'
    ) THEN
        RAISE EXCEPTION 'Assignment Attempt submission completion authority is not exact';
    END IF;
END
$$;
