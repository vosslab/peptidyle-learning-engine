BEGIN;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'ple_accepted_submission_execution_worker') THEN
        CREATE ROLE ple_accepted_submission_execution_worker
            NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
    END IF;
END $$;
ALTER ROLE ple_accepted_submission_execution_worker
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
REVOKE ALL ON SCHEMA public FROM ple_accepted_submission_execution_worker;
GRANT USAGE ON SCHEMA public TO ple_accepted_submission_execution_worker;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM public.submission_receipt_snapshot)
       OR EXISTS (SELECT 1 FROM public.attempt_feedback) THEN
        RAISE EXCEPTION 'W4 canonical evidence requires empty receipt and feedback tables; run devel/dist_clean.sh'
            USING ERRCODE = '55000';
    END IF;
END $$;
ALTER TABLE public.grading_execution ADD COLUMN active_worker_id uuid;
ALTER TABLE public.submission_evaluation
    ADD COLUMN automated_result_canonical_json text,
    ADD COLUMN automated_result_sha256 character(64),
    ADD COLUMN automated_result_canonical_json_version smallint,
    ADD CONSTRAINT submission_evaluation_automated_result_pair_check CHECK (
        (automated_result_canonical_json IS NULL) = (automated_result_sha256 IS NULL)
        AND (automated_result_canonical_json IS NULL) = (automated_result_canonical_json_version IS NULL)
    ),
    ADD CONSTRAINT submission_evaluation_automated_result_size_check CHECK (
        automated_result_canonical_json IS NULL
        OR octet_length(automated_result_canonical_json) BETWEEN 1 AND 4096
    ),
    ADD CONSTRAINT submission_evaluation_automated_result_sha256_check CHECK (
        automated_result_sha256 IS NULL OR (automated_result_sha256 ~ '^[0-9a-f]{64}$'
            AND automated_result_canonical_json_version = 1
            AND automated_result_sha256 = encode(pg_catalog.sha256(convert_to(automated_result_canonical_json, 'UTF8')), 'hex'))
    );
ALTER TABLE public.submission_receipt_snapshot
    ADD COLUMN receipt_attempt_payload jsonb,
    ADD COLUMN receipt_attempt_canonical_json text,
    ADD COLUMN receipt_attempt_payload_sha256 character(64),
    ADD COLUMN run_canonical_json text,
    ADD COLUMN summary_canonical_json text,
    ADD COLUMN presentation_canonical_json text,
    ADD COLUMN canonical_json_version smallint NOT NULL,
    ADD CONSTRAINT submission_receipt_snapshot_attempt_payload_pair_check CHECK (
        (receipt_attempt_payload IS NULL) = (receipt_attempt_payload_sha256 IS NULL)
    ),
    ADD CONSTRAINT submission_receipt_snapshot_attempt_payload_shape_check CHECK (
        receipt_attempt_payload IS NULL
        OR (jsonb_typeof(receipt_attempt_payload) = 'object'
            AND receipt_attempt_payload ? 'id'
            AND receipt_attempt_payload ? 'tenant'
            AND receipt_attempt_payload ? 'response'
            AND receipt_attempt_payload ? 'status'
            AND receipt_attempt_payload -> 'response' = 'null'::jsonb
            AND receipt_attempt_payload ->> 'status' IN (
                'submitted', 'auto_submitted', 'needs_manual_grading', 'exempt'
            ))
    ),
    ADD CONSTRAINT submission_receipt_snapshot_attempt_payload_sha256_check CHECK (
        receipt_attempt_payload_sha256 IS NULL
        OR receipt_attempt_payload_sha256 ~ '^[0-9a-f]{64}$'
    ),
    ADD CONSTRAINT submission_receipt_snapshot_canonical_json_version_check CHECK (
        canonical_json_version = 1
    ),
    ADD CONSTRAINT submission_receipt_snapshot_canonical_source_check CHECK (
        (receipt_attempt_canonical_json IS NULL) = (receipt_attempt_payload IS NULL)
        AND (run_canonical_json IS NULL) = (run_payload IS NULL)
        AND (summary_canonical_json IS NULL) = (summary_payload IS NULL)
        AND (presentation_canonical_json IS NULL) = (presentation_payload IS NULL)
        AND (receipt_attempt_canonical_json IS NULL OR octet_length(receipt_attempt_canonical_json) BETWEEN 1 AND 524288)
        AND (run_canonical_json IS NULL OR octet_length(run_canonical_json) BETWEEN 1 AND 524288)
        AND (summary_canonical_json IS NULL OR octet_length(summary_canonical_json) BETWEEN 1 AND 524288)
        AND (presentation_canonical_json IS NULL OR octet_length(presentation_canonical_json) BETWEEN 1 AND 524288)
        AND (receipt_attempt_canonical_json IS NULL OR receipt_attempt_canonical_json::jsonb IS NOT DISTINCT FROM receipt_attempt_payload)
        AND (run_canonical_json IS NULL OR run_canonical_json::jsonb IS NOT DISTINCT FROM run_payload)
        AND (summary_canonical_json IS NULL OR summary_canonical_json::jsonb IS NOT DISTINCT FROM summary_payload)
        AND (presentation_canonical_json IS NULL OR presentation_canonical_json::jsonb IS NOT DISTINCT FROM presentation_payload)
        AND (receipt_attempt_canonical_json IS NULL OR receipt_attempt_payload_sha256 = encode(pg_catalog.sha256(convert_to(receipt_attempt_canonical_json, 'UTF8')), 'hex'))
        AND (run_canonical_json IS NULL OR run_payload_sha256 = encode(pg_catalog.sha256(convert_to(run_canonical_json, 'UTF8')), 'hex'))
        AND (summary_canonical_json IS NULL OR summary_payload_sha256 = encode(pg_catalog.sha256(convert_to(summary_canonical_json, 'UTF8')), 'hex'))
        AND (presentation_canonical_json IS NULL OR presentation_payload_sha256 = encode(pg_catalog.sha256(convert_to(presentation_canonical_json, 'UTF8')), 'hex'))
    );
ALTER TABLE public.submission_receipt_snapshot
    ALTER COLUMN receipt_attempt_payload SET NOT NULL,
    ALTER COLUMN receipt_attempt_canonical_json SET NOT NULL,
    ALTER COLUMN receipt_attempt_payload_sha256 SET NOT NULL,
    ALTER COLUMN run_canonical_json SET NOT NULL,
    ALTER COLUMN summary_canonical_json SET NOT NULL;
ALTER TABLE public.attempt_feedback
    ADD COLUMN content_canonical_json text NOT NULL, ADD COLUMN content_canonical_json_version smallint NOT NULL,
    ADD CONSTRAINT attempt_feedback_content_canonical_json_check CHECK (
        content_canonical_json_version = 1 AND octet_length(content_canonical_json) BETWEEN 1 AND 65536
        AND content_sha256 = encode(pg_catalog.sha256(convert_to(content_canonical_json, 'UTF8')), 'hex')
        AND content_canonical_json::jsonb IS NOT DISTINCT FROM jsonb_build_array(hint, correct_response, rationale)
    );
CREATE FUNCTION public.ple_guard_receipt_attempt_snapshot() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    -- ASVS V2.2/V2.3: check receipt identity and terminal lifecycle at the storage boundary.
    IF NEW.receipt_attempt_payload ->> 'id' IS DISTINCT FROM NEW.attempt_id::text
       OR NEW.receipt_attempt_payload ->> 'tenant' IS DISTINCT FROM NEW.tenant_id::text
       OR NEW.receipt_attempt_payload -> 'response' <> 'null'::jsonb
       OR NEW.receipt_attempt_payload ->> 'status' NOT IN (
            'submitted', 'auto_submitted', 'needs_manual_grading', 'exempt'
       ) THEN
        RAISE EXCEPTION 'receipt attempt snapshot is not answer-free terminal evidence'
            USING ERRCODE = '22023';
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
END $$;
CREATE TRIGGER submission_receipt_snapshot_attempt_guard
    BEFORE INSERT OR UPDATE ON public.submission_receipt_snapshot
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_receipt_attempt_snapshot();
CREATE FUNCTION public.ple_forbid_automated_result_mutation() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF current_user = 'ple_retention_broker' THEN
            RETURN OLD;
        END IF;
        RAISE EXCEPTION 'automated result evidence is retention-deleted only'
            USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'UPDATE' AND OLD.automated_result_canonical_json IS NOT NULL
       AND (NEW.grading_status IS DISTINCT FROM OLD.grading_status
            OR NEW.credit_fraction IS DISTINCT FROM OLD.credit_fraction
            OR NEW.correct IS DISTINCT FROM OLD.correct
            OR NEW.payload IS DISTINCT FROM OLD.payload
            OR NEW.payload_sha256 IS DISTINCT FROM OLD.payload_sha256
            OR NEW.automated_result_canonical_json IS DISTINCT FROM OLD.automated_result_canonical_json
            OR NEW.automated_result_sha256 IS DISTINCT FROM OLD.automated_result_sha256
            OR NEW.automated_result_canonical_json_version IS DISTINCT FROM OLD.automated_result_canonical_json_version
            OR NEW.evaluated_at IS DISTINCT FROM OLD.evaluated_at
            OR NEW.evaluation_revision IS DISTINCT FROM OLD.evaluation_revision) THEN
        RAISE EXCEPTION 'automated result evidence is immutable' USING ERRCODE = '42501';
    END IF;
    RETURN COALESCE(NEW, OLD);
END $$;
CREATE TRIGGER submission_evaluation_automated_result_append_only BEFORE UPDATE OR DELETE ON public.submission_evaluation FOR EACH ROW EXECUTE FUNCTION public.ple_forbid_automated_result_mutation();
CREATE FUNCTION public.ple_guard_accepted_execution_evidence_writer() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    -- ASVS V2.3: a W4 execution has one sealed writer; legacy rows retain their established writer.
    IF EXISTS (SELECT 1 FROM public.grading_execution AS execution
               WHERE execution.tenant_id = NEW.tenant_id AND execution.attempt_id = NEW.attempt_id)
       AND current_user <> 'ple_accepted_submission_execution_worker' THEN
        RAISE EXCEPTION 'accepted-submission evidence requires its sealed worker'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER submission_evaluation_accepted_execution_writer BEFORE INSERT OR UPDATE ON public.submission_evaluation FOR EACH ROW EXECUTE FUNCTION public.ple_guard_accepted_execution_evidence_writer();
