-- WP-PROF-B2: assignment-scoped curriculum-adoption materializers.
--
-- Rust/qmodel owns semantic normalization, pin substitution, canonical bytes,
-- and calendar/DST resolution.  This broker rechecks locked relational
-- witnesses, turns an ID-free assignment plan into the existing ordinary
-- assignment command payload, and records durable receipt/evidence state.

BEGIN;

-- Fast-forward holds the exact witnessed assignment row while the existing
-- assignment mutator applies the replacement.  PostgreSQL requires one UPDATE
-- column to acquire FOR UPDATE; forced RLS exposes no broker UPDATE policy, so
-- this key-column grant is lock authority rather than mutation authority.
GRANT UPDATE(assignment_id) ON public.assignment
    TO ple_curriculum_adoption_broker;

-- The ordinary assignment writer requires storage-owned entry identities.
-- This conversion accepts only the Rust-private materialization vocabulary;
-- all learner-visible policy values continue through the established writer.
CREATE FUNCTION public.ple_caa_definition_payload_v1(p_materialization jsonb)
RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_entry jsonb; v_entries jsonb := '[]'::jsonb; v_candidates jsonb;
DECLARE v_schedule jsonb; v_defaults jsonb; v_value jsonb;
BEGIN
    PERFORM public.ple_cam_require_exact_object_v1(
        p_materialization, ARRAY['title','instructions','entries','defaults','schedule'], 524288
    );
    IF jsonb_typeof(p_materialization->'title') <> 'string'
       OR jsonb_typeof(p_materialization->'instructions') <> 'string'
       OR jsonb_typeof(p_materialization->'entries') <> 'array'
       OR jsonb_array_length(p_materialization->'entries') NOT BETWEEN 1 AND 1024
       OR jsonb_typeof(p_materialization->'defaults') <> 'object'
       OR jsonb_typeof(p_materialization->'schedule') <> 'object' THEN
        RAISE EXCEPTION 'curriculum adoption assignment plan is invalid' USING ERRCODE = '22023';
    END IF;
    v_defaults := p_materialization->'defaults';
    v_schedule := p_materialization->'schedule';
    PERFORM public.ple_cam_require_exact_object_v1(
        v_defaults,
        ARRAY['timeLimitSeconds','attemptLimit','lateSubmission','deadlineBehavior','runPolicies','learnerDisclosure'],
        16384
    );
    PERFORM public.ple_cam_require_exact_object_v1(
        v_schedule, ARRAY['timeZone','availableAt','dueAt','closesAt'], 16384
    );
    IF jsonb_typeof(v_defaults->'runPolicies') <> 'object'
       OR jsonb_typeof(v_defaults->'learnerDisclosure') <> 'object'
       OR jsonb_typeof(v_schedule->'timeZone') <> 'string' THEN
        RAISE EXCEPTION 'curriculum adoption assignment plan is invalid' USING ERRCODE = '22023';
    END IF;
    FOR v_entry IN SELECT value FROM jsonb_array_elements(p_materialization->'entries')
    LOOP
        IF jsonb_typeof(v_entry) <> 'object'
           OR v_entry->>'kind' NOT IN ('fixed','pool')
           OR jsonb_typeof(v_entry->'position') <> 'number'
           OR v_entry->>'position' !~ '^(0|[1-9][0-9]{0,3})$' THEN
            RAISE EXCEPTION 'curriculum adoption assignment entry is invalid' USING ERRCODE = '22023';
        END IF;
        IF v_entry->>'kind' = 'fixed' THEN
            PERFORM public.ple_cam_require_exact_object_v1(
                v_entry, ARRAY['kind','position','reference','pointsPossible','scoringMode'], 4096
            );
            PERFORM public.ple_cam_require_exact_object_v1(v_entry->'reference', ARRAY['problem','version'], 4096);
            v_value := jsonb_build_object(
                'kind','fixed','id',gen_random_uuid(),'position',v_entry->'position',
                'problemId',v_entry#>>'{reference,problem}','versionId',v_entry#>>'{reference,version}',
                'pointsPossible',v_entry->'pointsPossible','deliveryState','active',
                'scoringMode',v_entry->'scoringMode'
            );
        ELSE
            PERFORM public.ple_cam_require_exact_object_v1(
                v_entry,
                ARRAY['kind','position','candidates','drawCount','pointsPerItem','ordering','algorithm'],
                262144
            );
            IF jsonb_typeof(v_entry->'candidates') <> 'array'
               OR jsonb_array_length(v_entry->'candidates') NOT BETWEEN 1 AND 1024
               OR v_entry->>'algorithm' <> 'v1' THEN
                RAISE EXCEPTION 'curriculum adoption assignment pool is invalid' USING ERRCODE = '22023';
            END IF;
            SELECT coalesce(jsonb_agg(jsonb_build_object(
                'id',gen_random_uuid(),'position',candidate.value->'position',
                'problemId',candidate.value#>>'{reference,problem}',
                'versionId',candidate.value#>>'{reference,version}','deliveryState','active'
            ) ORDER BY candidate.ordinality), '[]'::jsonb)
              INTO v_candidates
              FROM jsonb_array_elements(v_entry->'candidates') WITH ORDINALITY AS candidate(value, ordinality);
            IF EXISTS (
                SELECT 1 FROM jsonb_array_elements(v_entry->'candidates') AS candidate(value)
                 WHERE jsonb_typeof(candidate.value) <> 'object'
                    OR NOT candidate.value ?& ARRAY['position','reference']
            ) THEN
                RAISE EXCEPTION 'curriculum adoption assignment pool is invalid' USING ERRCODE = '22023';
            END IF;
            v_value := jsonb_build_object(
                'kind','selectionGroup','id',gen_random_uuid(),'position',v_entry->'position',
                'drawCount',v_entry->'drawCount','pointsPerItem',v_entry->'pointsPerItem',
                'ordering',v_entry->'ordering','algorithmVersion',1,'candidates',v_candidates
            );
        END IF;
        v_entries := v_entries || jsonb_build_array(v_value);
    END LOOP;
    RETURN jsonb_build_object(
        'schemaVersion',1,'title',p_materialization->'title','lifecycle','draft',
        'instructions',p_materialization->'instructions','entries',v_entries,
        'policies',v_defaults->'runPolicies','disclosurePolicy',v_defaults->'learnerDisclosure',
        'basePolicy',jsonb_build_object(
            'availableAt', CASE WHEN v_schedule->'availableAt' = 'null'::jsonb THEN NULL
                ELSE v_schedule#>'{availableAt,timestamp}' END,
            'dueAt', CASE WHEN v_schedule->'dueAt' = 'null'::jsonb THEN NULL
                ELSE v_schedule#>'{dueAt,timestamp}' END,
            'closesAt', CASE WHEN v_schedule->'closesAt' = 'null'::jsonb THEN NULL
                ELSE v_schedule#>'{closesAt,timestamp}' END,
            'lateSubmission',v_defaults->'lateSubmission','deadlineBehavior',v_defaults->'deadlineBehavior',
            'timeLimitSeconds',v_defaults->'timeLimitSeconds','attemptLimit',v_defaults->'attemptLimit'
        ),
        'audience',jsonb_build_object('kind','courseWide')
    );
