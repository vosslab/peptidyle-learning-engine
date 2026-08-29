-- Retire the pre-production manual scoring mutation path.
-- This is intentionally a forward catalog transition.  Earlier migrations are evidence.

BEGIN;

DO $$
BEGIN
    IF to_regclass('public.manual_grade_receipt') IS NULL
       OR to_regprocedure('public.ple_bind_manual_grade_invalidation_v1(uuid,uuid,uuid)') IS NULL
       OR to_regprocedure('public.ple_commit_delete_retention_work_before_passwordless_identity(uuid,uuid,uuid,uuid,text,bigint)') IS NULL
       OR to_regprocedure('public.ple_verify_base_course_completion_internal(uuid,uuid,text,jsonb)') IS NULL THEN
        RAISE EXCEPTION 'automated scoring transition requires the exact pre-1883 catalog'
            USING ERRCODE = '55000';
    END IF;
    IF EXISTS (SELECT 1 FROM public.manual_grade_receipt) THEN
        RAISE EXCEPTION 'manual_grade_receipt is nonempty; rebuild the disposable pre-production volume before 1883'
            USING ERRCODE = '55000';
    END IF;
END;
$$;

ALTER TABLE public.submission_receipt_snapshot
    DROP CONSTRAINT submission_receipt_snapshot_attempt_payload_shape_check;
ALTER TABLE public.submission_receipt_snapshot
    ADD CONSTRAINT submission_receipt_snapshot_attempt_payload_shape_check CHECK (
        receipt_attempt_payload IS NULL OR (
            jsonb_typeof(receipt_attempt_payload) = 'object'
            AND receipt_attempt_payload ?& ARRAY['id', 'tenant', 'response', 'status']
            AND receipt_attempt_payload ->> 'status' = ANY (
                ARRAY['submitted', 'auto_submitted', 'exempt']
            )
            AND receipt_attempt_payload -> 'response' = 'null'::jsonb
        )
    );

CREATE OR REPLACE FUNCTION public.ple_guard_receipt_attempt_snapshot()
RETURNS trigger
LANGUAGE plpgsql
SECURITY INVOKER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF current_user = 'ple_retention_broker' THEN RETURN OLD; END IF;
        RAISE EXCEPTION 'receipt attempt snapshot is retention-deleted only' USING ERRCODE = '42501';
    END IF;
    IF NEW.receipt_attempt_payload ->> 'id' IS DISTINCT FROM NEW.attempt_id::text
       OR NEW.receipt_attempt_payload ->> 'tenant' IS DISTINCT FROM NEW.tenant_id::text
       OR NEW.receipt_attempt_payload -> 'response' <> 'null'::jsonb
       OR NEW.receipt_attempt_payload ->> 'status' NOT IN ('submitted', 'auto_submitted', 'exempt') THEN
        RAISE EXCEPTION 'receipt attempt snapshot is not answer-free terminal evidence' USING ERRCODE = '22023';
    END IF;
    IF TG_OP = 'UPDATE' AND (
        NEW.receipt_attempt_payload IS DISTINCT FROM OLD.receipt_attempt_payload
        OR NEW.receipt_attempt_canonical_json IS DISTINCT FROM OLD.receipt_attempt_canonical_json
        OR NEW.receipt_attempt_payload_sha256 IS DISTINCT FROM OLD.receipt_attempt_payload_sha256
        OR NEW.run_canonical_json IS DISTINCT FROM OLD.run_canonical_json
        OR NEW.run_payload IS DISTINCT FROM OLD.run_payload
        OR NEW.run_payload_sha256 IS DISTINCT FROM OLD.run_payload_sha256
        OR NEW.summary_canonical_json IS DISTINCT FROM OLD.summary_canonical_json
        OR NEW.summary_payload IS DISTINCT FROM OLD.summary_payload
        OR NEW.summary_payload_sha256 IS DISTINCT FROM OLD.summary_payload_sha256
        OR NEW.presentation_canonical_json IS DISTINCT FROM OLD.presentation_canonical_json
        OR NEW.presentation_payload IS DISTINCT FROM OLD.presentation_payload
        OR NEW.presentation_payload_sha256 IS DISTINCT FROM OLD.presentation_payload_sha256
        OR NEW.canonical_json_version IS DISTINCT FROM OLD.canonical_json_version
    ) THEN
        RAISE EXCEPTION 'receipt attempt snapshot is immutable' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END;
$$;

ALTER TABLE public.submission_evaluation
    DROP CONSTRAINT submission_evaluation_grading_status_check,
    DROP CONSTRAINT submission_evaluation_result_shape_check;
