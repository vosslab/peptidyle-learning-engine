-- WP-PROF-B2: execute-only curriculum-adoption and retention capabilities.
--
-- Durable evidence and schedule integrity are established by migration
-- 2026081838.  This migration composes those relations with existing brokers.

BEGIN;

-- The retention owner validates the exact leased job without exposing its
-- scheduling tables to the B2 broker (ASVS 2.1.1, 2.2.3).
CREATE FUNCTION public.ple_curriculum_adoption_retention_attested_v1(
    p_tenant uuid, p_course uuid, p_job uuid, p_token uuid, p_generation bigint
) RETURNS boolean LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE
    v_prepared_count bigint;
    v_manifest_count bigint;
BEGIN
    IF p_tenant IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_course IS NULL OR p_job IS NULL OR p_token IS NULL
       OR p_generation IS NULL OR p_generation <= 0
    THEN
        RETURN false;
    END IF;

    PERFORM 1
      FROM public.worker_job AS job
      JOIN public.course_retention_dispatch AS dispatch
        ON dispatch.tenant_id = job.tenant_id
       AND dispatch.course_id = p_course
       AND dispatch.stage = 'deleteStudentRecords'
       AND dispatch.generation = p_generation
       AND dispatch.job_id = job.job_id
      JOIN public.course_retention AS retention
        ON retention.tenant_id = dispatch.tenant_id
       AND retention.course_id = dispatch.course_id
       AND retention.generation = dispatch.generation
      JOIN public.course_retention_stage AS stage
        ON stage.tenant_id = dispatch.tenant_id
       AND stage.course_id = dispatch.course_id
       AND stage.stage = dispatch.stage
       AND stage.generation = dispatch.generation
       AND stage.job_id = dispatch.job_id
     WHERE job.tenant_id = p_tenant AND job.job_id = p_job
       AND job.state = 'leased' AND job.lease_token = p_token
       AND job.lease_expires_at > transaction_timestamp()
       AND job.payload = jsonb_build_object(
           'kind', 'retention', 'course', p_course::text,
           'stage', 'deleteStudentRecords', 'generation', p_generation
       )
       AND retention.lifecycle = 'archived'
       AND retention.assignment_disposition = 'delete'
       AND stage.state = 'started' AND stage.lease_token = p_token
     FOR UPDATE OF job, dispatch, retention, stage;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    SELECT manifest.object_count,
           COALESCE((
               SELECT count(*)
                 FROM public.course_retention_cleanup_manifest_object AS object
                WHERE object.tenant_id = manifest.tenant_id
                  AND object.course_id = manifest.course_id
                  AND object.generation = manifest.generation
                  AND object.stage = manifest.stage
           ), 0)
      INTO v_prepared_count, v_manifest_count
      FROM public.course_retention_cleanup_manifest AS manifest
     WHERE manifest.tenant_id = p_tenant
       AND manifest.course_id = p_course
       AND manifest.generation = p_generation
       AND manifest.stage = 'deleteStudentRecords'
       AND manifest.job_id = p_job AND manifest.state = 'prepared'
     FOR UPDATE;

    RETURN FOUND
       AND v_prepared_count IS NOT NULL
       AND v_prepared_count = v_manifest_count;
END $$;
ALTER FUNCTION public.ple_curriculum_adoption_retention_attested_v1(
    uuid, uuid, uuid, uuid, bigint
) OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_curriculum_adoption_retention_attested_v1(
    uuid, uuid, uuid, uuid, bigint
) FROM PUBLIC;

CREATE OR REPLACE FUNCTION public.ple_curriculum_adoption_immutable_refusal_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE
    v_course text;
    v_job uuid;
    v_token uuid;
    v_generation bigint;
BEGIN
    v_course := COALESCE(
        to_jsonb(OLD) ->> 'course_id',
        to_jsonb(OLD) ->> 'destination_course_id'
    );
    IF TG_OP = 'DELETE'
       AND current_user = 'ple_curriculum_adoption_broker'
       AND current_setting('ple.curriculum_adoption_maintenance', true) = 'retention'
       AND OLD.tenant_id::text = current_setting('ple.curriculum_adoption_tenant_id', true)
       AND v_course IS NOT NULL
       AND v_course = current_setting('ple.curriculum_adoption_course_id', true)
    THEN
        BEGIN
            v_job := current_setting('ple.curriculum_adoption_job_id', true)::uuid;
            v_token := current_setting('ple.curriculum_adoption_lease_token', true)::uuid;
            v_generation :=
                current_setting('ple.curriculum_adoption_generation', true)::bigint;
        EXCEPTION WHEN data_exception THEN
            RAISE EXCEPTION 'curriculum adoption immutable evidence is retained'
                USING ERRCODE = 'PBI01';
        END;
        IF public.ple_curriculum_adoption_retention_attested_v1(
            OLD.tenant_id, v_course::uuid, v_job, v_token, v_generation
        ) THEN
            RETURN OLD;
        END IF;
    END IF;
    RAISE EXCEPTION 'curriculum adoption immutable evidence is retained' USING ERRCODE = 'PBI01';
END $$;

