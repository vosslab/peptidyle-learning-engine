-- WP-PROF-T4: durable, non-submission idempotency for instructor rehearsal.
-- This is deliberately separate from the submission claim protocol in 1811.
BEGIN;

CREATE TABLE public.rehearsal_start_receipt (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    direct_instructor_membership_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    request_fingerprint bytea NOT NULL,
    rehearsal_run_id uuid NOT NULL,
    disposition text NOT NULL,
    response_projection jsonb NOT NULL,
    response_digest bytea NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY (tenant_id, course_id, assignment_id, direct_instructor_membership_id, actor_id, idempotency_key),
    FOREIGN KEY (tenant_id, rehearsal_run_id) REFERENCES public.rehearsal_run (tenant_id, rehearsal_run_id) ON DELETE RESTRICT,
    CONSTRAINT rehearsal_start_receipt_key_check CHECK (char_length(idempotency_key) BETWEEN 1 AND 128 AND idempotency_key !~ '[[:cntrl:]]'),
    CONSTRAINT rehearsal_start_receipt_fingerprint_check CHECK (octet_length(request_fingerprint) = 32),
    CONSTRAINT rehearsal_start_receipt_disposition_check CHECK (disposition IN ('started', 'resumed', 'replacedDifferentSubject', 'restartedAfterCompletion')),
    CONSTRAINT rehearsal_start_receipt_projection_check CHECK (jsonb_typeof(response_projection) = 'object' AND public.ple_rehearsal_jsonb_bytes(response_projection) <= 65536),
    CONSTRAINT rehearsal_start_receipt_digest_check CHECK (octet_length(response_digest) = 32)
);

CREATE TABLE public.rehearsal_delivery_operation_root (
    tenant_id uuid NOT NULL,
    rehearsal_run_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    operation_id uuid NOT NULL,
    request_fingerprint bytea NOT NULL,
    selected_attempt_id uuid NOT NULL,
    sealed_delivery_plan jsonb NOT NULL,
    plan_digest bytea NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY (tenant_id, rehearsal_run_id, idempotency_key),
    UNIQUE (tenant_id, rehearsal_run_id, operation_id),
    FOREIGN KEY (tenant_id, rehearsal_run_id) REFERENCES public.rehearsal_run (tenant_id, rehearsal_run_id) ON DELETE RESTRICT,
    CONSTRAINT rehearsal_delivery_root_key_check CHECK (char_length(idempotency_key) BETWEEN 1 AND 128 AND idempotency_key !~ '[[:cntrl:]]'),
    CONSTRAINT rehearsal_delivery_root_fingerprint_check CHECK (octet_length(request_fingerprint) = 32),
    CONSTRAINT rehearsal_delivery_root_plan_check CHECK (jsonb_typeof(sealed_delivery_plan) = 'object' AND public.ple_rehearsal_jsonb_bytes(sealed_delivery_plan) <= 524288),
    CONSTRAINT rehearsal_delivery_root_digest_check CHECK (octet_length(plan_digest) = 32)
);

CREATE TABLE public.rehearsal_delivery_operation_event (
    tenant_id uuid NOT NULL,
    rehearsal_run_id uuid NOT NULL,
    operation_id uuid NOT NULL,
    sequence bigint NOT NULL,
    phase text NOT NULL,
    completion_kind text,
    frozen_attempt_id uuid,
    screen_digest bytea,
    abandonment_reason text,
    recorded_at timestamp with time zone NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY (tenant_id, rehearsal_run_id, operation_id, sequence),
    FOREIGN KEY (tenant_id, rehearsal_run_id, operation_id) REFERENCES public.rehearsal_delivery_operation_root (tenant_id, rehearsal_run_id, operation_id) ON DELETE RESTRICT,
    CONSTRAINT rehearsal_delivery_event_sequence_check CHECK (sequence BETWEEN 1 AND 4294967295),
    CONSTRAINT rehearsal_delivery_event_phase_check CHECK (phase IN ('prepared', 'issueDispatched', 'completed', 'abandonedBeforeDispatch', 'revokedStaleRevision', 'revokedTerminalLifecycle', 'revokedSourceContextRemoved')),
    CONSTRAINT rehearsal_delivery_event_material_check CHECK (
        (phase = 'completed' AND completion_kind IN ('issued', 'completed') AND screen_digest IS NOT NULL AND octet_length(screen_digest) = 32 AND abandonment_reason IS NULL)
        OR (phase = 'abandonedBeforeDispatch' AND completion_kind IS NULL AND frozen_attempt_id IS NULL AND screen_digest IS NULL AND abandonment_reason IN ('localPreparationFailed', 'nativeBackendAdmissionRejected', 'trustedRendererAdmissionRejected'))
        OR (phase NOT IN ('completed', 'abandonedBeforeDispatch') AND completion_kind IS NULL AND frozen_attempt_id IS NULL AND screen_digest IS NULL AND abandonment_reason IS NULL)
    )
);

