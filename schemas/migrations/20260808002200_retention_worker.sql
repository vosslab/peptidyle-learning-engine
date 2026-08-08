-- MOD-RETENTION R3: lease-fenced worker effects.  Queue payloads name only a
-- course stage; this broker resolves and revokes exact student-record objects.

-- Keep the durable queue closed while admitting the one new key-free worker
-- family. This replaces the exact 01500 constraint rather than widening it.
ALTER TABLE worker_job DROP CONSTRAINT worker_job_payload_check;
ALTER TABLE worker_job ADD CONSTRAINT worker_job_payload_check CHECK (
    (payload->>'kind'='render' AND payload ?& ARRAY['kind','reference','seed']
     AND payload-ARRAY['kind','reference','seed']='{}'::jsonb
     AND jsonb_typeof(payload->'reference')='object'
     AND (payload->'reference') ?& ARRAY['problem','version']
     AND (payload->'reference')-ARRAY['problem','version']='{}'::jsonb
     AND jsonb_typeof(payload->'reference'->'problem')='string'
     AND jsonb_typeof(payload->'reference'->'version')='string'
     AND payload->'reference'->>'problem' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
     AND payload->'reference'->>'version' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
     AND jsonb_typeof(payload->'seed')='number' AND payload->>'seed' ~ '^(0|[1-9][0-9]{0,19})$'
     AND (payload->>'seed')::numeric <= 18446744073709551615)
 OR (payload->>'kind'='export' AND payload ?& ARRAY['kind','deliveryObject']
     AND payload-ARRAY['kind','deliveryObject']='{}'::jsonb
     AND jsonb_typeof(payload->'deliveryObject')='string'
     AND payload->>'deliveryObject' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$')
 OR (payload->>'kind'='import' AND payload ?& ARRAY['kind','sourceObject']
     AND payload-ARRAY['kind','sourceObject']='{}'::jsonb
     AND jsonb_typeof(payload->'sourceObject')='string'
     AND payload->>'sourceObject' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$')
 OR (payload->>'kind'='qtiImport' AND payload ?& ARRAY['kind','workspace','import','sourceObject']
     AND payload-ARRAY['kind','workspace','import','sourceObject']='{}'::jsonb
     AND jsonb_typeof(payload->'workspace')='string' AND jsonb_typeof(payload->'import')='string'
     AND jsonb_typeof(payload->'sourceObject')='string'
     AND payload->>'workspace' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
     AND payload->>'import' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
     AND payload->>'sourceObject' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$')
 OR (payload->>'kind'='retention' AND payload ?& ARRAY['kind','course','stage','generation']
     AND payload-ARRAY['kind','course','stage','generation']='{}'::jsonb
     AND jsonb_typeof(payload->'course')='string'
     AND payload->>'course' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
     AND payload->>'stage' IN ('notify','archiveStudentRecords','deleteStudentRecords')
     AND jsonb_typeof(payload->'generation')='number'
     AND payload->>'generation' ~ '^[1-9][0-9]{0,18}$'
     AND (payload->>'generation')::numeric <= 9223372036854775807)
);

CREATE TABLE course_retention_notification (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    generation bigint NOT NULL CHECK (generation > 0),
    intent text NOT NULL CHECK (intent IN ('archive', 'delete', 'extend')),
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, course_id, generation),
    FOREIGN KEY (tenant_id, course_id) REFERENCES course_retention(tenant_id, course_id) ON DELETE CASCADE
);

ALTER TABLE course_retention_stage
    ADD COLUMN job_id uuid,
    ADD COLUMN lease_token uuid,
    ADD COLUMN claimed_at timestamptz;
CREATE INDEX course_retention_stage_work_idx
    ON course_retention_stage (tenant_id, course_id, generation, stage)
    WHERE state IN ('scheduled', 'started');

ALTER TABLE course_retention_notification ENABLE ROW LEVEL SECURITY;
ALTER TABLE course_retention_notification FORCE ROW LEVEL SECURITY;
CREATE POLICY course_retention_notification_tenant ON course_retention_notification
    USING (tenant_id = ple_current_tenant()) WITH CHECK (tenant_id = ple_current_tenant());

GRANT SELECT, INSERT ON course_retention_notification TO ple_retention_broker;
GRANT SELECT, UPDATE ON course_retention_stage, worker_job, student_export_request
    TO ple_retention_broker;
GRANT SELECT ON course_retention, student_export_artifact TO ple_retention_broker;
GRANT SELECT, DELETE ON asset_delivery TO ple_retention_broker;
REVOKE ALL ON course_retention_notification, course_retention_stage, course_retention,
    student_export_request, student_export_artifact, asset_delivery
    FROM PUBLIC, ple_app, ple_student, ple_grader, ple_queue_broker;

-- `ple_retention_broker` is NOLOGIN/NOBYPASSRLS, so grants alone are not a
-- capability. These policies expose only the transaction-local tenant set by
-- the broker's authenticated caller; no public/application raw-table policy
-- is widened.
CREATE POLICY retention_broker_worker_job ON worker_job FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
CREATE POLICY retention_broker_worker_job_update ON worker_job FOR UPDATE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant()) WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY retention_broker_export_request ON student_export_request FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
CREATE POLICY retention_broker_export_request_update ON student_export_request FOR UPDATE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant()) WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY retention_broker_export_artifact ON student_export_artifact FOR SELECT TO ple_retention_broker
    USING (EXISTS (SELECT 1 FROM student_export_request r WHERE r.export_id = student_export_artifact.export_id
                   AND r.tenant_id = ple_current_tenant()));
