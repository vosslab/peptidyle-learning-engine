-- WP-PROF-G1 / G1-W5: source-specific scoring-invalidation witnesses.
--
-- Each public source wrapper derives its origin facts from the authority that
-- already owns that source.  The generic 1864 binder stays internal.

BEGIN;

-- Manual-grade receipts are the immutable source witness.  Carrying the
-- assignment coordinate here keeps source validation independent from mutable
-- attempt, run, and enrollment projections.
ALTER TABLE public.manual_grade_receipt
    ADD COLUMN assignment_id uuid NOT NULL;
ALTER TABLE public.manual_grade_receipt
    ADD CONSTRAINT manual_grade_receipt_assignment_fk
    FOREIGN KEY (tenant_id, assignment_id)
    REFERENCES public.assignment(tenant_id, assignment_id);

GRANT EXECUTE ON FUNCTION public.ple_request_scoring_invalidation_v1(
    uuid, uuid, uuid, text, uuid, uuid, uuid, integer
) TO ple_instructor_grading_operation_broker;

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
        tenant_id, action_id, grading_operation_id, course_id, actor_id, action_kind,
        request_sha256, recalculate_expected_assignment_revision,
        recalculate_created_operation_revision,
        resulting_scoring_generation, resulting_state
    ) VALUES (
        p_tenant_id, p_action_id, v_result.operation_reference, p_course_id, v_actor_id,
        'recalculate', v_request_sha256, p_expected_assignment_revision, 1,
        v_result.scoring_generation, 'recalculating'
    );
    RETURN QUERY SELECT 'accepted', v_result.operation_reference,
        p_expected_assignment_revision, 1::bigint, v_result.scoring_generation,
        'recalculating'::text,
        floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint;
END;
$$;

ALTER FUNCTION public.ple_recalculate_instructor_assignment_v1(
    uuid, character, uuid, uuid, bigint, uuid
) OWNER TO ple_instructor_grading_operation_broker;

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles
        WHERE rolname = 'ple_scoring_invalidation_source_broker') THEN
        CREATE ROLE ple_scoring_invalidation_source_broker
            NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOREPLICATION NOBYPASSRLS;
    END IF;
END $$;
ALTER ROLE ple_scoring_invalidation_source_broker
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;
DO $$ BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid = 'ple_scoring_invalidation_source_broker'::regrole
            OR membership.member = 'ple_scoring_invalidation_source_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'scoring invalidation source broker must have no memberships';
    END IF;
END $$;
REVOKE ALL ON SCHEMA public FROM ple_scoring_invalidation_source_broker;
GRANT USAGE ON SCHEMA public TO ple_scoring_invalidation_source_broker;

CREATE POLICY scoring_invalidation_source_manual_grade ON public.manual_grade_receipt
    FOR SELECT TO ple_scoring_invalidation_source_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY scoring_invalidation_source_audit ON public.audit_event
    FOR SELECT TO ple_scoring_invalidation_source_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY scoring_invalidation_source_attempt ON public.question_attempt
    FOR SELECT TO ple_scoring_invalidation_source_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY scoring_invalidation_source_run ON public.assignment_run
    FOR SELECT TO ple_scoring_invalidation_source_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY scoring_invalidation_source_enrollment ON public.enrollment
    FOR SELECT TO ple_scoring_invalidation_source_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY scoring_invalidation_source_execution ON public.grading_execution
    FOR SELECT TO ple_scoring_invalidation_source_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY scoring_invalidation_source_assignment ON public.assignment
    FOR SELECT TO ple_scoring_invalidation_source_broker
    USING (tenant_id = public.ple_current_tenant());
GRANT SELECT ON public.manual_grade_receipt, public.audit_event, public.question_attempt,
    public.assignment_run, public.enrollment, public.grading_execution, public.assignment
    TO ple_scoring_invalidation_source_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(),
    public.ple_course_records_accessible(uuid, uuid),
    public.ple_bind_scoring_invalidation_origin_v1(
    uuid, uuid, uuid, bigint, uuid, text, uuid, uuid, integer
    ) TO ple_scoring_invalidation_source_broker;

