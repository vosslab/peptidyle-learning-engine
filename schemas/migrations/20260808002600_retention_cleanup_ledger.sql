-- MOD-RETENTION R4.4A: trusted manifest ledger for deterministic learner cleanup.
-- Cleanup no longer infers StudentRecord object ownership from arbitrary JSON payloads.

CREATE TABLE course_retention_cleanup_manifest (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    generation bigint NOT NULL CHECK (generation > 0),
    stage text NOT NULL CHECK (stage IN ('archiveStudentRecords', 'deleteStudentRecords')),
    job_id uuid NOT NULL,
    state text NOT NULL CHECK (state IN ('prepared', 'completed')),
    object_count bigint NOT NULL CHECK (object_count >= 0),
    prepared_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    completed_at timestamptz,
    PRIMARY KEY (tenant_id, course_id, generation, stage),
    FOREIGN KEY (tenant_id, course_id, generation, stage)
        REFERENCES course_retention_stage (tenant_id, course_id, generation, stage)
        ON DELETE CASCADE,
    FOREIGN KEY (job_id) REFERENCES worker_job(job_id),
    FOREIGN KEY (tenant_id, course_id) REFERENCES course(tenant_id, course_id)
);
CREATE INDEX course_retention_cleanup_manifest_object_count_idx
    ON course_retention_cleanup_manifest (tenant_id, course_id, generation, stage, object_count)
    WHERE state = 'prepared';

CREATE TABLE course_retention_cleanup_manifest_object (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    generation bigint NOT NULL,
    stage text NOT NULL,
    object_id uuid NOT NULL,
    PRIMARY KEY (tenant_id, course_id, generation, stage, object_id),
    FOREIGN KEY (tenant_id, course_id, generation, stage)
        REFERENCES course_retention_cleanup_manifest (tenant_id, course_id, generation, stage)
        ON DELETE CASCADE,
    CHECK (
        tenant_id IS NOT NULL
        AND course_id IS NOT NULL
        AND object_id IS NOT NULL
    )
);
CREATE INDEX course_retention_cleanup_manifest_object_idx
    ON course_retention_cleanup_manifest_object (tenant_id, course_id, generation, stage, object_id);

ALTER TABLE course_retention_cleanup_manifest ENABLE ROW LEVEL SECURITY;
ALTER TABLE course_retention_cleanup_manifest FORCE ROW LEVEL SECURITY;
ALTER TABLE course_retention_cleanup_manifest_object ENABLE ROW LEVEL SECURITY;
ALTER TABLE course_retention_cleanup_manifest_object FORCE ROW LEVEL SECURITY;
CREATE POLICY retention_cleanup_manifest_broker ON course_retention_cleanup_manifest
    FOR ALL TO ple_retention_broker
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY retention_cleanup_manifest_object_broker ON course_retention_cleanup_manifest_object
    FOR ALL TO ple_retention_broker
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

GRANT SELECT, INSERT, UPDATE, DELETE ON
    course_retention_cleanup_manifest,
    course_retention_cleanup_manifest_object
    TO ple_retention_broker;
GRANT SELECT ON
    external_tool_exchange,
    question_attempt,
    assignment_run,
    enrollment,
    assignment
    TO ple_retention_broker;
REVOKE ALL ON
    course_retention_cleanup_manifest,
    course_retention_cleanup_manifest_object
    FROM PUBLIC, ple_app, ple_student, ple_grader, ple_queue_broker;

DROP POLICY IF EXISTS retention_broker_external_tool_exchange ON external_tool_exchange;
CREATE POLICY retention_broker_external_tool_exchange ON external_tool_exchange
    FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_question_attempt_tenant ON question_attempt;
CREATE POLICY retention_broker_question_attempt_tenant ON question_attempt
    FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_assignment_run_tenant ON assignment_run;
CREATE POLICY retention_broker_assignment_run_tenant ON assignment_run
    FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_enrollment_tenant ON enrollment;
