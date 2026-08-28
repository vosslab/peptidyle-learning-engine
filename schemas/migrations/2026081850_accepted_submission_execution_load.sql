-- WP-PROF-G1: lease-fenced private accepted-submission execution load.
-- This capability reads immutable accepted input only after an exact worker
-- lease. It never writes learner input, grading state, receipts, or scores.

BEGIN;

-- This role is a SET-only capability, never an inherited application role.
-- ASVS 8.1-8.4: the API accepts metadata; a worker explicitly activates this
-- capability for one lease-fenced private descriptor load.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_roles
         WHERE rolname = 'ple_accepted_submission_execution'
    ) THEN
        CREATE ROLE ple_accepted_submission_execution
            NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid = 'ple_accepted_submission_execution'::regrole
            OR membership.member = 'ple_accepted_submission_execution'::regrole
    ) THEN
        RAISE EXCEPTION 'accepted-submission execution capability must have no memberships';
    END IF;
END $$;
ALTER ROLE ple_accepted_submission_execution
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
REVOKE ALL ON SCHEMA public FROM ple_accepted_submission_execution;
GRANT USAGE ON SCHEMA public TO ple_accepted_submission_execution;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_roles
         WHERE rolname = 'ple_accepted_submission_execution_reader'
    ) THEN
        CREATE ROLE ple_accepted_submission_execution_reader
            NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid = 'ple_accepted_submission_execution_reader'::regrole
            OR membership.member = 'ple_accepted_submission_execution_reader'::regrole
    ) THEN
        RAISE EXCEPTION 'accepted-submission execution reader must not have role memberships';
    END IF;
END $$;
ALTER ROLE ple_accepted_submission_execution_reader
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
REVOKE ALL ON SCHEMA public FROM ple_accepted_submission_execution_reader;
GRANT USAGE ON SCHEMA public TO ple_accepted_submission_execution_reader;

-- Canonical learner response text has one authority.  The course-bearing
-- composite foreign keys bind it to both answer-free parents; a same-tenant
-- row can never be reattached to another course or attempt (ASVS 8.2.2-8.2.3).
CREATE TABLE public.accepted_submission_private_response (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    submission_id uuid NOT NULL,
    submission_occurred_at timestamp with time zone NOT NULL,
    response_canonical_json text NOT NULL,
    response_sha256 character(64) NOT NULL,
    CONSTRAINT accepted_submission_private_response_pkey PRIMARY KEY
        (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at),
    CONSTRAINT accepted_submission_private_response_size_check
        CHECK (octet_length(response_canonical_json) BETWEEN 1 AND 32768),
    CONSTRAINT accepted_submission_private_response_sha256_check
        CHECK (response_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT accepted_submission_private_response_submission_fk
        FOREIGN KEY (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at)
        REFERENCES public.submission (tenant_id, course_id, attempt_id, submission_id, occurred_at)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT accepted_submission_private_response_idempotency_fk
        FOREIGN KEY (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at)
        REFERENCES public.submission_idempotency
            (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at)
        ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED
);
ALTER TABLE public.accepted_submission_private_response ENABLE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.accepted_submission_private_response FORCE ROW LEVEL SECURITY;

-- ASVS 14.1-14.2: response text is retained only with its learner record and
-- is unavailable to the general application role and ordinary table readers.
-- Establish the private-child matrix before adding the three exact grants.
REVOKE ALL ON TABLE public.accepted_submission_private_response FROM PUBLIC,
    ple_app, ple_auth, ple_student, ple_grader, ple_grading_reader,
    ple_queue_broker, ple_accepted_submission_execution;
CREATE POLICY accepted_private_response_broker_select
    ON public.accepted_submission_private_response FOR SELECT
    TO ple_automated_grading_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY accepted_private_response_broker_insert
    ON public.accepted_submission_private_response FOR INSERT
    TO ple_automated_grading_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY accepted_private_response_reader
    ON public.accepted_submission_private_response FOR SELECT
    TO ple_accepted_submission_execution_reader
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY accepted_private_response_retention
    ON public.accepted_submission_private_response FOR DELETE TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());

GRANT SELECT, INSERT ON public.accepted_submission_private_response
    TO ple_automated_grading_broker;
GRANT SELECT (tenant_id, course_id, attempt_id, submission_id, submission_occurred_at,
    response_canonical_json, response_sha256)
    ON public.accepted_submission_private_response
    TO ple_accepted_submission_execution_reader;
GRANT SELECT, DELETE ON public.accepted_submission_private_response TO ple_retention_broker;

