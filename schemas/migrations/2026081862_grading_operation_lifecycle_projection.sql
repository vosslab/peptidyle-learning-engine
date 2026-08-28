-- WP-PROF-G1 W5: additive worker-to-operation lifecycle projection.
--
-- W4 and the scoring worker remain the only execution and score authorities.
-- These triggers project their already-fenced terminal state into the mutable
-- Instructor operation thread without creating an Instructor action receipt.

BEGIN;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_roles
         WHERE rolname = 'ple_grading_operation_lifecycle_broker'
    ) THEN
        CREATE ROLE ple_grading_operation_lifecycle_broker
            NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOREPLICATION NOBYPASSRLS;
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_auth_members AS membership
         WHERE membership.roleid = 'ple_grading_operation_lifecycle_broker'::regrole
            OR membership.member = 'ple_grading_operation_lifecycle_broker'::regrole
    ) THEN
        RAISE EXCEPTION 'grading-operation lifecycle broker must have no memberships';
    END IF;
END;
$$;

ALTER ROLE ple_grading_operation_lifecycle_broker
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOREPLICATION NOBYPASSRLS;
REVOKE ALL ON SCHEMA public FROM ple_grading_operation_lifecycle_broker;
GRANT USAGE ON SCHEMA public TO ple_grading_operation_lifecycle_broker;
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM ple_grading_operation_lifecycle_broker;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM ple_grading_operation_lifecycle_broker;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA public FROM ple_grading_operation_lifecycle_broker;

CREATE POLICY grading_operation_lifecycle_broker_select
    ON public.grading_operation FOR SELECT TO ple_grading_operation_lifecycle_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY grading_operation_lifecycle_broker_update
    ON public.grading_operation FOR UPDATE TO ple_grading_operation_lifecycle_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());

GRANT SELECT ON public.grading_operation TO ple_grading_operation_lifecycle_broker;
GRANT UPDATE (reason, state, revision, next_action, updated_at)
    ON public.grading_operation TO ple_grading_operation_lifecycle_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant()
    TO ple_grading_operation_lifecycle_broker;

-- AFTER is intentional: 1859 has established the exact W4 completion tuple
-- earlier in the same transaction.  A later failure still rolls back both
-- effects.  The trigger neither reads nor changes private response,
-- evaluation, or receipt data, so W4's execution/lease fences remain the only
-- authority for completion.
CREATE FUNCTION public.ple_project_completed_grading_operation_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF NEW.state = 'completed' AND OLD.state IS DISTINCT FROM 'completed' THEN
        UPDATE public.grading_operation AS operation_row
           SET state = 'completed', next_action = NULL,
               revision = operation_row.revision + 1,
               updated_at = transaction_timestamp()
         WHERE operation_row.tenant_id = NEW.tenant_id
           AND operation_row.target_kind = 'submission'
           AND operation_row.attempt_id = NEW.attempt_id
           AND operation_row.submission_id = NEW.submission_id
           AND operation_row.state = 'action_in_progress';
    END IF;
    RETURN NEW;
END;
$$;

-- PostgreSQL fires same-event triggers in lexical name order.  The `zz_`
-- trigger must follow 1849's `grading_operation_retention_fence`; the catalog
-- assertion below makes that predecessor part of this migration's contract.
-- W4's `INSERT ... ON CONFLICT DO NOTHING` therefore becomes an additive
-- reopen: an existing thread records the latest safe reason and returns NULL,
-- while a first deterministic failure retains W4's original insert unchanged.
CREATE FUNCTION public.ple_reopen_submission_grading_operation_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_existing_id bigint;
BEGIN
    IF NEW.target_kind <> 'submission' OR NEW.state <> 'actionable'
       OR NEW.next_action <> 'retry' THEN
        RETURN NEW;
    END IF;
    SELECT operation_row.grading_operation_id INTO v_existing_id
      FROM public.grading_operation AS operation_row
     WHERE operation_row.tenant_id = NEW.tenant_id
       AND operation_row.assignment_id = NEW.assignment_id
       AND operation_row.attempt_id = NEW.attempt_id
       AND operation_row.submission_id = NEW.submission_id
       AND operation_row.target_kind = 'submission'
     FOR UPDATE;
    IF NOT FOUND THEN RETURN NEW; END IF;
    UPDATE public.grading_operation AS operation_row
       SET reason = NEW.reason, state = 'actionable', next_action = 'retry',
           revision = operation_row.revision + 1,
           updated_at = transaction_timestamp()
     WHERE operation_row.tenant_id = NEW.tenant_id
       AND operation_row.grading_operation_id = v_existing_id;
    RETURN NULL;
END;
$$;

