-- MOD-RETENTION R4.1: broker-owned dispatch and generation-safe schedule controls.
-- A retention payload is executable only when this table binds it to one due
-- current-generation stage. The application cannot insert this binding.

CREATE TABLE course_retention_dispatch (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    stage text NOT NULL CHECK (stage IN ('notify', 'archiveStudentRecords', 'deleteStudentRecords')),
    generation bigint NOT NULL CHECK (generation > 0),
    job_id uuid NOT NULL UNIQUE,
    dispatched_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, course_id, stage, generation),
    FOREIGN KEY (tenant_id, course_id, stage, generation)
        REFERENCES course_retention_stage(tenant_id, course_id, stage, generation),
    FOREIGN KEY (job_id) REFERENCES worker_job(job_id) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX course_retention_dispatch_job_idx ON course_retention_dispatch (job_id);

ALTER TABLE course_retention_dispatch ENABLE ROW LEVEL SECURITY;
ALTER TABLE course_retention_dispatch FORCE ROW LEVEL SECURITY;
CREATE POLICY retention_dispatch_tenant ON course_retention_dispatch
    USING (tenant_id = ple_current_tenant()) WITH CHECK (tenant_id = ple_current_tenant());

-- This NOLOGIN/NOBYPASSRLS owner has no direct connection. The deliberately
-- fixed scheduler SDF below needs a global due scan, while all authenticated
-- course mutations remain tenant-scoped through ple_retention_authorize.
CREATE POLICY retention_broker_dispatch ON course_retention_dispatch
    FOR ALL TO ple_retention_broker USING (true) WITH CHECK (true);
CREATE POLICY retention_broker_course_schedule ON course_retention
    FOR SELECT TO ple_retention_broker USING (true);
CREATE POLICY retention_broker_stage_schedule ON course_retention_stage
    FOR ALL TO ple_retention_broker USING (true) WITH CHECK (true);
CREATE POLICY retention_broker_worker_job_insert ON worker_job
    FOR INSERT TO ple_retention_broker WITH CHECK (true);

GRANT SELECT, INSERT, UPDATE ON course_retention_dispatch, course_retention_stage TO ple_retention_broker;
GRANT INSERT ON worker_job TO ple_retention_broker;
REVOKE ALL ON course_retention_dispatch FROM PUBLIC, ple_app, ple_student, ple_grader, ple_queue_broker;

-- No tenant/course/stage/time is an application scheduler input. Database time
-- and SKIP LOCKED choose at most the trusted bounded number of current stages.
CREATE FUNCTION ple_dispatch_due_retention_stages(p_batch integer)
RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE dispatched bigint;
BEGIN
    IF p_batch NOT BETWEEN 1 AND 100 THEN
        RAISE EXCEPTION 'invalid retention dispatch batch' USING ERRCODE = '22023';
    END IF;
    WITH candidate AS (
        SELECT s.tenant_id, s.course_id, s.stage, s.generation
          FROM public.course_retention_stage s
          JOIN public.course_retention r
            ON r.tenant_id=s.tenant_id AND r.course_id=s.course_id AND r.generation=s.generation
          LEFT JOIN public.course_retention_dispatch d
            ON d.tenant_id=s.tenant_id AND d.course_id=s.course_id
           AND d.stage=s.stage AND d.generation=s.generation
         WHERE r.lifecycle='active' AND s.state='scheduled'
           AND s.due_at <= transaction_timestamp() AND d.job_id IS NULL
         ORDER BY s.due_at, s.tenant_id, s.course_id, s.stage
         FOR UPDATE OF s, r SKIP LOCKED
         LIMIT p_batch
    ), dispatch AS (
        INSERT INTO public.course_retention_dispatch
            (tenant_id, course_id, stage, generation, job_id, dispatched_at)
        SELECT tenant_id, course_id, stage, generation, gen_random_uuid(), transaction_timestamp()
          FROM candidate
        RETURNING tenant_id, course_id, stage, generation, job_id
    ), jobs AS (
        INSERT INTO public.worker_job (job_id, tenant_id, payload, state, max_attempts)
        SELECT job_id, tenant_id,
               jsonb_build_object('kind','retention','course',course_id::text,
                                  'stage',stage,'generation',generation),
               'ready', 3
          FROM dispatch
        RETURNING job_id
    )
    SELECT count(*) INTO dispatched FROM jobs;
    RETURN dispatched;
