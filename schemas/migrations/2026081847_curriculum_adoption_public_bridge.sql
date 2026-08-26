-- WP-PROF-B2: execute-only public broker bridge for curriculum adoption.
--
-- qmodel owns semantic normalization and canonical request digests.  This
-- migration owns session/actor locks, relational facts, one-use preparation,
-- replay serialization, and the route to the private materializers.
BEGIN;

-- ASVS 2.3.1, 8.2.1, 8.2.2: lock the active presented session before Rust
-- binds the actor into its request digest.  The lock remains transaction-scoped
-- through snapshot compilation and materialization.
CREATE OR REPLACE FUNCTION public.ple_curriculum_adoption_materialization_actor_v1(
    p_tenant uuid, p_session character(64)
) RETURNS uuid LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_session public.auth_session%ROWTYPE; v_actor uuid;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'curriculum adoption request is invalid' USING ERRCODE = '22023';
    END IF;
    SELECT session_row.* INTO v_session
      FROM public.auth_session AS session_row
     WHERE session_row.session_hash = p_session
       AND session_row.tenant_id = p_tenant
       AND session_row.user_id IS NOT NULL
       AND session_row.revoked_at IS NULL
       AND session_row.expires_at > transaction_timestamp()
     FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'curriculum adoption actor is unavailable' USING ERRCODE = '42501';
    END IF;
    v_actor := public.ple_curriculum_adoption_actor_v1(p_tenant, p_session);
    IF v_actor IS DISTINCT FROM v_session.user_id THEN
        RAISE EXCEPTION 'curriculum adoption actor is unavailable' USING ERRCODE = '42501';
    END IF;
    RETURN v_actor;
END $$;

