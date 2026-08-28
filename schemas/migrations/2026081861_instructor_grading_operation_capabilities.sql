-- WP-PROF-G1 W5: least-authority Instructor automated-grading operations.
--
-- The API receives only safe operation/group/action facts through these
-- capabilities.  The definer role owns the small mutation surface; ple_app
-- receives no direct table or sequence authority (ASVS 8.2.1-8.2.3, 8.4.1).

BEGIN;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_roles
         WHERE rolname = 'ple_instructor_grading_operation_broker'
    ) THEN
        CREATE ROLE ple_instructor_grading_operation_broker
            NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOREPLICATION NOBYPASSRLS;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_auth_members AS member
         WHERE member.roleid = 'ple_instructor_grading_operation_broker'::regrole
            OR member.member = 'ple_instructor_grading_operation_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'Instructor grading-operation broker must have no memberships';
    END IF;
END
$$;

ALTER ROLE ple_instructor_grading_operation_broker
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;
REVOKE ALL ON SCHEMA public FROM ple_instructor_grading_operation_broker;
GRANT USAGE ON SCHEMA public TO ple_instructor_grading_operation_broker;

-- The W5 route uses functions, never the mutable operation or queue tables.
REVOKE ALL ON public.grading_operation, public.grading_operation_receipt
    FROM ple_app;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public
    FROM ple_instructor_grading_operation_broker;
REVOKE ALL ON SEQUENCE public.grading_operation_grading_operation_id_seq FROM ple_app;
GRANT USAGE ON SEQUENCE public.grading_operation_grading_operation_id_seq
    TO ple_instructor_grading_operation_broker;

-- W2 made receipts append-only.  W5 extends that immutable record with the
-- exact safe action result so replay never derives a placeholder from mutable
-- execution state (ASVS 2.3.1-2.3.3).
ALTER TABLE public.grading_operation_receipt
    DROP CONSTRAINT grading_operation_receipt_revision_check,
    DROP COLUMN expected_revision,
    DROP COLUMN resulting_revision,
    ADD COLUMN resulting_execution_generation bigint,
    ADD COLUMN resulting_scoring_generation bigint,
    ADD COLUMN resulting_state text,
    ADD COLUMN retry_expected_operation_revision bigint,
    ADD COLUMN retry_resulting_operation_revision bigint,
    ADD COLUMN recalculate_expected_assignment_revision bigint,
    ADD COLUMN recalculate_created_operation_revision bigint;
ALTER TABLE public.grading_operation_receipt
    ADD CONSTRAINT grading_operation_receipt_result_check CHECK (
        (action_kind = 'retry'
            AND retry_expected_operation_revision IS NOT NULL
            AND retry_expected_operation_revision > 0
            AND retry_resulting_operation_revision IS NOT NULL
            AND retry_resulting_operation_revision = retry_expected_operation_revision + 1
            AND recalculate_expected_assignment_revision IS NULL
            AND recalculate_created_operation_revision IS NULL
            AND resulting_execution_generation IS NOT NULL
            AND resulting_execution_generation > 0
            AND resulting_scoring_generation IS NULL
            AND resulting_state IS NOT NULL
            AND resulting_state = 'ready')
        OR (action_kind = 'recalculate'
            AND retry_expected_operation_revision IS NULL
            AND retry_resulting_operation_revision IS NULL
            AND recalculate_expected_assignment_revision IS NOT NULL
            AND recalculate_expected_assignment_revision > 0
            AND recalculate_created_operation_revision IS NOT NULL
            AND recalculate_created_operation_revision > 0
            AND resulting_execution_generation IS NULL
            AND resulting_scoring_generation IS NOT NULL
            AND resulting_scoring_generation > 0
            AND resulting_state IS NOT NULL
            AND resulting_state = 'recalculating')
    );

ALTER TABLE public.grading_operation
    DROP CONSTRAINT grading_operation_reason_check,
    ADD CONSTRAINT grading_operation_reason_check CHECK (
        reason = ANY ('{grader_contract_failure,grader_execution_failure,
            issued_evidence_integrity,retry_exhausted,scoring_recalculation_failed,
            instructor_requested_recalculation}')
    );

