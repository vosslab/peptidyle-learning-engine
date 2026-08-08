-- One assignment export owns an immutable four-artifact bundle and one queue job.
CREATE TABLE student_export_request (
    export_id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    requester_id uuid NOT NULL,
    job_id uuid NOT NULL UNIQUE REFERENCES worker_job(job_id),
    manifest_object_id uuid NOT NULL UNIQUE,
    frozen_payload jsonb NOT NULL CHECK (jsonb_typeof(frozen_payload) = 'object'),
    frozen_payload_sha256 bytea NOT NULL CHECK (octet_length(frozen_payload_sha256) = 32),
    state text NOT NULL CHECK (state IN ('queued', 'ready', 'failed')),
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    ready_at timestamptz,
    CHECK ((state = 'ready' AND ready_at IS NOT NULL) OR (state <> 'ready' AND ready_at IS NULL))
);
CREATE UNIQUE INDEX student_export_request_tenant_manifest_idx
    ON student_export_request (tenant_id, manifest_object_id);

CREATE TABLE student_export_artifact (
    export_id uuid NOT NULL REFERENCES student_export_request(export_id),
    kind text NOT NULL CHECK (kind IN ('docx', 'pdf', 'accessibleDocx', 'accessiblePdf')),
    object_id uuid NOT NULL UNIQUE,
    delivery_id uuid,
    filename text,
    media_type text,
    object_payload jsonb,
    PRIMARY KEY (export_id, kind),
    CHECK ((delivery_id IS NULL AND filename IS NULL AND media_type IS NULL AND object_payload IS NULL)
        OR (delivery_id = object_id AND filename IS NOT NULL AND media_type IS NOT NULL
            AND object_payload IS NOT NULL))
);

ALTER TABLE student_export_request ENABLE ROW LEVEL SECURITY;
ALTER TABLE student_export_request FORCE ROW LEVEL SECURITY;
ALTER TABLE student_export_artifact ENABLE ROW LEVEL SECURITY;
ALTER TABLE student_export_artifact FORCE ROW LEVEL SECURITY;
CREATE POLICY student_export_request_tenant ON student_export_request TO ple_app
    USING (tenant_id = ple_current_tenant()) WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY student_export_artifact_tenant ON student_export_artifact TO ple_app
    USING (EXISTS (SELECT 1 FROM student_export_request r WHERE r.export_id = student_export_artifact.export_id
                  AND r.tenant_id = ple_current_tenant()));
GRANT SELECT, INSERT ON student_export_request, student_export_artifact TO ple_app;
GRANT SELECT, UPDATE ON worker_job, student_export_request, student_export_artifact TO ple_queue_broker;
GRANT INSERT ON asset_delivery TO ple_queue_broker;

-- Permanent or exhausted worker failures make an export safely terminal without exposing failure text.
CREATE FUNCTION ple_mark_failed_export_job() RETURNS trigger LANGUAGE plpgsql
SECURITY DEFINER SET search_path = pg_catalog, public AS $$
BEGIN
    IF NEW.state = 'dead' THEN
        UPDATE public.student_export_request SET state = 'failed'
         WHERE job_id = NEW.job_id AND state = 'queued';
    END IF;
    RETURN NEW;
END $$;
ALTER FUNCTION ple_mark_failed_export_job() OWNER TO ple_queue_broker;
REVOKE ALL ON FUNCTION ple_mark_failed_export_job() FROM PUBLIC;
CREATE TRIGGER worker_job_export_failure AFTER UPDATE OF state ON worker_job
    FOR EACH ROW EXECUTE FUNCTION ple_mark_failed_export_job();

-- The queue broker alone can couple active-lease validation, all delivery rows, result state and completion.
CREATE FUNCTION ple_commit_export_job(p_tenant uuid, p_job uuid, p_token uuid, p_manifest uuid,
                                      p_artifacts jsonb)
