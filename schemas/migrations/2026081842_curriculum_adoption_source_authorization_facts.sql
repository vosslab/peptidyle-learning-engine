-- WP-PROF-B2: private source authorization and source facts.
--
-- 1840 intentionally remains the public PBI01 shell.  1847 installs the
-- public dispatcher after the materializers can consume this compiler.
-- This migration owns only closed input validation and B1 source observations.
-- Teaching/current-state facts begin in 1843; qmodel remains the only owner of
-- semantic normalization, canonical bytes/digests, and DST handling.

BEGIN;

-- B1 is accepted and checksum-immutable.  Correct its relative-time validator
-- here so both upgraded and fresh databases receive the same forward repair.
CREATE OR REPLACE FUNCTION public.ple_reusable_definition_v1_is_valid(p_definition jsonb)
RETURNS boolean LANGUAGE plpgsql IMMUTABLE PARALLEL SAFE SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE entry_value jsonb; moment_value jsonb; candidate_count integer;
BEGIN
    IF jsonb_typeof(p_definition) <> 'object'
       OR NOT p_definition ?& ARRAY['title', 'instructions', 'entries', 'defaults', 'schedule']
       OR (SELECT count(*) FROM jsonb_object_keys(p_definition)) <> 5
       OR jsonb_typeof(p_definition->'title') <> 'string'
       OR char_length(p_definition->>'title') NOT BETWEEN 1 AND 200
       OR p_definition->>'title' <> btrim(p_definition->>'title')
       OR jsonb_typeof(p_definition->'instructions') <> 'string'
       OR jsonb_typeof(p_definition->'entries') <> 'array'
       OR jsonb_array_length(p_definition->'entries') NOT BETWEEN 1 AND 1024
       OR jsonb_typeof(p_definition->'defaults') <> 'object'
       OR NOT p_definition->'defaults' ?& ARRAY['timeLimitSeconds', 'attemptLimit', 'lateSubmission', 'deadlineBehavior', 'runPolicies', 'learnerDisclosure']
       OR (SELECT count(*) FROM jsonb_object_keys(p_definition->'defaults')) <> 6
       OR p_definition->'defaults'->>'lateSubmission' NOT IN ('accept', 'markLate', 'reject')
       OR p_definition->'defaults'->>'deadlineBehavior' <> 'autoSubmit'
       OR jsonb_typeof(p_definition->'defaults'->'runPolicies') <> 'object'
       OR jsonb_typeof(p_definition->'defaults'->'learnerDisclosure') <> 'object'
       OR jsonb_typeof(p_definition->'schedule') <> 'object'
       OR NOT p_definition->'schedule' ?& ARRAY['availableAt', 'dueAt', 'closesAt']
       OR (SELECT count(*) FROM jsonb_object_keys(p_definition->'schedule')) <> 3 THEN RETURN false; END IF;
    FOR moment_value IN SELECT value FROM jsonb_each(p_definition->'schedule') LOOP
        IF moment_value <> 'null'::jsonb AND (jsonb_typeof(moment_value) <> 'object' OR NOT moment_value ?& ARRAY['dayOffset', 'localTime'] OR (SELECT count(*) FROM jsonb_object_keys(moment_value)) <> 2 OR jsonb_typeof(moment_value->'dayOffset') <> 'number' OR (moment_value->>'dayOffset')::numeric <> trunc((moment_value->>'dayOffset')::numeric) OR (moment_value->>'dayOffset')::numeric NOT BETWEEN -2147483648 AND 2147483647 OR moment_value->>'localTime' !~ '^([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]\.[0-9]{3}$') THEN RETURN false; END IF;
    END LOOP;
    FOR entry_value IN SELECT value FROM jsonb_array_elements(p_definition->'entries') LOOP
        IF jsonb_typeof(entry_value) <> 'object' OR entry_value->>'kind' NOT IN ('fixed', 'pool') THEN RETURN false; END IF;
        IF entry_value->>'kind' = 'fixed' AND (NOT entry_value ?& ARRAY['kind','questionId','pointsPossible','scoringMode'] OR (SELECT count(*) FROM jsonb_object_keys(entry_value)) <> 4 OR entry_value->>'questionId' !~ '^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$' OR jsonb_typeof(entry_value->'pointsPossible') <> 'string' OR entry_value->>'pointsPossible' !~ '^[0-9]{1,10}(\.[0-9]{1,4})?$' OR (entry_value->>'pointsPossible')::numeric > 1000000000.9999 OR entry_value->>'scoringMode' NOT IN ('normal','fullCredit','extraCredit','excluded')) THEN RETURN false; END IF;
        IF entry_value->>'kind' = 'pool' THEN
            SELECT count(*) INTO candidate_count FROM jsonb_array_elements_text(entry_value->'candidates');
            IF NOT entry_value ?& ARRAY['kind','candidates','drawCount','pointsPerItem','ordering','algorithm'] OR (SELECT count(*) FROM jsonb_object_keys(entry_value)) <> 6 OR jsonb_typeof(entry_value->'candidates') <> 'array' OR candidate_count NOT BETWEEN 1 AND 1024 OR (entry_value->>'drawCount')::integer NOT BETWEEN 1 AND candidate_count OR jsonb_typeof(entry_value->'pointsPerItem') <> 'string' OR entry_value->>'pointsPerItem' !~ '^[0-9]{1,10}(\.[0-9]{1,4})?$' OR (entry_value->>'pointsPerItem')::numeric > 1000000000.9999 OR entry_value->>'ordering' NOT IN ('candidateOrder','randomized') OR entry_value->>'algorithm' <> 'v1' OR EXISTS (SELECT 1 FROM jsonb_array_elements_text(entry_value->'candidates') AS candidate(question_id) WHERE candidate.question_id !~ '^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$') OR (SELECT count(DISTINCT candidate.question_id) FROM jsonb_array_elements_text(entry_value->'candidates') AS candidate(question_id)) <> candidate_count THEN RETURN false; END IF;
        END IF;
    END LOOP;
    RETURN true;