ALTER TABLE public.submission_evaluation
    ADD CONSTRAINT submission_evaluation_grading_status_check CHECK (
        grading_status = ANY (ARRAY['automated_pending', 'automated_exception', 'graded', 'exempt'])
    ),
    ADD CONSTRAINT submission_evaluation_result_shape_check CHECK (
        (grading_status = ANY ('{automated_pending,automated_exception}')
            AND credit_fraction IS NULL AND correct IS NULL)
        OR (grading_status = ANY ('{graded,exempt}')
            AND credit_fraction IS NOT NULL AND correct IS NOT NULL)
    );

ALTER TABLE public.question_attempt
    DROP CONSTRAINT question_attempt_status_check,
    DROP CONSTRAINT question_attempt_submission_time_check;
ALTER TABLE public.question_attempt
    ADD CONSTRAINT question_attempt_status_check CHECK (
        attempt_status = ANY (ARRAY['in_progress', 'submitted', 'auto_submitted', 'cleared', 'exempt'])
    ),
    ADD CONSTRAINT question_attempt_submission_time_check CHECK (
        (attempt_status = 'in_progress' AND submitted_at IS NULL)
        OR (attempt_status = ANY ('{submitted,auto_submitted}') AND submitted_at IS NOT NULL)
        OR attempt_status = ANY ('{cleared,exempt}')
    );

-- These two accepted functions are unusually large and have later wrappers.
-- Derive their exact current definitions from the predecessor catalog, make one
-- audited replacement each, and preserve identity, owner, configuration, ACL,
-- and security properties rather than copying stale historical source.
DO $$
DECLARE
    v_proc oid;
    v_definition text;
    v_old text;
    v_new text;
    v_owner oid;
    v_acl aclitem[];
    v_config text[];
    v_security boolean;
BEGIN
    v_proc := 'public.ple_commit_delete_retention_work_before_passwordless_identity(uuid,uuid,uuid,uuid,text,bigint)'::regprocedure;
    SELECT pg_get_functiondef(v_proc), proowner, proacl, proconfig, prosecdef
      INTO v_definition, v_owner, v_acl, v_config, v_security FROM pg_proc WHERE oid = v_proc;
    v_old := 'DELETE FROM public.manual_grade_receipt' || chr(10) ||
        'WHERE tenant_id = p_tenant AND course_id = p_course; ';
    IF length(v_definition) - length(replace(v_definition, v_old, '')) <> length(v_old) THEN
        RAISE EXCEPTION 'retention predecessor does not contain one exact manual receipt deletion fragment';
    END IF;
    v_old := 'UNION ALL SELECT 1' || chr(10) ||
        'FROM public.manual_grade_receipt receipt WHERE receipt.tenant_id = p_tenant' || chr(10) ||
        'AND receipt.course_id = p_course ';
    IF length(v_definition) - length(replace(v_definition, v_old, '')) <> length(v_old) THEN
        RAISE EXCEPTION 'retention predecessor does not contain one exact manual receipt residual fragment';
    END IF;
    v_new := replace(replace(v_definition,
        'DELETE FROM public.manual_grade_receipt' || chr(10) || 'WHERE tenant_id = p_tenant AND course_id = p_course; ', ''),
        'UNION ALL SELECT 1' || chr(10) ||
        'FROM public.manual_grade_receipt receipt WHERE receipt.tenant_id = p_tenant' || chr(10) ||
        'AND receipt.course_id = p_course ', '');
    EXECUTE v_new;
    IF NOT EXISTS (SELECT 1 FROM pg_proc WHERE oid = v_proc AND proowner = v_owner
        AND proacl IS NOT DISTINCT FROM v_acl AND proconfig IS NOT DISTINCT FROM v_config AND prosecdef = v_security) THEN
        RAISE EXCEPTION 'retention predecessor identity or authority changed';
    END IF;
    IF position('manual_grade_receipt' IN pg_get_functiondef(v_proc)) <> 0 THEN
        RAISE EXCEPTION 'retention predecessor retained a manual receipt reference';
    END IF;

    v_proc := 'public.ple_verify_base_course_completion_internal(uuid,uuid,text,jsonb)'::regprocedure;
    SELECT pg_get_functiondef(v_proc), proowner, proacl, proconfig, prosecdef
      INTO v_definition, v_owner, v_acl, v_config, v_security FROM pg_proc WHERE oid = v_proc;
    v_old := ' UNION ALL SELECT 1 FROM public.manual_grade_receipt UNION ALL SELECT 1 FROM public.question_prefetch';
    IF length(v_definition) - length(replace(v_definition, v_old, '')) <> length(v_old) THEN
        RAISE EXCEPTION 'completion verifier does not contain one exact manual receipt branch';
    END IF;
    v_new := replace(v_definition, v_old, ' UNION ALL SELECT 1 FROM public.question_prefetch');
    EXECUTE v_new;
    IF NOT EXISTS (SELECT 1 FROM pg_proc WHERE oid = v_proc AND proowner = v_owner
        AND proacl IS NOT DISTINCT FROM v_acl AND proconfig IS NOT DISTINCT FROM v_config AND prosecdef = v_security) THEN
        RAISE EXCEPTION 'completion verifier identity or authority changed';
    END IF;
    IF position('manual_grade_receipt' IN pg_get_functiondef(v_proc)) <> 0 THEN
        RAISE EXCEPTION 'completion verifier retained a manual receipt reference';
    END IF;
