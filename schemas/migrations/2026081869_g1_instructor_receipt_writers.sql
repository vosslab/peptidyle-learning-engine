-- WP-PROF-G1 / G1-W7: final Instructor receipt writers.
BEGIN;
-- Only this trusted broker wrapper supplies the session-derived actor; direct
-- calls must never treat p_actor_id as caller-asserted Instructor identity.
CREATE FUNCTION public.ple_prepare_accepted_submission_retry_v2(
    p_tenant_id uuid, p_attempt_id uuid, p_submission_id uuid, p_job_id uuid, p_actor_id uuid
) RETURNS TABLE(resulting_execution_generation bigint, resulting_state text)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v_execution public.grading_execution%ROWTYPE;
    v_max_retry_count CONSTANT integer := 20;
    v_job_max_attempts CONSTANT integer := 3;
    v_row_count bigint;
BEGIN
    IF p_tenant_id IS NULL OR p_attempt_id IS NULL OR p_submission_id IS NULL
       OR p_job_id IS NULL OR p_actor_id IS NULL
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'accepted-submission retry arguments are invalid'
            USING ERRCODE = '22023';
    END IF;
    SELECT execution.* INTO v_execution
      FROM public.grading_execution AS execution
      JOIN public.submission_evaluation AS evaluation
        ON evaluation.tenant_id = execution.tenant_id
       AND evaluation.attempt_id = execution.attempt_id
       AND evaluation.submission_id = execution.submission_id
     WHERE execution.tenant_id = p_tenant_id
       AND execution.attempt_id = p_attempt_id
       AND execution.submission_id = p_submission_id
       AND execution.state = 'exception'
       AND execution.retry_count < v_max_retry_count
       AND evaluation.grading_status = 'automated_exception'
     FOR UPDATE OF execution, evaluation;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'accepted-submission retry target conflicts'
            USING ERRCODE = '55000';
    END IF;
    IF EXISTS (
        SELECT 1 FROM public.worker_job AS job
         WHERE job.tenant_id = p_tenant_id AND job.job_id = p_job_id
    ) THEN
        RAISE EXCEPTION 'accepted-submission retry action conflicts'
            USING ERRCODE = '55000';
    END IF;
    INSERT INTO public.worker_job (job_id, tenant_id, payload, state, max_attempts)
    VALUES (p_job_id, p_tenant_id, jsonb_build_object(
        'kind', 'gradeAcceptedSubmission', 'attempt', p_attempt_id::text,
        'submission', p_submission_id::text,
        'execution_generation', v_execution.execution_generation + 1
    ), 'ready', v_job_max_attempts)
    ON CONFLICT (tenant_id, job_id) DO NOTHING;
    GET DIAGNOSTICS v_row_count = ROW_COUNT;
    IF v_row_count <> 1 THEN
        RAISE EXCEPTION 'accepted-submission retry action conflicts'
            USING ERRCODE = '55000';
    END IF;
    UPDATE public.grading_execution AS execution
       SET execution_generation = v_execution.execution_generation + 1,
           state = 'ready', current_job_id = p_job_id, active_worker_id = NULL,
           retry_count = v_execution.retry_count + 1,
           updated_at = transaction_timestamp()
     WHERE execution.tenant_id = p_tenant_id
       AND execution.attempt_id = p_attempt_id
       AND execution.submission_id = p_submission_id;
    GET DIAGNOSTICS v_row_count = ROW_COUNT;
    IF v_row_count <> 1 THEN
        RAISE EXCEPTION 'accepted-submission retry execution changed while locked'
            USING ERRCODE = '40001';
    END IF;
    UPDATE public.submission_evaluation AS evaluation
       SET grading_status = 'automated_pending',
           evaluated_at = transaction_timestamp(),
           evaluation_revision = evaluation.evaluation_revision + 1
     WHERE evaluation.tenant_id = p_tenant_id
       AND evaluation.attempt_id = p_attempt_id
       AND evaluation.submission_id = p_submission_id;
    GET DIAGNOSTICS v_row_count = ROW_COUNT;
    IF v_row_count <> 1 THEN
        RAISE EXCEPTION 'accepted-submission retry evaluation changed while locked'
            USING ERRCODE = '40001';
    END IF;
    INSERT INTO public.grading_execution_receipt (
        tenant_id, receipt_id, attempt_id, submission_id,
        submission_occurred_at, course_id, execution_generation,
        resulting_state, safe_category, actor_id
    ) VALUES (
        p_tenant_id, pg_catalog.gen_random_uuid(), p_attempt_id, p_submission_id,
        v_execution.submission_occurred_at, v_execution.course_id,
        v_execution.execution_generation + 1, 'ready', 'instructor_retry', p_actor_id
    );
    RETURN QUERY SELECT v_execution.execution_generation + 1, 'ready'::text;