CREATE TABLE public.rehearsal_delivery_receipt (
    tenant_id uuid NOT NULL,
    rehearsal_run_id uuid NOT NULL,
    operation_id uuid NOT NULL,
    result_kind text NOT NULL,
    frozen_attempt_id uuid,
    screen_projection jsonb NOT NULL,
    screen_digest bytea NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY (tenant_id, rehearsal_run_id, operation_id),
    FOREIGN KEY (tenant_id, rehearsal_run_id, operation_id) REFERENCES public.rehearsal_delivery_operation_root (tenant_id, rehearsal_run_id, operation_id) ON DELETE RESTRICT,
    FOREIGN KEY (tenant_id, rehearsal_run_id, frozen_attempt_id) REFERENCES public.rehearsal_frozen_item (tenant_id, rehearsal_run_id, attempt_id) ON DELETE RESTRICT,
    CONSTRAINT rehearsal_delivery_receipt_kind_check CHECK (result_kind IN ('issued', 'completed')),
    CONSTRAINT rehearsal_delivery_receipt_frozen_shape_check CHECK ((result_kind = 'issued' AND frozen_attempt_id IS NOT NULL) OR (result_kind = 'completed' AND frozen_attempt_id IS NULL)),
    CONSTRAINT rehearsal_delivery_receipt_projection_check CHECK (jsonb_typeof(screen_projection) = 'object' AND public.ple_rehearsal_jsonb_bytes(screen_projection) <= 262144),
    CONSTRAINT rehearsal_delivery_receipt_digest_check CHECK (octet_length(screen_digest) = 32)
);

CREATE TABLE public.rehearsal_discard_receipt (
    tenant_id uuid NOT NULL,
    rehearsal_run_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    request_fingerprint bytea NOT NULL,
    response_projection jsonb NOT NULL,
    response_digest bytea NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY (tenant_id, rehearsal_run_id, idempotency_key),
    FOREIGN KEY (tenant_id, rehearsal_run_id) REFERENCES public.rehearsal_run (tenant_id, rehearsal_run_id) ON DELETE RESTRICT,
    CONSTRAINT rehearsal_discard_receipt_key_check CHECK (char_length(idempotency_key) BETWEEN 1 AND 128 AND idempotency_key !~ '[[:cntrl:]]'),
    CONSTRAINT rehearsal_discard_receipt_fingerprint_check CHECK (octet_length(request_fingerprint) = 32),
    CONSTRAINT rehearsal_discard_receipt_projection_check CHECK (jsonb_typeof(response_projection) = 'object' AND public.ple_rehearsal_jsonb_bytes(response_projection) <= 65536),
    CONSTRAINT rehearsal_discard_receipt_digest_check CHECK (octet_length(response_digest) = 32)
);

CREATE FUNCTION public.ple_rehearsal_delivery_operation_guard() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
BEGIN
    IF TG_OP IN ('UPDATE', 'DELETE') THEN
        RAISE EXCEPTION 'rehearsal operation records are append-only' USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION public.ple_revoke_open_rehearsal_delivery_operations() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE reason text;
BEGIN
    IF OLD.lifecycle = 'active' AND NEW.lifecycle <> 'active' THEN
        reason := CASE NEW.lifecycle
            WHEN 'discardedStaleRevision' THEN 'revokedStaleRevision'
            WHEN 'discardedSourceContextRemoved' THEN 'revokedSourceContextRemoved'
            ELSE 'revokedTerminalLifecycle'
        END;
        INSERT INTO public.rehearsal_delivery_operation_event (tenant_id, rehearsal_run_id, operation_id, sequence, phase)
        SELECT root.tenant_id, root.rehearsal_run_id, root.operation_id, latest.sequence + 1, reason
          FROM public.rehearsal_delivery_operation_root root
          CROSS JOIN LATERAL (
              SELECT event.sequence, event.phase
                FROM public.rehearsal_delivery_operation_event event
               WHERE event.tenant_id = root.tenant_id AND event.rehearsal_run_id = root.rehearsal_run_id
                 AND event.operation_id = root.operation_id
               ORDER BY event.sequence DESC LIMIT 1
          ) latest
         WHERE root.tenant_id = NEW.tenant_id AND root.rehearsal_run_id = NEW.rehearsal_run_id
           AND latest.phase IN ('prepared', 'issueDispatched');
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER rehearsal_start_receipt_append_only BEFORE UPDATE OR DELETE ON public.rehearsal_start_receipt FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_delivery_operation_guard();
CREATE TRIGGER rehearsal_delivery_root_append_only BEFORE UPDATE OR DELETE ON public.rehearsal_delivery_operation_root FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_delivery_operation_guard();
CREATE TRIGGER rehearsal_delivery_event_append_only BEFORE UPDATE OR DELETE ON public.rehearsal_delivery_operation_event FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_delivery_operation_guard();
CREATE TRIGGER rehearsal_delivery_receipt_append_only BEFORE UPDATE OR DELETE ON public.rehearsal_delivery_receipt FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_delivery_operation_guard();
CREATE TRIGGER rehearsal_discard_receipt_append_only BEFORE UPDATE OR DELETE ON public.rehearsal_discard_receipt FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_delivery_operation_guard();
CREATE TRIGGER rehearsal_run_delivery_operation_revocation AFTER UPDATE OF lifecycle ON public.rehearsal_run FOR EACH ROW EXECUTE FUNCTION public.ple_revoke_open_rehearsal_delivery_operations();