-- 1831 changes an assignment from recalculating to current only after its
-- exact leased generation publishes.  The existing queue-dead trigger changes
-- it to failed for that same generation first.  This AFTER trigger inherits
-- those fences and projects only an already-existing W5 operation thread.
CREATE FUNCTION public.ple_project_assignment_scoring_operation_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF OLD.scoring_status = 'recalculating' AND NEW.scoring_status = 'current' THEN
        UPDATE public.grading_operation AS operation_row
           SET state = 'completed', next_action = NULL,
               revision = operation_row.revision + 1,
               updated_at = transaction_timestamp()
         WHERE operation_row.tenant_id = NEW.tenant_id
           AND operation_row.course_id = NEW.course_id
           AND operation_row.assignment_id = NEW.assignment_id
           AND operation_row.target_kind = 'assignment_scoring_generation'
           AND operation_row.requested_scoring_generation = NEW.scoring_generation
           AND operation_row.state = 'action_in_progress';
    ELSIF OLD.scoring_status = 'recalculating' AND NEW.scoring_status = 'failed' THEN
        UPDATE public.grading_operation AS operation_row
           SET reason = 'scoring_recalculation_failed',
               state = 'actionable', next_action = 'recalculate',
               revision = operation_row.revision + 1,
               updated_at = transaction_timestamp()
         WHERE operation_row.tenant_id = NEW.tenant_id
           AND operation_row.course_id = NEW.course_id
           AND operation_row.assignment_id = NEW.assignment_id
           AND operation_row.target_kind = 'assignment_scoring_generation'
           AND operation_row.requested_scoring_generation = NEW.scoring_generation
           AND operation_row.state = 'action_in_progress';
    END IF;
    RETURN NEW;
END;
$$;

ALTER FUNCTION public.ple_project_completed_grading_operation_v1()
    OWNER TO ple_grading_operation_lifecycle_broker;
ALTER FUNCTION public.ple_reopen_submission_grading_operation_v1()
    OWNER TO ple_grading_operation_lifecycle_broker;
ALTER FUNCTION public.ple_project_assignment_scoring_operation_v1()
    OWNER TO ple_grading_operation_lifecycle_broker;
REVOKE ALL ON FUNCTION public.ple_project_completed_grading_operation_v1(),
    public.ple_reopen_submission_grading_operation_v1(),
    public.ple_project_assignment_scoring_operation_v1()
    FROM PUBLIC, ple_app;

CREATE TRIGGER zz_grading_operation_close_completed_execution
    AFTER UPDATE OF state ON public.grading_execution
    FOR EACH ROW EXECUTE FUNCTION public.ple_project_completed_grading_operation_v1();
CREATE TRIGGER zz_grading_operation_reopen_submission
    BEFORE INSERT ON public.grading_operation
    FOR EACH ROW EXECUTE FUNCTION public.ple_reopen_submission_grading_operation_v1();
CREATE TRIGGER zz_grading_operation_project_assignment_scoring
    AFTER UPDATE OF scoring_status ON public.assignment
    FOR EACH ROW EXECUTE FUNCTION public.ple_project_assignment_scoring_operation_v1();

DO $$
DECLARE
    v_functions regprocedure[] := ARRAY[
        'public.ple_project_completed_grading_operation_v1()'::regprocedure,
        'public.ple_reopen_submission_grading_operation_v1()'::regprocedure,
        'public.ple_project_assignment_scoring_operation_v1()'::regprocedure
    ];
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_proc AS procedure_row
        CROSS JOIN LATERAL pg_catalog.aclexplode(COALESCE(
            procedure_row.proacl, pg_catalog.acldefault('f', procedure_row.proowner)
        )) AS privilege_row
        WHERE procedure_row.oid = ANY(v_functions)
          AND privilege_row.grantee = 0 AND privilege_row.privilege_type = 'EXECUTE'
    ) OR EXISTS (
        SELECT 1 FROM pg_catalog.pg_trigger AS trigger_row
         WHERE trigger_row.tgname IN (
             'zz_grading_operation_close_completed_execution',
             'zz_grading_operation_reopen_submission',
             'zz_grading_operation_project_assignment_scoring'
         ) AND (trigger_row.tgisinternal OR NOT trigger_row.tgenabled = 'O')
    ) OR (SELECT count(*) FROM pg_catalog.pg_trigger AS trigger_row
          WHERE trigger_row.tgname IN (
              'zz_grading_operation_close_completed_execution',
              'zz_grading_operation_reopen_submission',
              'zz_grading_operation_project_assignment_scoring'
          )) <> 3
       OR NOT EXISTS (
           SELECT 1
             FROM pg_catalog.pg_trigger AS trigger_row
            WHERE trigger_row.tgrelid = 'public.grading_operation'::regclass
              AND trigger_row.tgname = 'grading_operation_retention_fence'
              AND NOT trigger_row.tgisinternal
              AND trigger_row.tgenabled = 'O'
       )
       OR NOT pg_catalog.has_column_privilege(
           'ple_grading_operation_lifecycle_broker', 'public.grading_operation',
           'reason', 'UPDATE'
       ) OR pg_catalog.has_table_privilege(
           'ple_grading_operation_lifecycle_broker', 'public.grading_execution', 'SELECT'
       ) OR pg_catalog.has_table_privilege(
           'ple_grading_operation_lifecycle_broker', 'public.assignment', 'UPDATE'
       ) OR EXISTS (
           SELECT 1 FROM pg_catalog.pg_roles AS role_row
            WHERE role_row.rolname = 'ple_grading_operation_lifecycle_broker'
              AND (role_row.rolcanlogin OR role_row.rolinherit OR role_row.rolsuper
                   OR role_row.rolcreatedb OR role_row.rolcreaterole
                   OR role_row.rolreplication OR role_row.rolbypassrls)
       ) THEN
        RAISE EXCEPTION 'grading-operation lifecycle projection privilege matrix is unsafe';
    END IF;
END;
$$;

COMMIT;