END $$;
ALTER FUNCTION ple_dispatch_due_retention_stages(integer) OWNER TO ple_retention_broker;

-- Administrators extend only future, unstarted stages. Completed stage history
-- remains visible as completed in the new generation and notification is not duplicated.
CREATE FUNCTION ple_extend_course_retention(p_session char(64), p_course uuid, p_days integer)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE current record; next_generation bigint;
BEGIN
    IF NOT public.ple_retention_authorize(p_session, p_course, true) THEN
        RAISE EXCEPTION 'retention schedule operation forbidden' USING ERRCODE = '42501';
    END IF;
    IF p_days NOT BETWEEN 1 AND 36500 THEN RETURN false; END IF;
    SELECT * INTO current FROM public.course_retention
      WHERE tenant_id=public.ple_current_tenant() AND course_id=p_course AND lifecycle='active' FOR UPDATE;
    IF NOT FOUND OR EXISTS (
        SELECT 1 FROM public.course_retention_stage s
         WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
           AND s.generation=current.generation AND s.state='started'
    ) THEN RETURN false; END IF;
    IF EXISTS (
        SELECT 1 FROM public.course_retention_stage s
         WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
           AND s.generation=current.generation AND s.state NOT IN ('scheduled','completed')
    ) THEN RETURN false; END IF;
    next_generation := current.generation + 1;
    UPDATE public.course_retention_stage SET state='superseded'
      WHERE tenant_id=current.tenant_id AND course_id=current.course_id
        AND generation=current.generation AND state='scheduled';
    UPDATE public.worker_job w SET state='dead', lease_token=NULL, lease_expires_at=NULL,
        completed_at=transaction_timestamp(), last_error='permanent'
      FROM public.course_retention_dispatch d
      WHERE d.tenant_id=current.tenant_id AND d.course_id=current.course_id
        AND d.generation=current.generation AND w.job_id=d.job_id AND w.state IN ('ready','leased');
    INSERT INTO public.course_retention_stage (tenant_id, course_id, stage, generation, due_at, state)
      SELECT s.tenant_id, s.course_id, s.stage, next_generation,
             CASE WHEN s.state='completed' THEN s.due_at ELSE s.due_at + p_days * interval '1 day' END,
             CASE WHEN s.state='completed' THEN 'completed' ELSE 'scheduled' END
        FROM public.course_retention_stage s
       WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
         AND s.generation=current.generation
       ORDER BY s.stage;
    UPDATE public.course_retention SET generation=next_generation
      WHERE tenant_id=current.tenant_id AND course_id=current.course_id AND generation=current.generation;
    RETURN FOUND;
END $$;
ALTER FUNCTION ple_extend_course_retention(char, uuid, integer) OWNER TO ple_retention_broker;

CREATE FUNCTION ple_set_archive_disposition(p_session char(64), p_course uuid, p_disposition text)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE current record;
BEGIN
    IF NOT public.ple_retention_authorize(p_session, p_course, false) THEN
        RAISE EXCEPTION 'retention schedule operation forbidden' USING ERRCODE = '42501';
    END IF;
    IF p_disposition NOT IN ('retain','delete') THEN RETURN false; END IF;
    SELECT * INTO current FROM public.course_retention
      WHERE tenant_id=public.ple_current_tenant() AND course_id=p_course AND lifecycle='active' FOR UPDATE;
    IF NOT FOUND OR NOT EXISTS (
        SELECT 1 FROM public.course_retention_stage s
         WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
           AND s.generation=current.generation AND s.stage='archiveStudentRecords'
           AND s.state='scheduled'
    ) THEN RETURN false; END IF;
    UPDATE public.course_retention SET assignment_disposition=p_disposition
      WHERE tenant_id=current.tenant_id AND course_id=current.course_id AND generation=current.generation;
    RETURN FOUND;
