-- WP-PROF-T4 source-context removal is an explicit, broker-owned transition.
BEGIN;

-- Retire the pre-T4 public source-fence surfaces.  The replacement
-- capabilities below are the only callable direct/retention mutation paths.
DROP FUNCTION IF EXISTS public.ple_rehearsal_fence_source_context(
    uuid, uuid, uuid, uuid
);
DROP FUNCTION IF EXISTS public.ple_fence_rehearsals_for_direct_instructor_removal(
    uuid, uuid, uuid, uuid
);
DROP FUNCTION IF EXISTS public.ple_prepare_direct_instructor_rehearsal_fence(
    uuid, uuid, uuid, uuid, bigint
);

-- The original T4 helper mixed source locking with aggregate mutation and was
-- callable by a public-looking wrapper.  Keep the mutation primitive private;
-- callers establish their source locks before entering it.
CREATE FUNCTION public.ple_rehearsal_fence_source_context_internal(
    p_tenant uuid, p_course uuid, p_assignment uuid, p_membership uuid
) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
    AS $$
DECLARE fenced bigint;
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR ((p_course IS NOT NULL)::integer + (p_assignment IS NOT NULL)::integer
           + (p_membership IS NOT NULL)::integer) <> 1 THEN
        RAISE EXCEPTION 'invalid rehearsal source-fence scope' USING ERRCODE = '42501';
    END IF;
    PERFORM 1 FROM public.rehearsal_run run
     WHERE run.tenant_id = p_tenant AND run.lifecycle = 'active'
       AND ((p_course IS NOT NULL AND run.course_id = p_course)
         OR (p_assignment IS NOT NULL AND run.assignment_id = p_assignment)
         OR (p_membership IS NOT NULL AND run.direct_instructor_membership_id = p_membership))
     ORDER BY run.rehearsal_run_id FOR UPDATE;
    INSERT INTO public.rehearsal_submission_claim_event (
        tenant_id, rehearsal_run_id, claim_id, sequence, operation_id, generation, phase
    )
    SELECT root.tenant_id, root.rehearsal_run_id, root.claim_id, latest.sequence + 1,
           latest.operation_id, latest.generation, 'revokedSourceContextRemoved'
      FROM public.rehearsal_submission_claim_root root
      JOIN public.rehearsal_run run ON run.tenant_id = root.tenant_id
       AND run.rehearsal_run_id = root.rehearsal_run_id
      CROSS JOIN LATERAL (
          SELECT event.sequence, event.operation_id, event.generation, event.phase
            FROM public.rehearsal_submission_claim_event event
           WHERE event.tenant_id = root.tenant_id
             AND event.rehearsal_run_id = root.rehearsal_run_id
             AND event.claim_id = root.claim_id
           ORDER BY event.sequence DESC LIMIT 1
      ) latest
     WHERE root.tenant_id = p_tenant AND run.lifecycle = 'active'
       AND ((p_course IS NOT NULL AND run.course_id = p_course)
         OR (p_assignment IS NOT NULL AND run.assignment_id = p_assignment)
         OR (p_membership IS NOT NULL AND run.direct_instructor_membership_id = p_membership))
       AND latest.phase IN ('prepared', 'gradingDispatched');
    UPDATE public.rehearsal_run SET lifecycle = 'discardedSourceContextRemoved',
        terminal_at = public.ple_rehearsal_now(), updated_at = public.ple_rehearsal_now()
     WHERE tenant_id = p_tenant AND lifecycle = 'active'
       AND ((p_course IS NOT NULL AND course_id = p_course)
         OR (p_assignment IS NOT NULL AND assignment_id = p_assignment)
         OR (p_membership IS NOT NULL AND direct_instructor_membership_id = p_membership));
    GET DIAGNOSTICS fenced = ROW_COUNT;
    RETURN fenced;
END
$$;

