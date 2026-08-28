-- WP-PROF-G1 / G1-W5: one capability owns every scoring invalidation closure.
--
-- The capability links the origin, exact generation, exact queue job, and
-- assignment-scoring operation in one transaction.  Migration 1830 remains
-- the sole generation/job allocator and 1831 remains the sole score publisher.

BEGIN;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_roles
         WHERE rolname = 'ple_scoring_invalidation_origin_broker'
    ) THEN
        CREATE ROLE ple_scoring_invalidation_origin_broker
            NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOREPLICATION NOBYPASSRLS;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid = 'ple_scoring_invalidation_origin_broker'::regrole
            OR membership.member = 'ple_scoring_invalidation_origin_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'scoring invalidation origin broker must have no memberships';
    END IF;
END;
$$;

ALTER ROLE ple_scoring_invalidation_origin_broker
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;
REVOKE ALL ON SCHEMA public FROM ple_scoring_invalidation_origin_broker;
GRANT USAGE ON SCHEMA public TO ple_scoring_invalidation_origin_broker;

ALTER TABLE public.grading_operation
    DROP CONSTRAINT grading_operation_reason_check,
    ADD CONSTRAINT grading_operation_reason_check CHECK (
        reason = ANY ('{grader_contract_failure,grader_execution_failure,
            issued_evidence_integrity,retry_exhausted,scoring_recalculation_failed,
            instructor_requested_recalculation,scoring_recalculation_requested}')
    );

CREATE POLICY scoring_invalidation_origin_broker_origin
    ON public.scoring_invalidation_origin FOR ALL
    TO ple_scoring_invalidation_origin_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY scoring_invalidation_origin_broker_assignment
    ON public.assignment FOR SELECT
    TO ple_scoring_invalidation_origin_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY scoring_invalidation_origin_broker_operation
    ON public.grading_operation FOR ALL
    TO ple_scoring_invalidation_origin_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY scoring_invalidation_origin_broker_job
    ON public.worker_job FOR SELECT
    TO ple_scoring_invalidation_origin_broker
    USING (tenant_id = public.ple_current_tenant());

GRANT SELECT, INSERT ON public.scoring_invalidation_origin
    TO ple_scoring_invalidation_origin_broker;
GRANT SELECT ON public.assignment, public.worker_job, public.grading_operation
    TO ple_scoring_invalidation_origin_broker;
GRANT INSERT, UPDATE (state, revision, next_action, updated_at)
    ON public.grading_operation TO ple_scoring_invalidation_origin_broker;
GRANT USAGE ON SEQUENCE public.grading_operation_grading_operation_id_seq
    TO ple_scoring_invalidation_origin_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(),
    public.ple_enqueue_assignment_recalculation(uuid, uuid, uuid, integer)
    TO ple_scoring_invalidation_origin_broker;

-- Bind a source broker's already-created generation and queue row.  The
-- source retains its domain mutation, while this capability creates or checks
-- the one operation and immutable causal record before commit.
CREATE FUNCTION public.ple_bind_scoring_invalidation_origin_v1(
    p_tenant_id uuid, p_course_id uuid, p_assignment_id uuid,
    p_scoring_generation bigint, p_recalculation_job_id uuid,
    p_origin_kind text, p_origin_id uuid, p_actor_id uuid DEFAULT NULL,
    p_existing_operation_reference integer DEFAULT NULL
) RETURNS TABLE(
    disposition text, operation_reference integer, scoring_generation bigint,
    recalculation_job_id uuid, origin_id uuid
)
LANGUAGE plpgsql
VOLATILE
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    v_origin public.scoring_invalidation_origin%ROWTYPE;
    v_operation public.grading_operation%ROWTYPE;
    v_operation_id bigint;