CREATE FUNCTION public.ple_curriculum_adoption_actor_v1(
    p_tenant uuid, p_session character(64)
) RETURNS uuid LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_actor uuid;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'curriculum adoption request is invalid' USING ERRCODE = '22023';
    END IF;
    SELECT actor.user_id INTO v_actor
      FROM public.ple_reusable_curriculum_instructor_actor(p_session, p_tenant) AS actor;
    IF NOT FOUND THEN RAISE EXCEPTION 'curriculum adoption actor is unavailable' USING ERRCODE = '42501'; END IF;
    RETURN v_actor;
END $$;

CREATE FUNCTION public.ple_curriculum_adoption_preflight_v1(
    p_tenant uuid, p_session character(64)
) RETURNS boolean LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    PERFORM public.ple_curriculum_adoption_actor_v1(p_tenant, p_session);
    RETURN true;
EXCEPTION WHEN insufficient_privilege THEN
    RETURN false;
END $$;

CREATE FUNCTION public.ple_curriculum_adoption_require_closed_json_v1(p_value jsonb)
RETURNS void LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
BEGIN
    IF p_value IS NULL OR jsonb_typeof(p_value) <> 'object' THEN
        RAISE EXCEPTION 'curriculum adoption request has one closed JSON object' USING ERRCODE = '22023';
    END IF;
END $$;

