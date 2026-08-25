-- Reusable assignment-definition capabilities need command-scoped scratch
-- state even when a caller performs several authorized writes in one
-- transaction. Reset the broker-owned temporary relations at each public
-- entry point while retaining the accepted validation and mutation bodies.

BEGIN;

CREATE FUNCTION public.ple_reset_assignment_definition_scratch_v1()
RETURNS void
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
BEGIN
    DROP TABLE IF EXISTS pg_temp.ple_definition_entry;
    DROP TABLE IF EXISTS pg_temp.ple_definition_reference;
    DROP TABLE IF EXISTS pg_temp.ple_unissued_entry;
    DROP TABLE IF EXISTS pg_temp.ple_unissued_reference;
    DROP TABLE IF EXISTS pg_temp.ple_unissued_audience_group;
END
$$;

CREATE OR REPLACE FUNCTION public.ple_create_assignment_definition_v1(
    p_tenant uuid,
    p_actor uuid,
    p_course uuid,
    p_assignment uuid,
    p_payload jsonb,
    p_recalculation_job uuid DEFAULT NULL,
    p_recalculation_max_attempts integer DEFAULT NULL
) RETURNS TABLE (
    assignment_id uuid,
    revision bigint,
    scoring_generation bigint,
    scoring_status text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
BEGIN
    PERFORM public.ple_reset_assignment_definition_scratch_v1();
    PERFORM public.ple_assignment_mutator_require_create_editor(
        p_tenant, p_actor, p_course, p_assignment
    );
    SELECT result.revision, result.scoring_generation, result.scoring_status
      INTO revision, scoring_generation, scoring_status
      FROM public.ple_assignment_definition_apply_v1(
          p_tenant, p_course, p_assignment, p_payload, false,
          p_recalculation_job, p_recalculation_max_attempts
      ) AS result;
    assignment_id := p_assignment;
    RETURN NEXT;
END
$$;

CREATE OR REPLACE FUNCTION public.ple_replace_assignment_definition_v1(
    p_tenant uuid,
    p_actor uuid,
    p_course uuid,
    p_assignment uuid,
    p_expected_revision bigint,
    p_payload jsonb,
    p_recalculation_job uuid,
    p_recalculation_max_attempts integer
) RETURNS TABLE (
    revision bigint,
    scoring_generation bigint,
    scoring_status text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
BEGIN
    PERFORM public.ple_reset_assignment_definition_scratch_v1();
    PERFORM public.ple_assignment_mutator_require_editor(
        p_tenant, p_actor, p_course, p_assignment, p_expected_revision
    );
    RETURN QUERY
    SELECT result.revision, result.scoring_generation, result.scoring_status
      FROM public.ple_assignment_definition_apply_v1(
          p_tenant, p_course, p_assignment, p_payload, true,
          p_recalculation_job, p_recalculation_max_attempts
      ) AS result;
END
$$;

ALTER FUNCTION public.ple_replace_unissued_assignment_definition_v1(
    uuid, uuid, uuid, uuid, bigint, jsonb
) RENAME TO ple_replace_unissued_assignment_definition_impl_v1;

CREATE FUNCTION public.ple_replace_unissued_assignment_definition_v1(
    p_tenant uuid,
    p_actor uuid,
    p_course uuid,
    p_assignment uuid,
    p_expected_revision bigint,
    p_payload jsonb
) RETURNS TABLE (
    outcome text,
    revision bigint,
    scoring_generation bigint,
    scoring_status text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
BEGIN
    PERFORM public.ple_reset_assignment_definition_scratch_v1();
    RETURN QUERY
    SELECT result.outcome, result.revision,
           result.scoring_generation, result.scoring_status
      FROM public.ple_replace_unissued_assignment_definition_impl_v1(
          p_tenant, p_actor, p_course, p_assignment,
          p_expected_revision, p_payload
      ) AS result;
END
$$;

ALTER FUNCTION public.ple_reset_assignment_definition_scratch_v1()
    OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_create_assignment_definition_v1(
    uuid, uuid, uuid, uuid, jsonb, uuid, integer
) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_replace_assignment_definition_v1(
    uuid, uuid, uuid, uuid, bigint, jsonb, uuid, integer
) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_replace_unissued_assignment_definition_v1(
    uuid, uuid, uuid, uuid, bigint, jsonb
) OWNER TO ple_assignment_mutator_broker;

REVOKE ALL ON FUNCTION public.ple_reset_assignment_definition_scratch_v1()
    FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_replace_unissued_assignment_definition_impl_v1(
    uuid, uuid, uuid, uuid, bigint, jsonb
) FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_create_assignment_definition_v1(
    uuid, uuid, uuid, uuid, jsonb, uuid, integer
) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_replace_assignment_definition_v1(
    uuid, uuid, uuid, uuid, bigint, jsonb, uuid, integer
) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_replace_unissued_assignment_definition_v1(
    uuid, uuid, uuid, uuid, bigint, jsonb
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION public.ple_create_assignment_definition_v1(
    uuid, uuid, uuid, uuid, jsonb, uuid, integer
) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_replace_assignment_definition_v1(
    uuid, uuid, uuid, uuid, bigint, jsonb, uuid, integer
) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_replace_unissued_assignment_definition_v1(
    uuid, uuid, uuid, uuid, bigint, jsonb
) TO ple_app;

DO $$
BEGIN
    IF has_function_privilege(
           'public',
           'public.ple_reset_assignment_definition_scratch_v1()'::regprocedure,
           'EXECUTE'
       )
       OR has_function_privilege(
           'ple_app',
           'public.ple_reset_assignment_definition_scratch_v1()'::regprocedure,
           'EXECUTE'
       )
       OR has_function_privilege(
           'public',
           'public.ple_replace_unissued_assignment_definition_impl_v1(uuid,uuid,uuid,uuid,bigint,jsonb)'::regprocedure,
           'EXECUTE'
       )
       OR has_function_privilege(
           'ple_app',
           'public.ple_replace_unissued_assignment_definition_impl_v1(uuid,uuid,uuid,uuid,bigint,jsonb)'::regprocedure,
           'EXECUTE'
       )
       OR NOT has_function_privilege(
           'ple_app',
           'public.ple_create_assignment_definition_v1(uuid,uuid,uuid,uuid,jsonb,uuid,integer)'::regprocedure,
           'EXECUTE'
       )
       OR NOT has_function_privilege(
           'ple_app',
           'public.ple_replace_assignment_definition_v1(uuid,uuid,uuid,uuid,bigint,jsonb,uuid,integer)'::regprocedure,
           'EXECUTE'
       )
       OR NOT has_function_privilege(
           'ple_app',
           'public.ple_replace_unissued_assignment_definition_v1(uuid,uuid,uuid,uuid,bigint,jsonb)'::regprocedure,
           'EXECUTE'
       ) THEN
        RAISE EXCEPTION 'assignment definition scratch capability catalog is unsafe';
    END IF;
END
$$;

COMMIT;