-- Prepare locks the complete direct-Instructor authority in stable order.  The
-- Store verifies every matching aggregate while these transaction-scoped locks
-- are held; the mutation below re-establishes this witness before committing.
CREATE FUNCTION public.ple_prepare_direct_instructor_rehearsal_fence(
    p_tenant uuid, p_actor uuid, p_course uuid, p_membership uuid,
    p_expected_roster_revision bigint
) RETURNS TABLE(
    roster_revision bigint,
    locked_rehearsal_count bigint,
    locked_rehearsal_run_ids uuid[]
)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
    AS $$
DECLARE actual_roster_revision bigint; target_count bigint; actor_count bigint;
DECLARE active_instructors bigint; locked_count bigint; locked_ids uuid[];
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_actor IS NULL OR p_course IS NULL OR p_membership IS NULL
       OR p_expected_roster_revision IS NULL OR p_expected_roster_revision <= 0 THEN
        RAISE EXCEPTION 'invalid direct Instructor removal capability' USING ERRCODE = '22023';
    END IF;
    PERFORM 1 FROM public.course course_record
     WHERE course_record.tenant_id = p_tenant AND course_record.course_id = p_course FOR KEY SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course is unavailable for Instructor removal' USING ERRCODE = '23503';
    END IF;
    SELECT roster.revision INTO actual_roster_revision FROM public.course_roster_state roster
     WHERE roster.tenant_id = p_tenant AND roster.course_id = p_course FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course roster state is unavailable' USING ERRCODE = '55000';
    END IF;
    IF actual_roster_revision <> p_expected_roster_revision THEN
        RAISE EXCEPTION 'stale course roster revision' USING ERRCODE = '55000';
    END IF;
    PERFORM 1 FROM public.course_member membership
     WHERE membership.tenant_id = p_tenant AND membership.course_id = p_course
       AND membership.role = 'instructor' AND membership.status = 'active'
     ORDER BY membership.course_membership_id FOR UPDATE;
    SELECT count(*) FILTER (WHERE membership.course_membership_id = p_membership),
           count(*) FILTER (WHERE membership.user_id = p_actor), count(*)
      INTO target_count, actor_count, active_instructors
      FROM public.course_member membership
     WHERE membership.tenant_id = p_tenant AND membership.course_id = p_course
       AND membership.role = 'instructor' AND membership.status = 'active';
    IF actor_count <> 1 THEN
        RAISE EXCEPTION 'actor lacks active direct Instructor authority' USING ERRCODE = '42501';
    END IF;
    IF target_count <> 1 THEN
        RAISE EXCEPTION 'target Instructor membership is unavailable' USING ERRCODE = '55000';
    END IF;
    IF active_instructors < 2 THEN
        RAISE EXCEPTION 'course must retain an active Instructor' USING ERRCODE = '55000';
    END IF;
    SELECT helper.locked_rehearsal_count, helper.locked_rehearsal_run_ids
      INTO locked_count, locked_ids
      FROM public.ple_lock_active_rehearsal_source_internal(
          p_tenant, NULL, NULL, p_membership
      ) helper;
    RETURN QUERY SELECT actual_roster_revision, locked_count, locked_ids;
END
$$;

-- Direct revocation rechecks the prepare witness, fences the exact target,
-- and commits only when Rust's locked aggregate count is unchanged.
CREATE FUNCTION public.ple_fence_rehearsals_for_direct_instructor_removal(
    p_tenant uuid, p_actor uuid, p_course uuid, p_membership uuid,
    p_expected_roster_revision bigint, p_locked_rehearsal_count bigint
) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
    AS $$