END $$;
ALTER FUNCTION ple_set_archive_disposition(char, uuid, text) OWNER TO ple_retention_broker;

REVOKE ALL ON FUNCTION ple_dispatch_due_retention_stages(integer),
    ple_extend_course_retention(char, uuid, integer),
    ple_set_archive_disposition(char, uuid, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_dispatch_due_retention_stages(integer),
    ple_extend_course_retention(char, uuid, integer),
    ple_set_archive_disposition(char, uuid, text) TO ple_app;

-- R3 worker entrypoints now prove the scheduler-created dispatch before they
-- expose a manifest or finalize an effect. Their remaining R3 behavior stays
-- byte-for-byte identical.
CREATE OR REPLACE FUNCTION ple_prepare_retention_work(p_tenant uuid, p_job uuid, p_token uuid,
                                           p_course uuid, p_stage text, p_generation bigint)
RETURNS jsonb LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE manifest jsonb;
BEGIN
    IF p_tenant <> public.ple_current_tenant() OR p_generation <= 0
       OR p_stage NOT IN ('notify', 'archiveStudentRecords', 'deleteStudentRecords') THEN
        RAISE EXCEPTION 'invalid retention worker capability' USING ERRCODE = '22023';
    END IF;
    PERFORM 1 FROM public.worker_job w JOIN public.course_retention r
      ON r.tenant_id=w.tenant_id AND r.course_id=p_course
      JOIN public.course_retention_dispatch d
        ON d.tenant_id=w.tenant_id AND d.course_id=p_course AND d.stage=p_stage
       AND d.generation=p_generation AND d.job_id=w.job_id
      WHERE w.job_id=p_job AND w.tenant_id=p_tenant AND w.state='leased'
        AND w.lease_token=p_token AND w.lease_expires_at > transaction_timestamp()
        AND r.generation=p_generation
        AND w.payload=jsonb_build_object('kind','retention','course',p_course::text,
            'stage',p_stage,'generation',p_generation) FOR UPDATE OF w;
    IF NOT FOUND THEN RETURN NULL; END IF;
    PERFORM 1 FROM public.course_retention_stage s
      WHERE s.tenant_id=p_tenant AND s.course_id=p_course AND s.stage=p_stage
        AND s.generation=p_generation AND s.due_at <= transaction_timestamp()
        AND (s.state='scheduled' OR (s.state='started' AND s.job_id=p_job)) FOR UPDATE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    UPDATE public.course_retention_stage SET state='started', job_id=p_job,
        lease_token=p_token, claimed_at=transaction_timestamp()
      WHERE tenant_id=p_tenant AND course_id=p_course AND stage=p_stage AND generation=p_generation;
    IF p_stage='notify' THEN RETURN jsonb_build_object('kind','notify'); END IF;
    UPDATE public.worker_job w SET state='dead', lease_token=NULL, lease_expires_at=NULL,
        completed_at=transaction_timestamp(), last_error='permanent'
      FROM public.student_export_request r
      WHERE r.tenant_id=p_tenant AND r.course_id=p_course AND r.job_id=w.job_id
        AND r.state='queued' AND w.state IN ('ready','leased');
    UPDATE public.student_export_request SET state='failed'
      WHERE tenant_id=p_tenant AND course_id=p_course AND state='queued';
    IF EXISTS (
        SELECT 1 FROM public.student_export_request r
        JOIN public.student_export_artifact a ON a.export_id=r.export_id
        WHERE r.tenant_id=p_tenant AND r.course_id=p_course AND a.object_payload IS NOT NULL
          AND (jsonb_typeof(a.object_payload) <> 'object'
               OR a.object_payload->>'id' <> a.object_id::text
               OR a.object_payload->>'bucket' <> 'student-records'
               OR a.object_payload->>'category' <> 'export'
               OR jsonb_typeof(a.object_payload->'key') <> 'object'
               OR a.object_payload->'key'->>'kind' <> 'studentRecord'
               OR a.object_payload->'key'->>'tenant' <> p_tenant::text
               OR a.object_payload->'key'->>'object' <> a.object_id::text)
    ) THEN RAISE EXCEPTION 'invalid student-record retention manifest' USING ERRCODE = '22023'; END IF;
    SELECT COALESCE(jsonb_agg(a.object_id::text ORDER BY a.object_id), '[]'::jsonb) INTO manifest
      FROM public.student_export_request r JOIN public.student_export_artifact a ON a.export_id=r.export_id
      WHERE r.tenant_id=p_tenant AND r.course_id=p_course;
    DELETE FROM public.asset_delivery d USING public.student_export_request r
      JOIN public.student_export_artifact a ON a.export_id=r.export_id
      WHERE r.tenant_id=p_tenant AND r.course_id=p_course AND d.tenant_id=p_tenant
        AND d.delivery_id=a.object_id;
    RETURN jsonb_build_object('kind','cleanup','objects',manifest);
END $$;

CREATE OR REPLACE FUNCTION ple_commit_retention_work(p_tenant uuid, p_job uuid, p_token uuid,
                                          p_course uuid, p_stage text, p_generation bigint)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
BEGIN
    IF p_tenant <> public.ple_current_tenant() THEN RAISE EXCEPTION 'invalid retention worker capability' USING ERRCODE = '22023'; END IF;
    PERFORM 1 FROM public.worker_job w JOIN public.course_retention_dispatch d
      ON d.tenant_id=w.tenant_id AND d.course_id=p_course AND d.stage=p_stage
     AND d.generation=p_generation AND d.job_id=w.job_id
      WHERE w.job_id=p_job AND w.tenant_id=p_tenant
        AND w.state='leased' AND w.lease_token=p_token AND w.lease_expires_at > transaction_timestamp()
        AND w.payload=jsonb_build_object('kind','retention','course',p_course::text,'stage',p_stage,'generation',p_generation) FOR UPDATE;
    IF NOT FOUND THEN RETURN false; END IF;
    PERFORM 1 FROM public.course_retention_stage s
      WHERE s.tenant_id=p_tenant AND s.course_id=p_course AND s.stage=p_stage
        AND s.generation=p_generation AND s.state='started' AND s.job_id=p_job AND s.lease_token=p_token
      FOR UPDATE;
    IF NOT FOUND THEN RETURN false; END IF;
    IF p_stage='notify' THEN
        INSERT INTO public.course_retention_notification (tenant_id, course_id, generation, intent)
        VALUES (p_tenant,p_course,p_generation,'archive') ON CONFLICT DO NOTHING;
    ELSIF p_stage NOT IN ('archiveStudentRecords','deleteStudentRecords') THEN RETURN false; END IF;
    UPDATE public.course_retention_stage SET state='completed'
      WHERE tenant_id=p_tenant AND course_id=p_course AND stage=p_stage AND generation=p_generation
        AND state='started' AND job_id=p_job AND lease_token=p_token;
    IF NOT FOUND THEN RETURN false; END IF;
    UPDATE public.worker_job SET state='completed', lease_token=NULL, lease_expires_at=NULL,
        completed_at=transaction_timestamp() WHERE job_id=p_job AND tenant_id=p_tenant AND state='leased'
        AND lease_token=p_token AND payload=jsonb_build_object('kind','retention','course',p_course::text,'stage',p_stage,'generation',p_generation);
    IF NOT FOUND THEN RETURN false; END IF;
    RETURN true;
END $$;
ALTER FUNCTION ple_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint) OWNER TO ple_retention_broker;
ALTER FUNCTION ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint) OWNER TO ple_retention_broker;