CREATE OR REPLACE FUNCTION public.ple_snapshot_curriculum_adoption_v1(
    p_tenant uuid, p_session character(64), p_operation jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_actor uuid; v_kind text; v_request jsonb; v_binding jsonb;
DECLARE v_digest bytea; v_receipt jsonb; v_facts jsonb; v_preparation uuid;
DECLARE v_receipt_operation text;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'curriculum adoption request is invalid' USING ERRCODE = '22023';
    END IF;
    v_kind := public.ple_curriculum_adoption_bridge_operation_v1(p_operation);
    v_request := p_operation->'request';
    -- Preview and inspection use ordinary tenant transactions because fact
    -- compilation holds witness locks through projection. Materializing
    -- operations additionally lock the session and recheck bindings before
    -- preparation or mutation. ASVS 2.3.3, 2.3.4: preserve workflow ordering
    -- and race-resistant authorization. ASVS 15.4.2, 15.4.3: keep atomic
    -- state checks and dependent projection under consistent broker-owned
    -- transaction locking. Existing broker checks retain tenant isolation and
    -- operation-specific authorization.
    IF v_kind IN (
        'applyForkAlpha', 'applyBlueprintInstantiation', 'applyAlphaInstantiation',
        'applyCourseRollover', 'applyCourseTermShift', 'applyAssignmentFastForward',
        'createSourceDerivedAssignment', 'reconcile'
    ) THEN
        v_actor := public.ple_curriculum_adoption_materialization_actor_v1(p_tenant, p_session);
    ELSE
        v_actor := public.ple_curriculum_adoption_actor_v1(p_tenant, p_session);
    END IF;

    IF v_kind IN (
        'applyForkAlpha', 'applyBlueprintInstantiation', 'applyAlphaInstantiation',
        'applyCourseRollover', 'applyCourseTermShift', 'applyAssignmentFastForward',
        'createSourceDerivedAssignment'
    ) THEN
        v_binding := p_operation->'materializationBinding';
        IF public.ple_cam_uuid_v1(v_binding->'actor') IS DISTINCT FROM v_actor THEN
            RAISE EXCEPTION 'curriculum adoption materialization binding is unavailable'
                USING ERRCODE = 'PBC01';
        END IF;
        v_digest := public.ple_cam_digest_bytes_v1(v_binding->'requestSha256');
        v_receipt_operation := CASE v_kind
            WHEN 'applyForkAlpha' THEN 'forkAlpha'
            WHEN 'applyBlueprintInstantiation' THEN 'blueprintInstantiation'
            WHEN 'applyAlphaInstantiation' THEN 'alphaInstantiation'
            WHEN 'applyCourseRollover' THEN 'courseRollover'
            WHEN 'applyCourseTermShift' THEN 'courseTermShift'
            WHEN 'applyAssignmentFastForward' THEN 'assignmentFastForward'
            WHEN 'createSourceDerivedAssignment' THEN 'sourceDerivedAssignment'
        END;
        v_receipt := public.ple_cam_select_receipt_v1(
            p_tenant, v_request->>'idempotencyKey', v_receipt_operation, v_actor, v_digest
        );
        IF v_receipt IS NOT NULL THEN
            RETURN jsonb_build_object(
                'kind', 'replay', 'version', 1, 'operation', p_operation->'operation',
                'actor', v_actor, 'requestSha256', v_binding->'requestSha256', 'result', v_receipt
            );
        END IF;
    END IF;

    v_facts := public.ple_compile_curriculum_adoption_facts_v1(
        p_tenant, p_session, v_actor, v_kind, v_request
    );
    IF v_kind = 'inspectImports' AND v_facts IS NULL THEN
        RETURN jsonb_build_object('kind', 'absent', 'version', 1, 'operation', p_operation->'operation');
    END IF;
    IF v_kind = 'inspectImports' THEN
        IF v_facts->>'kind' IS DISTINCT FROM 'inspection' THEN
            RAISE EXCEPTION 'curriculum adoption inspection facts are invalid' USING ERRCODE = 'PBI01';
        END IF;
        v_facts := jsonb_build_object('kind', 'inspection', 'inspection', v_facts);
    END IF;

    IF v_kind LIKE 'preview%' OR v_kind = 'inspectImports' THEN
        RETURN jsonb_build_object(
            'kind', 'preview', 'version', 1, 'operation', p_operation->'operation', 'facts', v_facts
        );
    END IF;
    IF v_kind = 'reconcile' THEN
        PERFORM public.ple_curriculum_adoption_prepare_temp_v1();
        v_preparation := gen_random_uuid();
        INSERT INTO pg_temp.ple_curriculum_adoption_reconciliation_preparation(
            preparation_id, tenant_id, actor_user_id, request, facts
        ) VALUES (v_preparation, p_tenant, v_actor, v_request, v_facts);
        RETURN jsonb_build_object(
            'kind', 'reconciliationPrepare', 'version', 1, 'operation', p_operation->'operation',
            'preparationId', v_preparation, 'actor', v_actor, 'facts', v_facts
        );
    END IF;
    IF v_kind NOT IN (
        'applyForkAlpha', 'applyBlueprintInstantiation', 'applyAlphaInstantiation',
        'applyCourseRollover', 'applyCourseTermShift', 'applyAssignmentFastForward',
        'createSourceDerivedAssignment'
    ) THEN
        RAISE EXCEPTION 'curriculum adoption bridge operation is invalid' USING ERRCODE = '22023';
    END IF;
    PERFORM public.ple_curriculum_adoption_prepare_temp_v1();
    v_preparation := gen_random_uuid();
    INSERT INTO pg_temp.ple_curriculum_adoption_materialization_preparation(
        preparation_id, tenant_id, actor_user_id, operation, request, facts, request_sha256
    ) VALUES (v_preparation, p_tenant, v_actor, v_kind, v_request, v_facts, v_digest);
    RETURN jsonb_build_object(
        'kind', 'prepare', 'version', 1, 'operation', p_operation->'operation',
        'preparationId', v_preparation, 'actor', v_actor,
        'requestSha256', v_binding->'requestSha256', 'facts', v_facts
    );
END $$;

-- Classify only a fully validated envelope.  Materializers retain the actual
-- consume-and-mutate ownership, so every rejected binding fails before writes.
CREATE FUNCTION public.ple_cam_materialization_dispatch_kind_v1(
    p_preparation uuid, p_envelope jsonb
) RETURNS text LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_envelope jsonb;
BEGIN
    IF p_envelope#>>'{operation,kind}' = 'reconcile' THEN
        v_envelope := public.ple_cam_validate_reconciliation_envelope_v1(p_preparation, p_envelope);
    ELSE
        v_envelope := public.ple_cam_validate_materialization_envelope_v1(p_preparation, p_envelope);
    END IF;
    RETURN v_envelope->>'operation';
END $$;

CREATE OR REPLACE FUNCTION public.ple_materialize_curriculum_adoption_v1(
    p_tenant uuid, p_session character(64), p_preparation uuid, p_plan jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_kind text;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL OR p_preparation IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'curriculum adoption materialization is unavailable' USING ERRCODE = '42501';
    END IF;
    v_kind := public.ple_cam_materialization_dispatch_kind_v1(p_preparation, p_plan);
    CASE v_kind
        WHEN 'applyForkAlpha' THEN
            RETURN public.ple_caa_apply_fork_alpha_v1(p_tenant, p_session, p_preparation, p_plan);
        WHEN 'applyBlueprintInstantiation', 'applyAssignmentFastForward', 'createSourceDerivedAssignment' THEN
            RETURN public.ple_caa_apply_assignment_v1(p_tenant, p_session, p_preparation, p_plan);
        WHEN 'applyAlphaInstantiation', 'applyCourseRollover' THEN
            RETURN public.ple_cmc_materialize_course_v1(p_tenant, p_session, p_preparation, p_plan, v_kind);
        WHEN 'applyCourseTermShift' THEN
            RETURN public.ple_cmc_materialize_term_shift_v1(p_tenant, p_session, p_preparation, p_plan);
        WHEN 'reconcile' THEN
            RETURN public.ple_caa_apply_reconciliation_v1(p_tenant, p_session, p_preparation, p_plan);
        ELSE
            RAISE EXCEPTION 'curriculum adoption materialization operation is invalid'
                USING ERRCODE = '22023';
    END CASE;
END $$;

ALTER FUNCTION public.ple_curriculum_adoption_materialization_actor_v1(uuid, character)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_snapshot_curriculum_adoption_v1(uuid, character, jsonb)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_materialization_dispatch_kind_v1(uuid, jsonb)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_materialize_curriculum_adoption_v1(uuid, character, uuid, jsonb)
    OWNER TO ple_curriculum_adoption_broker;

REVOKE ALL ON FUNCTION public.ple_curriculum_adoption_materialization_actor_v1(uuid, character),
    public.ple_snapshot_curriculum_adoption_v1(uuid, character, jsonb),
    public.ple_materialize_curriculum_adoption_v1(uuid, character, uuid, jsonb)
    FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_cam_materialization_dispatch_kind_v1(uuid, jsonb)
    FROM PUBLIC, ple_app, ple_auth, ple_student, ple_grader, ple_grading_reader;
GRANT EXECUTE ON FUNCTION public.ple_curriculum_adoption_materialization_actor_v1(uuid, character),
    public.ple_snapshot_curriculum_adoption_v1(uuid, character, jsonb),
    public.ple_materialize_curriculum_adoption_v1(uuid, character, uuid, jsonb)
    TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_cam_materialization_dispatch_kind_v1(uuid, jsonb)
    TO ple_curriculum_adoption_broker;

-- Keep the historical facades inert and remove application reachability.
REVOKE ALL ON FUNCTION public.ple_preview_fork_alpha_v1(uuid, character, jsonb),
    public.ple_apply_fork_alpha_v1(uuid, character, jsonb),
    public.ple_preview_blueprint_instantiation_v1(uuid, character, jsonb),
    public.ple_apply_blueprint_instantiation_v1(uuid, character, jsonb),
    public.ple_preview_alpha_instantiation_v1(uuid, character, jsonb),
    public.ple_apply_alpha_instantiation_v1(uuid, character, jsonb),
    public.ple_preview_course_rollover_v1(uuid, character, jsonb),
    public.ple_apply_course_rollover_v1(uuid, character, jsonb),
    public.ple_preview_course_term_shift_v1(uuid, character, jsonb),
    public.ple_apply_course_term_shift_v1(uuid, character, jsonb),
    public.ple_preview_assignment_fast_forward_v1(uuid, character, jsonb),
    public.ple_apply_assignment_fast_forward_v1(uuid, character, jsonb),
    public.ple_preview_source_derived_assignment_v1(uuid, character, jsonb),
    public.ple_create_source_derived_assignment_v1(uuid, character, jsonb),
    public.ple_inspect_curriculum_imports_v1(uuid, character, jsonb),
    public.ple_reconcile_curriculum_adoption_v1(uuid, character, jsonb)
    FROM ple_app;

DO $$
DECLARE v_function regprocedure; v_role text;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'public.ple_curriculum_adoption_materialization_actor_v1(uuid,character)'::regprocedure,
        'public.ple_snapshot_curriculum_adoption_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_materialize_curriculum_adoption_v1(uuid,character,uuid,jsonb)'::regprocedure
    ] LOOP
        IF (SELECT pg_get_userbyid(proowner) FROM pg_proc WHERE oid = v_function)
               <> 'ple_curriculum_adoption_broker'
           OR NOT (SELECT prosecdef FROM pg_proc WHERE oid = v_function)
           OR NOT coalesce((SELECT proconfig @> ARRAY['search_path=pg_catalog, public, pg_temp']
                             FROM pg_proc WHERE oid = v_function), false)
           OR NOT has_function_privilege('ple_app', v_function, 'EXECUTE')
           OR has_function_privilege('public', v_function, 'EXECUTE') THEN
            RAISE EXCEPTION 'curriculum adoption public bridge catalog is unsafe';
        END IF;
        FOREACH v_role IN ARRAY ARRAY['ple_auth', 'ple_student', 'ple_grader', 'ple_grading_reader'] LOOP
            IF has_function_privilege(v_role, v_function, 'EXECUTE') THEN
                RAISE EXCEPTION 'curriculum adoption public bridge leaked to %', v_role;
            END IF;
        END LOOP;
    END LOOP;
    FOREACH v_function IN ARRAY ARRAY[
        'public.ple_curriculum_adoption_actor_v1(uuid,character)'::regprocedure,
        'public.ple_curriculum_adoption_bridge_operation_v1(jsonb)'::regprocedure,
        'public.ple_curriculum_adoption_prepare_temp_v1()'::regprocedure,
        'public.ple_compile_curriculum_adoption_facts_v1(uuid,character,uuid,text,jsonb)'::regprocedure,
        'public.ple_cam_materialization_dispatch_kind_v1(uuid,jsonb)'::regprocedure,
        'public.ple_cam_validate_materialization_envelope_v1(uuid,jsonb)'::regprocedure,
        'public.ple_cam_validate_reconciliation_envelope_v1(uuid,jsonb)'::regprocedure,
        'public.ple_cam_consume_materialization_preparation_v1(uuid,character,uuid,jsonb)'::regprocedure,
        'public.ple_cam_consume_reconciliation_preparation_v1(uuid,character,uuid,jsonb)'::regprocedure,
        'public.ple_caa_apply_assignment_v1(uuid,character,uuid,jsonb)'::regprocedure,
        'public.ple_caa_apply_fork_alpha_v1(uuid,character,uuid,jsonb)'::regprocedure,
        'public.ple_caa_apply_reconciliation_v1(uuid,character,uuid,jsonb)'::regprocedure,
        'public.ple_cmc_create_course_v1(uuid,uuid,character,jsonb,jsonb)'::regprocedure,
        'public.ple_cmc_insert_course_evidence_v1(uuid,text,text,uuid,text,jsonb,uuid,bigint,uuid,bigint)'::regprocedure,
        'public.ple_cmc_materialize_course_v1(uuid,character,uuid,jsonb,text)'::regprocedure,
        'public.ple_cmc_materialize_term_shift_v1(uuid,character,uuid,jsonb)'::regprocedure,
        'public.ple_cmc_apply_term_schedule_v1(uuid,uuid,uuid,jsonb,jsonb)'::regprocedure,
        'public.ple_materialize_alpha_fork_v1(uuid,character,uuid,jsonb)'::regprocedure
    ] LOOP
        IF has_function_privilege('ple_app', v_function, 'EXECUTE')
           OR NOT has_function_privilege('ple_curriculum_adoption_broker', v_function, 'EXECUTE') THEN
            RAISE EXCEPTION 'curriculum adoption private capability catalog is unsafe';
        END IF;
    END LOOP;
    FOREACH v_function IN ARRAY ARRAY[
        'public.ple_preview_fork_alpha_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_apply_fork_alpha_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_preview_blueprint_instantiation_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_apply_blueprint_instantiation_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_preview_alpha_instantiation_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_apply_alpha_instantiation_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_preview_course_rollover_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_apply_course_rollover_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_preview_course_term_shift_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_apply_course_term_shift_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_preview_assignment_fast_forward_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_apply_assignment_fast_forward_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_preview_source_derived_assignment_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_create_source_derived_assignment_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_inspect_curriculum_imports_v1(uuid,character,jsonb)'::regprocedure,
        'public.ple_reconcile_curriculum_adoption_v1(uuid,character,jsonb)'::regprocedure
    ] LOOP
        IF has_function_privilege('ple_app', v_function, 'EXECUTE') THEN
            RAISE EXCEPTION 'curriculum adoption retired facade remains executable';
        END IF;
    END LOOP;
    IF has_table_privilege('ple_app', 'public.curriculum_adoption_receipt', 'SELECT,INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_app', 'public.curriculum_assignment_adoption_evidence', 'SELECT,INSERT,UPDATE,DELETE')
       OR NOT has_column_privilege('ple_curriculum_adoption_broker', 'public.auth_session', 'session_hash', 'UPDATE')
       OR has_column_privilege('ple_curriculum_adoption_broker', 'public.auth_session', 'user_id', 'UPDATE')
       OR NOT has_column_privilege('ple_curriculum_adoption_broker', 'public.curriculum_adoption_receipt', 'idempotency_key', 'UPDATE')
       OR has_column_privilege('ple_curriculum_adoption_broker', 'public.curriculum_adoption_receipt', 'actor_user_id', 'UPDATE')
       OR NOT has_column_privilege('ple_curriculum_adoption_broker', 'public.curriculum_adoption_receipt_assignment', 'receipt_key', 'UPDATE')
       OR has_column_privilege('ple_curriculum_adoption_broker', 'public.curriculum_adoption_receipt_assignment', 'assignment_id', 'UPDATE')
       OR NOT has_column_privilege('ple_curriculum_adoption_broker', 'public.curriculum_assignment_adoption_evidence', 'receipt_key', 'UPDATE')
       OR has_column_privilege('ple_curriculum_adoption_broker', 'public.curriculum_assignment_adoption_evidence', 'semantic_payload', 'UPDATE')
       OR NOT has_column_privilege('ple_curriculum_adoption_broker', 'public.curriculum_whole_course_adoption', 'receipt_key', 'UPDATE')
       OR has_column_privilege('ple_curriculum_adoption_broker', 'public.curriculum_whole_course_adoption', 'semantic_payload', 'UPDATE')
       OR NOT has_column_privilege('ple_curriculum_adoption_broker', 'public.course_member', 'course_membership_id', 'UPDATE')
       OR has_column_privilege('ple_curriculum_adoption_broker', 'public.course_member', 'role', 'UPDATE')
       OR NOT has_column_privilege('ple_curriculum_schedule_revision_broker', 'public.course_member', 'course_membership_id', 'UPDATE')
       OR has_column_privilege('ple_curriculum_schedule_revision_broker', 'public.course_member', 'role', 'UPDATE')
       OR NOT has_column_privilege('ple_curriculum_adoption_broker', 'public.assignment', 'assignment_id', 'UPDATE')
       OR has_column_privilege('ple_curriculum_adoption_broker', 'public.assignment', 'revision', 'UPDATE')
       OR NOT has_column_privilege('ple_curriculum_adoption_broker', 'public.course', 'course_id', 'UPDATE')
       OR has_column_privilege('ple_curriculum_adoption_broker', 'public.course', 'term_start_date', 'UPDATE') THEN
        RAISE EXCEPTION 'curriculum adoption bridge relation authority is unsafe';
    END IF;
END $$;

COMMIT;