DECLARE fenced bigint; locked_roster_revision bigint; prepared_count bigint;
DECLARE prepared_ids uuid[]; changed bigint;
BEGIN
    IF p_locked_rehearsal_count IS NULL OR p_locked_rehearsal_count < 0 THEN
        RAISE EXCEPTION 'invalid locked rehearsal count' USING ERRCODE = '22023';
    END IF;
    SELECT witness.roster_revision, witness.locked_rehearsal_count,
           witness.locked_rehearsal_run_ids
      INTO locked_roster_revision, prepared_count, prepared_ids
      FROM public.ple_prepare_direct_instructor_rehearsal_fence(
          p_tenant, p_actor, p_course, p_membership, p_expected_roster_revision
      ) witness;
    IF prepared_count <> p_locked_rehearsal_count THEN
        RAISE EXCEPTION 'locked rehearsal count changed during Instructor removal'
            USING ERRCODE = '55000';
    END IF;
    SELECT public.ple_rehearsal_fence_source_context_internal(
        p_tenant, NULL, NULL, p_membership
    ) INTO fenced;
    IF fenced <> p_locked_rehearsal_count THEN
        RAISE EXCEPTION 'locked rehearsal count changed during Instructor removal'
            USING ERRCODE = '55000';
    END IF;
    UPDATE public.course_member SET status = 'revoked', revoked_at = transaction_timestamp()
     WHERE tenant_id = p_tenant AND course_id = p_course
       AND course_membership_id = p_membership AND role = 'instructor' AND status = 'active';
    GET DIAGNOSTICS changed = ROW_COUNT;
    IF changed <> 1 THEN
        RAISE EXCEPTION 'target Instructor membership changed during removal' USING ERRCODE = '55000';
    END IF;
    UPDATE public.course_roster_state SET revision = revision + 1, updated_at = transaction_timestamp()
     WHERE tenant_id = p_tenant AND course_id = p_course AND revision = p_expected_roster_revision;
    GET DIAGNOSTICS changed = ROW_COUNT;
    IF changed <> 1 THEN
        RAISE EXCEPTION 'course roster changed during Instructor removal' USING ERRCODE = '55000';
    END IF;
    IF locked_roster_revision <> p_expected_roster_revision THEN
        RAISE EXCEPTION 'course roster witness changed during Instructor removal' USING ERRCODE = '55000';
    END IF;
    RETURN fenced;
END
$$;

-- Prepare locks the leased delete work and every source row that Rust will
-- verify.  It intentionally does not lock or inspect rehearsal aggregates.
DROP FUNCTION IF EXISTS public.ple_prepare_retention_delete_rehearsal_verification(
    uuid, uuid, uuid, uuid, text, bigint
);
CREATE FUNCTION public.ple_prepare_retention_delete_rehearsal_verification(
    p_tenant uuid, p_job uuid, p_lease_token uuid, p_course uuid,
    p_stage text, p_generation bigint
) RETURNS TABLE(
    retention_generation bigint,
    locked_rehearsal_count bigint,
    locked_rehearsal_run_ids uuid[]
)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
    AS $$
