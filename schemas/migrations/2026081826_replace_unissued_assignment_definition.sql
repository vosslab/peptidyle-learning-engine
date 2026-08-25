-- WP-PROF-T5: one complete structural edit before the first learner run.
--
-- ASVS 1.2.4, 2.2.1-2.2.3, and 2.3.1-2.3.4: closed JSON is validated by a
-- SECURITY DEFINER capability; all caller values are parameters; the same
-- course/advisory/assignment lock order as learner-work preparation decides
-- the structural-edit versus first-run race inside PostgreSQL.

BEGIN;

-- Pure complete v1 validator for the T5 pre-issuance structural replacement.
-- It writes no permanent relation, takes no business lock, and returns the
-- canonical payload only after validating every browser-independent v1
-- invariant. Ordinary 1814 replacement remains the accepted, separate
-- identity-preserving authority; this validator truthfully serves fresh graph
-- replacement before issued evidence exists.
CREATE FUNCTION public.ple_validate_t5_replacement_payload_v1(
    p_payload jsonb
) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v_entry jsonb; v_candidate jsonb; v_positions integer; v_min integer; v_max integer;
    v_candidate_count integer := 0; v_active_count integer; v_candidate_positions integer;
    v_completion_kind text; v_practice_kind text; v_uuid_pattern constant text := '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$';