CREATE TRIGGER attempt_feedback_accepted_execution_writer BEFORE INSERT ON public.attempt_feedback FOR EACH ROW EXECUTE FUNCTION public.ple_guard_accepted_execution_evidence_writer();
CREATE TRIGGER submission_receipt_snapshot_accepted_execution_writer BEFORE INSERT ON public.submission_receipt_snapshot FOR EACH ROW EXECUTE FUNCTION public.ple_guard_accepted_execution_evidence_writer();
CREATE POLICY accepted_execution_worker_job ON public.worker_job FOR ALL TO ple_accepted_submission_execution_worker USING (true) WITH CHECK (true);
CREATE POLICY accepted_execution_worker_execution ON public.grading_execution FOR ALL TO ple_accepted_submission_execution_worker USING (true) WITH CHECK (true);
CREATE POLICY accepted_execution_worker_evaluation ON public.submission_evaluation FOR ALL TO ple_accepted_submission_execution_worker USING (true) WITH CHECK (true);
CREATE POLICY accepted_execution_worker_receipt ON public.grading_execution_receipt FOR INSERT TO ple_accepted_submission_execution_worker WITH CHECK (true);
CREATE POLICY accepted_execution_worker_operation ON public.grading_operation FOR INSERT TO ple_accepted_submission_execution_worker WITH CHECK (true);
CREATE POLICY accepted_execution_worker_submission ON public.submission FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_idempotency ON public.submission_idempotency FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_attempt ON public.question_attempt FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_attempt_completion ON public.question_attempt FOR UPDATE TO ple_accepted_submission_execution_worker USING (true) WITH CHECK (true);
CREATE POLICY accepted_execution_worker_run ON public.assignment_run FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_enrollment ON public.enrollment FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_assignment ON public.assignment FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_audience ON public.assignment_audience_group FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_items ON public.assignment_item FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_run_items ON public.assignment_run_item FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_selection_groups ON public.assignment_selection_group FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_selection_candidates ON public.assignment_selection_candidate FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_retention ON public.course_retention FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_private_response ON public.accepted_submission_private_response FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_private_execution ON public.issued_attempt_private_execution FOR SELECT TO ple_accepted_submission_execution_worker USING (true);
CREATE POLICY accepted_execution_worker_feedback ON public.attempt_feedback FOR INSERT TO ple_accepted_submission_execution_worker WITH CHECK (true);
CREATE POLICY accepted_execution_worker_receipt_snapshot ON public.submission_receipt_snapshot FOR INSERT TO ple_accepted_submission_execution_worker WITH CHECK (true);
CREATE POLICY accepted_execution_worker_run_completion ON public.assignment_run FOR UPDATE TO ple_accepted_submission_execution_worker USING (true) WITH CHECK (true);
CREATE POLICY accepted_execution_worker_enrollment_completion ON public.enrollment FOR UPDATE TO ple_accepted_submission_execution_worker USING (true) WITH CHECK (true);
CREATE POLICY accepted_execution_worker_summary_completion ON public.student_assignment_summary FOR UPDATE TO ple_accepted_submission_execution_worker USING (true) WITH CHECK (true);
-- ASVS V8.2.1/V8.2.3/V8.3.1: reset and attest the sealed definer's exact server-side authority.
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM ple_accepted_submission_execution_worker;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM ple_accepted_submission_execution_worker;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA public FROM ple_accepted_submission_execution_worker;
GRANT SELECT, UPDATE (state, lease_token, lease_expires_at, attempt_count, last_error, completed_at, available_at) ON public.worker_job TO ple_accepted_submission_execution_worker;
GRANT SELECT, UPDATE (state, active_worker_id, retry_count, updated_at) ON public.grading_execution TO ple_accepted_submission_execution_worker;
GRANT SELECT, UPDATE (grading_status, credit_fraction, correct, payload, payload_sha256, automated_result_canonical_json, automated_result_sha256, automated_result_canonical_json_version, evaluated_at, evaluation_revision) ON public.submission_evaluation TO ple_accepted_submission_execution_worker;
GRANT INSERT ON public.grading_execution_receipt, public.grading_operation TO ple_accepted_submission_execution_worker;
GRANT SELECT ON public.submission, public.submission_idempotency, public.question_attempt, public.assignment_run, public.enrollment, public.assignment, public.assignment_audience_group, public.assignment_item, public.assignment_run_item, public.assignment_selection_group, public.assignment_selection_candidate, public.course_retention, public.accepted_submission_private_response, public.issued_attempt_private_execution TO ple_accepted_submission_execution_worker;
GRANT UPDATE (attempt_status, submitted_at, payload, payload_sha256) ON public.question_attempt TO ple_accepted_submission_execution_worker;
GRANT INSERT ON public.attempt_feedback, public.submission_receipt_snapshot TO ple_accepted_submission_execution_worker;
GRANT UPDATE (completed_at, payload, payload_sha256) ON public.assignment_run TO ple_accepted_submission_execution_worker;
GRANT UPDATE (first_completed_at, current_grade_run_id, best_grade_run_id) ON public.enrollment TO ple_accepted_submission_execution_worker;
GRANT UPDATE (current_score, best_score, latest_score, completed_run_count, total_question_attempts, last_activity_at, updated_at) ON public.student_assignment_summary TO ple_accepted_submission_execution_worker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(), public.ple_enqueue_assignment_recalculation(uuid, uuid, uuid, integer), public.ple_record_question_statistics(uuid, uuid, uuid, uuid, uuid, uuid, double precision, bigint, bigint, double precision, bytea) TO ple_accepted_submission_execution_worker;
REVOKE ALL ON public.submission_evaluation FROM ple_app;
GRANT SELECT (tenant_id, attempt_id, submission_id, grading_status, credit_fraction, correct, payload, payload_sha256, evaluated_at, course_id, evaluation_revision) ON public.submission_evaluation TO ple_app;
GRANT INSERT (tenant_id, attempt_id, submission_id, grading_status, credit_fraction,
    correct, payload, payload_sha256, evaluated_at, course_id, evaluation_revision)
    ON public.submission_evaluation TO ple_app;
GRANT UPDATE (submission_id, grading_status, credit_fraction, correct, payload, payload_sha256,
    evaluated_at, evaluation_revision) ON public.submission_evaluation TO ple_app;
REVOKE ALL ON FUNCTION public.ple_guard_receipt_attempt_snapshot(),
    public.ple_forbid_automated_result_mutation(), public.ple_guard_accepted_execution_evidence_writer()
    FROM PUBLIC, ple_app, ple_accepted_submission_execution;
CREATE VIEW public.ple_accepted_submission_execution_witness_v1
    WITH (security_invoker = true) AS
SELECT execution.tenant_id, execution.attempt_id, execution.submission_id,
    execution.submission_occurred_at, execution.course_id, execution.execution_generation,
    execution.current_job_id, execution.state AS execution_state, execution.active_worker_id,
    job.state AS job_state, job.lease_token, job.lease_expires_at,
    job.attempt_count, job.max_attempts, job.available_at,
    evaluation.grading_status, evaluation.automated_result_canonical_json,
    evaluation.automated_result_sha256, attempt.run_id, attempt.assignment_position,
    attempt.occurred_at AS attempt_occurred_at,
    assignment.assignment_id, assignment.revision AS assignment_revision,
    enrollment.enrollment_id, retention.lifecycle AS retention_lifecycle,
    accepted.course_id AS accepted_course_id, accepted.accepted_actor_id,
    accepted.idempotency_key AS accepted_idempotency_key, accepted.request_sha256,
    floor(extract(epoch FROM accepted.submitted_at) * 1000)::bigint AS accepted_millis,
    response.response_canonical_json, attempt.payload AS attempt_payload,
    attempt.payload_sha256 AS attempt_payload_sha256,
    attempt.presentation_descriptor_version, attempt.presentation_nonce,
    attempt.presentation_digest, attempt.presentation_capability,
    attempt.presentation_payload, attempt.presentation_payload_sha256,
    (attempt.presentation_payload IS NOT NULL) AS presentation_required,
    attempt.grading_envelope_payload, attempt.grading_envelope_payload_sha256,
    attempt.issued_question_snapshot_payload, attempt.issued_question_snapshot_payload_sha256,
    private.flat_required, private.flat_payload, private.flat_payload_sha256,
    private.webwork_required, private.webwork_payload, private.webwork_payload_sha256,
    private.webwork_replay_payload, private.webwork_replay_payload_sha256,
    private.qti_required, private.qti_payload, private.qti_payload_sha256,
    run.payload AS run_payload, run.payload_sha256 AS run_payload_sha256,
    run.completed_at AS run_completed_at, enrollment.first_completed_at,
    enrollment.current_grade_run_id, enrollment.best_grade_run_id,
    summary.current_score AS summary_current_score, summary.best_score AS summary_best_score,
    summary.latest_score AS summary_latest_score, summary.completed_run_count,
    summary.total_question_attempts,
    floor(extract(epoch FROM summary.last_activity_at) * 1000)::bigint AS summary_last_activity_at_millis,
    assignment.scoring_generation
  FROM public.grading_execution AS execution
  JOIN public.worker_job AS job
    ON (job.tenant_id, job.job_id) = (execution.tenant_id, execution.current_job_id)
  JOIN public.submission_evaluation AS evaluation
    ON (evaluation.tenant_id, evaluation.attempt_id, evaluation.submission_id)
     = (execution.tenant_id, execution.attempt_id, execution.submission_id)
  JOIN public.question_attempt AS attempt
    ON (attempt.tenant_id, attempt.attempt_id) = (execution.tenant_id, execution.attempt_id)
  JOIN public.submission AS accepted_submission
    ON accepted_submission.tenant_id = execution.tenant_id
   AND accepted_submission.attempt_id = execution.attempt_id
   AND accepted_submission.submission_id = execution.submission_id
   AND accepted_submission.occurred_at = execution.submission_occurred_at
  JOIN public.submission_idempotency AS accepted
    ON accepted.tenant_id = execution.tenant_id AND accepted.attempt_id = execution.attempt_id
   AND accepted.submission_id = execution.submission_id
   AND accepted.submission_occurred_at = execution.submission_occurred_at
  JOIN public.accepted_submission_private_response AS response
    ON response.tenant_id = execution.tenant_id AND response.course_id = execution.course_id
   AND response.attempt_id = execution.attempt_id AND response.submission_id = execution.submission_id
   AND response.submission_occurred_at = execution.submission_occurred_at
  JOIN public.issued_attempt_private_execution AS private
    ON private.tenant_id = attempt.tenant_id AND private.attempt_id = attempt.attempt_id
   AND private.attempt_occurred_at = attempt.occurred_at
  JOIN public.assignment_run AS run
    ON (run.tenant_id, run.run_id) = (attempt.tenant_id, attempt.run_id)
  JOIN public.enrollment AS enrollment
    ON (enrollment.tenant_id, enrollment.enrollment_id) = (run.tenant_id, run.enrollment_id)
  JOIN public.assignment AS assignment
    ON (assignment.tenant_id, assignment.assignment_id) = (enrollment.tenant_id, enrollment.assignment_id)
  JOIN public.student_assignment_summary AS summary
    ON (summary.tenant_id, summary.enrollment_id) = (enrollment.tenant_id, enrollment.enrollment_id)
  JOIN public.course_retention AS retention
    ON (retention.tenant_id, retention.course_id) = (assignment.tenant_id, assignment.course_id)
 WHERE execution.course_id = assignment.course_id
   AND attempt.course_id = assignment.course_id
   AND accepted.request_contract_version = 2 AND accepted.accepted_actor_id IS NOT NULL
   AND accepted.course_id = assignment.course_id AND accepted_submission.course_id = assignment.course_id
   AND accepted_submission.idempotency_key = accepted.idempotency_key
   AND accepted.request_sha256 = response.response_sha256
   AND response.response_sha256 = encode(pg_catalog.sha256(convert_to(response.response_canonical_json, 'UTF8')), 'hex')
   AND accepted_submission.payload_sha256 = encode(pg_catalog.sha256(convert_to(
       '{"kind":"acceptedPrivateResponseV1"}'::jsonb::text, 'UTF8'
   )), 'hex')
   AND accepted.payload_sha256 = encode(pg_catalog.sha256(convert_to(
       '{"kind":"acceptedPrivateResponseV1"}'::jsonb::text, 'UTF8'
   )), 'hex');
ALTER VIEW public.ple_accepted_submission_execution_witness_v1
    OWNER TO ple_accepted_submission_execution_worker;
REVOKE ALL ON public.ple_accepted_submission_execution_witness_v1 FROM PUBLIC,
    ple_app, ple_accepted_submission_execution;
