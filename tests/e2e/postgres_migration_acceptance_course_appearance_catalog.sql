-- Course Appearance catalog and Course-member navigation resolver acceptance.

DO $$
BEGIN
    IF to_regprocedure('ple_api.read_course_summary(uuid)') IS NULL
       OR NOT has_function_privilege(
           'ple_app', 'ple_api.read_course_summary(uuid)', 'EXECUTE'
       )
       OR has_function_privilege(
           'public', 'ple_api.read_course_summary(uuid)', 'EXECUTE'
       )
       OR COALESCE((
           SELECT array_to_string(proconfig, ',')
             FROM pg_proc
            WHERE oid = 'ple_api.read_course_summary(uuid)'::regprocedure
       ), '') NOT LIKE '%search_path=pg_catalog, ple_api, ple_data%' THEN
        RAISE EXCEPTION 'Course Summary lacks its exact session-authorized member reader';
    END IF;
END
$$;

DO $$
BEGIN
    IF to_regprocedure('ple_api.resolve_course_navigation(bigint)') IS NULL
       OR NOT has_function_privilege(
           'ple_app', 'ple_api.resolve_course_navigation(bigint)', 'EXECUTE'
       )
       OR has_function_privilege(
           'public', 'ple_api.resolve_course_navigation(bigint)', 'EXECUTE'
       )
       OR COALESCE((
           SELECT array_to_string(proconfig, ',')
             FROM pg_proc
            WHERE oid = 'ple_api.resolve_course_navigation(bigint)'::regprocedure
       ), '') NOT LIKE '%search_path=pg_catalog, ple_api, ple_data%' THEN
        RAISE EXCEPTION 'Course navigation lacks its exact session-authorized member resolver';
    END IF;
END
$$;

-- Keep durable work behind forced RLS; the saga acceptance exercises its operations.
DO $$
BEGIN
    IF NOT (SELECT relrowsecurity AND relforcerowsecurity
              FROM pg_class WHERE oid = 'ple_private.course_banner_work'::regclass) THEN
        RAISE EXCEPTION 'Course Banner work must retain forced RLS';
    END IF;
END
$$;