BEGIN
    IF p_tenant_id IS NULL OR p_course_id IS NULL OR p_assignment_id IS NULL
       OR p_scoring_generation IS NULL OR p_scoring_generation < 1
       OR p_recalculation_job_id IS NULL OR p_origin_id IS NULL
       OR p_origin_kind NOT IN (
           'instructor_recalculation', 'assignment_definition', 'manual_grade',
           'learner_support', 'accepted_submission_completion'
       ) OR (
           p_origin_kind IN (
               'instructor_recalculation', 'assignment_definition', 'manual_grade',
               'learner_support'
           ) AND p_actor_id IS NULL
       ) OR (
           p_origin_kind = 'accepted_submission_completion' AND p_actor_id IS NOT NULL
       )
       OR (p_existing_operation_reference IS NOT NULL AND p_existing_operation_reference < 1)
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'scoring invalidation origin arguments are invalid' USING ERRCODE = '22023';
    END IF;

    -- Origin evidence is immutable. A plain read preserves the broker's
    -- read-only authority; the source transaction owns the mutable generation
    -- and job while the deferred closure guards revalidate both before commit.
    SELECT * INTO v_origin
     FROM public.scoring_invalidation_origin AS origin_row
     WHERE origin_row.tenant_id = p_tenant_id
       AND origin_row.origin_kind = p_origin_kind
       AND origin_row.origin_id = p_origin_id;
    IF FOUND THEN
        IF v_origin.course_id IS DISTINCT FROM p_course_id
           OR v_origin.assignment_id IS DISTINCT FROM p_assignment_id
           OR v_origin.scoring_generation IS DISTINCT FROM p_scoring_generation
           OR v_origin.recalculation_job_id IS DISTINCT FROM p_recalculation_job_id
           OR v_origin.origin_kind IS DISTINCT FROM p_origin_kind
           OR v_origin.actor_id IS DISTINCT FROM p_actor_id
           OR (p_existing_operation_reference IS NOT NULL
               AND v_origin.grading_operation_id <> p_existing_operation_reference) THEN
            RAISE EXCEPTION 'scoring invalidation origin conflicts' USING ERRCODE = '55000';
        END IF;
        RETURN QUERY SELECT 'replayed', v_origin.grading_operation_id::integer,
            v_origin.scoring_generation, v_origin.recalculation_job_id, v_origin.origin_id;
        RETURN;
    END IF;

    PERFORM 1
      FROM public.assignment AS assignment_row
     WHERE assignment_row.tenant_id = p_tenant_id
       AND assignment_row.course_id = p_course_id
       AND assignment_row.assignment_id = p_assignment_id
       AND assignment_row.scoring_generation = p_scoring_generation
       AND assignment_row.scoring_status = 'recalculating';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'scoring invalidation generation is unavailable' USING ERRCODE = '55000';
    END IF;

    PERFORM 1
      FROM public.worker_job AS job_row
     WHERE job_row.tenant_id = p_tenant_id
       AND job_row.job_id = p_recalculation_job_id
       AND job_row.payload = jsonb_build_object(
           'kind', 'recalculateAssignment', 'assignment', p_assignment_id::text,
           'generation', p_scoring_generation
       );
    IF NOT FOUND THEN
        RAISE EXCEPTION 'scoring invalidation job is unavailable' USING ERRCODE = '55000';
    END IF;

    IF p_existing_operation_reference IS NULL THEN
        INSERT INTO public.grading_operation (
            tenant_id, assignment_id, course_id, target_kind,
            requested_scoring_generation, reason, state, revision, next_action
        ) VALUES (
            p_tenant_id, p_assignment_id, p_course_id,
            'assignment_scoring_generation', p_scoring_generation,
            CASE WHEN p_origin_kind = 'instructor_recalculation'
                THEN 'instructor_requested_recalculation'
                ELSE 'scoring_recalculation_requested' END,
            'action_in_progress', 1, NULL
        ) RETURNING grading_operation_id INTO v_operation_id;
    ELSE
        SELECT * INTO v_operation
          FROM public.grading_operation AS operation_row
         WHERE operation_row.tenant_id = p_tenant_id
           AND operation_row.course_id = p_course_id
           AND operation_row.assignment_id = p_assignment_id
           AND operation_row.grading_operation_id = p_existing_operation_reference
           AND operation_row.target_kind = 'assignment_scoring_generation'
           AND operation_row.requested_scoring_generation = p_scoring_generation
         FOR KEY SHARE;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'scoring invalidation operation is unavailable' USING ERRCODE = '55000';
        END IF;
        v_operation_id := v_operation.grading_operation_id;
    END IF;

    UPDATE public.grading_operation AS operation_row
       SET state = 'superseded', next_action = NULL,
           revision = operation_row.revision + 1,
           updated_at = transaction_timestamp()
     WHERE operation_row.tenant_id = p_tenant_id
       AND operation_row.course_id = p_course_id
       AND operation_row.assignment_id = p_assignment_id
       AND operation_row.target_kind = 'assignment_scoring_generation'
       AND operation_row.requested_scoring_generation < p_scoring_generation
       AND (
           operation_row.state = 'action_in_progress'
           OR (operation_row.state = 'actionable'
               AND operation_row.next_action = 'recalculate')
       );

    INSERT INTO public.scoring_invalidation_origin (
        tenant_id, origin_id, course_id, assignment_id, scoring_generation,
        recalculation_job_id, grading_operation_id, origin_kind, actor_id
    ) VALUES (
        p_tenant_id, p_origin_id, p_course_id, p_assignment_id, p_scoring_generation,
        p_recalculation_job_id, v_operation_id, p_origin_kind, p_actor_id
    );

    RETURN QUERY SELECT 'accepted', v_operation_id::integer, p_scoring_generation,
        p_recalculation_job_id, p_origin_id;
