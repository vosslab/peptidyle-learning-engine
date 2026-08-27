-- WP-PROF-G1: immutable accepted submissions and automated-grading recovery.
-- Execution, evaluation, and Instructor recovery are independent projections;
-- existing 1830/1831 capabilities remain the only score publishers.

BEGIN;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'ple_automated_grading_broker') THEN
        CREATE ROLE ple_automated_grading_broker
            NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid = 'ple_automated_grading_broker'::regrole
            OR membership.member = 'ple_automated_grading_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'ple_automated_grading_broker must not have role memberships';
    END IF;
END $$;
ALTER ROLE ple_automated_grading_broker
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
REVOKE ALL ON SCHEMA public FROM ple_automated_grading_broker;
GRANT USAGE ON SCHEMA public TO ple_automated_grading_broker;

-- One idempotency record names one accepted immutable response.  The
-- acceptance capability writes this binding and its execution atomically.
ALTER TABLE public.submission_idempotency
    ADD COLUMN submission_id uuid,
    ADD COLUMN submission_occurred_at timestamp with time zone,
    ADD CONSTRAINT submission_idempotency_submission_shape_check CHECK (
        (submission_id IS NULL) = (submission_occurred_at IS NULL)
    );
-- The accepted-input identity is course-bearing from its first use.  A
-- same-tenant idempotency record cannot name a submission in another course.
CREATE UNIQUE INDEX submission_course_attempt_identity_unique
    ON public.submission (tenant_id, course_id, attempt_id, submission_id, occurred_at);
CREATE UNIQUE INDEX submission_idempotency_course_submission_identity_unique
    ON public.submission_idempotency
        (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at);
ALTER TABLE public.submission_idempotency
    ADD CONSTRAINT submission_idempotency_submission_fk
    FOREIGN KEY (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at)
    REFERENCES public.submission (tenant_id, course_id, attempt_id, submission_id, occurred_at)
    ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE public.submission_evaluation
    DROP CONSTRAINT submission_evaluation_grading_status_check,
    DROP CONSTRAINT submission_evaluation_result_shape_check;
ALTER TABLE public.submission_evaluation
    ADD CONSTRAINT submission_evaluation_grading_status_check CHECK (
        grading_status = ANY (
            ARRAY['automated_pending', 'automated_exception', 'needs_manual_grading', 'graded', 'exempt']
        )
    ),
    ADD CONSTRAINT submission_evaluation_result_shape_check CHECK (
        (
            grading_status = ANY ('{automated_pending,automated_exception,needs_manual_grading}')
            AND credit_fraction IS NULL AND correct IS NULL
        ) OR (
            grading_status = ANY ('{graded,exempt}')
            AND credit_fraction IS NOT NULL AND correct IS NOT NULL
        )
    );

ALTER TABLE public.worker_job
    ADD CONSTRAINT worker_job_tenant_job_unique UNIQUE (tenant_id, job_id);