DECLARE locked_generation bigint; dispatch_course uuid;
DECLARE locked_count bigint; locked_ids uuid[];
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_job IS NULL OR p_lease_token IS NULL OR p_course IS NULL
       OR p_stage <> 'deleteStudentRecords' OR p_generation IS NULL OR p_generation <= 0 THEN
        RAISE EXCEPTION 'invalid retention rehearsal verification capability'
            USING ERRCODE = '22023';
    END IF;
    SELECT dispatch.course_id, retention.generation INTO dispatch_course, locked_generation
      FROM public.worker_job job
      JOIN public.course_retention_dispatch dispatch ON dispatch.tenant_id = job.tenant_id
       AND dispatch.job_id = job.job_id
      JOIN public.course_retention retention ON retention.tenant_id = dispatch.tenant_id
       AND retention.course_id = dispatch.course_id AND retention.generation = dispatch.generation
      JOIN public.course_retention_stage stage ON stage.tenant_id = dispatch.tenant_id
       AND stage.course_id = dispatch.course_id AND stage.generation = dispatch.generation
       AND stage.stage = dispatch.stage
     WHERE job.job_id = p_job AND job.tenant_id = p_tenant AND job.state = 'leased'
       AND job.lease_token = p_lease_token AND job.lease_expires_at > transaction_timestamp()
       AND dispatch.course_id = p_course AND dispatch.stage = p_stage
       AND stage.state = 'started' AND stage.job_id = p_job
       AND stage.lease_token = p_lease_token AND retention.assignment_disposition = 'delete'
       AND dispatch.generation = p_generation
     FOR UPDATE OF job, dispatch, retention, stage;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'retention delete work is not authorized for rehearsal verification'
            USING ERRCODE = '42501';
    END IF;
    PERFORM 1 FROM public.course course_record
     WHERE course_record.tenant_id = p_tenant AND course_record.course_id = dispatch_course FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'retention source course is absent' USING ERRCODE = '55000';
    END IF;
    PERFORM 1 FROM public.assignment assignment_record
     WHERE assignment_record.tenant_id = p_tenant AND assignment_record.course_id = dispatch_course
     ORDER BY assignment_record.assignment_id FOR UPDATE;
    PERFORM 1 FROM public.course_member membership
     WHERE membership.tenant_id = p_tenant AND membership.course_id = dispatch_course
       AND membership.role = 'instructor' AND membership.status = 'active'
     ORDER BY membership.course_membership_id FOR UPDATE;
    SELECT helper.locked_rehearsal_count, helper.locked_rehearsal_run_ids
      INTO locked_count, locked_ids
      FROM public.ple_lock_active_rehearsal_source_internal(
          p_tenant, dispatch_course, NULL, NULL
      ) helper;
    RETURN QUERY SELECT locked_generation, locked_count, locked_ids;
END
$$;