END;
$$;

-- The full request owns new generation allocation as well as the causal
-- closure.  Callers select a typed origin; 1830 allocates the exact next
-- generation and queue job under its established authority.
CREATE FUNCTION public.ple_request_scoring_invalidation_v1(
    p_tenant_id uuid, p_course_id uuid, p_assignment_id uuid,
    p_origin_kind text, p_origin_id uuid, p_recalculation_job_id uuid,
    p_actor_id uuid DEFAULT NULL, p_max_attempts integer DEFAULT 10
) RETURNS TABLE(
    disposition text, operation_reference integer, scoring_generation bigint,
    recalculation_job_id uuid, origin_id uuid
)
LANGUAGE plpgsql
VOLATILE
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    v_origin public.scoring_invalidation_origin%ROWTYPE;
    v_generation bigint;
BEGIN
    IF p_tenant_id IS NULL OR p_course_id IS NULL OR p_assignment_id IS NULL
       OR p_origin_id IS NULL OR p_recalculation_job_id IS NULL
       OR p_origin_kind NOT IN (
           'instructor_recalculation', 'assignment_definition', 'manual_grade',
           'learner_support', 'accepted_submission_completion'
       ) OR (
           p_origin_kind IN (
               'instructor_recalculation', 'assignment_definition', 'manual_grade',
               'learner_support'
           ) AND p_actor_id IS NULL
       ) OR (
           p_origin_kind = 'accepted_submission_completion' AND p_actor_id IS NOT NULL
       )
       OR p_max_attempts NOT BETWEEN 1 AND 20
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'scoring invalidation request is invalid' USING ERRCODE = '22023';
    END IF;

    -- The enqueue capability below owns the assignment lock. Keep this broker
    -- read-only until that single mutation authority is entered.
    SELECT * INTO v_origin
     FROM public.scoring_invalidation_origin AS origin_row
     WHERE origin_row.tenant_id = p_tenant_id
       AND origin_row.origin_kind = p_origin_kind
       AND origin_row.origin_id = p_origin_id;
    IF FOUND THEN
        IF v_origin.course_id IS DISTINCT FROM p_course_id
           OR v_origin.assignment_id IS DISTINCT FROM p_assignment_id
           OR v_origin.recalculation_job_id IS DISTINCT FROM p_recalculation_job_id
           OR v_origin.origin_kind IS DISTINCT FROM p_origin_kind
           OR v_origin.actor_id IS DISTINCT FROM p_actor_id THEN
            RAISE EXCEPTION 'scoring invalidation request conflicts' USING ERRCODE = '55000';
        END IF;
        RETURN QUERY SELECT 'replayed', v_origin.grading_operation_id::integer,
            v_origin.scoring_generation, v_origin.recalculation_job_id, v_origin.origin_id;
        RETURN;
    END IF;

    PERFORM 1
      FROM public.assignment AS assignment_row
     WHERE assignment_row.tenant_id = p_tenant_id
       AND assignment_row.course_id = p_course_id
       AND assignment_row.assignment_id = p_assignment_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'scoring invalidation assignment is unavailable' USING ERRCODE = '42501';
    END IF;

    BEGIN
        v_generation := public.ple_enqueue_assignment_recalculation(
            p_tenant_id, p_assignment_id, p_recalculation_job_id, p_max_attempts
        );
    EXCEPTION WHEN unique_violation THEN
        -- A concurrent action can reuse a job identity before either origin is
        -- visible. Preserve the public idempotency contract as Conflict while
        -- this subtransaction rolls back the generation advance.
        RAISE EXCEPTION 'scoring invalidation request conflicts' USING ERRCODE = '55000';
    END;
    RETURN QUERY SELECT * FROM public.ple_bind_scoring_invalidation_origin_v1(
        p_tenant_id, p_course_id, p_assignment_id, v_generation,
        p_recalculation_job_id, p_origin_kind, p_origin_id, p_actor_id, NULL
    );