END $$;

CREATE FUNCTION public.ple_caa_assignment_plan_source_v1(p_source jsonb)
RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_source jsonb; v_kind text;
BEGIN
    PERFORM public.ple_cam_require_exact_object_v1(p_source, ARRAY['kind','source'], 4096);
    v_kind := p_source->>'kind';
    IF v_kind = 'assignment' THEN
        v_source := p_source->'source';
        IF jsonb_typeof(v_source) <> 'object' OR v_source->>'kind' NOT IN ('blueprint','alpha') THEN
            RAISE EXCEPTION 'curriculum adoption assignment source is invalid' USING ERRCODE = '22023';
        END IF;
        v_kind := v_source->>'kind';
    ELSE
        v_source := p_source->'source';
    END IF;
    IF v_kind NOT IN ('blueprint','alpha') OR jsonb_typeof(v_source) <> 'object' THEN
        RAISE EXCEPTION 'curriculum adoption assignment source is invalid' USING ERRCODE = '22023';
    END IF;
    IF v_kind = 'blueprint' THEN
        PERFORM public.ple_cam_require_exact_object_v1(v_source,
            CASE WHEN p_source->>'kind' = 'assignment'
                THEN ARRAY['kind','reference','revision']
                ELSE ARRAY['reference','revision'] END, 4096);
        PERFORM public.ple_curriculum_adoption_route_number_v1(v_source->'reference', 'BP');
    ELSE
        PERFORM public.ple_cam_require_exact_object_v1(
            v_source, CASE WHEN p_source->>'kind' = 'assignment'
                THEN ARRAY['kind','reference','revision','moduleIndex','assignmentIndex']
                ELSE ARRAY['reference','revision','moduleIndex','assignmentIndex'] END, 4096
        );
        PERFORM public.ple_curriculum_adoption_route_number_v1(v_source->'reference', 'AC');
    END IF;
    PERFORM public.ple_cam_positive_revision_v1(v_source->'revision');
    RETURN jsonb_build_object('kind',v_kind,'source',v_source);
END $$;

