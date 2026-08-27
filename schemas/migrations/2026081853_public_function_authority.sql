-- WP-PROF-G1 / G1-W4: public-function EXECUTE authority baseline.
--
-- PostgreSQL gives PUBLIC EXECUTE on newly created functions by default. Make
-- the migration owner deny that ambient authority and require later capability
-- migrations to grant each caller explicitly (ASVS 8.1.1, 8.2.1, 8.3.1).

BEGIN;

REVOKE EXECUTE ON ALL FUNCTIONS IN SCHEMA public FROM PUBLIC;
-- PostgreSQL applies per-schema defaults in addition to global defaults. Its
-- built-in global PUBLIC EXECUTE therefore requires this global owner policy.
ALTER DEFAULT PRIVILEGES FOR ROLE CURRENT_USER
    REVOKE EXECUTE ON FUNCTIONS FROM PUBLIC;

-- The five-key v1 private loader lacks the active-worker identifier and fence
-- that the later v2 capability requires. It has no compatible caller.
REVOKE ALL ON FUNCTION public.ple_load_accepted_submission_execution_v1(
    uuid, uuid, uuid, uuid, bigint
) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_load_accepted_submission_execution_v1(
    uuid, uuid, uuid, uuid, bigint
) FROM ple_accepted_submission_execution;
REVOKE ALL ON FUNCTION public.ple_load_accepted_submission_execution_v1(
    uuid, uuid, uuid, uuid, bigint
) FROM ple_accepted_submission_execution_fast_path;

-- A CHECK constraint evaluates the byline validator with the inserting role.
-- Native publication uses ple_app, while the adoption materializer invokes the
-- validator directly. The reusable-curriculum broker's pre-existing grant is
-- retained as the third explicit caller.
GRANT EXECUTE ON FUNCTION public.ple_valid_public_byline(text[]) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_valid_public_byline(text[])
    TO ple_curriculum_adoption_broker;

-- Catalog search ranks with word_similarity and admits fuzzy candidates through
-- the <% operator. Both pg_trgm routines require this one app capability.
GRANT EXECUTE ON FUNCTION public.word_similarity(text, text),
    public.word_similarity_op(text, text) TO ple_app;

-- These sealed brokers hash their own durable records. Only catalog usage
-- mints opaque snapshot tokens, so only it receives random-byte generation.
GRANT EXECUTE ON FUNCTION public.digest(bytea, text)
    TO ple_catalog_usage_broker,
       ple_statistics_broker,
       ple_base_course_install_broker,
       ple_base_course_completion_verification_broker,
       ple_problem_curation_broker,
       ple_reusable_curriculum_broker,
       ple_curriculum_adoption_broker;
GRANT EXECUTE ON FUNCTION public.gen_random_bytes(integer)
    TO ple_catalog_usage_broker;

DO $$
DECLARE
    v_legacy_loader regprocedure :=
        'public.ple_load_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,bigint)'
        ::regprocedure;
    v_byline_validator regprocedure :=
        'public.ple_valid_public_byline(text[])'::regprocedure;
    v_word_similarity regprocedure :=
        'public.word_similarity(text,text)'::regprocedure;
    v_word_similarity_operator regprocedure;
    v_catalog_usage_digest regprocedure :=
        'public.digest(bytea,text)'::regprocedure;
    v_catalog_usage_random_bytes regprocedure :=
        'public.gen_random_bytes(integer)'::regprocedure;
    v_probe regprocedure;