END $$;

-- The source compiler may read only published catalog pins visible to its tenant.
CREATE POLICY curriculum_adoption_catalog_fact_read ON public.catalog_search_document
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (
        lifecycle = 'published' AND (
            publication_scope = 'public' OR EXISTS (
                SELECT 1 FROM public.catalog_tenant_grant AS grant_row
                 WHERE grant_row.tenant_id = public.ple_current_tenant()
                   AND grant_row.problem_id = catalog_search_document.problem_id
                   AND grant_row.version_id = catalog_search_document.version_id
            )
        )
    );
CREATE POLICY curriculum_adoption_catalog_grant_fact_read ON public.catalog_tenant_grant
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());

GRANT SELECT ON public.catalog_search_document, public.catalog_tenant_grant
    TO ple_curriculum_adoption_broker;

-- Validate all nested browser structures before a caller resolves a route or
-- obtains a source read.  Exact witness equality remains a locked-read check.
CREATE FUNCTION public.ple_cac_validate_request_v1(p_kind text, p_request jsonb)
RETURNS void LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_source jsonb; v_witness jsonb; v_term jsonb; v_replacement jsonb;
DECLARE v_assignment jsonb; v_position jsonb; v_key text;
BEGIN
    IF p_kind = 'inspectImports' THEN
        PERFORM public.ple_curriculum_adoption_route_number_v1(p_request, 'C');
        RETURN;
    END IF;
    IF p_request IS NULL OR jsonb_typeof(p_request) <> 'object'
       OR octet_length(p_request::text) > 524288 THEN
        RAISE EXCEPTION 'curriculum adoption request is invalid' USING ERRCODE = '22023';
    END IF;
    IF p_request ? 'source' THEN
        v_source := p_request->'source';
        IF jsonb_typeof(v_source) <> 'object' THEN
            RAISE EXCEPTION 'curriculum adoption source is invalid' USING ERRCODE = '22023';
        END IF;
        IF v_source ? 'kind' THEN
            IF v_source->>'kind' NOT IN ('blueprint', 'alpha')
               OR EXISTS (SELECT 1 FROM jsonb_object_keys(v_source) AS key
                            WHERE key NOT IN ('kind', 'reference', 'revision',
                                              'moduleIndex', 'assignmentIndex')) THEN
                RAISE EXCEPTION 'curriculum adoption assignment source is invalid'
                    USING ERRCODE = '22023';
            END IF;
            IF v_source->>'kind' = 'blueprint'
               AND (v_source ? 'moduleIndex' OR v_source ? 'assignmentIndex') THEN
                RAISE EXCEPTION 'curriculum adoption assignment source is invalid'
                    USING ERRCODE = '22023';
            END IF;
            IF v_source->>'kind' = 'alpha'
               AND NOT v_source ?& ARRAY['moduleIndex', 'assignmentIndex'] THEN
                RAISE EXCEPTION 'curriculum adoption assignment source is invalid'
                    USING ERRCODE = '22023';
            END IF;
        ELSIF EXISTS (SELECT 1 FROM jsonb_object_keys(v_source) AS key
                       WHERE key NOT IN ('reference', 'revision')) THEN
            RAISE EXCEPTION 'curriculum adoption source is invalid' USING ERRCODE = '22023';
        END IF;
        IF NOT v_source ?& ARRAY['reference', 'revision']
           OR jsonb_typeof(v_source->'reference') <> 'string'
           OR jsonb_typeof(v_source->'revision') <> 'string'
           OR v_source->>'revision' !~ '^[1-9][0-9]{0,18}$' THEN
            RAISE EXCEPTION 'curriculum adoption source is invalid' USING ERRCODE = '22023';
        END IF;
        IF coalesce(v_source->>'kind', '') = 'blueprint'
           OR v_source->>'reference' LIKE 'BP-%' THEN
            PERFORM public.ple_curriculum_adoption_route_number_v1(v_source->'reference', 'BP');
        ELSE
            PERFORM public.ple_curriculum_adoption_route_number_v1(v_source->'reference', 'AC');
        END IF;
        IF v_source ? 'moduleIndex' AND (
            jsonb_typeof(v_source->'moduleIndex') <> 'number'
            OR v_source->>'moduleIndex' !~ '^(0|[1-9][0-9]{0,3})$'
            OR (v_source->>'moduleIndex')::integer >= 1024
        ) THEN
            RAISE EXCEPTION 'curriculum adoption source position is invalid' USING ERRCODE = '22023';
        END IF;
        IF v_source ? 'assignmentIndex' AND (
            jsonb_typeof(v_source->'assignmentIndex') <> 'number'
            OR v_source->>'assignmentIndex' !~ '^(0|[1-9][0-9]{0,3})$'
            OR (v_source->>'assignmentIndex')::integer >= 1024
        ) THEN
            RAISE EXCEPTION 'curriculum adoption source position is invalid' USING ERRCODE = '22023';
        END IF;
    END IF;
    IF p_request ? 'course' THEN
        PERFORM public.ple_curriculum_adoption_route_number_v1(p_request->'course', 'C');
    END IF;
    IF p_request ? 'assignment' THEN
        v_assignment := p_request->'assignment';
        PERFORM public.ple_curriculum_adoption_closed_object_v1(
            v_assignment, ARRAY['assignment', 'revision'], 4096
        );
        IF NOT v_assignment ?& ARRAY['assignment', 'revision']
           OR jsonb_typeof(v_assignment->'revision') <> 'string'
           OR v_assignment->>'revision' !~ '^[1-9][0-9]{0,18}$' THEN
            RAISE EXCEPTION 'curriculum adoption assignment witness is invalid'
                USING ERRCODE = '22023';
        END IF;
        PERFORM public.ple_curriculum_adoption_route_number_v1(v_assignment->'assignment', 'A');
    END IF;
    v_witness := coalesce(p_request->'witness', p_request->'previewWitness');
    IF v_witness IS NOT NULL THEN
        PERFORM public.ple_curriculum_adoption_closed_object_v1(
            v_witness, ARRAY['course', 'scheduleRevision', 'assignmentRevisions'], 262144
        );
        IF NOT v_witness ?& ARRAY['course', 'scheduleRevision', 'assignmentRevisions']
           OR jsonb_typeof(v_witness->'scheduleRevision') <> 'string'
           OR v_witness->>'scheduleRevision' !~ '^[1-9][0-9]{0,18}$'
           OR jsonb_typeof(v_witness->'assignmentRevisions') <> 'array'
           OR jsonb_array_length(v_witness->'assignmentRevisions') > 1024 THEN
            RAISE EXCEPTION 'curriculum adoption course witness is invalid' USING ERRCODE = '22023';
        END IF;
        PERFORM public.ple_curriculum_adoption_route_number_v1(v_witness->'course', 'C');
        FOR v_assignment IN SELECT value FROM jsonb_array_elements(v_witness->'assignmentRevisions')
        LOOP
            PERFORM public.ple_curriculum_adoption_closed_object_v1(
                v_assignment, ARRAY['assignment', 'revision'], 4096
            );
            IF NOT v_assignment ?& ARRAY['assignment', 'revision']
               OR jsonb_typeof(v_assignment->'revision') <> 'string'
               OR v_assignment->>'revision' !~ '^[1-9][0-9]{0,18}$' THEN
                RAISE EXCEPTION 'curriculum adoption course witness is invalid'
                    USING ERRCODE = '22023';
            END IF;
            PERFORM public.ple_curriculum_adoption_route_number_v1(v_assignment->'assignment', 'A');
        END LOOP;
    END IF;
    IF p_request ? 'targetTerm' THEN
        v_term := p_request->'targetTerm';
        PERFORM public.ple_curriculum_adoption_closed_object_v1(
            v_term, ARRAY['startDate', 'endDate', 'timeZone'], 4096
        );
        IF NOT v_term ?& ARRAY['startDate', 'endDate', 'timeZone']
           OR jsonb_typeof(v_term->'startDate') <> 'string'
           OR jsonb_typeof(v_term->'endDate') <> 'string'
           OR jsonb_typeof(v_term->'timeZone') <> 'string'
           OR char_length(v_term->>'timeZone') NOT BETWEEN 1 AND 255
           OR v_term->>'timeZone' ~ '[[:space:]]' THEN
            RAISE EXCEPTION 'curriculum adoption target term is invalid' USING ERRCODE = '22023';
        END IF;
        BEGIN
            IF (v_term->>'startDate')::date > (v_term->>'endDate')::date THEN
                RAISE EXCEPTION 'curriculum adoption target term is invalid' USING ERRCODE = '22023';
            END IF;
        EXCEPTION WHEN datetime_field_overflow OR invalid_datetime_format THEN
            RAISE EXCEPTION 'curriculum adoption target term is invalid' USING ERRCODE = '22023';
        END;
    END IF;
    IF p_request ? 'title' AND (
        jsonb_typeof(p_request->'title') <> 'string'
        OR char_length(p_request->>'title') NOT BETWEEN 1 AND 200
        OR p_request->>'title' <> btrim(p_request->>'title')
    ) THEN
        RAISE EXCEPTION 'curriculum adoption title is invalid' USING ERRCODE = '22023';
    END IF;
    IF p_request ? 'replacements' THEN
        IF jsonb_typeof(p_request->'replacements') <> 'array'
           OR jsonb_array_length(p_request->'replacements') > 8192 THEN
            RAISE EXCEPTION 'curriculum adoption replacements are invalid' USING ERRCODE = '22023';
        END IF;
        FOR v_replacement IN SELECT value FROM jsonb_array_elements(p_request->'replacements')
        LOOP
            PERFORM public.ple_curriculum_adoption_closed_object_v1(
                v_replacement, ARRAY['position', 'question'], 4096
            );
            v_position := v_replacement->'position';
            PERFORM public.ple_curriculum_adoption_closed_object_v1(
                v_position, ARRAY['moduleIndex', 'assignmentIndex', 'entryIndex', 'candidateIndex'], 4096
            );
            IF NOT v_replacement ?& ARRAY['position', 'question']
               OR jsonb_typeof(v_replacement->'question') <> 'string'
               OR v_replacement->>'question' !~ '^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$'
               OR NOT v_position ?& ARRAY['moduleIndex','assignmentIndex','entryIndex','candidateIndex']
               OR jsonb_typeof(v_position->'assignmentIndex') <> 'number'
               OR jsonb_typeof(v_position->'entryIndex') <> 'number'
               OR v_position->>'assignmentIndex' !~ '^(0|[1-9][0-9]{0,3})$'
               OR v_position->>'entryIndex' !~ '^(0|[1-9][0-9]{0,3})$'
               OR (v_position->>'assignmentIndex')::integer >= 1024
               OR (v_position->>'entryIndex')::integer >= 1024
               OR (v_position->'moduleIndex' <> 'null'::jsonb AND (
                   jsonb_typeof(v_position->'moduleIndex') <> 'number'
                   OR v_position->>'moduleIndex' !~ '^(0|[1-9][0-9]{0,3})$'
                   OR (v_position->>'moduleIndex')::integer >= 1024
               ))
               OR (v_position->'candidateIndex' <> 'null'::jsonb AND (
                   jsonb_typeof(v_position->'candidateIndex') <> 'number'
                   OR v_position->>'candidateIndex' !~ '^(0|[1-9][0-9]{0,3})$'
                   OR (v_position->>'candidateIndex')::integer >= 1024
               )) THEN
                RAISE EXCEPTION 'curriculum adoption replacements are invalid' USING ERRCODE = '22023';
            END IF;
        END LOOP;
        IF EXISTS (
            SELECT 1
              FROM jsonb_array_elements(p_request->'replacements') AS replacement(value)
             GROUP BY replacement.value->'position'
            HAVING count(*) > 1
        ) THEN
            RAISE EXCEPTION 'curriculum adoption replacement positions are not unique'
                USING ERRCODE = '22023';
        END IF;
    END IF;
    IF p_request ? 'importRevision' AND (
        jsonb_typeof(p_request->'importRevision') <> 'string'
        OR p_request->>'importRevision' !~ '^[1-9][0-9]{0,18}$'
    ) THEN
        RAISE EXCEPTION 'curriculum adoption import revision is invalid' USING ERRCODE = '22023';
    END IF;
    IF p_kind IN ('applyForkAlpha', 'applyBlueprintInstantiation', 'applyAlphaInstantiation',
                  'applyCourseRollover', 'applyCourseTermShift', 'applyAssignmentFastForward',
                  'createSourceDerivedAssignment')
       AND (jsonb_typeof(p_request->'idempotencyKey') <> 'string'
            OR p_request->>'idempotencyKey' !~ '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$') THEN
        RAISE EXCEPTION 'curriculum adoption idempotency key is invalid' USING ERRCODE = '22023';
    END IF;
    IF p_kind = 'reconcile' THEN
        PERFORM public.ple_curriculum_adoption_closed_object_v1(
            p_request->'receipt', ARRAY['idempotencyKey'], 4096
        );
        IF jsonb_typeof(p_request->'receipt'->'idempotencyKey') <> 'string'
           OR p_request->'receipt'->>'idempotencyKey' !~ '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$' THEN
            RAISE EXCEPTION 'curriculum adoption receipt is invalid' USING ERRCODE = '22023';
        END IF;
    END IF;