-- Retention delete is the only source-fencing commit.  Rust supplies the
-- count for the locked aggregates; SQL only rechecks the lease and matches it.
DROP FUNCTION IF EXISTS public.ple_commit_delete_retention_work(
    uuid, uuid, uuid, uuid, text, bigint
);
CREATE FUNCTION public.ple_commit_delete_retention_work(
    p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text,
    p_generation bigint, p_locked_rehearsal_count bigint
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
    AS $$
DECLARE committed boolean; locked_generation bigint; fenced bigint;
DECLARE prepared_count bigint; prepared_ids uuid[];
BEGIN
    IF p_stage <> 'deleteStudentRecords' OR p_locked_rehearsal_count IS NULL
       OR p_locked_rehearsal_count < 0 THEN
        RAISE EXCEPTION 'invalid retention delete rehearsal commit capability'
            USING ERRCODE = '22023';
    END IF;
    SELECT witness.retention_generation, witness.locked_rehearsal_count,
           witness.locked_rehearsal_run_ids
      INTO locked_generation, prepared_count, prepared_ids
      FROM public.ple_prepare_retention_delete_rehearsal_verification(
          p_tenant, p_job, p_token, p_course, p_stage, p_generation
      ) witness;
    IF locked_generation <> p_generation THEN
        RAISE EXCEPTION 'retention generation changed during rehearsal verification'
            USING ERRCODE = '55000';
    END IF;
    IF prepared_count <> p_locked_rehearsal_count THEN
        RAISE EXCEPTION 'locked rehearsal count changed during retention delete'
            USING ERRCODE = '55000';
    END IF;
    SELECT public.ple_rehearsal_fence_source_context_internal(
        p_tenant, p_course, NULL, NULL
    ) INTO fenced;
    IF fenced <> p_locked_rehearsal_count THEN
        RAISE EXCEPTION 'locked rehearsal count changed during retention delete'
            USING ERRCODE = '55000';
    END IF;
    committed := public.ple_commit_delete_retention_work_before_passwordless_identity(
        p_tenant, p_job, p_token, p_course, p_stage, p_generation
    );
    IF NOT committed THEN RETURN false; END IF;
    DELETE FROM public.course_total_export_audit WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_grade_category_assignment WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_grade_letter_band WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_grade_category WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_grade_scheme WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_grade_export_audit WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_roster_import WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_roster_profile WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_invitation WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.tenant_learner_identity learner WHERE learner.tenant_id = p_tenant
      AND NOT EXISTS (SELECT 1 FROM public.course_member membership WHERE membership.tenant_id = learner.tenant_id AND membership.user_id = learner.user_id AND membership.role = 'student')
      AND NOT EXISTS (SELECT 1 FROM public.enrollment enrollment WHERE enrollment.tenant_id = learner.tenant_id AND enrollment.user_id = learner.user_id);
    IF EXISTS (
        SELECT 1 FROM public.course_invitation WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_roster_profile WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_grade_export_audit WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_total_export_audit WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_grade_category_assignment WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_grade_letter_band WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_grade_category WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_grade_scheme WHERE tenant_id = p_tenant AND course_id = p_course
    ) THEN
        RAISE EXCEPTION 'retention delete left cumulative cleanup residuals' USING ERRCODE = '55000';
    END IF;
    RETURN true;
END
$$;

DROP FUNCTION IF EXISTS public.ple_commit_retention_work(
    uuid, uuid, uuid, uuid, text, bigint
);
CREATE FUNCTION public.ple_commit_retention_work(
    p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text,
    p_generation bigint, p_locked_rehearsal_count bigint
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
    AS $$
BEGIN
    IF p_stage = 'deleteStudentRecords' THEN
        RETURN public.ple_commit_delete_retention_work(
            p_tenant, p_job, p_token, p_course, p_stage, p_generation,
            p_locked_rehearsal_count
        );
    END IF;
    IF p_locked_rehearsal_count IS NOT NULL THEN
        RAISE EXCEPTION 'non-delete retention work cannot carry a rehearsal count'
            USING ERRCODE = '22023';
    END IF;
    RETURN public.ple_commit_archive_retention_work(
        p_tenant, p_job, p_token, p_course, p_stage, p_generation
    );
END
$$;

REVOKE UPDATE (status, revoked_at) ON public.course_member FROM ple_app;
GRANT SELECT, UPDATE (course_id) ON public.course TO ple_rehearsal_broker;
GRANT SELECT, UPDATE (status, revoked_at) ON public.course_member TO ple_rehearsal_broker;
CREATE POLICY rehearsal_broker_roster_state_tenant ON public.course_roster_state
    TO ple_rehearsal_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
GRANT SELECT, UPDATE (revision, updated_at) ON public.course_roster_state TO ple_rehearsal_broker;
-- PostgreSQL requires UPDATE privilege for SELECT ... FOR UPDATE.  These
-- identity-column grants are lock-only capability support; no public role
-- receives them, and the retention broker still has no INSERT authority.
CREATE POLICY retention_broker_assignment_tenant_lock ON public.assignment
    FOR UPDATE TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
GRANT UPDATE (course_id) ON public.course TO ple_retention_broker;
GRANT UPDATE (assignment_id) ON public.assignment TO ple_retention_broker;
GRANT UPDATE (course_membership_id) ON public.course_member TO ple_retention_broker;
ALTER FUNCTION public.ple_rehearsal_fence_source_context_internal(uuid, uuid, uuid, uuid)
    OWNER TO ple_rehearsal_broker;
ALTER FUNCTION public.ple_prepare_direct_instructor_rehearsal_fence(uuid, uuid, uuid, uuid, bigint)
    OWNER TO ple_rehearsal_broker;
ALTER FUNCTION public.ple_fence_rehearsals_for_direct_instructor_removal(uuid, uuid, uuid, uuid, bigint, bigint)
    OWNER TO ple_rehearsal_broker;
ALTER FUNCTION public.ple_prepare_retention_delete_rehearsal_verification(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;
ALTER FUNCTION public.ple_commit_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint, bigint)
    OWNER TO ple_retention_broker;
ALTER FUNCTION public.ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint, bigint)
    OWNER TO ple_retention_broker;

REVOKE ALL ON FUNCTION public.ple_prepare_direct_instructor_rehearsal_fence(uuid, uuid, uuid, uuid, bigint) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_fence_rehearsals_for_direct_instructor_removal(uuid, uuid, uuid, uuid, bigint, bigint) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_prepare_retention_delete_rehearsal_verification(uuid, uuid, uuid, uuid, text, bigint) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_rehearsal_fence_source_context_internal(uuid, uuid, uuid, uuid) FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_commit_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint, bigint) FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_rehearsal_fence_source_context_internal(uuid, uuid, uuid, uuid) TO ple_retention_broker;
GRANT EXECUTE ON FUNCTION public.ple_prepare_direct_instructor_rehearsal_fence(uuid, uuid, uuid, uuid, bigint) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_fence_rehearsals_for_direct_instructor_removal(uuid, uuid, uuid, uuid, bigint, bigint) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_prepare_retention_delete_rehearsal_verification(uuid, uuid, uuid, uuid, text, bigint) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint, bigint) TO ple_app;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_assignment_mutator_broker') THEN
        EXECUTE 'REVOKE ALL ON FUNCTION public.ple_rehearsal_fence_source_context_internal(uuid, uuid, uuid, uuid) FROM ple_assignment_mutator_broker';
    END IF;