END;
$$;

ALTER TABLE public.scoring_invalidation_origin
    DROP CONSTRAINT scoring_invalidation_origin_kind_check,
    DROP CONSTRAINT scoring_invalidation_origin_actor_shape_check;
ALTER TABLE public.scoring_invalidation_origin
    ADD CONSTRAINT scoring_invalidation_origin_kind_check CHECK (
        origin_kind = ANY ('{instructor_recalculation,assignment_definition,learner_support,accepted_submission_completion}')
    ),
    ADD CONSTRAINT scoring_invalidation_origin_actor_shape_check CHECK (
        (origin_kind = ANY ('{instructor_recalculation,assignment_definition,learner_support}') AND actor_id IS NOT NULL)
        OR (origin_kind = 'accepted_submission_completion' AND actor_id IS NULL)
    );

-- Preserve the mature invalidation implementations byte-for-byte except for
-- the retired origin token.  Exact predecessor checks keep this migration
-- fail-closed if an earlier migration changes either implementation.
DO $$
DECLARE
    v_proc regprocedure;
    v_definition text;
    v_new text;
    v_old constant text := '''instructor_recalculation'', ''assignment_definition'', ''manual_grade'',';
    v_replacement constant text := '''instructor_recalculation'', ''assignment_definition'',';
    v_owner oid;
    v_acl aclitem[];
    v_config text[];
    v_security boolean;
BEGIN
    v_proc := 'public.ple_bind_scoring_invalidation_origin_v1('
        'uuid,uuid,uuid,bigint,uuid,text,uuid,uuid,integer)'::regprocedure;
    SELECT pg_get_functiondef(v_proc), proowner, proacl, proconfig, prosecdef
      INTO v_definition, v_owner, v_acl, v_config, v_security
      FROM pg_proc
     WHERE oid = v_proc;
    IF length(v_definition) - length(replace(v_definition, v_old, ''))
        <> 2 * length(v_old) THEN
        RAISE EXCEPTION 'origin binder does not contain two exact manual origin tokens';
    END IF;
    v_new := replace(v_definition, v_old, v_replacement);
    IF v_new IS NOT DISTINCT FROM v_definition
       OR position('manual_grade' IN v_new) <> 0 THEN
        RAISE EXCEPTION 'origin binder rewrite did not close the manual origin';
    END IF;
    EXECUTE v_new;
    IF NOT EXISTS (
        SELECT 1
          FROM pg_proc
         WHERE oid = v_proc
           AND proowner = v_owner
           AND proacl IS NOT DISTINCT FROM v_acl
           AND proconfig IS NOT DISTINCT FROM v_config
           AND prosecdef = v_security
    ) THEN
        RAISE EXCEPTION 'origin binder identity or authority changed';
    END IF;

    v_proc := 'public.ple_request_scoring_invalidation_v1('
        'uuid,uuid,uuid,text,uuid,uuid,uuid,integer)'::regprocedure;
    SELECT pg_get_functiondef(v_proc), proowner, proacl, proconfig, prosecdef
      INTO v_definition, v_owner, v_acl, v_config, v_security
      FROM pg_proc
     WHERE oid = v_proc;
    IF length(v_definition) - length(replace(v_definition, v_old, ''))
        <> 2 * length(v_old) THEN
        RAISE EXCEPTION 'invalidation request does not contain two exact manual origin tokens';
    END IF;
    v_new := replace(v_definition, v_old, v_replacement);
    IF v_new IS NOT DISTINCT FROM v_definition
       OR position('manual_grade' IN v_new) <> 0 THEN
        RAISE EXCEPTION 'invalidation request rewrite did not close the manual origin';
    END IF;
    EXECUTE v_new;
    IF NOT EXISTS (
        SELECT 1
          FROM pg_proc
         WHERE oid = v_proc
           AND proowner = v_owner
           AND proacl IS NOT DISTINCT FROM v_acl
           AND proconfig IS NOT DISTINCT FROM v_config
           AND prosecdef = v_security
    ) THEN
        RAISE EXCEPTION 'invalidation request identity or authority changed';
    END IF;
END;
$$;