END;
$$;

-- Both deferred directions are intentional.  They make an accepted-submission
-- source capability bind its already-created generation/job before commit,
-- while allowing the full capability to compose 1830 and the origin in one
-- transaction.
CREATE FUNCTION public.ple_guard_scoring_invalidation_assignment_origin()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
BEGIN
    IF NEW.scoring_status = 'recalculating' AND NOT EXISTS (
        SELECT 1
          FROM public.scoring_invalidation_origin AS origin_row
          JOIN public.grading_operation AS operation_row
            ON (operation_row.tenant_id, operation_row.course_id,
                operation_row.grading_operation_id) = (
                origin_row.tenant_id, origin_row.course_id,
                origin_row.grading_operation_id
            )
          JOIN public.worker_job AS job_row
            ON (job_row.tenant_id, job_row.job_id) = (
                origin_row.tenant_id, origin_row.recalculation_job_id
            )
         WHERE origin_row.tenant_id = NEW.tenant_id
           AND origin_row.course_id = NEW.course_id
           AND origin_row.assignment_id = NEW.assignment_id
           AND origin_row.scoring_generation = NEW.scoring_generation
           AND operation_row.assignment_id = NEW.assignment_id
           AND operation_row.target_kind = 'assignment_scoring_generation'
           AND operation_row.requested_scoring_generation = NEW.scoring_generation
           AND job_row.payload = jsonb_build_object(
               'kind', 'recalculateAssignment', 'assignment', NEW.assignment_id::text,
               'generation', NEW.scoring_generation
           )
    ) THEN
        RAISE EXCEPTION 'recalculating assignment requires an exact invalidation origin'
            USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END;
$$;

CREATE FUNCTION public.ple_guard_scoring_invalidation_job_origin()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    v_assignment_id uuid;
    v_generation bigint;
BEGIN
    IF NEW.payload ->> 'kind' <> 'recalculateAssignment' THEN
        RETURN NEW;
    END IF;
    v_assignment_id := (NEW.payload ->> 'assignment')::uuid;
    v_generation := (NEW.payload ->> 'generation')::bigint;
    IF NOT EXISTS (
        SELECT 1
          FROM public.scoring_invalidation_origin AS origin_row
          JOIN public.grading_operation AS operation_row
            ON (operation_row.tenant_id, operation_row.course_id,
                operation_row.grading_operation_id) = (
                origin_row.tenant_id, origin_row.course_id,
                origin_row.grading_operation_id
            )
          JOIN public.assignment AS assignment_row
            ON (assignment_row.tenant_id, assignment_row.course_id,
                assignment_row.assignment_id) = (
                origin_row.tenant_id, origin_row.course_id, origin_row.assignment_id
            )
         WHERE origin_row.tenant_id = NEW.tenant_id
           AND origin_row.recalculation_job_id = NEW.job_id
           AND origin_row.assignment_id = v_assignment_id
           AND origin_row.scoring_generation = v_generation
           AND operation_row.assignment_id = v_assignment_id
           AND operation_row.target_kind = 'assignment_scoring_generation'
           AND operation_row.requested_scoring_generation = v_generation
           AND assignment_row.scoring_generation = v_generation
           AND assignment_row.scoring_status = 'recalculating'
    ) THEN
        RAISE EXCEPTION 'recalculation job requires an exact invalidation origin'
            USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END;
$$;

CREATE CONSTRAINT TRIGGER scoring_invalidation_assignment_origin_fence
    AFTER INSERT OR UPDATE OF scoring_generation, scoring_status ON public.assignment
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION public.ple_guard_scoring_invalidation_assignment_origin();
CREATE CONSTRAINT TRIGGER scoring_invalidation_job_origin_fence
    AFTER INSERT OR UPDATE OF payload ON public.worker_job
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION public.ple_guard_scoring_invalidation_job_origin();