END;
$$;
CREATE OR REPLACE FUNCTION public.ple_retry_instructor_grading_operation_v1(
    p_tenant_id uuid, p_session character(64), p_course_id uuid,
    p_assignment_id uuid, p_operation_reference integer, p_expected_revision bigint,
    p_action_id uuid
) RETURNS TABLE(
    disposition text, operation_reference integer, resulting_operation_revision bigint,
    resulting_execution_generation bigint, resulting_state text,
    action_occurred_at_millis bigint
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_actor_id uuid; v_operation public.grading_operation%ROWTYPE;
        v_receipt public.grading_operation_receipt%ROWTYPE;
        v_retry record; v_request_sha256 character(64);
BEGIN
    IF p_operation_reference IS NULL OR p_operation_reference < 1
       OR p_expected_revision IS NULL OR p_expected_revision < 1 OR p_action_id IS NULL THEN
        RAISE EXCEPTION 'Instructor grading-operation retry arguments are invalid'
            USING ERRCODE = '22023';
    END IF;
    v_actor_id := public.ple_instructor_grading_operation_actor_v1(
        p_tenant_id, p_session, p_course_id, p_assignment_id
    );
    IF v_actor_id IS NULL THEN RETURN; END IF;
    SELECT * INTO v_operation FROM public.grading_operation AS operation_row
     WHERE operation_row.tenant_id = p_tenant_id AND operation_row.course_id = p_course_id
       AND operation_row.assignment_id = p_assignment_id
       AND operation_row.grading_operation_id = p_operation_reference
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    SELECT * INTO v_receipt FROM public.grading_operation_receipt AS receipt
     WHERE receipt.tenant_id = p_tenant_id AND receipt.action_id = p_action_id;
    IF FOUND THEN
        IF v_receipt.actor_id IS DISTINCT FROM v_actor_id OR v_receipt.action_kind <> 'retry'
           OR v_receipt.retry_expected_operation_revision <> p_expected_revision
           OR v_receipt.grading_operation_id <> p_operation_reference THEN
            RAISE EXCEPTION 'Instructor grading-operation action conflicts' USING ERRCODE = '55000';
        END IF;
        RETURN QUERY SELECT 'replayed', v_receipt.grading_operation_id::integer,
            v_receipt.retry_resulting_operation_revision, v_receipt.resulting_execution_generation,
            v_receipt.resulting_state,
            floor(extract(epoch FROM v_receipt.occurred_at) * 1000)::bigint;
        RETURN;
    END IF;
    IF v_operation.revision <> p_expected_revision OR v_operation.target_kind <> 'submission'
       OR v_operation.state <> 'actionable' OR v_operation.next_action <> 'retry' THEN
        RAISE EXCEPTION 'Instructor grading-operation revision conflicts' USING ERRCODE = '55000';
    END IF;
    SELECT * INTO v_retry
      FROM public.ple_prepare_accepted_submission_retry_v2(
          p_tenant_id, v_operation.attempt_id, v_operation.submission_id, p_action_id, v_actor_id
      );
    UPDATE public.grading_operation SET revision = revision + 1,
        state = 'action_in_progress', next_action = NULL, updated_at = transaction_timestamp()
     WHERE tenant_id = p_tenant_id AND grading_operation_id = v_operation.grading_operation_id;
    v_request_sha256 := encode(pg_catalog.sha256(convert_to(jsonb_build_object(
        'action', 'retry', 'assignment', p_assignment_id::text,
        'operation', p_operation_reference, 'revision', p_expected_revision
    )::text, 'UTF8')), 'hex');
    INSERT INTO public.grading_operation_receipt (
        tenant_id, action_id, grading_operation_id, course_id, actor_id, action_kind, safe_category,
        request_sha256, retry_expected_operation_revision,
        retry_resulting_operation_revision,
        resulting_execution_generation, resulting_state
    ) VALUES (p_tenant_id, p_action_id, v_operation.grading_operation_id, p_course_id,
        v_actor_id, 'retry', 'instructor_retry', v_request_sha256, p_expected_revision, p_expected_revision + 1,
        v_retry.resulting_execution_generation, v_retry.resulting_state);
    RETURN QUERY SELECT 'accepted', p_operation_reference, p_expected_revision + 1,
        v_retry.resulting_execution_generation, v_retry.resulting_state,
        floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint;
END;
$$;
CREATE OR REPLACE FUNCTION public.ple_recalculate_instructor_assignment_v1(
    p_tenant_id uuid, p_session character(64), p_course_id uuid,
    p_assignment_id uuid, p_expected_assignment_revision bigint, p_action_id uuid
) RETURNS TABLE(
    disposition text, operation_reference integer, assignment_revision bigint,
    created_operation_revision bigint, scoring_generation bigint, scoring_status text,
    action_occurred_at_millis bigint
)
LANGUAGE plpgsql
VOLATILE
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    v_actor_id uuid;
    v_assignment public.assignment%ROWTYPE;
    v_receipt public.grading_operation_receipt%ROWTYPE;
    v_result record;
    v_request_sha256 character(64);
BEGIN
    IF p_expected_assignment_revision IS NULL OR p_expected_assignment_revision < 1
       OR p_action_id IS NULL THEN
        RAISE EXCEPTION 'Instructor recalculation arguments are invalid' USING ERRCODE = '22023';
    END IF;
    v_actor_id := public.ple_instructor_grading_operation_actor_v1(
        p_tenant_id, p_session, p_course_id, p_assignment_id
    );
    IF v_actor_id IS NULL THEN
        RETURN;
    END IF;
    SELECT * INTO v_assignment
      FROM public.assignment AS assignment_row
     WHERE assignment_row.tenant_id = p_tenant_id
       AND assignment_row.course_id = p_course_id
       AND assignment_row.assignment_id = p_assignment_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    -- Receipts are append-only evidence. A plain read preserves the broker's
    -- read/insert-only authority; the assignment lock above serializes this
    -- assignment's command transition.
    SELECT * INTO v_receipt
      FROM public.grading_operation_receipt AS receipt
     WHERE receipt.tenant_id = p_tenant_id
       AND receipt.action_id = p_action_id;
    IF FOUND THEN
        IF v_receipt.actor_id IS DISTINCT FROM v_actor_id
           OR v_receipt.action_kind <> 'recalculate'
           OR v_receipt.recalculate_expected_assignment_revision
                <> p_expected_assignment_revision THEN
            RAISE EXCEPTION 'Instructor grading-operation action conflicts' USING ERRCODE = '55000';
        END IF;
        RETURN QUERY SELECT 'replayed', v_receipt.grading_operation_id::integer,
            p_expected_assignment_revision, v_receipt.recalculate_created_operation_revision,
            v_receipt.resulting_scoring_generation, v_receipt.resulting_state,
            floor(extract(epoch FROM v_receipt.occurred_at) * 1000)::bigint;
        RETURN;
    END IF;
    IF v_assignment.revision <> p_expected_assignment_revision
       OR v_assignment.scoring_status NOT IN ('current', 'failed') THEN
        RAISE EXCEPTION 'Instructor assignment revision conflicts' USING ERRCODE = '55000';
    END IF;
    SELECT * INTO v_result
      FROM public.ple_request_scoring_invalidation_v1(
          p_tenant_id, p_course_id, p_assignment_id,
          'instructor_recalculation', p_action_id, p_action_id, v_actor_id, 10
      );
    v_request_sha256 := encode(pg_catalog.sha256(convert_to(jsonb_build_object(
        'action', 'recalculate', 'assignment', p_assignment_id::text,
        'revision', p_expected_assignment_revision
    )::text, 'UTF8')), 'hex');
    INSERT INTO public.grading_operation_receipt (
        tenant_id, action_id, grading_operation_id, course_id, actor_id, action_kind, safe_category,
        request_sha256, recalculate_expected_assignment_revision,
        recalculate_created_operation_revision,
        resulting_scoring_generation, resulting_state
    ) VALUES (
        p_tenant_id, p_action_id, v_result.operation_reference, p_course_id, v_actor_id,
        'recalculate', 'instructor_recalculation', v_request_sha256, p_expected_assignment_revision, 1,
        v_result.scoring_generation, 'recalculating'
    );
    RETURN QUERY SELECT 'accepted', v_result.operation_reference,
        p_expected_assignment_revision, 1::bigint, v_result.scoring_generation,
        'recalculating'::text,
        floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint;
END;
$$;
ALTER FUNCTION public.ple_prepare_accepted_submission_retry_v2(uuid, uuid, uuid, uuid, uuid) OWNER TO ple_accepted_submission_execution_worker;
ALTER FUNCTION public.ple_retry_instructor_grading_operation_v1(uuid, character, uuid, uuid, integer, bigint, uuid) OWNER TO ple_instructor_grading_operation_broker;
ALTER FUNCTION public.ple_recalculate_instructor_assignment_v1(uuid, character, uuid, uuid, bigint, uuid) OWNER TO ple_instructor_grading_operation_broker;
REVOKE ALL ON FUNCTION public.ple_prepare_accepted_submission_retry_v1(uuid, uuid, uuid, uuid) FROM PUBLIC, ple_app, ple_instructor_grading_operation_broker;
DROP FUNCTION public.ple_prepare_accepted_submission_retry_v1(uuid, uuid, uuid, uuid) RESTRICT;
DO $$
DECLARE
    v_function regprocedure;
    v_signature text;
    v_grantee record;
    v_owner regrole;
    v_expected oid[];
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'public.ple_prepare_accepted_submission_retry_v2(uuid,uuid,uuid,uuid,uuid)'::regprocedure,
        'public.ple_retry_instructor_grading_operation_v1(uuid,character,uuid,uuid,integer,bigint,uuid)'::regprocedure,
        'public.ple_recalculate_instructor_assignment_v1(uuid,character,uuid,uuid,bigint,uuid)'::regprocedure
    ] LOOP
        SELECT format('%I.%I(%s)', namespace_row.nspname, procedure_row.proname,
                      pg_get_function_identity_arguments(procedure_row.oid)) INTO v_signature
          FROM pg_catalog.pg_proc AS procedure_row JOIN pg_catalog.pg_namespace AS namespace_row
            ON namespace_row.oid = procedure_row.pronamespace WHERE procedure_row.oid = v_function;
        IF v_function = 'public.ple_prepare_accepted_submission_retry_v2(uuid,uuid,uuid,uuid,uuid)'::regprocedure THEN
            v_owner := 'ple_accepted_submission_execution_worker';
            v_expected := ARRAY['ple_instructor_grading_operation_broker'::regrole::oid];
        ELSE
            v_owner := 'ple_instructor_grading_operation_broker'; v_expected := ARRAY['ple_app'::regrole::oid];
        END IF;
        FOR v_grantee IN SELECT DISTINCT acl.grantee, role_row.rolname FROM pg_catalog.pg_proc AS procedure_row
            CROSS JOIN LATERAL pg_catalog.aclexplode(COALESCE(procedure_row.proacl, pg_catalog.acldefault('f', procedure_row.proowner))) AS acl
            LEFT JOIN pg_catalog.pg_roles AS role_row ON role_row.oid = acl.grantee
            WHERE procedure_row.oid = v_function AND acl.grantee <> procedure_row.proowner LOOP
            IF v_grantee.grantee = 0 THEN EXECUTE format('REVOKE ALL PRIVILEGES ON FUNCTION %s FROM PUBLIC', v_signature);
            ELSE EXECUTE format('REVOKE ALL PRIVILEGES ON FUNCTION %s FROM %I', v_signature, v_grantee.rolname); END IF;
        END LOOP;
        FOR v_grantee IN SELECT role_row.rolname FROM unnest(v_expected) AS expected(grantee)
            JOIN pg_catalog.pg_roles AS role_row ON role_row.oid = expected.grantee LOOP
            EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO %I', v_signature, v_grantee.rolname);
        END LOOP;
        IF EXISTS (SELECT 1 FROM pg_catalog.pg_proc AS procedure_row WHERE procedure_row.oid = v_function AND (
                procedure_row.proowner <> v_owner OR NOT procedure_row.prosecdef
                OR procedure_row.proconfig IS DISTINCT FROM ARRAY['search_path=pg_catalog, public, pg_temp']))
           OR EXISTS (WITH expected AS (SELECT unnest(v_expected) AS grantee, 'EXECUTE'::text AS privilege_type, false AS is_grantable), actual AS (
                SELECT acl.grantee, acl.privilege_type, acl.is_grantable FROM pg_catalog.pg_proc AS procedure_row
                CROSS JOIN LATERAL pg_catalog.aclexplode(COALESCE(procedure_row.proacl, pg_catalog.acldefault('f', procedure_row.proowner))) AS acl
                WHERE procedure_row.oid = v_function AND acl.grantee <> procedure_row.proowner)
                SELECT 1 FROM ((SELECT * FROM expected EXCEPT SELECT * FROM actual) UNION ALL (SELECT * FROM actual EXCEPT SELECT * FROM expected)) AS acl_difference) THEN
            RAISE EXCEPTION 'G1 Instructor receipt writer authority is unsafe';
        END IF;
    END LOOP;
    IF to_regprocedure('public.ple_prepare_accepted_submission_retry_v1(uuid,uuid,uuid,uuid)') IS NOT NULL THEN
        RAISE EXCEPTION 'G1 retired retry capability is still present';
    END IF;
END;
$$;
COMMIT;
