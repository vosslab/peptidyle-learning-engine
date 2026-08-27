-- WP-PROF-G1 W4: exact, lease-fenced failure transition for accepted work.
--
-- The accepted response remains private.  This capability records only the
-- closed worker outcome needed to recover or route the work to an instructor.

BEGIN;

CREATE FUNCTION public.ple_fail_accepted_submission_execution_v1(
    p_tenant_id uuid,
    p_worker_job_id uuid,
    p_lease_token uuid,
    p_submission_id uuid,
    p_execution_generation bigint,
    p_worker_id uuid,
    p_failure_kind text,
    p_operation_reason text
) RETURNS TABLE(
    disposition text,
    resulting_execution_state text,
    resulting_evaluation_status text
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    v_execution public.grading_execution%ROWTYPE;
    v_assignment_id uuid;
    v_terminal boolean;
    v_reason text;
BEGIN
    -- ASVS V2.2/V2.3: validate the closed outcome vocabulary before any lock.
    IF p_tenant_id IS NULL
       OR p_worker_job_id IS NULL
       OR p_lease_token IS NULL
       OR p_submission_id IS NULL
       OR p_execution_generation IS NULL
       OR p_execution_generation <= 0
       OR p_worker_id IS NULL
       OR p_failure_kind IS NULL
       OR p_failure_kind NOT IN ('deterministic', 'transient', 'timed_out', 'terminal')
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant()
    THEN
        RAISE EXCEPTION 'accepted-submission failure arguments are invalid'
            USING ERRCODE = '22023';
    END IF;

    IF p_failure_kind = 'deterministic'
       AND (
           p_operation_reason IS NULL
           OR p_operation_reason NOT IN (
               'grader_contract_failure',
               'grader_execution_failure',
               'issued_evidence_integrity'
           )
       )
    THEN
        RAISE EXCEPTION 'accepted-submission deterministic reason is invalid'
            USING ERRCODE = '22023';
    END IF;

    IF p_failure_kind <> 'deterministic' AND p_operation_reason IS NOT NULL THEN
        RAISE EXCEPTION 'accepted-submission failure reason is invalid'
            USING ERRCODE = '22023';
    END IF;

    -- ASVS V2.3/V15.4: the one locked witness rechecks the complete live
    -- claim tuple and immutable accepted-input fence before any state change.
    SELECT execution.*
      INTO v_execution
      FROM public.ple_accepted_submission_execution_witness_v1 AS witness
      JOIN public.grading_execution AS execution
        ON (execution.tenant_id, execution.attempt_id)
         = (witness.tenant_id, witness.attempt_id)
      JOIN public.worker_job AS job
        ON (job.tenant_id, job.job_id)
         = (witness.tenant_id, witness.current_job_id)
      JOIN public.submission_evaluation AS evaluation
        ON (evaluation.tenant_id, evaluation.attempt_id, evaluation.submission_id)
         = (execution.tenant_id, execution.attempt_id, execution.submission_id)
     WHERE witness.tenant_id = p_tenant_id
       AND witness.current_job_id = p_worker_job_id
       AND witness.submission_id = p_submission_id
       AND witness.execution_generation = p_execution_generation
       AND witness.execution_state = 'running'
       AND witness.active_worker_id = p_worker_id
       AND witness.job_state = 'leased'
       AND witness.lease_token = p_lease_token
       AND witness.lease_expires_at > transaction_timestamp()
       AND witness.retention_lifecycle = 'active'
       AND witness.grading_status = 'automated_pending'
       AND witness.automated_result_canonical_json IS NULL
       AND witness.automated_result_sha256 IS NULL
       AND job.payload = jsonb_build_object(
           'kind',
           'gradeAcceptedSubmission',
           'attempt',
           witness.attempt_id::text,
           'submission',
           witness.submission_id::text,
           'execution_generation',
           witness.execution_generation
       )
     FOR UPDATE OF execution, job, evaluation;

    IF NOT FOUND THEN
        RETURN QUERY SELECT 'claim_no_longer_active', NULL::text, NULL::text;
        RETURN;
    END IF;

    SELECT witness.assignment_id
      INTO v_assignment_id
      FROM public.ple_accepted_submission_execution_witness_v1 AS witness
     WHERE witness.tenant_id = p_tenant_id
       AND witness.attempt_id = v_execution.attempt_id
       AND witness.current_job_id = p_worker_job_id
       AND witness.submission_id = p_submission_id
       AND witness.execution_generation = p_execution_generation;

    IF NOT FOUND THEN
        RETURN QUERY SELECT 'claim_no_longer_active', NULL::text, NULL::text;
        RETURN;
    END IF;

    SELECT p_failure_kind IN ('deterministic', 'terminal')
        OR job.attempt_count >= job.max_attempts
      INTO v_terminal
      FROM public.worker_job AS job
     WHERE job.tenant_id = p_tenant_id
       AND job.job_id = p_worker_job_id;

    IF v_terminal THEN
        v_reason := COALESCE(
            p_operation_reason,
            CASE
                WHEN p_failure_kind IN ('transient', 'timed_out') THEN 'retry_exhausted'
                ELSE 'grader_execution_failure'
            END
        );

        UPDATE public.submission_evaluation AS evaluation
           SET grading_status = 'automated_exception',
               evaluated_at = transaction_timestamp(),
               evaluation_revision = evaluation.evaluation_revision + 1
         WHERE evaluation.tenant_id = p_tenant_id
           AND evaluation.attempt_id = v_execution.attempt_id
           AND evaluation.submission_id = p_submission_id
           AND evaluation.grading_status = 'automated_pending'
           AND evaluation.automated_result_canonical_json IS NULL
           AND evaluation.automated_result_sha256 IS NULL;

        IF NOT FOUND THEN
            RETURN QUERY SELECT 'claim_no_longer_active', NULL::text, NULL::text;
            RETURN;
        END IF;

        UPDATE public.grading_execution AS execution
           SET state = 'exception',
               active_worker_id = NULL,
               updated_at = transaction_timestamp()
         WHERE execution.tenant_id = p_tenant_id
           AND execution.attempt_id = v_execution.attempt_id;

        UPDATE public.worker_job AS job
           SET state = 'dead',
               lease_token = NULL,
               lease_expires_at = NULL,
               last_error = CASE
                   WHEN p_failure_kind = 'timed_out' THEN 'timed_out'
                   ELSE 'permanent'
               END,
               completed_at = transaction_timestamp()
         WHERE job.tenant_id = p_tenant_id
           AND job.job_id = p_worker_job_id;

        INSERT INTO public.grading_execution_receipt (
            tenant_id,
            receipt_id,
            attempt_id,
            submission_id,
            submission_occurred_at,
            course_id,
            execution_generation,
            resulting_state,
            worker_id
        ) VALUES (
            p_tenant_id,
            gen_random_uuid(),
            v_execution.attempt_id,
            p_submission_id,
            v_execution.submission_occurred_at,
            v_execution.course_id,
            p_execution_generation,
            'exception',
            p_worker_id
        );

        INSERT INTO public.grading_operation (
            tenant_id,
            attempt_id,
            submission_id,
            submission_occurred_at,
            assignment_id,
            course_id,
            target_kind,
            reason,
            state,
            next_action
        ) VALUES (
            p_tenant_id,
            v_execution.attempt_id,
            p_submission_id,
            v_execution.submission_occurred_at,
            v_assignment_id,
            v_execution.course_id,
            'submission',
            v_reason,
            'actionable',
            'retry'
        ) ON CONFLICT DO NOTHING;

        RETURN QUERY SELECT 'terminal', 'exception', 'automated_exception';
        RETURN;
    END IF;

    UPDATE public.grading_execution AS execution
       SET state = 'retry_wait',
           active_worker_id = NULL,
           retry_count = execution.retry_count + 1,
           updated_at = transaction_timestamp()
     WHERE execution.tenant_id = p_tenant_id
       AND execution.attempt_id = v_execution.attempt_id;

    UPDATE public.worker_job AS job
       SET state = 'ready',
           available_at = transaction_timestamp()
               + make_interval(
                   secs => (1 << LEAST(GREATEST(job.attempt_count - 1, 0), 8))
               ),
           lease_token = NULL,
           lease_expires_at = NULL,
           last_error = CASE
               WHEN p_failure_kind = 'timed_out' THEN 'timed_out'
               ELSE 'transient'
           END,
           completed_at = NULL
     WHERE job.tenant_id = p_tenant_id
       AND job.job_id = p_worker_job_id;

    INSERT INTO public.grading_execution_receipt (
        tenant_id,
        receipt_id,
        attempt_id,
        submission_id,
        submission_occurred_at,
        course_id,
        execution_generation,
        resulting_state,
        worker_id
    ) VALUES (
        p_tenant_id,
        gen_random_uuid(),
        v_execution.attempt_id,
        p_submission_id,
        v_execution.submission_occurred_at,
        v_execution.course_id,
        p_execution_generation,
        'retry_wait',
        p_worker_id
    );

    RETURN QUERY SELECT 'rescheduled', 'retry_wait', 'automated_pending';
END;
$$;

ALTER FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) OWNER TO ple_accepted_submission_execution_worker;