CREATE FUNCTION public.ple_curriculum_adoption_unavailable_v1(
    p_tenant uuid, p_session character(64), p_request jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    PERFORM public.ple_curriculum_adoption_actor_v1(p_tenant, p_session);
    PERFORM public.ple_curriculum_adoption_require_closed_json_v1(p_request);
    RAISE EXCEPTION 'curriculum adoption operation requires the canonical broker materializer' USING ERRCODE = 'PBI01';
END $$;

CREATE FUNCTION public.ple_preview_fork_alpha_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;
CREATE FUNCTION public.ple_apply_fork_alpha_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;
CREATE FUNCTION public.ple_preview_blueprint_instantiation_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;
CREATE FUNCTION public.ple_apply_blueprint_instantiation_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;
CREATE FUNCTION public.ple_preview_alpha_instantiation_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;
CREATE FUNCTION public.ple_apply_alpha_instantiation_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;
CREATE FUNCTION public.ple_preview_course_rollover_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;
CREATE FUNCTION public.ple_apply_course_rollover_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;
CREATE FUNCTION public.ple_preview_course_term_shift_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;
CREATE FUNCTION public.ple_apply_course_term_shift_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;
CREATE FUNCTION public.ple_preview_assignment_fast_forward_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;
CREATE FUNCTION public.ple_apply_assignment_fast_forward_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;
CREATE FUNCTION public.ple_preview_source_derived_assignment_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;
CREATE FUNCTION public.ple_create_source_derived_assignment_v1(uuid, character(64), jsonb) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path TO 'pg_catalog', 'public', pg_temp AS
$$ SELECT public.ple_curriculum_adoption_unavailable_v1($1, $2, $3) $$;

CREATE FUNCTION public.ple_inspect_curriculum_imports_v1(
    p_tenant uuid, p_session character(64), p_course_reference jsonb
) RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    PERFORM public.ple_curriculum_adoption_actor_v1(p_tenant, p_session);
    IF jsonb_typeof(p_course_reference) <> 'string'
       OR trim(both '"' FROM p_course_reference::text) !~ '^C-[1-9][0-9]*$' THEN
        RAISE EXCEPTION 'curriculum adoption inspection has one course reference' USING ERRCODE = '22023';
    END IF;
    RAISE EXCEPTION 'curriculum adoption inspection requires the canonical broker projection' USING ERRCODE = 'PBI01';
END $$;

CREATE FUNCTION public.ple_reconcile_curriculum_adoption_v1(
    p_tenant uuid, p_session character(64), p_command jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    PERFORM public.ple_curriculum_adoption_actor_v1(p_tenant, p_session);
    PERFORM public.ple_curriculum_adoption_require_closed_json_v1(p_command);
    RAISE EXCEPTION 'curriculum adoption reconciliation requires canonical immutable validation' USING ERRCODE = 'PBI01';
END $$;

CREATE FUNCTION public.ple_purge_curriculum_adoption_for_retention_v1(
    p_tenant uuid, p_course uuid, p_job uuid, p_token uuid, p_generation bigint
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v_attested boolean;
BEGIN
    IF p_tenant IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_course IS NULL
    THEN
        RETURN false;
    END IF;

    v_attested := public.ple_curriculum_adoption_retention_attested_v1(
        p_tenant, p_course, p_job, p_token, p_generation
    );
    IF NOT v_attested THEN
        -- Courses with no adoption state keep their pre-B2 deletion behavior.
        RETURN NOT EXISTS (
            SELECT 1
              FROM public.curriculum_adoption_receipt AS receipt
             WHERE receipt.tenant_id = p_tenant
               AND receipt.destination_course_id = p_course
        );
    END IF;

    -- A valid wrapper invocation records the exact capability even when the
    -- course has no assignments or B2 rows.  Every later assignment trigger in
    -- this transaction therefore reattests the same leased stage.
    PERFORM set_config('ple.curriculum_adoption_maintenance', 'retention', true);
    PERFORM set_config('ple.curriculum_adoption_tenant_id', p_tenant::text, true);
    PERFORM set_config('ple.curriculum_adoption_course_id', p_course::text, true);
    PERFORM set_config('ple.curriculum_adoption_job_id', p_job::text, true);
    PERFORM set_config('ple.curriculum_adoption_lease_token', p_token::text, true);
    PERFORM set_config(
        'ple.curriculum_adoption_generation', p_generation::text, true
    );

    IF NOT EXISTS (
        SELECT 1
          FROM public.curriculum_adoption_receipt AS receipt
         WHERE receipt.tenant_id = p_tenant
           AND receipt.destination_course_id = p_course
    ) THEN
        RETURN true;
    END IF;

    -- The exact receipt-root cascades cover current pointers, assignment
    -- evidence, and whole-course topology.  Alpha fork lineage has no
    -- destination course and is intentionally independent of course retention.
    DELETE FROM public.curriculum_adoption_receipt
     WHERE tenant_id = p_tenant AND destination_course_id = p_course;

    RETURN NOT EXISTS (
        SELECT 1
          FROM public.curriculum_adoption_receipt AS receipt
         WHERE receipt.tenant_id = p_tenant
           AND receipt.destination_course_id = p_course
    );
END $$;

CREATE FUNCTION public.ple_curriculum_adoption_retention_assignment_delete_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v_job uuid;
    v_token uuid;
    v_generation bigint;
BEGIN
    IF TG_OP <> 'DELETE' THEN
        RAISE EXCEPTION 'curriculum adoption assignment retention is delete-only'
            USING ERRCODE = '42501';
    END IF;

    BEGIN
        v_job := current_setting('ple.curriculum_adoption_job_id', true)::uuid;
        v_token := current_setting('ple.curriculum_adoption_lease_token', true)::uuid;
        v_generation :=
            current_setting('ple.curriculum_adoption_generation', true)::bigint;
    EXCEPTION WHEN data_exception THEN
        v_job := NULL;
        v_token := NULL;
        v_generation := NULL;
    END;

    IF NOT public.ple_purge_curriculum_adoption_for_retention_v1(
        OLD.tenant_id, OLD.course_id, v_job, v_token, v_generation
    ) THEN
        RAISE EXCEPTION 'curriculum adoption retention authority is unavailable'
            USING ERRCODE = '42501';
    END IF;
    RETURN OLD;
END $$;
CREATE TRIGGER curriculum_adoption_retention_assignment_delete
BEFORE DELETE ON public.assignment
FOR EACH ROW EXECUTE FUNCTION public.ple_curriculum_adoption_retention_assignment_delete_v1();

ALTER FUNCTION public.ple_commit_delete_retention_work(
    uuid, uuid, uuid, uuid, text, bigint
) RENAME TO ple_commit_delete_retention_work_before_curriculum_adoption;
REVOKE ALL ON FUNCTION public.ple_commit_delete_retention_work_before_curriculum_adoption(
    uuid, uuid, uuid, uuid, text, bigint
) FROM PUBLIC, ple_app, ple_curriculum_adoption_broker;

-- Retention remains the only public owner of the job workflow.  Its B2 access
-- is one execute-only course cleanup capability (ASVS 2.1.1, 8.1.1).
CREATE FUNCTION public.ple_commit_delete_retention_work(
    p_tenant uuid, p_job uuid, p_token uuid, p_course uuid,
    p_stage text, p_generation bigint
) RETURNS boolean LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE
    v_committed boolean;
    v_attested boolean;
BEGIN
    IF p_stage IS DISTINCT FROM 'deleteStudentRecords' THEN
        RETURN public.ple_commit_delete_retention_work_before_curriculum_adoption(
            p_tenant, p_job, p_token, p_course, p_stage, p_generation
        );
    END IF;

    v_attested := public.ple_curriculum_adoption_retention_attested_v1(
        p_tenant, p_course, p_job, p_token, p_generation
    );
    IF NOT v_attested THEN
        -- Preserve the accepted false/conflict or validation result without
        -- granting an unattested request any B2 cleanup side effect.
        RETURN public.ple_commit_delete_retention_work_before_curriculum_adoption(
            p_tenant, p_job, p_token, p_course, p_stage, p_generation
        );
    END IF;

    -- The exception block is an internal subtransaction.  Its private
    -- sentinel rolls back B2 plus accepted cleanup before returning the
    -- accepted false/conflict result.  Genuine serialization failures and
    -- deadlocks are not caught and retain their retryable SQLSTATEs.
    BEGIN
        IF NOT public.ple_purge_curriculum_adoption_for_retention_v1(
            p_tenant, p_course, p_job, p_token, p_generation
        ) THEN
            RAISE EXCEPTION 'curriculum adoption retention authority changed'
                USING ERRCODE = 'PBI02';
        END IF;

        v_committed := public.ple_commit_delete_retention_work_before_curriculum_adoption(
            p_tenant, p_job, p_token, p_course, p_stage, p_generation
        );
        IF NOT v_committed THEN
            RAISE EXCEPTION 'curriculum adoption retention commit conflicted'
                USING ERRCODE = 'PBI02';
        END IF;
    EXCEPTION WHEN SQLSTATE 'PBI02' THEN
        RETURN false;
    END;
    RETURN true;
END $$;

ALTER FUNCTION public.ple_curriculum_adoption_immutable_refusal_v1() OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_bump_course_term_schedule_revision_v1()
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_bump_assignment_schedule_revision_v1()
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_curriculum_adoption_actor_v1(uuid, character) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_curriculum_adoption_preflight_v1(uuid, character) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_curriculum_adoption_require_closed_json_v1(jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_curriculum_adoption_unavailable_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_preview_fork_alpha_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_apply_fork_alpha_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_preview_blueprint_instantiation_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_apply_blueprint_instantiation_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_preview_alpha_instantiation_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_apply_alpha_instantiation_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_preview_course_rollover_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_apply_course_rollover_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_preview_course_term_shift_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_apply_course_term_shift_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_preview_assignment_fast_forward_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_apply_assignment_fast_forward_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_preview_source_derived_assignment_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_create_source_derived_assignment_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_inspect_curriculum_imports_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_reconcile_curriculum_adoption_v1(uuid, character, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_purge_curriculum_adoption_for_retention_v1(
    uuid, uuid, uuid, uuid, bigint
) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_curriculum_adoption_retention_assignment_delete_v1() OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_commit_delete_retention_work_before_curriculum_adoption(
    uuid, uuid, uuid, uuid, text, bigint
) OWNER TO ple_retention_broker;
ALTER FUNCTION public.ple_commit_delete_retention_work(
    uuid, uuid, uuid, uuid, text, bigint
) OWNER TO ple_retention_broker;

REVOKE ALL ON SCHEMA public FROM ple_curriculum_adoption_broker;
GRANT USAGE ON SCHEMA public TO ple_curriculum_adoption_broker;
REVOKE ALL ON SCHEMA public FROM ple_curriculum_schedule_revision_broker;
GRANT USAGE ON SCHEMA public TO ple_curriculum_schedule_revision_broker;
-- The final 1847 actor facade locks the presented session before Rust binds
-- its request digest.  RLS limits the broker to its current tenant; UPDATE on
-- the key column is PostgreSQL's narrow lock prerequisite, not mutation
-- authority (ASVS 2.3.1, 8.2.2).
CREATE POLICY curriculum_adoption_broker_session ON public.auth_session
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());
GRANT SELECT, UPDATE(session_hash) ON public.auth_session
    TO ple_curriculum_adoption_broker;
GRANT SELECT, INSERT ON public.curriculum_adoption_receipt_assignment,
    public.curriculum_assignment_adoption_evidence, public.curriculum_whole_course_adoption,
    public.curriculum_whole_course_module, public.curriculum_whole_course_assignment,
    public.curriculum_alpha_fork_lineage TO ple_curriculum_adoption_broker;
GRANT SELECT, INSERT, DELETE ON public.curriculum_adoption_receipt
    TO ple_curriculum_adoption_broker;
-- Receipt replay serializes on one immutable key with SELECT FOR UPDATE.
-- PostgreSQL requires column-level UPDATE authority to acquire that row lock;
-- the immutable trigger continues to reject every actual receipt update.
GRANT UPDATE(idempotency_key) ON public.curriculum_adoption_receipt
    TO ple_curriculum_adoption_broker;
-- Immutable evidence readers use row locks to serialize against retention.
-- Key-column UPDATE is PostgreSQL's lock prerequisite; the immutable triggers
-- continue to refuse every attempted row mutation.
GRANT UPDATE(receipt_key) ON public.curriculum_adoption_receipt_assignment,
    public.curriculum_assignment_adoption_evidence,
    public.curriculum_whole_course_adoption
    TO ple_curriculum_adoption_broker;
CREATE POLICY curriculum_adoption_receipt_lock ON public.curriculum_adoption_receipt
    FOR UPDATE TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant()) WITH CHECK (false);
CREATE POLICY curriculum_adoption_receipt_assignment_lock
    ON public.curriculum_adoption_receipt_assignment
    FOR UPDATE TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant()) WITH CHECK (false);
CREATE POLICY curriculum_adoption_evidence_lock
    ON public.curriculum_assignment_adoption_evidence
    FOR UPDATE TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant()) WITH CHECK (false);
CREATE POLICY curriculum_adoption_whole_course_lock
    ON public.curriculum_whole_course_adoption
    FOR UPDATE TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant()) WITH CHECK (false);