CREATE POLICY instructor_grading_operation_broker_session
    ON public.auth_session FOR SELECT TO ple_instructor_grading_operation_broker
    USING (
        tenant_id = public.ple_current_tenant()
        AND session_hash = NULLIF(current_setting('ple.session_hash', true), '')::character(64)
    );
CREATE POLICY instructor_grading_operation_broker_session_lock
    ON public.auth_session FOR UPDATE TO ple_instructor_grading_operation_broker
    USING (
        tenant_id = public.ple_current_tenant()
        AND session_hash = NULLIF(current_setting('ple.session_hash', true), '')::character(64)
    ) WITH CHECK (false);
CREATE POLICY instructor_grading_operation_broker_course
    ON public.course FOR SELECT TO ple_instructor_grading_operation_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY instructor_grading_operation_broker_member
    ON public.course_member FOR SELECT TO ple_instructor_grading_operation_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY instructor_grading_operation_broker_member_lock
    ON public.course_member FOR UPDATE TO ple_instructor_grading_operation_broker
    USING (tenant_id = public.ple_current_tenant()) WITH CHECK (false);
CREATE POLICY instructor_grading_operation_broker_assignment
    ON public.assignment FOR SELECT TO ple_instructor_grading_operation_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY instructor_grading_operation_broker_assignment_lock
    ON public.assignment FOR UPDATE TO ple_instructor_grading_operation_broker
    USING (tenant_id = public.ple_current_tenant()) WITH CHECK (false);
CREATE POLICY instructor_grading_operation_broker_operation
    ON public.grading_operation FOR ALL TO ple_instructor_grading_operation_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY instructor_grading_operation_broker_receipt_select
    ON public.grading_operation_receipt FOR SELECT
    TO ple_instructor_grading_operation_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY instructor_grading_operation_broker_receipt_insert
    ON public.grading_operation_receipt FOR INSERT
    TO ple_instructor_grading_operation_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY instructor_grading_operation_broker_execution_select
    ON public.grading_execution FOR SELECT TO ple_instructor_grading_operation_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY instructor_grading_operation_broker_attempt
    ON public.question_attempt FOR SELECT TO ple_instructor_grading_operation_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY instructor_grading_operation_broker_run
    ON public.assignment_run FOR SELECT TO ple_instructor_grading_operation_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY instructor_grading_operation_broker_enrollment
    ON public.enrollment FOR SELECT TO ple_instructor_grading_operation_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY instructor_grading_operation_broker_roster_profile
    ON public.course_roster_profile FOR SELECT TO ple_instructor_grading_operation_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY instructor_grading_operation_broker_problem
    ON public.problem FOR SELECT TO ple_instructor_grading_operation_broker
    USING (true);
CREATE POLICY instructor_grading_operation_broker_problem_version
    ON public.problem_version FOR SELECT TO ple_instructor_grading_operation_broker
    USING (true);

-- PostgreSQL requires UPDATE privilege for SELECT ... FOR KEY SHARE. These
-- single-column grants enable the broker's authoritative witness locks; the
-- matching RLS policies use WITH CHECK (false), so they confer no write path.
GRANT SELECT ON public.auth_session, public.course, public.course_member,
    public.assignment, public.grading_operation, public.grading_operation_receipt,
    public.grading_execution, public.question_attempt, public.assignment_run,
    public.enrollment, public.course_roster_profile, public.problem,
    public.problem_version TO ple_instructor_grading_operation_broker;
GRANT UPDATE (session_hash) ON public.auth_session
    TO ple_instructor_grading_operation_broker;
GRANT UPDATE (course_membership_id) ON public.course_member
    TO ple_instructor_grading_operation_broker;
GRANT UPDATE (assignment_id) ON public.assignment
    TO ple_instructor_grading_operation_broker;
GRANT UPDATE (state, revision, next_action, updated_at)
    ON public.grading_operation TO ple_instructor_grading_operation_broker;