RETURNS text LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE
    request_id uuid;
    existing jsonb;
    supplied jsonb;
BEGIN
    IF p_tenant <> public.ple_current_tenant() OR jsonb_typeof(p_artifacts) <> 'array'
       OR jsonb_array_length(p_artifacts) <> 4 THEN
        RAISE EXCEPTION 'invalid export commit capability' USING ERRCODE = '22023';
    END IF;
    SELECT export_id INTO request_id FROM public.student_export_request
     WHERE tenant_id = p_tenant AND job_id = p_job AND manifest_object_id = p_manifest FOR UPDATE;
    IF request_id IS NULL THEN RETURN NULL; END IF;
    SELECT jsonb_agg(jsonb_build_object('kind', a.kind, 'object', a.object_id::text,
          'filename', a.filename, 'mediaType', a.media_type, 'objectRecord', a.object_payload)
          ORDER BY a.kind) INTO existing
      FROM public.student_export_artifact a WHERE a.export_id = request_id AND a.delivery_id IS NOT NULL;
    SELECT jsonb_agg(jsonb_build_object('kind', x->>'kind', 'object', x->>'object',
          'filename', x->>'filename', 'mediaType', x->>'mediaType, 'objectRecord', x->'objectRecord')
          ORDER BY x->>'kind') INTO supplied FROM jsonb_array_elements(p_artifacts) x;
    IF (SELECT state FROM public.student_export_request WHERE export_id = request_id) = 'ready' THEN
        IF existing = supplied THEN RETURN 'already_committed'; END IF;
        RETURN NULL;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM public.worker_job w WHERE w.job_id = p_job AND w.tenant_id = p_tenant
        AND w.state = 'leased' AND w.lease_token = p_token AND w.lease_expires_at > transaction_timestamp()
        AND w.payload = jsonb_build_object('kind','export','deliveryObject',p_manifest::text)) THEN
        RETURN NULL;
    END IF;
    IF (SELECT count(*) FROM jsonb_array_elements(p_artifacts) x) <> 4
       OR EXISTS (SELECT 1 FROM public.student_export_artifact a
                   WHERE a.export_id = request_id AND NOT EXISTS (
                       SELECT 1 FROM jsonb_array_elements(p_artifacts) x
                        WHERE x->>'kind' = a.kind AND x->>'object' = a.object_id::text)) THEN
        RETURN NULL;
    END IF;
    UPDATE public.student_export_artifact a
       SET delivery_id = a.object_id, filename = x->>'filename', media_type = x->>'mediaType',
           object_payload = x->'objectRecord'
      FROM jsonb_array_elements(p_artifacts) x
     WHERE a.export_id = request_id AND x->>'kind' = a.kind AND x->>'object' = a.object_id::text;
    INSERT INTO public.asset_delivery (delivery_id, delivery_kind, tenant_id, object_id, payload, payload_sha256)
      SELECT a.object_id, 'student_record', p_tenant, a.object_id, x->'delivery',
             x->>'deliverySha256'
        FROM public.student_export_artifact a JOIN jsonb_array_elements(p_artifacts) x
          ON x->>'kind' = a.kind AND x->>'object' = a.object_id::text
       WHERE a.export_id = request_id;
    UPDATE public.student_export_request SET state = 'ready', ready_at = transaction_timestamp()
     WHERE export_id = request_id;
    UPDATE public.worker_job SET state = 'completed', lease_token = NULL, lease_expires_at = NULL,
           completed_at = transaction_timestamp()
     WHERE job_id = p_job;
    RETURN 'committed';
END $$;
ALTER FUNCTION ple_commit_export_job(uuid, uuid, uuid, uuid, jsonb) OWNER TO ple_queue_broker;
REVOKE ALL ON FUNCTION ple_commit_export_job(uuid, uuid, uuid, uuid, jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_commit_export_job(uuid, uuid, uuid, uuid, jsonb) TO ple_app;