GRANT SELECT, INSERT, UPDATE, DELETE ON public.curriculum_assignment_import_current
    TO ple_curriculum_adoption_broker;
GRANT SELECT, INSERT, UPDATE ON public.course_schedule_revision
    TO ple_curriculum_adoption_broker;
GRANT SELECT, INSERT, UPDATE ON public.course_schedule_revision
    TO ple_curriculum_schedule_revision_broker;
REVOKE ALL ON public.curriculum_adoption_receipt,
    public.curriculum_adoption_receipt_assignment,
    public.curriculum_assignment_adoption_evidence,
    public.curriculum_assignment_import_current,
    public.curriculum_whole_course_adoption,
    public.curriculum_whole_course_module,
    public.curriculum_whole_course_assignment,
    public.curriculum_alpha_fork_lineage
    FROM ple_curriculum_schedule_revision_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(),
    public.ple_curriculum_adoption_actor_v1(uuid, character),
    public.ple_reusable_curriculum_instructor_actor(character, uuid),
    public.ple_get_curriculum_blueprint_v1(uuid, character, integer),
    public.ple_get_curriculum_alpha_v1(uuid, character, integer),
    public.ple_reusable_pinned_catalog_v1(uuid, uuid),
    public.ple_create_course_as_instructor_v1(uuid, uuid, text, date, date, text, uuid, character),
    public.ple_create_assignment_definition_v1(uuid, uuid, uuid, uuid, jsonb, uuid, integer),
    public.ple_replace_unissued_assignment_definition_v1(uuid, uuid, uuid, uuid, bigint, jsonb)
    TO ple_curriculum_adoption_broker;