CREATE FUNCTION public.ple_caa_assignment_provenance_v1(p_source jsonb)
RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_source jsonb;
BEGIN
    v_source := public.ple_caa_assignment_plan_source_v1(p_source);
    IF v_source->>'kind' = 'blueprint' THEN
        RETURN jsonb_build_object('kind','blueprint','reference',v_source#>'{source,reference}','revision',v_source#>'{source,revision}');
    END IF;
    RETURN jsonb_build_object('kind','alpha','alphaCourseId',(
        SELECT alpha_course_id FROM public.alpha_course
         WHERE alpha_course_reference = public.ple_curriculum_adoption_route_number_v1(v_source#>'{source,reference}','AC')
    ),'revision',v_source#>'{source,revision}','moduleIndex',v_source#>'{source,moduleIndex}','assignmentIndex',v_source#>'{source,assignmentIndex}');
END $$;

-- ASVS 2.3.1/2.3.3: consume the one-use preparation before any new
-- assignment UUID is allocated.  A matching replay returns before all writes.
CREATE FUNCTION public.ple_caa_apply_assignment_v1(
    p_tenant uuid, p_session character(64), p_preparation uuid, p_envelope jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_consumed jsonb; v_plan jsonb; v_operation text; v_actor uuid; v_key text;
DECLARE v_digest bytea; v_replay jsonb; v_course uuid; v_assignment uuid;
DECLARE v_revision bigint; v_import_revision bigint; v_source jsonb; v_provenance jsonb;
DECLARE v_outcome text; v_expected bigint; v_imported_source jsonb; v_current_source jsonb;
BEGIN
    v_consumed := public.ple_cam_consume_materialization_preparation_v1(
        p_tenant, p_session, p_preparation, p_envelope
    );
    v_operation := v_consumed->>'operation';
    IF v_operation NOT IN ('applyBlueprintInstantiation','createSourceDerivedAssignment',
                           'applyAssignmentFastForward') THEN
        RAISE EXCEPTION 'curriculum adoption assignment operation is invalid' USING ERRCODE = '22023';
    END IF;
    v_actor := public.ple_cam_uuid_v1(v_consumed->'actor');
    v_key := v_consumed->>'idempotencyKey';
    v_digest := public.ple_cam_digest_bytes_v1(v_consumed->'requestSha256');
    v_replay := public.ple_cam_select_receipt_v1(p_tenant, v_key,
        CASE v_operation WHEN 'applyBlueprintInstantiation' THEN 'blueprintInstantiation'
                         WHEN 'createSourceDerivedAssignment' THEN 'sourceDerivedAssignment'
                         ELSE 'assignmentFastForward' END,
        v_actor, v_digest);
    IF v_replay IS NOT NULL THEN RETURN v_replay; END IF;
    v_plan := v_consumed->'plan'->'plan';
    IF jsonb_typeof(v_plan) <> 'object' THEN
        RAISE EXCEPTION 'curriculum adoption assignment plan is invalid' USING ERRCODE = '22023';
    END IF;
    PERFORM public.ple_cam_validate_semantic_v1(v_plan->'semantic');
    IF v_operation = 'applyAssignmentFastForward' THEN
        PERFORM public.ple_cam_require_exact_object_v1(v_plan,
            ARRAY['semantic','witness','assignment','expectedAssignmentRevision','expectedImportRevision','targetTerm','materialization'],
            524288
        );
        v_course := public.ple_curriculum_adoption_lock_course_v1(p_tenant, v_actor, v_plan->'witness'->'course');
        PERFORM public.ple_cac_require_witness_v1(p_tenant, v_course, v_plan->'witness');
        v_assignment := (SELECT assignment_id FROM public.assignment WHERE tenant_id=p_tenant
            AND course_id=v_course AND public_id=public.ple_curriculum_adoption_route_number_v1(v_plan->'assignment','A') FOR UPDATE);
        v_expected := public.ple_cam_positive_revision_v1(v_plan->'expectedAssignmentRevision');
        IF v_assignment IS NULL OR (SELECT revision FROM public.assignment WHERE tenant_id=p_tenant AND assignment_id=v_assignment) <> v_expected THEN
            RAISE EXCEPTION 'curriculum adoption assignment witness is stale' USING ERRCODE = 'PBC01';
        END IF;
        v_imported_source := v_consumed#>'{facts,import,destination,importedSource}';
        v_current_source := v_consumed#>'{facts,import,source,currentSource}';
        IF v_imported_source IS NULL OR v_current_source IS NULL
           OR v_imported_source->>'kind' IS DISTINCT FROM v_current_source->>'kind'
           OR v_imported_source->>'reference' IS DISTINCT FROM v_current_source->>'reference'
           OR (v_imported_source->>'kind' = 'alpha' AND (
               v_imported_source->'moduleIndex' IS DISTINCT FROM v_current_source->'moduleIndex'
               OR v_imported_source->'assignmentIndex' IS DISTINCT FROM v_current_source->'assignmentIndex'
           )) THEN
            RAISE EXCEPTION 'curriculum adoption fast-forward source is stale' USING ERRCODE = 'PBC01';
        END IF;
        PERFORM public.ple_cac_reusable_document_v1(p_tenant,p_session,v_current_source);
        PERFORM 1 FROM public.curriculum_assignment_import_current AS current_row
          JOIN public.curriculum_assignment_adoption_evidence AS evidence
            ON (evidence.tenant_id,evidence.receipt_key,evidence.assignment_id) =
               (current_row.tenant_id,current_row.receipt_key,current_row.assignment_id)
         WHERE current_row.tenant_id=p_tenant AND current_row.assignment_id=v_assignment
           AND evidence.import_revision=public.ple_cam_positive_revision_v1(v_plan->'expectedImportRevision')
           AND ((v_imported_source->>'kind'='blueprint'
                 AND evidence.source_kind='blueprint'
                 AND ('BP-' || evidence.source_blueprint_reference::text)=v_imported_source->>'reference'
                 AND evidence.source_blueprint_revision::text=v_imported_source->>'revision')
             OR (v_imported_source->>'kind'='alpha'
                 AND evidence.source_kind='alpha'
                 AND ('AC-' || (SELECT alpha_course_reference::text FROM public.alpha_course
                                  WHERE alpha_course_id=evidence.source_alpha_course_id))=v_imported_source->>'reference'
                 AND evidence.source_alpha_revision::text=v_imported_source->>'revision'
                 AND evidence.source_module_position=(v_imported_source->>'moduleIndex')::integer
                 AND evidence.source_definition_position=(v_imported_source->>'assignmentIndex')::integer))
         FOR KEY SHARE OF current_row,evidence;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'curriculum adoption fast-forward baseline is stale' USING ERRCODE = 'PBC01';
        END IF;
        SELECT outcome, revision INTO v_outcome, v_revision FROM public.ple_replace_unissued_assignment_definition_v1(
            p_tenant,v_actor,v_course,v_assignment,v_expected,public.ple_caa_definition_payload_v1(v_plan->'materialization')
        );
        IF v_outcome <> 'replaced' OR v_revision IS NULL THEN
            RAISE EXCEPTION 'curriculum adoption assignment cannot be fast-forwarded' USING ERRCODE = 'PBC01';
        END IF;
        v_import_revision := public.ple_cam_positive_revision_v1(v_plan->'expectedImportRevision') + 1;
        PERFORM public.ple_cam_insert_receipt_v1(p_tenant,v_key,'assignmentFastForward',v_actor,v_digest,
            v_course,v_assignment,NULL,NULL,NULL,v_import_revision,NULL);
        v_provenance := public.ple_caa_assignment_provenance_v1(
            jsonb_build_object('kind','assignment','source',v_current_source)
        );
    ELSE
        PERFORM public.ple_cam_require_exact_object_v1(v_plan,
            ARRAY['semantic','source','destinationWitness','targetTerm','preview','corrections','materialization'],
            524288
        );
        IF v_plan->'corrections' <> '[]'::jsonb THEN
            RAISE EXCEPTION 'curriculum adoption assignment schedule requires correction' USING ERRCODE = 'PBC01';
        END IF;
        v_course := public.ple_curriculum_adoption_lock_course_v1(p_tenant,v_actor,v_plan->'destinationWitness'->'course');
        PERFORM public.ple_cac_require_witness_v1(p_tenant,v_course,v_plan->'destinationWitness');
        v_source := public.ple_caa_assignment_plan_source_v1(v_plan->'source');
        PERFORM public.ple_cac_reusable_document_v1(p_tenant,p_session,v_source->'source');
        v_assignment := gen_random_uuid();
        SELECT assignment_id, revision INTO v_assignment,v_revision
          FROM public.ple_create_assignment_definition_v1(p_tenant,v_actor,v_course,v_assignment,
              public.ple_caa_definition_payload_v1(v_plan->'materialization'),NULL,NULL);
        IF NOT FOUND THEN RAISE EXCEPTION 'curriculum adoption assignment creation is unavailable' USING ERRCODE='PBI01'; END IF;
        PERFORM public.ple_cam_insert_receipt_v1(p_tenant,v_key,
            CASE v_operation WHEN 'applyBlueprintInstantiation' THEN 'blueprintInstantiation' ELSE 'sourceDerivedAssignment' END,
            v_actor,v_digest,v_course,v_assignment,NULL,NULL,NULL,NULL,
            CASE WHEN v_operation='applyBlueprintInstantiation' THEN v_plan->'targetTerm' ELSE NULL END);
        v_import_revision := 1;
        v_provenance := public.ple_caa_assignment_provenance_v1(v_plan->'source');
    END IF;
    INSERT INTO public.curriculum_adoption_receipt_assignment(
        tenant_id,receipt_key,operation,course_id,assignment_id,single_destination_assignment_id
    ) VALUES (p_tenant,v_key,CASE v_operation WHEN 'applyBlueprintInstantiation' THEN 'blueprintInstantiation'
                 WHEN 'createSourceDerivedAssignment' THEN 'sourceDerivedAssignment' ELSE 'assignmentFastForward' END,
              v_course,v_assignment,v_assignment);
    PERFORM public.ple_cam_insert_evidence_v1(p_tenant,v_key,v_course,v_assignment,v_import_revision,
        v_plan#>'{semantic,semanticInput}',(v_plan#>>'{semantic,canonicalVersion}')::integer,
        public.ple_cam_bytes_v1(v_plan#>'{semantic,canonicalBytes}',1,524288),
        public.ple_cam_digest_bytes_v1(v_plan#>'{semantic,semanticDigest}'),v_provenance);
    PERFORM public.ple_cam_upsert_current_v1(p_tenant,v_assignment,v_key);
    RETURN public.ple_cam_receipt_result_v1(p_tenant,v_key,false);
END $$;

-- Alpha aggregate writes stay in the reusable-curriculum authority boundary.
-- The adoption broker receives only an internal/result identity, never Alpha
-- table or sequence authority.
CREATE FUNCTION public.ple_materialize_alpha_fork_v1(
    p_tenant uuid, p_session character(64), p_creator uuid, p_semantic jsonb
) RETURNS TABLE(alpha_course_id uuid, alpha_course_reference integer, revision bigint)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_actor uuid; v_alpha jsonb; v_row public.alpha_course%ROWTYPE; v_byline text[];
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_creator IS NULL
       OR jsonb_typeof(p_semantic) <> 'object' OR p_semantic->>'kind' <> 'course'
       OR NOT p_semantic ?& ARRAY['kind','title','modules']
       OR (SELECT count(*) FROM jsonb_object_keys(p_semantic)) <> 3
       OR jsonb_typeof(p_semantic->'title') <> 'string'
       OR jsonb_typeof(p_semantic->'modules') <> 'array'
       OR jsonb_array_length(p_semantic->'modules') NOT BETWEEN 1 AND 1024
       OR EXISTS (
            SELECT 1 FROM jsonb_array_elements(p_semantic->'modules') AS module(value)
             WHERE jsonb_typeof(module.value) <> 'object'
                OR NOT module.value ?& ARRAY['label','assignments']
                OR (SELECT count(*) FROM jsonb_object_keys(module.value)) <> 2
                OR jsonb_typeof(module.value->'assignments') <> 'array'
                OR jsonb_array_length(module.value->'assignments') NOT BETWEEN 1 AND 1024
       ) THEN
        RAISE EXCEPTION 'Alpha fork materialization is invalid' USING ERRCODE='22023';
    END IF;
    SELECT user_id INTO v_actor FROM public.ple_reusable_curriculum_instructor_actor(p_session,p_tenant);
    IF NOT FOUND OR v_actor IS DISTINCT FROM p_creator THEN
        RAISE EXCEPTION 'Alpha fork creator authority is unavailable' USING ERRCODE='42501';
    END IF;
    SELECT ARRAY[account.display_name] INTO v_byline FROM public.ple_account AS account
     WHERE account.user_id=v_actor;
    IF NOT FOUND OR NOT public.ple_valid_public_byline(v_byline) THEN
        RAISE EXCEPTION 'Alpha fork creator byline is unavailable' USING ERRCODE='42501';
    END IF;
    IF EXISTS (
        SELECT 1 FROM jsonb_array_elements(p_semantic->'modules') AS module(value)
        CROSS JOIN LATERAL jsonb_array_elements(module.value->'assignments') AS definition(value)
        CROSS JOIN LATERAL jsonb_array_elements(definition.value->'entries') AS entry(value)
        LEFT JOIN public.catalog_search_document AS fixed_document
          ON entry.value->>'kind'='fixed' AND (fixed_document.problem_id,fixed_document.version_id)=
             ((entry.value#>>'{reference,problem}')::uuid,(entry.value#>>'{reference,version}')::uuid)
        LEFT JOIN LATERAL jsonb_array_elements(CASE WHEN entry.value->>'kind'='pool'
                                                    THEN entry.value->'candidates' ELSE '[]'::jsonb END)
          AS candidate(value) ON true
        LEFT JOIN public.catalog_search_document AS candidate_document
          ON candidate.value IS NOT NULL AND (candidate_document.problem_id,candidate_document.version_id)=
             ((candidate.value#>>'{reference,problem}')::uuid,(candidate.value#>>'{reference,version}')::uuid)
        WHERE (entry.value->>'kind'='fixed' AND fixed_document.problem_id IS NULL)
           OR (entry.value->>'kind'='pool' AND candidate_document.problem_id IS NULL)
    ) THEN
        RAISE EXCEPTION 'Alpha fork source pin is unavailable' USING ERRCODE='42501';
    END IF;
    SELECT jsonb_build_object('title',p_semantic->'title','modules',coalesce(jsonb_agg(
        jsonb_build_object('label',module.value->'label','definitions',coalesce((
            SELECT jsonb_agg((definition.value-'entries') || jsonb_build_object('entries',coalesce((
                SELECT jsonb_agg(CASE entry.value->>'kind'
                    WHEN 'fixed' THEN jsonb_build_object('kind','fixed','questionId',
                        substr(document.question_id::text,1,3)||'-'||substr(document.question_id::text,4),
                        'pointsPossible',entry.value->'pointsPossible','scoringMode',entry.value->'scoringMode')
                    WHEN 'pool' THEN jsonb_build_object('kind','pool','candidates',coalesce((
                        SELECT jsonb_agg(substr(candidate_document.question_id::text,1,3)||'-'||substr(candidate_document.question_id::text,4)
                            ORDER BY candidate.ordinality)
                          FROM jsonb_array_elements(entry.value->'candidates') WITH ORDINALITY AS candidate(value,ordinality)
                          JOIN public.catalog_search_document AS candidate_document
                            ON (candidate_document.problem_id,candidate_document.version_id)=
                               ((candidate.value#>>'{reference,problem}')::uuid,(candidate.value#>>'{reference,version}')::uuid)
                    ),'[]'::jsonb),'drawCount',entry.value->'drawCount','pointsPerItem',entry.value->'pointsPerItem',
                        'ordering',entry.value->'ordering','algorithm',entry.value->'algorithm')
                END ORDER BY entry.ordinality)
                  FROM jsonb_array_elements(definition.value->'entries') WITH ORDINALITY AS entry(value,ordinality)
                  LEFT JOIN public.catalog_search_document AS document
                    ON entry.value->>'kind'='fixed' AND (document.problem_id,document.version_id)=
                       ((entry.value#>>'{reference,problem}')::uuid,(entry.value#>>'{reference,version}')::uuid)
            ),'[]'::jsonb)) ORDER BY definition.ordinality)
              FROM jsonb_array_elements(module.value->'assignments') WITH ORDINALITY AS definition(value,ordinality)
        ),'[]'::jsonb)) ORDER BY module.ordinality),'[]'::jsonb)) INTO v_alpha
      FROM jsonb_array_elements(p_semantic->'modules') WITH ORDINALITY AS module(value,ordinality);
    IF NOT public.ple_reusable_alpha_v1_is_valid(v_alpha) THEN
        RAISE EXCEPTION 'Alpha fork semantic tree is invalid' USING ERRCODE='22023';
    END IF;
    PERFORM public.ple_reusable_resolve_definition_v1(p_tenant,definition.value,true)
      FROM jsonb_array_elements(v_alpha->'modules') AS module(value)
      CROSS JOIN LATERAL jsonb_array_elements(module.value->'definitions') AS definition(value);
    INSERT INTO public.alpha_course(creator_tenant_id,creator_user_id,title,creator_public_byline,semantic_sha256)
    VALUES(p_tenant,v_actor,v_alpha->>'title',v_byline,digest(convert_to(v_alpha::text,'UTF8'),'sha256'))
    RETURNING * INTO v_row;
    INSERT INTO public.alpha_course_module(alpha_course_id,position,label)
    SELECT v_row.alpha_course_id,module.ordinality-1,module.value->>'label'
      FROM jsonb_array_elements(v_alpha->'modules') WITH ORDINALITY AS module(value,ordinality);
    INSERT INTO public.alpha_course_definition(alpha_course_id,module_position,position,definition_json)
    SELECT v_row.alpha_course_id,module.ordinality-1,definition.ordinality-1,definition.value-'entries'
      FROM jsonb_array_elements(v_alpha->'modules') WITH ORDINALITY AS module(value,ordinality)
      CROSS JOIN LATERAL jsonb_array_elements(module.value->'definitions') WITH ORDINALITY AS definition(value,ordinality);
    INSERT INTO public.alpha_course_entry(alpha_course_id,module_position,definition_position,position,kind)
    SELECT v_row.alpha_course_id,module.ordinality-1,definition.ordinality-1,entry.ordinality-1,entry.value->>'kind'
      FROM jsonb_array_elements(v_alpha->'modules') WITH ORDINALITY AS module(value,ordinality)
      CROSS JOIN LATERAL jsonb_array_elements(module.value->'definitions') WITH ORDINALITY AS definition(value,ordinality)
      CROSS JOIN LATERAL jsonb_array_elements(definition.value->'entries') WITH ORDINALITY AS entry(value,ordinality);
    INSERT INTO public.alpha_course_fixed(alpha_course_id,module_position,definition_position,position,problem_id,version_id,points_possible,scoring_mode)
    SELECT v_row.alpha_course_id,module.ordinality-1,definition.ordinality-1,entry.ordinality-1,document.problem_id,document.version_id,
           (entry.value->>'pointsPossible')::numeric,entry.value->>'scoringMode'
      FROM jsonb_array_elements(v_alpha->'modules') WITH ORDINALITY AS module(value,ordinality)
      CROSS JOIN LATERAL jsonb_array_elements(module.value->'definitions') WITH ORDINALITY AS definition(value,ordinality)
      CROSS JOIN LATERAL jsonb_array_elements(definition.value->'entries') WITH ORDINALITY AS entry(value,ordinality)
      JOIN public.catalog_search_document AS document ON document.question_id=replace(entry.value->>'questionId','-','')
     WHERE entry.value->>'kind'='fixed';
    INSERT INTO public.alpha_course_pool(alpha_course_id,module_position,definition_position,position,draw_count,points_per_item,ordering_policy,algorithm_version)
    SELECT v_row.alpha_course_id,module.ordinality-1,definition.ordinality-1,entry.ordinality-1,
           (entry.value->>'drawCount')::integer,(entry.value->>'pointsPerItem')::numeric,entry.value->>'ordering',1
      FROM jsonb_array_elements(v_alpha->'modules') WITH ORDINALITY AS module(value,ordinality)
      CROSS JOIN LATERAL jsonb_array_elements(module.value->'definitions') WITH ORDINALITY AS definition(value,ordinality)
      CROSS JOIN LATERAL jsonb_array_elements(definition.value->'entries') WITH ORDINALITY AS entry(value,ordinality)
     WHERE entry.value->>'kind'='pool';
    INSERT INTO public.alpha_course_pool_candidate(alpha_course_id,module_position,definition_position,pool_position,position,problem_id,version_id)
    SELECT v_row.alpha_course_id,module.ordinality-1,definition.ordinality-1,entry.ordinality-1,candidate.ordinality-1,
           document.problem_id,document.version_id
      FROM jsonb_array_elements(v_alpha->'modules') WITH ORDINALITY AS module(value,ordinality)
      CROSS JOIN LATERAL jsonb_array_elements(module.value->'definitions') WITH ORDINALITY AS definition(value,ordinality)
      CROSS JOIN LATERAL jsonb_array_elements(definition.value->'entries') WITH ORDINALITY AS entry(value,ordinality)
      CROSS JOIN LATERAL jsonb_array_elements_text(entry.value->'candidates') WITH ORDINALITY AS candidate(question_id,ordinality)
      JOIN public.catalog_search_document AS document ON document.question_id=replace(candidate.question_id,'-','')
     WHERE entry.value->>'kind'='pool';
    alpha_course_id:=v_row.alpha_course_id; alpha_course_reference:=v_row.alpha_course_reference; revision:=v_row.revision;
    RETURN NEXT;
END $$;

-- Forking records adoption receipt/lineage after the reusable broker has
-- atomically materialized the Alpha aggregate.
CREATE FUNCTION public.ple_caa_apply_fork_alpha_v1(
    p_tenant uuid, p_session character(64), p_preparation uuid, p_envelope jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_consumed jsonb; v_plan jsonb; v_actor uuid; v_key text; v_digest bytea; v_replay jsonb;
DECLARE v_source jsonb; v_source_alpha uuid; v_alpha uuid; v_semantic jsonb;
BEGIN
    v_consumed := public.ple_cam_consume_materialization_preparation_v1(p_tenant,p_session,p_preparation,p_envelope);
    IF v_consumed->>'operation' <> 'applyForkAlpha' THEN RAISE EXCEPTION 'curriculum adoption fork is invalid' USING ERRCODE='22023'; END IF;
    v_actor := public.ple_cam_uuid_v1(v_consumed->'actor'); v_key := v_consumed->>'idempotencyKey';
    v_digest := public.ple_cam_digest_bytes_v1(v_consumed->'requestSha256');
    v_replay := public.ple_cam_select_receipt_v1(p_tenant,v_key,'forkAlpha',v_actor,v_digest);
    IF v_replay IS NOT NULL THEN RETURN v_replay; END IF;
    v_plan := v_consumed->'plan'->'plan';
    PERFORM public.ple_cam_require_exact_object_v1(v_plan,ARRAY['semantic','source'],524288);
    PERFORM public.ple_cam_validate_semantic_v1(v_plan->'semantic');
    v_source := v_plan->'source'; PERFORM public.ple_cac_reusable_document_v1(p_tenant,p_session,v_source);
    v_source_alpha := (SELECT alpha_course_id FROM public.alpha_course WHERE alpha_course_reference=
        public.ple_curriculum_adoption_route_number_v1(v_source->'reference','AC'));
    v_semantic := v_plan#>'{semantic,semanticInput}';
    SELECT alpha_course_id INTO v_alpha
      FROM public.ple_materialize_alpha_fork_v1(p_tenant,p_session,v_actor,v_semantic);
    IF v_alpha IS NULL THEN RAISE EXCEPTION 'curriculum adoption fork is unavailable' USING ERRCODE='PBI01'; END IF;
    PERFORM public.ple_cam_insert_receipt_v1(p_tenant,v_key,'forkAlpha',v_actor,v_digest,NULL,NULL,v_alpha,NULL,v_source_alpha,NULL,NULL);
    INSERT INTO public.curriculum_alpha_fork_lineage(tenant_id,alpha_course_id,receipt_key,source_alpha_course_id,source_alpha_revision,
        semantic_payload,semantic_canonical_version,semantic_canonical_bytes,semantic_sha256)
    VALUES(p_tenant,v_alpha,v_key,v_source_alpha,public.ple_cam_positive_revision_v1(v_source->'revision'),v_semantic,
        1,public.ple_cam_bytes_v1(v_plan#>'{semantic,canonicalBytes}',1,524288),
        public.ple_cam_digest_bytes_v1(v_plan#>'{semantic,semanticDigest}'));
    RETURN public.ple_cam_receipt_result_v1(p_tenant,v_key,false);
END $$;

CREATE FUNCTION public.ple_caa_apply_reconciliation_v1(
    p_tenant uuid, p_session character(64), p_preparation uuid, p_envelope jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_consumed jsonb; v_repaired jsonb;
BEGIN
    v_consumed := public.ple_cam_consume_reconciliation_preparation_v1(p_tenant,p_session,p_preparation,p_envelope);
    v_repaired := public.ple_cam_reconcile_current_v1(p_tenant,v_consumed->>'receiptKey',v_consumed->'repairs');
    RETURN public.ple_cam_reconciliation_result_v1(p_tenant,v_consumed->>'receiptKey',v_repaired);
END $$;

ALTER FUNCTION public.ple_materialize_alpha_fork_v1(uuid,character,uuid,jsonb)
    OWNER TO ple_reusable_curriculum_broker;
REVOKE ALL ON FUNCTION public.ple_materialize_alpha_fork_v1(uuid,character,uuid,jsonb)
    FROM PUBLIC,ple_app;
GRANT EXECUTE ON FUNCTION public.ple_materialize_alpha_fork_v1(uuid,character,uuid,jsonb)
    TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_caa_definition_payload_v1(jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_caa_assignment_plan_source_v1(jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_caa_assignment_provenance_v1(jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_caa_apply_assignment_v1(uuid,character,uuid,jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_caa_apply_fork_alpha_v1(uuid,character,uuid,jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_caa_apply_reconciliation_v1(uuid,character,uuid,jsonb) OWNER TO ple_curriculum_adoption_broker;
REVOKE ALL ON FUNCTION public.ple_caa_definition_payload_v1(jsonb),public.ple_caa_assignment_plan_source_v1(jsonb),
    public.ple_caa_assignment_provenance_v1(jsonb),public.ple_caa_apply_assignment_v1(uuid,character,uuid,jsonb),
    public.ple_caa_apply_fork_alpha_v1(uuid,character,uuid,jsonb),public.ple_caa_apply_reconciliation_v1(uuid,character,uuid,jsonb)
    FROM PUBLIC,ple_app;
GRANT EXECUTE ON FUNCTION public.ple_caa_definition_payload_v1(jsonb),public.ple_caa_assignment_plan_source_v1(jsonb),
    public.ple_caa_assignment_provenance_v1(jsonb),public.ple_caa_apply_assignment_v1(uuid,character,uuid,jsonb),
    public.ple_caa_apply_fork_alpha_v1(uuid,character,uuid,jsonb),public.ple_caa_apply_reconciliation_v1(uuid,character,uuid,jsonb)
    TO ple_curriculum_adoption_broker;

DO $$
DECLARE v_function regprocedure;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'public.ple_caa_definition_payload_v1(jsonb)'::regprocedure,
        'public.ple_caa_assignment_plan_source_v1(jsonb)'::regprocedure,
        'public.ple_caa_assignment_provenance_v1(jsonb)'::regprocedure,
        'public.ple_caa_apply_assignment_v1(uuid,character,uuid,jsonb)'::regprocedure,
        'public.ple_caa_apply_fork_alpha_v1(uuid,character,uuid,jsonb)'::regprocedure,
        'public.ple_caa_apply_reconciliation_v1(uuid,character,uuid,jsonb)'::regprocedure
    ] LOOP
        IF (SELECT pg_get_userbyid(proowner) FROM pg_proc WHERE oid=v_function)
               <> 'ple_curriculum_adoption_broker'
           OR NOT (SELECT prosecdef FROM pg_proc WHERE oid=v_function)
           OR NOT coalesce((SELECT proconfig @> ARRAY['search_path=pg_catalog, public, pg_temp']
                            FROM pg_proc WHERE oid=v_function),false)
           OR has_function_privilege('public',v_function,'EXECUTE')
           OR has_function_privilege('ple_app',v_function,'EXECUTE')
           OR NOT has_function_privilege('ple_curriculum_adoption_broker',v_function,'EXECUTE') THEN
            RAISE EXCEPTION 'curriculum adoption assignment materializer catalog is unsafe';
        END IF;
    END LOOP;
END $$;

DO $$
DECLARE v_function regprocedure := 'public.ple_materialize_alpha_fork_v1(uuid,character,uuid,jsonb)'::regprocedure;
BEGIN
    IF (SELECT pg_get_userbyid(proowner) FROM pg_proc WHERE oid=v_function)
           <> 'ple_reusable_curriculum_broker'
       OR NOT (SELECT prosecdef FROM pg_proc WHERE oid=v_function)
       OR NOT coalesce((SELECT proconfig @> ARRAY['search_path=pg_catalog, public, pg_temp']
                        FROM pg_proc WHERE oid=v_function),false)
       OR has_function_privilege('public',v_function,'EXECUTE')
       OR has_function_privilege('ple_app',v_function,'EXECUTE')
       OR NOT has_function_privilege('ple_curriculum_adoption_broker',v_function,'EXECUTE') THEN
        RAISE EXCEPTION 'Alpha fork materializer catalog is unsafe';
    END IF;
END $$;
COMMIT;
