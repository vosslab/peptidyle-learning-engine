-- WP-PROF-T6: persisted incomplete assignment drafts.
--
-- This capability keeps the definition graph and policy aggregate in one
-- transaction while making the Draft state an honest persisted state.  The
-- normal complete-definition rules remain authoritative for nonempty
-- definitions.  The unified broker owns the empty-definition boundary and
-- preserves the same tenant, actor, course, assignment, revision, RLS, and
-- grant controls.
-- ASVS 1.2.4, 1.5.2, 2.2.1-2.2.3, 2.3.1-2.3.4, 8.2.1, 8.2.2.

BEGIN;

CREATE OR REPLACE FUNCTION public.ple_assignment_definition_apply_v1(
    p_tenant uuid, p_course uuid, p_assignment uuid, p_payload jsonb, p_replace boolean,
    p_recalculation_job uuid, p_recalculation_max_attempts integer
) RETURNS TABLE(revision bigint, scoring_generation bigint, scoring_status text)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE v_entry jsonb; v_candidate jsonb; audience jsonb; policy jsonb; disclosure jsonb;
DECLARE title_value text; lifecycle_value text; instructions_value text; audience_kind_value text;
DECLARE completion_kind text; new_completion_threshold numeric; grade_value text;
DECLARE practice_kind text; practice_limit integer; variation_value text;
DECLARE score_value text; correctness_value text; feedback_value text; solution_value text; statistics_value text;
DECLARE late_value text; deadline_value text; time_limit_value integer; attempt_limit_value integer;
DECLARE old_title text; old_generation bigint; has_scores boolean; changed boolean := false;
DECLARE old_revision bigint; item_id uuid; group_id uuid; candidate_id uuid; item_count integer := 0;
DECLARE candidate_count integer := 0; active_candidates integer; expected_entries integer; expected_candidates integer;
DECLARE entry_position integer; candidate_position integer; reference_lifecycle text;
BEGIN
    IF p_payload IS NULL OR octet_length(p_payload::text) > 524288 THEN
        RAISE EXCEPTION 'assignment definition payload exceeds its bounded v1 contract' USING ERRCODE = '22023';
    END IF;
    PERFORM public.ple_assignment_definition_require_object(
        p_payload,
        ARRAY['schemaVersion','title','lifecycle','instructions','policies','disclosurePolicy','audience','basePolicy','entries'],
        ARRAY['schemaVersion','title','lifecycle','instructions','policies','disclosurePolicy','audience','basePolicy','entries'],
        524288);
    IF p_payload ->> 'schemaVersion' <> '1' OR jsonb_typeof(p_payload -> 'schemaVersion') <> 'number'
       OR jsonb_typeof(p_payload -> 'entries') <> 'array' OR jsonb_array_length(p_payload -> 'entries') NOT BETWEEN 0 AND 1024 THEN
        RAISE EXCEPTION 'assignment definition schema version or entries are invalid' USING ERRCODE = '22023';
    END IF;
    title_value := public.ple_assignment_definition_require_text(p_payload, 'title', NULL, 200);
    IF title_value <> btrim(title_value) OR char_length(title_value) = 0 THEN
        RAISE EXCEPTION 'assignment title is invalid' USING ERRCODE = '22023'; END IF;
    lifecycle_value := public.ple_assignment_definition_require_text(p_payload, 'lifecycle', ARRAY['draft','published','closed','archived'], 16);
    instructions_value := public.ple_assignment_definition_require_text(p_payload, 'instructions', NULL, 50000);
    policy := p_payload -> 'policies'; disclosure := p_payload -> 'disclosurePolicy';
    PERFORM public.ple_assignment_definition_require_object(policy, ARRAY['completion','grade','continuedPractice','variation'], ARRAY['completion','grade','continuedPractice','variation'], 4096);
    PERFORM public.ple_assignment_definition_require_object(disclosure, ARRAY['score','perItemCorrectness','feedbackText','solution','classStatistics'], ARRAY['score','perItemCorrectness','feedbackText','solution','classStatistics'], 4096);
    PERFORM public.ple_assignment_definition_require_object(p_payload -> 'basePolicy', ARRAY['availableAt','dueAt','closesAt','lateSubmission','deadlineBehavior','timeLimitSeconds','attemptLimit'], ARRAY['availableAt','dueAt','closesAt','lateSubmission','deadlineBehavior','timeLimitSeconds','attemptLimit'], 4096);
    audience := p_payload -> 'audience';
    PERFORM public.ple_assignment_definition_require_object(audience, ARRAY['kind','groups'], ARRAY['kind'], 65536);
    audience_kind_value := public.ple_assignment_definition_require_text(audience, 'kind', ARRAY['courseWide','anyOfGroups'], 16);
    IF audience_kind_value = 'courseWide' AND audience ? 'groups' THEN RAISE EXCEPTION 'course-wide audience cannot include groups' USING ERRCODE = '22023'; END IF;
    IF audience_kind_value = 'anyOfGroups' AND (jsonb_typeof(audience -> 'groups') <> 'array' OR jsonb_array_length(audience -> 'groups') NOT BETWEEN 1 AND 512) THEN
        RAISE EXCEPTION 'group audience is invalid' USING ERRCODE = '22023'; END IF;
    PERFORM public.ple_assignment_definition_require_object(policy->'completion', ARRAY['kind','threshold'], ARRAY['kind'], 256);
    completion_kind := public.ple_assignment_definition_require_text(policy->'completion', 'kind', ARRAY['answerAll','allCorrect','scoreAtLeast'], 32);
    IF completion_kind = 'scoreAtLeast' THEN
        IF jsonb_typeof(policy#>'{completion,threshold}') <> 'string'
           OR policy#>>'{completion,threshold}' !~ '^(0|1|0\\.[0-9]{1,8}|1\\.0{1,8})$' THEN
            RAISE EXCEPTION 'score-at-least threshold is invalid' USING ERRCODE='22023';
        END IF;
        new_completion_threshold := (policy#>>'{completion,threshold}')::numeric;
    ELSIF policy#>'{completion,threshold}' IS NOT NULL THEN
        RAISE EXCEPTION 'only score-at-least accepts a threshold' USING ERRCODE='22023';
    END IF;
    grade_value := public.ple_assignment_definition_require_text(policy, 'grade', ARRAY['first','last','highest','instructorSelected'], 32);
    PERFORM public.ple_assignment_definition_require_object(policy->'continuedPractice', ARRAY['kind','maxAdditionalRuns'], ARRAY['kind'], 256);
    practice_kind := public.ple_assignment_definition_require_text(policy->'continuedPractice', 'kind', ARRAY['unlimited','capped','closed'], 32);
    IF practice_kind = 'capped' THEN
        IF jsonb_typeof(policy#>'{continuedPractice,maxAdditionalRuns}') <> 'number'
           OR policy#>>'{continuedPractice,maxAdditionalRuns}' !~ '^(0|[1-9][0-9]{0,8})$' THEN RAISE EXCEPTION 'practice limit is invalid' USING ERRCODE='22023'; END IF;
        practice_limit := (policy#>>'{continuedPractice,maxAdditionalRuns}')::integer;
    ELSIF policy#>'{continuedPractice,maxAdditionalRuns}' IS NOT NULL THEN RAISE EXCEPTION 'only capped practice accepts a limit' USING ERRCODE='22023'; END IF;
    variation_value := public.ple_assignment_definition_require_text(policy, 'variation', ARRAY['newSeeds','selectedProblemVariants','fullRegeneration'], 32);
    score_value := public.ple_assignment_definition_require_text(disclosure, 'score', ARRAY['duringAttempt','afterSubmit','afterDue','afterClose','never'], 32);
    correctness_value := public.ple_assignment_definition_require_text(disclosure, 'perItemCorrectness', ARRAY['duringAttempt','afterSubmit','afterDue','afterClose','never'], 32);
    feedback_value := public.ple_assignment_definition_require_text(disclosure, 'feedbackText', ARRAY['duringAttempt','afterSubmit','afterDue','afterClose','never'], 32);
    solution_value := public.ple_assignment_definition_require_text(disclosure, 'solution', ARRAY['duringAttempt','afterSubmit','afterDue','afterClose','never'], 32);
    statistics_value := public.ple_assignment_definition_require_text(disclosure, 'classStatistics', ARRAY['duringAttempt','afterSubmit','afterDue','afterClose','never'], 32);
    late_value := public.ple_assignment_definition_require_text(p_payload->'basePolicy', 'lateSubmission', ARRAY['accept','reject','markLate'], 32);
    deadline_value := public.ple_assignment_definition_require_text(p_payload->'basePolicy', 'deadlineBehavior', ARRAY['autoSubmit'], 32);
    IF p_payload#>'{basePolicy,timeLimitSeconds}' <> 'null'::jsonb THEN
        IF jsonb_typeof(p_payload#>'{basePolicy,timeLimitSeconds}') <> 'number' OR p_payload#>>'{basePolicy,timeLimitSeconds}' !~ '^[1-9][0-9]{0,8}$' THEN RAISE EXCEPTION 'time limit is invalid' USING ERRCODE='22023'; END IF;
        time_limit_value := (p_payload#>>'{basePolicy,timeLimitSeconds}')::integer;
    END IF;
    IF p_payload#>'{basePolicy,attemptLimit}' <> 'null'::jsonb THEN
        IF jsonb_typeof(p_payload#>'{basePolicy,attemptLimit}') <> 'number' OR p_payload#>>'{basePolicy,attemptLimit}' !~ '^[1-9][0-9]{0,8}$' THEN RAISE EXCEPTION 'attempt limit is invalid' USING ERRCODE='22023'; END IF;
        attempt_limit_value := (p_payload#>>'{basePolicy,attemptLimit}')::integer;
    END IF;

    -- IDs are emitted solely by the private Rust command codec.  The browser
    -- never gets this wire type; SQL still verifies uniqueness and binding.
    CREATE TEMP TABLE pg_temp.ple_definition_entry (entry jsonb NOT NULL) ON COMMIT DROP;
    INSERT INTO pg_temp.ple_definition_entry SELECT value FROM jsonb_array_elements(p_payload -> 'entries');
    CREATE TEMP TABLE pg_temp.ple_definition_reference (problem_id uuid NOT NULL, version_id uuid NOT NULL, PRIMARY KEY(problem_id, version_id)) ON COMMIT DROP;
    FOR v_entry IN SELECT staged.entry FROM pg_temp.ple_definition_entry AS staged LOOP
        IF jsonb_typeof(v_entry) <> 'object' OR v_entry ->> 'kind' NOT IN ('fixed','selectionGroup') THEN
            RAISE EXCEPTION 'assignment entry is invalid' USING ERRCODE = '22023'; END IF;
        IF jsonb_typeof(v_entry -> 'position') <> 'number' OR v_entry ->> 'position' !~ '^(0|[1-9][0-9]{0,8})$' THEN
            RAISE EXCEPTION 'assignment entry position is invalid' USING ERRCODE = '22023'; END IF;
        IF v_entry ->> 'kind' = 'fixed' THEN
            PERFORM public.ple_assignment_definition_require_object(v_entry, ARRAY['kind','id','position','problemId','versionId','pointsPossible','deliveryState','scoringMode'], ARRAY['kind','id','position','problemId','versionId','pointsPossible','deliveryState','scoringMode'], 4096);
            IF v_entry->>'pointsPossible' !~ '^(0|[1-9][0-9]{0,11})(\.[0-9]{1,4})?$' OR v_entry->>'deliveryState' NOT IN ('active','retired') OR v_entry->>'scoringMode' NOT IN ('normal','fullCredit','extraCredit','excluded') OR (v_entry->>'deliveryState'='retired' AND v_entry->>'scoringMode'<>'excluded') THEN RAISE EXCEPTION 'fixed item values are invalid' USING ERRCODE='22023'; END IF;
            -- The same immutable publication may intentionally occupy more
            -- than one ordered assignment position. This temporary table is
            -- only the distinct set of references that must be locked and
            -- validated once for the command.
            INSERT INTO pg_temp.ple_definition_reference VALUES ((v_entry ->> 'problemId')::uuid, (v_entry ->> 'versionId')::uuid) ON CONFLICT DO NOTHING;
        ELSE
            PERFORM public.ple_assignment_definition_require_object(v_entry, ARRAY['kind','id','position','drawCount','pointsPerItem','ordering','algorithmVersion','candidates'], ARRAY['kind','id','position','drawCount','pointsPerItem','ordering','algorithmVersion','candidates'], 262144);
            IF v_entry->>'drawCount' !~ '^[1-9][0-9]{0,8}$' OR v_entry->>'pointsPerItem' !~ '^(0|[1-9][0-9]{0,11})(\.[0-9]{1,4})?$' OR v_entry->>'ordering' NOT IN ('candidateOrder','randomized') OR v_entry->>'algorithmVersion' !~ '^[1-9][0-9]{0,8}$' THEN RAISE EXCEPTION 'selection group values are invalid' USING ERRCODE='22023'; END IF;
            IF jsonb_typeof(v_entry -> 'candidates') <> 'array' OR jsonb_array_length(v_entry -> 'candidates') NOT BETWEEN 1 AND 1024 THEN RAISE EXCEPTION 'selection candidates are invalid' USING ERRCODE='22023'; END IF;
            FOR v_candidate IN SELECT elements.value FROM jsonb_array_elements(v_entry -> 'candidates') AS elements LOOP
                PERFORM public.ple_assignment_definition_require_object(v_candidate, ARRAY['id','position','problemId','versionId','deliveryState'], ARRAY['id','position','problemId','versionId','deliveryState'], 4096);
                IF v_candidate->>'position' !~ '^(0|[1-9][0-9]{0,8})$' OR v_candidate->>'deliveryState' NOT IN ('active','retired') THEN RAISE EXCEPTION 'selection candidate values are invalid' USING ERRCODE='22023'; END IF;
                INSERT INTO pg_temp.ple_definition_reference VALUES ((v_candidate ->> 'problemId')::uuid, (v_candidate ->> 'versionId')::uuid) ON CONFLICT DO NOTHING;
                candidate_count := candidate_count + 1;
            END LOOP;
            IF (SELECT count(*) FROM jsonb_array_elements(v_entry->'candidates')) <> (SELECT max((elements.value->>'position')::integer)+1 FROM jsonb_array_elements(v_entry->'candidates') AS elements) OR (SELECT count(*) FROM jsonb_array_elements(v_entry->'candidates') AS elements WHERE elements.value->>'deliveryState'='active') < (v_entry->>'drawCount')::integer THEN RAISE EXCEPTION 'selection candidate positions or draw count are invalid' USING ERRCODE='22023'; END IF;
        END IF;
        item_count := item_count + 1;
    END LOOP;
    IF item_count = 0 AND lifecycle_value NOT IN ('draft','archived') THEN RAISE EXCEPTION 'empty definitions require the draft or archived lifecycle' USING ERRCODE='22023'; END IF;
    IF lifecycle_value = 'published' AND NOT (
        EXISTS (SELECT 1 FROM pg_temp.ple_definition_entry AS staged WHERE staged.entry->>'kind'='fixed' AND staged.entry->>'deliveryState'='active')
        OR EXISTS (SELECT 1 FROM pg_temp.ple_definition_entry AS staged WHERE staged.entry->>'kind'='selectionGroup')
    ) THEN RAISE EXCEPTION 'published assignments require an active deliverable position' USING ERRCODE='22023'; END IF;
    IF candidate_count > 8192 OR EXISTS (SELECT staged.entry->>'id' FROM pg_temp.ple_definition_entry AS staged GROUP BY staged.entry->>'id' HAVING count(*) > 1) OR EXISTS (SELECT elements.value->>'id' FROM pg_temp.ple_definition_entry AS staged CROSS JOIN LATERAL jsonb_array_elements(staged.entry->'candidates') AS elements GROUP BY elements.value->>'id' HAVING count(*) > 1) OR EXISTS (SELECT 1 FROM pg_temp.ple_definition_entry AS fixed JOIN pg_temp.ple_definition_entry AS grouped ON true CROSS JOIN LATERAL jsonb_array_elements(grouped.entry->'candidates') AS elements WHERE fixed.entry->>'kind'='fixed' AND grouped.entry->>'kind'='selectionGroup' AND fixed.entry->>'id'=elements.value->>'id') THEN RAISE EXCEPTION 'assignment entry identities are invalid' USING ERRCODE='22023'; END IF;
    FOR reference_lifecycle IN SELECT public.ple_lock_assignable_problem_version(reference.problem_id, reference.version_id) FROM pg_temp.ple_definition_reference AS reference ORDER BY reference.problem_id, reference.version_id LOOP
        IF reference_lifecycle NOT IN ('published','deprecated') THEN RAISE EXCEPTION 'assignment publication is unavailable' USING ERRCODE='42501'; END IF;
    END LOOP;
    IF audience_kind_value = 'anyOfGroups' THEN
        IF EXISTS (SELECT 1 FROM jsonb_array_elements_text(audience -> 'groups') value GROUP BY value HAVING count(*) > 1)
           OR EXISTS (SELECT 1 FROM jsonb_array_elements_text(audience -> 'groups') value WHERE value !~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$') THEN RAISE EXCEPTION 'audience groups are invalid' USING ERRCODE='22023'; END IF;
        PERFORM 1 FROM public.course_group WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id IN (SELECT value::uuid FROM jsonb_array_elements_text(audience -> 'groups')) AND purpose IN ('section','lab','cohort') ORDER BY course_group_id FOR UPDATE;
        IF (SELECT count(*) FROM public.course_group WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id IN (SELECT value::uuid FROM jsonb_array_elements_text(audience -> 'groups')) AND purpose IN ('section','lab','cohort')) <> jsonb_array_length(audience -> 'groups') THEN RAISE EXCEPTION 'audience course group is unavailable' USING ERRCODE='42501'; END IF;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_temp.ple_definition_entry AS staged GROUP BY (staged.entry->>'position') HAVING count(*) > 1)
       OR (SELECT count(*) FROM pg_temp.ple_definition_entry AS staged) <> (SELECT max((staged.entry->>'position')::integer) + 1 FROM pg_temp.ple_definition_entry AS staged) THEN
        RAISE EXCEPTION 'assignment entry positions must be contiguous' USING ERRCODE='22023';
    END IF;
    IF p_replace THEN
        SELECT assignment.title, assignment.revision, assignment.scoring_generation INTO old_title, old_revision, old_generation FROM public.assignment AS assignment WHERE assignment.tenant_id=p_tenant AND assignment.course_id=p_course AND assignment.assignment_id=p_assignment FOR UPDATE;
        IF NOT FOUND THEN RAISE EXCEPTION 'assignment is unavailable' USING ERRCODE='42501'; END IF;
        -- A zero-entry definition is a valid persisted Draft/Archived state.
        -- Nonempty ordinary replacements retain the accepted identity fence.
        IF item_count > 0 THEN
            IF EXISTS (SELECT 1 FROM pg_temp.ple_definition_entry e WHERE e.entry ->> 'kind'='fixed' AND (NOT e.entry ? 'id' OR NOT EXISTS (SELECT 1 FROM public.assignment_item i WHERE i.tenant_id=p_tenant AND i.assignment_id=p_assignment AND i.assignment_item_id=(e.entry ->> 'id')::uuid AND i.problem_id=(e.entry ->> 'problemId')::uuid AND i.version_id=(e.entry ->> 'versionId')::uuid)))
               OR EXISTS (SELECT 1 FROM public.assignment_item i WHERE i.tenant_id=p_tenant AND i.assignment_id=p_assignment AND NOT EXISTS (SELECT 1 FROM pg_temp.ple_definition_entry e WHERE e.entry ->> 'kind'='fixed' AND e.entry ->> 'id'=i.assignment_item_id::text)) THEN RAISE EXCEPTION 'replacement must preserve fixed item identities and references' USING ERRCODE='55000'; END IF;
            IF EXISTS (SELECT 1 FROM pg_temp.ple_definition_entry e WHERE e.entry ->> 'kind'='selectionGroup' AND (NOT e.entry ? 'id' OR NOT EXISTS (SELECT 1 FROM public.assignment_selection_group g WHERE g.tenant_id=p_tenant AND g.assignment_id=p_assignment AND g.selection_group_id=(e.entry ->> 'id')::uuid)))
               OR EXISTS (SELECT 1 FROM public.assignment_selection_group g WHERE g.tenant_id=p_tenant AND g.assignment_id=p_assignment AND NOT EXISTS (SELECT 1 FROM pg_temp.ple_definition_entry e WHERE e.entry ->>'kind'='selectionGroup' AND e.entry->>'id'=g.selection_group_id::text)) THEN RAISE EXCEPTION 'replacement must preserve selection group identities' USING ERRCODE='55000'; END IF;
            IF EXISTS (SELECT 1 FROM pg_temp.ple_definition_entry e CROSS JOIN LATERAL jsonb_array_elements(e.entry->'candidates') c WHERE e.entry->>'kind'='selectionGroup' AND (NOT c ? 'id' OR NOT EXISTS (SELECT 1 FROM public.assignment_selection_candidate x WHERE x.tenant_id=p_tenant AND x.selection_group_id=(e.entry->>'id')::uuid AND x.candidate_id=(c->>'id')::uuid AND x.problem_id=(c->>'problemId')::uuid AND x.version_id=(c->>'versionId')::uuid))) THEN RAISE EXCEPTION 'replacement must preserve candidate identities and references' USING ERRCODE='55000'; END IF;
            IF EXISTS (SELECT 1 FROM public.assignment_selection_candidate x JOIN public.assignment_selection_group g ON g.tenant_id=x.tenant_id AND g.selection_group_id=x.selection_group_id WHERE x.tenant_id=p_tenant AND g.assignment_id=p_assignment AND NOT EXISTS (SELECT 1 FROM pg_temp.ple_definition_entry e CROSS JOIN LATERAL jsonb_array_elements(e.entry->'candidates') c WHERE e.entry->>'kind'='selectionGroup' AND e.entry->>'id'=x.selection_group_id::text AND c->>'id'=x.candidate_id::text)) THEN RAISE EXCEPTION 'replacement must preserve every candidate identity' USING ERRCODE='55000'; END IF;
        END IF;
        -- This is deliberately the same scoring surface as the current Rust
        -- oracle: completion/grade policy plus item/group points, scoring
        -- mode, and delivery state.  Title, instructions, disclosure,
        -- audience, schedule, ordering, and variation still revise/re-resolve
        -- the definition but do not manufacture a recalculation generation.
        SELECT a.completion_policy IS DISTINCT FROM CASE completion_kind WHEN 'answerAll' THEN 'answer_all' WHEN 'allCorrect' THEN 'all_correct' ELSE 'score_at_least' END
            OR a.completion_threshold IS DISTINCT FROM new_completion_threshold
            OR a.attempt_selection_policy IS DISTINCT FROM CASE grade_value WHEN 'instructorSelected' THEN 'instructor_selected' ELSE grade_value END
          INTO changed FROM public.assignment a WHERE a.tenant_id=p_tenant AND a.assignment_id=p_assignment;
        changed := changed OR EXISTS (SELECT 1 FROM public.assignment_item i JOIN pg_temp.ple_definition_entry e ON e.entry->>'id'=i.assignment_item_id::text WHERE i.tenant_id=p_tenant AND i.assignment_id=p_assignment AND e.entry->>'kind'='fixed' AND (i.points_possible IS DISTINCT FROM (e.entry->>'pointsPossible')::numeric OR i.delivery_state IS DISTINCT FROM e.entry->>'deliveryState' OR i.scoring_mode IS DISTINCT FROM e.entry->>'scoringMode'));
        changed := changed OR EXISTS (SELECT 1 FROM public.assignment_selection_group g JOIN pg_temp.ple_definition_entry e ON e.entry->>'id'=g.selection_group_id::text WHERE g.tenant_id=p_tenant AND g.assignment_id=p_assignment AND e.entry->>'kind'='selectionGroup' AND (g.points_per_item IS DISTINCT FROM (e.entry->>'pointsPerItem')::numeric));
        changed := changed OR EXISTS (SELECT 1 FROM public.assignment_selection_candidate x JOIN pg_temp.ple_definition_entry e ON e.entry->>'kind'='selectionGroup' AND e.entry->>'id'=x.selection_group_id::text CROSS JOIN LATERAL jsonb_array_elements(e.entry->'candidates') c WHERE x.tenant_id=p_tenant AND c->>'id'=x.candidate_id::text AND x.delivery_state IS DISTINCT FROM c->>'deliveryState');
    ELSE
        old_generation := 1;
        INSERT INTO public.assignment (tenant_id,assignment_id,course_id,title,instructions,lifecycle,audience_kind,completion_policy,completion_threshold,attempt_selection_policy,continued_practice_policy,practice_max_additional_runs,variation_policy,score_disclosure,per_item_correctness_disclosure,feedback_text_disclosure,solution_disclosure,class_statistics_disclosure)
        VALUES (p_tenant,p_assignment,p_course,title_value,instructions_value,lifecycle_value,CASE WHEN audience_kind_value='courseWide' THEN 'course_wide' ELSE 'any_of_groups' END,CASE completion_kind WHEN 'answerAll' THEN 'answer_all' WHEN 'allCorrect' THEN 'all_correct' ELSE 'score_at_least' END,new_completion_threshold,CASE grade_value WHEN 'instructorSelected' THEN 'instructor_selected' ELSE grade_value END,practice_kind,practice_limit,CASE variation_value WHEN 'newSeeds' THEN 'new_seeds' WHEN 'selectedProblemVariants' THEN 'selected_problem_variants' ELSE 'full_regeneration' END,public.ple_assignment_definition_disclosure(score_value),public.ple_assignment_definition_disclosure(correctness_value),public.ple_assignment_definition_disclosure(feedback_value),public.ple_assignment_definition_disclosure(solution_value),public.ple_assignment_definition_disclosure(statistics_value));
    END IF;
    UPDATE public.assignment AS assignment SET title=title_value,instructions=instructions_value,lifecycle=lifecycle_value,audience_kind=CASE WHEN audience_kind_value='courseWide' THEN 'course_wide' ELSE 'any_of_groups' END,completion_policy=CASE completion_kind WHEN 'answerAll' THEN 'answer_all' WHEN 'allCorrect' THEN 'all_correct' ELSE 'score_at_least' END,completion_threshold=new_completion_threshold,attempt_selection_policy=CASE grade_value WHEN 'instructorSelected' THEN 'instructor_selected' ELSE grade_value END,continued_practice_policy=practice_kind,practice_max_additional_runs=practice_limit,variation_policy=CASE variation_value WHEN 'newSeeds' THEN 'new_seeds' WHEN 'selectedProblemVariants' THEN 'selected_problem_variants' ELSE 'full_regeneration' END,score_disclosure=public.ple_assignment_definition_disclosure(score_value),per_item_correctness_disclosure=public.ple_assignment_definition_disclosure(correctness_value),feedback_text_disclosure=public.ple_assignment_definition_disclosure(feedback_value),solution_disclosure=public.ple_assignment_definition_disclosure(solution_value),class_statistics_disclosure=public.ple_assignment_definition_disclosure(statistics_value),updated_at=transaction_timestamp() WHERE assignment.tenant_id=p_tenant AND assignment.assignment_id=p_assignment;
    INSERT INTO public.assignment_effective_policy_base (tenant_id,assignment_id,course_id,available_at,due_at,closes_at,late_submission_policy,deadline_behavior,time_limit_seconds,attempt_limit)
    VALUES (p_tenant,p_assignment,p_course,public.ple_assignment_definition_millis(p_payload#>'{basePolicy,availableAt}'),public.ple_assignment_definition_millis(p_payload#>'{basePolicy,dueAt}'),public.ple_assignment_definition_millis(p_payload#>'{basePolicy,closesAt}'),CASE late_value WHEN 'markLate' THEN 'mark_late' ELSE late_value END,'auto_submit',time_limit_value,attempt_limit_value)
    ON CONFLICT (tenant_id,assignment_id) DO UPDATE SET available_at=EXCLUDED.available_at,due_at=EXCLUDED.due_at,closes_at=EXCLUDED.closes_at,late_submission_policy=EXCLUDED.late_submission_policy,deadline_behavior=EXCLUDED.deadline_behavior,time_limit_seconds=EXCLUDED.time_limit_seconds,attempt_limit=EXCLUDED.attempt_limit,updated_at=transaction_timestamp();
    DELETE FROM public.assignment_audience_group WHERE tenant_id=p_tenant AND assignment_id=p_assignment;
    IF audience_kind_value='anyOfGroups' THEN INSERT INTO public.assignment_audience_group SELECT p_tenant,p_assignment,p_course,value::uuid FROM jsonb_array_elements_text(audience->'groups'); END IF;
    IF p_replace THEN
        -- The item and group tables share one unique position namespace.  Move
        -- both graphs out of that namespace first, then install the staged
        -- final positions.  This keeps a swap atomic under immediate unique
        -- constraints rather than relying on update ordering.
        IF item_count = 0 THEN
            DELETE FROM public.assignment_selection_candidate
             WHERE tenant_id=p_tenant AND assignment_id=p_assignment;
            DELETE FROM public.assignment_selection_group
             WHERE tenant_id=p_tenant AND assignment_id=p_assignment;
            DELETE FROM public.assignment_item
             WHERE tenant_id=p_tenant AND assignment_id=p_assignment;
        ELSE
            UPDATE public.assignment_item AS item SET position=item.position+1000000000 WHERE item.tenant_id=p_tenant AND item.assignment_id=p_assignment;
            UPDATE public.assignment_selection_group AS selection_group SET position=selection_group.position+1000000000 WHERE selection_group.tenant_id=p_tenant AND selection_group.assignment_id=p_assignment;
            UPDATE public.assignment_selection_candidate AS selection_candidate SET position=selection_candidate.position+1000000000 WHERE selection_candidate.tenant_id=p_tenant AND selection_candidate.selection_group_id IN (SELECT selection_group.selection_group_id FROM public.assignment_selection_group AS selection_group WHERE selection_group.tenant_id=p_tenant AND selection_group.assignment_id=p_assignment);
        END IF;
        FOR v_entry IN SELECT staged.entry FROM pg_temp.ple_definition_entry AS staged ORDER BY (staged.entry->>'position')::integer LOOP
            entry_position := (v_entry->>'position')::integer;
            IF v_entry->>'kind'='fixed' THEN
                UPDATE public.assignment_item AS item SET position=entry_position,points_possible=(v_entry->>'pointsPossible')::numeric,delivery_state=v_entry->>'deliveryState',scoring_mode=v_entry->>'scoringMode',revision=item.revision+1,updated_at=transaction_timestamp() WHERE item.tenant_id=p_tenant AND item.assignment_item_id=(v_entry->>'id')::uuid;
            ELSE
                group_id := (v_entry->>'id')::uuid;
                UPDATE public.assignment_selection_group AS selection_group SET position=entry_position,draw_count=(v_entry->>'drawCount')::integer,points_per_item=(v_entry->>'pointsPerItem')::numeric,ordering_policy=CASE WHEN v_entry->>'ordering'='candidateOrder' THEN 'candidate_order' ELSE v_entry->>'ordering' END,algorithm_version=(v_entry->>'algorithmVersion')::integer,revision=selection_group.revision+1,updated_at=transaction_timestamp() WHERE selection_group.tenant_id=p_tenant AND selection_group.selection_group_id=group_id;
                FOR v_candidate IN SELECT elements.value FROM jsonb_array_elements(v_entry->'candidates') AS elements LOOP
                    candidate_position := (v_candidate->>'position')::integer;
                    UPDATE public.assignment_selection_candidate AS selection_candidate SET position=candidate_position,delivery_state=v_candidate->>'deliveryState',updated_at=transaction_timestamp() WHERE selection_candidate.tenant_id=p_tenant AND selection_candidate.selection_group_id=group_id AND selection_candidate.candidate_id=(v_candidate->>'id')::uuid;
                END LOOP;
            END IF;
        END LOOP;
    ELSE
        FOR v_entry IN SELECT staged.entry FROM pg_temp.ple_definition_entry AS staged ORDER BY (staged.entry->>'position')::integer LOOP
            entry_position := (v_entry->>'position')::integer;
            IF v_entry->>'kind'='fixed' THEN
                item_id := (v_entry->>'id')::uuid;
                INSERT INTO public.assignment_item (tenant_id,assignment_id,assignment_item_id,position,problem_id,version_id,points_possible,delivery_state,scoring_mode) VALUES (p_tenant,p_assignment,item_id,entry_position,(v_entry->>'problemId')::uuid,(v_entry->>'versionId')::uuid,(v_entry->>'pointsPossible')::numeric,v_entry->>'deliveryState',v_entry->>'scoringMode');
            ELSE
                group_id := (v_entry->>'id')::uuid;
                INSERT INTO public.assignment_selection_group (tenant_id,assignment_id,selection_group_id,position,draw_count,points_per_item,ordering_policy,algorithm_version) VALUES (p_tenant,p_assignment,group_id,entry_position,(v_entry->>'drawCount')::integer,(v_entry->>'pointsPerItem')::numeric,CASE WHEN v_entry->>'ordering'='candidateOrder' THEN 'candidate_order' ELSE v_entry->>'ordering' END,(v_entry->>'algorithmVersion')::integer);
                FOR v_candidate IN SELECT elements.value FROM jsonb_array_elements(v_entry->'candidates') AS elements LOOP
                    candidate_position := (v_candidate->>'position')::integer; candidate_id := (v_candidate->>'id')::uuid;
                    INSERT INTO public.assignment_selection_candidate (tenant_id,assignment_id,selection_group_id,candidate_id,position,problem_id,version_id,delivery_state) VALUES (p_tenant,p_assignment,group_id,candidate_id,candidate_position,(v_candidate->>'problemId')::uuid,(v_candidate->>'versionId')::uuid,v_candidate->>'deliveryState');
                END LOOP;
            END IF;
        END LOOP;
    END IF;
    IF p_replace THEN
        SELECT new_revision INTO revision FROM public.ple_apply_verified_assignment_definition_revision(p_tenant,p_course,p_assignment,old_revision);
        IF old_title IS DISTINCT FROM title_value THEN
            UPDATE public.course_grade_scheme AS scheme SET revision=scheme.revision+1,updated_at=transaction_timestamp() WHERE scheme.tenant_id=p_tenant AND scheme.course_id=p_course;
        END IF;
        SELECT EXISTS (SELECT 1 FROM public.attempt_score_current WHERE tenant_id=p_tenant AND assignment_id=p_assignment) INTO has_scores;
        IF changed THEN
            UPDATE public.assignment AS assignment SET scoring_generation=assignment.scoring_generation+1,scoring_status=CASE WHEN has_scores THEN 'recalculating' ELSE 'current' END,updated_at=transaction_timestamp() WHERE assignment.tenant_id=p_tenant AND assignment.assignment_id=p_assignment RETURNING assignment.scoring_generation INTO scoring_generation;
        END IF;
        IF changed AND has_scores THEN
            IF p_recalculation_job IS NULL OR p_recalculation_max_attempts NOT BETWEEN 1 AND 20 THEN
                RAISE EXCEPTION 'server-owned recalculation job arguments are required' USING ERRCODE='22023';
            END IF;
            INSERT INTO public.worker_job (job_id,tenant_id,payload,state,max_attempts) VALUES (p_recalculation_job,p_tenant,jsonb_build_object('kind','recalculateAssignment','assignment',p_assignment::text,'generation',scoring_generation),'ready',p_recalculation_max_attempts);
        END IF;
    ELSE revision:=1; END IF;
    IF NOT p_replace THEN
        UPDATE public.course_grade_scheme AS scheme SET revision=scheme.revision+1,updated_at=transaction_timestamp()
         WHERE scheme.tenant_id=p_tenant AND scheme.course_id=p_course;
    END IF;
    SELECT assignment.scoring_generation,assignment.scoring_status INTO scoring_generation,scoring_status FROM public.assignment AS assignment WHERE assignment.tenant_id=p_tenant AND assignment.assignment_id=p_assignment;
    RETURN NEXT;
END
$$;
-- Teaching settings are retained for focused schedule work. Publication must
-- obey the same readiness rule as every complete definition mutation.
CREATE OR REPLACE FUNCTION public.ple_put_assignment_teaching_settings(
    p_tenant uuid,p_actor uuid,p_course uuid,p_assignment uuid,p_expected_revision bigint,p_settings jsonb
) RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE policy jsonb; current_lifecycle text; next_lifecycle text; instructions_value text;
DECLARE available_value timestamptz; due_value timestamptz; closes_value timestamptz;
DECLARE late_value text; deadline_value text; time_limit_value integer; attempt_limit_value integer;
DECLARE term_start date; term_end date; course_zone text;
BEGIN
    PERFORM public.ple_assignment_mutator_require_editor(p_tenant,p_actor,p_course,p_assignment,p_expected_revision);
    PERFORM public.ple_assignment_mutator_closed_object(
        p_settings,ARRAY['lifecycle','instructions','basePolicy'],524288);
    IF NOT (p_settings ?& ARRAY['lifecycle','instructions','basePolicy'])
       OR jsonb_typeof(p_settings->'lifecycle') <> 'string'
       OR jsonb_typeof(p_settings->'instructions') <> 'string' THEN
        RAISE EXCEPTION 'assignment teaching settings are incomplete' USING ERRCODE='22023';
    END IF;
    policy := p_settings->'basePolicy';
    PERFORM public.ple_assignment_mutator_closed_object(
        policy,ARRAY['availableAt','dueAt','closesAt','lateSubmission','deadlineBehavior','timeLimitSeconds','attemptLimit'],262144);
    IF NOT (policy ?& ARRAY['availableAt','dueAt','closesAt','lateSubmission','deadlineBehavior','timeLimitSeconds','attemptLimit']) THEN
        RAISE EXCEPTION 'assignment base policy is incomplete' USING ERRCODE='22023';
    END IF;
    instructions_value := p_settings->>'instructions';
    IF char_length(instructions_value) > 50000 THEN
        RAISE EXCEPTION 'assignment instructions exceed the maximum length' USING ERRCODE='22023';
    END IF;
    next_lifecycle := p_settings->>'lifecycle';
    IF next_lifecycle NOT IN ('draft','published','closed','archived') THEN
        RAISE EXCEPTION 'assignment lifecycle is invalid' USING ERRCODE='22023';
    END IF;
    late_value := policy->>'lateSubmission'; deadline_value := policy->>'deadlineBehavior';
    IF late_value NOT IN ('accept','markLate','reject') OR deadline_value <> 'autoSubmit' THEN
        RAISE EXCEPTION 'assignment base policy enum is invalid' USING ERRCODE='22023';
    END IF;
    available_value := public.ple_assignment_mutator_millis(policy->'availableAt');
    due_value := public.ple_assignment_mutator_millis(policy->'dueAt');
    closes_value := public.ple_assignment_mutator_millis(policy->'closesAt');
    IF jsonb_typeof(policy->'timeLimitSeconds') = 'null' THEN time_limit_value := NULL;
    ELSIF jsonb_typeof(policy->'timeLimitSeconds') = 'number'
       AND (policy->>'timeLimitSeconds') ~ '^[0-9]+$'
       AND (policy->>'timeLimitSeconds')::numeric BETWEEN 1 AND 2147483647 THEN
        time_limit_value := (policy->>'timeLimitSeconds')::integer;
    ELSE RAISE EXCEPTION 'assignment time limit is invalid' USING ERRCODE='22023'; END IF;
    IF jsonb_typeof(policy->'attemptLimit') = 'null' THEN attempt_limit_value := NULL;
    ELSIF jsonb_typeof(policy->'attemptLimit') = 'number'
       AND (policy->>'attemptLimit') ~ '^[0-9]+$'
       AND (policy->>'attemptLimit')::numeric BETWEEN 1 AND 2147483647 THEN
        attempt_limit_value := (policy->>'attemptLimit')::integer;
    ELSE RAISE EXCEPTION 'assignment attempt limit is invalid' USING ERRCODE='22023'; END IF;
    IF (available_value IS NOT NULL AND due_value IS NOT NULL AND available_value > due_value)
       OR (due_value IS NOT NULL AND closes_value IS NOT NULL AND due_value > closes_value)
       OR (available_value IS NOT NULL AND closes_value IS NOT NULL AND available_value > closes_value) THEN
        RAISE EXCEPTION 'assignment base policy schedule is invalid' USING ERRCODE='22023';
    END IF;
    SELECT lifecycle INTO current_lifecycle FROM public.assignment
     WHERE tenant_id=p_tenant AND course_id=p_course AND assignment_id=p_assignment FOR UPDATE;
    IF NOT FOUND OR NOT ((current_lifecycle='draft' AND next_lifecycle IN ('draft','published','archived'))
       OR (current_lifecycle='published' AND next_lifecycle IN ('published','closed','archived'))
       OR (current_lifecycle='closed' AND next_lifecycle IN ('closed','published','archived'))
       OR (current_lifecycle='archived' AND next_lifecycle='archived')) THEN
        RAISE EXCEPTION 'assignment lifecycle transition is invalid' USING ERRCODE='22023';
    END IF;
    IF next_lifecycle = 'published' AND NOT (
        EXISTS (
            SELECT 1 FROM public.assignment_item
             WHERE tenant_id=p_tenant AND assignment_id=p_assignment AND delivery_state='active'
        ) OR EXISTS (
            SELECT 1 FROM public.assignment_selection_group AS selection_group
             JOIN public.assignment_selection_candidate AS candidate
               ON candidate.tenant_id=selection_group.tenant_id
              AND candidate.selection_group_id=selection_group.selection_group_id
             WHERE selection_group.tenant_id=p_tenant
               AND selection_group.assignment_id=p_assignment
             GROUP BY selection_group.selection_group_id, selection_group.draw_count
            HAVING count(*) FILTER (WHERE candidate.delivery_state='active') >= selection_group.draw_count
        )
    ) THEN
        RAISE EXCEPTION 'published assignments require an active deliverable position'
            USING ERRCODE='22023';
    END IF;
    SELECT term_start_date,term_end_date,time_zone INTO term_start,term_end,course_zone
      FROM public.course WHERE tenant_id=p_tenant AND course_id=p_course FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'course is unavailable' USING ERRCODE='42501'; END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name=course_zone) THEN
        RAISE EXCEPTION 'course time zone is invalid' USING ERRCODE='55000';
    END IF;
    IF (available_value IS NOT NULL AND (available_value AT TIME ZONE course_zone)::date NOT BETWEEN term_start AND term_end)
       OR (due_value IS NOT NULL AND (due_value AT TIME ZONE course_zone)::date NOT BETWEEN term_start AND term_end)
       OR (closes_value IS NOT NULL AND (closes_value AT TIME ZONE course_zone)::date NOT BETWEEN term_start AND term_end) THEN
        RAISE EXCEPTION 'assignment base policy is outside the course term' USING ERRCODE='22023';
    END IF;
    UPDATE public.assignment SET lifecycle=next_lifecycle,instructions=instructions_value,
        updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND assignment_id=p_assignment;
    INSERT INTO public.assignment_effective_policy_base
        (tenant_id,assignment_id,course_id,available_at,due_at,closes_at,late_submission_policy,deadline_behavior,time_limit_seconds,attempt_limit)
    VALUES (p_tenant,p_assignment,p_course,available_value,due_value,closes_value,
        CASE late_value WHEN 'markLate' THEN 'mark_late' ELSE late_value END,'auto_submit',time_limit_value,attempt_limit_value)
    ON CONFLICT (tenant_id,assignment_id) DO UPDATE SET course_id=EXCLUDED.course_id,
        available_at=EXCLUDED.available_at,due_at=EXCLUDED.due_at,closes_at=EXCLUDED.closes_at,
        late_submission_policy=EXCLUDED.late_submission_policy,deadline_behavior=EXCLUDED.deadline_behavior,
        time_limit_seconds=EXCLUDED.time_limit_seconds,attempt_limit=EXCLUDED.attempt_limit,
        updated_at=transaction_timestamp();
    RETURN public.ple_assignment_mutator_finish(p_tenant,p_course,p_assignment,p_expected_revision);
END
$$;

CREATE OR REPLACE FUNCTION public.ple_replace_unissued_assignment_definition_v1(
    p_tenant uuid, p_actor uuid, p_course uuid, p_assignment uuid,
    p_expected_revision bigint, p_payload jsonb
) RETURNS TABLE(outcome text, revision bigint, scoring_generation bigint, scoring_status text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_message text;
BEGIN
    PERFORM public.ple_reset_assignment_definition_scratch_v1();
    PERFORM public.ple_assignment_mutator_require_editor(
        p_tenant, p_actor, p_course, p_assignment, p_expected_revision
    );
    IF EXISTS (
        SELECT 1 FROM public.assignment_run AS run
        JOIN public.enrollment AS enrollment
          ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id
        WHERE run.tenant_id=p_tenant AND enrollment.assignment_id=p_assignment
    ) THEN
        outcome := 'issued';
        RETURN NEXT;
        RETURN;
    END IF;
    IF p_payload IS NOT NULL AND jsonb_typeof(p_payload->'entries')='array'
       AND jsonb_array_length(p_payload->'entries')=0 THEN
        SELECT result.revision, result.scoring_generation, result.scoring_status
          INTO revision, scoring_generation, scoring_status
          FROM public.ple_replace_assignment_definition_v1(
              p_tenant,p_actor,p_course,p_assignment,p_expected_revision,p_payload,NULL,NULL
          ) AS result;
        outcome := 'replaced';
        RETURN NEXT;
        RETURN;
    END IF;
    RETURN QUERY
    SELECT result.outcome, result.revision, result.scoring_generation, result.scoring_status
      FROM public.ple_replace_unissued_assignment_definition_impl_v1(
          p_tenant,p_actor,p_course,p_assignment,p_expected_revision,p_payload
      ) AS result;
EXCEPTION WHEN SQLSTATE '55000' THEN
    GET STACKED DIAGNOSTICS v_message = MESSAGE_TEXT;
    IF v_message = 'assignment revision is stale' THEN
        outcome := 'revisionConflict';
        RETURN NEXT;
        RETURN;
    END IF;
    RAISE;
END
$$;

ALTER FUNCTION public.ple_assignment_definition_apply_v1(
    uuid,uuid,uuid,jsonb,boolean,uuid,integer
) OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_put_assignment_teaching_settings(uuid,uuid,uuid,uuid,bigint,jsonb)
    OWNER TO ple_assignment_mutator_broker;
ALTER FUNCTION public.ple_replace_unissued_assignment_definition_v1(uuid,uuid,uuid,uuid,bigint,jsonb)
    OWNER TO ple_assignment_mutator_broker;

REVOKE ALL ON FUNCTION public.ple_assignment_definition_apply_v1(
    uuid,uuid,uuid,jsonb,boolean,uuid,integer
) FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_put_assignment_teaching_settings(uuid,uuid,uuid,uuid,bigint,jsonb)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_put_assignment_teaching_settings(uuid,uuid,uuid,uuid,bigint,jsonb)
    TO ple_app;
REVOKE ALL ON FUNCTION public.ple_replace_unissued_assignment_definition_v1(uuid,uuid,uuid,uuid,bigint,jsonb)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_replace_unissued_assignment_definition_v1(uuid,uuid,uuid,uuid,bigint,jsonb)
    TO ple_app;

DO $$
BEGIN
    IF has_function_privilege(
           'public',
           'public.ple_assignment_definition_apply_v1(uuid,uuid,uuid,jsonb,boolean,uuid,integer)'::regprocedure,
           'EXECUTE'
       )
       OR has_function_privilege(
           'ple_app',
           'public.ple_assignment_definition_apply_v1(uuid,uuid,uuid,jsonb,boolean,uuid,integer)'::regprocedure,
           'EXECUTE'
       )
       OR has_table_privilege('ple_app', 'public.assignment', 'INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_app', 'public.assignment_item', 'INSERT,UPDATE,DELETE') THEN
        RAISE EXCEPTION 'assignment workspace draft capability grants are unsafe';
    END IF;
END
$$;

COMMIT;
