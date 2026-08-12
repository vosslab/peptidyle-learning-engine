-- Forward-only widening of the secure flat-question grading capability after
-- the presentation-boundary migration has already shipped.

CREATE OR REPLACE FUNCTION public.ple_flat_question_grading_material(
    p_tenant uuid,
    p_problem uuid,
    p_version uuid
)
RETURNS TABLE(key_payload jsonb, key_sha256 character(64))
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public'
AS $$
BEGIN
    IF p_tenant IS NULL OR p_problem IS NULL OR p_version IS NULL
       OR p_tenant <> public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid flat-question grading read capability'
            USING ERRCODE = '22023';
    END IF;

    RETURN QUERY
    SELECT answer.key_payload, answer.key_sha256
      FROM public.answer_key AS answer
      JOIN public.problem_version AS version_row
        ON version_row.problem_id = answer.problem_id
       AND version_row.version_id = answer.version_id
      JOIN public.problem_version_payload AS version_payload
        ON version_payload.problem_id = version_row.problem_id
       AND version_payload.version_id = version_row.version_id
     WHERE answer.problem_id = p_problem
       AND answer.version_id = p_version
       AND version_row.backend = 'native'::text
       AND version_payload.payload #>> '{question,source,backend}' = 'native'::text
       AND version_payload.payload #>> '{question,source,family}' = ANY (
            ARRAY[
                'flat_single_choice_v1',
                'flat_single_choice_v2',
                'flat_multiple_answer_v2',
                'flat_fill_in_v2',
                'flat_multi_fill_in_v2',
                'flat_numeric_v2',
                'flat_matching_v2',
                'flat_ordering_v2',
                'flat_hotspot_v2'
            ]::text[]
       )
       AND (
            version_row.publication_scope = 'public'::text
            OR EXISTS (
                SELECT 1
                  FROM public.catalog_tenant_grant AS grant_row
                 WHERE grant_row.tenant_id = p_tenant
                   AND grant_row.problem_id = version_row.problem_id
                   AND grant_row.version_id = version_row.version_id
            )
       );
END
$$;