-- The guard is intentionally invoker-owned: only the retention capability
-- can delete accepted input. ASVS 15.3 and 15.4 keep the immutable first
-- effect and its private child inseparable.
CREATE FUNCTION public.ple_forbid_accepted_submission_mutation() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' AND current_user = 'ple_retention_broker' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'accepted submissions are retention-deleted only' USING ERRCODE = '42501';
END $$;
CREATE TRIGGER submission_accepted_input_append_only
    BEFORE UPDATE OR DELETE ON public.submission
    FOR EACH ROW EXECUTE FUNCTION public.ple_forbid_accepted_submission_mutation();
CREATE TRIGGER submission_idempotency_accepted_input_append_only
    BEFORE UPDATE OR DELETE ON public.submission_idempotency
    FOR EACH ROW WHEN (OLD.request_contract_version = 2)
    EXECUTE FUNCTION public.ple_forbid_accepted_submission_mutation();
CREATE TRIGGER accepted_submission_private_response_append_only
    BEFORE UPDATE OR DELETE ON public.accepted_submission_private_response
    FOR EACH ROW EXECUTE FUNCTION public.ple_forbid_accepted_submission_mutation();
-- ASVS 14.2.4 and 14.2.7: private response insertion shares the current
-- retention lock authority with every other learner record.
CREATE TRIGGER accepted_submission_private_response_retention_fence
    BEFORE INSERT OR UPDATE ON public.accepted_submission_private_response
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_learner_record_write();

-- ASVS 1.2.4, 1.5.2-1.5.3, 2.2-2.3: typed Rust supplies canonical JSON;
-- this broker bounds and parses it, hashes its exact UTF-8 text, and performs
-- the complete first effect under the attempt lock. It returns metadata only.
CREATE FUNCTION public.ple_accept_automated_submission_v1(
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
         course_id, execution_generation, resulting_state, occurred_at)
    VALUES (p_tenant, p_job, p_attempt, v_submission, v_occurred_at,
            v_course, 1, 'ready', v_occurred_at);
    RETURN QUERY SELECT 'accepted', p_tenant, v_course, v_assignment, p_attempt,
        v_submission, p_actor, p_idempotency_key, v_response_sha256,
        floor(extract(epoch FROM v_occurred_at) * 1000)::bigint;
END $$;
ALTER FUNCTION public.ple_accept_automated_submission_v1(uuid, uuid, uuid, uuid, uuid, text, text, uuid)
    OWNER TO ple_automated_grading_broker;
ALTER FUNCTION public.ple_forbid_accepted_submission_mutation() OWNER TO ple_automated_grading_broker;
REVOKE ALL ON FUNCTION public.ple_accept_automated_submission_v1(uuid, uuid, uuid, uuid, uuid, text, text, uuid)
    FROM PUBLIC, ple_auth, ple_student, ple_grader, ple_grading_reader,
        ple_queue_broker, ple_accepted_submission_execution,
        ple_accepted_submission_execution_reader, ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_forbid_accepted_submission_mutation() FROM PUBLIC, ple_app,
    ple_auth, ple_student, ple_grader, ple_grading_reader, ple_queue_broker,
    ple_accepted_submission_execution, ple_accepted_submission_execution_reader,
    ple_retention_broker;
GRANT EXECUTE ON FUNCTION public.ple_accept_automated_submission_v1(
    uuid, uuid, uuid, uuid, uuid, text, text, uuid
) TO ple_app;

-- ASVS 8.2.1-8.2.3: the acceptance broker reads only the sealed timing
-- witness needed by its own first-effect capability. FORCE RLS keeps the
-- function owner tenant-bound even under SECURITY DEFINER.
REVOKE ALL ON TABLE public.attempt_effective_policy_current,
    public.attempt_effective_policy_receipt FROM ple_automated_grading_broker;
CREATE POLICY automated_grading_broker_effective_policy_current_select
    ON public.attempt_effective_policy_current FOR SELECT
    TO ple_automated_grading_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY automated_grading_broker_effective_policy_receipt_select
    ON public.attempt_effective_policy_receipt FOR SELECT
    TO ple_automated_grading_broker
    USING (tenant_id = public.ple_current_tenant());
GRANT SELECT (tenant_id, attempt_id, attempt_occurred_at, assignment_id, course_id,
    receipt_generation) ON public.attempt_effective_policy_current
    TO ple_automated_grading_broker;
GRANT SELECT (tenant_id, attempt_id, receipt_generation, attempt_occurred_at,
    assignment_id, course_id, auto_submit_at, sealed_at)
    ON public.attempt_effective_policy_receipt TO ple_automated_grading_broker;