CREATE POLICY retention_broker_enrollment_tenant ON enrollment
    FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_assignment_tenant ON assignment;
CREATE POLICY retention_broker_assignment_tenant ON assignment
    FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

-- Asset access audit rows are now anchored to normalized delivery scope.
-- Existing rows must be backfilled before migration to avoid silent rewrites.
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM public.audit_event) THEN
        RAISE EXCEPTION 'audit_event migration requires an explicit event backfill';
    END IF;
END $$;
ALTER TABLE audit_event
    ADD COLUMN delivery_scope text NOT NULL,
    ADD COLUMN delivery_id uuid NOT NULL,
    ADD COLUMN course_id uuid;
ALTER TABLE audit_event
    ADD CONSTRAINT audit_event_delivery_scope_shape CHECK (
        (delivery_scope = 'catalog' AND delivery_id IS NOT NULL AND course_id IS NULL)
        OR (delivery_scope = 'student_record' AND delivery_id IS NOT NULL AND course_id IS NOT NULL)
    );
ALTER TABLE audit_event
    ADD CONSTRAINT audit_event_delivery_scope
        CHECK (delivery_scope IN ('catalog', 'student_record')),
    ADD CONSTRAINT audit_event_delivery_course_fk
        FOREIGN KEY (tenant_id, course_id) REFERENCES course(tenant_id, course_id);
CREATE INDEX audit_event_tenant_course_time_idx
    ON audit_event (tenant_id, course_id, occurred_at)
    WHERE delivery_scope = 'student_record';

-- R4.4A worker contract:
--  * validate exact tenant/stage/course/generation/lease/job binding,
--  * replay an existing prepared manifest for lease renewal,
--  * persist a relational manifest before return,
--  * do not infer ownership from JSON payloads.
CREATE OR REPLACE FUNCTION ple_prepare_retention_work(p_tenant uuid, p_job uuid, p_token uuid,
                                           p_course uuid, p_stage text, p_generation bigint)