GRANT EXECUTE ON FUNCTION public.ple_curriculum_adoption_retention_attested_v1(
    uuid, uuid, uuid, uuid, bigint
) TO ple_curriculum_adoption_broker;
GRANT EXECUTE ON FUNCTION public.ple_advance_course_schedule_revision_v1(
    uuid, uuid, boolean, name
) TO ple_curriculum_adoption_broker;

REVOKE ALL ON public.course_schedule_revision, public.curriculum_adoption_receipt,
    public.curriculum_adoption_receipt_assignment,
    public.curriculum_assignment_adoption_evidence, public.curriculum_assignment_import_current,
    public.curriculum_whole_course_adoption, public.curriculum_whole_course_module,
    public.curriculum_whole_course_assignment, public.curriculum_alpha_fork_lineage
    FROM PUBLIC, ple_app, ple_auth, ple_student, ple_grader, ple_grading_reader,
         ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_curriculum_adoption_immutable_refusal_v1(),
    public.ple_bump_course_term_schedule_revision_v1(),
    public.ple_bump_assignment_schedule_revision_v1(),
    public.ple_curriculum_adoption_actor_v1(uuid, character),
    public.ple_curriculum_adoption_require_closed_json_v1(jsonb),
    public.ple_curriculum_adoption_unavailable_v1(uuid, character, jsonb),
    public.ple_curriculum_adoption_retention_assignment_delete_v1()
    FROM PUBLIC, ple_app, ple_auth, ple_student, ple_grader, ple_grading_reader,
         ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_curriculum_adoption_immutable_refusal_v1(),
    public.ple_bump_course_term_schedule_revision_v1(),
    public.ple_bump_assignment_schedule_revision_v1(),
    public.ple_curriculum_adoption_retention_assignment_delete_v1()
    FROM ple_curriculum_adoption_broker;
REVOKE ALL ON FUNCTION public.ple_advance_course_schedule_revision_v1(
    uuid, uuid, boolean, name
) FROM PUBLIC, ple_app, ple_auth, ple_student, ple_grader,
       ple_grading_reader, ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_curriculum_adoption_retention_attested_v1(
        uuid, uuid, uuid, uuid, bigint
    ), public.ple_purge_curriculum_adoption_for_retention_v1(
        uuid, uuid, uuid, uuid, bigint
    ) FROM PUBLIC, ple_app, ple_auth, ple_student, ple_grader, ple_grading_reader;
GRANT EXECUTE ON FUNCTION public.ple_purge_curriculum_adoption_for_retention_v1(
    uuid, uuid, uuid, uuid, bigint
) TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_commit_delete_retention_work(
        uuid, uuid, uuid, uuid, text, bigint
    ), public.ple_commit_delete_retention_work_before_curriculum_adoption(
        uuid, uuid, uuid, uuid, text, bigint
    ) FROM PUBLIC, ple_app, ple_auth, ple_student, ple_grader, ple_grading_reader,
           ple_curriculum_adoption_broker;
REVOKE ALL ON FUNCTION public.ple_curriculum_adoption_preflight_v1(uuid, character),
    public.ple_preview_fork_alpha_v1(uuid, character, jsonb),
    public.ple_apply_fork_alpha_v1(uuid, character, jsonb),
    public.ple_preview_blueprint_instantiation_v1(uuid, character, jsonb),
    public.ple_apply_blueprint_instantiation_v1(uuid, character, jsonb),
    public.ple_preview_alpha_instantiation_v1(uuid, character, jsonb),
    public.ple_apply_alpha_instantiation_v1(uuid, character, jsonb),
    public.ple_preview_course_rollover_v1(uuid, character, jsonb),
    public.ple_apply_course_rollover_v1(uuid, character, jsonb),
    public.ple_preview_course_term_shift_v1(uuid, character, jsonb),
    public.ple_apply_course_term_shift_v1(uuid, character, jsonb),
    public.ple_preview_assignment_fast_forward_v1(uuid, character, jsonb),
    public.ple_apply_assignment_fast_forward_v1(uuid, character, jsonb),
    public.ple_preview_source_derived_assignment_v1(uuid, character, jsonb),
    public.ple_create_source_derived_assignment_v1(uuid, character, jsonb),
    public.ple_inspect_curriculum_imports_v1(uuid, character, jsonb),
    public.ple_reconcile_curriculum_adoption_v1(uuid, character, jsonb)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_curriculum_adoption_preflight_v1(uuid, character),
    public.ple_preview_fork_alpha_v1(uuid, character, jsonb),
    public.ple_apply_fork_alpha_v1(uuid, character, jsonb),
    public.ple_preview_blueprint_instantiation_v1(uuid, character, jsonb),
    public.ple_apply_blueprint_instantiation_v1(uuid, character, jsonb),
    public.ple_preview_alpha_instantiation_v1(uuid, character, jsonb),
    public.ple_apply_alpha_instantiation_v1(uuid, character, jsonb),
    public.ple_preview_course_rollover_v1(uuid, character, jsonb),
    public.ple_apply_course_rollover_v1(uuid, character, jsonb),
    public.ple_preview_course_term_shift_v1(uuid, character, jsonb),
    public.ple_apply_course_term_shift_v1(uuid, character, jsonb),
    public.ple_preview_assignment_fast_forward_v1(uuid, character, jsonb),
    public.ple_apply_assignment_fast_forward_v1(uuid, character, jsonb),
    public.ple_preview_source_derived_assignment_v1(uuid, character, jsonb),
    public.ple_create_source_derived_assignment_v1(uuid, character, jsonb),
    public.ple_inspect_curriculum_imports_v1(uuid, character, jsonb),
    public.ple_reconcile_curriculum_adoption_v1(uuid, character, jsonb)
    TO ple_app;