REVOKE ALL ON FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) FROM ple_app;

REVOKE ALL ON FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) FROM ple_auth;

REVOKE ALL ON FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) FROM ple_student;

REVOKE ALL ON FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) FROM ple_grader;

REVOKE ALL ON FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) FROM ple_grading_reader;

REVOKE ALL ON FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) FROM ple_queue_broker;

REVOKE ALL ON FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) FROM ple_automated_grading_broker;

REVOKE ALL ON FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) FROM ple_retention_broker;

REVOKE ALL ON FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) FROM ple_accepted_submission_execution_reader;

REVOKE ALL ON FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) FROM ple_accepted_submission_execution_fast_path;

GRANT EXECUTE ON FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) TO ple_accepted_submission_execution;

GRANT EXECUTE ON FUNCTION public.ple_fail_accepted_submission_execution_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid,
    text,
    text
) TO ple_accepted_submission_execution_fast_path;

-- ASVS V8.2/V8.3: fail installation when the capability's identity or
-- callable surface differs from the sealed worker contract.
DO $$
DECLARE
    v_function regprocedure := to_regprocedure(
        'public.ple_fail_accepted_submission_execution_v1('
        || 'uuid,uuid,uuid,uuid,bigint,uuid,text,text)'
    );
BEGIN
    IF v_function IS NULL
       OR EXISTS (
           SELECT 1
             FROM pg_catalog.pg_proc AS procedure_row
            WHERE procedure_row.oid = v_function
              AND (
                  procedure_row.proowner
                      <> 'ple_accepted_submission_execution_worker'::regrole
                  OR NOT procedure_row.prosecdef
                  OR procedure_row.proconfig
                      IS DISTINCT FROM ARRAY['search_path=pg_catalog, public, pg_temp']
              )
       )
       OR NOT has_function_privilege(
           'ple_accepted_submission_execution',
           v_function,
           'EXECUTE'
       )
       OR has_function_privilege('public', v_function, 'EXECUTE')
       OR EXISTS (
           SELECT 1
             FROM unnest(ARRAY[
                 'ple_app',
                 'ple_auth',
                 'ple_student',
                 'ple_grader',
                 'ple_grading_reader',
                 'ple_queue_broker',
                 'ple_automated_grading_broker',
                 'ple_retention_broker',
                 'ple_accepted_submission_execution_reader'
             ]) AS denied_role(role_name)
            WHERE has_function_privilege(denied_role.role_name, v_function, 'EXECUTE')
       )
       OR EXISTS (
           WITH expected(grantee, privilege_type, is_grantable) AS (
               VALUES
                   (
                       'ple_accepted_submission_execution'::regrole::oid,
                       'EXECUTE'::text,
                       false
                   ),
                   (
                       'ple_accepted_submission_execution_fast_path'::regrole::oid,
                       'EXECUTE'::text,
                       false
                   )
           ), actual AS (
               SELECT
                   privilege_row.grantee,
                   privilege_row.privilege_type,
                   privilege_row.is_grantable
                 FROM pg_catalog.pg_proc AS procedure_row
                 CROSS JOIN LATERAL pg_catalog.aclexplode(
                     COALESCE(
                         procedure_row.proacl,
                         pg_catalog.acldefault('f', procedure_row.proowner)
                     )
                 ) AS privilege_row
                WHERE procedure_row.oid = v_function
                  AND privilege_row.grantee <> procedure_row.proowner
           )
           SELECT 1
             FROM (
                 (SELECT * FROM actual EXCEPT SELECT * FROM expected)
                 UNION ALL
                 (SELECT * FROM expected EXCEPT SELECT * FROM actual)
             ) AS privilege_difference
       )
    THEN
        RAISE EXCEPTION 'accepted-submission failure capability authority is invalid'
            USING ERRCODE = '42501';
    END IF;
END;
$$;

COMMIT;