BEGIN
    -- acldefault makes a NULL proacl's implicit PUBLIC grant observable. OID 0
    -- is PUBLIC, so this detects both direct and default-derived EXECUTE ACLs.
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
          JOIN pg_catalog.pg_namespace AS namespace_row
            ON namespace_row.oid = procedure_row.pronamespace
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  procedure_row.proacl,
                  pg_catalog.acldefault('f', procedure_row.proowner)
              )
          ) AS privilege_row(grantor, grantee, privilege_type, is_grantable)
         WHERE namespace_row.nspname = 'public'
           AND privilege_row.grantee = 0
           AND privilege_row.privilege_type = 'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'PUBLIC retains EXECUTE on a public-schema function';
    END IF;

    IF pg_catalog.has_function_privilege(
           'ple_accepted_submission_execution', v_legacy_loader, 'EXECUTE'
       )
       OR pg_catalog.has_function_privilege(
           'ple_accepted_submission_execution_fast_path', v_legacy_loader, 'EXECUTE'
       )
    THEN
        RAISE EXCEPTION 'retired accepted-submission v1 loader remains executable';
    END IF;

    IF NOT pg_catalog.has_function_privilege('ple_app', v_byline_validator, 'EXECUTE')
       OR NOT pg_catalog.has_function_privilege(
           'ple_curriculum_adoption_broker', v_byline_validator, 'EXECUTE'
       )
       OR NOT pg_catalog.has_function_privilege(
           'ple_reusable_curriculum_broker', v_byline_validator, 'EXECUTE'
       )
       OR pg_catalog.has_function_privilege(
           'ple_accepted_submission_execution', v_byline_validator, 'EXECUTE'
       )
       OR pg_catalog.has_function_privilege(
           'ple_accepted_submission_execution_fast_path', v_byline_validator, 'EXECUTE'
       )
    THEN
        RAISE EXCEPTION 'public byline validator caller authority is unsafe';
    END IF;

    -- The complete direct non-owner ACL is capability-specific. The expected
    -- entries are intentionally non-grantable, and the global OID-0 check
    -- above verifies that PUBLIC is not a fourth caller.
    IF EXISTS (
        WITH expected(grantee, privilege_type, is_grantable) AS (
            VALUES
                ('ple_app'::regrole::oid, 'EXECUTE'::text, false),
                ('ple_curriculum_adoption_broker'::regrole::oid, 'EXECUTE'::text, false),
                ('ple_reusable_curriculum_broker'::regrole::oid, 'EXECUTE'::text, false)
        ),
        actual(grantee, privilege_type, is_grantable) AS (
            SELECT privilege_row.grantee,
                   privilege_row.privilege_type,
                   privilege_row.is_grantable
              FROM pg_catalog.pg_proc AS procedure_row
              CROSS JOIN LATERAL pg_catalog.aclexplode(
                  COALESCE(
                      procedure_row.proacl,
                      pg_catalog.acldefault('f', procedure_row.proowner)
                  )
              ) AS privilege_row(grantor, grantee, privilege_type, is_grantable)
             WHERE procedure_row.oid = v_byline_validator
               AND privilege_row.grantee <> procedure_row.proowner
        )
        SELECT 1 FROM (
            (SELECT * FROM actual EXCEPT SELECT * FROM expected)
            UNION ALL
            (SELECT * FROM expected EXCEPT SELECT * FROM actual)
        ) AS mismatch
    ) THEN
        RAISE EXCEPTION 'public byline validator ACL is not exact';
    END IF;

    -- The index-supported <% operator must retain the pg_trgm function that
    -- complements the query's word_similarity ranking function.
    SELECT operator_row.oprcode::regprocedure
      INTO v_word_similarity_operator
      FROM pg_catalog.pg_operator AS operator_row
      JOIN pg_catalog.pg_namespace AS namespace_row
        ON namespace_row.oid = operator_row.oprnamespace
     WHERE namespace_row.nspname = 'public'
       AND operator_row.oprname = '<%'
       AND operator_row.oprleft = 'text'::regtype
       AND operator_row.oprright = 'text'::regtype;
    IF v_word_similarity_operator IS DISTINCT FROM
       'public.word_similarity_op(text,text)'::regprocedure
       OR NOT pg_catalog.has_function_privilege(
           'ple_app', v_word_similarity, 'EXECUTE'
       )
       OR NOT pg_catalog.has_function_privilege(
           'ple_app', v_word_similarity_operator, 'EXECUTE'
       )
       OR pg_catalog.has_function_privilege(
           'ple_accepted_submission_execution', v_word_similarity, 'EXECUTE'
       )
       OR pg_catalog.has_function_privilege(
           'ple_accepted_submission_execution_fast_path', v_word_similarity, 'EXECUTE'
       )
       OR pg_catalog.has_function_privilege(
           'ple_accepted_submission_execution', v_word_similarity_operator, 'EXECUTE'
       )
       OR pg_catalog.has_function_privilege(
           'ple_accepted_submission_execution_fast_path',
           v_word_similarity_operator,
           'EXECUTE'
       )
    THEN
        RAISE EXCEPTION 'catalog pg_trgm function authority is unsafe';
    END IF;

    IF EXISTS (
        WITH expected(function_id, grantee, privilege_type, is_grantable) AS (
            VALUES
                (v_word_similarity::oid, 'ple_app'::regrole::oid, 'EXECUTE'::text, false),
                (
                    v_word_similarity_operator::oid,
                    'ple_app'::regrole::oid,
                    'EXECUTE'::text,
                    false
                )
        ),
        actual(function_id, grantee, privilege_type, is_grantable) AS (
            SELECT procedure_row.oid,
                   privilege_row.grantee,
                   privilege_row.privilege_type,
                   privilege_row.is_grantable
              FROM pg_catalog.pg_proc AS procedure_row
              CROSS JOIN LATERAL pg_catalog.aclexplode(
                  COALESCE(
                      procedure_row.proacl,
                      pg_catalog.acldefault('f', procedure_row.proowner)
                  )
              ) AS privilege_row(grantor, grantee, privilege_type, is_grantable)
             WHERE procedure_row.oid IN (
                 v_word_similarity::oid,
                 v_word_similarity_operator::oid
             )
               AND privilege_row.grantee <> procedure_row.proowner
        )
        SELECT 1 FROM (
            (SELECT * FROM actual EXCEPT SELECT * FROM expected)
            UNION ALL
            (SELECT * FROM expected EXCEPT SELECT * FROM actual)
        ) AS mismatch
    ) THEN
        RAISE EXCEPTION 'catalog pg_trgm function ACL is not exact';
    END IF;

    -- Migration 1828's sealed catalog-usage broker calls only these exact
    -- public pgcrypto routines. The broker receives no grant option.
    IF NOT pg_catalog.has_function_privilege(
           'ple_catalog_usage_broker', v_catalog_usage_digest, 'EXECUTE'
       )
       OR NOT pg_catalog.has_function_privilege(
           'ple_catalog_usage_broker', v_catalog_usage_random_bytes, 'EXECUTE'
       )
       OR EXISTS (
           SELECT 1
             FROM unnest(ARRAY[
                 'ple_statistics_broker',
                 'ple_base_course_install_broker',
                 'ple_base_course_completion_verification_broker',
                 'ple_problem_curation_broker',
                 'ple_reusable_curriculum_broker',
                 'ple_curriculum_adoption_broker'
             ]) AS digest_consumer(role_name)
            WHERE NOT pg_catalog.has_function_privilege(
                      digest_consumer.role_name, v_catalog_usage_digest, 'EXECUTE'
                  )
               OR pg_catalog.has_function_privilege(
                      digest_consumer.role_name,
                      v_catalog_usage_random_bytes,
                      'EXECUTE'
                  )
       )
       OR pg_catalog.has_function_privilege('ple_app', v_catalog_usage_digest, 'EXECUTE')
       OR pg_catalog.has_function_privilege(
           'ple_app', v_catalog_usage_random_bytes, 'EXECUTE'
       )
       OR pg_catalog.has_function_privilege(
           'ple_accepted_submission_execution', v_catalog_usage_digest, 'EXECUTE'
       )
       OR pg_catalog.has_function_privilege(
           'ple_accepted_submission_execution',
           v_catalog_usage_random_bytes,
           'EXECUTE'
       )
       OR pg_catalog.has_function_privilege(
           'ple_accepted_submission_execution_fast_path',
           v_catalog_usage_digest,
           'EXECUTE'
       )
       OR pg_catalog.has_function_privilege(
           'ple_accepted_submission_execution_fast_path',
           v_catalog_usage_random_bytes,
           'EXECUTE'
       )
    THEN
        RAISE EXCEPTION 'catalog pgcrypto function authority is unsafe';
    END IF;

    IF EXISTS (
        WITH expected(function_id, grantee, privilege_type, is_grantable) AS (
            VALUES
                (
                    v_catalog_usage_digest::oid,
                    'ple_catalog_usage_broker'::regrole::oid,
                    'EXECUTE'::text,
                    false
                ),
                (
                    v_catalog_usage_digest::oid,
                    'ple_statistics_broker'::regrole::oid,
                    'EXECUTE'::text,
                    false
                ),
                (
                    v_catalog_usage_digest::oid,
                    'ple_base_course_install_broker'::regrole::oid,
                    'EXECUTE'::text,
                    false
                ),
                (
                    v_catalog_usage_digest::oid,
                    'ple_base_course_completion_verification_broker'::regrole::oid,
                    'EXECUTE'::text,
                    false
                ),
                (
                    v_catalog_usage_digest::oid,
                    'ple_problem_curation_broker'::regrole::oid,
                    'EXECUTE'::text,
                    false
                ),
                (
                    v_catalog_usage_digest::oid,
                    'ple_reusable_curriculum_broker'::regrole::oid,
                    'EXECUTE'::text,
                    false
                ),
                (
                    v_catalog_usage_digest::oid,
                    'ple_curriculum_adoption_broker'::regrole::oid,
                    'EXECUTE'::text,
                    false
                ),
                (
                    v_catalog_usage_random_bytes::oid,
                    'ple_catalog_usage_broker'::regrole::oid,
                    'EXECUTE'::text,
                    false
                )
        ),
        actual(function_id, grantee, privilege_type, is_grantable) AS (
            SELECT procedure_row.oid,
                   privilege_row.grantee,
                   privilege_row.privilege_type,
                   privilege_row.is_grantable
              FROM pg_catalog.pg_proc AS procedure_row
              CROSS JOIN LATERAL pg_catalog.aclexplode(
                  COALESCE(
                      procedure_row.proacl,
                      pg_catalog.acldefault('f', procedure_row.proowner)
                  )
              ) AS privilege_row(grantor, grantee, privilege_type, is_grantable)
             WHERE procedure_row.oid IN (
                 v_catalog_usage_digest::oid,
                 v_catalog_usage_random_bytes::oid
             )
               AND privilege_row.grantee <> procedure_row.proowner
        )
        SELECT 1 FROM (
            (SELECT * FROM actual EXCEPT SELECT * FROM expected)
            UNION ALL
            (SELECT * FROM expected EXCEPT SELECT * FROM actual)
        ) AS mismatch
    ) THEN
        RAISE EXCEPTION 'catalog pgcrypto function ACL is not exact';
    END IF;

    -- This function is created by the migration connection principal. It
    -- proves that the CURRENT_USER default privilege applies to future work.
    EXECUTE
        'CREATE FUNCTION public.ple_public_execute_default_probe_2026081853() '
        || 'RETURNS void LANGUAGE sql AS ''SELECT NULL::void''';
    v_probe := pg_catalog.to_regprocedure(
        'public.ple_public_execute_default_probe_2026081853()'
    );
    IF v_probe IS NULL
       OR EXISTS (
           SELECT 1
             FROM pg_catalog.pg_proc AS procedure_row
             CROSS JOIN LATERAL pg_catalog.aclexplode(
                 COALESCE(
                     procedure_row.proacl,
                     pg_catalog.acldefault('f', procedure_row.proowner)
                 )
             ) AS privilege_row(grantor, grantee, privilege_type, is_grantable)
            WHERE procedure_row.oid = v_probe
              AND privilege_row.grantee = 0
              AND privilege_row.privilege_type = 'EXECUTE'
       )
    THEN
        RAISE EXCEPTION 'migration-owner default function privilege keeps PUBLIC EXECUTE';
    END IF;
    EXECUTE 'DROP FUNCTION public.ple_public_execute_default_probe_2026081853()';

    -- Check direct ACL entries as well as effective role privileges. The
    -- is_grantable column stays intentionally unconstrained: either variant
    -- would retain the retired capability and must fail this migration.
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  procedure_row.proacl,
                  pg_catalog.acldefault('f', procedure_row.proowner)
              )
          ) AS privilege_row(grantor, grantee, privilege_type, is_grantable)
         WHERE procedure_row.oid = v_legacy_loader
           AND privilege_row.grantee <> procedure_row.proowner
           AND privilege_row.grantee IN (
               'ple_accepted_submission_execution'::regrole::oid,
               'ple_accepted_submission_execution_fast_path'::regrole::oid
           )
           AND privilege_row.privilege_type = 'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'retired accepted-submission v1 loader ACL is not closed';
    END IF;
END $$;

COMMIT;