DO $$
DECLARE
    v_relation text;
    v_role text;
    v_function regprocedure;
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_roles
         WHERE rolname = ANY (ARRAY[
             'ple_curriculum_adoption_broker',
             'ple_curriculum_schedule_revision_broker'
         ])
          AND (rolcanlogin OR rolinherit OR rolbypassrls OR rolsuper
                OR rolcreatedb OR rolcreaterole OR rolreplication)
    ) THEN
        RAISE EXCEPTION 'curriculum adoption broker role is unsafe';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_class AS relation
          JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
         WHERE namespace.nspname = 'public'
           AND relation.relname = ANY (ARRAY[
               'course_schedule_revision', 'curriculum_adoption_receipt',
               'curriculum_adoption_receipt_assignment',
               'curriculum_assignment_adoption_evidence',
               'curriculum_assignment_import_current',
               'curriculum_whole_course_adoption', 'curriculum_whole_course_module',
               'curriculum_whole_course_assignment', 'curriculum_alpha_fork_lineage'
           ])
           AND (NOT relation.relrowsecurity OR NOT relation.relforcerowsecurity)
    ) THEN
        RAISE EXCEPTION 'curriculum adoption relation is not forced-RLS';
    END IF;

    IF NOT has_table_privilege(
           'ple_curriculum_adoption_broker', 'public.auth_session', 'SELECT'
       )
       OR NOT has_column_privilege(
           'ple_curriculum_adoption_broker', 'public.auth_session', 'session_hash', 'UPDATE'
       )
       OR has_column_privilege(
           'ple_curriculum_adoption_broker', 'public.auth_session', 'tenant_id', 'UPDATE'
       )
       OR has_column_privilege(
           'ple_curriculum_adoption_broker', 'public.auth_session', 'user_id', 'UPDATE'
       )
       OR has_column_privilege(
           'ple_curriculum_adoption_broker', 'public.auth_session', 'display_name', 'UPDATE'
       )
       OR has_column_privilege(
           'ple_curriculum_adoption_broker', 'public.auth_session', 'roles', 'UPDATE'
       )
       OR has_column_privilege(
           'ple_curriculum_adoption_broker', 'public.auth_session', 'created_at', 'UPDATE'
       )
       OR has_column_privilege(
           'ple_curriculum_adoption_broker', 'public.auth_session', 'expires_at', 'UPDATE'
       )
       OR has_column_privilege(
           'ple_curriculum_adoption_broker', 'public.auth_session', 'revoked_at', 'UPDATE'
       )
       OR NOT EXISTS (
            SELECT 1 FROM pg_policies
             WHERE schemaname = 'public' AND tablename = 'auth_session'
               AND policyname = 'curriculum_adoption_broker_session'
               AND cmd = 'SELECT'
               AND roles = ARRAY['ple_curriculum_adoption_broker']::name[]
       )
    THEN
        RAISE EXCEPTION 'curriculum adoption session lock authority is unsafe';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM (VALUES
              ('public.ple_curriculum_adoption_retention_attested_v1(uuid,uuid,uuid,uuid,bigint)'::regprocedure,
               'ple_retention_broker'),
              ('public.ple_purge_curriculum_adoption_for_retention_v1(uuid,uuid,uuid,uuid,bigint)'::regprocedure,
               'ple_curriculum_adoption_broker'),
              ('public.ple_curriculum_adoption_immutable_refusal_v1()'::regprocedure,
               'ple_curriculum_adoption_broker'),
              ('public.ple_advance_course_schedule_revision_v1(uuid,uuid,boolean,name)'::regprocedure,
               'ple_curriculum_schedule_revision_broker'),
              ('public.ple_bump_course_term_schedule_revision_v1()'::regprocedure,
               'ple_curriculum_adoption_broker'),
              ('public.ple_bump_assignment_schedule_revision_v1()'::regprocedure,
               'ple_curriculum_adoption_broker'),
              ('public.ple_curriculum_adoption_retention_assignment_delete_v1()'::regprocedure,
               'ple_curriculum_adoption_broker'),
              ('public.ple_commit_delete_retention_work(uuid,uuid,uuid,uuid,text,bigint)'::regprocedure,
               'ple_retention_broker'),
              ('public.ple_commit_delete_retention_work_before_curriculum_adoption(uuid,uuid,uuid,uuid,text,bigint)'::regprocedure,
               'ple_retention_broker')
          ) AS expected(function_oid, owner_name)
          JOIN pg_proc AS procedure_row ON procedure_row.oid = expected.function_oid
         WHERE pg_get_userbyid(procedure_row.proowner) <> expected.owner_name
    ) THEN
        RAISE EXCEPTION 'curriculum adoption function ownership is unsafe';
    END IF;

    IF NOT has_function_privilege(
           'ple_curriculum_adoption_broker',
           'public.ple_curriculum_adoption_actor_v1(uuid,character)'::regprocedure,
           'EXECUTE'
       ) THEN
        RAISE EXCEPTION 'curriculum adoption actor helper authority is unsafe';
    END IF;

    FOREACH v_role IN ARRAY ARRAY[
        'public', 'ple_app', 'ple_auth', 'ple_student', 'ple_grader',
        'ple_grading_reader', 'ple_retention_broker'
    ] LOOP
        FOREACH v_relation IN ARRAY ARRAY[
            'course_schedule_revision', 'curriculum_adoption_receipt',
            'curriculum_adoption_receipt_assignment',
            'curriculum_assignment_adoption_evidence',
            'curriculum_assignment_import_current',
            'curriculum_whole_course_adoption', 'curriculum_whole_course_module',
            'curriculum_whole_course_assignment', 'curriculum_alpha_fork_lineage'
        ] LOOP
            IF has_table_privilege(v_role, 'public.' || v_relation, 'SELECT')
               OR has_table_privilege(v_role, 'public.' || v_relation, 'INSERT')
               OR has_table_privilege(v_role, 'public.' || v_relation, 'UPDATE')
               OR has_table_privilege(v_role, 'public.' || v_relation, 'DELETE')
               OR has_table_privilege(v_role, 'public.' || v_relation, 'TRUNCATE')
               OR has_table_privilege(v_role, 'public.' || v_relation, 'REFERENCES')
               OR has_table_privilege(v_role, 'public.' || v_relation, 'TRIGGER')
            THEN
                RAISE EXCEPTION 'curriculum adoption table authority leaked to %', v_role;
            END IF;
        END LOOP;
    END LOOP;

    FOREACH v_relation IN ARRAY ARRAY[
        'course_schedule_revision', 'curriculum_adoption_receipt',
        'curriculum_adoption_receipt_assignment',
        'curriculum_assignment_adoption_evidence',
        'curriculum_assignment_import_current',
        'curriculum_whole_course_adoption', 'curriculum_whole_course_module',
        'curriculum_whole_course_assignment', 'curriculum_alpha_fork_lineage'
    ] LOOP
        IF has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'TRUNCATE'
           )
           OR has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'REFERENCES'
           )
           OR has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'TRIGGER'
           )
        THEN
            RAISE EXCEPTION 'curriculum adoption broker DDL authority is unsafe';
        END IF;
    END LOOP;

    FOREACH v_relation IN ARRAY ARRAY[
        'curriculum_adoption_receipt_assignment',
        'curriculum_assignment_adoption_evidence',
        'curriculum_whole_course_adoption', 'curriculum_whole_course_module',
        'curriculum_whole_course_assignment', 'curriculum_alpha_fork_lineage'
    ] LOOP
        IF NOT has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'SELECT'
           )
           OR NOT has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'INSERT'
           )
           OR has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'UPDATE'
           )
           OR has_table_privilege(
               'ple_curriculum_adoption_broker', 'public.' || v_relation, 'DELETE'
           )
        THEN
            RAISE EXCEPTION 'curriculum adoption immutable table authority is unsafe';
        END IF;
    END LOOP;

    IF NOT has_table_privilege(
           'ple_curriculum_adoption_broker',
           'public.curriculum_adoption_receipt', 'SELECT'
       )
       OR NOT has_table_privilege(
           'ple_curriculum_adoption_broker',
           'public.curriculum_adoption_receipt', 'INSERT'
       )
       OR NOT has_table_privilege(
           'ple_curriculum_adoption_broker',
           'public.curriculum_adoption_receipt', 'DELETE'
       )
       OR has_table_privilege(
           'ple_curriculum_adoption_broker',
           'public.curriculum_adoption_receipt', 'UPDATE'
       )
       OR NOT has_table_privilege(
           'ple_curriculum_adoption_broker',
           'public.curriculum_assignment_import_current', 'SELECT'
       )
       OR NOT has_table_privilege(
           'ple_curriculum_adoption_broker',
           'public.curriculum_assignment_import_current', 'INSERT'
       )
       OR NOT has_table_privilege(
           'ple_curriculum_adoption_broker',
           'public.curriculum_assignment_import_current', 'UPDATE'
       )
       OR NOT has_table_privilege(
           'ple_curriculum_adoption_broker',
           'public.curriculum_assignment_import_current', 'DELETE'
       )
       OR NOT has_table_privilege(
           'ple_curriculum_adoption_broker',
           'public.course_schedule_revision', 'SELECT'
       )
       OR NOT has_table_privilege(
           'ple_curriculum_adoption_broker',
           'public.course_schedule_revision', 'INSERT'
       )
       OR NOT has_table_privilege(
           'ple_curriculum_adoption_broker',
           'public.course_schedule_revision', 'UPDATE'
       )
       OR has_table_privilege(
           'ple_curriculum_adoption_broker',
           'public.course_schedule_revision', 'DELETE'
       )
    THEN
        RAISE EXCEPTION 'curriculum adoption mutable table authority is unsafe';
    END IF;

    IF NOT has_table_privilege(
           'ple_curriculum_schedule_revision_broker',
           'public.course_schedule_revision', 'SELECT'
       )
       OR NOT has_table_privilege(
           'ple_curriculum_schedule_revision_broker',
           'public.course_schedule_revision', 'INSERT'
       )
       OR NOT has_table_privilege(
           'ple_curriculum_schedule_revision_broker',
           'public.course_schedule_revision', 'UPDATE'
       )
       OR has_table_privilege(
           'ple_curriculum_schedule_revision_broker',
           'public.course_schedule_revision', 'DELETE'
       )
       OR has_table_privilege(
           'ple_curriculum_schedule_revision_broker',
           'public.course_schedule_revision', 'TRUNCATE'
       )
       OR has_table_privilege(
           'ple_curriculum_schedule_revision_broker',
           'public.course_schedule_revision', 'REFERENCES'
       )
       OR has_table_privilege(
           'ple_curriculum_schedule_revision_broker',
           'public.course_schedule_revision', 'TRIGGER'
       )
       OR has_schema_privilege(
           'ple_curriculum_schedule_revision_broker', 'public', 'CREATE'
       )
    THEN
        RAISE EXCEPTION 'schedule revision helper table authority is unsafe';
    END IF;

    FOREACH v_relation IN ARRAY ARRAY[
        'curriculum_adoption_receipt', 'curriculum_adoption_receipt_assignment',
        'curriculum_assignment_adoption_evidence',
        'curriculum_assignment_import_current',
        'curriculum_whole_course_adoption', 'curriculum_whole_course_module',
        'curriculum_whole_course_assignment', 'curriculum_alpha_fork_lineage'
    ] LOOP
        IF has_table_privilege(
               'ple_curriculum_schedule_revision_broker',
               'public.' || v_relation, 'SELECT'
           )
           OR has_table_privilege(
               'ple_curriculum_schedule_revision_broker',
               'public.' || v_relation, 'INSERT'
           )
           OR has_table_privilege(
               'ple_curriculum_schedule_revision_broker',
               'public.' || v_relation, 'UPDATE'
           )
           OR has_table_privilege(
               'ple_curriculum_schedule_revision_broker',
               'public.' || v_relation, 'DELETE'
           )
        THEN
            RAISE EXCEPTION 'schedule revision helper authority leaked to %', v_relation;
        END IF;
    END LOOP;

    FOREACH v_function IN ARRAY ARRAY[
        'public.ple_curriculum_adoption_retention_attested_v1(uuid,uuid,uuid,uuid,bigint)'::regprocedure,
        'public.ple_purge_curriculum_adoption_for_retention_v1(uuid,uuid,uuid,uuid,bigint)'::regprocedure,
        'public.ple_curriculum_adoption_immutable_refusal_v1()'::regprocedure,
        'public.ple_advance_course_schedule_revision_v1(uuid,uuid,boolean,name)'::regprocedure,
        'public.ple_bump_course_term_schedule_revision_v1()'::regprocedure,
        'public.ple_bump_assignment_schedule_revision_v1()'::regprocedure,
        'public.ple_curriculum_adoption_retention_assignment_delete_v1()'::regprocedure,
        'public.ple_commit_delete_retention_work_before_curriculum_adoption(uuid,uuid,uuid,uuid,text,bigint)'::regprocedure
    ] LOOP
        FOREACH v_role IN ARRAY ARRAY[
            'public', 'ple_app', 'ple_auth', 'ple_student', 'ple_grader',
            'ple_grading_reader'
        ] LOOP
            IF has_function_privilege(v_role, v_function, 'EXECUTE') THEN
                RAISE EXCEPTION 'curriculum adoption internal function leaked to %', v_role;
            END IF;
        END LOOP;
    END LOOP;

    IF NOT has_function_privilege(
           'ple_curriculum_adoption_broker',
           'public.ple_curriculum_adoption_retention_attested_v1(uuid,uuid,uuid,uuid,bigint)'::regprocedure,
           'EXECUTE'
       )
       OR NOT has_function_privilege(
           'ple_curriculum_adoption_broker',
           'public.ple_advance_course_schedule_revision_v1(uuid,uuid,boolean,name)'::regprocedure,
           'EXECUTE'
       )
       OR has_function_privilege(
           'ple_curriculum_adoption_broker',
           'public.ple_curriculum_adoption_immutable_refusal_v1()'::regprocedure,
           'EXECUTE'
       )
       OR has_function_privilege(
           'ple_curriculum_adoption_broker',
           'public.ple_bump_course_term_schedule_revision_v1()'::regprocedure,
           'EXECUTE'
       )
       OR has_function_privilege(
           'ple_curriculum_adoption_broker',
           'public.ple_bump_assignment_schedule_revision_v1()'::regprocedure,
           'EXECUTE'
       )
       OR NOT has_function_privilege(
           'ple_retention_broker',
           'public.ple_purge_curriculum_adoption_for_retention_v1(uuid,uuid,uuid,uuid,bigint)'::regprocedure,
           'EXECUTE'
       )
       OR has_function_privilege(
           'ple_retention_broker',
           'public.ple_curriculum_adoption_retention_assignment_delete_v1()'::regprocedure,
           'EXECUTE'
       )
       OR has_function_privilege(
           'public',
           'public.ple_curriculum_adoption_preflight_v1(uuid,character)'::regprocedure,
           'EXECUTE'
       )
       OR NOT has_function_privilege(
           'ple_app',
           'public.ple_reconcile_curriculum_adoption_v1(uuid,character,jsonb)'::regprocedure,
           'EXECUTE'
       )
    THEN
        RAISE EXCEPTION 'curriculum adoption execute authority is unsafe';
    END IF;
END $$;

COMMIT;