ALTER FUNCTION public.ple_bind_scoring_invalidation_origin_v1(
    uuid, uuid, uuid, bigint, uuid, text, uuid, uuid, integer
) OWNER TO ple_scoring_invalidation_origin_broker;
ALTER FUNCTION public.ple_request_scoring_invalidation_v1(
    uuid, uuid, uuid, text, uuid, uuid, uuid, integer
) OWNER TO ple_scoring_invalidation_origin_broker;
ALTER FUNCTION public.ple_guard_scoring_invalidation_assignment_origin()
    OWNER TO ple_scoring_invalidation_origin_broker;
ALTER FUNCTION public.ple_guard_scoring_invalidation_job_origin()
    OWNER TO ple_scoring_invalidation_origin_broker;
ALTER FUNCTION public.ple_reject_scoring_invalidation_origin_mutation()
    OWNER TO ple_scoring_invalidation_origin_broker;

REVOKE ALL ON FUNCTION public.ple_bind_scoring_invalidation_origin_v1(
    uuid, uuid, uuid, bigint, uuid, text, uuid, uuid, integer
), public.ple_request_scoring_invalidation_v1(
    uuid, uuid, uuid, text, uuid, uuid, uuid, integer
), public.ple_guard_scoring_invalidation_assignment_origin(),
    public.ple_guard_scoring_invalidation_job_origin()
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_bind_scoring_invalidation_origin_v1(
    uuid, uuid, uuid, bigint, uuid, text, uuid, uuid, integer
) TO ple_scoring_invalidation_origin_broker;
GRANT EXECUTE ON FUNCTION public.ple_request_scoring_invalidation_v1(
    uuid, uuid, uuid, text, uuid, uuid, uuid, integer
) TO ple_scoring_invalidation_origin_broker;

DO $$
DECLARE
    v_capabilities regprocedure[] := ARRAY[
        ('public.ple_bind_scoring_invalidation_origin_v1('
            || 'uuid,uuid,uuid,bigint,uuid,text,uuid,uuid,integer)')::regprocedure,
        'public.ple_request_scoring_invalidation_v1(uuid,uuid,uuid,text,uuid,uuid,uuid,integer)'::regprocedure,
        'public.ple_guard_scoring_invalidation_assignment_origin()'::regprocedure,
        'public.ple_guard_scoring_invalidation_job_origin()'::regprocedure
    ];
BEGIN
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
          CROSS JOIN LATERAL pg_catalog.aclexplode(COALESCE(
              procedure_row.proacl,
              pg_catalog.acldefault('f', procedure_row.proowner)
          )) AS privilege_row
         WHERE procedure_row.oid = ANY(v_capabilities)
           AND privilege_row.grantee = 0
           AND privilege_row.privilege_type = 'EXECUTE'
    ) OR EXISTS (
        SELECT 1 FROM pg_catalog.pg_roles AS role_row
         WHERE role_row.rolname = 'ple_scoring_invalidation_origin_broker'
           AND (role_row.rolcanlogin OR role_row.rolinherit OR role_row.rolsuper
                OR role_row.rolcreatedb OR role_row.rolcreaterole
                OR role_row.rolreplication OR role_row.rolbypassrls)
    ) OR has_table_privilege(
        'ple_app', 'public.scoring_invalidation_origin', 'INSERT,UPDATE,DELETE'
    ) OR has_table_privilege(
        'ple_scoring_invalidation_origin_broker',
        'public.scoring_invalidation_origin', 'UPDATE,DELETE,TRUNCATE,TRIGGER'
    ) OR has_function_privilege(
        'ple_app',
        'public.ple_request_scoring_invalidation_v1(uuid,uuid,uuid,text,uuid,uuid,uuid,integer)',
        'EXECUTE'
    ) OR has_function_privilege(
        'ple_app',
        'public.ple_bind_scoring_invalidation_origin_v1(uuid,uuid,uuid,bigint,uuid,text,uuid,uuid,integer)',
        'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'scoring invalidation origin capability privilege matrix is unsafe';
    END IF;
END;
$$;

COMMIT;