END $$;

-- B1 getters perform the source authorization.  The advisory binding is also
-- taken by later B2 writers before their final revision recheck; it gives the
-- snapshot a stable source aggregate lock without granting B2 direct B1 rows.
CREATE FUNCTION public.ple_cac_reusable_document_v1(
    p_tenant uuid, p_session character(64), p_source jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_reference integer; v_document jsonb; v_prefix text;
BEGIN
    IF coalesce(p_source->>'kind', '') = 'blueprint'
       OR p_source->>'reference' LIKE 'BP-%' THEN
        v_prefix := 'BP';
    ELSE
        v_prefix := 'AC';
    END IF;
    v_reference := public.ple_curriculum_adoption_route_number_v1(p_source->'reference', v_prefix);
    PERFORM pg_advisory_xact_lock(
        hashtextextended('ple.curriculum-adoption.source.' || v_prefix || ':' || v_reference::text, 0)
    );
    IF v_prefix = 'BP' THEN
        v_document := public.ple_get_curriculum_blueprint_v1(p_tenant, p_session, v_reference);
    ELSE
        v_document := public.ple_get_curriculum_alpha_v1(p_tenant, p_session, v_reference);
    END IF;
    IF v_document IS NULL OR v_document->>'revision' <> p_source->>'revision' THEN
        RAISE EXCEPTION 'curriculum adoption source witness is stale or unavailable'
            USING ERRCODE = 'PBC01';
    END IF;
    RETURN v_document;
END $$;

CREATE FUNCTION public.ple_cac_semantic_definition_v1(p_definition jsonb)
RETURNS jsonb LANGUAGE sql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
    SELECT jsonb_build_object(
        'kind', 'assignment',
        'definition', (p_definition - 'entries') || jsonb_build_object('entries', coalesce((
            SELECT jsonb_agg(CASE entry.value->>'kind'
                WHEN 'fixed' THEN jsonb_build_object(
                    'kind', 'fixed',
                    'reference', jsonb_build_object(
                        'problem', entry.value->>'problemId',
                        'version', entry.value->>'versionId'
                    ),
                    'pointsPossible', entry.value->>'pointsPossible',
                    'scoringMode', entry.value->>'scoringMode'
                )
                ELSE jsonb_build_object(
                    'kind', 'pool',
                    'candidates', coalesce((
                        SELECT jsonb_agg(jsonb_build_object(
                            'problem', candidate.value->>'problemId',
                            'version', candidate.value->>'versionId'
                        ) ORDER BY candidate.ordinality)
                          FROM jsonb_array_elements(entry.value->'candidates') WITH ORDINALITY
                               AS candidate(value, ordinality)
                    ), '[]'::jsonb),
                    'drawCount', entry.value->'drawCount',
                    'pointsPerItem', entry.value->>'pointsPerItem',
                    'ordering', entry.value->'ordering',
                    'algorithm', entry.value->'algorithm'
                ) END ORDER BY entry.ordinality)
              FROM jsonb_array_elements(p_definition->'entries') WITH ORDINALITY
                   AS entry(value, ordinality)
        ), '[]'::jsonb))
    )
$$;

CREATE FUNCTION public.ple_cac_semantic_alpha_v1(p_document jsonb)
RETURNS jsonb LANGUAGE sql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
    SELECT jsonb_build_object(
        'kind', 'course',
        'title', p_document->'title',
        'modules', coalesce((
            SELECT jsonb_agg(jsonb_build_object(
                'label', module.value->'label',
                'assignments', coalesce((
                    SELECT jsonb_agg(public.ple_cac_semantic_definition_v1(definition.value->'definition')
                                          -> 'definition' ORDER BY definition.ordinality)
                      FROM jsonb_array_elements(module.value->'definitions') WITH ORDINALITY
                           AS definition(value, ordinality)
                ), '[]'::jsonb)
            ) ORDER BY module.ordinality)
              FROM jsonb_array_elements(p_document->'modules') WITH ORDINALITY AS module(value, ordinality)
        ), '[]'::jsonb)
    )
$$;

CREATE FUNCTION public.ple_cac_resolved_replacements_v1(p_replacements jsonb)
RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_resolved jsonb; v_count integer; v_requested integer;
BEGIN
    v_requested := jsonb_array_length(coalesce(p_replacements, '[]'::jsonb));
    SELECT coalesce(jsonb_agg(jsonb_build_object(
        'position', replacement.value->'position',
        'reference', jsonb_build_object('problem', document.problem_id, 'version', document.version_id)
    ) ORDER BY
        coalesce((replacement.value#>>'{position,moduleIndex}')::integer, -1),
        (replacement.value#>>'{position,assignmentIndex}')::integer,
        (replacement.value#>>'{position,entryIndex}')::integer,
        coalesce((replacement.value#>>'{position,candidateIndex}')::integer, -1)), '[]'::jsonb),
           count(*) INTO v_resolved, v_count
      FROM jsonb_array_elements(coalesce(p_replacements, '[]'::jsonb)) AS replacement(value)
      JOIN public.catalog_search_document AS document
        ON document.question_id = replace(replacement.value->>'question', '-', '')
       AND document.lifecycle = 'published';
    IF v_count <> v_requested THEN
        RAISE EXCEPTION 'curriculum adoption replacement is unavailable' USING ERRCODE = 'PBC01';
    END IF;
    RETURN v_resolved;
END $$;

CREATE FUNCTION public.ple_cac_pin_availability_v1(
    p_definition jsonb, p_module integer, p_assignment integer, p_replacements jsonb
) RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_pin jsonb; v_choices jsonb;
BEGIN
    WITH pins AS (
        SELECT entry.ordinality - 1 AS entry_index, NULL::integer AS candidate_index,
               entry.value->>'problemId' AS problem_id, entry.value->>'versionId' AS version_id,
               coalesce((entry.value#>>'{catalog,selectionAvailable}')::boolean, false) AS available
          FROM jsonb_array_elements(p_definition->'entries') WITH ORDINALITY AS entry(value, ordinality)
         WHERE entry.value->>'kind' = 'fixed'
        UNION ALL
        SELECT entry.ordinality - 1, candidate.ordinality - 1,
               candidate.value->>'problemId', candidate.value->>'versionId',
               coalesce((candidate.value#>>'{catalog,selectionAvailable}')::boolean, false)
          FROM jsonb_array_elements(p_definition->'entries') WITH ORDINALITY AS entry(value, ordinality)
          CROSS JOIN LATERAL jsonb_array_elements(entry.value->'candidates') WITH ORDINALITY
              AS candidate(value, ordinality)
         WHERE entry.value->>'kind' = 'pool'
    )
    SELECT jsonb_build_object(
        'position', jsonb_build_object(
            'moduleIndex', to_jsonb(p_module), 'assignmentIndex', p_assignment,
            'entryIndex', entry_index, 'candidateIndex', to_jsonb(candidate_index)
        ),
        'reference', jsonb_build_object('problem', problem_id, 'version', version_id)
    ) INTO v_pin
      FROM pins
     WHERE NOT available
       AND NOT EXISTS (
            SELECT 1 FROM jsonb_array_elements(coalesce(p_replacements, '[]'::jsonb)) AS replacement(value)
             WHERE replacement.value->'position' = jsonb_build_object(
                 'moduleIndex', to_jsonb(p_module), 'assignmentIndex', p_assignment,
                 'entryIndex', entry_index, 'candidateIndex', to_jsonb(candidate_index)
             )
       )
     ORDER BY entry_index, candidate_index NULLS FIRST
     LIMIT 1;
    IF v_pin IS NULL THEN
        RETURN jsonb_build_object('kind', 'available');
    END IF;
    SELECT coalesce(jsonb_agg(
        substr(document.question_id::text, 1, 3) || '-' || substr(document.question_id::text, 4)
        ORDER BY document.question_id
    ), '[]'::jsonb)
      INTO v_choices
      FROM (
          SELECT question_id
            FROM public.catalog_search_document
           WHERE lifecycle = 'published'
           ORDER BY question_id
           LIMIT 32
      ) AS document;
    IF jsonb_array_length(v_choices) = 0 THEN
        RAISE EXCEPTION 'curriculum adoption replacement choices are unavailable' USING ERRCODE = 'PBC01';
    END IF;
    RETURN jsonb_build_object('kind', 'unavailable', 'pin', v_pin, 'candidates', v_choices);
END $$;

CREATE FUNCTION public.ple_cac_source_facts_v1(
    p_tenant uuid, p_session character(64), p_source jsonb, p_replacements jsonb,
    p_target_term jsonb, p_assignment_source boolean
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_document jsonb; v_definition jsonb; v_binding jsonb; v_semantic jsonb; v_pin_availability jsonb;
DECLARE v_module integer; v_assignment integer; v_is_blueprint boolean;
BEGIN
    v_document := public.ple_cac_reusable_document_v1(p_tenant, p_session, p_source);
    v_is_blueprint := coalesce(p_source->>'kind', '') = 'blueprint'
        OR p_source->>'reference' LIKE 'BP-%';
    IF v_is_blueprint THEN
        v_definition := v_document->'definition';
        v_binding := jsonb_build_object('kind', CASE WHEN p_assignment_source THEN 'assignment' ELSE 'blueprint' END,
            'source', CASE WHEN p_assignment_source THEN
                jsonb_build_object('kind', 'blueprint', 'reference', p_source->'reference', 'revision', p_source->'revision')
            ELSE jsonb_build_object('reference', p_source->'reference', 'revision', p_source->'revision') END);
        v_module := NULL; v_assignment := 0;
    ELSIF p_assignment_source THEN
        v_module := (p_source->>'moduleIndex')::integer;
        v_assignment := (p_source->>'assignmentIndex')::integer;
        SELECT definition.value->'definition' INTO v_definition
          FROM jsonb_array_elements(v_document->'modules') WITH ORDINALITY AS module(value, module_ordinality)
          CROSS JOIN LATERAL jsonb_array_elements(module.value->'definitions') WITH ORDINALITY
              AS definition(value, definition_ordinality)
         WHERE module_ordinality - 1 = v_module AND definition_ordinality - 1 = v_assignment;
        IF v_definition IS NULL THEN
            RAISE EXCEPTION 'curriculum adoption source witness is stale or unavailable' USING ERRCODE = 'PBC01';
        END IF;
        v_binding := jsonb_build_object('kind', 'assignment', 'source', jsonb_build_object(
            'kind', 'alpha', 'reference', p_source->'reference', 'revision', p_source->'revision',
            'moduleIndex', v_module, 'assignmentIndex', v_assignment
        ));
    ELSE
        v_definition := NULL;
        v_binding := jsonb_build_object('kind', 'alpha', 'source', jsonb_build_object(
            'reference', p_source->'reference', 'revision', p_source->'revision'
        ));
        v_module := NULL; v_assignment := 0;
    END IF;
    v_semantic := CASE WHEN p_assignment_source OR v_is_blueprint
        THEN public.ple_cac_semantic_definition_v1(v_definition)
        ELSE public.ple_cac_semantic_alpha_v1(v_document) END;
    IF NOT p_assignment_source AND NOT v_is_blueprint THEN
        v_pin_availability := jsonb_build_object('kind', 'available');
        FOR v_module, v_assignment, v_definition IN
            SELECT module.ordinality - 1, definition.ordinality - 1, definition.value->'definition'
              FROM jsonb_array_elements(v_document->'modules') WITH ORDINALITY AS module(value, ordinality)
              CROSS JOIN LATERAL jsonb_array_elements(module.value->'definitions') WITH ORDINALITY
                  AS definition(value, ordinality)
             ORDER BY module.ordinality, definition.ordinality
        LOOP
            v_pin_availability := public.ple_cac_pin_availability_v1(
                v_definition, v_module, v_assignment, p_replacements
            );
            EXIT WHEN v_pin_availability->>'kind' = 'unavailable';
        END LOOP;
    ELSE
        v_pin_availability := public.ple_cac_pin_availability_v1(
            v_definition, v_module, v_assignment, p_replacements
        );
    END IF;
    RETURN jsonb_build_object(
        'requestedSource', v_binding,
        'currentSource', v_binding,
        'rawSemantic', v_semantic,
        'resolvedReplacements', public.ple_cac_resolved_replacements_v1(coalesce(p_replacements, '[]'::jsonb)),
        'targetTerm', p_target_term,
        'requestedReplacements', coalesce(p_replacements, '[]'::jsonb),
        'pinAvailability', v_pin_availability
    );
END $$;


ALTER FUNCTION public.ple_cac_validate_request_v1(text, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_reusable_document_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_semantic_definition_v1(jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_semantic_alpha_v1(jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_resolved_replacements_v1(jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_pin_availability_v1(jsonb, integer, integer, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_source_facts_v1(uuid, character, jsonb, jsonb, jsonb, boolean)
    OWNER TO ple_curriculum_adoption_broker;

REVOKE ALL ON FUNCTION public.ple_cac_validate_request_v1(text, jsonb),
    public.ple_cac_reusable_document_v1(uuid, character, jsonb), public.ple_cac_semantic_definition_v1(jsonb),
    public.ple_cac_semantic_alpha_v1(jsonb), public.ple_cac_resolved_replacements_v1(jsonb),
    public.ple_cac_pin_availability_v1(jsonb, integer, integer, jsonb),
    public.ple_cac_source_facts_v1(uuid, character, jsonb, jsonb, jsonb, boolean)
    FROM PUBLIC, ple_app, ple_curriculum_adoption_broker;

-- The adoption broker composes these source readers behind the public bridge.
-- Grant the complete private family to that one broker after closing the
-- application-facing surface.
GRANT EXECUTE ON FUNCTION public.ple_cac_validate_request_v1(text, jsonb),
    public.ple_cac_reusable_document_v1(uuid, character, jsonb), public.ple_cac_semantic_definition_v1(jsonb),
    public.ple_cac_semantic_alpha_v1(jsonb), public.ple_cac_resolved_replacements_v1(jsonb),
    public.ple_cac_pin_availability_v1(jsonb, integer, integer, jsonb),
    public.ple_cac_source_facts_v1(uuid, character, jsonb, jsonb, jsonb, boolean)
    TO ple_curriculum_adoption_broker;

DO $$
DECLARE v_function regprocedure; v_role text;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'public.ple_cac_validate_request_v1(text,jsonb)'::regprocedure,
        'public.ple_cac_reusable_document_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_cac_semantic_definition_v1(jsonb)'::regprocedure,
        'public.ple_cac_semantic_alpha_v1(jsonb)'::regprocedure,
        'public.ple_cac_resolved_replacements_v1(jsonb)'::regprocedure,
        'public.ple_cac_pin_availability_v1(jsonb,integer,integer,jsonb)'::regprocedure,
        'public.ple_cac_source_facts_v1(uuid,character,jsonb,jsonb,jsonb,boolean)'::regprocedure
    ] LOOP
        IF (SELECT pg_get_userbyid(proowner) FROM pg_proc WHERE oid = v_function)
               <> 'ple_curriculum_adoption_broker' THEN
            RAISE EXCEPTION 'curriculum adoption source fact ownership is unsafe';
        END IF;
        IF NOT has_function_privilege('ple_curriculum_adoption_broker', v_function, 'EXECUTE') THEN
            RAISE EXCEPTION 'curriculum adoption source fact capability is incomplete';
        END IF;
        FOREACH v_role IN ARRAY ARRAY['public', 'ple_app', 'ple_auth', 'ple_student', 'ple_grader', 'ple_grading_reader'] LOOP
            IF has_function_privilege(v_role, v_function, 'EXECUTE') THEN
                RAISE EXCEPTION 'curriculum adoption source facts leaked to %', v_role;
            END IF;
        END LOOP;
    END LOOP;
    IF has_table_privilege('ple_curriculum_adoption_broker', 'public.catalog_search_document', 'INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_curriculum_adoption_broker', 'public.catalog_tenant_grant', 'INSERT,UPDATE,DELETE') THEN
        RAISE EXCEPTION 'curriculum adoption source reader can mutate catalog state';
    END IF;
END $$;

COMMIT;