-- Every private read remains tenant scoped under FORCE RLS. The capability
-- owner gets only the columns its sealed descriptor returns.
CREATE POLICY accepted_execution_reader_worker_job ON public.worker_job FOR SELECT
    TO ple_accepted_submission_execution_reader
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY accepted_execution_reader_grading_execution ON public.grading_execution FOR SELECT
    TO ple_accepted_submission_execution_reader
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY accepted_execution_reader_submission ON public.submission FOR SELECT
    TO ple_accepted_submission_execution_reader
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY accepted_execution_reader_idempotency ON public.submission_idempotency FOR SELECT
    TO ple_accepted_submission_execution_reader
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY accepted_execution_reader_attempt ON public.question_attempt FOR SELECT
    TO ple_accepted_submission_execution_reader
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY accepted_execution_reader_private_execution
    ON public.issued_attempt_private_execution FOR SELECT
    TO ple_accepted_submission_execution_reader
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY accepted_execution_reader_run ON public.assignment_run FOR SELECT
    TO ple_accepted_submission_execution_reader
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY accepted_execution_reader_enrollment ON public.enrollment FOR SELECT
    TO ple_accepted_submission_execution_reader
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY accepted_execution_reader_assignment ON public.assignment FOR SELECT
    TO ple_accepted_submission_execution_reader
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY accepted_execution_reader_retention ON public.course_retention FOR SELECT
    TO ple_accepted_submission_execution_reader
    USING (tenant_id = public.ple_current_tenant());

GRANT SELECT (tenant_id, job_id, payload, state, lease_token, lease_expires_at)
    ON public.worker_job TO ple_accepted_submission_execution_reader;
GRANT SELECT (tenant_id, attempt_id, submission_id, submission_occurred_at, course_id,
    execution_generation, state, current_job_id)
    ON public.grading_execution TO ple_accepted_submission_execution_reader;
GRANT SELECT (tenant_id, submission_id, occurred_at, payload_sha256, attempt_id,
    idempotency_key, course_id)
    ON public.submission TO ple_accepted_submission_execution_reader;
GRANT SELECT (tenant_id, attempt_id, idempotency_key, request_contract_version,
    request_sha256, submitted_at, payload_sha256, course_id, submission_id,
    submission_occurred_at, accepted_actor_id)
    ON public.submission_idempotency TO ple_accepted_submission_execution_reader;
GRANT SELECT (tenant_id, attempt_id, run_id, problem_id, version_id, occurred_at, payload,
    payload_sha256, course_id, presentation_descriptor_version, presentation_nonce,
    presentation_digest, presentation_capability, presentation_payload,
    presentation_payload_sha256, grading_envelope_payload, grading_envelope_payload_sha256,
    issued_question_snapshot_payload, issued_question_snapshot_payload_sha256)
    ON public.question_attempt TO ple_accepted_submission_execution_reader;
GRANT SELECT (tenant_id, attempt_id, attempt_occurred_at, flat_required, flat_payload,
    flat_payload_sha256, webwork_required, webwork_payload, webwork_payload_sha256,
    webwork_replay_payload, webwork_replay_payload_sha256, qti_required, qti_payload,
    qti_payload_sha256)
    ON public.issued_attempt_private_execution TO ple_accepted_submission_execution_reader;
GRANT SELECT (tenant_id, run_id, enrollment_id)
    ON public.assignment_run TO ple_accepted_submission_execution_reader;
GRANT SELECT (tenant_id, enrollment_id, assignment_id)
    ON public.enrollment TO ple_accepted_submission_execution_reader;
GRANT SELECT (tenant_id, assignment_id, course_id)
    ON public.assignment TO ple_accepted_submission_execution_reader;
GRANT SELECT (tenant_id, course_id, lifecycle)
    ON public.course_retention TO ple_accepted_submission_execution_reader;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant()
    TO ple_accepted_submission_execution_reader;