GRANT INSERT ON public.grading_operation, public.grading_operation_receipt
    TO ple_instructor_grading_operation_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(),
    public.ple_course_records_accessible(uuid, uuid),
    public.ple_enqueue_assignment_recalculation(uuid, uuid, uuid, integer)
    TO ple_instructor_grading_operation_broker;

-- One locked witness is deliberately shared by every public capability.  It
-- checks the presented session, current Instructor role, active membership,
-- tenant, accessible course, and exact assignment in the same transaction.
CREATE FUNCTION public.ple_instructor_grading_operation_actor_v1(
    p_tenant_id uuid, p_session character(64), p_course_id uuid, p_assignment_id uuid
) RETURNS uuid
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_actor_id uuid;
BEGIN
    IF p_tenant_id IS NULL OR p_session IS NULL OR p_course_id IS NULL
       OR p_assignment_id IS NULL
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant() THEN
        RETURN NULL;
    END IF;
    SELECT session.user_id INTO v_actor_id
      FROM public.auth_session AS session
     WHERE session.tenant_id = p_tenant_id
       AND session.session_hash = p_session
       AND session.revoked_at IS NULL
       AND session.expires_at > transaction_timestamp()
       AND session.roles @> '["instructor"]'::jsonb
     FOR KEY SHARE;
    IF NOT FOUND OR NOT public.ple_course_records_accessible(p_tenant_id, p_course_id) THEN
        RETURN NULL;
    END IF;
    PERFORM 1 FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant_id AND member.course_id = p_course_id
       AND member.user_id = v_actor_id AND member.role = 'instructor'
       AND member.status = 'active'
     FOR KEY SHARE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    PERFORM 1 FROM public.assignment AS assignment_row
     WHERE assignment_row.tenant_id = p_tenant_id
       AND assignment_row.course_id = p_course_id
       AND assignment_row.assignment_id = p_assignment_id
     FOR KEY SHARE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    RETURN v_actor_id;
END;
$$;