DROP POLICY scoring_invalidation_source_manual_grade ON public.manual_grade_receipt;
REVOKE SELECT ON public.manual_grade_receipt FROM ple_scoring_invalidation_source_broker;
REVOKE ALL ON FUNCTION public.ple_bind_manual_grade_invalidation_v1(uuid,uuid,uuid) FROM PUBLIC;
DROP FUNCTION public.ple_bind_manual_grade_invalidation_v1(uuid,uuid,uuid);
REVOKE ALL ON FUNCTION public.ple_bind_attempt_support_invalidation_v1(uuid,uuid,uuid),
    public.ple_bind_assignment_definition_invalidation_v1(uuid,uuid,uuid,uuid,bigint,uuid),
    public.ple_bind_accepted_completion_invalidation_v1(uuid,uuid,uuid,bigint,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_bind_attempt_support_invalidation_v1(uuid,uuid,uuid),
    public.ple_bind_assignment_definition_invalidation_v1(uuid,uuid,uuid,uuid,bigint,uuid) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_bind_accepted_completion_invalidation_v1(uuid,uuid,uuid,bigint,uuid)
    TO ple_accepted_submission_execution, ple_accepted_submission_execution_fast_path;

DROP TABLE public.manual_grade_receipt;

DO $$
DECLARE v_definition text;
BEGIN
    SELECT string_agg(pg_get_functiondef(p.oid), E'\n') INTO v_definition FROM pg_proc p
      JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public'
        AND p.proname IN ('ple_guard_receipt_attempt_snapshot','ple_commit_delete_retention_work_before_passwordless_identity',
            'ple_verify_base_course_completion_internal','ple_bind_scoring_invalidation_origin_v1','ple_request_scoring_invalidation_v1');
    IF to_regclass('public.manual_grade_receipt') IS NOT NULL
       OR to_regprocedure('public.ple_bind_manual_grade_invalidation_v1(uuid,uuid,uuid)') IS NOT NULL
       OR EXISTS(SELECT 1 FROM public.scoring_invalidation_origin WHERE origin_kind='manual_grade')
       OR position('manual_grade_receipt' IN coalesce(v_definition,''))<>0
       OR position('manual_grade' IN coalesce(v_definition,''))<>0 THEN
        RAISE EXCEPTION 'retired manual scoring catalog remains reachable';
    END IF;
    IF EXISTS(SELECT 1 FROM pg_proc WHERE oid IN (
          'public.ple_bind_scoring_invalidation_origin_v1(uuid,uuid,uuid,bigint,uuid,text,uuid,uuid,integer)'::regprocedure,
          'public.ple_request_scoring_invalidation_v1(uuid,uuid,uuid,text,uuid,uuid,uuid,integer)'::regprocedure)
        AND (proowner <> 'ple_scoring_invalidation_origin_broker'::regrole
             OR NOT prosecdef
             OR proconfig IS DISTINCT FROM ARRAY['search_path=pg_catalog, public, pg_temp'])) THEN
        RAISE EXCEPTION 'generic invalidation authority is unsafe';
    END IF;
    IF NOT has_function_privilege('ple_scoring_invalidation_source_broker',
            'public.ple_bind_scoring_invalidation_origin_v1(uuid,uuid,uuid,bigint,uuid,text,uuid,uuid,integer)', 'EXECUTE')
       OR has_function_privilege('ple_scoring_invalidation_source_broker',
            'public.ple_request_scoring_invalidation_v1(uuid,uuid,uuid,text,uuid,uuid,uuid,integer)', 'EXECUTE')
       OR NOT has_function_privilege('ple_instructor_grading_operation_broker',
            'public.ple_request_scoring_invalidation_v1(uuid,uuid,uuid,text,uuid,uuid,uuid,integer)', 'EXECUTE')
       OR has_function_privilege('ple_instructor_grading_operation_broker',
            'public.ple_bind_scoring_invalidation_origin_v1(uuid,uuid,uuid,bigint,uuid,text,uuid,uuid,integer)', 'EXECUTE') THEN
        RAISE EXCEPTION 'generic invalidation caller matrix is unsafe';
    END IF;
    IF has_function_privilege('ple_app','public.ple_bind_scoring_invalidation_origin_v1(uuid,uuid,uuid,bigint,uuid,text,uuid,uuid,integer)','EXECUTE')
       OR has_function_privilege('ple_app','public.ple_request_scoring_invalidation_v1(uuid,uuid,uuid,text,uuid,uuid,uuid,integer)','EXECUTE')
       OR has_function_privilege('public','public.ple_bind_scoring_invalidation_origin_v1(uuid,uuid,uuid,bigint,uuid,text,uuid,uuid,integer)','EXECUTE')
       OR has_function_privilege('public','public.ple_request_scoring_invalidation_v1(uuid,uuid,uuid,text,uuid,uuid,uuid,integer)','EXECUTE') THEN
        RAISE EXCEPTION 'generic invalidation capability is overprivileged';
    END IF;
END;
$$;

COMMIT;