-- ASVS 1.5.2, 2.2.1-2.2.3, 2.3.1-2.3.4, and 8.3.1: caller-provided
-- identifiers select exactly one active worker claim. The projection has no
-- mutable catalog source, browser DTO, score, feedback, or failure text.
CREATE FUNCTION public.ple_load_accepted_submission_execution_v1(
    p_tenant uuid, p_job uuid, p_lease_token uuid, p_submission uuid,
    p_execution_generation bigint
) RETURNS TABLE(
    worker_job_id uuid, worker_lease_token uuid, execution_generation bigint,
    execution_state text, accepted_tenant_id uuid, accepted_course_id uuid,
    accepted_assignment_id uuid, accepted_attempt_id uuid,
    accepted_submission_id uuid, accepted_actor_id uuid,
    accepted_idempotency_key text, accepted_request_sha256 character(64),
    accepted_millis bigint, response_canonical_json text, attempt_payload jsonb,
    attempt_payload_sha256 character(64), presentation_descriptor_version smallint,
    presentation_nonce bytea, presentation_digest bytea, presentation_capability text,
    presentation_payload jsonb, presentation_payload_sha256 character(64),
    grading_envelope_payload jsonb, grading_envelope_payload_sha256 character(64),
    issued_question_snapshot_payload jsonb,
    issued_question_snapshot_payload_sha256 character(64), flat_required boolean,
    flat_payload jsonb, flat_payload_sha256 character(64), webwork_required boolean,
    webwork_payload jsonb, webwork_payload_sha256 character(64),
    webwork_replay_payload jsonb, webwork_replay_payload_sha256 character(64),
    qti_required boolean, qti_payload bytea, qti_payload_sha256 character(64)
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_tenant IS NULL OR p_job IS NULL OR p_lease_token IS NULL
       OR p_submission IS NULL OR p_execution_generation IS NULL
       OR p_execution_generation <= 0
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
    THEN
        RETURN;
    END IF;

    RETURN QUERY
    SELECT job.job_id, job.lease_token, execution.execution_generation, execution.state,
        accepted.tenant_id, accepted.course_id, assignment.assignment_id, attempt.attempt_id,
        accepted.submission_id, accepted.accepted_actor_id, accepted.idempotency_key,
        accepted.request_sha256,
        floor(extract(epoch FROM accepted.submitted_at) * 1000)::bigint,
        response.response_canonical_json, attempt.payload, attempt.payload_sha256,
        attempt.presentation_descriptor_version, attempt.presentation_nonce,
        attempt.presentation_digest, attempt.presentation_capability,
        attempt.presentation_payload, attempt.presentation_payload_sha256,
        attempt.grading_envelope_payload, attempt.grading_envelope_payload_sha256,
        attempt.issued_question_snapshot_payload,
        attempt.issued_question_snapshot_payload_sha256, private.flat_required,
        private.flat_payload, private.flat_payload_sha256, private.webwork_required,
        private.webwork_payload, private.webwork_payload_sha256,
        private.webwork_replay_payload, private.webwork_replay_payload_sha256,
        private.qti_required, private.qti_payload, private.qti_payload_sha256
      FROM public.worker_job AS job
      JOIN public.grading_execution AS execution
        ON execution.tenant_id = job.tenant_id
       AND execution.current_job_id = job.job_id
      JOIN public.question_attempt AS attempt
        ON attempt.tenant_id = execution.tenant_id
       AND attempt.attempt_id = execution.attempt_id
      JOIN public.submission AS accepted_submission
        ON accepted_submission.tenant_id = execution.tenant_id
       AND accepted_submission.attempt_id = execution.attempt_id
       AND accepted_submission.submission_id = execution.submission_id
       AND accepted_submission.occurred_at = execution.submission_occurred_at
      JOIN public.submission_idempotency AS accepted
        ON accepted.tenant_id = execution.tenant_id
       AND accepted.attempt_id = execution.attempt_id
       AND accepted.submission_id = execution.submission_id
       AND accepted.submission_occurred_at = execution.submission_occurred_at
      JOIN public.accepted_submission_private_response AS response
        ON response.tenant_id = execution.tenant_id
       AND response.course_id = execution.course_id
       AND response.attempt_id = execution.attempt_id
       AND response.submission_id = execution.submission_id
       AND response.submission_occurred_at = execution.submission_occurred_at
      JOIN public.issued_attempt_private_execution AS private
        ON private.tenant_id = attempt.tenant_id
       AND private.attempt_id = attempt.attempt_id
       AND private.attempt_occurred_at = attempt.occurred_at
      JOIN public.assignment_run AS run
        ON run.tenant_id = attempt.tenant_id AND run.run_id = attempt.run_id
      JOIN public.enrollment AS enrollment
        ON enrollment.tenant_id = run.tenant_id
       AND enrollment.enrollment_id = run.enrollment_id
      JOIN public.assignment AS assignment
        ON assignment.tenant_id = enrollment.tenant_id
       AND assignment.assignment_id = enrollment.assignment_id
      JOIN public.course_retention AS retention
        ON retention.tenant_id = assignment.tenant_id
       AND retention.course_id = assignment.course_id AND retention.lifecycle = 'active'
     WHERE job.tenant_id = p_tenant AND job.job_id = p_job
       AND job.state = 'leased' AND job.lease_token = p_lease_token
       AND job.lease_expires_at > transaction_timestamp()
       AND job.payload = jsonb_build_object(
            'kind', 'gradeAcceptedSubmission', 'attempt', attempt.attempt_id::text,
            'submission', p_submission::text,
            'execution_generation', p_execution_generation
       )
       AND execution.submission_id = p_submission
       AND execution.execution_generation = p_execution_generation
       AND execution.state = 'ready'
       AND execution.course_id = assignment.course_id
       AND attempt.course_id = assignment.course_id
       AND accepted.request_contract_version = 2
       AND accepted.accepted_actor_id IS NOT NULL
       AND accepted.course_id = assignment.course_id
       AND accepted_submission.course_id = assignment.course_id
       AND accepted_submission.idempotency_key = accepted.idempotency_key
       AND accepted.request_sha256 = response.response_sha256
       AND response.response_sha256 = encode(
            pg_catalog.sha256(convert_to(response.response_canonical_json, 'UTF8')), 'hex'
       )
       AND accepted_submission.payload_sha256 = encode(
            pg_catalog.sha256(convert_to(
                '{"kind":"acceptedPrivateResponseV1"}'::jsonb::text, 'UTF8'
            )), 'hex'
       )
       AND accepted.payload_sha256 = encode(
            pg_catalog.sha256(convert_to(
                '{"kind":"acceptedPrivateResponseV1"}'::jsonb::text, 'UTF8'
            )), 'hex'
       )
     FOR SHARE OF job, execution, attempt, retention;
END $$;

ALTER FUNCTION public.ple_load_accepted_submission_execution_v1(
    uuid, uuid, uuid, uuid, bigint
) OWNER TO ple_accepted_submission_execution_reader;
REVOKE ALL ON FUNCTION public.ple_load_accepted_submission_execution_v1(
    uuid, uuid, uuid, uuid, bigint
) FROM PUBLIC, ple_app, ple_auth, ple_student, ple_grader, ple_grading_reader,
    ple_queue_broker, ple_automated_grading_broker, ple_retention_broker;
GRANT EXECUTE ON FUNCTION public.ple_load_accepted_submission_execution_v1(
    uuid, uuid, uuid, uuid, bigint
) TO ple_accepted_submission_execution;

-- The process-login provisioner grants SET-only membership to ple_worker_login
-- after migration administration. This capability itself remains membership
-- free, so API sessions cannot inherit or delegate the private read path.
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM ple_accepted_submission_execution;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM ple_accepted_submission_execution;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA public FROM ple_accepted_submission_execution;
GRANT EXECUTE ON FUNCTION public.ple_load_accepted_submission_execution_v1(
    uuid, uuid, uuid, uuid, bigint
) TO ple_accepted_submission_execution;

-- The preceding retention wrapper deletes the two answer-free parents. Their
-- composite cascade deletes this child, and this final wrapper makes that
-- expectation an explicit fail-closed residual proof (ASVS 14.2.4, 14.2.7).
ALTER FUNCTION public.ple_commit_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    RENAME TO ple_commit_delete_retention_work_before_private_accepted_input;
REVOKE ALL ON FUNCTION public.ple_commit_delete_retention_work_before_private_accepted_input(
    uuid, uuid, uuid, uuid, text, bigint
) FROM PUBLIC, ple_app, ple_automated_grading_broker;
CREATE FUNCTION public.ple_commit_delete_retention_work(
    p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint
) RETURNS boolean LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v_committed boolean;
BEGIN
    v_committed := public.ple_commit_delete_retention_work_before_private_accepted_input(
        p_tenant, p_job, p_token, p_course, p_stage, p_generation
    );
    IF NOT v_committed THEN
        RETURN false;
    END IF;
    IF p_stage <> 'deleteStudentRecords' THEN
        RETURN true;
    END IF;
    RETURN NOT EXISTS (
        SELECT 1 FROM public.accepted_submission_private_response
         WHERE tenant_id = p_tenant AND course_id = p_course
    );
END $$;
ALTER FUNCTION public.ple_commit_delete_retention_work_before_private_accepted_input(
    uuid, uuid, uuid, uuid, text, bigint
) OWNER TO ple_retention_broker;
ALTER FUNCTION public.ple_commit_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_commit_delete_retention_work(
    uuid, uuid, uuid, uuid, text, bigint
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_commit_delete_retention_work(
    uuid, uuid, uuid, uuid, text, bigint
) TO ple_app;

-- This source-proven unused mutation authority can be removed now without
-- disturbing the legacy read paths that WP-P2 will deliberately replace.
REVOKE UPDATE, DELETE ON public.submission FROM ple_app;

DO $$
BEGIN
    -- ASVS 8.2.1-8.2.3: acceptance owns an intentionally narrow timing read.
    -- The broker has neither a general table grant nor a mutation path for
    -- either policy relation; its two tenant-scoped SELECT policies are part
    -- of the capability contract above.
    IF has_table_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_current', 'SELECT')
       OR has_table_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_receipt', 'SELECT')
       OR EXISTS (
            SELECT 1
              FROM unnest(ARRAY[
                  'public.attempt_effective_policy_current',
                  'public.attempt_effective_policy_receipt'
              ]) AS relation_name
              CROSS JOIN LATERAL unnest(ARRAY['INSERT', 'UPDATE', 'DELETE', 'TRUNCATE'])
                  AS mutation_privilege
             WHERE has_table_privilege(
                 'ple_automated_grading_broker', relation_name, mutation_privilege
             )
       )
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_current', 'tenant_id', 'SELECT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_current', 'attempt_id', 'SELECT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_current', 'attempt_occurred_at', 'SELECT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_current', 'assignment_id', 'SELECT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_current', 'course_id', 'SELECT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_current', 'receipt_generation', 'SELECT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_receipt', 'tenant_id', 'SELECT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_receipt', 'attempt_id', 'SELECT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_receipt', 'receipt_generation', 'SELECT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_receipt', 'attempt_occurred_at', 'SELECT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_receipt', 'assignment_id', 'SELECT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_receipt', 'course_id', 'SELECT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_receipt', 'auto_submit_at', 'SELECT')
       OR NOT has_column_privilege('ple_automated_grading_broker',
            'public.attempt_effective_policy_receipt', 'sealed_at', 'SELECT')
       OR EXISTS (
            SELECT 1
              FROM pg_catalog.pg_attribute AS attribute
              CROSS JOIN LATERAL pg_catalog.aclexplode(attribute.attacl) AS privilege
             WHERE attribute.attrelid IN (
                       'public.attempt_effective_policy_current'::regclass,
                       'public.attempt_effective_policy_receipt'::regclass
                   )
               AND attribute.attnum > 0
               AND NOT attribute.attisdropped
               AND privilege.grantee = 'ple_automated_grading_broker'::regrole
               AND (
                    privilege.privilege_type IN ('INSERT', 'UPDATE', 'REFERENCES')
                    OR (attribute.attrelid = 'public.attempt_effective_policy_current'::regclass
                        AND privilege.privilege_type = 'SELECT'
                        AND attribute.attname NOT IN (
                            'tenant_id', 'attempt_id', 'attempt_occurred_at', 'assignment_id',
                            'course_id', 'receipt_generation'
                        ))
                    OR (attribute.attrelid = 'public.attempt_effective_policy_receipt'::regclass
                        AND privilege.privilege_type = 'SELECT'
                        AND attribute.attname NOT IN (
                            'tenant_id', 'attempt_id', 'receipt_generation', 'attempt_occurred_at',
                            'assignment_id', 'course_id', 'auto_submit_at', 'sealed_at'
                        ))
               )
       )
       OR NOT EXISTS (
            SELECT 1 FROM pg_catalog.pg_policy AS policy
             WHERE policy.polrelid = 'public.attempt_effective_policy_current'::regclass
               AND policy.polname = 'automated_grading_broker_effective_policy_current_select'
               AND policy.polcmd = 'r'
               AND policy.polroles = ARRAY['ple_automated_grading_broker'::regrole]::oid[]
       )
       OR NOT EXISTS (
            SELECT 1 FROM pg_catalog.pg_policy AS policy
             WHERE policy.polrelid = 'public.attempt_effective_policy_receipt'::regclass
               AND policy.polname = 'automated_grading_broker_effective_policy_receipt_select'
               AND policy.polcmd = 'r'
               AND policy.polroles = ARRAY['ple_automated_grading_broker'::regrole]::oid[]
       )
    THEN
        RAISE EXCEPTION 'accepted-submission timing-witness authority is unsafe';
    END IF;

    -- ASVS 8.2.1-8.2.3 and 14.2.4: this is the complete direct table
    -- authority matrix. The broker accepts, the reader owns loader reads, and
    -- retention alone deletes; every other application role has no child ACL.
    IF NOT has_table_privilege('ple_automated_grading_broker',
            'public.accepted_submission_private_response', 'SELECT')
       OR NOT has_table_privilege('ple_automated_grading_broker',
            'public.accepted_submission_private_response', 'INSERT')
       OR has_table_privilege('ple_automated_grading_broker',
            'public.accepted_submission_private_response', 'UPDATE')
       OR has_table_privilege('ple_automated_grading_broker',
            'public.accepted_submission_private_response', 'DELETE')
       OR NOT has_column_privilege('ple_accepted_submission_execution_reader',
            'public.accepted_submission_private_response', 'tenant_id', 'SELECT')
       OR NOT has_column_privilege('ple_accepted_submission_execution_reader',
            'public.accepted_submission_private_response', 'course_id', 'SELECT')
       OR NOT has_column_privilege('ple_accepted_submission_execution_reader',
            'public.accepted_submission_private_response', 'attempt_id', 'SELECT')
       OR NOT has_column_privilege('ple_accepted_submission_execution_reader',
            'public.accepted_submission_private_response', 'submission_id', 'SELECT')
       OR NOT has_column_privilege('ple_accepted_submission_execution_reader',
            'public.accepted_submission_private_response', 'submission_occurred_at', 'SELECT')
       OR NOT has_column_privilege('ple_accepted_submission_execution_reader',
            'public.accepted_submission_private_response', 'response_canonical_json', 'SELECT')
       OR NOT has_column_privilege('ple_accepted_submission_execution_reader',
            'public.accepted_submission_private_response', 'response_sha256', 'SELECT')
       OR has_table_privilege('ple_accepted_submission_execution_reader',
            'public.accepted_submission_private_response', 'INSERT')
       OR has_table_privilege('ple_accepted_submission_execution_reader',
            'public.accepted_submission_private_response', 'UPDATE')
       OR has_table_privilege('ple_accepted_submission_execution_reader',
            'public.accepted_submission_private_response', 'DELETE')
       OR NOT has_table_privilege('ple_retention_broker',
            'public.accepted_submission_private_response', 'SELECT')
       OR NOT has_table_privilege('ple_retention_broker',
            'public.accepted_submission_private_response', 'DELETE')
       OR has_table_privilege('ple_retention_broker',
            'public.accepted_submission_private_response', 'INSERT')
       OR has_table_privilege('ple_retention_broker',
            'public.accepted_submission_private_response', 'UPDATE')
       OR EXISTS (
            -- PUBLIC is ACL grantee zero, not an ordinary pg_roles entry.
            SELECT 1
              FROM pg_catalog.pg_class AS relation
              CROSS JOIN LATERAL pg_catalog.aclexplode(COALESCE(
                  relation.relacl,
                  pg_catalog.acldefault('r', relation.relowner)
              )) AS privilege
             WHERE relation.oid = 'public.accepted_submission_private_response'::regclass
               AND privilege.grantee = 0
               AND privilege.privilege_type IN ('SELECT', 'INSERT', 'UPDATE', 'DELETE')
       )
       OR EXISTS (
            SELECT 1
              FROM unnest(ARRAY[
                  'ple_app', 'ple_auth', 'ple_student', 'ple_grader',
                  'ple_grading_reader', 'ple_queue_broker',
                  'ple_accepted_submission_execution'
              ]) AS unrelated(role_name)
             WHERE has_table_privilege(role_name,
                       'public.accepted_submission_private_response', 'SELECT')
                OR has_table_privilege(role_name,
                       'public.accepted_submission_private_response', 'INSERT')
                OR has_table_privilege(role_name,
                       'public.accepted_submission_private_response', 'UPDATE')
                OR has_table_privilege(role_name,
                       'public.accepted_submission_private_response', 'DELETE')
       )
    THEN
        RAISE EXCEPTION 'accepted-submission private child authority is unsafe';
    END IF;

    IF has_function_privilege('public',
            'public.ple_load_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,bigint)',
            'EXECUTE')
       OR NOT has_function_privilege('ple_accepted_submission_execution',
            'public.ple_load_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,bigint)',
            'EXECUTE')
       OR EXISTS (
            SELECT 1
              FROM unnest(ARRAY[
                  'ple_app', 'ple_auth', 'ple_student', 'ple_grader',
                  'ple_grading_reader', 'ple_queue_broker',
                  'ple_automated_grading_broker', 'ple_retention_broker'
              ]) AS unrelated(role_name)
             WHERE has_function_privilege(role_name,
                    'public.ple_load_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,bigint)',
                    'EXECUTE')
       )
       OR has_table_privilege('ple_accepted_submission_execution',
            'public.accepted_submission_private_response', 'SELECT')
       OR has_table_privilege('ple_accepted_submission_execution',
            'public.accepted_submission_private_response', 'INSERT')
       OR has_table_privilege('ple_accepted_submission_execution',
            'public.accepted_submission_private_response', 'UPDATE')
       OR has_table_privilege('ple_accepted_submission_execution',
            'public.accepted_submission_private_response', 'DELETE')
       OR pg_catalog.pg_has_role('ple_app', 'ple_accepted_submission_execution', 'MEMBER')
       OR pg_catalog.pg_has_role('ple_app', 'ple_accepted_submission_execution', 'USAGE')
       OR has_schema_privilege('ple_accepted_submission_execution', 'public', 'CREATE')
       OR has_schema_privilege('ple_accepted_submission_execution_reader', 'public', 'CREATE')
       OR has_table_privilege('ple_accepted_submission_execution_reader',
            'public.issued_attempt_private_execution', 'UPDATE')
       OR has_table_privilege('ple_accepted_submission_execution_reader',
            'public.submission', 'UPDATE')
       OR has_table_privilege('ple_accepted_submission_execution_reader',
            'public.worker_job', 'UPDATE')
    THEN
        RAISE EXCEPTION 'accepted-submission execution-loader authority is unsafe';
    END IF;

    IF NOT has_function_privilege('ple_app',
            'public.ple_accept_automated_submission_v1(uuid,uuid,uuid,uuid,uuid,text,text,uuid)',
            'EXECUTE')
       OR has_function_privilege('public',
            'public.ple_accept_automated_submission_v1(uuid,uuid,uuid,uuid,uuid,text,text,uuid)',
            'EXECUTE')
       OR EXISTS (
            SELECT 1
              FROM unnest(ARRAY[
                  'ple_auth', 'ple_student', 'ple_grader', 'ple_grading_reader',
                  'ple_queue_broker', 'ple_accepted_submission_execution',
                  'ple_accepted_submission_execution_reader', 'ple_retention_broker'
              ]) AS unrelated(role_name)
             WHERE has_function_privilege(role_name,
                    'public.ple_accept_automated_submission_v1(uuid,uuid,uuid,uuid,uuid,text,text,uuid)',
                    'EXECUTE')
       )
       OR has_function_privilege('public',
            'public.ple_forbid_accepted_submission_mutation()', 'EXECUTE')
       OR has_function_privilege('ple_app',
            'public.ple_forbid_accepted_submission_mutation()', 'EXECUTE')
       OR NOT EXISTS (
            SELECT 1 FROM pg_catalog.pg_class AS relation
             WHERE relation.oid = 'public.accepted_submission_private_response'::regclass
               AND relation.relrowsecurity AND relation.relforcerowsecurity
       ) OR NOT EXISTS (
            SELECT 1 FROM pg_catalog.pg_policy AS policy
             WHERE policy.polrelid = 'public.accepted_submission_private_response'::regclass
               AND policy.polname = 'accepted_private_response_broker_select'
               AND policy.polcmd = 'r'
               AND policy.polroles = ARRAY['ple_automated_grading_broker'::regrole]::oid[]
       ) OR NOT EXISTS (
            SELECT 1 FROM pg_catalog.pg_policy AS policy
             WHERE policy.polrelid = 'public.accepted_submission_private_response'::regclass
               AND policy.polname = 'accepted_private_response_broker_insert'
               AND policy.polcmd = 'a'
               AND policy.polroles = ARRAY['ple_automated_grading_broker'::regrole]::oid[]
       ) OR NOT EXISTS (
            SELECT 1 FROM pg_catalog.pg_policy AS policy
             WHERE policy.polrelid = 'public.accepted_submission_private_response'::regclass
               AND policy.polname = 'accepted_private_response_reader'
               AND policy.polcmd = 'r'
               AND policy.polroles = ARRAY['ple_accepted_submission_execution_reader'::regrole]::oid[]
       ) OR NOT EXISTS (
            SELECT 1 FROM pg_catalog.pg_policy AS policy
             WHERE policy.polrelid = 'public.accepted_submission_private_response'::regclass
               AND policy.polname = 'accepted_private_response_retention'
               AND policy.polcmd = 'd'
               AND policy.polroles = ARRAY['ple_retention_broker'::regrole]::oid[]
       ) OR NOT EXISTS (
            SELECT 1 FROM pg_catalog.pg_trigger AS trigger
             WHERE trigger.tgname = 'accepted_submission_private_response_retention_fence'
               AND trigger.tgrelid = 'public.accepted_submission_private_response'::regclass
               AND trigger.tgfoid = 'public.ple_fence_learner_record_write()'::regprocedure
       )
    THEN
        RAISE EXCEPTION 'accepted-submission private boundary or retention fence is unsafe';
    END IF;
END $$;

COMMIT;