CREATE FUNCTION public.ple_bind_manual_grade_invalidation_v1(
    p_tenant uuid, p_manual_grade_action uuid, p_recalculation_job uuid
) RETURNS TABLE(disposition text, operation_reference integer, scoring_generation bigint,
    recalculation_job_id uuid, origin_id uuid)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE r record;
BEGIN
    SELECT receipt.actor_id, receipt.course_id, receipt.assignment_id,
           receipt.scoring_generation
      INTO r FROM public.manual_grade_receipt receipt
     WHERE receipt.tenant_id = p_tenant
       AND receipt.manual_grade_action_id = p_manual_grade_action;
    IF NOT FOUND THEN RAISE EXCEPTION 'manual-grade invalidation witness is unavailable' USING ERRCODE='42501'; END IF;
    RETURN QUERY SELECT * FROM public.ple_bind_scoring_invalidation_origin_v1(
        p_tenant, r.course_id, r.assignment_id, r.scoring_generation, p_recalculation_job,
        'manual_grade', p_manual_grade_action, r.actor_id, NULL
    );
END $$;

CREATE FUNCTION public.ple_bind_attempt_support_invalidation_v1(
    p_tenant uuid, p_support_action uuid, p_recalculation_job uuid
) RETURNS TABLE(disposition text, operation_reference integer, scoring_generation bigint,
    recalculation_job_id uuid, origin_id uuid)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE r record;
BEGIN
    SELECT event.actor_id, event.course_id, enrollment.assignment_id,
           assignment.scoring_generation
      INTO r FROM public.audit_event event
      JOIN public.question_attempt attempt ON (attempt.tenant_id, attempt.attempt_id)
           = (event.tenant_id, event.target_id)
      JOIN public.assignment_run run_row ON (run_row.tenant_id, run_row.run_id)
           = (attempt.tenant_id, attempt.run_id)
      JOIN public.enrollment enrollment ON (enrollment.tenant_id, enrollment.enrollment_id)
           = (run_row.tenant_id, run_row.enrollment_id)
      JOIN public.assignment assignment ON (assignment.tenant_id, assignment.assignment_id)
           = (enrollment.tenant_id, enrollment.assignment_id)
     WHERE event.tenant_id = p_tenant AND event.audit_event_id = p_support_action
       AND event.target_kind = 'question_attempt' AND event.action = 'attempt.clear';
    IF NOT FOUND THEN RAISE EXCEPTION 'attempt-support invalidation witness is unavailable' USING ERRCODE='42501'; END IF;
    RETURN QUERY SELECT * FROM public.ple_bind_scoring_invalidation_origin_v1(
        p_tenant, r.course_id, r.assignment_id, r.scoring_generation, p_recalculation_job,
        'learner_support', p_support_action, r.actor_id, NULL
    );
END $$;

CREATE FUNCTION public.ple_bind_assignment_definition_invalidation_v1(
    p_tenant uuid, p_actor uuid, p_course uuid, p_assignment uuid,
    p_resulting_revision bigint, p_recalculation_job uuid
) RETURNS TABLE(disposition text, operation_reference integer, scoring_generation bigint,
    recalculation_job_id uuid, origin_id uuid)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE generation bigint; origin_uuid uuid; origin_hex text;
