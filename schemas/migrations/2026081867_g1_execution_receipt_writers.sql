-- WP-PROF-G1 / G1-W7: final execution receipt writers.
BEGIN;
CREATE OR REPLACE FUNCTION public.ple_accept_automated_submission_v1(
    p_tenant uuid, p_actor uuid, p_expected_course uuid, p_expected_assignment uuid,
    p_attempt uuid, p_idempotency_key text, p_response_canonical_json text, p_job uuid
) RETURNS TABLE(
    result_kind text, accepted_tenant_id uuid, accepted_course_id uuid,
    accepted_assignment_id uuid, accepted_attempt_id uuid, accepted_submission_id uuid,
    accepted_actor_id uuid, accepted_idempotency_key text,
    accepted_request_sha256 character(64), accepted_millis bigint
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v_course uuid;
    v_assignment uuid;
    v_attempt_occurred_at timestamptz;
    v_submission uuid;
    v_occurred_at timestamptz;
    v_auto_submit_at timestamptz;
    v_response_sha256 character(64);
    v_marker jsonb := '{"kind":"acceptedPrivateResponseV1"}'::jsonb;
    v_marker_sha256 character(64);
    v_response jsonb;
    v_attempt_status text;
    v_existing public.submission_idempotency%ROWTYPE;
    v_existing_response text;
    v_existing_sha256 character(64);
    v_job_max_attempts CONSTANT integer := 3;
BEGIN
    IF p_tenant IS NULL OR p_actor IS NULL OR p_expected_course IS NULL
       OR p_expected_assignment IS NULL OR p_attempt IS NULL OR p_job IS NULL
       OR p_idempotency_key IS NULL
       OR octet_length(p_idempotency_key) NOT BETWEEN 1 AND 200
       OR p_idempotency_key ~ '[[:space:][:cntrl:]]'
       OR p_response_canonical_json IS NULL
       OR octet_length(p_response_canonical_json) NOT BETWEEN 1 AND 32768
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
    THEN
        RETURN;
    END IF;
    BEGIN
        v_response := p_response_canonical_json::jsonb;
    EXCEPTION WHEN invalid_text_representation THEN
        RETURN;
    END;
    IF jsonb_typeof(v_response) <> 'object' THEN RETURN; END IF;
    v_response_sha256 := encode(
        pg_catalog.sha256(convert_to(p_response_canonical_json, 'UTF8')), 'hex'
    );
    v_marker_sha256 := encode(pg_catalog.sha256(convert_to(v_marker::text, 'UTF8')), 'hex');

    SELECT assignment.course_id, assignment.assignment_id, attempt.attempt_status,
           attempt.occurred_at
      INTO v_course, v_assignment, v_attempt_status, v_attempt_occurred_at
      FROM public.question_attempt AS attempt
      JOIN public.assignment_run AS run
        ON run.tenant_id = attempt.tenant_id AND run.run_id = attempt.run_id
      JOIN public.enrollment AS enrollment
        ON enrollment.tenant_id = run.tenant_id AND enrollment.enrollment_id = run.enrollment_id
      JOIN public.assignment AS assignment
        ON assignment.tenant_id = enrollment.tenant_id
       AND assignment.assignment_id = enrollment.assignment_id
      JOIN public.course_member AS member
        ON member.tenant_id = assignment.tenant_id AND member.course_id = assignment.course_id
       AND member.user_id = p_actor AND member.role = 'student' AND member.status = 'active'
     WHERE attempt.tenant_id = p_tenant AND attempt.attempt_id = p_attempt
       AND enrollment.user_id = p_actor AND attempt.attempt_status IN ('in_progress', 'submitted')
     FOR UPDATE OF attempt;
    IF NOT FOUND OR v_course IS DISTINCT FROM p_expected_course
       OR v_assignment IS DISTINCT FROM p_expected_assignment
       OR NOT public.ple_course_records_accessible(p_tenant, v_course)
    THEN
        RETURN;
    END IF;

    SELECT * INTO v_existing FROM public.submission_idempotency
     WHERE tenant_id = p_tenant AND attempt_id = p_attempt FOR UPDATE;
    IF FOUND THEN
        IF v_existing.request_contract_version <> 2
           OR v_existing.accepted_actor_id IS DISTINCT FROM p_actor
           OR v_existing.course_id IS DISTINCT FROM v_course
           OR v_existing.idempotency_key IS DISTINCT FROM p_idempotency_key
           OR v_existing.request_sha256 IS DISTINCT FROM v_response_sha256
           OR v_existing.payload IS DISTINCT FROM v_marker
           OR v_existing.payload_sha256 IS DISTINCT FROM v_marker_sha256
           OR v_existing.submission_id IS NULL OR v_existing.submission_occurred_at IS NULL
        THEN
            RETURN QUERY SELECT 'conflict', NULL::uuid, NULL::uuid, NULL::uuid,
                NULL::uuid, NULL::uuid, NULL::uuid, NULL::text, NULL::character(64), NULL::bigint;
            RETURN;
        END IF;
        SELECT response_canonical_json, response_sha256
          INTO v_existing_response, v_existing_sha256
          FROM public.accepted_submission_private_response
         WHERE tenant_id = p_tenant AND course_id = v_course AND attempt_id = p_attempt
           AND submission_id = v_existing.submission_id
           AND submission_occurred_at = v_existing.submission_occurred_at;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'accepted submission private response is unavailable' USING ERRCODE = '55000';
        END IF;
        IF v_existing_response IS NOT DISTINCT FROM p_response_canonical_json
           AND v_existing_sha256 IS NOT DISTINCT FROM v_response_sha256
        THEN
            RETURN QUERY SELECT 'replayed', v_existing.tenant_id, v_existing.course_id,
                v_assignment, v_existing.attempt_id, v_existing.submission_id,
                v_existing.accepted_actor_id, v_existing.idempotency_key,
                v_existing.request_sha256,
                floor(extract(epoch FROM v_existing.submitted_at) * 1000)::bigint;
        ELSE
            RETURN QUERY SELECT 'conflict', NULL::uuid, NULL::uuid, NULL::uuid,
                NULL::uuid, NULL::uuid, NULL::uuid, NULL::text, NULL::character(64), NULL::bigint;
        END IF;
        RETURN;
    END IF;

    IF v_attempt_status <> 'in_progress' THEN
        RETURN QUERY SELECT 'conflict', NULL::uuid, NULL::uuid, NULL::uuid,
            NULL::uuid, NULL::uuid, NULL::uuid, NULL::text, NULL::character(64), NULL::bigint;
        RETURN;
    END IF;

    -- ASVS 2.2.1-2.2.3, 2.3.1-2.3.4: a first-effect acceptance evaluates the
    -- exact sealed policy witness while its issued attempt is locked. Every
    -- path that replaces this timing witness either creates that attempt in
    -- the same transaction or takes the same attempt lock before replacing
    -- the pointer; this SELECT consequently needs no mutation privilege on
    -- the timing relations. The one transaction timestamp is both the durable
    -- accepted time and the authority for the inclusive cutoff comparison.
    v_submission := p_attempt;
    v_occurred_at := transaction_timestamp();
    SELECT receipt.auto_submit_at
      INTO v_auto_submit_at
      FROM public.attempt_effective_policy_current AS current_effect
      JOIN public.attempt_effective_policy_receipt AS receipt
        ON receipt.tenant_id = current_effect.tenant_id
       AND receipt.attempt_id = current_effect.attempt_id
       AND receipt.receipt_generation = current_effect.receipt_generation
       AND receipt.course_id = current_effect.course_id
       AND receipt.assignment_id = current_effect.assignment_id
     WHERE current_effect.tenant_id = p_tenant
       AND current_effect.attempt_id = p_attempt
       AND current_effect.attempt_occurred_at = v_attempt_occurred_at
       AND current_effect.course_id = v_course
       AND current_effect.assignment_id = v_assignment
       AND receipt.attempt_occurred_at = v_attempt_occurred_at
       AND receipt.sealed_at IS NOT NULL;
    IF NOT FOUND THEN
        RETURN QUERY SELECT 'unavailable', NULL::uuid, NULL::uuid, NULL::uuid,
            NULL::uuid, NULL::uuid, NULL::uuid, NULL::text, NULL::character(64), NULL::bigint;
        RETURN;
    END IF;
    IF v_auto_submit_at IS NOT NULL AND v_occurred_at > v_auto_submit_at THEN
        RETURN QUERY SELECT 'timed_out', NULL::uuid, NULL::uuid, NULL::uuid,
            NULL::uuid, NULL::uuid, NULL::uuid, NULL::text, NULL::character(64), NULL::bigint;
        RETURN;
    END IF;

    INSERT INTO public.submission
        (tenant_id, submission_id, attempt_id, idempotency_key, occurred_at,
         payload, payload_sha256, course_id)
    VALUES (p_tenant, v_submission, p_attempt, p_idempotency_key, v_occurred_at,
            v_marker, v_marker_sha256, v_course);
    INSERT INTO public.submission_idempotency
        (tenant_id, attempt_id, idempotency_key, request_contract_version,
         request_sha256, submitted_at, payload, payload_sha256, course_id,
         submission_id, submission_occurred_at, accepted_actor_id)
    VALUES (p_tenant, p_attempt, p_idempotency_key, 2, v_response_sha256,
            v_occurred_at, v_marker, v_marker_sha256, v_course, v_submission,
            v_occurred_at, p_actor);
    INSERT INTO public.accepted_submission_private_response
        (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at,
         response_canonical_json, response_sha256)
    VALUES (p_tenant, v_course, p_attempt, v_submission, v_occurred_at,
            p_response_canonical_json, v_response_sha256);
    UPDATE public.question_attempt SET attempt_status = 'submitted', submitted_at = v_occurred_at
     WHERE tenant_id = p_tenant AND attempt_id = p_attempt AND attempt_status = 'in_progress';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'accepted-submission attempt state changed under lock' USING ERRCODE = '55000';
    END IF;
    INSERT INTO public.submission_evaluation
        (tenant_id, attempt_id, submission_id, grading_status, payload, payload_sha256, course_id)
    VALUES (p_tenant, p_attempt, v_submission, 'automated_pending', '{}'::jsonb,
            encode(pg_catalog.sha256(convert_to('{}', 'UTF8')), 'hex'), v_course);
    INSERT INTO public.worker_job (job_id, tenant_id, payload, state, max_attempts)
    VALUES (p_job, p_tenant, jsonb_build_object(
            'kind', 'gradeAcceptedSubmission', 'attempt', p_attempt::text,
            'submission', v_submission::text, 'execution_generation', 1
        ), 'ready', v_job_max_attempts);
    INSERT INTO public.grading_execution
        (tenant_id, attempt_id, submission_id, submission_occurred_at, course_id,
         execution_generation, state, current_job_id)
    VALUES (p_tenant, p_attempt, v_submission, v_occurred_at, v_course, 1, 'ready', p_job);
    INSERT INTO public.grading_execution_receipt
        (tenant_id, receipt_id, attempt_id, submission_id, submission_occurred_at,
         course_id, execution_generation, resulting_state, safe_category, actor_id, occurred_at)
    VALUES (p_tenant, p_job, p_attempt, v_submission, v_occurred_at,
            v_course, 1, 'ready', 'accepted_submission', p_actor, v_occurred_at);
    RETURN QUERY SELECT 'accepted', p_tenant, v_course, v_assignment, p_attempt,
        v_submission, p_actor, p_idempotency_key, v_response_sha256,
        floor(extract(epoch FROM v_occurred_at) * 1000)::bigint;
END $$;
CREATE OR REPLACE FUNCTION public.ple_claim_accepted_submission_execution_transition_v1(
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
            safe_category,
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
            'retry_exhausted',
            p_worker_id,
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
        safe_category,
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
        'worker_claim',
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

CREATE OR REPLACE FUNCTION public.ple_claim_accepted_submission_execution_v1(
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

CREATE OR REPLACE FUNCTION public.ple_claim_exact_accepted_submission_execution_v1(
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
CREATE OR REPLACE FUNCTION public.ple_fail_accepted_submission_execution_v1(
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
    v_safe_category text;
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
        v_safe_category := v_reason;

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
            safe_category,
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
            v_safe_category,
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
        safe_category,
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
        'dependency_retry',
        p_worker_id
    );
    RETURN QUERY SELECT 'rescheduled', 'retry_wait', 'automated_pending';
END;
$$;
ALTER FUNCTION public.ple_accept_automated_submission_v1(uuid, uuid, uuid, uuid, uuid, text, text, uuid) OWNER TO ple_automated_grading_broker;
ALTER FUNCTION public.ple_claim_accepted_submission_execution_transition_v1(uuid, uuid, uuid, uuid, uuid, uuid, integer) OWNER TO ple_accepted_submission_execution_worker;
ALTER FUNCTION public.ple_fail_accepted_submission_execution_v1(uuid, uuid, uuid, uuid, bigint, uuid, text, text) OWNER TO ple_accepted_submission_execution_worker;
-- Normalize ambient/default grants, then attest every acceptance and claim path.
DO $$
DECLARE
    v_function regprocedure;
    v_signature text;
    v_grantee record;
    v_owner regrole;
    v_security_definer boolean;
    v_expected oid[];
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'public.ple_accept_automated_submission_v1(uuid,uuid,uuid,uuid,uuid,text,text,uuid)'::regprocedure,
        'public.ple_claim_accepted_submission_execution_transition_v1(uuid,uuid,uuid,uuid,uuid,uuid,integer)'::regprocedure,
        'public.ple_claim_accepted_submission_execution_v1(uuid,uuid,integer)'::regprocedure,
        'public.ple_claim_exact_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,uuid,uuid,integer)'::regprocedure,
        'public.ple_fail_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,bigint,uuid,text,text)'::regprocedure
    ] LOOP
        SELECT format('%I.%I(%s)', namespace_row.nspname, procedure_row.proname,
                      pg_get_function_identity_arguments(procedure_row.oid))
          INTO v_signature
          FROM pg_catalog.pg_proc AS procedure_row
          JOIN pg_catalog.pg_namespace AS namespace_row
            ON namespace_row.oid = procedure_row.pronamespace
         WHERE procedure_row.oid = v_function;
        IF v_function = 'public.ple_accept_automated_submission_v1(uuid,uuid,uuid,uuid,uuid,text,text,uuid)'::regprocedure THEN
            v_owner := 'ple_automated_grading_broker'; v_security_definer := true;
            v_expected := ARRAY['ple_app'::regrole::oid];
        ELSIF v_function = 'public.ple_claim_accepted_submission_execution_transition_v1(uuid,uuid,uuid,uuid,uuid,uuid,integer)'::regprocedure THEN
            v_owner := 'ple_accepted_submission_execution_worker'; v_security_definer := false;
            v_expected := ARRAY[]::oid[];
        ELSIF v_function = 'public.ple_claim_accepted_submission_execution_v1(uuid,uuid,integer)'::regprocedure THEN
            v_owner := 'ple_accepted_submission_execution_worker'; v_security_definer := true;
            v_expected := ARRAY['ple_accepted_submission_execution'::regrole::oid];
        ELSIF v_function = 'public.ple_claim_exact_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,uuid,uuid,integer)'::regprocedure THEN
            v_owner := 'ple_accepted_submission_execution_worker'; v_security_definer := true;
            v_expected := ARRAY['ple_accepted_submission_execution_fast_path'::regrole::oid];
        ELSE
            v_owner := 'ple_accepted_submission_execution_worker'; v_security_definer := true;
            v_expected := ARRAY['ple_accepted_submission_execution'::regrole::oid,
                                'ple_accepted_submission_execution_fast_path'::regrole::oid];
        END IF;
        FOR v_grantee IN
            SELECT DISTINCT acl.grantee, role_row.rolname
              FROM pg_catalog.pg_proc AS procedure_row
              CROSS JOIN LATERAL pg_catalog.aclexplode(COALESCE(
                  procedure_row.proacl, pg_catalog.acldefault('f', procedure_row.proowner))) AS acl
              LEFT JOIN pg_catalog.pg_roles AS role_row ON role_row.oid = acl.grantee
             WHERE procedure_row.oid = v_function AND acl.grantee <> procedure_row.proowner
        LOOP
            IF v_grantee.grantee = 0 THEN
                EXECUTE format('REVOKE ALL PRIVILEGES ON FUNCTION %s FROM PUBLIC', v_signature);
            ELSE
                EXECUTE format('REVOKE ALL PRIVILEGES ON FUNCTION %s FROM %I', v_signature, v_grantee.rolname);
            END IF;
        END LOOP;
        FOR v_grantee IN SELECT role_row.rolname FROM unnest(v_expected) AS expected(grantee)
            JOIN pg_catalog.pg_roles AS role_row ON role_row.oid = expected.grantee LOOP
            EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO %I', v_signature, v_grantee.rolname);
        END LOOP;
        IF EXISTS (SELECT 1 FROM pg_catalog.pg_proc AS procedure_row
                    WHERE procedure_row.oid = v_function AND (
                        procedure_row.proowner <> v_owner OR procedure_row.prosecdef IS DISTINCT FROM v_security_definer
                        OR procedure_row.proconfig IS DISTINCT FROM ARRAY['search_path=pg_catalog, public, pg_temp']))
           OR EXISTS (WITH expected AS (SELECT unnest(v_expected) AS grantee, 'EXECUTE'::text AS privilege_type, false AS is_grantable),
                           actual AS (SELECT acl.grantee, acl.privilege_type, acl.is_grantable FROM pg_catalog.pg_proc AS procedure_row
                                      CROSS JOIN LATERAL pg_catalog.aclexplode(COALESCE(procedure_row.proacl, pg_catalog.acldefault('f', procedure_row.proowner))) AS acl
                                     WHERE procedure_row.oid = v_function AND acl.grantee <> procedure_row.proowner)
                      SELECT 1 FROM ((SELECT * FROM expected EXCEPT SELECT * FROM actual) UNION ALL (SELECT * FROM actual EXCEPT SELECT * FROM expected)) AS acl_difference) THEN
            RAISE EXCEPTION 'G1 execution receipt writer authority is unsafe';
        END IF;
    END LOOP;
END;
$$;
COMMIT;