BEGIN
    IF p_payload IS NULL OR octet_length(p_payload::text) > 524288 THEN
        RAISE EXCEPTION 'assignment definition capability arguments are invalid' USING ERRCODE='22023';
    END IF;
    PERFORM public.ple_assignment_definition_require_object(
        p_payload,
        ARRAY['schemaVersion','title','lifecycle','instructions','policies','disclosurePolicy','audience','basePolicy','entries'],
        ARRAY['schemaVersion','title','lifecycle','instructions','policies','disclosurePolicy','audience','basePolicy','entries'],
        524288
    );
    IF p_payload->>'schemaVersion' <> '1' OR jsonb_typeof(p_payload->'schemaVersion') <> 'number'
       OR jsonb_typeof(p_payload->'entries') <> 'array'
       OR jsonb_array_length(p_payload->'entries') NOT BETWEEN 1 AND 1024 THEN
        RAISE EXCEPTION 'assignment definition schema version or entries are invalid' USING ERRCODE='22023';
    END IF;
    IF public.ple_assignment_definition_require_text(p_payload,'title',NULL,200) <> btrim(p_payload->>'title')
       OR char_length(p_payload->>'title')=0 THEN
        RAISE EXCEPTION 'assignment title is invalid' USING ERRCODE='22023';
    END IF;
    PERFORM public.ple_assignment_definition_require_text(p_payload,'lifecycle',ARRAY['draft','published','closed','archived'],16);
    PERFORM public.ple_assignment_definition_require_text(p_payload,'instructions',NULL,50000);
    PERFORM public.ple_assignment_definition_require_object(p_payload->'policies',ARRAY['completion','grade','continuedPractice','variation'],ARRAY['completion','grade','continuedPractice','variation'],4096);
    PERFORM public.ple_assignment_definition_require_object(p_payload#>'{policies,completion}',ARRAY['kind','threshold'],ARRAY['kind'],256);
    v_completion_kind := public.ple_assignment_definition_require_text(p_payload#>'{policies,completion}','kind',ARRAY['answerAll','allCorrect','scoreAtLeast'],32);
    IF (v_completion_kind='scoreAtLeast' AND (jsonb_typeof(p_payload#>'{policies,completion,threshold}')<>'string' OR p_payload#>>'{policies,completion,threshold}' !~ '^(0|1|0\.[0-9]{1,8}|1\.0{1,8})$'))
       OR (v_completion_kind<>'scoreAtLeast' AND p_payload#>'{policies,completion,threshold}' IS NOT NULL) THEN
        RAISE EXCEPTION 'completion policy is invalid' USING ERRCODE='22023';
    END IF;
    PERFORM public.ple_assignment_definition_require_text(p_payload->'policies','grade',ARRAY['first','last','highest','instructorSelected'],32);
    PERFORM public.ple_assignment_definition_require_object(p_payload#>'{policies,continuedPractice}',ARRAY['kind','maxAdditionalRuns'],ARRAY['kind'],256);
    v_practice_kind := public.ple_assignment_definition_require_text(p_payload#>'{policies,continuedPractice}','kind',ARRAY['unlimited','capped','closed'],32);
    IF (v_practice_kind='capped' AND (jsonb_typeof(p_payload#>'{policies,continuedPractice,maxAdditionalRuns}')<>'number' OR p_payload#>>'{policies,continuedPractice,maxAdditionalRuns}' !~ '^(0|[1-9][0-9]{0,8})$'))
       OR (v_practice_kind<>'capped' AND p_payload#>'{policies,continuedPractice,maxAdditionalRuns}' IS NOT NULL) THEN
        RAISE EXCEPTION 'continued-practice policy is invalid' USING ERRCODE='22023';
    END IF;
    PERFORM public.ple_assignment_definition_require_text(p_payload->'policies','variation',ARRAY['newSeeds','selectedProblemVariants','fullRegeneration'],32);
    PERFORM public.ple_assignment_definition_require_object(p_payload->'disclosurePolicy',ARRAY['score','perItemCorrectness','feedbackText','solution','classStatistics'],ARRAY['score','perItemCorrectness','feedbackText','solution','classStatistics'],4096);
    PERFORM public.ple_assignment_definition_require_text(p_payload->'disclosurePolicy','score',ARRAY['duringAttempt','afterSubmit','afterDue','afterClose','never'],32);
    PERFORM public.ple_assignment_definition_require_text(p_payload->'disclosurePolicy','perItemCorrectness',ARRAY['duringAttempt','afterSubmit','afterDue','afterClose','never'],32);
    PERFORM public.ple_assignment_definition_require_text(p_payload->'disclosurePolicy','feedbackText',ARRAY['duringAttempt','afterSubmit','afterDue','afterClose','never'],32);
    PERFORM public.ple_assignment_definition_require_text(p_payload->'disclosurePolicy','solution',ARRAY['duringAttempt','afterSubmit','afterDue','afterClose','never'],32);
    PERFORM public.ple_assignment_definition_require_text(p_payload->'disclosurePolicy','classStatistics',ARRAY['duringAttempt','afterSubmit','afterDue','afterClose','never'],32);
    PERFORM public.ple_assignment_definition_require_object(p_payload->'audience',ARRAY['kind','groups'],ARRAY['kind'],65536);
    PERFORM public.ple_assignment_definition_require_text(p_payload->'audience','kind',ARRAY['courseWide','anyOfGroups'],16);
    PERFORM public.ple_assignment_definition_require_object(p_payload->'basePolicy',ARRAY['availableAt','dueAt','closesAt','lateSubmission','deadlineBehavior','timeLimitSeconds','attemptLimit'],ARRAY['availableAt','dueAt','closesAt','lateSubmission','deadlineBehavior','timeLimitSeconds','attemptLimit'],4096);
    IF p_payload#>>'{basePolicy,lateSubmission}' NOT IN ('accept','reject','markLate')
       OR p_payload#>>'{basePolicy,deadlineBehavior}' <> 'autoSubmit' THEN
        RAISE EXCEPTION 'assignment base policy is invalid' USING ERRCODE='22023';
    END IF;
    IF (p_payload#>'{basePolicy,availableAt}' <> 'null'::jsonb AND (jsonb_typeof(p_payload#>'{basePolicy,availableAt}')<>'number' OR p_payload#>>'{basePolicy,availableAt}' !~ '^-?[0-9]+$'))
       OR (p_payload#>'{basePolicy,dueAt}' <> 'null'::jsonb AND (jsonb_typeof(p_payload#>'{basePolicy,dueAt}')<>'number' OR p_payload#>>'{basePolicy,dueAt}' !~ '^-?[0-9]+$'))
       OR (p_payload#>'{basePolicy,closesAt}' <> 'null'::jsonb AND (jsonb_typeof(p_payload#>'{basePolicy,closesAt}')<>'number' OR p_payload#>>'{basePolicy,closesAt}' !~ '^-?[0-9]+$'))
       OR (p_payload#>'{basePolicy,timeLimitSeconds}' <> 'null'::jsonb AND (jsonb_typeof(p_payload#>'{basePolicy,timeLimitSeconds}')<>'number' OR p_payload#>>'{basePolicy,timeLimitSeconds}' !~ '^[1-9][0-9]{0,8}$'))
       OR (p_payload#>'{basePolicy,attemptLimit}' <> 'null'::jsonb AND (jsonb_typeof(p_payload#>'{basePolicy,attemptLimit}')<>'number' OR p_payload#>>'{basePolicy,attemptLimit}' !~ '^[1-9][0-9]{0,8}$')) THEN
        RAISE EXCEPTION 'base policy timestamps or limits are invalid' USING ERRCODE='22023';
    END IF;
    IF p_payload#>>'{audience,kind}' = 'courseWide' AND p_payload#>'{audience,groups}' IS NOT NULL THEN
        RAISE EXCEPTION 'course-wide audience cannot include groups' USING ERRCODE='22023';
    END IF;
    IF p_payload#>>'{audience,kind}' = 'anyOfGroups'
       AND (jsonb_typeof(p_payload#>'{audience,groups}') <> 'array'
            OR jsonb_array_length(p_payload#>'{audience,groups}') NOT BETWEEN 1 AND 512
            OR EXISTS (SELECT 1 FROM jsonb_array_elements_text(p_payload#>'{audience,groups}') value GROUP BY value HAVING count(*)>1)
            OR EXISTS (SELECT 1 FROM jsonb_array_elements_text(p_payload#>'{audience,groups}') value WHERE value !~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$')) THEN
        RAISE EXCEPTION 'assignment audience groups are invalid' USING ERRCODE='22023';
    END IF;
    FOR v_entry IN SELECT value FROM jsonb_array_elements(p_payload->'entries') LOOP
        IF jsonb_typeof(v_entry)<>'object' OR v_entry->>'kind' NOT IN ('fixed','selectionGroup')
           OR v_entry->>'position' !~ '^(0|[1-9][0-9]{0,8})$' THEN
            RAISE EXCEPTION 'assignment entry is invalid' USING ERRCODE='22023';
        END IF;
        IF v_entry->>'kind'='fixed' THEN
            PERFORM public.ple_assignment_definition_require_object(v_entry,ARRAY['kind','id','position','problemId','versionId','pointsPossible','deliveryState','scoringMode'],ARRAY['kind','id','position','problemId','versionId','pointsPossible','deliveryState','scoringMode'],4096);
            IF v_entry->>'id' !~* v_uuid_pattern OR v_entry->>'problemId' !~* v_uuid_pattern OR v_entry->>'versionId' !~* v_uuid_pattern
               OR v_entry->>'pointsPossible' !~ '^(0|[1-9][0-9]{0,11})(\.[0-9]{1,4})?$'
               OR v_entry->>'deliveryState' NOT IN ('active','retired')
               OR v_entry->>'scoringMode' NOT IN ('normal','fullCredit','extraCredit','excluded')
               OR (v_entry->>'deliveryState'='retired' AND v_entry->>'scoringMode'<>'excluded') THEN
                RAISE EXCEPTION 'fixed item values are invalid' USING ERRCODE='22023';
            END IF;
        ELSE
            PERFORM public.ple_assignment_definition_require_object(v_entry,ARRAY['kind','id','position','drawCount','pointsPerItem','ordering','algorithmVersion','candidates'],ARRAY['kind','id','position','drawCount','pointsPerItem','ordering','algorithmVersion','candidates'],262144);
            IF v_entry->>'algorithmVersion' <> '1' OR jsonb_typeof(v_entry->'algorithmVersion')<>'number'
               OR v_entry->>'drawCount' !~ '^[1-9][0-9]{0,8}$'
               OR v_entry->>'ordering' NOT IN ('candidateOrder','randomized')
               OR jsonb_typeof(v_entry->'candidates')<>'array'
               OR jsonb_array_length(v_entry->'candidates') NOT BETWEEN 1 AND 1024 THEN
                RAISE EXCEPTION 'selection group values are invalid' USING ERRCODE='22023';
            END IF;
            FOR v_candidate IN SELECT value FROM jsonb_array_elements(v_entry->'candidates') LOOP
                PERFORM public.ple_assignment_definition_require_object(v_candidate,ARRAY['id','position','problemId','versionId','deliveryState'],ARRAY['id','position','problemId','versionId','deliveryState'],4096);
                IF v_candidate->>'position' !~ '^(0|[1-9][0-9]{0,8})$' OR v_candidate->>'deliveryState' NOT IN ('active','retired') THEN
                    RAISE EXCEPTION 'selection candidate values are invalid' USING ERRCODE='22023';
                END IF;
                IF v_candidate->>'id' !~* v_uuid_pattern OR v_candidate->>'problemId' !~* v_uuid_pattern OR v_candidate->>'versionId' !~* v_uuid_pattern THEN
                    RAISE EXCEPTION 'selection candidate identifiers are invalid' USING ERRCODE='22023';
                END IF;
                v_candidate_count := v_candidate_count + 1;
            END LOOP;
            IF v_entry->>'id' !~* v_uuid_pattern OR v_entry->>'pointsPerItem' !~ '^(0|[1-9][0-9]{0,11})(\.[0-9]{1,4})?$' THEN
                RAISE EXCEPTION 'selection group identifiers or points are invalid' USING ERRCODE='22023';
            END IF;
            SELECT count(*),count(*) FILTER (WHERE value->>'deliveryState'='active'),count(DISTINCT (value->>'position')::integer)
              INTO v_candidate_positions,v_active_count,v_positions FROM jsonb_array_elements(v_entry->'candidates');
            IF v_candidate_positions <> (SELECT max((value->>'position')::integer)+1 FROM jsonb_array_elements(v_entry->'candidates'))
               OR v_positions<>v_candidate_positions OR v_active_count<(v_entry->>'drawCount')::integer THEN
                RAISE EXCEPTION 'selection candidate positions or draw count are invalid' USING ERRCODE='22023';
            END IF;
        END IF;
    END LOOP;
    SELECT count(*)::integer,min((value->>'position')::integer),max((value->>'position')::integer)
      INTO v_positions,v_min,v_max FROM jsonb_array_elements(p_payload->'entries');
    IF v_min<>0 OR v_max<>v_positions-1
       OR (SELECT count(DISTINCT (value->>'position')::integer) FROM jsonb_array_elements(p_payload->'entries'))<>v_positions THEN
        RAISE EXCEPTION 'fixed items and selection groups require one contiguous position namespace' USING ERRCODE='22023';
    END IF;
    IF v_candidate_count>8192
       OR EXISTS (
           WITH all_ids AS (
               SELECT value->>'id' AS identity FROM jsonb_array_elements(p_payload->'entries')
               UNION ALL
               SELECT candidate.value->>'id' FROM jsonb_array_elements(p_payload->'entries') entry(value)
               CROSS JOIN LATERAL jsonb_array_elements(entry.value->'candidates') candidate(value)
           ) SELECT 1 FROM all_ids GROUP BY identity HAVING count(*)>1
       ) THEN
        RAISE EXCEPTION 'assignment entry identities or candidate limit are invalid' USING ERRCODE='22023';
    END IF;
    IF p_payload#>>'{policies,variation}'='selectedProblemVariants'
       AND EXISTS (SELECT 1 FROM jsonb_array_elements(p_payload->'entries') entry(value) WHERE entry.value->>'kind'='selectionGroup') THEN
        RAISE EXCEPTION 'selected-problem variants require an explicit pool selection model' USING ERRCODE='22023';
    END IF;
    RETURN p_payload;
END
$$;

CREATE FUNCTION public.ple_replace_unissued_assignment_definition_v1(
    p_tenant uuid, p_actor uuid, p_course uuid, p_assignment uuid,
    p_expected_revision bigint, p_payload jsonb
) RETURNS TABLE(outcome text, revision bigint, scoring_generation bigint, scoring_status text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v_old_revision bigint;
    v_entry jsonb;
    v_candidate jsonb;
    v_title text;
    v_lifecycle text;
    v_instructions text;
    v_audience jsonb;
    v_reference_lifecycle text;
    v_old_title text;
    v_position_count integer;
    v_min_position integer;
    v_max_position integer;
BEGIN
    -- `ple_assignment_mutator_require_editor` holds course, advisory, and
    -- assignment-row locks.  1817 learner-work preparation obtains that same
    -- order before it creates an assignment run, making this the sole race
    -- decision point rather than an application pre-read (ASVS 2.3.3/2.3.4).
    v_old_revision := public.ple_assignment_mutator_require_editor(
        p_tenant, p_actor, p_course, p_assignment, p_expected_revision
    );

    IF EXISTS (
        SELECT 1
          FROM public.assignment_run AS run
          JOIN public.enrollment AS enrollment
            ON enrollment.tenant_id = run.tenant_id
           AND enrollment.enrollment_id = run.enrollment_id
         WHERE run.tenant_id = p_tenant
           AND enrollment.assignment_id = p_assignment
    ) THEN
        outcome := 'issued';
        RETURN NEXT;
        RETURN;
    END IF;

    p_payload := public.ple_validate_t5_replacement_payload_v1(p_payload);
    -- Normalize once, then use these exact staged rows for deterministic final
    -- locks and source-graph writes. No mutable validator side effect exists.
    CREATE TEMP TABLE pg_temp.ple_unissued_entry (entry jsonb NOT NULL) ON COMMIT DROP;
    INSERT INTO pg_temp.ple_unissued_entry SELECT value FROM jsonb_array_elements(p_payload->'entries');
    CREATE TEMP TABLE pg_temp.ple_unissued_reference (
        problem_id uuid NOT NULL, version_id uuid NOT NULL,
        PRIMARY KEY(problem_id,version_id)
    ) ON COMMIT DROP;
    INSERT INTO pg_temp.ple_unissued_reference
    SELECT (entry->>'problemId')::uuid,(entry->>'versionId')::uuid
      FROM pg_temp.ple_unissued_entry WHERE entry->>'kind'='fixed'
    ON CONFLICT DO NOTHING;
    INSERT INTO pg_temp.ple_unissued_reference
    SELECT (candidate.value->>'problemId')::uuid,(candidate.value->>'versionId')::uuid
      FROM pg_temp.ple_unissued_entry entry
      CROSS JOIN LATERAL jsonb_array_elements(entry.entry->'candidates') candidate(value)
     WHERE entry.entry->>'kind'='selectionGroup'
    ON CONFLICT DO NOTHING;
    FOR v_reference_lifecycle IN
        SELECT public.ple_lock_assignable_problem_version(reference.problem_id,reference.version_id)
          FROM pg_temp.ple_unissued_reference reference
         ORDER BY reference.problem_id,reference.version_id
    LOOP
        IF v_reference_lifecycle NOT IN ('published','deprecated') THEN
            RAISE EXCEPTION 'assignment definition references an unavailable publication' USING ERRCODE='42501';
        END IF;
    END LOOP;
    v_audience := p_payload->'audience';
    IF v_audience->>'kind'='anyOfGroups' THEN
        CREATE TEMP TABLE pg_temp.ple_unissued_audience_group (course_group_id uuid PRIMARY KEY) ON COMMIT DROP;
        INSERT INTO pg_temp.ple_unissued_audience_group
        SELECT value::uuid FROM jsonb_array_elements_text(v_audience->'groups');
        PERFORM 1 FROM public.course_group group_row
         JOIN pg_temp.ple_unissued_audience_group selected
           ON selected.course_group_id=group_row.course_group_id
         WHERE group_row.tenant_id=p_tenant AND group_row.course_id=p_course
           AND group_row.purpose IN ('section','lab','cohort')
         ORDER BY group_row.course_group_id FOR UPDATE;
        IF (SELECT count(*) FROM public.course_group group_row
             JOIN pg_temp.ple_unissued_audience_group selected ON selected.course_group_id=group_row.course_group_id
             WHERE group_row.tenant_id=p_tenant AND group_row.course_id=p_course
               AND group_row.purpose IN ('section','lab','cohort'))
           <> (SELECT count(*) FROM pg_temp.ple_unissued_audience_group) THEN
            RAISE EXCEPTION 'audience course group is unavailable' USING ERRCODE='42501';
        END IF;
    END IF;

    SELECT title INTO v_old_title
      FROM public.assignment
     WHERE tenant_id = p_tenant AND assignment_id = p_assignment;
    v_title := p_payload ->> 'title';
    v_lifecycle := p_payload ->> 'lifecycle';
    v_instructions := p_payload ->> 'instructions';
    v_audience := p_payload -> 'audience';

    UPDATE public.assignment AS assignment
       SET title = v_title,
           instructions = v_instructions,
           lifecycle = v_lifecycle,
           audience_kind = CASE WHEN v_audience ->> 'kind' = 'courseWide'
                                THEN 'course_wide' ELSE 'any_of_groups' END,
           completion_policy = CASE p_payload #>> '{policies,completion,kind}'
               WHEN 'answerAll' THEN 'answer_all'
               WHEN 'allCorrect' THEN 'all_correct'
               ELSE 'score_at_least' END,
           completion_threshold = CASE WHEN p_payload #>> '{policies,completion,kind}' = 'scoreAtLeast'
               THEN (p_payload #>> '{policies,completion,threshold}')::numeric ELSE NULL END,
           attempt_selection_policy = CASE p_payload #>> '{policies,grade}'
               WHEN 'instructorSelected' THEN 'instructor_selected' ELSE p_payload #>> '{policies,grade}' END,
           continued_practice_policy = p_payload #>> '{policies,continuedPractice,kind}',
           practice_max_additional_runs = CASE WHEN p_payload #>> '{policies,continuedPractice,kind}' = 'capped'
               THEN (p_payload #>> '{policies,continuedPractice,maxAdditionalRuns}')::integer ELSE NULL END,
           variation_policy = CASE p_payload #>> '{policies,variation}'
               WHEN 'newSeeds' THEN 'new_seeds'
               WHEN 'selectedProblemVariants' THEN 'selected_problem_variants'
               ELSE 'full_regeneration' END,
           score_disclosure = public.ple_assignment_definition_disclosure(p_payload #>> '{disclosurePolicy,score}'),
           per_item_correctness_disclosure = public.ple_assignment_definition_disclosure(p_payload #>> '{disclosurePolicy,perItemCorrectness}'),
           feedback_text_disclosure = public.ple_assignment_definition_disclosure(p_payload #>> '{disclosurePolicy,feedbackText}'),
           solution_disclosure = public.ple_assignment_definition_disclosure(p_payload #>> '{disclosurePolicy,solution}'),
           class_statistics_disclosure = public.ple_assignment_definition_disclosure(p_payload #>> '{disclosurePolicy,classStatistics}'),
           updated_at = transaction_timestamp()
     WHERE assignment.tenant_id = p_tenant AND assignment.assignment_id = p_assignment;

    INSERT INTO public.assignment_effective_policy_base (
        tenant_id, assignment_id, course_id, available_at, due_at, closes_at,
        late_submission_policy, deadline_behavior, time_limit_seconds, attempt_limit
    ) VALUES (
        p_tenant, p_assignment, p_course,
        public.ple_assignment_definition_millis(p_payload #> '{basePolicy,availableAt}'),
        public.ple_assignment_definition_millis(p_payload #> '{basePolicy,dueAt}'),
        public.ple_assignment_definition_millis(p_payload #> '{basePolicy,closesAt}'),
        CASE p_payload #>> '{basePolicy,lateSubmission}' WHEN 'markLate' THEN 'mark_late'
             ELSE p_payload #>> '{basePolicy,lateSubmission}' END,
        'auto_submit',
        CASE WHEN p_payload #> '{basePolicy,timeLimitSeconds}' = 'null'::jsonb THEN NULL
             ELSE (p_payload #>> '{basePolicy,timeLimitSeconds}')::integer END,
        CASE WHEN p_payload #> '{basePolicy,attemptLimit}' = 'null'::jsonb THEN NULL
             ELSE (p_payload #>> '{basePolicy,attemptLimit}')::integer END
    ) ON CONFLICT (tenant_id, assignment_id) DO UPDATE SET
        available_at = EXCLUDED.available_at, due_at = EXCLUDED.due_at,
        closes_at = EXCLUDED.closes_at, late_submission_policy = EXCLUDED.late_submission_policy,
        deadline_behavior = EXCLUDED.deadline_behavior,
        time_limit_seconds = EXCLUDED.time_limit_seconds, attempt_limit = EXCLUDED.attempt_limit,
        updated_at = transaction_timestamp();

    DELETE FROM public.assignment_audience_group
     WHERE tenant_id = p_tenant AND assignment_id = p_assignment;
    IF v_audience ->> 'kind' = 'anyOfGroups' THEN
        INSERT INTO public.assignment_audience_group (tenant_id, assignment_id, course_id, course_group_id)
        SELECT p_tenant, p_assignment, p_course, course_group_id
          FROM pg_temp.ple_unissued_audience_group
         ORDER BY course_group_id;
    END IF;

    -- No `assignment_run` exists under the shared lock, so replacing this
    -- mutable source graph cannot alter immutable issued evidence.
    DELETE FROM public.assignment_selection_candidate
     WHERE tenant_id = p_tenant AND assignment_id = p_assignment;
    DELETE FROM public.assignment_selection_group
     WHERE tenant_id = p_tenant AND assignment_id = p_assignment;
    DELETE FROM public.assignment_item
     WHERE tenant_id = p_tenant AND assignment_id = p_assignment;
    FOR v_entry IN SELECT entry FROM pg_temp.ple_unissued_entry ORDER BY (entry ->> 'position')::integer LOOP
        IF v_entry ->> 'kind' = 'fixed' THEN
            INSERT INTO public.assignment_item (
                tenant_id, assignment_id, assignment_item_id, position, problem_id, version_id,
                points_possible, delivery_state, scoring_mode
            ) VALUES (
                p_tenant, p_assignment, (v_entry ->> 'id')::uuid,
                (v_entry ->> 'position')::integer, (v_entry ->> 'problemId')::uuid,
                (v_entry ->> 'versionId')::uuid, (v_entry ->> 'pointsPossible')::numeric,
                v_entry ->> 'deliveryState', v_entry ->> 'scoringMode'
            );
        ELSE
            INSERT INTO public.assignment_selection_group (
                tenant_id, assignment_id, selection_group_id, position, draw_count,
                points_per_item, ordering_policy, algorithm_version
            ) VALUES (
                p_tenant, p_assignment, (v_entry ->> 'id')::uuid,
                (v_entry ->> 'position')::integer, (v_entry ->> 'drawCount')::integer,
                (v_entry ->> 'pointsPerItem')::numeric,
                CASE WHEN v_entry ->> 'ordering' = 'candidateOrder' THEN 'candidate_order'
                     ELSE v_entry ->> 'ordering' END,
                1
            );
            FOR v_candidate IN SELECT value FROM jsonb_array_elements(v_entry -> 'candidates') LOOP
                INSERT INTO public.assignment_selection_candidate (
                    tenant_id, assignment_id, selection_group_id, candidate_id, position,
                    problem_id, version_id, delivery_state
                ) VALUES (
                    p_tenant, p_assignment, (v_entry ->> 'id')::uuid,
                    (v_candidate ->> 'id')::uuid, (v_candidate ->> 'position')::integer,
                    (v_candidate ->> 'problemId')::uuid, (v_candidate ->> 'versionId')::uuid,
                    v_candidate ->> 'deliveryState'
                );
            END LOOP;
        END IF;
    END LOOP;

    SELECT new_revision INTO revision
      FROM public.ple_apply_verified_assignment_definition_revision(
          p_tenant, p_course, p_assignment, v_old_revision
      );
    IF v_old_title IS DISTINCT FROM v_title THEN
        UPDATE public.course_grade_scheme AS scheme
           SET revision = scheme.revision + 1, updated_at = transaction_timestamp()
         WHERE scheme.tenant_id = p_tenant AND scheme.course_id = p_course;
    END IF;
    SELECT assignment.scoring_generation, assignment.scoring_status
      INTO scoring_generation, scoring_status
     FROM public.assignment AS assignment
     WHERE assignment.tenant_id = p_tenant AND assignment.assignment_id = p_assignment;
    outcome := 'replaced';
    RETURN NEXT;
END
$$;

ALTER FUNCTION public.ple_replace_unissued_assignment_definition_v1(
    uuid, uuid, uuid, uuid, bigint, jsonb
) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_validate_t5_replacement_payload_v1(jsonb)
    OWNER TO ple_assignment_mutator_broker;
REVOKE ALL ON FUNCTION public.ple_validate_t5_replacement_payload_v1(jsonb)
    FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_replace_unissued_assignment_definition_v1(
    uuid, uuid, uuid, uuid, bigint, jsonb
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_replace_unissued_assignment_definition_v1(
    uuid, uuid, uuid, uuid, bigint, jsonb
) TO ple_app;

DO $$
BEGIN
    IF NOT has_function_privilege(
            'ple_app',
            'public.ple_replace_unissued_assignment_definition_v1(uuid,uuid,uuid,uuid,bigint,jsonb)',
            'EXECUTE'
       )
       OR has_table_privilege(
            'ple_app', 'public.assignment_item', 'INSERT,UPDATE,DELETE'
       )
       OR has_table_privilege(
            'ple_app', 'public.assignment_selection_group', 'INSERT,UPDATE,DELETE'
       )
       OR has_table_privilege(
            'ple_app', 'public.assignment_selection_candidate', 'INSERT,UPDATE,DELETE'
       ) THEN
        RAISE EXCEPTION 'unissued assignment-definition capability grants are unsafe';
    END IF;
END
$$;

COMMIT;