BEGIN
    PERFORM public.ple_assignment_mutator_require_editor(
        p_tenant,p_actor,p_course,p_assignment,p_resulting_revision
    );
    -- The assignment-definition capability already owns the mutation lock;
    -- this read-only source broker verifies its committed-in-transaction result.
    SELECT assignment_row.scoring_generation
      INTO generation
      FROM public.assignment AS assignment_row
     WHERE assignment_row.tenant_id = p_tenant
       AND assignment_row.course_id = p_course
       AND assignment_row.assignment_id = p_assignment
       AND assignment_row.revision = p_resulting_revision
       AND assignment_row.scoring_status = 'recalculating';
    IF NOT FOUND THEN RAISE EXCEPTION 'assignment-definition invalidation witness is unavailable' USING ERRCODE='55000'; END IF;
    origin_hex := encode(pg_catalog.sha256(
        uuid_send(p_assignment) || int8send(p_resulting_revision)
    ), 'hex');
    origin_uuid := (
        substr(origin_hex,1,8) || '-' || substr(origin_hex,9,4) || '-'
        || substr(origin_hex,13,4) || '-' || substr(origin_hex,17,4) || '-'
        || substr(origin_hex,21,12)
    )::uuid;
    IF p_recalculation_job IS DISTINCT FROM origin_uuid THEN
        RAISE EXCEPTION 'assignment-definition recalculation job is not canonical'
            USING ERRCODE='55000';
    END IF;
    RETURN QUERY SELECT * FROM public.ple_bind_scoring_invalidation_origin_v1(
        p_tenant,p_course,p_assignment,generation,p_recalculation_job,
        'assignment_definition',origin_uuid,p_actor,NULL
    );
END $$;

CREATE TABLE public.accepted_completion_invalidation_receipt (
    tenant_id uuid NOT NULL, recalculation_job_id uuid NOT NULL,
    completion_receipt_id uuid NOT NULL,
    execution_job_id uuid NOT NULL, attempt_id uuid NOT NULL, submission_id uuid NOT NULL,
    execution_generation bigint NOT NULL, course_id uuid NOT NULL, assignment_id uuid NOT NULL,
    scoring_generation bigint NOT NULL, occurred_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, recalculation_job_id),
    UNIQUE (tenant_id, completion_receipt_id),
    UNIQUE (tenant_id, execution_job_id, execution_generation),
    CHECK (execution_generation > 0 AND scoring_generation > 0),
    FOREIGN KEY (tenant_id, recalculation_job_id) REFERENCES public.worker_job(tenant_id,job_id)
        DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY (tenant_id, completion_receipt_id)
        REFERENCES public.grading_execution_receipt(tenant_id,receipt_id)
);
ALTER TABLE public.accepted_completion_invalidation_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.accepted_completion_invalidation_receipt FORCE ROW LEVEL SECURITY;
CREATE POLICY accepted_completion_invalidation_source ON public.accepted_completion_invalidation_receipt
    FOR ALL TO ple_scoring_invalidation_source_broker
    USING (tenant_id=public.ple_current_tenant()) WITH CHECK (tenant_id=public.ple_current_tenant());
GRANT SELECT,INSERT ON public.accepted_completion_invalidation_receipt
    TO ple_scoring_invalidation_source_broker;
CREATE FUNCTION public.ple_reject_accepted_completion_invalidation_mutation()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    RAISE EXCEPTION 'accepted completion invalidation receipt is immutable'
        USING ERRCODE = '55000';
END $$;
CREATE TRIGGER accepted_completion_invalidation_receipt_immutable
    BEFORE UPDATE OR DELETE ON public.accepted_completion_invalidation_receipt
    FOR EACH ROW EXECUTE FUNCTION public.ple_reject_accepted_completion_invalidation_mutation();
REVOKE ALL ON FUNCTION public.ple_reject_accepted_completion_invalidation_mutation()
    FROM PUBLIC;

CREATE UNIQUE INDEX grading_execution_one_completed_receipt_idx
    ON public.grading_execution_receipt (
        tenant_id, attempt_id, submission_id, execution_generation
    ) WHERE resulting_state = 'completed';
CREATE POLICY scoring_invalidation_source_execution_receipt
    ON public.grading_execution_receipt
    FOR SELECT TO ple_scoring_invalidation_source_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY scoring_invalidation_source_worker_job
    ON public.worker_job
    FOR SELECT TO ple_scoring_invalidation_source_broker
    USING (tenant_id = public.ple_current_tenant());
GRANT SELECT ON public.grading_execution_receipt, public.worker_job
    TO ple_scoring_invalidation_source_broker;

