-- MOD-RETENTION R4.2: safe instructor projection and conditional lifecycle
-- requests. Requests only create the existing closed dispatch binding; they do
-- not expose a manifest or perform archive/delete effects in the API process.

CREATE TABLE course_retention_api_receipt (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    expected_generation bigint NOT NULL CHECK (expected_generation > 0),
    actor_id uuid NOT NULL,
    action text NOT NULL CHECK (action IN ('archive', 'delete')),
    assignment_disposition text CHECK (assignment_disposition IN ('retain', 'delete')),
    resulting_generation bigint NOT NULL CHECK (resulting_generation > 0),
    stage text NOT NULL CHECK (stage IN ('archiveStudentRecords', 'deleteStudentRecords')),
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, course_id, expected_generation),
    FOREIGN KEY (tenant_id, course_id) REFERENCES course_retention(tenant_id, course_id) ON DELETE CASCADE
);
ALTER TABLE course_retention_api_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE course_retention_api_receipt FORCE ROW LEVEL SECURITY;
CREATE POLICY retention_api_receipt_tenant ON course_retention_api_receipt
    USING (tenant_id=ple_current_tenant()) WITH CHECK (tenant_id=ple_current_tenant());
CREATE POLICY retention_broker_api_receipt ON course_retention_api_receipt
    FOR ALL TO ple_retention_broker
    USING (tenant_id=ple_current_tenant()) WITH CHECK (tenant_id=ple_current_tenant());
GRANT SELECT, INSERT ON course_retention_api_receipt TO ple_retention_broker;
REVOKE ALL ON course_retention_api_receipt FROM PUBLIC, ple_app, ple_student, ple_grader, ple_queue_broker;

CREATE FUNCTION ple_read_retention_notification(p_session char(64), p_course uuid)
RETURNS TABLE (intent text, created_at_millis bigint)
LANGUAGE sql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public AS $$
    SELECT n.intent, floor(extract(epoch FROM n.created_at) * 1000)::bigint
      FROM public.course_retention r
      JOIN public.course_retention_notification n
        ON n.tenant_id=r.tenant_id AND n.course_id=r.course_id
     WHERE public.ple_retention_authorize(p_session, p_course, false)
       AND r.tenant_id=public.ple_current_tenant() AND r.course_id=p_course
     ORDER BY n.generation DESC, n.created_at DESC
     LIMIT 1
$$;
ALTER FUNCTION ple_read_retention_notification(char, uuid) OWNER TO ple_retention_broker;

