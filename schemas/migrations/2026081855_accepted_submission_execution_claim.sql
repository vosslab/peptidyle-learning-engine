-- WP-PROF-G1 / G1-W4: one lease transition for generic and exact claims.
-- Both public entry points return the same opaque, fully fenced claim tuple.

BEGIN;

CREATE FUNCTION public.ple_claim_accepted_submission_execution_transition_v1(
    p_target_tenant_id uuid,
    p_target_attempt_id uuid,
    p_target_submission_id uuid,
    p_target_worker_job_id uuid,
    p_lease_token uuid,
    p_worker_id uuid,
    p_lease_seconds integer
) RETURNS TABLE (
    tenant_id uuid,
    worker_job_id uuid,
    worker_lease_token uuid,
    submission_id uuid,
    execution_generation bigint,
    worker_id uuid
)
LANGUAGE plpgsql
SECURITY INVOKER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $claim_transition$
DECLARE
    v_execution public.grading_execution%ROWTYPE;
    v_now timestamp with time zone := pg_catalog.transaction_timestamp();
    v_row_count integer;
BEGIN
    -- ASVS 2.2.1-2.2.2: validate the complete lease boundary before state access.
    IF p_lease_token IS NULL
       OR p_worker_id IS NULL
       OR p_lease_seconds IS NULL
       OR p_lease_seconds NOT BETWEEN 1 AND 900 THEN
        RAISE EXCEPTION 'accepted-submission claim arguments are invalid'
            USING ERRCODE = '22023';
    END IF;

    IF pg_catalog.num_nonnulls(
        p_target_tenant_id,
        p_target_attempt_id,
        p_target_submission_id,
        p_target_worker_job_id
    ) NOT IN (0, 4) THEN
        RAISE EXCEPTION 'accepted-submission exact claim target is incomplete'
            USING ERRCODE = '22023';
    END IF;

    -- ASVS 2.3.3-2.3.4 and 15.4.2: converge one ready or expired
    -- execution at its attempt ceiling while all three state rows are locked.
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
        ON (evaluation.tenant_id, evaluation.attempt_id)
         = (witness.tenant_id, witness.attempt_id)
     WHERE (
            (
                witness.execution_state IN ('ready', 'retry_wait')
                AND witness.job_state = 'ready'
                AND witness.available_at <= v_now
            )
            OR (
                witness.execution_state = 'running'
                AND witness.job_state = 'leased'
                AND witness.lease_expires_at <= v_now
            )
       )
       AND witness.attempt_count >= witness.max_attempts
       AND witness.retention_lifecycle = 'active'
       AND witness.grading_status = 'automated_pending'
       AND witness.automated_result_canonical_json IS NULL
       AND witness.automated_result_sha256 IS NULL
       AND job.payload = pg_catalog.jsonb_build_object(
            'kind', 'gradeAcceptedSubmission',
            'attempt', witness.attempt_id::text,
            'submission', witness.submission_id::text,
            'execution_generation', witness.execution_generation
       )
       AND (
            p_target_tenant_id IS NULL
            OR (
                witness.tenant_id = p_target_tenant_id
                AND witness.attempt_id = p_target_attempt_id
                AND witness.submission_id = p_target_submission_id
                AND witness.current_job_id = p_target_worker_job_id
            )
       )
     ORDER BY witness.available_at, witness.current_job_id
       FOR UPDATE OF execution, job, evaluation SKIP LOCKED
     LIMIT 1;

    IF FOUND THEN
        PERFORM pg_catalog.set_config(
            'ple.tenant_id',
            v_execution.tenant_id::text,
            true
        );

        UPDATE public.worker_job AS job_row
           SET state = 'dead',
               lease_token = NULL,
               lease_expires_at = NULL,
               last_error = CASE
                   WHEN job_row.state = 'leased' THEN 'timed_out'
                   ELSE COALESCE(job_row.last_error, 'permanent')
               END,
               completed_at = v_now
         WHERE job_row.tenant_id = v_execution.tenant_id
           AND job_row.job_id = v_execution.current_job_id
           AND job_row.attempt_count >= job_row.max_attempts
           AND (
                (
                    job_row.state = 'ready'
                    AND job_row.available_at <= v_now
                )
                OR (
                    job_row.state = 'leased'
                    AND job_row.lease_expires_at <= v_now
                )
           );
        GET DIAGNOSTICS v_row_count = ROW_COUNT;
        IF v_row_count <> 1 THEN
            RAISE EXCEPTION 'exhausted accepted-submission job changed while locked'
                USING ERRCODE = '40001';
        END IF;

        UPDATE public.grading_execution AS execution_row
           SET state = 'exception',
               active_worker_id = NULL,
               updated_at = v_now
         WHERE execution_row.tenant_id = v_execution.tenant_id
           AND execution_row.attempt_id = v_execution.attempt_id
           AND execution_row.submission_id = v_execution.submission_id
           AND execution_row.execution_generation = v_execution.execution_generation
           AND execution_row.current_job_id = v_execution.current_job_id
           AND execution_row.state IN ('ready', 'retry_wait', 'running');
        GET DIAGNOSTICS v_row_count = ROW_COUNT;
        IF v_row_count <> 1 THEN
            RAISE EXCEPTION 'exhausted accepted-submission execution changed while locked'
                USING ERRCODE = '40001';
        END IF;

        UPDATE public.submission_evaluation AS evaluation_row
           SET grading_status = 'automated_exception',
               evaluated_at = v_now,
               evaluation_revision = evaluation_revision + 1
         WHERE evaluation_row.tenant_id = v_execution.tenant_id
           AND evaluation_row.attempt_id = v_execution.attempt_id
           AND evaluation_row.submission_id = v_execution.submission_id
           AND evaluation_row.grading_status = 'automated_pending';
        GET DIAGNOSTICS v_row_count = ROW_COUNT;
        IF v_row_count <> 1 THEN
            RAISE EXCEPTION 'exhausted accepted-submission evaluation changed while locked'
                USING ERRCODE = '40001';
        END IF;

        INSERT INTO public.grading_execution_receipt (
            tenant_id,
            receipt_id,
            attempt_id,
            submission_id,
            submission_occurred_at,
            course_id,
            execution_generation,
            resulting_state,
            worker_id,
            occurred_at
        ) VALUES (
            v_execution.tenant_id,
            pg_catalog.gen_random_uuid(),
            v_execution.attempt_id,
            v_execution.submission_id,
            v_execution.submission_occurred_at,
            v_execution.course_id,
            v_execution.execution_generation,
            'exception',
            NULL,
            v_now
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
            next_action,
            created_at,
            updated_at
        )
        SELECT v_execution.tenant_id,
               v_execution.attempt_id,
               v_execution.submission_id,
               v_execution.submission_occurred_at,
               enrollment.assignment_id,
               v_execution.course_id,
               'submission',
               'retry_exhausted',
               'actionable',
               'retry',
               v_now,
               v_now
          FROM public.question_attempt AS attempt
          JOIN public.assignment_run AS run
            ON (run.tenant_id, run.run_id)
             = (attempt.tenant_id, attempt.run_id)
          JOIN public.enrollment AS enrollment
            ON (enrollment.tenant_id, enrollment.enrollment_id)
             = (run.tenant_id, run.enrollment_id)
         WHERE attempt.tenant_id = v_execution.tenant_id
           AND attempt.attempt_id = v_execution.attempt_id
        ON CONFLICT DO NOTHING;
    END IF;

    -- The generic and exact entry points meet at this one locked lease transition.
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
        ON (evaluation.tenant_id, evaluation.attempt_id)
         = (witness.tenant_id, witness.attempt_id)
     WHERE witness.retention_lifecycle = 'active'
       AND witness.grading_status = 'automated_pending'
       AND witness.automated_result_canonical_json IS NULL
       AND witness.automated_result_sha256 IS NULL
       AND job.payload = pg_catalog.jsonb_build_object(
            'kind', 'gradeAcceptedSubmission',
            'attempt', witness.attempt_id::text,
            'submission', witness.submission_id::text,
            'execution_generation', witness.execution_generation
       )
       AND (
            p_target_tenant_id IS NULL
            OR (
                witness.tenant_id = p_target_tenant_id
                AND witness.attempt_id = p_target_attempt_id
                AND witness.submission_id = p_target_submission_id
                AND witness.current_job_id = p_target_worker_job_id
            )
       )
       AND (
            (
                witness.job_state = 'ready'
                AND witness.available_at <= v_now
                AND witness.attempt_count < witness.max_attempts
                AND witness.execution_state IN ('ready', 'retry_wait')
            )
            OR (
                witness.job_state = 'leased'
                AND witness.lease_expires_at <= v_now
                AND witness.attempt_count < witness.max_attempts
                AND witness.execution_state = 'running'
            )
       )
     ORDER BY witness.available_at, witness.current_job_id
       FOR UPDATE OF execution, job, evaluation SKIP LOCKED
     LIMIT 1;

    IF NOT FOUND THEN
        RETURN;
    END IF;

    PERFORM pg_catalog.set_config(
        'ple.tenant_id',
        v_execution.tenant_id::text,
        true
    );

    UPDATE public.worker_job AS job_row
       SET state = 'leased',
           lease_token = p_lease_token,
           lease_expires_at = v_now + pg_catalog.make_interval(secs => p_lease_seconds),
           attempt_count = attempt_count + 1,
           last_error = NULL,
           completed_at = NULL
     WHERE job_row.tenant_id = v_execution.tenant_id
       AND job_row.job_id = v_execution.current_job_id
       AND (
            (
                job_row.state = 'ready'
                AND job_row.available_at <= v_now
                AND job_row.attempt_count < job_row.max_attempts
            )
            OR (
                job_row.state = 'leased'
                AND job_row.lease_expires_at <= v_now
                AND job_row.attempt_count < job_row.max_attempts
            )
       );
    GET DIAGNOSTICS v_row_count = ROW_COUNT;
    IF v_row_count <> 1 THEN
        RAISE EXCEPTION 'accepted-submission job changed while locked'
            USING ERRCODE = '40001';
    END IF;

    UPDATE public.grading_execution AS execution_row
       SET state = 'running',
           active_worker_id = p_worker_id,
           updated_at = v_now
     WHERE execution_row.tenant_id = v_execution.tenant_id
       AND execution_row.attempt_id = v_execution.attempt_id
       AND execution_row.submission_id = v_execution.submission_id
       AND execution_row.execution_generation = v_execution.execution_generation
       AND execution_row.current_job_id = v_execution.current_job_id
       AND execution_row.state IN ('ready', 'retry_wait', 'running');
    GET DIAGNOSTICS v_row_count = ROW_COUNT;
    IF v_row_count <> 1 THEN
        RAISE EXCEPTION 'accepted-submission execution changed while locked'
            USING ERRCODE = '40001';
    END IF;

    INSERT INTO public.grading_execution_receipt (
        tenant_id,
        receipt_id,
        attempt_id,
        submission_id,
        submission_occurred_at,
        course_id,
        execution_generation,
        resulting_state,
        worker_id,
        occurred_at
    ) VALUES (
        v_execution.tenant_id,
        pg_catalog.gen_random_uuid(),
        v_execution.attempt_id,
        v_execution.submission_id,
        v_execution.submission_occurred_at,
        v_execution.course_id,
        v_execution.execution_generation,
        'running',
        p_worker_id,
        v_now
    );

    tenant_id := v_execution.tenant_id;
    worker_job_id := v_execution.current_job_id;
    worker_lease_token := p_lease_token;
    submission_id := v_execution.submission_id;
    execution_generation := v_execution.execution_generation;
    worker_id := p_worker_id;
    RETURN NEXT;