CREATE FUNCTION public.ple_bind_accepted_completion_invalidation_v1(
    p_tenant uuid, p_execution_job uuid, p_submission uuid, p_execution_generation bigint,
    p_recalculation_job uuid
) RETURNS TABLE(disposition text, operation_reference integer, scoring_generation bigint,
    recalculation_job_id uuid, origin_id uuid)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE r record;
BEGIN
    -- Worker completion and scoring enqueue own their mutable rows. This
    -- source broker verifies the exact result with read-only authority.
    SELECT completion.receipt_id AS completion_receipt_id,
           execution.attempt_id, execution.course_id, enrollment.assignment_id,
           assignment.scoring_generation INTO r
      FROM public.grading_execution execution
      JOIN public.worker_job execution_job
        ON (execution_job.tenant_id, execution_job.job_id)
         = (execution.tenant_id, execution.current_job_id)
      JOIN public.grading_execution_receipt completion
        ON completion.tenant_id=execution.tenant_id
       AND completion.attempt_id=execution.attempt_id
       AND completion.submission_id=execution.submission_id
       AND completion.course_id=execution.course_id
       AND completion.execution_generation=execution.execution_generation
       AND completion.resulting_state='completed'
      JOIN public.question_attempt attempt ON (attempt.tenant_id,attempt.attempt_id)=(execution.tenant_id,execution.attempt_id)
      JOIN public.assignment_run run_row ON (run_row.tenant_id,run_row.run_id)=(attempt.tenant_id,attempt.run_id)
      JOIN public.enrollment enrollment ON (enrollment.tenant_id,enrollment.enrollment_id)=(run_row.tenant_id,run_row.enrollment_id)
      JOIN public.assignment assignment ON (assignment.tenant_id,assignment.assignment_id)=(enrollment.tenant_id,enrollment.assignment_id)
     WHERE execution.tenant_id=p_tenant AND execution.current_job_id=p_execution_job
       AND execution.submission_id=p_submission
       AND execution.execution_generation=p_execution_generation
       AND execution.state='completed'
       AND execution_job.state='completed'
       AND execution_job.payload=jsonb_build_object(
            'kind','gradeAcceptedSubmission',
            'attempt',execution.attempt_id::text,
            'submission',execution.submission_id::text,
            'execution_generation',execution.execution_generation
       )
       AND assignment.scoring_status='recalculating';
    IF NOT FOUND THEN RAISE EXCEPTION 'accepted-completion invalidation witness is unavailable' USING ERRCODE='55000'; END IF;
    INSERT INTO public.accepted_completion_invalidation_receipt (
        tenant_id, recalculation_job_id, completion_receipt_id,
        execution_job_id, attempt_id,
        submission_id, execution_generation, course_id, assignment_id,
        scoring_generation
    ) VALUES (
        p_tenant, p_recalculation_job, r.completion_receipt_id,
        p_execution_job, r.attempt_id,
        p_submission, p_execution_generation, r.course_id, r.assignment_id,
        r.scoring_generation
    ) ON CONFLICT ON CONSTRAINT accepted_completion_invalidation_receipt_pkey
        DO NOTHING;
    IF NOT FOUND AND NOT EXISTS (
        SELECT 1 FROM public.accepted_completion_invalidation_receipt receipt
         WHERE receipt.tenant_id=p_tenant AND receipt.recalculation_job_id=p_recalculation_job
           AND receipt.execution_job_id=p_execution_job AND receipt.attempt_id=r.attempt_id
           AND receipt.completion_receipt_id=r.completion_receipt_id
           AND receipt.submission_id=p_submission AND receipt.execution_generation=p_execution_generation
           AND receipt.course_id=r.course_id AND receipt.assignment_id=r.assignment_id
           AND receipt.scoring_generation=r.scoring_generation
    ) THEN
        RAISE EXCEPTION 'accepted-completion invalidation receipt conflicts'
            USING ERRCODE='55000';
    END IF;
    RETURN QUERY SELECT * FROM public.ple_bind_scoring_invalidation_origin_v1(
        p_tenant,r.course_id,r.assignment_id,r.scoring_generation,p_recalculation_job,
        'accepted_submission_completion',p_submission,NULL,NULL
    );