-- List rows are the complete W5 metadata surface.  They contain no response,
-- evaluation payload, feedback, score, or durable private identifier.  The
-- caller selects a bounded grouping; group keys use public Question IDs or
-- public course-membership references, never attempt, submission, job, or UUID identities.
CREATE FUNCTION public.ple_list_instructor_grading_operations_v1(
    p_tenant_id uuid, p_session character(64), p_course_id uuid,
    p_assignment_id uuid, p_group_by text, p_after_group_key text,
    p_after_operation_reference integer, p_limit integer
) RETURNS TABLE(
    operation_reference integer, group_kind text, group_key text, group_label text,
    question_id character(7), question_title text, course_membership_reference integer,
    learner_display_name text,
    affected_learner_count bigint, target_kind text, reason text, operation_state text,
    operation_revision bigint, next_action text, execution_generation bigint,
    assignment_scoring_generation bigint, assignment_scoring_status text,
    updated_at_millis bigint
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_actor_id uuid; v_query_limit integer;
BEGIN
    -- p_limit is the public page-size contract. Keep the SQL-only overfetch
    -- bounded here rather than requiring callers to pass 101 for a 100-row
    -- page, which would make the public and PostgreSQL contracts diverge.
    IF p_group_by NOT IN ('question', 'learner') OR p_limit NOT BETWEEN 1 AND 100
       OR (p_after_group_key IS NULL) <> (p_after_operation_reference IS NULL)
       OR (p_after_operation_reference IS NOT NULL AND p_after_operation_reference < 1) THEN
        RAISE EXCEPTION 'Instructor grading-operation list arguments are invalid'
            USING ERRCODE = '22023';
    END IF;
    v_query_limit := p_limit + 1;
    v_actor_id := public.ple_instructor_grading_operation_actor_v1(
        p_tenant_id, p_session, p_course_id, p_assignment_id
    );
    IF v_actor_id IS NULL THEN RETURN; END IF;
    RETURN QUERY
    WITH visible AS (
        SELECT operation_row.grading_operation_id::integer AS reference,
               CASE WHEN operation_row.target_kind = 'assignment_scoring_generation'
                    THEN 'assignment'
                    WHEN p_group_by = 'question' THEN 'question' ELSE 'learner' END AS kind,
               problem.question_id, version.title AS question_title,
               profile.display_name AS learner_display_name,
               CASE WHEN operation_row.target_kind = 'assignment_scoring_generation'
                    THEN 'a'
                    WHEN p_group_by = 'question'
                    THEN 'q:' || COALESCE(problem.question_id::text, 'unavailable')
                    ELSE 'l:' || COALESCE(
                        lpad(member.public_id::text, 10, '0'), 'unavailable'
                    ) END AS key,
               CASE WHEN operation_row.target_kind = 'assignment_scoring_generation'
                    THEN 'Assignment scoring'
                    WHEN p_group_by = 'question'
                    THEN COALESCE(version.title, 'Assignment scoring')
                    ELSE COALESCE(profile.display_name, member.roster_id, 'Learner') END AS label,
               member.public_id AS course_membership_reference,
               CASE WHEN operation_row.target_kind = 'assignment_scoring_generation' THEN (
                    SELECT count(*)::bigint FROM public.enrollment AS affected
                     WHERE affected.tenant_id = operation_row.tenant_id
                       AND affected.course_id = operation_row.course_id
                       AND affected.assignment_id = operation_row.assignment_id
                ) WHEN p_group_by = 'question' THEN (
                    SELECT count(DISTINCT affected.course_membership_id)::bigint
                      FROM public.grading_operation AS affected_operation
                      JOIN public.question_attempt AS affected_attempt
                        ON affected_attempt.tenant_id = affected_operation.tenant_id
                       AND affected_attempt.attempt_id = affected_operation.attempt_id
                      JOIN public.assignment_run AS affected_run
                        ON affected_run.tenant_id = affected_attempt.tenant_id
                       AND affected_run.run_id = affected_attempt.run_id
                      JOIN public.enrollment AS affected
                        ON affected.tenant_id = affected_run.tenant_id
                       AND affected.enrollment_id = affected_run.enrollment_id
                     WHERE affected_operation.tenant_id = operation_row.tenant_id
                       AND affected_operation.assignment_id = operation_row.assignment_id
                       AND affected_attempt.problem_id = attempt.problem_id
                       AND affected_attempt.version_id = attempt.version_id
                ) ELSE 1::bigint END AS affected_count,
               operation_row.target_kind, operation_row.reason, operation_row.state,
               operation_row.revision, operation_row.next_action,
               execution.execution_generation, assignment_row.scoring_generation,
               assignment_row.scoring_status, operation_row.updated_at
          FROM public.grading_operation AS operation_row
          JOIN public.assignment AS assignment_row
            ON assignment_row.tenant_id = operation_row.tenant_id
           AND assignment_row.assignment_id = operation_row.assignment_id
           AND assignment_row.course_id = operation_row.course_id
          LEFT JOIN public.grading_execution AS execution
            ON execution.tenant_id = operation_row.tenant_id
           AND execution.attempt_id = operation_row.attempt_id
           AND execution.submission_id = operation_row.submission_id
          LEFT JOIN public.question_attempt AS attempt
            ON attempt.tenant_id = operation_row.tenant_id
           AND attempt.attempt_id = operation_row.attempt_id
          LEFT JOIN public.assignment_run AS run
            ON run.tenant_id = attempt.tenant_id AND run.run_id = attempt.run_id
          LEFT JOIN public.enrollment AS enrollment
            ON enrollment.tenant_id = run.tenant_id
           AND enrollment.enrollment_id = run.enrollment_id
          LEFT JOIN public.course_member AS member
            ON member.tenant_id = enrollment.tenant_id
           AND member.course_id = operation_row.course_id
           AND member.course_membership_id = enrollment.course_membership_id
           AND member.role = 'student'
          LEFT JOIN public.course_roster_profile AS profile
            ON profile.tenant_id = member.tenant_id AND profile.course_id = member.course_id
           AND profile.course_membership_id = member.course_membership_id
          LEFT JOIN public.problem AS problem
            ON problem.problem_id = attempt.problem_id
          LEFT JOIN public.problem_version AS version
            ON version.problem_id = attempt.problem_id
           AND version.version_id = attempt.version_id
         WHERE operation_row.tenant_id = p_tenant_id
           AND operation_row.course_id = p_course_id
           AND operation_row.assignment_id = p_assignment_id
    )
    SELECT visible_row.reference, visible_row.kind, visible_row.key,
           visible_row.label, visible_row.question_id, visible_row.question_title,
           visible_row.course_membership_reference,
           visible_row.learner_display_name, visible_row.affected_count,
           visible_row.target_kind, visible_row.reason, visible_row.state,
           visible_row.revision, visible_row.next_action,
           visible_row.execution_generation, visible_row.scoring_generation,
           visible_row.scoring_status,
           floor(extract(epoch FROM visible_row.updated_at) * 1000)::bigint
      FROM visible AS visible_row
     WHERE p_after_group_key IS NULL
        OR (visible_row.key, visible_row.reference) > (
            p_after_group_key, p_after_operation_reference
        )
     ORDER BY visible_row.key, visible_row.reference LIMIT v_query_limit;
END;
$$;

-- Retry changes the sealed automated-evaluation lifecycle, so the Instructor
-- broker delegates that exact transition to the existing worker owner.  The
-- public action retains the Instructor lock, receipt, and operation update;
-- this internal capability owns only generation, queue, and pending evidence.
-- Its additional column privileges are exactly the fields this new retry
-- transition mutates; the execution login cannot assume the internal owner.
GRANT INSERT (job_id, tenant_id, payload, state, max_attempts)
    ON public.worker_job TO ple_accepted_submission_execution_worker;
GRANT UPDATE (execution_generation, current_job_id)
    ON public.grading_execution TO ple_accepted_submission_execution_worker;

CREATE FUNCTION public.ple_prepare_accepted_submission_retry_v1(
    p_tenant_id uuid, p_attempt_id uuid, p_submission_id uuid, p_job_id uuid
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
       OR p_job_id IS NULL
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
        resulting_state, worker_id
    ) VALUES (
        p_tenant_id, pg_catalog.gen_random_uuid(), p_attempt_id, p_submission_id,
        v_execution.submission_occurred_at, v_execution.course_id,
        v_execution.execution_generation + 1, 'ready', NULL
    );
    RETURN QUERY SELECT v_execution.execution_generation + 1, 'ready'::text;
END;
$$;

CREATE FUNCTION public.ple_retry_instructor_grading_operation_v1(
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
      FROM public.ple_prepare_accepted_submission_retry_v1(
          p_tenant_id, v_operation.attempt_id, v_operation.submission_id, p_action_id
      );
    UPDATE public.grading_operation SET revision = revision + 1,
        state = 'action_in_progress', next_action = NULL, updated_at = transaction_timestamp()
     WHERE tenant_id = p_tenant_id AND grading_operation_id = v_operation.grading_operation_id;
    v_request_sha256 := encode(pg_catalog.sha256(convert_to(jsonb_build_object(
        'action', 'retry', 'assignment', p_assignment_id::text,
        'operation', p_operation_reference, 'revision', p_expected_revision
    )::text, 'UTF8')), 'hex');
    INSERT INTO public.grading_operation_receipt (
        tenant_id, action_id, grading_operation_id, course_id, actor_id, action_kind,
        request_sha256, retry_expected_operation_revision,
        retry_resulting_operation_revision,
        resulting_execution_generation, resulting_state
    ) VALUES (p_tenant_id, p_action_id, v_operation.grading_operation_id, p_course_id,
        v_actor_id, 'retry', v_request_sha256, p_expected_revision, p_expected_revision + 1,
        v_retry.resulting_execution_generation, v_retry.resulting_state);
    RETURN QUERY SELECT 'accepted', p_operation_reference, p_expected_revision + 1,
        v_retry.resulting_execution_generation, v_retry.resulting_state,
        floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint;
END;
$$;

CREATE FUNCTION public.ple_recalculate_instructor_assignment_v1(
    p_tenant_id uuid, p_session character(64), p_course_id uuid,
    p_assignment_id uuid, p_expected_assignment_revision bigint, p_action_id uuid
) RETURNS TABLE(
    disposition text, operation_reference integer, assignment_revision bigint,
    created_operation_revision bigint, scoring_generation bigint, scoring_status text,
    action_occurred_at_millis bigint
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_actor_id uuid; v_assignment public.assignment%ROWTYPE;
        v_receipt public.grading_operation_receipt%ROWTYPE; v_operation_id bigint;
        v_generation bigint; v_job_id uuid; v_request_sha256 character(64);
BEGIN
    IF p_expected_assignment_revision IS NULL OR p_expected_assignment_revision < 1
       OR p_action_id IS NULL THEN
        RAISE EXCEPTION 'Instructor recalculation arguments are invalid' USING ERRCODE = '22023';
    END IF;
    v_actor_id := public.ple_instructor_grading_operation_actor_v1(
        p_tenant_id, p_session, p_course_id, p_assignment_id
    );
    IF v_actor_id IS NULL THEN RETURN; END IF;
    SELECT * INTO v_assignment FROM public.assignment AS assignment_row
     WHERE assignment_row.tenant_id = p_tenant_id AND assignment_row.course_id = p_course_id
       AND assignment_row.assignment_id = p_assignment_id FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    SELECT * INTO v_receipt FROM public.grading_operation_receipt AS receipt
     WHERE receipt.tenant_id = p_tenant_id AND receipt.action_id = p_action_id;
    IF FOUND THEN
        IF v_receipt.actor_id IS DISTINCT FROM v_actor_id OR v_receipt.action_kind <> 'recalculate'
           OR v_receipt.recalculate_expected_assignment_revision
                <> p_expected_assignment_revision THEN
            RAISE EXCEPTION 'Instructor grading-operation action conflicts' USING ERRCODE = '55000';
        END IF;
        PERFORM 1 FROM public.grading_operation AS operation_row
         WHERE operation_row.tenant_id = p_tenant_id
           AND operation_row.grading_operation_id = v_receipt.grading_operation_id
           AND operation_row.course_id = p_course_id
           AND operation_row.assignment_id = p_assignment_id
           AND operation_row.target_kind = 'assignment_scoring_generation';
        IF NOT FOUND THEN
            RAISE EXCEPTION 'Instructor grading-operation action conflicts' USING ERRCODE = '55000';
        END IF;
        RETURN QUERY SELECT 'replayed', v_receipt.grading_operation_id::integer,
            p_expected_assignment_revision, v_receipt.recalculate_created_operation_revision,
            v_receipt.resulting_scoring_generation,
            v_receipt.resulting_state,
            floor(extract(epoch FROM v_receipt.occurred_at) * 1000)::bigint;
        RETURN;
    END IF;
    IF v_assignment.revision <> p_expected_assignment_revision
       OR v_assignment.scoring_status NOT IN ('current', 'failed') THEN
        RAISE EXCEPTION 'Instructor assignment revision conflicts' USING ERRCODE = '55000';
    END IF;
    v_job_id := p_action_id;
    IF EXISTS (
        SELECT 1 FROM public.worker_job AS job
         WHERE job.tenant_id = p_tenant_id AND job.job_id = v_job_id
    ) THEN
        RAISE EXCEPTION 'Instructor grading-operation action conflicts' USING ERRCODE = '55000';
    END IF;
    v_generation := public.ple_enqueue_assignment_recalculation(
        p_tenant_id, p_assignment_id, v_job_id, 10
    );
    INSERT INTO public.grading_operation (
        tenant_id, assignment_id, course_id, target_kind, requested_scoring_generation,
        reason, state, revision, next_action
    ) VALUES (
        p_tenant_id, p_assignment_id, p_course_id, 'assignment_scoring_generation', v_generation,
        'instructor_requested_recalculation', 'action_in_progress', 1, NULL
    ) RETURNING grading_operation_id INTO v_operation_id;
    v_request_sha256 := encode(pg_catalog.sha256(convert_to(jsonb_build_object(
        'action', 'recalculate', 'assignment', p_assignment_id::text,
        'revision', p_expected_assignment_revision
    )::text, 'UTF8')), 'hex');
    INSERT INTO public.grading_operation_receipt (
        tenant_id, action_id, grading_operation_id, course_id, actor_id, action_kind,
        request_sha256, recalculate_expected_assignment_revision,
        recalculate_created_operation_revision,
        resulting_scoring_generation, resulting_state
    ) VALUES (p_tenant_id, p_action_id, v_operation_id, p_course_id, v_actor_id,
        'recalculate', v_request_sha256, p_expected_assignment_revision, 1,
        v_generation, 'recalculating');
    RETURN QUERY SELECT 'accepted', v_operation_id::integer,
        p_expected_assignment_revision, 1::bigint, v_generation, 'recalculating'::text,
        floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint;
END;
$$;

ALTER FUNCTION public.ple_instructor_grading_operation_actor_v1(uuid, character, uuid, uuid)
    OWNER TO ple_instructor_grading_operation_broker;
ALTER FUNCTION public.ple_list_instructor_grading_operations_v1(
    uuid, character, uuid, uuid, text, text, integer, integer
) OWNER TO ple_instructor_grading_operation_broker;
ALTER FUNCTION public.ple_prepare_accepted_submission_retry_v1(uuid, uuid, uuid, uuid)
    OWNER TO ple_accepted_submission_execution_worker;
ALTER FUNCTION public.ple_retry_instructor_grading_operation_v1(
    uuid, character, uuid, uuid, integer, bigint, uuid
) OWNER TO ple_instructor_grading_operation_broker;
ALTER FUNCTION public.ple_recalculate_instructor_assignment_v1(
    uuid, character, uuid, uuid, bigint, uuid
) OWNER TO ple_instructor_grading_operation_broker;

REVOKE ALL ON FUNCTION public.ple_instructor_grading_operation_actor_v1(uuid, character, uuid, uuid)
    FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_list_instructor_grading_operations_v1(
    uuid, character, uuid, uuid, text, text, integer, integer
) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_prepare_accepted_submission_retry_v1(
    uuid, uuid, uuid, uuid
) FROM PUBLIC, ple_app, ple_accepted_submission_execution,
    ple_accepted_submission_execution_fast_path;
GRANT EXECUTE ON FUNCTION public.ple_prepare_accepted_submission_retry_v1(
    uuid, uuid, uuid, uuid
) TO ple_instructor_grading_operation_broker;
REVOKE ALL ON FUNCTION public.ple_retry_instructor_grading_operation_v1(
    uuid, character, uuid, uuid, integer, bigint, uuid
) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_recalculate_instructor_assignment_v1(
    uuid, character, uuid, uuid, bigint, uuid
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_list_instructor_grading_operations_v1(
    uuid, character, uuid, uuid, text, text, integer, integer
), public.ple_retry_instructor_grading_operation_v1(
    uuid, character, uuid, uuid, integer, bigint, uuid
), public.ple_recalculate_instructor_assignment_v1(
    uuid, character, uuid, uuid, bigint, uuid
) TO ple_app;

DO $$
DECLARE
    v_functions regprocedure[] := ARRAY[
        (
            'public.ple_list_instructor_grading_operations_v1'
            '(uuid,character,uuid,uuid,text,text,integer,integer)'
        )::regprocedure,
        (
            'public.ple_retry_instructor_grading_operation_v1'
            '(uuid,character,uuid,uuid,integer,bigint,uuid)'
        )::regprocedure,
        (
            'public.ple_recalculate_instructor_assignment_v1'
            '(uuid,character,uuid,uuid,bigint,uuid)'
        )::regprocedure
    ];
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_proc AS procedure_row
        CROSS JOIN LATERAL pg_catalog.aclexplode(COALESCE(
            procedure_row.proacl, pg_catalog.acldefault('f', procedure_row.proowner)
        )) AS privilege_row
        WHERE procedure_row.oid = ANY(v_functions) AND privilege_row.grantee = 0
          AND privilege_row.privilege_type = 'EXECUTE'
    ) OR EXISTS (
        SELECT 1 FROM unnest(v_functions) AS function_row
        WHERE NOT pg_catalog.has_function_privilege('ple_app', function_row, 'EXECUTE')
    ) OR pg_catalog.has_table_privilege('ple_app', 'public.grading_operation', 'SELECT')
       OR pg_catalog.has_table_privilege('ple_app', 'public.grading_operation_receipt', 'SELECT')
       OR pg_catalog.has_sequence_privilege(
           'ple_app', 'public.grading_operation_grading_operation_id_seq', 'USAGE'
       )
       OR NOT pg_catalog.has_sequence_privilege(
           'ple_instructor_grading_operation_broker',
           'public.grading_operation_grading_operation_id_seq', 'USAGE'
       )
       OR NOT pg_catalog.has_function_privilege(
           'ple_instructor_grading_operation_broker',
           'public.ple_enqueue_assignment_recalculation(uuid,uuid,uuid,integer)', 'EXECUTE'
       ) OR NOT pg_catalog.has_function_privilege(
           'ple_instructor_grading_operation_broker',
           'public.ple_prepare_accepted_submission_retry_v1(uuid,uuid,uuid,uuid)',
           'EXECUTE'
       ) OR pg_catalog.has_function_privilege(
           'ple_app',
           'public.ple_prepare_accepted_submission_retry_v1(uuid,uuid,uuid,uuid)',
           'EXECUTE'
       ) OR EXISTS (
           SELECT 1
             FROM unnest(ARRAY[
                 'job_id', 'tenant_id', 'payload', 'state', 'max_attempts'
             ]) AS required_column(column_name)
            WHERE NOT pg_catalog.has_column_privilege(
                'ple_accepted_submission_execution_worker', 'public.worker_job',
                required_column.column_name, 'INSERT'
            )
       ) OR EXISTS (
           SELECT 1
             FROM unnest(ARRAY[
                 'execution_generation', 'current_job_id'
             ]) AS required_column(column_name)
            WHERE NOT pg_catalog.has_column_privilege(
                'ple_accepted_submission_execution_worker', 'public.grading_execution',
                required_column.column_name, 'UPDATE'
            )
       ) OR pg_catalog.has_table_privilege(
           'ple_instructor_grading_operation_broker',
           'public.submission_evaluation', 'SELECT,INSERT,UPDATE,DELETE'
       ) OR pg_catalog.has_table_privilege(
           'ple_instructor_grading_operation_broker',
           'public.worker_job', 'SELECT,INSERT,UPDATE,DELETE'
       ) OR pg_catalog.has_table_privilege(
           'ple_instructor_grading_operation_broker',
           'public.grading_operation_receipt', 'UPDATE,DELETE,TRUNCATE,TRIGGER'
       ) OR EXISTS (
           SELECT 1 FROM pg_catalog.pg_proc AS procedure_row
            WHERE procedure_row.oid =
                'public.ple_prepare_accepted_submission_retry_v1(uuid,uuid,uuid,uuid)'::regprocedure
              AND (
                  NOT procedure_row.prosecdef
                  OR procedure_row.proowner <>
                     'ple_accepted_submission_execution_worker'::regrole
              )
       ) OR EXISTS (
           SELECT 1 FROM pg_catalog.pg_roles AS role_row
            WHERE role_row.rolname = 'ple_instructor_grading_operation_broker'
              AND (role_row.rolcanlogin OR role_row.rolinherit OR role_row.rolsuper
                   OR role_row.rolcreatedb OR role_row.rolcreaterole
                   OR role_row.rolreplication OR role_row.rolbypassrls)
       ) THEN
        RAISE EXCEPTION 'Instructor grading-operation capability privilege matrix is unsafe';
    END IF;
END;
$$;

COMMIT;