ALTER TABLE public.rehearsal_start_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.rehearsal_start_receipt FORCE ROW LEVEL SECURITY;
ALTER TABLE public.rehearsal_delivery_operation_root ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.rehearsal_delivery_operation_root FORCE ROW LEVEL SECURITY;
ALTER TABLE public.rehearsal_delivery_operation_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.rehearsal_delivery_operation_event FORCE ROW LEVEL SECURITY;
ALTER TABLE public.rehearsal_delivery_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.rehearsal_delivery_receipt FORCE ROW LEVEL SECURITY;
ALTER TABLE public.rehearsal_discard_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.rehearsal_discard_receipt FORCE ROW LEVEL SECURITY;
CREATE POLICY rehearsal_start_receipt_app_tenant ON public.rehearsal_start_receipt TO ple_app USING (tenant_id = public.ple_current_tenant());
CREATE POLICY rehearsal_delivery_root_app_tenant ON public.rehearsal_delivery_operation_root TO ple_app USING (tenant_id = public.ple_current_tenant());
CREATE POLICY rehearsal_delivery_event_app_tenant ON public.rehearsal_delivery_operation_event TO ple_app USING (tenant_id = public.ple_current_tenant());
CREATE POLICY rehearsal_delivery_receipt_app_tenant ON public.rehearsal_delivery_receipt TO ple_app USING (tenant_id = public.ple_current_tenant());
CREATE POLICY rehearsal_discard_receipt_app_tenant ON public.rehearsal_discard_receipt TO ple_app USING (tenant_id = public.ple_current_tenant());
CREATE POLICY rehearsal_start_receipt_broker_tenant ON public.rehearsal_start_receipt TO ple_rehearsal_broker USING (tenant_id = public.ple_current_tenant()) WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY rehearsal_delivery_root_broker_tenant ON public.rehearsal_delivery_operation_root TO ple_rehearsal_broker USING (tenant_id = public.ple_current_tenant()) WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY rehearsal_delivery_event_broker_tenant ON public.rehearsal_delivery_operation_event TO ple_rehearsal_broker USING (tenant_id = public.ple_current_tenant()) WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY rehearsal_delivery_receipt_broker_tenant ON public.rehearsal_delivery_receipt TO ple_rehearsal_broker USING (tenant_id = public.ple_current_tenant()) WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY rehearsal_discard_receipt_broker_tenant ON public.rehearsal_discard_receipt TO ple_rehearsal_broker USING (tenant_id = public.ple_current_tenant()) WITH CHECK (tenant_id = public.ple_current_tenant());

GRANT SELECT, INSERT ON public.rehearsal_start_receipt, public.rehearsal_delivery_operation_root, public.rehearsal_delivery_operation_event, public.rehearsal_delivery_receipt, public.rehearsal_discard_receipt TO ple_rehearsal_broker;
GRANT SELECT ON public.rehearsal_start_receipt, public.rehearsal_delivery_operation_root, public.rehearsal_delivery_operation_event, public.rehearsal_delivery_receipt, public.rehearsal_discard_receipt TO ple_app;
ALTER FUNCTION public.ple_rehearsal_delivery_operation_guard() OWNER TO ple_rehearsal_broker;
ALTER FUNCTION public.ple_revoke_open_rehearsal_delivery_operations() OWNER TO ple_rehearsal_broker;
REVOKE ALL ON FUNCTION public.ple_rehearsal_delivery_operation_guard(), public.ple_revoke_open_rehearsal_delivery_operations() FROM PUBLIC, ple_app;
REVOKE EXECUTE ON FUNCTION public.ple_rehearsal_start(uuid, uuid, uuid, uuid, integer, bigint, jsonb, bytea, bytea, uuid, boolean, uuid) FROM ple_app;

COMMIT;