RETURNS jsonb LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE
    manifest jsonb;
    existing_object_count bigint;
    existing_object_rows bigint;
    existing_state text;
    existing_job uuid;
    object_count bigint;
    object_ids uuid[];
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_generation <= 0
       OR p_stage NOT IN ('notify', 'archiveStudentRecords', 'deleteStudentRecords') THEN
        RAISE EXCEPTION 'invalid retention worker capability' USING ERRCODE = '22023';
    END IF;

    IF p_stage = 'deleteStudentRecords' THEN
        RETURN NULL;
    END IF;

    -- Validate the exact scheduled worker binding before any rows can be touched.
    PERFORM 1 FROM public.worker_job w
      JOIN public.course_retention r
        ON r.tenant_id = w.tenant_id AND r.course_id = p_course
      JOIN public.course_retention_dispatch d
        ON d.tenant_id = w.tenant_id AND d.course_id = p_course AND d.stage = p_stage
       AND d.generation = p_generation AND d.job_id = w.job_id
     WHERE w.job_id = p_job
       AND w.tenant_id = p_tenant
       AND w.state = 'leased'
       AND w.lease_token = p_token
       AND w.lease_expires_at > transaction_timestamp()
       AND r.generation = p_generation
       AND w.payload = jsonb_build_object('kind','retention','course',p_course::text,
           'stage',p_stage,'generation',p_generation)
     FOR UPDATE OF w, r;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    -- Stage start must match the scheduled stage row or an already-bound lease.
    PERFORM 1 FROM public.course_retention_stage s
      WHERE s.tenant_id = p_tenant
        AND s.course_id = p_course
        AND s.stage = p_stage
        AND s.generation = p_generation
        AND s.due_at <= transaction_timestamp()
        AND (s.state = 'scheduled' OR (s.state = 'started' AND s.job_id = p_job))
      FOR UPDATE;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    IF p_stage = 'notify' THEN
        UPDATE public.course_retention_stage SET state = 'started', job_id = p_job,
            lease_token = p_token, claimed_at = transaction_timestamp()
          WHERE tenant_id = p_tenant AND course_id = p_course AND stage = p_stage
            AND generation = p_generation;
        IF NOT FOUND THEN
            RETURN NULL;
        END IF;
        RETURN jsonb_build_object('kind', 'notify');
    END IF;

    -- Lease renewals replay the previously derived manifest for the same job.
    SELECT
        m.state,
        m.job_id,
        m.object_count,
        COALESCE((
            SELECT COUNT(*)
              FROM public.course_retention_cleanup_manifest_object o
             WHERE o.tenant_id = m.tenant_id
               AND o.course_id = m.course_id
               AND o.generation = m.generation
               AND o.stage = m.stage
        ), 0)
      INTO existing_state, existing_job, existing_object_count, existing_object_rows
      FROM public.course_retention_cleanup_manifest m
      WHERE m.tenant_id = p_tenant
        AND m.course_id = p_course
        AND m.generation = p_generation
        AND m.stage = p_stage
      FOR UPDATE;
    IF FOUND THEN
        IF existing_state <> 'prepared' OR existing_job IS DISTINCT FROM p_job THEN
            RETURN NULL;
        END IF;
        IF existing_object_count IS DISTINCT FROM existing_object_rows THEN
            RETURN NULL;
        END IF;

        UPDATE public.course_retention_stage SET state = 'started', job_id = p_job,
            lease_token = p_token, claimed_at = transaction_timestamp()
          WHERE tenant_id = p_tenant AND course_id = p_course
            AND generation = p_generation AND stage = p_stage;
        IF NOT FOUND THEN
            RETURN NULL;
        END IF;

        SELECT COALESCE(jsonb_agg(object_id::text ORDER BY object_id), '[]'::jsonb)
          INTO manifest
          FROM public.course_retention_cleanup_manifest_object
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = p_generation
           AND stage = p_stage;

        RETURN jsonb_build_object('kind', 'cleanup', 'objects', manifest);
    END IF;

    -- Bind this stage to this lease before any delivery delete side effects.
    UPDATE public.course_retention_stage SET state = 'started', job_id = p_job,
        lease_token = p_token, claimed_at = transaction_timestamp()
      WHERE tenant_id = p_tenant AND course_id = p_course
        AND generation = p_generation AND stage = p_stage;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    -- Export artifacts are queued by students and may contain full payloads;
    -- reject malformed rows before deliveries are revoked.
    UPDATE public.worker_job w
       SET state='dead', lease_token=NULL, lease_expires_at=NULL,
           completed_at=transaction_timestamp(), last_error='permanent'
      FROM public.student_export_request r
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course
       AND r.job_id = w.job_id
       AND r.state = 'queued'
       AND w.state IN ('ready', 'leased');
    UPDATE public.student_export_request
       SET state = 'failed'
     WHERE tenant_id = p_tenant AND course_id = p_course AND state = 'queued';

    IF EXISTS (
        SELECT 1
          FROM public.student_export_request r
          JOIN public.student_export_artifact a
            ON a.export_id = r.export_id
         WHERE r.tenant_id = p_tenant
           AND r.course_id = p_course
           AND a.object_payload IS NOT NULL
           AND (
               jsonb_typeof(a.object_payload) <> 'object'
               OR a.object_payload->>'id' <> a.object_id::text
               OR a.object_payload->>'bucket' <> 'student-records'
               OR a.object_payload->>'category' <> 'export'
               OR jsonb_typeof(a.object_payload->'key') <> 'object'
               OR a.object_payload->'key'->>'kind' <> 'studentRecord'
               OR a.object_payload->'key'->>'tenant' <> p_tenant::text
               OR a.object_payload->'key'->>'object' <> a.object_id::text
           )
    ) THEN
        RAISE EXCEPTION 'invalid student-record retention manifest' USING ERRCODE = '22023';
    END IF;

    WITH manifest_object AS (
        SELECT DISTINCT object_id
          FROM (
              SELECT a.object_id AS object_id
                FROM public.student_export_request r
                JOIN public.student_export_artifact a
                  ON a.export_id = r.export_id
               WHERE r.tenant_id = p_tenant
                 AND r.course_id = p_course
              UNION
              SELECT et.transcript_object_id AS object_id
                FROM public.external_tool_exchange et
                JOIN public.question_attempt qa
                  ON qa.tenant_id = et.tenant_id
                 AND qa.attempt_id = et.attempt_id
                JOIN public.assignment_run ar
                  ON ar.tenant_id = qa.tenant_id
                 AND ar.run_id = qa.run_id
                JOIN public.enrollment e
                  ON e.tenant_id = ar.tenant_id
                 AND e.enrollment_id = ar.enrollment_id
                JOIN public.assignment a
                  ON a.tenant_id = e.tenant_id
                 AND a.assignment_id = e.assignment_id
               WHERE et.tenant_id = p_tenant
                 AND a.course_id = p_course
                 AND et.transcript_object_id IS NOT NULL
          ) AS combined
         WHERE object_id IS NOT NULL
    )
    SELECT
        COALESCE(jsonb_agg(object_id::text ORDER BY object_id), '[]'::jsonb),
        COALESCE(array_agg(object_id ORDER BY object_id), ARRAY[]::uuid[]),
        COUNT(*)
      INTO manifest, object_ids, object_count
      FROM manifest_object;

    INSERT INTO public.course_retention_cleanup_manifest
        (tenant_id, course_id, generation, stage, job_id, state, object_count, prepared_at)
    VALUES (p_tenant, p_course, p_generation, p_stage, p_job, 'prepared', object_count,
            transaction_timestamp());

    INSERT INTO public.course_retention_cleanup_manifest_object
        (tenant_id, course_id, generation, stage, object_id)
    SELECT p_tenant, p_course, p_generation, p_stage, object_id
      FROM unnest(object_ids) AS object_id;

    DELETE FROM public.asset_delivery d
      USING public.student_export_request r
      JOIN public.student_export_artifact a
        ON a.export_id = r.export_id
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course
       AND d.tenant_id = p_tenant
       AND d.delivery_id = a.object_id;

    RETURN jsonb_build_object('kind', 'cleanup', 'objects', manifest);