END;
$claim_transition$;

CREATE FUNCTION public.ple_claim_accepted_submission_execution_v1(
    p_lease_token uuid,
    p_worker_id uuid,
    p_lease_seconds integer
) RETURNS TABLE (
    tenant_id uuid,
    worker_job_id uuid,
    worker_lease_token uuid,
    submission_id uuid,
    execution_generation bigint,
    worker_id uuid
)
LANGUAGE sql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $generic_claim$
    SELECT *
      FROM public.ple_claim_accepted_submission_execution_transition_v1(
          NULL,
          NULL,
          NULL,
          NULL,
          p_lease_token,
          p_worker_id,
          p_lease_seconds
      );
$generic_claim$;

CREATE FUNCTION public.ple_claim_exact_accepted_submission_execution_v1(
    p_tenant_id uuid,
    p_attempt_id uuid,
    p_submission_id uuid,
    p_worker_job_id uuid,
    p_lease_token uuid,
    p_worker_id uuid,
    p_lease_seconds integer
) RETURNS TABLE (
    tenant_id uuid,
    worker_job_id uuid,
    worker_lease_token uuid,
    submission_id uuid,
    execution_generation bigint,
    worker_id uuid
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $exact_claim$
BEGIN
    -- An absent exact target must never degrade into generic queue ownership.
    IF p_tenant_id IS NULL
       OR p_attempt_id IS NULL
       OR p_submission_id IS NULL
       OR p_worker_job_id IS NULL THEN
        RAISE EXCEPTION 'accepted-submission exact claim target is invalid'
            USING ERRCODE = '22023';
    END IF;

    RETURN QUERY
    SELECT claim.tenant_id,
           claim.worker_job_id,
           claim.worker_lease_token,
           claim.submission_id,
           claim.execution_generation,
           claim.worker_id
      FROM public.ple_claim_accepted_submission_execution_transition_v1(
          p_tenant_id,
          p_attempt_id,
          p_submission_id,
          p_worker_job_id,
          p_lease_token,
          p_worker_id,
          p_lease_seconds
      ) AS claim;
END;
$exact_claim$;

ALTER FUNCTION public.ple_claim_accepted_submission_execution_transition_v1(
    uuid, uuid, uuid, uuid, uuid, uuid, integer
) OWNER TO ple_accepted_submission_execution_worker;

ALTER FUNCTION public.ple_claim_accepted_submission_execution_v1(
    uuid, uuid, integer
) OWNER TO ple_accepted_submission_execution_worker;

ALTER FUNCTION public.ple_claim_exact_accepted_submission_execution_v1(
    uuid, uuid, uuid, uuid, uuid, uuid, integer
) OWNER TO ple_accepted_submission_execution_worker;

REVOKE ALL ON FUNCTION public.ple_claim_accepted_submission_execution_transition_v1(
    uuid, uuid, uuid, uuid, uuid, uuid, integer
) FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_claim_accepted_submission_execution_transition_v1(
    uuid, uuid, uuid, uuid, uuid, uuid, integer
) FROM ple_accepted_submission_execution;

REVOKE ALL ON FUNCTION public.ple_claim_accepted_submission_execution_transition_v1(
    uuid, uuid, uuid, uuid, uuid, uuid, integer
) FROM ple_accepted_submission_execution_fast_path;

REVOKE ALL ON FUNCTION public.ple_claim_accepted_submission_execution_v1(
    uuid, uuid, integer
) FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_claim_accepted_submission_execution_v1(
    uuid, uuid, integer
) FROM ple_accepted_submission_execution_fast_path;

REVOKE ALL ON FUNCTION public.ple_claim_exact_accepted_submission_execution_v1(
    uuid, uuid, uuid, uuid, uuid, uuid, integer
) FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_claim_exact_accepted_submission_execution_v1(
    uuid, uuid, uuid, uuid, uuid, uuid, integer
) FROM ple_accepted_submission_execution;

GRANT EXECUTE ON FUNCTION public.ple_claim_accepted_submission_execution_v1(
    uuid, uuid, integer
) TO ple_accepted_submission_execution;

GRANT EXECUTE ON FUNCTION public.ple_claim_exact_accepted_submission_execution_v1(
    uuid, uuid, uuid, uuid, uuid, uuid, integer
) TO ple_accepted_submission_execution_fast_path;

-- ASVS 8.2.1-8.2.3 and 8.3.1: attest the exact sealed function surface.
DO $claim_catalog$
DECLARE
    v_internal regprocedure :=
        'public.ple_claim_accepted_submission_execution_transition_v1('
        'uuid,uuid,uuid,uuid,uuid,uuid,integer)'::regprocedure;
    v_generic regprocedure :=
        'public.ple_claim_accepted_submission_execution_v1('
        'uuid,uuid,integer)'::regprocedure;
    v_exact regprocedure :=
        'public.ple_claim_exact_accepted_submission_execution_v1('
        'uuid,uuid,uuid,uuid,uuid,uuid,integer)'::regprocedure;
BEGIN
    IF EXISTS (
        SELECT 1
          FROM (
                VALUES
                    (v_internal, false),
                    (v_generic, true),
                    (v_exact, true)
          ) AS expected(procedure_id, security_definer)
          JOIN pg_catalog.pg_proc AS procedure_row
            ON procedure_row.oid = expected.procedure_id
         WHERE procedure_row.proowner
                   <> 'ple_accepted_submission_execution_worker'::regrole
            OR procedure_row.prosecdef IS DISTINCT FROM expected.security_definer
            OR procedure_row.prokind <> 'f'
            OR procedure_row.provolatile <> 'v'
            OR NOT procedure_row.proretset
            OR procedure_row.prorettype <> 'record'::regtype
            OR procedure_row.proconfig IS DISTINCT FROM
               ARRAY['search_path=pg_catalog, public, pg_temp']
    ) THEN
        RAISE EXCEPTION 'accepted-submission claim function catalog is invalid';
    END IF;

    IF (
        SELECT procedure_row.proargnames
          FROM pg_catalog.pg_proc AS procedure_row
         WHERE procedure_row.oid = v_internal
    ) IS DISTINCT FROM ARRAY[
        'p_target_tenant_id',
        'p_target_attempt_id',
        'p_target_submission_id',
        'p_target_worker_job_id',
        'p_lease_token',
        'p_worker_id',
        'p_lease_seconds',
        'tenant_id',
        'worker_job_id',
        'worker_lease_token',
        'submission_id',
        'execution_generation',
        'worker_id'
    ] THEN
        RAISE EXCEPTION 'accepted-submission internal claim columns are invalid';
    END IF;

    IF (
        SELECT procedure_row.proargnames
          FROM pg_catalog.pg_proc AS procedure_row
         WHERE procedure_row.oid = v_generic
    ) IS DISTINCT FROM ARRAY[
        'p_lease_token',
        'p_worker_id',
        'p_lease_seconds',
        'tenant_id',
        'worker_job_id',
        'worker_lease_token',
        'submission_id',
        'execution_generation',
        'worker_id'
    ] THEN
        RAISE EXCEPTION 'accepted-submission generic claim columns are invalid';
    END IF;

    IF (
        SELECT procedure_row.proargnames
          FROM pg_catalog.pg_proc AS procedure_row
         WHERE procedure_row.oid = v_exact
    ) IS DISTINCT FROM ARRAY[
        'p_tenant_id',
        'p_attempt_id',
        'p_submission_id',
        'p_worker_job_id',
        'p_lease_token',
        'p_worker_id',
        'p_lease_seconds',
        'tenant_id',
        'worker_job_id',
        'worker_lease_token',
        'submission_id',
        'execution_generation',
        'worker_id'
    ] THEN
        RAISE EXCEPTION 'accepted-submission exact claim columns are invalid';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM (
                VALUES
                    (
                        v_internal,
                        ARRAY[
                            'i', 'i', 'i', 'i', 'i', 'i', 'i',
                            't', 't', 't', 't', 't', 't'
                        ]::"char"[],
                        ARRAY[
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'integer'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'bigint'::regtype::oid,
                            'uuid'::regtype::oid
                        ]::oid[]
                    ),
                    (
                        v_generic,
                        ARRAY[
                            'i', 'i', 'i',
                            't', 't', 't', 't', 't', 't'
                        ]::"char"[],
                        ARRAY[
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'integer'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'bigint'::regtype::oid,
                            'uuid'::regtype::oid
                        ]::oid[]
                    ),
                    (
                        v_exact,
                        ARRAY[
                            'i', 'i', 'i', 'i', 'i', 'i', 'i',
                            't', 't', 't', 't', 't', 't'
                        ]::"char"[],
                        ARRAY[
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'integer'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'uuid'::regtype::oid,
                            'bigint'::regtype::oid,
                            'uuid'::regtype::oid
                        ]::oid[]
                    )
          ) AS expected(procedure_id, argument_modes, all_argument_types)
          JOIN pg_catalog.pg_proc AS procedure_row
            ON procedure_row.oid = expected.procedure_id
         WHERE procedure_row.proargmodes IS DISTINCT FROM expected.argument_modes
            OR procedure_row.proallargtypes
               IS DISTINCT FROM expected.all_argument_types
    ) THEN
        RAISE EXCEPTION 'accepted-submission claim argument types are invalid';
    END IF;

    IF EXISTS (
        WITH claim_functions AS (
            SELECT unnest(ARRAY[v_internal, v_generic, v_exact])::oid AS procedure_id
        ),
        actual AS (
            SELECT procedure_row.oid AS procedure_id,
                   acl.grantee,
                   acl.privilege_type,
                   acl.is_grantable
              FROM claim_functions
              JOIN pg_catalog.pg_proc AS procedure_row
                ON procedure_row.oid = claim_functions.procedure_id
             CROSS JOIN LATERAL pg_catalog.aclexplode(procedure_row.proacl) AS acl
             WHERE acl.grantee <> procedure_row.proowner
        ),
        expected AS (
            SELECT v_generic::oid AS procedure_id,
                   'ple_accepted_submission_execution'::regrole::oid AS grantee,
                   'EXECUTE'::text AS privilege_type,
                   false AS is_grantable
            UNION ALL
            SELECT v_exact::oid,
                   'ple_accepted_submission_execution_fast_path'::regrole::oid,
                   'EXECUTE'::text,
                   false
        ),
        difference AS (
            (SELECT * FROM actual EXCEPT SELECT * FROM expected)
            UNION ALL
            (SELECT * FROM expected EXCEPT SELECT * FROM actual)
        )
        SELECT 1 FROM difference
    ) THEN
        RAISE EXCEPTION 'accepted-submission claim execute grants are invalid';
    END IF;
END;
$claim_catalog$;

COMMIT;