CREATE TABLE public.grading_execution (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    submission_id uuid NOT NULL,
    submission_occurred_at timestamp with time zone NOT NULL,
    course_id uuid NOT NULL,
    execution_generation bigint NOT NULL,
    state text NOT NULL,
    current_job_id uuid NOT NULL,
    retry_count integer NOT NULL DEFAULT 0,
    updated_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    CONSTRAINT grading_execution_pkey PRIMARY KEY (tenant_id, attempt_id),
    CONSTRAINT grading_execution_submission_unique
        UNIQUE (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at),
    CONSTRAINT grading_execution_generation_check CHECK (execution_generation > 0),
    CONSTRAINT grading_execution_retry_check CHECK (retry_count BETWEEN 0 AND 20),
    CONSTRAINT grading_execution_state_check CHECK (
        state = ANY ('{ready,running,completed,exception,retry_wait,superseded}')
    ),
    CONSTRAINT grading_execution_submission_fk
        FOREIGN KEY (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at)
        REFERENCES public.submission (tenant_id, course_id, attempt_id, submission_id, occurred_at)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT grading_execution_idempotency_fk
        FOREIGN KEY (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at)
        REFERENCES public.submission_idempotency
            (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT grading_execution_course_fk FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course(tenant_id, course_id),
    CONSTRAINT grading_execution_job_fk FOREIGN KEY (tenant_id, current_job_id)
        REFERENCES public.worker_job(tenant_id, job_id)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX grading_execution_course_state_idx
    ON public.grading_execution (tenant_id, course_id, state, updated_at, attempt_id);
CREATE INDEX grading_execution_job_idx ON public.grading_execution (tenant_id, current_job_id);
ALTER TABLE public.grading_execution ENABLE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.grading_execution FORCE ROW LEVEL SECURITY;

CREATE TABLE public.grading_execution_receipt (
    tenant_id uuid NOT NULL,
    receipt_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    submission_id uuid NOT NULL,
    submission_occurred_at timestamp with time zone NOT NULL,
    course_id uuid NOT NULL,
    execution_generation bigint NOT NULL,
    resulting_state text NOT NULL,
    worker_id uuid,
    occurred_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    CONSTRAINT grading_execution_receipt_pkey PRIMARY KEY (tenant_id, receipt_id),
    CONSTRAINT grading_execution_receipt_generation_check CHECK (execution_generation > 0),
    CONSTRAINT grading_execution_receipt_state_check CHECK (
        resulting_state = ANY ('{ready,running,completed,exception,retry_wait,superseded}')
    ),
    CONSTRAINT grading_execution_receipt_execution_fk
        FOREIGN KEY (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at)
        REFERENCES public.grading_execution
            (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at)
        ON DELETE CASCADE,
    CONSTRAINT grading_execution_receipt_course_fk FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course(tenant_id, course_id)
);
CREATE INDEX grading_execution_receipt_course_time_idx
    ON public.grading_execution_receipt (tenant_id, course_id, occurred_at, receipt_id);
ALTER TABLE public.grading_execution_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.grading_execution_receipt FORCE ROW LEVEL SECURITY;

CREATE TABLE public.grading_operation (
    tenant_id uuid NOT NULL,
    grading_operation_id bigint GENERATED ALWAYS AS IDENTITY,
    attempt_id uuid,
    submission_id uuid,
    submission_occurred_at timestamp with time zone,
    assignment_id uuid NOT NULL,
    course_id uuid NOT NULL,
    target_kind text NOT NULL,
    requested_scoring_generation bigint,
    reason text NOT NULL,
    state text NOT NULL,
    revision bigint NOT NULL DEFAULT 1,
    next_action text,
    created_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    updated_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    CONSTRAINT grading_operation_pkey PRIMARY KEY (tenant_id, grading_operation_id),
    CONSTRAINT grading_operation_course_identity_unique
        UNIQUE (tenant_id, course_id, grading_operation_id),
    CONSTRAINT grading_operation_reference_check CHECK (grading_operation_id BETWEEN 1 AND 2147483647),
    CONSTRAINT grading_operation_target_check CHECK (
        target_kind = ANY ('{submission,assignment_scoring_generation}')
    ),
    CONSTRAINT grading_operation_reason_check CHECK (
        reason = ANY ('{grader_contract_failure,grader_execution_failure,issued_evidence_integrity,retry_exhausted,scoring_recalculation_failed}')
    ),
    CONSTRAINT grading_operation_state_check CHECK (
        state = ANY ('{actionable,action_in_progress,completed,repair_required,failed,superseded}')
    ),
    CONSTRAINT grading_operation_action_check CHECK (
        next_action IS NULL OR next_action = ANY ('{retry,recalculate}')
    ),
    CONSTRAINT grading_operation_revision_check CHECK (revision > 0),
    CONSTRAINT grading_operation_target_shape_check CHECK (
        (target_kind = 'submission' AND attempt_id IS NOT NULL AND submission_id IS NOT NULL
            AND submission_occurred_at IS NOT NULL AND requested_scoring_generation IS NULL)
        OR (target_kind = 'assignment_scoring_generation' AND attempt_id IS NULL AND submission_id IS NULL
            AND submission_occurred_at IS NULL AND requested_scoring_generation > 0)
    ),
    CONSTRAINT grading_operation_attempt_fk FOREIGN KEY (tenant_id, attempt_id)
        REFERENCES public.submission_idempotency(tenant_id, attempt_id) ON DELETE CASCADE,
    CONSTRAINT grading_operation_submission_fk
        FOREIGN KEY (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at)
        REFERENCES public.grading_execution
            (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at)
        ON DELETE CASCADE,
    CONSTRAINT grading_operation_assignment_fk FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment(tenant_id, course_id, assignment_id),
    CONSTRAINT grading_operation_course_fk FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course(tenant_id, course_id)
);
CREATE UNIQUE INDEX grading_operation_submission_thread_unique
    ON public.grading_operation (tenant_id, assignment_id, attempt_id, submission_id)
    WHERE target_kind = 'submission';
CREATE UNIQUE INDEX grading_operation_scoring_thread_unique
    ON public.grading_operation (tenant_id, assignment_id, requested_scoring_generation)
    WHERE target_kind = 'assignment_scoring_generation';
CREATE INDEX grading_operation_course_state_idx
    ON public.grading_operation (tenant_id, course_id, assignment_id, state, updated_at, grading_operation_id);
ALTER TABLE public.grading_operation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.grading_operation FORCE ROW LEVEL SECURITY;

CREATE TABLE public.grading_operation_receipt (
    tenant_id uuid NOT NULL,
    action_id uuid NOT NULL,
    grading_operation_id bigint NOT NULL,
    course_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    action_kind text NOT NULL,
    expected_revision bigint NOT NULL,
    resulting_revision bigint NOT NULL,
    request_sha256 character(64) NOT NULL,
    occurred_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    CONSTRAINT grading_operation_receipt_pkey PRIMARY KEY (tenant_id, action_id),
    CONSTRAINT grading_operation_receipt_action_check CHECK (action_kind = ANY ('{retry,recalculate}')),
    CONSTRAINT grading_operation_receipt_revision_check CHECK (
        expected_revision > 0 AND resulting_revision = expected_revision + 1
    ),
    CONSTRAINT grading_operation_receipt_request_sha256_check CHECK (request_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT grading_operation_receipt_operation_fk
        FOREIGN KEY (tenant_id, course_id, grading_operation_id)
        REFERENCES public.grading_operation(tenant_id, course_id, grading_operation_id)
        ON DELETE CASCADE,
    CONSTRAINT grading_operation_receipt_course_fk FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course(tenant_id, course_id)
);
CREATE INDEX grading_operation_receipt_course_time_idx
    ON public.grading_operation_receipt (tenant_id, course_id, occurred_at, action_id);
ALTER TABLE public.grading_operation_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.grading_operation_receipt FORCE ROW LEVEL SECURITY;

-- The acceptance transaction inserts this job and execution together.  A
-- deferred fence prevents a queue row of another kind from being associated.
CREATE FUNCTION public.ple_guard_grading_execution_job() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM public.worker_job AS job
         WHERE job.tenant_id = NEW.tenant_id AND job.job_id = NEW.current_job_id
           AND job.payload = jsonb_build_object(
               'kind', 'gradeAcceptedSubmission', 'attempt', NEW.attempt_id::text,
               'submission', NEW.submission_id::text,
               'execution_generation', NEW.execution_generation
           )
    ) THEN
        RAISE EXCEPTION 'grading execution must own its exact accepted-submission job'
            USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END $$;
CREATE CONSTRAINT TRIGGER grading_execution_exact_job_fence
    AFTER INSERT OR UPDATE OF current_job_id, execution_generation, submission_id, attempt_id
    ON public.grading_execution DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_grading_execution_job();

-- ASVS 2.3.1, 2.3.3, and 8.2.2: the converse deferred fence closes the
-- direct queue-insert path.  Every accepted-submission job must have the one
-- exact execution identity by commit; other legacy queue kinds keep their
-- existing lifecycle.
CREATE FUNCTION public.ple_guard_accepted_submission_job_execution() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF NEW.payload ->> 'kind' = 'gradeAcceptedSubmission' AND NOT EXISTS (
        SELECT 1
          FROM public.grading_execution AS execution
         WHERE execution.tenant_id = NEW.tenant_id
           AND execution.current_job_id = NEW.job_id
           AND NEW.payload = jsonb_build_object(
               'kind', 'gradeAcceptedSubmission',
               'attempt', execution.attempt_id::text,
               'submission', execution.submission_id::text,
               'execution_generation', execution.execution_generation
           )
    ) THEN
        RAISE EXCEPTION 'accepted-submission job must own its exact grading execution'
            USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END $$;
CREATE CONSTRAINT TRIGGER worker_job_exact_grading_execution_fence
    AFTER INSERT OR UPDATE ON public.worker_job DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_accepted_submission_job_execution();

CREATE FUNCTION public.ple_forbid_grading_receipt_mutation() RETURNS trigger
LANGUAGE plpgsql
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' AND current_user = 'ple_retention_broker' THEN RETURN OLD; END IF;
    RAISE EXCEPTION 'automated-grading receipts are append-only and retention-deleted'
        USING ERRCODE = '42501';
END $$;
CREATE TRIGGER grading_execution_receipt_append_only
    BEFORE UPDATE OR DELETE ON public.grading_execution_receipt
    FOR EACH ROW EXECUTE FUNCTION public.ple_forbid_grading_receipt_mutation();
CREATE TRIGGER grading_operation_receipt_append_only
    BEFORE UPDATE OR DELETE ON public.grading_operation_receipt
    FOR EACH ROW EXECUTE FUNCTION public.ple_forbid_grading_receipt_mutation();
ALTER FUNCTION public.ple_guard_grading_execution_job() OWNER TO ple_automated_grading_broker;
ALTER FUNCTION public.ple_guard_accepted_submission_job_execution()
    OWNER TO ple_automated_grading_broker;
ALTER FUNCTION public.ple_forbid_grading_receipt_mutation() OWNER TO ple_automated_grading_broker;
REVOKE ALL ON FUNCTION public.ple_guard_grading_execution_job(),
    public.ple_guard_accepted_submission_job_execution(),
    public.ple_forbid_grading_receipt_mutation() FROM PUBLIC, ple_app;

-- Replace the prior exhaustive queue type gate with all current kinds plus
-- the identifier-only accepted-submission work item.
ALTER TABLE public.worker_job DROP CONSTRAINT worker_job_payload_kind_check;
ALTER TABLE public.worker_job ADD CONSTRAINT worker_job_payload_kind_check CHECK (
    CASE payload ->> 'kind'
        WHEN 'render' THEN payload ?& ARRAY['kind','reference','seed']
            AND payload - ARRAY['kind','reference','seed'] = '{}'::jsonb
            AND jsonb_typeof(payload -> 'reference') = 'object'
            AND (payload -> 'reference') ?& ARRAY['problem','version']
            AND (payload -> 'reference') - ARRAY['problem','version'] = '{}'::jsonb
            AND (payload #>> '{reference,problem}') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            AND (payload #>> '{reference,version}') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            AND jsonb_typeof(payload -> 'seed') = 'number' AND (payload ->> 'seed') ~ '^(0|[1-9][0-9]{0,19})$'
            AND (payload ->> 'seed')::numeric <= 18446744073709551615
        WHEN 'export' THEN payload ?& ARRAY['kind','delivery_object']
            AND payload - ARRAY['kind','delivery_object'] = '{}'::jsonb
            AND (payload ->> 'delivery_object') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        WHEN 'import' THEN payload ?& ARRAY['kind','source_object']
            AND payload - ARRAY['kind','source_object'] = '{}'::jsonb
            AND (payload ->> 'source_object') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        WHEN 'qtiImport' THEN payload ?& ARRAY['kind','workspace','import','source_object']
            AND payload - ARRAY['kind','workspace','import','source_object'] = '{}'::jsonb
            AND (payload ->> 'workspace') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            AND (payload ->> 'import') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            AND (payload ->> 'source_object') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        WHEN 'retention' THEN payload ?& ARRAY['kind','course','stage','generation']
            AND payload - ARRAY['kind','course','stage','generation'] = '{}'::jsonb
            AND (payload ->> 'course') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            AND (payload ->> 'stage') = ANY (ARRAY['notify','archiveStudentRecords','deleteStudentRecords'])
            AND jsonb_typeof(payload -> 'generation') = 'number' AND (payload ->> 'generation') ~ '^[1-9][0-9]{0,18}$'
            AND (payload ->> 'generation')::numeric <= 9223372036854775807
        WHEN 'recalculateAssignment' THEN payload ?& ARRAY['kind','assignment','generation']
            AND payload - ARRAY['kind','assignment','generation'] = '{}'::jsonb
            AND (payload ->> 'assignment') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            AND jsonb_typeof(payload -> 'generation') = 'number' AND (payload ->> 'generation') ~ '^[1-9][0-9]{0,18}$'
            AND (payload ->> 'generation')::numeric <= 9223372036854775807
        WHEN 'recalculateCourseItemAnalysis' THEN payload ?& ARRAY['kind','assignment','generation']
            AND payload - ARRAY['kind','assignment','generation'] = '{}'::jsonb
            AND (payload ->> 'assignment') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            AND jsonb_typeof(payload -> 'generation') = 'number' AND (payload ->> 'generation') ~ '^[1-9][0-9]{0,18}$'
            AND (payload ->> 'generation')::numeric <= 9223372036854775807
        WHEN 'autoSubmitAttempt' THEN payload ?& ARRAY['kind','attempt','timing_generation']
            AND payload - ARRAY['kind','attempt','timing_generation'] = '{}'::jsonb
            AND (payload ->> 'attempt') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            AND jsonb_typeof(payload -> 'timing_generation') = 'number' AND (payload ->> 'timing_generation') ~ '^[1-9][0-9]{0,18}$'
            AND (payload ->> 'timing_generation')::numeric <= 9223372036854775807
        WHEN 'publishPublicAssets' THEN payload ?& ARRAY['kind','reference']
            AND payload - ARRAY['kind','reference'] = '{}'::jsonb
            AND jsonb_typeof(payload -> 'reference') = 'object'
            AND (payload -> 'reference') ?& ARRAY['problem','version']
            AND (payload -> 'reference') - ARRAY['problem','version'] = '{}'::jsonb
            AND (payload #>> '{reference,problem}') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            AND (payload #>> '{reference,version}') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        WHEN 'gradeAcceptedSubmission' THEN payload ?& ARRAY['kind','attempt','submission','execution_generation']
            AND payload - ARRAY['kind','attempt','submission','execution_generation'] = '{}'::jsonb
            AND (payload ->> 'attempt') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            AND (payload ->> 'submission') ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            AND jsonb_typeof(payload -> 'execution_generation') = 'number'
            AND (payload ->> 'execution_generation') ~ '^[1-9][0-9]{0,18}$'
            AND (payload ->> 'execution_generation')::numeric <= 9223372036854775807
        ELSE false
    END
);

-- The legacy API may enqueue its existing public kinds, but never fabricate
-- private accepted-submission work outside the atomic acceptance transaction.
ALTER POLICY worker_job_tenant_insert ON public.worker_job
    WITH CHECK (
        tenant_id = public.ple_current_tenant()
        AND state = 'ready'
        AND payload ->> 'kind' <> 'gradeAcceptedSubmission'
    );

-- The closed payload admits the new kind for the atomic acceptance broker,
-- but generic API-executable queue functions never claim, count, complete, or
-- fail it. WP-PROF-G1 W4 owns the later worker-only exact-claim and outcome
-- capability.
CREATE OR REPLACE FUNCTION public.ple_claim_worker_job(
    p_token uuid, p_lease_seconds integer, p_kinds text[]
) RETURNS TABLE(job_id uuid, tenant_id uuid, payload jsonb, lease_token uuid, attempt_count integer)
LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public' AS $$
BEGIN
    IF p_token IS NULL OR p_lease_seconds NOT BETWEEN 1 AND 900 OR p_kinds IS NULL
       OR cardinality(p_kinds) NOT BETWEEN 1 AND 10 OR NOT (p_kinds <@ ARRAY[
            'recalculateAssignment','recalculateCourseItemAnalysis',
            'autoSubmitAttempt','retention','render','export','import','qtiImport','publishPublicAssets'
       ]::text[]) THEN
        RAISE EXCEPTION 'invalid queue claim arguments' USING ERRCODE = '22023';
    END IF;
    UPDATE public.worker_job AS expired SET state = 'dead', lease_token = NULL, lease_expires_at = NULL,
        last_error = 'timed_out', completed_at = transaction_timestamp()
     WHERE expired.state = 'leased' AND expired.payload ->> 'kind' = ANY(p_kinds)
       AND expired.lease_expires_at <= transaction_timestamp() AND expired.attempt_count >= expired.max_attempts;
    RETURN QUERY WITH candidate AS (
        SELECT queued.job_id FROM public.worker_job AS queued
         WHERE queued.payload ->> 'kind' = ANY(p_kinds)
           AND ((queued.state = 'ready' AND queued.available_at <= transaction_timestamp())
             OR (queued.state = 'leased' AND queued.lease_expires_at <= transaction_timestamp()
                 AND queued.attempt_count < queued.max_attempts))
         ORDER BY CASE WHEN queued.payload ->> 'kind' = 'recalculateCourseItemAnalysis' THEN 1 ELSE 0 END,
                  queued.available_at, queued.job_id FOR UPDATE SKIP LOCKED LIMIT 1
    ), claimed AS (
        UPDATE public.worker_job AS queued SET state = 'leased', lease_token = p_token,
            lease_expires_at = transaction_timestamp() + make_interval(secs => p_lease_seconds),
            attempt_count = queued.attempt_count + 1, last_error = NULL, completed_at = NULL
          FROM candidate WHERE queued.job_id = candidate.job_id
        RETURNING queued.job_id, queued.tenant_id, queued.payload, queued.lease_token, queued.attempt_count
    ) SELECT * FROM claimed;
END $$;
CREATE OR REPLACE FUNCTION public.ple_ready_worker_queue_depth(p_kinds text[]) RETURNS bigint
LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public' AS $$
BEGIN
    IF p_kinds IS NULL OR cardinality(p_kinds) NOT BETWEEN 1 AND 10 OR NOT (p_kinds <@ ARRAY[
            'recalculateAssignment','recalculateCourseItemAnalysis',
            'autoSubmitAttempt','retention','render','export','import','qtiImport','publishPublicAssets'
       ]::text[]) THEN
        RAISE EXCEPTION 'invalid queue depth arguments' USING ERRCODE = '22023';
    END IF;
    RETURN (SELECT count(*)::bigint FROM public.worker_job
             WHERE state = 'ready' AND available_at <= transaction_timestamp()
               AND payload ->> 'kind' = ANY(p_kinds));
END $$;
CREATE OR REPLACE FUNCTION public.ple_complete_worker_job(p_job_id uuid, p_token uuid)
RETURNS boolean LANGUAGE sql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
    WITH completed AS (
        UPDATE public.worker_job
           SET state = 'completed',
               lease_token = NULL,
               lease_expires_at = NULL,
               completed_at = transaction_timestamp()
         WHERE job_id = p_job_id
           AND state = 'leased'
           AND lease_token = p_token
           AND lease_expires_at > transaction_timestamp()
           AND payload ->> 'kind' <> 'gradeAcceptedSubmission'
        RETURNING 1
    )
    SELECT EXISTS(SELECT 1 FROM completed)
$$;
CREATE OR REPLACE FUNCTION public.ple_fail_worker_job(
    p_job_id uuid, p_token uuid, p_failure text
) RETURNS text LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE
    next_state text;
BEGIN
    IF p_failure NOT IN ('transient', 'permanent', 'timed_out') THEN
        RAISE EXCEPTION 'invalid queue failure kind' USING ERRCODE = '22023';
    END IF;
    UPDATE public.worker_job
       SET state = CASE
               WHEN p_failure = 'permanent' OR attempt_count >= max_attempts THEN 'dead'
               ELSE 'ready'
           END,
           available_at = CASE
               WHEN p_failure = 'permanent' OR attempt_count >= max_attempts
                   THEN available_at
               ELSE transaction_timestamp() + make_interval(
                   secs => (1 << LEAST(GREATEST(attempt_count - 1, 0), 8))
               )
           END,
           lease_token = NULL,
           lease_expires_at = NULL,
           last_error = p_failure,
           completed_at = CASE
               WHEN p_failure = 'permanent' OR attempt_count >= max_attempts
                   THEN transaction_timestamp()
               ELSE NULL
           END
     WHERE job_id = p_job_id
       AND state = 'leased'
       AND lease_token = p_token
       AND lease_expires_at > transaction_timestamp()
       AND payload ->> 'kind' <> 'gradeAcceptedSubmission'
    RETURNING state INTO next_state;
    IF next_state IS NULL THEN
        RETURN NULL;
    END IF;
    RETURN CASE WHEN next_state = 'dead' THEN 'dead' ELSE 'retrying' END;
END $$;
ALTER FUNCTION public.ple_claim_worker_job(uuid, integer, text[]) OWNER TO ple_queue_broker;
ALTER FUNCTION public.ple_ready_worker_queue_depth(text[]) OWNER TO ple_queue_broker;
ALTER FUNCTION public.ple_complete_worker_job(uuid, uuid) OWNER TO ple_queue_broker;
ALTER FUNCTION public.ple_fail_worker_job(uuid, uuid, text) OWNER TO ple_queue_broker;

-- Forced RLS plus separate application, automated-broker, and retention
-- policies retain the existing capabilities' authority boundaries.
CREATE POLICY grading_execution_tenant ON public.grading_execution TO ple_app
    USING (tenant_id = public.ple_current_tenant() AND public.ple_course_records_accessible(tenant_id, course_id))
    WITH CHECK (tenant_id = public.ple_current_tenant() AND public.ple_course_records_accessible(tenant_id, course_id));
CREATE POLICY grading_execution_receipt_tenant ON public.grading_execution_receipt TO ple_app
    USING (tenant_id = public.ple_current_tenant() AND public.ple_course_records_accessible(tenant_id, course_id))
    WITH CHECK (tenant_id = public.ple_current_tenant() AND public.ple_course_records_accessible(tenant_id, course_id));
CREATE POLICY grading_operation_tenant ON public.grading_operation TO ple_app
    USING (tenant_id = public.ple_current_tenant() AND public.ple_course_records_accessible(tenant_id, course_id))
    WITH CHECK (tenant_id = public.ple_current_tenant() AND public.ple_course_records_accessible(tenant_id, course_id));
CREATE POLICY grading_operation_receipt_tenant ON public.grading_operation_receipt TO ple_app
    USING (tenant_id = public.ple_current_tenant() AND public.ple_course_records_accessible(tenant_id, course_id))
    WITH CHECK (tenant_id = public.ple_current_tenant() AND public.ple_course_records_accessible(tenant_id, course_id));
CREATE POLICY automated_grading_broker_execution_select ON public.grading_execution FOR SELECT
    TO ple_automated_grading_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_execution_insert ON public.grading_execution FOR INSERT
    TO ple_automated_grading_broker WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_execution_receipt_insert
    ON public.grading_execution_receipt FOR INSERT TO ple_automated_grading_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_attempt_select ON public.question_attempt FOR SELECT
    TO ple_automated_grading_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_attempt_update ON public.question_attempt FOR UPDATE
    TO ple_automated_grading_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_run ON public.assignment_run FOR SELECT TO ple_automated_grading_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_enrollment ON public.enrollment FOR SELECT TO ple_automated_grading_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_assignment ON public.assignment FOR SELECT TO ple_automated_grading_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_course_member ON public.course_member FOR SELECT TO ple_automated_grading_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_submission_insert ON public.submission FOR INSERT
    TO ple_automated_grading_broker WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_idempotency_select
    ON public.submission_idempotency FOR SELECT TO ple_automated_grading_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_idempotency_insert
    ON public.submission_idempotency FOR INSERT TO ple_automated_grading_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());
-- PostgreSQL requires an UPDATE privilege and policy for SELECT FOR UPDATE.
-- The false check admits row locking but rejects every data update.
CREATE POLICY automated_grading_broker_idempotency_lock
    ON public.submission_idempotency FOR UPDATE TO ple_automated_grading_broker
    USING (tenant_id = public.ple_current_tenant()) WITH CHECK (false);
CREATE POLICY automated_grading_broker_evaluation ON public.submission_evaluation FOR INSERT TO ple_automated_grading_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_job ON public.worker_job FOR INSERT TO ple_automated_grading_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_job_fence_read ON public.worker_job FOR SELECT TO ple_automated_grading_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY retention_broker_grading_execution_select ON public.grading_execution FOR SELECT TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY retention_broker_grading_execution_delete ON public.grading_execution FOR DELETE TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY retention_broker_grading_execution_receipt_select ON public.grading_execution_receipt FOR SELECT TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY retention_broker_grading_execution_receipt_delete ON public.grading_execution_receipt FOR DELETE TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY retention_broker_grading_operation_select ON public.grading_operation FOR SELECT TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY retention_broker_grading_operation_delete ON public.grading_operation FOR DELETE TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY retention_broker_grading_operation_receipt_select ON public.grading_operation_receipt FOR SELECT TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY retention_broker_grading_operation_receipt_delete ON public.grading_operation_receipt FOR DELETE TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
GRANT SELECT ON public.grading_execution, public.grading_execution_receipt,
    public.grading_operation, public.grading_operation_receipt TO ple_app;
GRANT SELECT ON public.question_attempt, public.assignment_run, public.enrollment,
    public.assignment, public.course_member, public.submission_idempotency,
    public.grading_execution, public.worker_job TO ple_automated_grading_broker;
-- The two actual state columns also satisfy PostgreSQL's SELECT FOR UPDATE
-- requirement; immutable attempt identity receives no mutation authority.
GRANT UPDATE (attempt_status, submitted_at) ON public.question_attempt
    TO ple_automated_grading_broker;
-- PostgreSQL requires UPDATE on an existing column for SELECT FOR UPDATE. The
-- idempotency lock policy rejects every attempted data change.
GRANT UPDATE (idempotency_key) ON public.submission_idempotency
    TO ple_automated_grading_broker;
GRANT INSERT ON public.submission, public.submission_idempotency,
    public.submission_evaluation, public.worker_job, public.grading_execution,
    public.grading_execution_receipt TO ple_automated_grading_broker;
GRANT SELECT, DELETE ON public.grading_execution, public.grading_execution_receipt,
    public.grading_operation, public.grading_operation_receipt TO ple_retention_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(),
    public.ple_course_records_accessible(uuid, uuid) TO ple_automated_grading_broker;

-- The accepted-submission broker is the only learner-input mutation boundary.
-- It locks the attempt before checking idempotency, so both an existing key and
-- an absent key have one linearization point (ASVS 2.3.1, 2.3.3, 2.3.4).
ALTER TABLE public.submission_idempotency
    ADD COLUMN accepted_actor_id uuid,
    DROP CONSTRAINT submission_idempotency_request_contract_version_check,
    ADD CONSTRAINT submission_idempotency_request_contract_version_check CHECK (
        request_contract_version IN (0, 1, 2)
    ),
    ADD CONSTRAINT submission_idempotency_accepted_actor_check CHECK (
        request_contract_version < 2 OR accepted_actor_id IS NOT NULL
    );
-- G1 learner records participate through the active learner-record fence.
-- Existing trigger bindings retain the predecessor OID; the new definition
-- owns the newly introduced relation family and the same course lifecycle
-- lock. This keeps a forward migration from rewriting unrelated trigger ABI.
ALTER FUNCTION public.ple_fence_learner_record_write()
    RENAME TO ple_fence_learner_record_write_before_automated_grading;
CREATE FUNCTION public.ple_fence_learner_record_write() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF TG_TABLE_NAME NOT IN (
        'grading_execution', 'grading_execution_receipt',
        'grading_operation', 'grading_operation_receipt',
        'accepted_submission_private_response'
    ) THEN
        RAISE EXCEPTION 'unsupported learner record fence table' USING ERRCODE = '22023';
    END IF;
    IF TG_OP = 'UPDATE' AND OLD.course_id IS DISTINCT FROM NEW.course_id THEN
        RAISE EXCEPTION 'learner record course ownership is immutable' USING ERRCODE = '22023';
    END IF;
    IF NEW.tenant_id IS NULL OR NEW.course_id IS NULL
       OR NOT public.ple_lock_course_write(NEW.tenant_id, NEW.course_id, false)
    THEN
        RAISE EXCEPTION 'learner record course is unavailable' USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END $$;
ALTER FUNCTION public.ple_fence_learner_record_write() OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_fence_learner_record_write() FROM PUBLIC, ple_app,
    ple_automated_grading_broker;
CREATE TRIGGER grading_execution_retention_fence BEFORE INSERT OR UPDATE ON public.grading_execution
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();
CREATE TRIGGER grading_execution_receipt_retention_fence BEFORE INSERT OR UPDATE ON public.grading_execution_receipt
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();
CREATE TRIGGER grading_operation_retention_fence BEFORE INSERT OR UPDATE ON public.grading_operation
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();
CREATE TRIGGER grading_operation_receipt_retention_fence BEFORE INSERT OR UPDATE ON public.grading_operation_receipt
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

-- Prepared deletion work proves the exact G1 rows and accepted-submission
-- jobs that the active retention capability owns. The counts live with the
-- existing manifest, not in a side-car fence.
ALTER TABLE public.course_retention_cleanup_manifest
    ADD COLUMN automated_grading_execution_count bigint NOT NULL DEFAULT 0
        CHECK (automated_grading_execution_count >= 0),
    ADD COLUMN automated_grading_job_count bigint NOT NULL DEFAULT 0
        CHECK (automated_grading_job_count >= 0);
CREATE TABLE public.course_retention_purge_grading_execution (
    tenant_id uuid NOT NULL, course_id uuid NOT NULL, generation bigint NOT NULL,
    stage text NOT NULL, attempt_id uuid NOT NULL,
    PRIMARY KEY (tenant_id, course_id, generation, stage, attempt_id),
    FOREIGN KEY (tenant_id, course_id, generation, stage)
        REFERENCES public.course_retention_cleanup_manifest(tenant_id, course_id, generation, stage)
        ON DELETE CASCADE
);
CREATE TABLE public.course_retention_purge_grading_job (
    tenant_id uuid NOT NULL, course_id uuid NOT NULL, generation bigint NOT NULL,
    stage text NOT NULL, job_id uuid NOT NULL,
    PRIMARY KEY (tenant_id, course_id, generation, stage, job_id),
    FOREIGN KEY (tenant_id, course_id, generation, stage)
        REFERENCES public.course_retention_cleanup_manifest(tenant_id, course_id, generation, stage)
        ON DELETE CASCADE
);
ALTER TABLE public.course_retention_purge_grading_execution ENABLE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.course_retention_purge_grading_execution FORCE ROW LEVEL SECURITY;
ALTER TABLE public.course_retention_purge_grading_job ENABLE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.course_retention_purge_grading_job FORCE ROW LEVEL SECURITY;
CREATE POLICY retention_purge_grading_execution_broker
    ON public.course_retention_purge_grading_execution TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY retention_purge_grading_job_broker
    ON public.course_retention_purge_grading_job TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
GRANT SELECT, INSERT, DELETE ON public.course_retention_purge_grading_execution,
    public.course_retention_purge_grading_job TO ple_retention_broker;

ALTER FUNCTION public.ple_prepare_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    RENAME TO ple_prepare_delete_retention_work_before_automated_grading;
CREATE FUNCTION public.ple_prepare_delete_retention_work(
    p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v_result jsonb;
    v_execution_count bigint;
    v_job_count bigint;
BEGIN
    v_result := public.ple_prepare_delete_retention_work_before_automated_grading(
        p_tenant, p_job, p_token, p_course, p_stage, p_generation
    );
    IF v_result IS NULL OR p_stage <> 'deleteStudentRecords' THEN
        RETURN v_result;
    END IF;
    INSERT INTO public.course_retention_purge_grading_execution
        (tenant_id, course_id, generation, stage, attempt_id)
    SELECT p_tenant, p_course, p_generation, p_stage, execution.attempt_id
      FROM public.grading_execution execution
     WHERE execution.tenant_id = p_tenant AND execution.course_id = p_course
    ON CONFLICT DO NOTHING;
    INSERT INTO public.course_retention_purge_grading_job
        (tenant_id, course_id, generation, stage, job_id)
    SELECT p_tenant, p_course, p_generation, p_stage, job.job_id
      FROM public.grading_execution AS execution
      JOIN public.worker_job AS job
        ON job.tenant_id = execution.tenant_id
       AND job.job_id = execution.current_job_id
     WHERE execution.tenant_id = p_tenant AND execution.course_id = p_course
       AND job.payload = jsonb_build_object(
           'kind', 'gradeAcceptedSubmission',
           'attempt', execution.attempt_id::text,
           'submission', execution.submission_id::text,
           'execution_generation', execution.execution_generation
       )
    ON CONFLICT DO NOTHING;
    SELECT count(*) INTO v_execution_count
      FROM public.course_retention_purge_grading_execution
     WHERE tenant_id = p_tenant AND course_id = p_course
       AND generation = p_generation AND stage = p_stage;
    SELECT count(*) INTO v_job_count
      FROM public.course_retention_purge_grading_job
     WHERE tenant_id = p_tenant AND course_id = p_course
       AND generation = p_generation AND stage = p_stage;
    UPDATE public.course_retention_cleanup_manifest
       SET automated_grading_execution_count = v_execution_count,
           automated_grading_job_count = v_job_count
     WHERE tenant_id = p_tenant AND course_id = p_course
       AND generation = p_generation AND stage = p_stage AND job_id = p_job
       AND state = 'prepared';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'automated-grading retention manifest is unavailable' USING ERRCODE = '55000';
    END IF;
    RETURN v_result;
END $$;
ALTER FUNCTION public.ple_prepare_delete_retention_work_before_automated_grading(
    uuid, uuid, uuid, uuid, text, bigint
) OWNER TO ple_retention_broker;
ALTER FUNCTION public.ple_prepare_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_prepare_delete_retention_work_before_automated_grading(
    uuid, uuid, uuid, uuid, text, bigint
) FROM PUBLIC, ple_app, ple_automated_grading_broker;
REVOKE ALL ON FUNCTION public.ple_prepare_delete_retention_work(
    uuid, uuid, uuid, uuid, text, bigint
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_prepare_delete_retention_work(
    uuid, uuid, uuid, uuid, text, bigint
) TO ple_app;

-- Child G1 evidence purges before the predecessor deletes submissions and
-- attempts.  Exact retention attestation happens before mutation; a false
-- predecessor result rolls the subtransaction back.
ALTER FUNCTION public.ple_commit_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    RENAME TO ple_commit_delete_retention_work_before_automated_grading;
REVOKE ALL ON FUNCTION public.ple_commit_delete_retention_work_before_automated_grading(
    uuid, uuid, uuid, uuid, text, bigint
) FROM PUBLIC, ple_app, ple_automated_grading_broker;
CREATE FUNCTION public.ple_automated_grading_retention_attested(
    p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_tenant IS NULL OR p_job IS NULL OR p_token IS NULL OR p_course IS NULL
       OR p_generation IS NULL OR p_generation <= 0 OR p_stage <> 'deleteStudentRecords'
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN RETURN false; END IF;
    PERFORM 1 FROM public.worker_job AS job JOIN public.course_retention_dispatch AS dispatch
        ON dispatch.tenant_id = job.tenant_id AND dispatch.job_id = job.job_id
       AND dispatch.course_id = p_course AND dispatch.stage = p_stage AND dispatch.generation = p_generation
      JOIN public.course_retention AS retention
        ON retention.tenant_id = dispatch.tenant_id AND retention.course_id = dispatch.course_id
       AND retention.generation = dispatch.generation
     WHERE job.tenant_id = p_tenant AND job.job_id = p_job AND job.state = 'leased'
       AND job.lease_token = p_token AND job.lease_expires_at > transaction_timestamp()
       AND job.payload = jsonb_build_object('kind','retention','course',p_course::text,'stage',p_stage,'generation',p_generation)
       AND retention.lifecycle = 'archived' FOR UPDATE OF job, dispatch, retention;
    RETURN FOUND;
END $$;
CREATE FUNCTION public.ple_commit_delete_retention_work(
    p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint
) RETURNS boolean LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v_committed boolean;
    v_expected_execution_count bigint;
    v_expected_job_count bigint;
    v_execution_count bigint;
    v_job_count bigint;
BEGIN
    IF p_stage <> 'deleteStudentRecords' THEN
        RETURN public.ple_commit_delete_retention_work_before_automated_grading(p_tenant,p_job,p_token,p_course,p_stage,p_generation);
    END IF;
    IF NOT public.ple_automated_grading_retention_attested(p_tenant,p_job,p_token,p_course,p_stage,p_generation) THEN RETURN false; END IF;
    SELECT automated_grading_execution_count, automated_grading_job_count
      INTO v_expected_execution_count, v_expected_job_count
      FROM public.course_retention_cleanup_manifest
     WHERE tenant_id = p_tenant AND course_id = p_course AND generation = p_generation
       AND stage = p_stage AND job_id = p_job AND state = 'prepared'
     FOR UPDATE;
    IF NOT FOUND THEN RETURN false; END IF;
    SELECT count(*) INTO v_execution_count
      FROM public.course_retention_purge_grading_execution
     WHERE tenant_id = p_tenant AND course_id = p_course AND generation = p_generation AND stage = p_stage;
    SELECT count(*) INTO v_job_count
      FROM public.course_retention_purge_grading_job
     WHERE tenant_id = p_tenant AND course_id = p_course AND generation = p_generation AND stage = p_stage;
    IF v_execution_count IS DISTINCT FROM v_expected_execution_count
       OR v_job_count IS DISTINCT FROM v_expected_job_count
    THEN RETURN false; END IF;
    BEGIN
        -- Operation receipts can be assignment-scoped; delete all course rows
        -- only after the prepared execution set established retention ownership.
        DELETE FROM public.grading_operation_receipt WHERE tenant_id = p_tenant AND course_id = p_course;
        DELETE FROM public.grading_execution_receipt receipt
         USING public.course_retention_purge_grading_execution prepared
         WHERE receipt.tenant_id = p_tenant AND receipt.course_id = p_course
           AND prepared.tenant_id = p_tenant AND prepared.course_id = p_course
           AND prepared.generation = p_generation AND prepared.stage = p_stage
           AND prepared.attempt_id = receipt.attempt_id;
        DELETE FROM public.grading_operation WHERE tenant_id = p_tenant AND course_id = p_course;
        DELETE FROM public.grading_execution execution
         USING public.course_retention_purge_grading_execution prepared
         WHERE execution.tenant_id = p_tenant AND execution.course_id = p_course
           AND prepared.tenant_id = p_tenant AND prepared.course_id = p_course
           AND prepared.generation = p_generation AND prepared.stage = p_stage
           AND prepared.attempt_id = execution.attempt_id;
        DELETE FROM public.worker_job job
         USING public.course_retention_purge_grading_job prepared
         WHERE job.tenant_id = p_tenant AND job.job_id = prepared.job_id
           AND prepared.tenant_id = p_tenant AND prepared.course_id = p_course
           AND prepared.generation = p_generation AND prepared.stage = p_stage;
        v_committed := public.ple_commit_delete_retention_work_before_automated_grading(p_tenant,p_job,p_token,p_course,p_stage,p_generation);
        IF NOT v_committed THEN RAISE EXCEPTION 'automated-grading retention commit conflicted' USING ERRCODE = 'PBI03'; END IF;
    EXCEPTION WHEN SQLSTATE 'PBI03' THEN RETURN false;
    END;
    RETURN NOT EXISTS (
        SELECT 1 FROM public.grading_operation_receipt WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.grading_execution_receipt WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.grading_operation WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.grading_execution WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL
        SELECT 1
          FROM public.grading_execution AS execution
          JOIN public.worker_job AS job
            ON job.tenant_id = execution.tenant_id
           AND job.job_id = execution.current_job_id
         WHERE execution.tenant_id = p_tenant AND execution.course_id = p_course
           AND job.payload = jsonb_build_object(
               'kind', 'gradeAcceptedSubmission',
               'attempt', execution.attempt_id::text,
               'submission', execution.submission_id::text,
               'execution_generation', execution.execution_generation
           )
    );
END $$;
ALTER FUNCTION public.ple_automated_grading_retention_attested(uuid, uuid, uuid, uuid, text, bigint) OWNER TO ple_retention_broker;
ALTER FUNCTION public.ple_commit_delete_retention_work_before_automated_grading(uuid, uuid, uuid, uuid, text, bigint) OWNER TO ple_retention_broker;
ALTER FUNCTION public.ple_commit_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint) OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_automated_grading_retention_attested(uuid, uuid, uuid, uuid, text, bigint)
    FROM PUBLIC, ple_app, ple_automated_grading_broker;
REVOKE ALL ON FUNCTION public.ple_commit_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_commit_delete_retention_work(
    uuid, uuid, uuid, uuid, text, bigint
) TO ple_app;

DO $$
BEGIN
    -- ASVS 8.2.1-8.2.3: acceptance has only its exact read, insert, and
    -- lock-column authority. Each positive grant below is exercised by the
    -- acceptance transaction or one of its deferred relationship fences.
    IF NOT has_table_privilege('ple_automated_grading_broker',
            'public.question_attempt', 'SELECT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.assignment_run', 'SELECT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.enrollment', 'SELECT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.assignment', 'SELECT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.course_member', 'SELECT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.submission_idempotency', 'SELECT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.grading_execution', 'SELECT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.worker_job', 'SELECT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.submission', 'INSERT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.submission_idempotency', 'INSERT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.submission_evaluation', 'INSERT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.worker_job', 'INSERT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.grading_execution', 'INSERT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.grading_execution_receipt', 'INSERT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.question_attempt', 'attempt_status', 'UPDATE')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.question_attempt', 'submitted_at', 'UPDATE')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.submission_idempotency', 'idempotency_key', 'UPDATE')
       OR NOT has_function_privilege('ple_automated_grading_broker',
            'public.ple_current_tenant()', 'EXECUTE')
       OR NOT has_function_privilege('ple_automated_grading_broker',
            'public.ple_course_records_accessible(uuid,uuid)', 'EXECUTE')
    THEN
        RAISE EXCEPTION 'automated-grading acceptance capability is incomplete';
    END IF;

    IF has_table_privilege('ple_app', 'public.grading_execution', 'INSERT')
       OR has_table_privilege('ple_app', 'public.grading_operation', 'UPDATE')
       OR has_table_privilege('ple_automated_grading_broker', 'public.worker_job', 'UPDATE')
       OR has_table_privilege('ple_automated_grading_broker', 'public.grading_execution', 'UPDATE')
       OR has_table_privilege('ple_automated_grading_broker', 'public.grading_execution_receipt', 'UPDATE')
       OR has_table_privilege('ple_automated_grading_broker', 'public.grading_execution_receipt', 'DELETE')
       OR has_table_privilege('ple_automated_grading_broker', 'public.grading_operation', 'SELECT')
       OR has_table_privilege('ple_automated_grading_broker', 'public.grading_operation_receipt', 'INSERT')
       OR has_table_privilege('ple_automated_grading_broker', 'public.manual_grade_receipt', 'UPDATE')
       OR has_table_privilege('ple_automated_grading_broker', 'public.assignment', 'UPDATE')
       OR has_table_privilege('ple_automated_grading_broker', 'public.attempt_score_current', 'UPDATE')
       OR has_column_privilege('ple_automated_grading_broker',
            'public.question_attempt', 'attempt_id', 'UPDATE')
       OR has_column_privilege('ple_automated_grading_broker',
            'public.question_attempt', 'course_id', 'UPDATE')
       OR has_column_privilege('ple_automated_grading_broker',
            'public.submission_idempotency', 'payload', 'UPDATE')
       OR has_column_privilege('ple_automated_grading_broker',
            'public.submission_idempotency', 'course_id', 'UPDATE')
       OR has_column_privilege('ple_automated_grading_broker',
            'public.submission', 'payload', 'SELECT')
       OR has_schema_privilege('ple_automated_grading_broker', 'public', 'CREATE')
       OR has_function_privilege('public', 'public.ple_fence_learner_record_write()', 'EXECUTE')
       OR has_function_privilege('ple_app',
            'public.ple_guard_accepted_submission_job_execution()', 'EXECUTE')
    THEN
        RAISE EXCEPTION 'automated-grading acceptance capability is overprivileged';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_class AS relation
         WHERE relation.oid = 'public.grading_execution'::regclass
           AND relation.relrowsecurity AND relation.relforcerowsecurity
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_policy AS policy
         WHERE policy.polrelid = 'public.worker_job'::regclass
           AND policy.polname = 'worker_job_tenant_insert'
           AND policy.polcmd = 'a'
           AND policy.polroles = ARRAY['ple_app'::regrole]::oid[]
           AND pg_catalog.pg_get_expr(policy.polwithcheck, policy.polrelid)
               LIKE '%gradeAcceptedSubmission%'
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_trigger AS trigger
         WHERE trigger.tgname = 'grading_execution_exact_job_fence'
           AND trigger.tgrelid = 'public.grading_execution'::regclass
           AND trigger.tgfoid = 'public.ple_guard_grading_execution_job()'::regprocedure
           AND trigger.tgdeferrable AND trigger.tginitdeferred
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_trigger AS trigger
         WHERE trigger.tgname = 'worker_job_exact_grading_execution_fence'
           AND trigger.tgrelid = 'public.worker_job'::regclass
           AND trigger.tgfoid =
               'public.ple_guard_accepted_submission_job_execution()'::regprocedure
           AND trigger.tgdeferrable AND trigger.tginitdeferred
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_constraint AS catalog_constraint
         WHERE catalog_constraint.conname = 'submission_idempotency_submission_fk'
           AND catalog_constraint.contype = 'f'
           AND catalog_constraint.conrelid = 'public.submission_idempotency'::regclass
           AND catalog_constraint.confrelid = 'public.submission'::regclass
           AND catalog_constraint.condeferrable AND catalog_constraint.condeferred
           AND catalog_constraint.conkey = ARRAY[
               (SELECT attnum FROM pg_catalog.pg_attribute
                 WHERE attrelid = 'public.submission_idempotency'::regclass
                   AND attname = 'tenant_id'),
               (SELECT attnum FROM pg_catalog.pg_attribute
                 WHERE attrelid = 'public.submission_idempotency'::regclass
                   AND attname = 'course_id'),
               (SELECT attnum FROM pg_catalog.pg_attribute
                 WHERE attrelid = 'public.submission_idempotency'::regclass
                   AND attname = 'attempt_id'),
               (SELECT attnum FROM pg_catalog.pg_attribute
                 WHERE attrelid = 'public.submission_idempotency'::regclass
                   AND attname = 'submission_id'),
               (SELECT attnum FROM pg_catalog.pg_attribute
                 WHERE attrelid = 'public.submission_idempotency'::regclass
                   AND attname = 'submission_occurred_at')
           ]::smallint[]
    ) THEN
        RAISE EXCEPTION 'automated-grading relationship or RLS fence is unsafe';
    END IF;
END $$;

COMMIT;