END $$;

CREATE OR REPLACE FUNCTION ple_commit_retention_work(p_tenant uuid, p_job uuid, p_token uuid,
                                          p_course uuid, p_stage text, p_generation bigint)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE
    lifecycle text;
    prepared_count bigint;
    manifest_count bigint;
BEGIN
    IF p_tenant IS NULL
       OR p_course IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_generation <= 0
       OR p_stage NOT IN ('notify', 'archiveStudentRecords', 'deleteStudentRecords') THEN
        RETURN false;
    END IF;

    PERFORM 1
      FROM public.worker_job w
      JOIN public.course_retention_dispatch d
        ON d.tenant_id = w.tenant_id AND d.course_id = p_course AND d.stage = p_stage
       AND d.generation = p_generation AND d.job_id = w.job_id
      JOIN public.course_retention r
        ON r.tenant_id = d.tenant_id AND r.course_id = d.course_id
       AND r.generation = d.generation
     WHERE w.job_id = p_job
       AND w.tenant_id = p_tenant
       AND w.state = 'leased'
       AND w.lease_token = p_token
       AND w.lease_expires_at > transaction_timestamp()
       AND w.payload = jsonb_build_object('kind', 'retention', 'course', p_course::text,
                                          'stage', p_stage, 'generation', p_generation)
     FOR UPDATE OF w, r;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    PERFORM 1
      FROM public.course_retention_stage s
     WHERE s.tenant_id = p_tenant
       AND s.course_id = p_course
       AND s.stage = p_stage
       AND s.generation = p_generation
       AND s.state = 'started'
       AND s.job_id = p_job
       AND s.lease_token = p_token
     FOR UPDATE;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    -- All required checks complete before mutation.
    IF p_stage = 'notify' THEN
        INSERT INTO public.course_retention_notification (tenant_id, course_id, generation, intent)
        VALUES (p_tenant,p_course,p_generation,'archive') ON CONFLICT DO NOTHING;
    ELSIF p_stage IN ('archiveStudentRecords', 'deleteStudentRecords') THEN
        SELECT m.object_count,
               COALESCE((
                   SELECT COUNT(*)
                     FROM public.course_retention_cleanup_manifest_object o
                    WHERE o.tenant_id = m.tenant_id
                      AND o.course_id = m.course_id
                      AND o.generation = m.generation
                      AND o.stage = m.stage
               ), 0)
          INTO prepared_count, manifest_count
          FROM public.course_retention_cleanup_manifest m
         WHERE m.tenant_id = p_tenant
           AND m.course_id = p_course
           AND m.generation = p_generation
           AND m.stage = p_stage
           AND m.job_id = p_job
           AND m.state = 'prepared'
         FOR UPDATE;
        IF NOT FOUND OR prepared_count IS NULL THEN
            RETURN false;
        END IF;

        IF manifest_count <> prepared_count THEN
            RETURN false;
        END IF;

        IF p_stage = 'archiveStudentRecords' THEN
            SELECT lifecycle INTO lifecycle
              FROM public.course_retention m
             WHERE m.tenant_id = p_tenant
               AND m.course_id = p_course
               AND m.generation = p_generation
               AND m.lifecycle = 'active'
             FOR UPDATE;
            IF NOT FOUND OR lifecycle <> 'active' THEN
                RETURN false;
            END IF;
        END IF;
    ELSE
        RETURN false;
    END IF;

    IF p_stage IN ('archiveStudentRecords', 'deleteStudentRecords') THEN
        UPDATE public.course_retention_cleanup_manifest
           SET state = 'completed', completed_at = transaction_timestamp()
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = p_generation
           AND stage = p_stage
           AND job_id = p_job
           AND state = 'prepared';
        IF NOT FOUND THEN
            RAISE EXCEPTION 'failed to finalize retention manifest for stage %', p_stage
                USING ERRCODE = '45000';
        END IF;
    END IF;


    UPDATE public.course_retention_stage SET state = 'completed'
      WHERE tenant_id = p_tenant
        AND course_id = p_course
        AND stage = p_stage
        AND generation = p_generation
        AND state = 'started'
        AND job_id = p_job
        AND lease_token = p_token;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to finalize retention stage for stage %', p_stage
            USING ERRCODE = '45000';
    END IF;

    IF p_stage = 'archiveStudentRecords' THEN
        UPDATE public.course_retention
           SET lifecycle = 'archived'
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = p_generation
           AND lifecycle = 'active';
        IF NOT FOUND THEN
            RAISE EXCEPTION 'failed to transition course retention lifecycle for stage %', p_stage
                USING ERRCODE = '45000';
        END IF;
    END IF;

    UPDATE public.worker_job
       SET state = 'completed', lease_token = NULL, lease_expires_at = NULL,
           completed_at = transaction_timestamp()
       WHERE job_id = p_job
       AND tenant_id = p_tenant
       AND state = 'leased'
       AND lease_token = p_token
       AND payload = jsonb_build_object('kind', 'retention', 'course', p_course::text,
                                      'stage', p_stage, 'generation', p_generation);
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to complete retention worker job for stage %', p_stage
            USING ERRCODE = '45000';
    END IF;

    RETURN true;
END $$;

ALTER FUNCTION ple_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;
ALTER FUNCTION ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION ple_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint),
    ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    FROM PUBLIC, ple_app;
GRANT EXECUTE ON FUNCTION ple_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint),
    ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    TO ple_app;