END
$$;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM pg_proc procedure
         WHERE procedure.oid IN (
             'public.ple_rehearsal_fence_source_context_internal(uuid,uuid,uuid,uuid)'::regprocedure,
             'public.ple_prepare_direct_instructor_rehearsal_fence(uuid,uuid,uuid,uuid,bigint)'::regprocedure,
             'public.ple_fence_rehearsals_for_direct_instructor_removal(uuid,uuid,uuid,uuid,bigint,bigint)'::regprocedure,
             'public.ple_prepare_retention_delete_rehearsal_verification(uuid,uuid,uuid,uuid,text,bigint)'::regprocedure,
             'public.ple_commit_delete_retention_work(uuid,uuid,uuid,uuid,text,bigint,bigint)'::regprocedure,
             'public.ple_commit_retention_work(uuid,uuid,uuid,uuid,text,bigint,bigint)'::regprocedure
         ) AND has_function_privilege('public', procedure.oid, 'EXECUTE')
    ) OR to_regprocedure('public.ple_rehearsal_fence_source_context(uuid,uuid,uuid,uuid)') IS NOT NULL
       OR to_regprocedure('public.ple_fence_rehearsals_for_direct_instructor_removal(uuid,uuid,uuid,uuid)') IS NOT NULL
       OR to_regprocedure('public.ple_fence_rehearsals_for_direct_instructor_removal(uuid,uuid,uuid,uuid,bigint)') IS NOT NULL
       OR to_regprocedure('public.ple_commit_retention_work(uuid,uuid,uuid,uuid,text,bigint)') IS NOT NULL
       OR has_function_privilege('ple_app', 'public.ple_rehearsal_fence_source_context_internal(uuid,uuid,uuid,uuid)', 'EXECUTE')
       OR has_function_privilege('ple_app', 'public.ple_commit_delete_retention_work(uuid,uuid,uuid,uuid,text,bigint,bigint)', 'EXECUTE')
       OR NOT has_function_privilege('ple_app', 'public.ple_prepare_direct_instructor_rehearsal_fence(uuid,uuid,uuid,uuid,bigint)', 'EXECUTE')
       OR NOT has_function_privilege('ple_app', 'public.ple_fence_rehearsals_for_direct_instructor_removal(uuid,uuid,uuid,uuid,bigint,bigint)', 'EXECUTE')
       OR NOT has_function_privilege('ple_app', 'public.ple_prepare_retention_delete_rehearsal_verification(uuid,uuid,uuid,uuid,text,bigint)', 'EXECUTE')
       OR NOT has_function_privilege('ple_app', 'public.ple_commit_retention_work(uuid,uuid,uuid,uuid,text,bigint,bigint)', 'EXECUTE')
       THEN
        RAISE EXCEPTION 'rehearsal source-fence privilege inventory failed' USING ERRCODE = '42501';
    END IF;
END
$$;
COMMIT;