CREATE POLICY retention_broker_asset_delivery ON asset_delivery FOR SELECT TO ple_retention_broker
    USING (delivery_kind = 'student_record' AND tenant_id = ple_current_tenant());
CREATE POLICY retention_broker_asset_delivery_delete ON asset_delivery FOR DELETE TO ple_retention_broker
    USING (delivery_kind = 'student_record' AND tenant_id = ple_current_tenant());

-- This function has the only manifest path. It accepts no object key and
-- removes protected delivery rows before a worker can call external storage.
CREATE FUNCTION ple_prepare_retention_work(p_tenant uuid, p_job uuid, p_token uuid,
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
    -- An export may have private bytes already written before its atomic
    -- delivery commit. Terminalize every non-ready export and revoke its job
    -- lease before yielding the expected object identities for cleanup.
    UPDATE public.worker_job w SET state='dead', lease_token=NULL, lease_expires_at=NULL,
        completed_at=transaction_timestamp(), last_error='permanent'
      FROM public.student_export_request r
      WHERE r.tenant_id=p_tenant AND r.course_id=p_course AND r.job_id=w.job_id
        AND r.state='queued' AND w.state IN ('ready','leased');
    UPDATE public.student_export_request SET state='failed'
      WHERE tenant_id=p_tenant AND course_id=p_course AND state='queued';
    -- Validate the entire exact manifest before revoking any delivery. A
    -- corrupt artifact row must fail closed, not turn a content/temp object
    -- into an unavailable delivery before Rust can reject its typed key.
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
ALTER FUNCTION ple_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint) OWNER TO ple_retention_broker;

CREATE FUNCTION ple_commit_retention_work(p_tenant uuid, p_job uuid, p_token uuid,
                                          p_course uuid, p_stage text, p_generation bigint)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
BEGIN
    IF p_tenant <> public.ple_current_tenant() THEN RAISE EXCEPTION 'invalid retention worker capability' USING ERRCODE = '22023'; END IF;
    PERFORM 1 FROM public.worker_job w WHERE w.job_id=p_job AND w.tenant_id=p_tenant
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
    ELSIF p_stage IN ('archiveStudentRecords','deleteStudentRecords') THEN
        -- R3 only deletes exact derived student-record artifacts. It does not
        -- yet revoke every educational row/access path, so R4 alone may make
        -- the persisted archived/deleted lifecycle claim.
        NULL;
    ELSE RETURN false; END IF;
    UPDATE public.course_retention_stage SET state='completed'
      WHERE tenant_id=p_tenant AND course_id=p_course AND stage=p_stage AND generation=p_generation
        AND state='started' AND job_id=p_job AND lease_token=p_token;
    IF NOT FOUND THEN RETURN false; END IF;
    IF p_stage='notify' THEN
        UPDATE public.course_retention SET lifecycle='active'
          WHERE tenant_id=p_tenant AND course_id=p_course AND generation=p_generation;
    END IF;
    UPDATE public.worker_job SET state='completed', lease_token=NULL, lease_expires_at=NULL,
        completed_at=transaction_timestamp() WHERE job_id=p_job AND tenant_id=p_tenant AND state='leased'
        AND lease_token=p_token AND payload=jsonb_build_object('kind','retention','course',p_course::text,'stage',p_stage,'generation',p_generation);
    IF NOT FOUND THEN RETURN false; END IF;
    RETURN true;
END $$;
ALTER FUNCTION ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint) OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION ple_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint),
    ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint),
    ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint) TO ple_app;