CREATE FUNCTION public.ple_claim_accepted_submission_execution_v1(
    p_lease_token uuid, p_worker_id uuid, p_lease_seconds integer
) RETURNS TABLE(
    tenant_id uuid, worker_job_id uuid, worker_lease_token uuid,
    submission_id uuid, execution_generation bigint, worker_id uuid
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v_execution public.grading_execution%ROWTYPE;
BEGIN
    IF p_lease_token IS NULL OR p_worker_id IS NULL
       OR p_lease_seconds NOT BETWEEN 1 AND 900 THEN
        RAISE EXCEPTION 'accepted-submission claim arguments are invalid' USING ERRCODE = '22023';
    END IF;
    -- Exhausted work becomes terminal evidence before another worker observes it.
    SELECT execution.* INTO v_execution
      FROM public.ple_accepted_submission_execution_witness_v1 AS witness
      JOIN public.grading_execution AS execution
        ON (execution.tenant_id, execution.attempt_id) = (witness.tenant_id, witness.attempt_id)
      JOIN public.worker_job AS job
        ON (job.tenant_id, job.job_id) = (witness.tenant_id, witness.current_job_id)
      JOIN public.submission_evaluation AS evaluation
        ON (evaluation.tenant_id, evaluation.attempt_id) = (witness.tenant_id, witness.attempt_id)
     WHERE witness.execution_state = 'running' AND witness.job_state = 'leased'
       AND witness.lease_expires_at <= transaction_timestamp()
       AND witness.attempt_count >= witness.max_attempts
       AND witness.retention_lifecycle = 'active'
       AND witness.grading_status = 'automated_pending'
       AND witness.automated_result_canonical_json IS NULL
       AND witness.automated_result_sha256 IS NULL
     FOR UPDATE OF execution, job, evaluation SKIP LOCKED LIMIT 1;
    IF FOUND THEN
        UPDATE public.worker_job SET state = 'dead', lease_token = NULL, lease_expires_at = NULL,
            last_error = 'timed_out', completed_at = transaction_timestamp()
         WHERE tenant_id = v_execution.tenant_id AND job_id = v_execution.current_job_id;
        UPDATE public.grading_execution SET state = 'exception', active_worker_id = NULL,
            updated_at = transaction_timestamp()
         WHERE tenant_id = v_execution.tenant_id AND attempt_id = v_execution.attempt_id;
        UPDATE public.submission_evaluation SET grading_status = 'automated_exception',
            evaluated_at = transaction_timestamp(), evaluation_revision = evaluation_revision + 1
         WHERE tenant_id = v_execution.tenant_id AND attempt_id = v_execution.attempt_id
           AND submission_id = v_execution.submission_id AND grading_status = 'automated_pending';
        INSERT INTO public.grading_execution_receipt
            (tenant_id, receipt_id, attempt_id, submission_id, submission_occurred_at, course_id,
             execution_generation, resulting_state, worker_id)
        VALUES (v_execution.tenant_id, gen_random_uuid(), v_execution.attempt_id,
            v_execution.submission_id, v_execution.submission_occurred_at, v_execution.course_id,
            v_execution.execution_generation, 'exception', NULL);
        INSERT INTO public.grading_operation
            (tenant_id, attempt_id, submission_id, submission_occurred_at, assignment_id, course_id,
             target_kind, reason, state, next_action)
        SELECT v_execution.tenant_id, v_execution.attempt_id, v_execution.submission_id,
            v_execution.submission_occurred_at, enrollment.assignment_id, v_execution.course_id,
            'submission', 'retry_exhausted', 'actionable', 'retry'
          FROM public.question_attempt AS attempt
          JOIN public.assignment_run AS run ON (run.tenant_id, run.run_id) = (attempt.tenant_id, attempt.run_id)
          JOIN public.enrollment AS enrollment ON (enrollment.tenant_id, enrollment.enrollment_id) = (run.tenant_id, run.enrollment_id)
         WHERE attempt.tenant_id = v_execution.tenant_id AND attempt.attempt_id = v_execution.attempt_id
        ON CONFLICT DO NOTHING;
    END IF;
    SELECT execution.* INTO v_execution
      FROM public.ple_accepted_submission_execution_witness_v1 AS witness
      JOIN public.grading_execution AS execution
        ON (execution.tenant_id, execution.attempt_id) = (witness.tenant_id, witness.attempt_id)
      JOIN public.worker_job AS job
        ON (job.tenant_id, job.job_id) = (witness.tenant_id, witness.current_job_id)
      JOIN public.submission_evaluation AS evaluation
        ON (evaluation.tenant_id, evaluation.attempt_id) = (witness.tenant_id, witness.attempt_id)
     WHERE witness.retention_lifecycle = 'active'
       AND witness.grading_status = 'automated_pending'
       AND witness.automated_result_canonical_json IS NULL
       AND witness.automated_result_sha256 IS NULL
       AND ((witness.job_state = 'ready' AND witness.available_at <= transaction_timestamp()
             AND witness.execution_state IN ('ready', 'retry_wait'))
            OR (witness.job_state = 'leased' AND witness.lease_expires_at <= transaction_timestamp()
                AND witness.attempt_count < witness.max_attempts AND witness.execution_state = 'running'))
     ORDER BY witness.available_at, witness.current_job_id
     FOR UPDATE OF execution, job, evaluation SKIP LOCKED LIMIT 1;
    IF NOT FOUND THEN RETURN; END IF;
    UPDATE public.worker_job SET state = 'leased', lease_token = p_lease_token,
        lease_expires_at = transaction_timestamp() + make_interval(secs => p_lease_seconds),
        attempt_count = attempt_count + 1, last_error = NULL, completed_at = NULL
     WHERE tenant_id = v_execution.tenant_id AND job_id = v_execution.current_job_id;
    UPDATE public.grading_execution SET state = 'running', active_worker_id = p_worker_id,
        updated_at = transaction_timestamp()
     WHERE tenant_id = v_execution.tenant_id AND attempt_id = v_execution.attempt_id;
    INSERT INTO public.grading_execution_receipt
        (tenant_id, receipt_id, attempt_id, submission_id, submission_occurred_at, course_id,
         execution_generation, resulting_state, worker_id)
    VALUES (v_execution.tenant_id, gen_random_uuid(), v_execution.attempt_id,
        v_execution.submission_id, v_execution.submission_occurred_at, v_execution.course_id,
        v_execution.execution_generation, 'running', p_worker_id);
    tenant_id := v_execution.tenant_id; worker_job_id := v_execution.current_job_id;
    worker_lease_token := p_lease_token; submission_id := v_execution.submission_id;
    execution_generation := v_execution.execution_generation; worker_id := p_worker_id;
    RETURN NEXT;
END $$;
CREATE FUNCTION public.ple_load_accepted_submission_execution_v2(
    p_tenant_id uuid, p_worker_job_id uuid, p_lease_token uuid, p_submission_id uuid,
    p_execution_generation bigint, p_worker_id uuid
) RETURNS TABLE(
    worker_job_id uuid, worker_lease_token uuid, execution_generation bigint, worker_id uuid,
    execution_state text, accepted_tenant_id uuid, accepted_course_id uuid,
    accepted_assignment_id uuid, accepted_attempt_id uuid, accepted_submission_id uuid,
    accepted_actor_id uuid, accepted_idempotency_key text, accepted_request_sha256 character(64),
    accepted_millis bigint, response_canonical_json text, attempt_payload jsonb,
    attempt_payload_sha256 character(64), presentation_descriptor_version smallint,
    presentation_nonce bytea, presentation_digest bytea, presentation_capability text,
    presentation_payload jsonb, presentation_payload_sha256 character(64),
    grading_envelope_payload jsonb, grading_envelope_payload_sha256 character(64),
    issued_question_snapshot_payload jsonb, issued_question_snapshot_payload_sha256 character(64),
    flat_required boolean, flat_payload jsonb, flat_payload_sha256 character(64),
    webwork_required boolean, webwork_payload jsonb, webwork_payload_sha256 character(64),
    webwork_replay_payload jsonb, webwork_replay_payload_sha256 character(64),
    qti_required boolean, qti_payload bytea, qti_payload_sha256 character(64)
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_tenant_id IS NULL OR p_worker_job_id IS NULL OR p_lease_token IS NULL
       OR p_submission_id IS NULL OR p_execution_generation IS NULL OR p_execution_generation <= 0
       OR p_worker_id IS NULL OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant() THEN RETURN; END IF;
    RETURN QUERY
    SELECT witness.current_job_id, witness.lease_token, witness.execution_generation,
        p_worker_id, witness.execution_state, witness.tenant_id, witness.accepted_course_id,
        witness.assignment_id, witness.attempt_id, witness.submission_id,
        witness.accepted_actor_id, witness.accepted_idempotency_key, witness.request_sha256,
        witness.accepted_millis, witness.response_canonical_json, witness.attempt_payload,
        witness.attempt_payload_sha256, witness.presentation_descriptor_version,
        witness.presentation_nonce, witness.presentation_digest, witness.presentation_capability,
        witness.presentation_payload, witness.presentation_payload_sha256,
        witness.grading_envelope_payload, witness.grading_envelope_payload_sha256,
        witness.issued_question_snapshot_payload, witness.issued_question_snapshot_payload_sha256,
        witness.flat_required, witness.flat_payload, witness.flat_payload_sha256,
        witness.webwork_required, witness.webwork_payload, witness.webwork_payload_sha256,
        witness.webwork_replay_payload, witness.webwork_replay_payload_sha256,
        witness.qti_required, witness.qti_payload, witness.qti_payload_sha256
      FROM public.ple_accepted_submission_execution_witness_v1 AS witness
     WHERE witness.tenant_id = p_tenant_id AND witness.current_job_id = p_worker_job_id
       AND witness.submission_id = p_submission_id
       AND witness.execution_generation = p_execution_generation
       AND witness.execution_state = 'running' AND witness.active_worker_id = p_worker_id
       AND witness.job_state = 'leased' AND witness.lease_token = p_lease_token
       AND witness.lease_expires_at > transaction_timestamp()
       AND witness.retention_lifecycle = 'active'
       AND witness.grading_status = 'automated_pending'
       AND witness.automated_result_canonical_json IS NULL
       AND witness.automated_result_sha256 IS NULL;
END $$;
CREATE FUNCTION public.ple_lock_accepted_submission_completion_v1(
    p_tenant_id uuid, p_worker_job_id uuid, p_lease_token uuid, p_submission_id uuid,
    p_execution_generation bigint, p_worker_id uuid
) RETURNS TABLE(
    tenant_id uuid, worker_job_id uuid, worker_lease_token uuid,
    submission_id uuid, execution_generation bigint, worker_id uuid,
    attempt_id uuid, assignment_id uuid, assignment_header jsonb,
    assignment_audience_groups jsonb, assignment_items jsonb,
    assignment_selection_groups jsonb, assignment_selection_candidates jsonb,
    enrollment_id uuid, enrollment_user_id uuid, enrollment_student_id uuid, run_id uuid,
    assignment_scoring_generation bigint, accepted_at_millis bigint,
    attempt_payload jsonb, attempt_payload_sha256 character(64),
    presentation_payload jsonb, presentation_payload_sha256 character(64),
    presentation_required boolean, run_payload jsonb, run_payload_sha256 character(64),
    run_completed_at_millis bigint, enrollment_first_completed_at_millis bigint,
    enrollment_current_grade_run_id uuid, enrollment_best_grade_run_id uuid,
    summary_tenant_id uuid, summary_enrollment_id uuid,
    summary_current_score double precision, summary_best_score double precision,
    summary_latest_score double precision, summary_completed_run_count bigint,
    summary_total_question_attempts bigint, summary_last_activity_at_millis bigint,
    same_run_attempts jsonb, run_items jsonb
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v_attempt_id uuid;
    v_enrollment_id uuid;
    v_run_id uuid;
BEGIN
    IF p_tenant_id IS NULL OR p_worker_job_id IS NULL OR p_lease_token IS NULL
       OR p_submission_id IS NULL OR p_execution_generation IS NULL
       OR p_execution_generation <= 0 OR p_worker_id IS NULL
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant() THEN
        RETURN;
    END IF;
    -- ASVS V2.3: lock before reading lifecycle inputs; later snapshots build immutable source.
    SELECT execution.attempt_id, run.enrollment_id, attempt.run_id
      INTO v_attempt_id, v_enrollment_id, v_run_id
      FROM public.grading_execution AS execution
      JOIN public.worker_job AS job
        ON (job.tenant_id, job.job_id) = (execution.tenant_id, execution.current_job_id)
      JOIN public.submission_evaluation AS evaluation
        ON (evaluation.tenant_id, evaluation.attempt_id, evaluation.submission_id)
         = (execution.tenant_id, execution.attempt_id, execution.submission_id)
      JOIN public.question_attempt AS attempt
        ON (attempt.tenant_id, attempt.attempt_id) = (execution.tenant_id, execution.attempt_id)
      JOIN public.submission AS accepted_submission
        ON accepted_submission.tenant_id = execution.tenant_id
       AND accepted_submission.attempt_id = execution.attempt_id
       AND accepted_submission.submission_id = execution.submission_id
       AND accepted_submission.occurred_at = execution.submission_occurred_at
      JOIN public.submission_idempotency AS accepted
        ON accepted.tenant_id = execution.tenant_id AND accepted.attempt_id = execution.attempt_id
       AND accepted.submission_id = execution.submission_id
       AND accepted.submission_occurred_at = execution.submission_occurred_at
      JOIN public.accepted_submission_private_response AS response
        ON response.tenant_id = execution.tenant_id AND response.course_id = execution.course_id
       AND response.attempt_id = execution.attempt_id AND response.submission_id = execution.submission_id
       AND response.submission_occurred_at = execution.submission_occurred_at
      JOIN public.issued_attempt_private_execution AS private
        ON private.tenant_id = attempt.tenant_id AND private.attempt_id = attempt.attempt_id
       AND private.attempt_occurred_at = attempt.occurred_at
      JOIN public.assignment_run AS run
        ON (run.tenant_id, run.run_id) = (attempt.tenant_id, attempt.run_id)
      JOIN public.enrollment AS enrollment
        ON (enrollment.tenant_id, enrollment.enrollment_id) = (run.tenant_id, run.enrollment_id)
      JOIN public.assignment AS assignment
        ON (assignment.tenant_id, assignment.assignment_id) = (enrollment.tenant_id, enrollment.assignment_id)
      JOIN public.student_assignment_summary AS summary
        ON (summary.tenant_id, summary.enrollment_id) = (enrollment.tenant_id, enrollment.enrollment_id)
      JOIN public.course_retention AS retention
        ON (retention.tenant_id, retention.course_id) = (execution.tenant_id, execution.course_id)
     WHERE execution.tenant_id = p_tenant_id
       AND execution.current_job_id = p_worker_job_id
       AND execution.submission_id = p_submission_id
       AND execution.execution_generation = p_execution_generation
       AND execution.state = 'running' AND execution.active_worker_id = p_worker_id
       AND job.state = 'leased' AND job.lease_token = p_lease_token
       AND job.lease_expires_at > transaction_timestamp()
       AND job.payload = jsonb_build_object('kind', 'gradeAcceptedSubmission',
             'attempt', execution.attempt_id::text, 'submission', execution.submission_id::text,
             'execution_generation', execution.execution_generation)
       AND evaluation.grading_status = 'automated_pending'
       AND evaluation.automated_result_canonical_json IS NULL
       AND evaluation.automated_result_sha256 IS NULL
       AND retention.lifecycle = 'active'
       AND execution.course_id = assignment.course_id AND attempt.course_id = assignment.course_id
       AND accepted.request_contract_version = 2 AND accepted.accepted_actor_id IS NOT NULL
       AND accepted.course_id = assignment.course_id AND accepted_submission.course_id = assignment.course_id
       AND accepted_submission.idempotency_key = accepted.idempotency_key
       AND accepted.request_sha256 = response.response_sha256
       AND response.response_sha256 = encode(pg_catalog.sha256(convert_to(response.response_canonical_json, 'UTF8')), 'hex')
       AND accepted_submission.payload_sha256 = encode(pg_catalog.sha256(convert_to(
           '{"kind":"acceptedPrivateResponseV1"}'::jsonb::text, 'UTF8'
       )), 'hex')
       AND accepted.payload_sha256 = encode(pg_catalog.sha256(convert_to(
           '{"kind":"acceptedPrivateResponseV1"}'::jsonb::text, 'UTF8'
       )), 'hex')
    FOR UPDATE OF execution, job, evaluation, attempt, run, enrollment, summary;
    IF NOT FOUND THEN RETURN; END IF;
    RETURN QUERY
    SELECT execution.tenant_id, job.job_id, job.lease_token, execution.submission_id,
        execution.execution_generation, p_worker_id, attempt.attempt_id, assignment.assignment_id,
        jsonb_build_object('assignmentId',assignment.assignment_id,'courseId',assignment.course_id,'title',assignment.title,'lifecycle',assignment.lifecycle,'instructions',assignment.instructions,'completionPolicy',assignment.completion_policy,'completionThreshold',assignment.completion_threshold::text,'attemptSelectionPolicy',assignment.attempt_selection_policy,'continuedPracticePolicy',assignment.continued_practice_policy,'practiceMaxAdditionalRuns',assignment.practice_max_additional_runs,'variationPolicy',assignment.variation_policy,'audienceKind',assignment.audience_kind,'scoreDisclosure',assignment.score_disclosure,'perItemCorrectnessDisclosure',assignment.per_item_correctness_disclosure,'feedbackTextDisclosure',assignment.feedback_text_disclosure,'solutionDisclosure',assignment.solution_disclosure,'classStatisticsDisclosure',assignment.class_statistics_disclosure),
        (SELECT coalesce(jsonb_agg(course_group_id ORDER BY course_group_id),'[]'::jsonb) FROM public.assignment_audience_group WHERE tenant_id=execution.tenant_id AND assignment_id=assignment.assignment_id),
        (SELECT coalesce(jsonb_agg(jsonb_build_object('assignmentItemId',assignment_item_id,'position',position,'problemId',problem_id,'versionId',version_id,'pointsPossible',points_possible::text,'deliveryState',delivery_state,'scoringMode',scoring_mode) ORDER BY position),'[]'::jsonb) FROM public.assignment_item WHERE tenant_id=execution.tenant_id AND assignment_id=assignment.assignment_id),
        (SELECT coalesce(jsonb_agg(jsonb_build_object('selectionGroupId',selection_group_id,'position',position,'drawCount',draw_count,'pointsPerItem',points_per_item::text,'orderingPolicy',ordering_policy,'algorithmVersion',algorithm_version) ORDER BY position),'[]'::jsonb) FROM public.assignment_selection_group WHERE tenant_id=execution.tenant_id AND assignment_id=assignment.assignment_id),
        (SELECT coalesce(jsonb_agg(jsonb_build_object('selectionGroupId',selection_group_id,'candidateId',candidate_id,'position',position,'problemId',problem_id,'versionId',version_id,'deliveryState',delivery_state) ORDER BY selection_group_id,position),'[]'::jsonb) FROM public.assignment_selection_candidate WHERE tenant_id=execution.tenant_id AND assignment_id=assignment.assignment_id),
        enrollment.enrollment_id, enrollment.user_id, enrollment.student_id, run.run_id, assignment.scoring_generation,
        floor(extract(epoch FROM accepted.submitted_at) * 1000)::bigint,
        attempt.payload, attempt.payload_sha256,
        attempt.presentation_payload, attempt.presentation_payload_sha256,
        attempt.presentation_payload IS NOT NULL, run.payload, run.payload_sha256,
        floor(extract(epoch FROM run.completed_at) * 1000)::bigint,
        floor(extract(epoch FROM enrollment.first_completed_at) * 1000)::bigint,
        enrollment.current_grade_run_id, enrollment.best_grade_run_id,
        summary.tenant_id, summary.enrollment_id, summary.current_score, summary.best_score, summary.latest_score,
        summary.completed_run_count, summary.total_question_attempts,
        floor(extract(epoch FROM summary.last_activity_at) * 1000)::bigint,
        (SELECT coalesce(jsonb_agg(jsonb_build_object(
             'attemptId', peer.attempt_id, 'payload', peer.payload,
             'payloadSha256', peer.payload_sha256, 'status', peer.attempt_status,
             'submittedAtMillis', floor(extract(epoch FROM peer.submitted_at) * 1000)::bigint,
             'evaluation', peer_evaluation.payload,
             'evaluationSha256', peer_evaluation.payload_sha256,
             'evaluationStatus', peer_evaluation.grading_status
         ) ORDER BY peer.assignment_position, peer.occurred_at, peer.attempt_id), '[]'::jsonb)
           FROM public.question_attempt AS peer
           LEFT JOIN public.submission_evaluation AS peer_evaluation
             ON (peer_evaluation.tenant_id, peer_evaluation.attempt_id) = (peer.tenant_id, peer.attempt_id)
          WHERE peer.tenant_id = execution.tenant_id AND peer.run_id = run.run_id),
        (SELECT coalesce(jsonb_agg(jsonb_build_object(
             'run', item.run_id, 'assignmentItem', item.assignment_item_id,
             'sourcePosition', item.source_position, 'issuedPosition', item.issued_position,
             'reference', jsonb_build_object('problem', item.problem_id, 'version', item.version_id),
             'statisticsEligible', item.statistics_eligible,
             'selectionGroup', item.selection_group_id, 'selectionSeed', item.selection_seed
         ) ORDER BY item.issued_position), '[]'::jsonb)
           FROM public.assignment_run_item AS item
          WHERE item.tenant_id = execution.tenant_id AND item.run_id = run.run_id)
      FROM public.grading_execution AS execution
      JOIN public.worker_job AS job
        ON (job.tenant_id, job.job_id) = (execution.tenant_id, execution.current_job_id)
      JOIN public.question_attempt AS attempt
        ON (attempt.tenant_id, attempt.attempt_id) = (execution.tenant_id, execution.attempt_id)
      JOIN public.assignment_run AS run
        ON (run.tenant_id, run.run_id) = (attempt.tenant_id, attempt.run_id)
      JOIN public.enrollment AS enrollment
        ON (enrollment.tenant_id, enrollment.enrollment_id) = (run.tenant_id, run.enrollment_id)
      JOIN public.assignment AS assignment
        ON (assignment.tenant_id, assignment.assignment_id) = (enrollment.tenant_id, enrollment.assignment_id)
      JOIN public.student_assignment_summary AS summary
        ON (summary.tenant_id, summary.enrollment_id) = (enrollment.tenant_id, enrollment.enrollment_id)
      JOIN public.submission_idempotency AS accepted
        ON accepted.tenant_id = execution.tenant_id AND accepted.attempt_id = execution.attempt_id
       AND accepted.submission_id = execution.submission_id
       AND accepted.submission_occurred_at = execution.submission_occurred_at
     WHERE execution.tenant_id = p_tenant_id AND execution.attempt_id = v_attempt_id
       AND enrollment.enrollment_id = v_enrollment_id AND run.run_id = v_run_id;
END $$;
CREATE FUNCTION public.ple_commit_accepted_submission_completion_v2(
    p_tenant_id uuid, p_worker_job_id uuid, p_lease_token uuid, p_submission_id uuid,
    p_execution_generation bigint, p_worker_id uuid, p_canonical_json_version smallint, p_evaluation_status text,
    p_evaluation_canonical_json text, p_evaluation_sha256 character(64),
    p_attempt_canonical_json text, p_attempt_payload jsonb, p_attempt_payload_sha256 character(64),
    p_attempt_current_canonical_json text, p_attempt_current_payload_sha256 character(64),
    p_feedback_canonical_json text, p_feedback_content_sha256 character(64),
    p_run_canonical_json text, p_run_payload jsonb, p_run_payload_sha256 character(64),
    p_run_current_canonical_json text, p_run_current_payload_sha256 character(64),
    p_run_completed_at_millis bigint, p_enrollment_first_completed_at_millis bigint,
    p_enrollment_current_grade_run_id uuid, p_enrollment_best_grade_run_id uuid,
    p_summary_canonical_json text, p_summary_payload jsonb, p_summary_payload_sha256 character(64),
    p_presentation_canonical_json text, p_presentation_payload jsonb, p_presentation_payload_sha256 character(64), p_presentation_required boolean,
    p_assignment_item_id uuid,
    p_statistics jsonb, p_expected_scoring_generation bigint,
    p_recalculation_job_id uuid, p_recalculation_max_attempts integer
) RETURNS TABLE(disposition text, resulting_execution_state text, resulting_evaluation_status text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v public.ple_accepted_submission_execution_witness_v1%ROWTYPE;
    r jsonb; feedback jsonb; attempt_source jsonb; run_source jsonb; summary_source jsonb; presentation_source jsonb;
    attempt_current_source jsonb; run_current_source jsonb;
    correct boolean; earned numeric; possible numeric; statistic jsonb;
    summary_current_score double precision; summary_best_score double precision; summary_latest_score double precision;
    summary_completed_run_count bigint; summary_total_question_attempts bigint; summary_last_activity_at_millis bigint;
BEGIN
    IF p_tenant_id IS NULL OR p_worker_job_id IS NULL OR p_lease_token IS NULL OR p_submission_id IS NULL
       OR p_execution_generation IS NULL OR p_execution_generation <= 0 OR p_worker_id IS NULL OR p_canonical_json_version IS DISTINCT FROM 1
       OR p_evaluation_status <> 'graded' OR p_evaluation_canonical_json IS NULL
       OR octet_length(p_evaluation_canonical_json) NOT BETWEEN 1 AND 4096
       OR p_evaluation_sha256 !~ '^[0-9a-f]{64}$' OR p_attempt_canonical_json IS NULL OR p_attempt_payload IS NULL
       OR octet_length(p_attempt_canonical_json) NOT BETWEEN 1 AND 524288 OR p_attempt_payload_sha256 !~ '^[0-9a-f]{64}$'
       OR p_attempt_current_canonical_json IS NULL OR octet_length(p_attempt_current_canonical_json) NOT BETWEEN 1 AND 524288 OR p_attempt_current_payload_sha256 !~ '^[0-9a-f]{64}$'
       OR p_feedback_canonical_json IS NULL OR octet_length(p_feedback_canonical_json) NOT BETWEEN 1 AND 65536 OR p_feedback_content_sha256 !~ '^[0-9a-f]{64}$'
       OR p_run_canonical_json IS NULL OR p_run_payload IS NULL OR octet_length(p_run_canonical_json) NOT BETWEEN 1 AND 524288 OR p_run_payload_sha256 !~ '^[0-9a-f]{64}$'
       OR p_run_current_canonical_json IS NULL OR octet_length(p_run_current_canonical_json) NOT BETWEEN 1 AND 524288 OR p_run_current_payload_sha256 !~ '^[0-9a-f]{64}$'
       OR p_summary_canonical_json IS NULL OR p_summary_payload IS NULL OR octet_length(p_summary_canonical_json) NOT BETWEEN 1 AND 524288 OR p_summary_payload_sha256 !~ '^[0-9a-f]{64}$'
       OR (p_presentation_required AND (p_presentation_canonical_json IS NULL OR p_presentation_payload IS NULL OR octet_length(p_presentation_canonical_json) NOT BETWEEN 1 AND 524288 OR p_presentation_payload_sha256 !~ '^[0-9a-f]{64}$'))
       OR (NOT p_presentation_required AND (p_presentation_canonical_json IS NOT NULL OR p_presentation_payload IS NOT NULL OR p_presentation_payload_sha256 IS NOT NULL))
       OR p_assignment_item_id IS NULL
       OR p_statistics IS NULL OR jsonb_typeof(p_statistics) <> 'array' OR jsonb_array_length(p_statistics) > 1024
       OR p_expected_scoring_generation IS NULL OR p_expected_scoring_generation <= 0
       OR p_recalculation_job_id IS NULL OR p_recalculation_max_attempts NOT BETWEEN 1 AND 20
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'accepted-submission completion arguments are invalid' USING ERRCODE = '22023';
    END IF;
    IF p_evaluation_sha256 IS DISTINCT FROM encode(pg_catalog.sha256(convert_to(p_evaluation_canonical_json, 'UTF8')), 'hex') THEN
        RAISE EXCEPTION 'accepted-submission result digest is invalid' USING ERRCODE = '22023'; END IF;
    BEGIN r := p_evaluation_canonical_json::jsonb; correct := (r->>'correct')::boolean;
        earned := (r->>'pointsEarned')::numeric; possible := (r->>'pointsPossible')::numeric;
    EXCEPTION WHEN invalid_text_representation THEN RAISE EXCEPTION 'accepted-submission result is invalid' USING ERRCODE = '22023'; END;
    IF jsonb_typeof(r) <> 'object' OR NOT (r ?& ARRAY['correct','pointsEarned','pointsPossible'])
       OR r - ARRAY['correct','pointsEarned','pointsPossible'] <> '{}'::jsonb
       OR jsonb_typeof(r->'correct') <> 'boolean' OR jsonb_typeof(r->'pointsEarned') <> 'number'
       OR jsonb_typeof(r->'pointsPossible') <> 'number' OR earned < 0 OR possible <= 0 OR earned > possible THEN
        RAISE EXCEPTION 'accepted-submission result shape is invalid' USING ERRCODE = '22023'; END IF;
    BEGIN attempt_source:=p_attempt_canonical_json::jsonb; attempt_current_source:=p_attempt_current_canonical_json::jsonb; run_source:=p_run_canonical_json::jsonb; run_current_source:=p_run_current_canonical_json::jsonb; summary_source:=p_summary_canonical_json::jsonb; presentation_source:=CASE WHEN p_presentation_required THEN p_presentation_canonical_json::jsonb END; feedback:=p_feedback_canonical_json::jsonb;
    EXCEPTION WHEN invalid_text_representation THEN RAISE EXCEPTION 'accepted-submission canonical evidence is invalid' USING ERRCODE='22023'; END;
    IF jsonb_typeof(feedback)<>'array' OR jsonb_array_length(feedback)<>3 OR p_feedback_content_sha256 IS DISTINCT FROM encode(pg_catalog.sha256(convert_to(p_feedback_canonical_json,'UTF8')),'hex') THEN RAISE EXCEPTION 'feedback evidence is invalid' USING ERRCODE='22023'; END IF;
    IF attempt_source IS DISTINCT FROM p_attempt_payload OR attempt_current_source IS DISTINCT FROM p_attempt_payload OR run_source IS DISTINCT FROM p_run_payload OR run_current_source IS DISTINCT FROM p_run_payload OR summary_source IS DISTINCT FROM p_summary_payload OR presentation_source IS DISTINCT FROM p_presentation_payload THEN RAISE EXCEPTION 'accepted-submission canonical source does not match projection' USING ERRCODE='22023'; END IF;
    IF jsonb_typeof(summary_source)<>'object' OR NOT summary_source ?& ARRAY['tenant','enrollment','currentScore','bestScore','latestScore','completedRunCount','totalQuestionAttempts','lastActivityAt'] OR summary_source-ARRAY['tenant','enrollment','currentScore','bestScore','latestScore','completedRunCount','totalQuestionAttempts','lastActivityAt']<>'{}'::jsonb OR jsonb_typeof(summary_source->'tenant')<>'string' OR jsonb_typeof(summary_source->'enrollment')<>'string' OR jsonb_typeof(summary_source->'currentScore') NOT IN ('null','number') OR jsonb_typeof(summary_source->'bestScore') NOT IN ('null','number') OR jsonb_typeof(summary_source->'latestScore') NOT IN ('null','number') OR jsonb_typeof(summary_source->'completedRunCount')<>'number' OR jsonb_typeof(summary_source->'totalQuestionAttempts')<>'number' OR jsonb_typeof(summary_source->'lastActivityAt') NOT IN ('null','number') THEN RAISE EXCEPTION 'accepted-submission summary shape is invalid' USING ERRCODE='22023'; END IF;
    BEGIN summary_current_score:=CASE WHEN summary_source->'currentScore'='null'::jsonb THEN NULL ELSE (summary_source->>'currentScore')::double precision END; summary_best_score:=CASE WHEN summary_source->'bestScore'='null'::jsonb THEN NULL ELSE (summary_source->>'bestScore')::double precision END; summary_latest_score:=CASE WHEN summary_source->'latestScore'='null'::jsonb THEN NULL ELSE (summary_source->>'latestScore')::double precision END; summary_completed_run_count:=(summary_source->>'completedRunCount')::bigint; summary_total_question_attempts:=(summary_source->>'totalQuestionAttempts')::bigint; summary_last_activity_at_millis:=CASE WHEN summary_source->'lastActivityAt'='null'::jsonb THEN NULL ELSE (summary_source->>'lastActivityAt')::bigint END;
    EXCEPTION WHEN invalid_text_representation OR numeric_value_out_of_range THEN RAISE EXCEPTION 'accepted-submission summary scalars are invalid' USING ERRCODE='22023'; END;
    IF summary_current_score NOT BETWEEN 0 AND 1 OR summary_best_score NOT BETWEEN 0 AND 1 OR summary_latest_score NOT BETWEEN 0 AND 1 OR summary_completed_run_count<0 OR summary_completed_run_count>4294967295 OR summary_total_question_attempts<0 THEN RAISE EXCEPTION 'accepted-submission summary scalars are out of range' USING ERRCODE='22023'; END IF;
    SELECT witness.* INTO v FROM public.ple_accepted_submission_execution_witness_v1 AS witness
    JOIN public.grading_execution execution ON (execution.tenant_id,execution.attempt_id)=(witness.tenant_id,witness.attempt_id)
    JOIN public.worker_job job ON (job.tenant_id,job.job_id)=(witness.tenant_id,witness.current_job_id)
    JOIN public.submission_evaluation evaluation ON (evaluation.tenant_id,evaluation.attempt_id)=(witness.tenant_id,witness.attempt_id)
    JOIN public.question_attempt attempt ON (attempt.tenant_id,attempt.attempt_id)=(witness.tenant_id,witness.attempt_id)
    JOIN public.assignment_run run ON (run.tenant_id,run.run_id)=(witness.tenant_id,witness.run_id)
    JOIN public.enrollment enrollment ON (enrollment.tenant_id,enrollment.enrollment_id)=(witness.tenant_id,witness.enrollment_id)
    JOIN public.student_assignment_summary summary ON (summary.tenant_id,summary.enrollment_id)=(witness.tenant_id,witness.enrollment_id)
    JOIN public.course_retention retention ON (retention.tenant_id,retention.course_id)=(witness.tenant_id,witness.course_id)
    WHERE witness.tenant_id=p_tenant_id AND witness.current_job_id=p_worker_job_id AND witness.submission_id=p_submission_id
      AND witness.execution_generation=p_execution_generation AND witness.active_worker_id=p_worker_id
      AND witness.execution_state='running' AND witness.job_state='leased' AND witness.lease_token=p_lease_token
      AND witness.lease_expires_at>transaction_timestamp() AND witness.retention_lifecycle='active'
      AND witness.grading_status='automated_pending' AND witness.automated_result_canonical_json IS NULL
      AND witness.automated_result_sha256 IS NULL AND witness.scoring_generation=p_expected_scoring_generation
      AND EXISTS (SELECT 1 FROM public.assignment AS assignment
                  WHERE assignment.tenant_id=witness.tenant_id AND assignment.assignment_id=witness.assignment_id
                    AND assignment.revision=witness.assignment_revision)
      AND EXISTS (SELECT 1 FROM public.course_retention AS current_retention
                  WHERE current_retention.tenant_id=witness.tenant_id AND current_retention.course_id=witness.course_id
                    AND current_retention.lifecycle='active')
    FOR UPDATE OF execution,job,evaluation,attempt,run,enrollment,summary;
    IF NOT FOUND THEN RETURN QUERY SELECT 'claim_no_longer_active',NULL::text,NULL::text; RETURN; END IF;
    IF p_attempt_payload->>'id' IS DISTINCT FROM v.attempt_id::text OR p_attempt_payload->>'tenant' IS DISTINCT FROM p_tenant_id::text
       OR p_attempt_payload->>'run' IS DISTINCT FROM v.run_id::text OR p_attempt_payload->'response' <> 'null'::jsonb
       OR p_attempt_payload->>'status' <> 'submitted' OR p_attempt_payload->'result' IS DISTINCT FROM r
       OR ((p_attempt_payload - ARRAY['response','status','result']) #- '{timer,submittedAt}')
          IS DISTINCT FROM ((v.attempt_payload - ARRAY['response','status','result']) #- '{timer,submittedAt}')
       OR p_attempt_payload #>> '{timer,submittedAt}' IS DISTINCT FROM v.accepted_millis::text
       OR p_run_payload->>'id' IS DISTINCT FROM v.run_id::text
       OR (p_run_payload - ARRAY['completedAt','score'])
          IS DISTINCT FROM (v.run_payload - ARRAY['completedAt','score'])
       OR (p_run_payload #>> '{completedAt}') IS DISTINCT FROM p_run_completed_at_millis::text
       OR (p_run_payload #>> '{score}')::numeric IS DISTINCT FROM (CASE
              WHEN p_run_completed_at_millis IS NULL THEN NULL ELSE earned / possible END)
       OR p_summary_payload->>'tenant' IS DISTINCT FROM p_tenant_id::text
       OR p_summary_payload->>'enrollment' IS DISTINCT FROM v.enrollment_id::text
       OR NOT EXISTS (SELECT 1 FROM public.assignment_run_item item WHERE item.tenant_id=p_tenant_id AND item.run_id=v.run_id
           AND item.assignment_item_id=p_assignment_item_id AND item.issued_position=v.assignment_position)
       OR p_presentation_required IS DISTINCT FROM v.presentation_required
       OR (p_presentation_required AND p_presentation_payload IS DISTINCT FROM v.presentation_payload)
       OR (NOT p_presentation_required AND (p_presentation_payload IS NOT NULL OR p_presentation_payload_sha256 IS NOT NULL)) THEN
        RAISE EXCEPTION 'accepted-submission completion plan does not match locked evidence' USING ERRCODE='22023'; END IF;
    IF p_attempt_payload_sha256 IS DISTINCT FROM encode(pg_catalog.sha256(convert_to(p_attempt_canonical_json,'UTF8')),'hex')
       OR p_run_payload_sha256 IS DISTINCT FROM encode(pg_catalog.sha256(convert_to(p_run_canonical_json,'UTF8')),'hex')
       OR p_summary_payload_sha256 IS DISTINCT FROM encode(pg_catalog.sha256(convert_to(p_summary_canonical_json,'UTF8')),'hex')
       OR p_attempt_current_payload_sha256 IS DISTINCT FROM encode(pg_catalog.sha256(convert_to(p_attempt_current_canonical_json,'UTF8')),'hex')
       OR p_run_current_payload_sha256 IS DISTINCT FROM encode(pg_catalog.sha256(convert_to(p_run_current_canonical_json,'UTF8')),'hex')
       OR (p_presentation_required AND p_presentation_payload_sha256 IS DISTINCT FROM encode(pg_catalog.sha256(convert_to(p_presentation_canonical_json,'UTF8')),'hex'))
       THEN
        RAISE EXCEPTION 'accepted-submission completion checksum is invalid' USING ERRCODE='22023'; END IF;
    UPDATE public.question_attempt SET attempt_status='submitted',
      submitted_at=to_timestamp(v.accepted_millis::double precision/1000),
      payload=p_attempt_payload,payload_sha256=p_attempt_current_payload_sha256
    WHERE tenant_id=p_tenant_id AND attempt_id=v.attempt_id;
    INSERT INTO public.attempt_feedback(tenant_id,attempt_id,hint,correct_response,rationale,content_canonical_json,content_canonical_json_version,content_sha256,course_id)
    VALUES(p_tenant_id,v.attempt_id,NULLIF(feedback->0,'null'::jsonb),NULLIF(feedback->1,'null'::jsonb),NULLIF(feedback->2,'null'::jsonb),p_feedback_canonical_json,p_canonical_json_version,p_feedback_content_sha256,v.course_id);
    UPDATE public.submission_evaluation SET grading_status=p_evaluation_status,correct=correct,credit_fraction=earned/possible,payload=r,
      payload_sha256=p_evaluation_sha256,automated_result_canonical_json=p_evaluation_canonical_json,
      automated_result_sha256=p_evaluation_sha256,automated_result_canonical_json_version=p_canonical_json_version,evaluated_at=transaction_timestamp(),evaluation_revision=evaluation_revision+1
    WHERE tenant_id=p_tenant_id AND attempt_id=v.attempt_id AND submission_id=p_submission_id;
    UPDATE public.assignment_run SET payload=p_run_payload,payload_sha256=p_run_current_payload_sha256,completed_at=to_timestamp(p_run_completed_at_millis::double precision/1000) WHERE tenant_id=p_tenant_id AND run_id=v.run_id;
    UPDATE public.enrollment SET first_completed_at=to_timestamp(p_enrollment_first_completed_at_millis::double precision/1000),current_grade_run_id=p_enrollment_current_grade_run_id,best_grade_run_id=p_enrollment_best_grade_run_id WHERE tenant_id=p_tenant_id AND enrollment_id=v.enrollment_id;
    UPDATE public.student_assignment_summary SET current_score=summary_current_score,best_score=summary_best_score,
      latest_score=summary_latest_score,completed_run_count=summary_completed_run_count,
      total_question_attempts=summary_total_question_attempts,
      last_activity_at=to_timestamp(summary_last_activity_at_millis::double precision/1000),updated_at=transaction_timestamp()
      WHERE tenant_id=p_tenant_id AND enrollment_id=v.enrollment_id;
    INSERT INTO public.submission_receipt_snapshot(tenant_id,attempt_id,canonical_json_version,receipt_attempt_canonical_json,receipt_attempt_payload,receipt_attempt_payload_sha256,run_canonical_json,run_payload,run_payload_sha256,summary_canonical_json,summary_payload,summary_payload_sha256,presentation_canonical_json,presentation_payload,presentation_payload_sha256,presentation_required)
    VALUES(p_tenant_id,v.attempt_id,p_canonical_json_version,p_attempt_canonical_json,p_attempt_payload,p_attempt_payload_sha256,p_run_canonical_json,p_run_payload,p_run_payload_sha256,p_summary_canonical_json,p_summary_payload,p_summary_payload_sha256,p_presentation_canonical_json,p_presentation_payload,p_presentation_payload_sha256,p_presentation_required);
    FOR statistic IN SELECT value FROM jsonb_array_elements(p_statistics) LOOP
      PERFORM public.ple_record_question_statistics(p_tenant_id,v.enrollment_id,v.run_id,(statistic->>'attemptId')::uuid,(statistic->>'problemId')::uuid,(statistic->>'versionId')::uuid,(statistic->>'normalizedScore')::double precision,(statistic->>'attempts')::bigint,(statistic->>'durationSeconds')::bigint,(statistic->>'restScore')::double precision,decode(statistic->>'observationSha256','hex'));
    END LOOP;
    UPDATE public.grading_execution SET state='completed',active_worker_id=NULL,updated_at=transaction_timestamp() WHERE tenant_id=p_tenant_id AND attempt_id=v.attempt_id;
    UPDATE public.worker_job SET state='completed',lease_token=NULL,lease_expires_at=NULL,completed_at=transaction_timestamp() WHERE tenant_id=p_tenant_id AND job_id=p_worker_job_id;
    INSERT INTO public.grading_execution_receipt(tenant_id,receipt_id,attempt_id,submission_id,submission_occurred_at,course_id,execution_generation,resulting_state,worker_id) VALUES(p_tenant_id,gen_random_uuid(),v.attempt_id,p_submission_id,v.submission_occurred_at,v.course_id,p_execution_generation,'completed',p_worker_id);
    PERFORM public.ple_enqueue_assignment_recalculation(p_tenant_id,v.assignment_id,p_recalculation_job_id,p_recalculation_max_attempts);
    RETURN QUERY SELECT 'committed','completed','graded';
END $$;
CREATE FUNCTION public.ple_fail_accepted_submission_execution_v1(
    p_tenant_id uuid, p_worker_job_id uuid, p_lease_token uuid, p_submission_id uuid,
    p_execution_generation bigint, p_worker_id uuid, p_failure_kind text, p_operation_reason text
) RETURNS TABLE(disposition text, resulting_execution_state text, resulting_evaluation_status text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_execution public.grading_execution%ROWTYPE; v_terminal boolean; v_reason text; v_assignment uuid;
BEGIN
    IF p_tenant_id IS NULL OR p_worker_job_id IS NULL OR p_lease_token IS NULL OR p_submission_id IS NULL
       OR p_execution_generation IS NULL OR p_execution_generation <= 0 OR p_worker_id IS NULL
       OR p_failure_kind NOT IN ('deterministic','transient','timed_out','terminal')
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'accepted-submission failure arguments are invalid' USING ERRCODE='22023'; END IF;
    IF p_failure_kind='deterministic' AND p_operation_reason NOT IN
       ('grader_contract_failure','grader_execution_failure','issued_evidence_integrity') THEN
        RAISE EXCEPTION 'accepted-submission deterministic reason is invalid' USING ERRCODE='22023'; END IF;
    IF p_failure_kind<>'deterministic' AND p_operation_reason IS NOT NULL THEN
        RAISE EXCEPTION 'accepted-submission failure reason is invalid' USING ERRCODE='22023'; END IF;
    SELECT execution.* INTO v_execution FROM public.grading_execution AS execution
    JOIN public.worker_job AS job ON (job.tenant_id,job.job_id)=(execution.tenant_id,execution.current_job_id)
    JOIN public.submission_evaluation AS evaluation ON (evaluation.tenant_id,evaluation.attempt_id,evaluation.submission_id)
      = (execution.tenant_id,execution.attempt_id,execution.submission_id)
    JOIN public.course_retention AS retention ON (retention.tenant_id,retention.course_id)=(execution.tenant_id,execution.course_id)
    JOIN public.question_attempt AS attempt ON (attempt.tenant_id,attempt.attempt_id)=(execution.tenant_id,execution.attempt_id)
    JOIN public.submission AS accepted_submission ON accepted_submission.tenant_id=execution.tenant_id
      AND accepted_submission.attempt_id=execution.attempt_id AND accepted_submission.submission_id=execution.submission_id
      AND accepted_submission.occurred_at=execution.submission_occurred_at
    JOIN public.submission_idempotency AS accepted ON accepted.tenant_id=execution.tenant_id
      AND accepted.attempt_id=execution.attempt_id AND accepted.submission_id=execution.submission_id
      AND accepted.submission_occurred_at=execution.submission_occurred_at
    JOIN public.accepted_submission_private_response AS response ON response.tenant_id=execution.tenant_id
      AND response.course_id=execution.course_id AND response.attempt_id=execution.attempt_id
      AND response.submission_id=execution.submission_id AND response.submission_occurred_at=execution.submission_occurred_at
    JOIN public.issued_attempt_private_execution AS private ON private.tenant_id=attempt.tenant_id
      AND private.attempt_id=attempt.attempt_id AND private.attempt_occurred_at=attempt.occurred_at
    JOIN public.assignment_run AS run ON (run.tenant_id,run.run_id)=(attempt.tenant_id,attempt.run_id)
    JOIN public.enrollment AS enrollment ON (enrollment.tenant_id,enrollment.enrollment_id)=(run.tenant_id,run.enrollment_id)
    JOIN public.assignment AS assignment ON (assignment.tenant_id,assignment.assignment_id)=(enrollment.tenant_id,enrollment.assignment_id)
    WHERE execution.tenant_id=p_tenant_id AND execution.current_job_id=p_worker_job_id
      AND execution.submission_id=p_submission_id AND execution.execution_generation=p_execution_generation
      AND execution.state='running' AND execution.active_worker_id=p_worker_id
      AND job.state='leased' AND job.lease_token=p_lease_token AND job.lease_expires_at>transaction_timestamp()
      AND job.payload=jsonb_build_object('kind','gradeAcceptedSubmission','attempt',execution.attempt_id::text,
          'submission',execution.submission_id::text,'execution_generation',execution.execution_generation)
      AND retention.lifecycle='active'
      AND evaluation.grading_status='automated_pending' AND evaluation.automated_result_canonical_json IS NULL
      AND evaluation.automated_result_sha256 IS NULL AND execution.course_id=assignment.course_id
      AND attempt.course_id=assignment.course_id AND accepted.request_contract_version=2
      AND accepted.accepted_actor_id IS NOT NULL AND accepted.course_id=assignment.course_id
      AND accepted_submission.course_id=assignment.course_id AND accepted_submission.idempotency_key=accepted.idempotency_key
      AND accepted.request_sha256=response.response_sha256
      AND response.response_sha256=encode(pg_catalog.sha256(convert_to(response.response_canonical_json,'UTF8')),'hex')
      AND accepted_submission.payload_sha256=encode(pg_catalog.sha256(convert_to(
          '{"kind":"acceptedPrivateResponseV1"}'::jsonb::text,'UTF8')),'hex')
      AND accepted.payload_sha256=encode(pg_catalog.sha256(convert_to(
          '{"kind":"acceptedPrivateResponseV1"}'::jsonb::text,'UTF8')),'hex')
    FOR UPDATE OF execution,job,evaluation;
    IF NOT FOUND THEN RETURN QUERY SELECT 'claim_no_longer_active',NULL::text,NULL::text; RETURN; END IF;
    SELECT enrollment.assignment_id INTO v_assignment FROM public.question_attempt AS attempt
     JOIN public.assignment_run AS run ON (run.tenant_id,run.run_id)=(attempt.tenant_id,attempt.run_id)
     JOIN public.enrollment AS enrollment ON (enrollment.tenant_id,enrollment.enrollment_id)=(run.tenant_id,run.enrollment_id)
     WHERE attempt.tenant_id=p_tenant_id AND attempt.attempt_id=v_execution.attempt_id;
    IF NOT FOUND THEN RETURN QUERY SELECT 'claim_no_longer_active',NULL::text,NULL::text; RETURN; END IF;
    SELECT p_failure_kind IN ('deterministic','terminal') OR job.attempt_count>=job.max_attempts
      INTO v_terminal FROM public.worker_job AS job WHERE job.tenant_id=p_tenant_id AND job.job_id=p_worker_job_id;
    IF v_terminal THEN
        v_reason := COALESCE(p_operation_reason, CASE WHEN p_failure_kind IN ('transient','timed_out') THEN 'retry_exhausted' ELSE 'grader_execution_failure' END);
        UPDATE public.submission_evaluation SET grading_status='automated_exception', evaluated_at=transaction_timestamp(),
            evaluation_revision=evaluation_revision+1 WHERE tenant_id=p_tenant_id AND attempt_id=v_execution.attempt_id
            AND submission_id=p_submission_id AND grading_status='automated_pending'
            AND automated_result_canonical_json IS NULL AND automated_result_sha256 IS NULL;
        IF NOT FOUND THEN RETURN QUERY SELECT 'claim_no_longer_active',NULL::text,NULL::text; RETURN; END IF;
        UPDATE public.grading_execution SET state='exception',active_worker_id=NULL,updated_at=transaction_timestamp()
         WHERE tenant_id=p_tenant_id AND attempt_id=v_execution.attempt_id;
        UPDATE public.worker_job SET state='dead',lease_token=NULL,lease_expires_at=NULL,last_error=
            CASE WHEN p_failure_kind='timed_out' THEN 'timed_out' ELSE 'permanent' END,completed_at=transaction_timestamp()
         WHERE tenant_id=p_tenant_id AND job_id=p_worker_job_id;
        INSERT INTO public.grading_execution_receipt (tenant_id,receipt_id,attempt_id,submission_id,submission_occurred_at,
          course_id,execution_generation,resulting_state,worker_id) VALUES(p_tenant_id,gen_random_uuid(),v_execution.attempt_id,
          p_submission_id,v_execution.submission_occurred_at,v_execution.course_id,p_execution_generation,'exception',p_worker_id);
        INSERT INTO public.grading_operation (tenant_id,attempt_id,submission_id,submission_occurred_at,assignment_id,course_id,
          target_kind,reason,state,next_action) VALUES(p_tenant_id,v_execution.attempt_id,p_submission_id,
          v_execution.submission_occurred_at,v_assignment,v_execution.course_id,'submission',v_reason,'actionable','retry') ON CONFLICT DO NOTHING;
        RETURN QUERY SELECT 'terminal','exception','automated_exception';
    END IF;
    UPDATE public.grading_execution SET state='retry_wait',active_worker_id=NULL,retry_count=retry_count+1,updated_at=transaction_timestamp()
      WHERE tenant_id=p_tenant_id AND attempt_id=v_execution.attempt_id;
    UPDATE public.worker_job SET state='ready',available_at=transaction_timestamp()+make_interval(secs => (1 << LEAST(GREATEST(attempt_count-1,0),8))),
      lease_token=NULL,lease_expires_at=NULL,last_error=CASE WHEN p_failure_kind='timed_out' THEN 'timed_out' ELSE 'transient' END,
      completed_at=NULL WHERE tenant_id=p_tenant_id AND job_id=p_worker_job_id;
    INSERT INTO public.grading_execution_receipt (tenant_id,receipt_id,attempt_id,submission_id,submission_occurred_at,course_id,
      execution_generation,resulting_state,worker_id) VALUES(p_tenant_id,gen_random_uuid(),v_execution.attempt_id,p_submission_id,
      v_execution.submission_occurred_at,v_execution.course_id,p_execution_generation,'retry_wait',p_worker_id);
    RETURN QUERY SELECT 'rescheduled','retry_wait','automated_pending';
END $$;
ALTER FUNCTION public.ple_claim_accepted_submission_execution_v1(uuid,uuid,integer)
    OWNER TO ple_accepted_submission_execution_worker;
ALTER FUNCTION public.ple_load_accepted_submission_execution_v2(uuid,uuid,uuid,uuid,bigint,uuid)
    OWNER TO ple_accepted_submission_execution_worker;
ALTER FUNCTION public.ple_lock_accepted_submission_completion_v1(uuid,uuid,uuid,uuid,bigint,uuid)
    OWNER TO ple_accepted_submission_execution_worker;
ALTER FUNCTION public.ple_commit_accepted_submission_completion_v2(uuid,uuid,uuid,uuid,bigint,uuid,smallint,text,text,character,text,jsonb,character,text,character,text,character,text,jsonb,character,text,character,bigint,bigint,uuid,uuid,text,jsonb,character,text,jsonb,character,boolean,uuid,jsonb,bigint,uuid,integer)
    OWNER TO ple_accepted_submission_execution_worker;
ALTER FUNCTION public.ple_fail_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,bigint,uuid,text,text)
    OWNER TO ple_accepted_submission_execution_worker;
ALTER FUNCTION public.ple_guard_receipt_attempt_snapshot() OWNER TO ple_accepted_submission_execution_worker; ALTER FUNCTION public.ple_forbid_automated_result_mutation() OWNER TO ple_accepted_submission_execution_worker; ALTER FUNCTION public.ple_guard_accepted_execution_evidence_writer() OWNER TO ple_accepted_submission_execution_worker;
REVOKE ALL ON FUNCTION public.ple_claim_accepted_submission_execution_v1(uuid,uuid,integer),
    public.ple_load_accepted_submission_execution_v2(uuid,uuid,uuid,uuid,bigint,uuid),
    public.ple_lock_accepted_submission_completion_v1(uuid,uuid,uuid,uuid,bigint,uuid),
    public.ple_commit_accepted_submission_completion_v2(uuid,uuid,uuid,uuid,bigint,uuid,smallint,text,text,character,text,jsonb,character,text,character,text,character,text,jsonb,character,text,character,bigint,bigint,uuid,uuid,text,jsonb,character,text,jsonb,character,boolean,uuid,jsonb,bigint,uuid,integer),
    public.ple_fail_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,bigint,uuid,text,text)
    FROM PUBLIC, ple_app, ple_queue_broker, ple_automated_grading_broker, ple_retention_broker;
GRANT EXECUTE ON FUNCTION public.ple_claim_accepted_submission_execution_v1(uuid,uuid,integer),
    public.ple_load_accepted_submission_execution_v2(uuid,uuid,uuid,uuid,bigint,uuid),
    public.ple_lock_accepted_submission_completion_v1(uuid,uuid,uuid,uuid,bigint,uuid),
    public.ple_commit_accepted_submission_completion_v2(uuid,uuid,uuid,uuid,bigint,uuid,smallint,text,text,character,text,jsonb,character,text,character,text,character,text,jsonb,character,text,character,bigint,bigint,uuid,uuid,text,jsonb,character,text,jsonb,character,boolean,uuid,jsonb,bigint,uuid,integer),
    public.ple_fail_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,bigint,uuid,text,text)
    TO ple_accepted_submission_execution;
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM ple_accepted_submission_execution; REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM ple_accepted_submission_execution;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_attribute WHERE attrelid='public.grading_execution'::regclass
        AND attname='active_worker_id' AND NOT attisdropped)
       OR (SELECT count(*) FROM pg_catalog.pg_attribute WHERE attrelid='public.submission_evaluation'::regclass
           AND attname = ANY (ARRAY['automated_result_canonical_json','automated_result_sha256','automated_result_canonical_json_version'])
           AND NOT attisdropped) <> 3
       OR NOT EXISTS (SELECT 1 FROM pg_catalog.pg_constraint WHERE conrelid='public.submission_evaluation'::regclass
        AND conname='submission_evaluation_automated_result_pair_check')
       OR (SELECT count(*) FROM pg_catalog.pg_constraint WHERE conname = ANY (ARRAY[
            'submission_receipt_snapshot_canonical_source_check', 'submission_receipt_snapshot_canonical_json_version_check',
            'attempt_feedback_content_canonical_json_check'])) <> 3
       OR EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname='ple_accepted_submission_execution_worker'
          AND (rolcanlogin OR rolinherit OR rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls))
       OR EXISTS (SELECT 1 FROM pg_catalog.pg_auth_members WHERE roleid='ple_accepted_submission_execution_worker'::regrole
          OR member='ple_accepted_submission_execution_worker'::regrole)
       OR EXISTS (SELECT 1 FROM unnest(ARRAY[
            'public.ple_claim_accepted_submission_execution_v1(uuid,uuid,integer)',
            'public.ple_load_accepted_submission_execution_v2(uuid,uuid,uuid,uuid,bigint,uuid)',
            'public.ple_lock_accepted_submission_completion_v1(uuid,uuid,uuid,uuid,bigint,uuid)',
            'public.ple_commit_accepted_submission_completion_v2(uuid,uuid,uuid,uuid,bigint,uuid,smallint,text,text,character,text,jsonb,character,text,character,text,character,text,jsonb,character,text,character,bigint,bigint,uuid,uuid,text,jsonb,character,text,jsonb,character,boolean,uuid,jsonb,bigint,uuid,integer)',
            'public.ple_fail_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,bigint,uuid,text,text)'
          ]) AS function_name
          WHERE NOT has_function_privilege('ple_accepted_submission_execution', function_name, 'EXECUTE')
             OR has_function_privilege('ple_app', function_name, 'EXECUTE')
             OR has_function_privilege('public', function_name, 'EXECUTE'))
       OR EXISTS (SELECT 1 FROM unnest(ARRAY[
            'public.ple_guard_receipt_attempt_snapshot()'::regprocedure,
            'public.ple_forbid_automated_result_mutation()'::regprocedure,
            'public.ple_guard_accepted_execution_evidence_writer()'::regprocedure
          ]) AS expected(procedure_id) JOIN pg_catalog.pg_proc AS procedure_row ON procedure_row.oid=expected.procedure_id
          WHERE procedure_row.proowner <> 'ple_accepted_submission_execution_worker'::regrole
             OR procedure_row.prosecdef
             OR procedure_row.proconfig IS DISTINCT FROM ARRAY['search_path=pg_catalog, public, pg_temp'])
       OR EXISTS (SELECT 1 FROM (VALUES
            ('public.submission_evaluation'::regclass,'submission_evaluation_automated_result_append_only','public.ple_forbid_automated_result_mutation()'::regprocedure,27::smallint),
            ('public.submission_evaluation'::regclass,'submission_evaluation_accepted_execution_writer','public.ple_guard_accepted_execution_evidence_writer()'::regprocedure,23::smallint),
            ('public.attempt_feedback'::regclass,'attempt_feedback_accepted_execution_writer','public.ple_guard_accepted_execution_evidence_writer()'::regprocedure,7::smallint),
            ('public.submission_receipt_snapshot'::regclass,'submission_receipt_snapshot_attempt_guard','public.ple_guard_receipt_attempt_snapshot()'::regprocedure,23::smallint),
            ('public.submission_receipt_snapshot'::regclass,'submission_receipt_snapshot_accepted_execution_writer','public.ple_guard_accepted_execution_evidence_writer()'::regprocedure,7::smallint)
          ) AS expected(relation_id,trigger_name,procedure_id,trigger_type)
          WHERE NOT EXISTS (SELECT 1 FROM pg_catalog.pg_trigger AS trigger_row
                            WHERE trigger_row.tgrelid=expected.relation_id AND trigger_row.tgname=expected.trigger_name
                              AND trigger_row.tgfoid=expected.procedure_id AND trigger_row.tgtype=expected.trigger_type
                              AND trigger_row.tgenabled='O' AND NOT trigger_row.tgisinternal))
       OR EXISTS (SELECT 1 FROM unnest(ARRAY['public.ple_guard_receipt_attempt_snapshot()','public.ple_forbid_automated_result_mutation()','public.ple_guard_accepted_execution_evidence_writer()']) AS guard(function_name) WHERE has_function_privilege('public',guard.function_name,'EXECUTE') OR has_function_privilege('ple_app',guard.function_name,'EXECUTE') OR has_function_privilege('ple_accepted_submission_execution',guard.function_name,'EXECUTE'))
       OR has_table_privilege('ple_accepted_submission_execution','public.accepted_submission_private_response','SELECT')
       OR NOT has_table_privilege('ple_retention_broker','public.submission_evaluation','DELETE')
       OR EXISTS (
            SELECT 1
              FROM unnest(ARRAY[
                    'ple_app', 'ple_auth', 'ple_student', 'ple_grader', 'ple_grading_reader',
                    'ple_queue_broker', 'ple_automated_grading_broker',
                    'ple_accepted_submission_execution',
                    'ple_accepted_submission_execution_worker'
              ]) AS denied_role(role_name)
             WHERE has_table_privilege(denied_role.role_name, 'public.submission_evaluation', 'DELETE')
       )
       OR has_column_privilege('ple_app','public.submission_evaluation','automated_result_canonical_json','SELECT')
       OR has_column_privilege('ple_app','public.submission_evaluation','automated_result_canonical_json','UPDATE')
       OR has_column_privilege('ple_app','public.submission_evaluation','automated_result_sha256','SELECT')
       OR has_column_privilege('ple_app','public.submission_evaluation','automated_result_sha256','UPDATE')
       OR has_column_privilege('ple_app','public.submission_evaluation','automated_result_canonical_json_version','SELECT')
       OR has_column_privilege('ple_app','public.submission_evaluation','automated_result_canonical_json_version','UPDATE')
       OR EXISTS (SELECT 1 FROM pg_catalog.pg_attribute AS attribute_row
                    WHERE attribute_row.attrelid='public.submission_evaluation'::regclass
                      AND attribute_row.attnum>0 AND NOT attribute_row.attisdropped
                      AND ((has_column_privilege('ple_app','public.submission_evaluation',attribute_row.attname,'SELECT')
                            AND attribute_row.attname <> ALL(ARRAY['tenant_id','attempt_id','submission_id','grading_status','credit_fraction','correct','payload','payload_sha256','evaluated_at','course_id','evaluation_revision']))
                        OR (has_column_privilege('ple_app','public.submission_evaluation',attribute_row.attname,'INSERT')
                            AND attribute_row.attname <> ALL(ARRAY['tenant_id','attempt_id','submission_id','grading_status','credit_fraction','correct','payload','payload_sha256','evaluated_at','course_id','evaluation_revision']))
                        OR (has_column_privilege('ple_app','public.submission_evaluation',attribute_row.attname,'UPDATE')
                            AND attribute_row.attname <> ALL(ARRAY['grading_status','credit_fraction','correct','payload','payload_sha256','evaluated_at','evaluation_revision']))))
       OR EXISTS (SELECT 1 FROM unnest(ARRAY['tenant_id','attempt_id','submission_id','grading_status','credit_fraction','correct','payload','payload_sha256','evaluated_at','course_id','evaluation_revision']) AS allowed(column_name)
                    WHERE NOT has_column_privilege('ple_app','public.submission_evaluation',allowed.column_name,'SELECT')
                       OR NOT has_column_privilege('ple_app','public.submission_evaluation',allowed.column_name,'INSERT'))
       OR EXISTS (SELECT 1 FROM unnest(ARRAY['submission_id','grading_status','credit_fraction','correct','payload','payload_sha256','evaluated_at','evaluation_revision']) AS allowed(column_name)
                    WHERE NOT has_column_privilege('ple_app','public.submission_evaluation',allowed.column_name,'UPDATE'))
       OR EXISTS (
            SELECT 1
              FROM unnest(ARRAY[
                    'public.ple_claim_accepted_submission_execution_v1(uuid,uuid,integer)'::regprocedure,
                    'public.ple_load_accepted_submission_execution_v2(uuid,uuid,uuid,uuid,bigint,uuid)'::regprocedure,
                    'public.ple_lock_accepted_submission_completion_v1(uuid,uuid,uuid,uuid,bigint,uuid)'::regprocedure,
                    'public.ple_commit_accepted_submission_completion_v2(uuid,uuid,uuid,uuid,bigint,uuid,smallint,text,text,character,text,jsonb,character,text,character,text,character,text,jsonb,character,text,character,bigint,bigint,uuid,uuid,text,jsonb,character,text,jsonb,character,boolean,uuid,jsonb,bigint,uuid,integer)'::regprocedure,
                    'public.ple_fail_accepted_submission_execution_v1(uuid,uuid,uuid,uuid,bigint,uuid,text,text)'::regprocedure
              ]) AS expected(procedure_id)
              JOIN pg_catalog.pg_proc AS procedure_row ON procedure_row.oid = expected.procedure_id
             WHERE procedure_row.proowner <> 'ple_accepted_submission_execution_worker'::regrole
                OR NOT procedure_row.prosecdef
                OR procedure_row.proconfig IS DISTINCT FROM ARRAY['search_path=pg_catalog, public, pg_temp']
       )
       OR has_schema_privilege('ple_accepted_submission_execution_worker', 'public', 'CREATE')
       OR NOT has_schema_privilege('ple_accepted_submission_execution_worker', 'public', 'USAGE')
       OR (SELECT relowner FROM pg_catalog.pg_class WHERE oid='public.ple_accepted_submission_execution_witness_v1'::regclass) <> 'ple_accepted_submission_execution_worker'::regrole
       OR EXISTS (WITH expected AS (SELECT 'table'::text AS authority_kind, split_part(entry, ':', 1)::regclass::oid AS object_id, NULL::name AS column_name, split_part(entry, ':', 2) AS privilege_type, false AS is_grantable FROM unnest(ARRAY['public.worker_job:SELECT','public.grading_execution:SELECT','public.submission_evaluation:SELECT','public.grading_execution_receipt:INSERT','public.grading_operation:INSERT','public.submission:SELECT','public.submission_idempotency:SELECT','public.question_attempt:SELECT','public.assignment_run:SELECT','public.enrollment:SELECT','public.assignment:SELECT','public.assignment_audience_group:SELECT','public.assignment_item:SELECT','public.assignment_run_item:SELECT','public.assignment_selection_group:SELECT','public.assignment_selection_candidate:SELECT','public.course_retention:SELECT','public.accepted_submission_private_response:SELECT','public.issued_attempt_private_execution:SELECT','public.attempt_feedback:INSERT','public.submission_receipt_snapshot:INSERT']) AS expected(entry) UNION ALL SELECT 'column', split_part(entry, ':', 1)::regclass::oid, split_part(entry, ':', 2)::name, split_part(entry, ':', 3), false FROM unnest(ARRAY['public.worker_job:state:UPDATE','public.worker_job:lease_token:UPDATE','public.worker_job:lease_expires_at:UPDATE','public.worker_job:attempt_count:UPDATE','public.worker_job:last_error:UPDATE','public.worker_job:completed_at:UPDATE','public.worker_job:available_at:UPDATE','public.grading_execution:state:UPDATE','public.grading_execution:active_worker_id:UPDATE','public.grading_execution:retry_count:UPDATE','public.grading_execution:updated_at:UPDATE','public.submission_evaluation:grading_status:UPDATE','public.submission_evaluation:credit_fraction:UPDATE','public.submission_evaluation:correct:UPDATE','public.submission_evaluation:payload:UPDATE','public.submission_evaluation:payload_sha256:UPDATE','public.submission_evaluation:automated_result_canonical_json:UPDATE','public.submission_evaluation:automated_result_sha256:UPDATE','public.submission_evaluation:automated_result_canonical_json_version:UPDATE','public.submission_evaluation:evaluated_at:UPDATE','public.submission_evaluation:evaluation_revision:UPDATE','public.question_attempt:attempt_status:UPDATE','public.question_attempt:submitted_at:UPDATE','public.question_attempt:payload:UPDATE','public.question_attempt:payload_sha256:UPDATE','public.assignment_run:completed_at:UPDATE','public.assignment_run:payload:UPDATE','public.assignment_run:payload_sha256:UPDATE','public.enrollment:first_completed_at:UPDATE','public.enrollment:current_grade_run_id:UPDATE','public.enrollment:best_grade_run_id:UPDATE','public.student_assignment_summary:current_score:UPDATE','public.student_assignment_summary:best_score:UPDATE','public.student_assignment_summary:latest_score:UPDATE','public.student_assignment_summary:completed_run_count:UPDATE','public.student_assignment_summary:total_question_attempts:UPDATE','public.student_assignment_summary:last_activity_at:UPDATE','public.student_assignment_summary:updated_at:UPDATE']) AS expected(entry) UNION ALL SELECT 'function', split_part(entry, ':', 1)::regprocedure::oid, NULL::name, 'EXECUTE', false FROM unnest(ARRAY['public.ple_current_tenant()','public.ple_enqueue_assignment_recalculation(uuid,uuid,uuid,integer)','public.ple_record_question_statistics(uuid,uuid,uuid,uuid,uuid,uuid,double precision,bigint,bigint,double precision,bytea)']) AS expected(entry)), actual AS (SELECT 'table'::text, relation_row.oid, NULL::name, acl.privilege_type, acl.is_grantable FROM pg_catalog.pg_class AS relation_row JOIN pg_catalog.pg_namespace AS namespace_row ON namespace_row.oid=relation_row.relnamespace CROSS JOIN LATERAL pg_catalog.aclexplode(relation_row.relacl) AS acl WHERE namespace_row.nspname='public' AND relation_row.relkind IN ('r','p','v','m','f') AND relation_row.relowner<>'ple_accepted_submission_execution_worker'::regrole AND acl.grantee='ple_accepted_submission_execution_worker'::regrole UNION ALL SELECT 'column', attribute_row.attrelid, attribute_row.attname, acl.privilege_type, acl.is_grantable FROM pg_catalog.pg_attribute AS attribute_row CROSS JOIN LATERAL pg_catalog.aclexplode(attribute_row.attacl) AS acl WHERE attribute_row.attnum>0 AND NOT attribute_row.attisdropped AND acl.grantee='ple_accepted_submission_execution_worker'::regrole UNION ALL SELECT 'sequence', relation_row.oid, NULL::name, acl.privilege_type, acl.is_grantable FROM pg_catalog.pg_class AS relation_row JOIN pg_catalog.pg_namespace AS namespace_row ON namespace_row.oid=relation_row.relnamespace CROSS JOIN LATERAL pg_catalog.aclexplode(relation_row.relacl) AS acl WHERE namespace_row.nspname='public' AND relation_row.relkind='S' AND acl.grantee='ple_accepted_submission_execution_worker'::regrole UNION ALL SELECT 'function', procedure_row.oid, NULL::name, acl.privilege_type, acl.is_grantable FROM pg_catalog.pg_proc AS procedure_row JOIN pg_catalog.pg_namespace AS namespace_row ON namespace_row.oid=procedure_row.pronamespace CROSS JOIN LATERAL pg_catalog.aclexplode(procedure_row.proacl) AS acl WHERE namespace_row.nspname='public' AND procedure_row.proowner<>'ple_accepted_submission_execution_worker'::regrole AND acl.grantee='ple_accepted_submission_execution_worker'::regrole) SELECT 1 FROM ((SELECT * FROM actual EXCEPT SELECT * FROM expected) UNION ALL (SELECT * FROM expected EXCEPT SELECT * FROM actual)) AS privilege_difference)
       OR EXISTS (SELECT 1 FROM (VALUES
            ('worker_job','accepted_execution_worker_job','*','true','true'),('grading_execution','accepted_execution_worker_execution','*','true','true'),('submission_evaluation','accepted_execution_worker_evaluation','*','true','true'),
            ('grading_execution_receipt','accepted_execution_worker_receipt','a','','true'),('grading_operation','accepted_execution_worker_operation','a','','true'),('submission','accepted_execution_worker_submission','r','true',''),
            ('submission_idempotency','accepted_execution_worker_idempotency','r','true',''),('question_attempt','accepted_execution_worker_attempt','r','true',''),('question_attempt','accepted_execution_worker_attempt_completion','w','true','true'),
            ('assignment_run','accepted_execution_worker_run','r','true',''),('assignment_run','accepted_execution_worker_run_completion','w','true','true'),('enrollment','accepted_execution_worker_enrollment','r','true',''),
            ('enrollment','accepted_execution_worker_enrollment_completion','w','true','true'),('assignment','accepted_execution_worker_assignment','r','true',''),('assignment_audience_group','accepted_execution_worker_audience','r','true',''),
            ('assignment_item','accepted_execution_worker_items','r','true',''),('assignment_run_item','accepted_execution_worker_run_items','r','true',''),('assignment_selection_group','accepted_execution_worker_selection_groups','r','true',''),('assignment_selection_candidate','accepted_execution_worker_selection_candidates','r','true',''),
            ('course_retention','accepted_execution_worker_retention','r','true',''),('accepted_submission_private_response','accepted_execution_worker_private_response','r','true',''),('issued_attempt_private_execution','accepted_execution_worker_private_execution','r','true',''),
            ('attempt_feedback','accepted_execution_worker_feedback','a','','true'),('submission_receipt_snapshot','accepted_execution_worker_receipt_snapshot','a','','true'),('student_assignment_summary','accepted_execution_worker_summary_completion','w','true','true')
          ) AS expected(relation_name,policy_name,policy_command,policy_using,policy_check)
          WHERE NOT EXISTS (SELECT 1 FROM pg_catalog.pg_policy AS policy_row JOIN pg_catalog.pg_class AS relation_row ON relation_row.oid=policy_row.polrelid JOIN pg_catalog.pg_namespace AS namespace_row ON namespace_row.oid=relation_row.relnamespace
                            WHERE namespace_row.nspname='public' AND relation_row.relname=expected.relation_name AND policy_row.polname=expected.policy_name
                              AND policy_row.polcmd::text=expected.policy_command AND policy_row.polroles=ARRAY['ple_accepted_submission_execution_worker'::regrole]::oid[]
                              AND COALESCE(pg_catalog.pg_get_expr(policy_row.polqual,policy_row.polrelid),'')=expected.policy_using
                              AND COALESCE(pg_catalog.pg_get_expr(policy_row.polwithcheck,policy_row.polrelid),'')=expected.policy_check))
       OR EXISTS (SELECT 1 FROM unnest(ARRAY['worker_job','grading_execution','submission_evaluation','grading_execution_receipt','grading_operation','submission','submission_idempotency','question_attempt','assignment_run','enrollment','assignment','assignment_audience_group','assignment_item','assignment_run_item','assignment_selection_group','assignment_selection_candidate','course_retention','accepted_submission_private_response','issued_attempt_private_execution','attempt_feedback','submission_receipt_snapshot','student_assignment_summary']) AS expected(relation_name)
                    WHERE NOT EXISTS (SELECT 1 FROM pg_catalog.pg_class AS relation_row JOIN pg_catalog.pg_namespace AS namespace_row ON namespace_row.oid=relation_row.relnamespace WHERE namespace_row.nspname='public' AND relation_row.relname=expected.relation_name AND relation_row.relrowsecurity AND relation_row.relforcerowsecurity))
    THEN RAISE NOTICE 'diagnostic: accepted-submission worker authority is unsafe'; END IF;
END $$; COMMIT;