END $$;

ALTER FUNCTION public.ple_bind_manual_grade_invalidation_v1(uuid,uuid,uuid)
    OWNER TO ple_scoring_invalidation_source_broker;
ALTER FUNCTION public.ple_bind_attempt_support_invalidation_v1(uuid,uuid,uuid)
    OWNER TO ple_scoring_invalidation_source_broker;
ALTER FUNCTION public.ple_bind_assignment_definition_invalidation_v1(
    uuid,uuid,uuid,uuid,bigint,uuid
) OWNER TO ple_scoring_invalidation_source_broker;
ALTER FUNCTION public.ple_bind_accepted_completion_invalidation_v1(
    uuid,uuid,uuid,bigint,uuid
) OWNER TO ple_scoring_invalidation_source_broker;
ALTER FUNCTION public.ple_reject_accepted_completion_invalidation_mutation()
    OWNER TO ple_scoring_invalidation_source_broker;
REVOKE ALL ON FUNCTION public.ple_bind_manual_grade_invalidation_v1(uuid,uuid,uuid),
    public.ple_bind_attempt_support_invalidation_v1(uuid,uuid,uuid),
    public.ple_bind_assignment_definition_invalidation_v1(uuid,uuid,uuid,uuid,bigint,uuid),
    public.ple_bind_accepted_completion_invalidation_v1(uuid,uuid,uuid,bigint,uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_bind_manual_grade_invalidation_v1(uuid,uuid,uuid),
    public.ple_bind_attempt_support_invalidation_v1(uuid,uuid,uuid),
    public.ple_bind_assignment_definition_invalidation_v1(uuid,uuid,uuid,uuid,bigint,uuid)
    TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_bind_accepted_completion_invalidation_v1(
    uuid,uuid,uuid,bigint,uuid
) TO ple_accepted_submission_execution, ple_accepted_submission_execution_fast_path;
GRANT EXECUTE ON FUNCTION public.ple_assignment_mutator_require_editor(
    uuid,uuid,uuid,uuid,bigint
) TO ple_scoring_invalidation_source_broker;

DO $$
BEGIN
    IF NOT has_function_privilege(
        'ple_scoring_invalidation_source_broker',
        'public.ple_current_tenant()', 'EXECUTE'
    ) OR NOT has_function_privilege(
        'ple_scoring_invalidation_source_broker',
        'public.ple_course_records_accessible(uuid,uuid)', 'EXECUTE'
    ) OR NOT has_function_privilege(
        'ple_scoring_invalidation_source_broker',
        'public.ple_assignment_mutator_require_editor(uuid,uuid,uuid,uuid,bigint)',
        'EXECUTE'
    ) OR NOT has_function_privilege(
        'ple_instructor_grading_operation_broker',
        'public.ple_request_scoring_invalidation_v1(uuid,uuid,uuid,text,uuid,uuid,uuid,integer)',
        'EXECUTE'
    ) OR has_function_privilege(
        'ple_app',
        'public.ple_request_scoring_invalidation_v1(uuid,uuid,uuid,text,uuid,uuid,uuid,integer)',
        'EXECUTE'
    ) OR has_function_privilege(
        'ple_app',
        'public.ple_bind_scoring_invalidation_origin_v1(uuid,uuid,uuid,bigint,uuid,text,uuid,uuid,integer)',
        'EXECUTE'
    ) OR has_table_privilege(
        'ple_app', 'public.accepted_completion_invalidation_receipt',
        'INSERT,UPDATE,DELETE'
    ) OR has_table_privilege(
        'ple_scoring_invalidation_source_broker',
        'public.accepted_completion_invalidation_receipt', 'UPDATE,DELETE,TRUNCATE,TRIGGER'
    ) OR EXISTS (
        SELECT 1 FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid = 'ple_scoring_invalidation_source_broker'::regrole
            OR membership.member = 'ple_scoring_invalidation_source_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'Instructor invalidation source binding is unsafe';
    END IF;
END;
$$;

COMMIT;