CREATE FUNCTION ple_apply_retention_api_action(
    p_session char(64), p_course uuid, p_expected_generation bigint,
    p_action text, p_days integer DEFAULT NULL, p_disposition text DEFAULT NULL
) RETURNS text LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE current record; replay record; next_generation bigint; immediate_stage text; next_disposition text; target_state text; actor uuid;
BEGIN
    IF p_expected_generation <= 0 OR p_action NOT IN ('extend','archive','delete') THEN RETURN NULL; END IF;
    IF p_action='extend' THEN
        IF NOT public.ple_retention_authorize(p_session, p_course, true) THEN
            RAISE EXCEPTION 'retention API operation forbidden' USING ERRCODE = '42501';
        END IF;
        IF p_days NOT BETWEEN 1 AND 36500 OR p_disposition IS NOT NULL THEN RETURN NULL; END IF;
    ELSIF NOT public.ple_retention_authorize(p_session, p_course, false) THEN
        RAISE EXCEPTION 'retention API operation forbidden' USING ERRCODE = '42501';
    ELSIF p_action='archive' AND (p_days IS NOT NULL OR p_disposition NOT IN ('retain','delete')) THEN
        RETURN NULL;
    ELSIF p_action='delete' AND (p_days IS NOT NULL OR p_disposition IS NOT NULL) THEN
        RETURN NULL;
    END IF;
    SELECT user_id INTO actor FROM public.auth_session
     WHERE session_hash=p_session AND tenant_id=public.ple_current_tenant()
       AND revoked_at IS NULL AND expires_at > transaction_timestamp();
    IF actor IS NULL THEN RAISE EXCEPTION 'retention API operation forbidden' USING ERRCODE = '42501'; END IF;
    -- Serialize every conditional action on the course row before consulting
    -- its replay receipt. A concurrent original-ETag retry therefore sees the
    -- first transaction's committed receipt instead of a stale generation.
    SELECT * INTO current FROM public.course_retention
     WHERE tenant_id=public.ple_current_tenant() AND course_id=p_course
       AND lifecycle='active' FOR UPDATE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    IF p_action IN ('archive','delete') THEN
        SELECT * INTO replay FROM public.course_retention_api_receipt
         WHERE tenant_id=public.ple_current_tenant() AND course_id=p_course
           AND expected_generation=p_expected_generation;
        IF FOUND THEN
            IF replay.actor_id<>actor OR replay.action<>p_action
               OR replay.assignment_disposition IS DISTINCT FROM p_disposition THEN RETURN NULL; END IF;
            SELECT state INTO target_state FROM public.course_retention_stage
             WHERE tenant_id=replay.tenant_id AND course_id=replay.course_id
               AND generation=replay.resulting_generation AND stage=replay.stage;
            IF target_state='scheduled' THEN RETURN 'scheduled'; END IF;
            IF target_state='started' THEN RETURN 'inProgress'; END IF;
            IF target_state='completed' THEN RETURN 'completed'; END IF;
            RETURN NULL;
        END IF;
    END IF;
    IF current.generation<>p_expected_generation THEN RETURN NULL; END IF;
    immediate_stage := CASE p_action WHEN 'archive' THEN 'archiveStudentRecords'
                                      WHEN 'delete' THEN 'deleteStudentRecords' END;
    IF immediate_stage IS NOT NULL THEN
        SELECT state INTO target_state FROM public.course_retention_stage s
         WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
           AND s.generation=current.generation AND s.stage=immediate_stage;
        IF target_state='started' THEN RETURN 'inProgress'; END IF;
        IF target_state='completed' THEN RETURN 'completed'; END IF;
        IF target_state <> 'scheduled' THEN RETURN NULL; END IF;
        IF EXISTS (
            SELECT 1 FROM public.course_retention_dispatch d
             WHERE d.tenant_id=current.tenant_id AND d.course_id=current.course_id
               AND d.generation=current.generation AND d.stage=immediate_stage
        ) THEN
            IF p_action='archive' AND current.assignment_disposition IS DISTINCT FROM p_disposition THEN
                RETURN NULL;
            END IF;
            RETURN 'scheduled';
        END IF;
    END IF;
    IF EXISTS (
        SELECT 1 FROM public.course_retention_stage s
         WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
           AND s.generation=current.generation AND s.state NOT IN ('scheduled','completed')
    ) THEN RETURN NULL; END IF;
    next_generation := current.generation + 1;
    next_disposition := CASE WHEN p_action='archive' THEN p_disposition ELSE current.assignment_disposition END;
    -- Copy before superseding: completed notification history must not be
    -- converted into a new scheduled stage by the generation transition.
    INSERT INTO public.course_retention_stage (tenant_id, course_id, stage, generation, due_at, state)
    SELECT s.tenant_id, s.course_id, s.stage, next_generation,
           CASE WHEN s.state='completed' THEN s.due_at
                WHEN s.stage=immediate_stage THEN transaction_timestamp()
                WHEN p_action='extend' THEN s.due_at + p_days * interval '1 day'
                ELSE s.due_at END,
           CASE WHEN s.state='completed' THEN 'completed' ELSE 'scheduled' END
     FROM public.course_retention_stage s
     WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
       AND s.generation=current.generation;
    UPDATE public.course_retention_stage SET state='superseded'
     WHERE tenant_id=current.tenant_id AND course_id=current.course_id
       AND generation=current.generation AND state='scheduled';
    UPDATE public.worker_job w SET state='dead', lease_token=NULL, lease_expires_at=NULL,
        completed_at=transaction_timestamp(), last_error='permanent'
      FROM public.course_retention_dispatch d
     WHERE d.tenant_id=current.tenant_id AND d.course_id=current.course_id
       AND d.generation=current.generation AND d.job_id=w.job_id AND w.state IN ('ready','leased');
    UPDATE public.course_retention SET generation=next_generation, assignment_disposition=next_disposition
     WHERE tenant_id=current.tenant_id AND course_id=current.course_id AND generation=current.generation;
    IF immediate_stage IS NOT NULL THEN
        WITH dispatch AS (
            INSERT INTO public.course_retention_dispatch
                (tenant_id, course_id, stage, generation, job_id, dispatched_at)
            VALUES (current.tenant_id, current.course_id, immediate_stage, next_generation,
                    gen_random_uuid(), transaction_timestamp())
            RETURNING job_id
        )
        INSERT INTO public.worker_job (job_id, tenant_id, payload, state, max_attempts)
        SELECT job_id, current.tenant_id,
               jsonb_build_object('kind','retention','course',current.course_id::text,
                                  'stage',immediate_stage,'generation',next_generation),
               'ready', 3 FROM dispatch;
        INSERT INTO public.course_retention_api_receipt
            (tenant_id, course_id, expected_generation, actor_id, action, assignment_disposition,
             resulting_generation, stage)
        VALUES (current.tenant_id, current.course_id, p_expected_generation, actor, p_action,
                p_disposition, next_generation, immediate_stage);
    END IF;
    RETURN CASE WHEN immediate_stage IS NULL THEN 'changed' ELSE 'scheduled' END;
END $$;
ALTER FUNCTION ple_apply_retention_api_action(char, uuid, bigint, text, integer, text)
    OWNER TO ple_retention_broker;

REVOKE ALL ON FUNCTION ple_read_retention_notification(char, uuid),
    ple_apply_retention_api_action(char, uuid, bigint, text, integer, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_read_retention_notification(char, uuid),
    ple_apply_retention_api_action(char, uuid, bigint, text, integer, text) TO ple_app;
