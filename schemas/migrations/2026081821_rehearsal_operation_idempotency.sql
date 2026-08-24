-- WP-PROF-T4: durable idempotency and operation lifecycle for live instructor rehearsal.
-- Frozen source and private grader material are introduced only by 1822.
BEGIN;

CREATE TABLE public.rehearsal_start_operation_root (
    tenant_id uuid NOT NULL, course_id uuid NOT NULL, assignment_id uuid NOT NULL,
    direct_instructor_membership_id uuid NOT NULL, actor_id uuid NOT NULL,
    idempotency_key text NOT NULL, request_fingerprint bytea NOT NULL,
    operation_id uuid NOT NULL, resulting_run_id uuid NOT NULL, disposition text NOT NULL,
    structural_witness jsonb NOT NULL, structural_witness_digest bytea NOT NULL,
    prepare_nonce uuid NOT NULL, prepared_txid bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY (tenant_id,course_id,assignment_id,direct_instructor_membership_id,actor_id,idempotency_key),
    UNIQUE (tenant_id,operation_id),
    FOREIGN KEY (tenant_id,resulting_run_id) REFERENCES public.rehearsal_run(tenant_id,rehearsal_run_id) ON DELETE RESTRICT,
    CHECK (char_length(idempotency_key) BETWEEN 1 AND 128 AND idempotency_key !~ '[[:cntrl:]]'),
    CHECK (octet_length(request_fingerprint)=32 AND octet_length(structural_witness_digest)=32),
    CHECK (disposition IN ('started','resumed','replacedDifferentSubject','restartedAfterCompletion')),
    CHECK (jsonb_typeof(structural_witness)='object' AND public.ple_rehearsal_jsonb_bytes(structural_witness)<=65536)
);
CREATE TABLE public.rehearsal_start_receipt (
    tenant_id uuid NOT NULL, operation_id uuid NOT NULL, response_projection jsonb NOT NULL,
    response_digest bytea NOT NULL, created_at timestamptz NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY(tenant_id,operation_id),
    FOREIGN KEY(tenant_id,operation_id) REFERENCES public.rehearsal_start_operation_root(tenant_id,operation_id) ON DELETE RESTRICT,
    CHECK(jsonb_typeof(response_projection)='object' AND public.ple_rehearsal_jsonb_bytes(response_projection)<=65536),
    CHECK(octet_length(response_digest)=32)
);
CREATE TABLE public.rehearsal_discard_operation_root (
    tenant_id uuid NOT NULL, rehearsal_run_id uuid NOT NULL, idempotency_key text NOT NULL,
    request_fingerprint bytea NOT NULL, operation_id uuid NOT NULL, structural_witness jsonb NOT NULL,
    structural_witness_digest bytea NOT NULL, prepare_nonce uuid NOT NULL, prepared_txid bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY(tenant_id,rehearsal_run_id,idempotency_key), UNIQUE(tenant_id,operation_id),
    FOREIGN KEY(tenant_id,rehearsal_run_id) REFERENCES public.rehearsal_run(tenant_id,rehearsal_run_id) ON DELETE RESTRICT,
    CHECK(char_length(idempotency_key) BETWEEN 1 AND 128 AND idempotency_key !~ '[[:cntrl:]]'),
    CHECK(octet_length(request_fingerprint)=32 AND octet_length(structural_witness_digest)=32),
    CHECK(jsonb_typeof(structural_witness)='object' AND public.ple_rehearsal_jsonb_bytes(structural_witness)<=65536)
);
CREATE TABLE public.rehearsal_discard_receipt (
    tenant_id uuid NOT NULL, operation_id uuid NOT NULL, response_projection jsonb NOT NULL,
    response_digest bytea NOT NULL, created_at timestamptz NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY(tenant_id,operation_id),
    FOREIGN KEY(tenant_id,operation_id) REFERENCES public.rehearsal_discard_operation_root(tenant_id,operation_id) ON DELETE RESTRICT,
    CHECK(jsonb_typeof(response_projection)='object' AND public.ple_rehearsal_jsonb_bytes(response_projection)<=65536),
    CHECK(octet_length(response_digest)=32)
);
-- The root is the immutable user action.  A generation is one concrete
-- server execution of that action.  In particular an attested pre-dispatch
-- abandonment never reopens or mutates a previous execution.
CREATE TABLE public.rehearsal_delivery_operation_root (
    tenant_id uuid NOT NULL, rehearsal_run_id uuid NOT NULL, root_id uuid NOT NULL,
    idempotency_key text NOT NULL, request_fingerprint bytea NOT NULL,
    created_at timestamptz NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY(tenant_id,rehearsal_run_id,root_id),
    UNIQUE(tenant_id,rehearsal_run_id,idempotency_key),
    FOREIGN KEY(tenant_id,rehearsal_run_id) REFERENCES public.rehearsal_run(tenant_id,rehearsal_run_id) ON DELETE RESTRICT,
    CHECK(char_length(idempotency_key) BETWEEN 1 AND 128 AND idempotency_key !~ '[[:cntrl:]]'),
    CHECK(octet_length(request_fingerprint)=32)
);
CREATE TABLE public.rehearsal_delivery_operation_generation (
    tenant_id uuid NOT NULL, rehearsal_run_id uuid NOT NULL, root_id uuid NOT NULL,
    generation integer NOT NULL, operation_id uuid NOT NULL, selected_attempt_id uuid NOT NULL,
    execution_descriptor jsonb NOT NULL, descriptor_digest bytea NOT NULL,
    structural_binding jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY(tenant_id,rehearsal_run_id,root_id,generation),
    UNIQUE(tenant_id,rehearsal_run_id,operation_id),
    FOREIGN KEY(tenant_id,rehearsal_run_id,root_id) REFERENCES public.rehearsal_delivery_operation_root(tenant_id,rehearsal_run_id,root_id) ON DELETE RESTRICT,
    CHECK(generation BETWEEN 1 AND 2147483647),
    CHECK(octet_length(descriptor_digest)=32),
    CHECK(jsonb_typeof(execution_descriptor)='object' AND public.ple_rehearsal_jsonb_bytes(execution_descriptor)<=65536),
    CHECK(jsonb_typeof(structural_binding)='object' AND public.ple_rehearsal_jsonb_bytes(structural_binding)<=65536)
);
CREATE TABLE public.rehearsal_delivery_admission (
    tenant_id uuid NOT NULL,rehearsal_run_id uuid NOT NULL,idempotency_key text NOT NULL,
    request_fingerprint bytea NOT NULL,prepare_nonce uuid NOT NULL,admission_witness jsonb NOT NULL,
    admission_digest bytea NOT NULL,prepared_txid bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY(tenant_id,rehearsal_run_id,idempotency_key),
    FOREIGN KEY(tenant_id,rehearsal_run_id) REFERENCES public.rehearsal_run(tenant_id,rehearsal_run_id) ON DELETE RESTRICT,
    CHECK(char_length(idempotency_key) BETWEEN 1 AND 128 AND idempotency_key !~ '[[:cntrl:]]'),
    CHECK(octet_length(request_fingerprint)=32 AND octet_length(admission_digest)=32),
    CHECK(jsonb_typeof(admission_witness)='object' AND public.ple_rehearsal_jsonb_bytes(admission_witness)<=65536)
);
CREATE TABLE public.rehearsal_delivery_operation_event (
    tenant_id uuid NOT NULL,rehearsal_run_id uuid NOT NULL,root_id uuid NOT NULL,generation integer NOT NULL,operation_id uuid NOT NULL,sequence bigint NOT NULL,
    phase text NOT NULL,completion_kind text,frozen_attempt_id uuid,screen_digest bytea,abandonment_reason text,
    recorded_at timestamptz NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY(tenant_id,rehearsal_run_id,root_id,generation,sequence),
    FOREIGN KEY(tenant_id,rehearsal_run_id,root_id,generation) REFERENCES public.rehearsal_delivery_operation_generation(tenant_id,rehearsal_run_id,root_id,generation) ON DELETE RESTRICT,
    FOREIGN KEY(tenant_id,rehearsal_run_id,operation_id) REFERENCES public.rehearsal_delivery_operation_generation(tenant_id,rehearsal_run_id,operation_id) ON DELETE RESTRICT,
    CHECK(sequence BETWEEN 1 AND 4294967295),
    CHECK(phase IN ('prepared','issueDispatched','completed','expired','abandonedBeforeDispatch','revokedStaleRevision','revokedTerminalLifecycle','revokedSourceContextRemoved')),
    CHECK((phase='completed' AND completion_kind IN ('issued','completed') AND screen_digest IS NOT NULL AND octet_length(screen_digest)=32 AND abandonment_reason IS NULL) OR (phase='abandonedBeforeDispatch' AND completion_kind IS NULL AND frozen_attempt_id IS NULL AND screen_digest IS NULL AND abandonment_reason IN ('localPreparationFailed','nativeBackendAdmissionRejected','trustedRendererAdmissionRejected')) OR (phase NOT IN ('completed','abandonedBeforeDispatch') AND completion_kind IS NULL AND frozen_attempt_id IS NULL AND screen_digest IS NULL AND abandonment_reason IS NULL))
);
CREATE TABLE public.rehearsal_delivery_receipt (
    tenant_id uuid NOT NULL,rehearsal_run_id uuid NOT NULL,root_id uuid NOT NULL,generation integer NOT NULL,operation_id uuid NOT NULL,result_kind text NOT NULL,
    frozen_attempt_id uuid,screen_projection jsonb NOT NULL,screen_digest bytea NOT NULL,
    created_at timestamptz NOT NULL DEFAULT public.ple_rehearsal_now(),
    PRIMARY KEY(tenant_id,rehearsal_run_id,root_id,generation),
    UNIQUE(tenant_id,rehearsal_run_id,operation_id),
    FOREIGN KEY(tenant_id,rehearsal_run_id,root_id,generation) REFERENCES public.rehearsal_delivery_operation_generation(tenant_id,rehearsal_run_id,root_id,generation) ON DELETE RESTRICT,
    FOREIGN KEY(tenant_id,rehearsal_run_id,frozen_attempt_id) REFERENCES public.rehearsal_frozen_item(tenant_id,rehearsal_run_id,attempt_id) ON DELETE RESTRICT,
    CHECK(result_kind IN ('issued','completed')),
    CHECK((result_kind='issued' AND frozen_attempt_id IS NOT NULL) OR (result_kind='completed' AND frozen_attempt_id IS NULL)),
    CHECK(jsonb_typeof(screen_projection)='object' AND public.ple_rehearsal_jsonb_bytes(screen_projection)<=262144), CHECK(octet_length(screen_digest)=32)
);

CREATE FUNCTION public.ple_rehearsal_operation_guard() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
BEGIN IF TG_OP IN ('UPDATE','DELETE') THEN RAISE EXCEPTION 'rehearsal operation records are append-only' USING ERRCODE='55000'; END IF; RETURN NEW; END $$;
CREATE FUNCTION public.ple_rehearsal_require_start_receipt() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
BEGIN IF NOT EXISTS(SELECT 1 FROM public.rehearsal_start_receipt WHERE tenant_id=NEW.tenant_id AND operation_id=NEW.operation_id) THEN RAISE EXCEPTION 'prepared rehearsal start must receive a receipt before commit' USING ERRCODE='23514'; END IF; RETURN NULL; END $$;
CREATE FUNCTION public.ple_rehearsal_require_discard_receipt() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
BEGIN IF NOT EXISTS(SELECT 1 FROM public.rehearsal_discard_receipt WHERE tenant_id=NEW.tenant_id AND operation_id=NEW.operation_id) THEN RAISE EXCEPTION 'prepared rehearsal discard must receive a receipt before commit' USING ERRCODE='23514'; END IF; RETURN NULL; END $$;
CREATE FUNCTION public.ple_rehearsal_require_delivery_claim() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
BEGIN IF NOT EXISTS(SELECT 1 FROM public.rehearsal_delivery_operation_root WHERE tenant_id=NEW.tenant_id AND rehearsal_run_id=NEW.rehearsal_run_id AND idempotency_key=NEW.idempotency_key) THEN RAISE EXCEPTION 'prepared rehearsal delivery admission must be claimed before commit' USING ERRCODE='23514'; END IF; RETURN NULL; END $$;
CREATE TRIGGER rehearsal_start_root_append_only BEFORE UPDATE OR DELETE ON public.rehearsal_start_operation_root FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_operation_guard();
CREATE TRIGGER rehearsal_start_receipt_append_only BEFORE UPDATE OR DELETE ON public.rehearsal_start_receipt FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_operation_guard();
CREATE TRIGGER rehearsal_discard_root_append_only BEFORE UPDATE OR DELETE ON public.rehearsal_discard_operation_root FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_operation_guard();
CREATE TRIGGER rehearsal_discard_receipt_append_only BEFORE UPDATE OR DELETE ON public.rehearsal_discard_receipt FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_operation_guard();
CREATE TRIGGER rehearsal_delivery_root_append_only BEFORE UPDATE OR DELETE ON public.rehearsal_delivery_operation_root FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_operation_guard();
CREATE TRIGGER rehearsal_delivery_admission_append_only BEFORE UPDATE OR DELETE ON public.rehearsal_delivery_admission FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_operation_guard();
CREATE TRIGGER rehearsal_delivery_event_append_only BEFORE UPDATE OR DELETE ON public.rehearsal_delivery_operation_event FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_operation_guard();
CREATE TRIGGER rehearsal_delivery_receipt_append_only BEFORE UPDATE OR DELETE ON public.rehearsal_delivery_receipt FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_operation_guard();
CREATE CONSTRAINT TRIGGER rehearsal_start_requires_receipt AFTER INSERT ON public.rehearsal_start_operation_root DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_require_start_receipt();
CREATE CONSTRAINT TRIGGER rehearsal_discard_requires_receipt AFTER INSERT ON public.rehearsal_discard_operation_root DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_require_discard_receipt();
CREATE CONSTRAINT TRIGGER rehearsal_delivery_admission_requires_claim AFTER INSERT ON public.rehearsal_delivery_admission DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION public.ple_rehearsal_require_delivery_claim();

CREATE FUNCTION public.ple_prepare_rehearsal_start_idempotent(p_tenant uuid,p_actor uuid,p_course uuid,p_assignment uuid,p_assignment_reference integer,p_revision bigint,p_subject jsonb,p_subject_fingerprint bytea,p_genesis_digest bytea,p_run uuid,p_start_new_after_completion boolean,p_key text,p_fingerprint bytea)
RETURNS TABLE(result_kind text,operation_id uuid,prepare_nonce uuid,structural_witness jsonb,structural_witness_digest bytea,response_projection jsonb,response_digest bytea) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE p record;e public.rehearsal_start_operation_root%ROWTYPE;r public.rehearsal_run%ROWTYPE;prior public.rehearsal_run%ROWTYPE;ref bigint;w jsonb;d bytea;op uuid:=gen_random_uuid();n uuid:=gen_random_uuid();
BEGIN
 IF p_key IS NULL OR char_length(p_key) NOT BETWEEN 1 AND 128 OR p_key~'[[:cntrl:]]' OR octet_length(p_fingerprint)<>32 THEN RETURN; END IF;
 SELECT * INTO p FROM public.ple_prepare_rehearsal_start(p_tenant,p_actor,p_course,p_assignment_reference,p_revision,NULL); IF NOT FOUND OR p.assignment_id<>p_assignment THEN RETURN; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended(p_tenant::text||':'||p_course::text||':'||p_assignment::text||':'||p.direct_instructor_membership_id::text,0));
 SELECT * INTO e FROM public.rehearsal_start_operation_root WHERE tenant_id=p_tenant AND course_id=p_course AND assignment_id=p_assignment AND direct_instructor_membership_id=p.direct_instructor_membership_id AND actor_id=p_actor AND idempotency_key=p_key;
 IF FOUND THEN IF e.request_fingerprint<>p_fingerprint THEN result_kind:='conflict';RETURN NEXT;RETURN; END IF; SELECT receipt.response_projection,receipt.response_digest INTO response_projection,response_digest FROM public.rehearsal_start_receipt receipt WHERE receipt.tenant_id=p_tenant AND receipt.operation_id=e.operation_id; result_kind:='replay';RETURN NEXT;RETURN; END IF;
 SELECT * INTO prior FROM public.rehearsal_run WHERE tenant_id=p_tenant AND course_id=p_course AND assignment_id=p_assignment AND direct_instructor_membership_id=p.direct_instructor_membership_id ORDER BY rehearsal_reference DESC LIMIT 1 FOR UPDATE;
 ref:=public.ple_rehearsal_start(p_tenant,p_actor,p_course,p_assignment,p_assignment_reference,p_revision,p_subject,p_subject_fingerprint,p_genesis_digest,p_run,p_start_new_after_completion,prior.rehearsal_run_id); IF ref IS NULL THEN RETURN; END IF;
 SELECT * INTO r FROM public.rehearsal_run WHERE tenant_id=p_tenant AND rehearsal_reference=ref FOR UPDATE;
 w:=jsonb_build_object('rehearsalReference',r.rehearsal_reference,'assignmentReference',p_assignment_reference,'revision',p_revision,'lifecycle',r.lifecycle,'startedAtMillis',floor(extract(epoch FROM r.started_at)*1000)::bigint,'updatedAtMillis',floor(extract(epoch FROM r.updated_at)*1000)::bigint,'disposition',CASE WHEN prior.rehearsal_run_id IS NULL THEN 'started' WHEN prior.lifecycle='active' AND prior.subject_fingerprint=p_subject_fingerprint AND prior.actor_id=p_actor THEN 'resumed' WHEN prior.lifecycle='completed' THEN 'restartedAfterCompletion' ELSE 'replacedDifferentSubject' END);
 d:=digest(convert_to(w::text,'utf8'),'sha256'); INSERT INTO public.rehearsal_start_operation_root VALUES(p_tenant,p_course,p_assignment,p.direct_instructor_membership_id,p_actor,p_key,p_fingerprint,op,r.rehearsal_run_id,w->>'disposition',w,d,n,txid_current(),DEFAULT);
 result_kind:='apply';operation_id:=op;prepare_nonce:=n;structural_witness:=w;structural_witness_digest:=d;RETURN NEXT;
END $$;
CREATE FUNCTION public.ple_complete_rehearsal_start_idempotent(p_tenant uuid,p_operation uuid,p_nonce uuid,p_witness_digest bytea,p_response jsonb,p_response_digest bytea)
RETURNS TABLE(response_projection jsonb,response_digest bytea) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE r public.rehearsal_start_operation_root%ROWTYPE;
-- Operation roots, generations, and events are append-only. Their prepare
-- brokers already hold the mutable rehearsal-run lock for the transaction,
-- so later phases use plain reads and keep UPDATE authority unavailable.
BEGIN SELECT * INTO r FROM public.rehearsal_start_operation_root WHERE tenant_id=p_tenant AND operation_id=p_operation; IF NOT FOUND OR r.prepare_nonce<>p_nonce OR r.prepared_txid<>txid_current() OR r.structural_witness_digest<>p_witness_digest OR jsonb_typeof(p_response)<>'object' OR public.ple_rehearsal_jsonb_bytes(p_response)>65536 OR octet_length(p_response_digest)<>32 THEN RETURN; END IF; INSERT INTO public.rehearsal_start_receipt VALUES(p_tenant,p_operation,p_response,p_response_digest,DEFAULT); RETURN QUERY SELECT p_response,p_response_digest; END $$;
CREATE FUNCTION public.ple_prepare_rehearsal_discard_idempotent(p_tenant uuid,p_actor uuid,p_course uuid,p_assignment_reference integer,p_revision bigint,p_rehearsal_reference bigint,p_key text,p_fingerprint bytea)
RETURNS TABLE(result_kind text,operation_id uuid,prepare_nonce uuid,structural_witness jsonb,structural_witness_digest bytea,response_projection jsonb,response_digest bytea) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE w record;e public.rehearsal_discard_operation_root%ROWTYPE;j jsonb;d bytea;op uuid:=gen_random_uuid();n uuid:=gen_random_uuid();
BEGIN IF p_key IS NULL OR char_length(p_key) NOT BETWEEN 1 AND 128 OR p_key~'[[:cntrl:]]' OR octet_length(p_fingerprint)<>32 THEN RETURN; END IF; SELECT * INTO w FROM public.ple_prepare_rehearsal_operation(p_tenant,p_actor,p_course,p_assignment_reference,p_revision,p_rehearsal_reference); IF NOT FOUND THEN RETURN; END IF; SELECT * INTO e FROM public.rehearsal_discard_operation_root WHERE tenant_id=p_tenant AND rehearsal_run_id=w.rehearsal_run_id AND idempotency_key=p_key; IF FOUND THEN IF e.request_fingerprint<>p_fingerprint THEN result_kind:='conflict';RETURN NEXT;RETURN; END IF; SELECT receipt.response_projection,receipt.response_digest INTO response_projection,response_digest FROM public.rehearsal_discard_receipt receipt WHERE receipt.tenant_id=p_tenant AND receipt.operation_id=e.operation_id; result_kind:='replay';RETURN NEXT;RETURN; END IF; IF NOT public.ple_rehearsal_terminalize(p_tenant,p_actor,p_course,w.assignment_id,p_revision,w.rehearsal_run_id,'discardedByInstructor') THEN RETURN; END IF; j:=jsonb_build_object('rehearsalReference',p_rehearsal_reference,'assignmentReference',p_assignment_reference,'revision',p_revision,'lifecycle','discarded','reason','discardedByInstructor');d:=digest(convert_to(j::text,'utf8'),'sha256');INSERT INTO public.rehearsal_discard_operation_root VALUES(p_tenant,w.rehearsal_run_id,p_key,p_fingerprint,op,j,d,n,txid_current(),DEFAULT);result_kind:='apply';operation_id:=op;prepare_nonce:=n;structural_witness:=j;structural_witness_digest:=d;RETURN NEXT;END $$;
CREATE FUNCTION public.ple_complete_rehearsal_discard_idempotent(p_tenant uuid,p_operation uuid,p_nonce uuid,p_witness_digest bytea,p_response jsonb,p_response_digest bytea) RETURNS TABLE(response_projection jsonb,response_digest bytea) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$ DECLARE r public.rehearsal_discard_operation_root%ROWTYPE; BEGIN SELECT * INTO r FROM public.rehearsal_discard_operation_root WHERE tenant_id=p_tenant AND operation_id=p_operation; IF NOT FOUND OR r.prepare_nonce<>p_nonce OR r.prepared_txid<>txid_current() OR r.structural_witness_digest<>p_witness_digest OR jsonb_typeof(p_response)<>'object' OR public.ple_rehearsal_jsonb_bytes(p_response)>65536 OR octet_length(p_response_digest)<>32 THEN RETURN; END IF; INSERT INTO public.rehearsal_discard_receipt VALUES(p_tenant,p_operation,p_response,p_response_digest,DEFAULT);RETURN QUERY SELECT p_response,p_response_digest;END $$;

CREATE OR REPLACE FUNCTION public.ple_prepare_rehearsal_delivery(p_tenant uuid,p_actor uuid,p_course uuid,p_assignment_reference integer,p_revision bigint,p_rehearsal_reference bigint,p_key text,p_fingerprint bytea)
RETURNS TABLE(result_kind text,operation_id uuid,prepare_nonce uuid,admission_witness jsonb,admission_digest bytea,screen_projection jsonb,screen_digest bytea,phase text,execution_descriptor jsonb)
LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE w record; root public.rehearsal_delivery_operation_root%ROWTYPE; latest public.rehearsal_delivery_operation_generation%ROWTYPE; last_event record; receipt public.rehearsal_delivery_receipt%ROWTYPE; a public.rehearsal_delivery_admission%ROWTYPE; j jsonb; d bytea; n uuid:=gen_random_uuid(); replacement uuid;
BEGIN
 IF p_key IS NULL OR char_length(p_key) NOT BETWEEN 1 AND 128 OR p_key~'[[:cntrl:]]' OR octet_length(p_fingerprint)<>32 THEN RETURN; END IF;
 SELECT * INTO w FROM public.ple_prepare_rehearsal_operation(p_tenant,p_actor,p_course,p_assignment_reference,p_revision,p_rehearsal_reference); IF NOT FOUND THEN RETURN; END IF;
 SELECT * INTO root FROM public.rehearsal_delivery_operation_root WHERE tenant_id=p_tenant AND rehearsal_run_id=w.rehearsal_run_id AND idempotency_key=p_key;
 IF FOUND THEN
   IF root.request_fingerprint<>p_fingerprint THEN result_kind:='conflict'; RETURN NEXT; RETURN; END IF;
   SELECT * INTO latest FROM public.rehearsal_delivery_operation_generation WHERE tenant_id=p_tenant AND rehearsal_run_id=w.rehearsal_run_id AND root_id=root.root_id ORDER BY generation DESC LIMIT 1;
   IF NOT FOUND THEN RETURN; END IF;
   SELECT * INTO receipt FROM public.rehearsal_delivery_receipt WHERE tenant_id=p_tenant AND rehearsal_run_id=w.rehearsal_run_id AND root_id=root.root_id AND generation=latest.generation;
   IF FOUND THEN result_kind:='replay';screen_projection:=receipt.screen_projection;screen_digest:=receipt.screen_digest;RETURN NEXT;RETURN;END IF;
   SELECT * INTO last_event FROM public.rehearsal_delivery_operation_event WHERE tenant_id=p_tenant AND rehearsal_run_id=w.rehearsal_run_id AND root_id=root.root_id AND generation=latest.generation ORDER BY sequence DESC LIMIT 1;
   IF last_event.phase='issueDispatched' THEN result_kind:='pending';operation_id:=latest.operation_id;execution_descriptor:=latest.execution_descriptor;RETURN NEXT;RETURN;END IF;
   IF last_event.phase='prepared' THEN result_kind:='claimed';operation_id:=latest.operation_id;execution_descriptor:=latest.execution_descriptor;RETURN NEXT;RETURN;END IF;
   IF last_event.phase='abandonedBeforeDispatch' AND latest.generation<2147483647 THEN
     replacement:=gen_random_uuid();
     INSERT INTO public.rehearsal_delivery_operation_generation(tenant_id,rehearsal_run_id,root_id,generation,operation_id,selected_attempt_id,execution_descriptor,descriptor_digest,structural_binding)
       VALUES(p_tenant,w.rehearsal_run_id,root.root_id,latest.generation+1,replacement,latest.selected_attempt_id,latest.execution_descriptor,latest.descriptor_digest,latest.structural_binding);
     INSERT INTO public.rehearsal_delivery_operation_event(tenant_id,rehearsal_run_id,root_id,generation,operation_id,sequence,phase) VALUES(p_tenant,w.rehearsal_run_id,root.root_id,latest.generation+1,replacement,1,'prepared');
     result_kind:='claimed';operation_id:=replacement;execution_descriptor:=latest.execution_descriptor;RETURN NEXT;RETURN;
   END IF;
   RETURN;
 END IF;
 IF NOT EXISTS(SELECT 1 FROM public.rehearsal_run WHERE tenant_id=p_tenant AND rehearsal_run_id=w.rehearsal_run_id AND lifecycle='active') THEN RETURN; END IF;
 j:=jsonb_build_object('rehearsalRunId',w.rehearsal_run_id,'rehearsalReference',p_rehearsal_reference,'assignmentReference',p_assignment_reference,'revision',p_revision);d:=digest(convert_to(j::text,'utf8'),'sha256');
 INSERT INTO public.rehearsal_delivery_admission VALUES(p_tenant,w.rehearsal_run_id,p_key,p_fingerprint,n,j,d,txid_current(),DEFAULT);
 result_kind:='admit';prepare_nonce:=n;admission_witness:=j;admission_digest:=d;RETURN NEXT;
END $$;

CREATE OR REPLACE FUNCTION public.ple_rehearsal_mark_delivery_dispatched(p_tenant uuid,p_actor uuid,p_course uuid,p_assignment_reference integer,p_revision bigint,p_rehearsal_reference bigint,p_operation uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE w record; generation_row public.rehearsal_delivery_operation_generation%ROWTYPE; latest record;
BEGIN
 SELECT * INTO w FROM public.ple_prepare_rehearsal_operation(p_tenant,p_actor,p_course,p_assignment_reference,p_revision,p_rehearsal_reference); IF NOT FOUND OR p_operation IS NULL THEN RETURN false; END IF;
 SELECT * INTO generation_row FROM public.rehearsal_delivery_operation_generation WHERE tenant_id=p_tenant AND rehearsal_run_id=w.rehearsal_run_id AND operation_id=p_operation; IF NOT FOUND THEN RETURN false; END IF;
 SELECT * INTO latest FROM public.rehearsal_delivery_operation_event WHERE tenant_id=p_tenant AND rehearsal_run_id=w.rehearsal_run_id AND root_id=generation_row.root_id AND generation=generation_row.generation ORDER BY sequence DESC LIMIT 1;
 IF NOT FOUND OR latest.phase<>'prepared' THEN RETURN false; END IF;
 INSERT INTO public.rehearsal_delivery_operation_event(tenant_id,rehearsal_run_id,root_id,generation,operation_id,sequence,phase) VALUES(p_tenant,w.rehearsal_run_id,generation_row.root_id,generation_row.generation,p_operation,latest.sequence+1,'issueDispatched');
 RETURN true;
END $$;

CREATE OR REPLACE FUNCTION public.ple_rehearsal_abandon_delivery_before_dispatch(p_tenant uuid,p_actor uuid,p_course uuid,p_assignment_reference integer,p_revision bigint,p_rehearsal_reference bigint,p_operation uuid,p_reason text)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE w record; generation_row public.rehearsal_delivery_operation_generation%ROWTYPE; latest record;
BEGIN
 SELECT * INTO w FROM public.ple_prepare_rehearsal_operation(p_tenant,p_actor,p_course,p_assignment_reference,p_revision,p_rehearsal_reference); IF NOT FOUND OR p_operation IS NULL OR p_reason NOT IN('localPreparationFailed','nativeBackendAdmissionRejected','trustedRendererAdmissionRejected') THEN RETURN false; END IF;
 SELECT * INTO generation_row FROM public.rehearsal_delivery_operation_generation WHERE tenant_id=p_tenant AND rehearsal_run_id=w.rehearsal_run_id AND operation_id=p_operation; IF NOT FOUND THEN RETURN false; END IF;
 SELECT * INTO latest FROM public.rehearsal_delivery_operation_event WHERE tenant_id=p_tenant AND rehearsal_run_id=w.rehearsal_run_id AND root_id=generation_row.root_id AND generation=generation_row.generation ORDER BY sequence DESC LIMIT 1;
 IF NOT FOUND OR latest.phase<>'prepared' THEN RETURN false; END IF;
 INSERT INTO public.rehearsal_delivery_operation_event(tenant_id,rehearsal_run_id,root_id,generation,operation_id,sequence,phase,abandonment_reason) VALUES(p_tenant,w.rehearsal_run_id,generation_row.root_id,generation_row.generation,p_operation,latest.sequence+1,'abandonedBeforeDispatch',p_reason);
 RETURN true;
END $$;

CREATE OR REPLACE FUNCTION public.ple_rehearsal_complete_delivery(p_tenant uuid,p_actor uuid,p_course uuid,p_assignment_reference integer,p_revision bigint,p_rehearsal_reference bigint,p_operation uuid,p_kind text,p_frozen_attempt uuid,p_screen jsonb,p_screen_digest bytea)
RETURNS TABLE(screen_projection jsonb,screen_digest bytea) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE w record; generation_row public.rehearsal_delivery_operation_generation%ROWTYPE; latest record;
BEGIN
 SELECT * INTO w FROM public.ple_prepare_rehearsal_operation(p_tenant,p_actor,p_course,p_assignment_reference,p_revision,p_rehearsal_reference);
 IF NOT FOUND OR p_operation IS NULL OR p_kind NOT IN('issued','completed') OR jsonb_typeof(p_screen)<>'object' OR public.ple_rehearsal_jsonb_bytes(p_screen)>262144 OR octet_length(p_screen_digest)<>32 OR (p_kind='issued' AND p_frozen_attempt IS NULL) OR (p_kind='completed' AND p_frozen_attempt IS NOT NULL) THEN RETURN; END IF;
 SELECT * INTO generation_row FROM public.rehearsal_delivery_operation_generation WHERE tenant_id=p_tenant AND rehearsal_run_id=w.rehearsal_run_id AND operation_id=p_operation; IF NOT FOUND THEN RETURN; END IF;
 SELECT * INTO latest FROM public.rehearsal_delivery_operation_event WHERE tenant_id=p_tenant AND rehearsal_run_id=w.rehearsal_run_id AND root_id=generation_row.root_id AND generation=generation_row.generation ORDER BY sequence DESC LIMIT 1;
 IF NOT FOUND OR latest.phase<>'issueDispatched' THEN RETURN; END IF;
 IF p_kind='issued' AND p_frozen_attempt<>generation_row.selected_attempt_id THEN RETURN; END IF;
 INSERT INTO public.rehearsal_delivery_receipt(tenant_id,rehearsal_run_id,root_id,generation,operation_id,result_kind,frozen_attempt_id,screen_projection,screen_digest) VALUES(p_tenant,w.rehearsal_run_id,generation_row.root_id,generation_row.generation,p_operation,p_kind,p_frozen_attempt,p_screen,p_screen_digest);
 INSERT INTO public.rehearsal_delivery_operation_event(tenant_id,rehearsal_run_id,root_id,generation,operation_id,sequence,phase,completion_kind,frozen_attempt_id,screen_digest) VALUES(p_tenant,w.rehearsal_run_id,generation_row.root_id,generation_row.generation,p_operation,latest.sequence+1,'completed',p_kind,p_frozen_attempt,p_screen_digest);
 RETURN QUERY SELECT p_screen,p_screen_digest;
END $$;

-- A revision/source/lifecycle fence terminalizes only the latest open
-- generation.  Earlier generations remain immutable audit evidence.
CREATE OR REPLACE FUNCTION public.ple_revoke_open_rehearsal_delivery_operations()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE reason text;
BEGIN
 IF OLD.lifecycle='active' AND NEW.lifecycle<>'active' THEN
   reason:=CASE NEW.lifecycle WHEN 'discardedStaleRevision' THEN 'revokedStaleRevision' WHEN 'discardedSourceContextRemoved' THEN 'revokedSourceContextRemoved' ELSE 'revokedTerminalLifecycle' END;
   INSERT INTO public.rehearsal_delivery_operation_event(tenant_id,rehearsal_run_id,root_id,generation,operation_id,sequence,phase)
   SELECT generation_row.tenant_id,generation_row.rehearsal_run_id,generation_row.root_id,generation_row.generation,generation_row.operation_id,last_event.sequence+1,reason
     FROM public.rehearsal_delivery_operation_root root
     JOIN LATERAL (SELECT * FROM public.rehearsal_delivery_operation_generation generation_row WHERE generation_row.tenant_id=root.tenant_id AND generation_row.rehearsal_run_id=root.rehearsal_run_id AND generation_row.root_id=root.root_id ORDER BY generation DESC LIMIT 1) generation_row ON true
     JOIN LATERAL (SELECT * FROM public.rehearsal_delivery_operation_event event_row WHERE event_row.tenant_id=generation_row.tenant_id AND event_row.rehearsal_run_id=generation_row.rehearsal_run_id AND event_row.root_id=generation_row.root_id AND event_row.generation=generation_row.generation ORDER BY sequence DESC LIMIT 1) last_event ON true
    WHERE root.tenant_id=NEW.tenant_id AND root.rehearsal_run_id=NEW.rehearsal_run_id AND last_event.phase IN ('prepared','issueDispatched');
 END IF;
 RETURN NEW;
END $$;

ALTER TABLE public.rehearsal_start_operation_root ENABLE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_start_operation_root FORCE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_start_receipt ENABLE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_start_receipt FORCE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_discard_operation_root ENABLE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_discard_operation_root FORCE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_discard_receipt ENABLE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_discard_receipt FORCE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_delivery_admission ENABLE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_delivery_admission FORCE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_delivery_operation_root ENABLE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_delivery_operation_root FORCE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_delivery_operation_generation ENABLE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_delivery_operation_generation FORCE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_delivery_operation_event ENABLE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_delivery_operation_event FORCE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_delivery_receipt ENABLE ROW LEVEL SECURITY; ALTER TABLE public.rehearsal_delivery_receipt FORCE ROW LEVEL SECURITY;
CREATE POLICY rehearsal_start_root_broker ON public.rehearsal_start_operation_root TO ple_rehearsal_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY rehearsal_start_receipt_broker ON public.rehearsal_start_receipt TO ple_rehearsal_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY rehearsal_discard_root_broker ON public.rehearsal_discard_operation_root TO ple_rehearsal_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY rehearsal_discard_receipt_broker ON public.rehearsal_discard_receipt TO ple_rehearsal_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY rehearsal_delivery_admission_broker ON public.rehearsal_delivery_admission TO ple_rehearsal_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY rehearsal_delivery_root_broker ON public.rehearsal_delivery_operation_root TO ple_rehearsal_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY rehearsal_delivery_generation_broker ON public.rehearsal_delivery_operation_generation TO ple_rehearsal_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY rehearsal_delivery_event_broker ON public.rehearsal_delivery_operation_event TO ple_rehearsal_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY rehearsal_delivery_receipt_broker ON public.rehearsal_delivery_receipt TO ple_rehearsal_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant());
GRANT SELECT,INSERT ON public.rehearsal_start_operation_root,public.rehearsal_start_receipt,public.rehearsal_discard_operation_root,public.rehearsal_discard_receipt,public.rehearsal_delivery_admission,public.rehearsal_delivery_operation_root,public.rehearsal_delivery_operation_generation,public.rehearsal_delivery_operation_event,public.rehearsal_delivery_receipt TO ple_rehearsal_broker;
REVOKE ALL ON public.rehearsal_start_operation_root,public.rehearsal_start_receipt,public.rehearsal_discard_operation_root,public.rehearsal_discard_receipt,public.rehearsal_delivery_admission,public.rehearsal_delivery_operation_root,public.rehearsal_delivery_operation_generation,public.rehearsal_delivery_operation_event,public.rehearsal_delivery_receipt FROM ple_app;
ALTER FUNCTION public.ple_prepare_rehearsal_start_idempotent(uuid,uuid,uuid,uuid,integer,bigint,jsonb,bytea,bytea,uuid,boolean,text,bytea) OWNER TO ple_rehearsal_broker; ALTER FUNCTION public.ple_complete_rehearsal_start_idempotent(uuid,uuid,uuid,bytea,jsonb,bytea) OWNER TO ple_rehearsal_broker; ALTER FUNCTION public.ple_prepare_rehearsal_discard_idempotent(uuid,uuid,uuid,integer,bigint,bigint,text,bytea) OWNER TO ple_rehearsal_broker; ALTER FUNCTION public.ple_complete_rehearsal_discard_idempotent(uuid,uuid,uuid,bytea,jsonb,bytea) OWNER TO ple_rehearsal_broker; ALTER FUNCTION public.ple_prepare_rehearsal_delivery(uuid,uuid,uuid,integer,bigint,bigint,text,bytea) OWNER TO ple_rehearsal_broker; ALTER FUNCTION public.ple_rehearsal_mark_delivery_dispatched(uuid,uuid,uuid,integer,bigint,bigint,uuid) OWNER TO ple_rehearsal_broker; ALTER FUNCTION public.ple_rehearsal_abandon_delivery_before_dispatch(uuid,uuid,uuid,integer,bigint,bigint,uuid,text) OWNER TO ple_rehearsal_broker; ALTER FUNCTION public.ple_rehearsal_complete_delivery(uuid,uuid,uuid,integer,bigint,bigint,uuid,text,uuid,jsonb,bytea) OWNER TO ple_rehearsal_broker;
REVOKE ALL ON FUNCTION public.ple_prepare_rehearsal_start_idempotent(uuid,uuid,uuid,uuid,integer,bigint,jsonb,bytea,bytea,uuid,boolean,text,bytea),public.ple_complete_rehearsal_start_idempotent(uuid,uuid,uuid,bytea,jsonb,bytea),public.ple_prepare_rehearsal_discard_idempotent(uuid,uuid,uuid,integer,bigint,bigint,text,bytea),public.ple_complete_rehearsal_discard_idempotent(uuid,uuid,uuid,bytea,jsonb,bytea),public.ple_prepare_rehearsal_delivery(uuid,uuid,uuid,integer,bigint,bigint,text,bytea),public.ple_rehearsal_complete_delivery(uuid,uuid,uuid,integer,bigint,bigint,uuid,text,uuid,jsonb,bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_prepare_rehearsal_start_idempotent(uuid,uuid,uuid,uuid,integer,bigint,jsonb,bytea,bytea,uuid,boolean,text,bytea),public.ple_complete_rehearsal_start_idempotent(uuid,uuid,uuid,bytea,jsonb,bytea),public.ple_prepare_rehearsal_discard_idempotent(uuid,uuid,uuid,integer,bigint,bigint,text,bytea),public.ple_complete_rehearsal_discard_idempotent(uuid,uuid,uuid,bytea,jsonb,bytea),public.ple_prepare_rehearsal_delivery(uuid,uuid,uuid,integer,bigint,bigint,text,bytea),public.ple_rehearsal_mark_delivery_dispatched(uuid,uuid,uuid,integer,bigint,bigint,uuid),public.ple_rehearsal_abandon_delivery_before_dispatch(uuid,uuid,uuid,integer,bigint,bigint,uuid,text),public.ple_rehearsal_complete_delivery(uuid,uuid,uuid,integer,bigint,bigint,uuid,text,uuid,jsonb,bytea) TO ple_app;
REVOKE EXECUTE ON FUNCTION public.ple_rehearsal_start(uuid,uuid,uuid,uuid,integer,bigint,jsonb,bytea,bytea,uuid,boolean,uuid) FROM ple_app;

-- ASVS 8.2.1, 8.4.1: separate the all-tenant sealed rehearsal witness from
-- the Base Course broker.  The registry is intentionally closed: later
-- migrations must register every new public relation or freshness refuses.
DO $$
BEGIN
 IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname='ple_rehearsal_freshness_witness') THEN
  CREATE ROLE ple_rehearsal_freshness_witness NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
 END IF;
 IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname='ple_base_course_freshness_registry_owner') THEN
  CREATE ROLE ple_base_course_freshness_registry_owner NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
 END IF;
 IF EXISTS (SELECT 1 FROM pg_catalog.pg_auth_members
            WHERE roleid='ple_rehearsal_freshness_witness'::regrole
               OR member='ple_rehearsal_freshness_witness'::regrole) THEN
  RAISE EXCEPTION 'rehearsal freshness witness must have no memberships' USING ERRCODE='55000';
 END IF;
 IF EXISTS (SELECT 1 FROM pg_catalog.pg_auth_members WHERE roleid='ple_base_course_freshness_registry_owner'::regrole OR member='ple_base_course_freshness_registry_owner'::regrole) THEN
  RAISE EXCEPTION 'freshness registry owner must have no memberships' USING ERRCODE='55000';
 END IF;
END $$;
ALTER ROLE ple_rehearsal_freshness_witness NOLOGIN NOSUPERUSER NOCREATEDB
    NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
ALTER ROLE ple_base_course_freshness_registry_owner NOLOGIN NOSUPERUSER NOCREATEDB
    NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
REVOKE ALL ON SCHEMA public FROM ple_rehearsal_freshness_witness;
REVOKE ALL ON SCHEMA public FROM ple_base_course_freshness_registry_owner;
GRANT USAGE ON SCHEMA public TO ple_rehearsal_freshness_witness;
GRANT USAGE ON SCHEMA public TO ple_base_course_freshness_registry_owner;

CREATE TABLE public.ple_base_course_freshness_domain(
 domain text PRIMARY KEY,
 inspection_role name NOT NULL,
 verifier_name text NOT NULL,
 CHECK(domain IN('raw','sealed_rehearsal'))
);
INSERT INTO public.ple_base_course_freshness_domain(domain,inspection_role,verifier_name) VALUES
 ('raw','ple_base_course_freshness_broker','direct_raw_relation_empty'),
 ('sealed_rehearsal','ple_rehearsal_freshness_witness','public.ple_verify_sealed_rehearsal_freshness_empty()');
CREATE TABLE public.ple_base_course_freshness_relation(
 relation_oid oid PRIMARY KEY,
 domain text NOT NULL REFERENCES public.ple_base_course_freshness_domain(domain) ON DELETE RESTRICT,
 UNIQUE(relation_oid,domain)
);
INSERT INTO public.ple_base_course_freshness_relation(relation_oid,domain)
SELECT table_row.oid,'raw'
  FROM pg_catalog.pg_class AS table_row
  JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=table_row.relnamespace
 WHERE namespace.nspname='public' AND table_row.relkind IN('r','p')
   AND table_row.relname NOT IN('_sqlx_migrations','question_id_namespace','ple_base_course_freshness_relation','ple_base_course_freshness_domain',
    'rehearsal_run','rehearsal_frozen_item','rehearsal_evidence','rehearsal_submission_claim_root','rehearsal_submission_claim_event','rehearsal_submission_receipt',
    'rehearsal_start_operation_root','rehearsal_start_receipt','rehearsal_discard_operation_root','rehearsal_discard_receipt','rehearsal_delivery_admission',
    'rehearsal_delivery_operation_root','rehearsal_delivery_operation_generation','rehearsal_delivery_operation_event','rehearsal_delivery_receipt');
INSERT INTO public.ple_base_course_freshness_relation(relation_oid,domain) VALUES
 ('public.rehearsal_run'::regclass,'sealed_rehearsal'),('public.rehearsal_frozen_item'::regclass,'sealed_rehearsal'),('public.rehearsal_evidence'::regclass,'sealed_rehearsal'),
 ('public.rehearsal_submission_claim_root'::regclass,'sealed_rehearsal'),('public.rehearsal_submission_claim_event'::regclass,'sealed_rehearsal'),('public.rehearsal_submission_receipt'::regclass,'sealed_rehearsal'),
 ('public.rehearsal_start_operation_root'::regclass,'sealed_rehearsal'),('public.rehearsal_start_receipt'::regclass,'sealed_rehearsal'),('public.rehearsal_discard_operation_root'::regclass,'sealed_rehearsal'),('public.rehearsal_discard_receipt'::regclass,'sealed_rehearsal'),
 ('public.rehearsal_delivery_admission'::regclass,'sealed_rehearsal'),('public.rehearsal_delivery_operation_root'::regclass,'sealed_rehearsal'),('public.rehearsal_delivery_operation_generation'::regclass,'sealed_rehearsal'),('public.rehearsal_delivery_operation_event'::regclass,'sealed_rehearsal'),('public.rehearsal_delivery_receipt'::regclass,'sealed_rehearsal');

-- The witness first SHARE-locks every sealed relation and only then reads one.
-- This keeps the empty-domain answer coherent with the raw-domain locks held
-- by the caller (ASVS 2.3.1, 2.3.3).
CREATE FUNCTION public.ple_verify_sealed_rehearsal_freshness_empty()
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE relation_row record; relation_has_rows boolean;
BEGIN
 LOCK TABLE ONLY public.ple_base_course_freshness_domain IN SHARE MODE;
 LOCK TABLE ONLY public.ple_base_course_freshness_relation IN SHARE MODE;
 FOR relation_row IN
  SELECT table_row.oid,namespace.nspname,table_row.relname
    FROM public.ple_base_course_freshness_relation registry
    JOIN pg_catalog.pg_class table_row ON table_row.oid=registry.relation_oid
    JOIN pg_catalog.pg_namespace namespace ON namespace.oid=table_row.relnamespace
   WHERE registry.domain='sealed_rehearsal'
   ORDER BY namespace.nspname,table_row.relname,table_row.oid
 LOOP
  EXECUTE format('LOCK TABLE ONLY %I.%I IN SHARE MODE',relation_row.nspname,relation_row.relname);
 END LOOP;
 FOR relation_row IN
  SELECT table_row.oid,namespace.nspname,table_row.relname
    FROM public.ple_base_course_freshness_relation registry
    JOIN pg_catalog.pg_class table_row ON table_row.oid=registry.relation_oid
    JOIN pg_catalog.pg_namespace namespace ON namespace.oid=table_row.relnamespace
   WHERE registry.domain='sealed_rehearsal'
   ORDER BY namespace.nspname,table_row.relname,table_row.oid
 LOOP
  EXECUTE format('SELECT EXISTS(SELECT 1 FROM ONLY %I.%I LIMIT 1)',relation_row.nspname,relation_row.relname)
    INTO relation_has_rows;
  IF relation_has_rows THEN RETURN false; END IF;
 END LOOP;
 RETURN true;
END $$;
CREATE OR REPLACE FUNCTION public.ple_require_fresh_base_course_install_internal()
RETURNS TABLE(failure_kind text,relation_name text) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE relation_row record; namespace_rows bigint; unconsumed_namespace_rows bigint;
relation_has_rows boolean;
BEGIN
 -- The catalog/registry/authority matrix is checked before reading domain data.
 LOCK TABLE ONLY public.ple_base_course_freshness_domain IN SHARE MODE;
 LOCK TABLE ONLY public.ple_base_course_freshness_relation IN SHARE MODE;
 IF (SELECT jsonb_agg(jsonb_build_object('domain',domain,'inspectionRole',inspection_role,'verifier',verifier_name) ORDER BY domain)
       FROM public.ple_base_course_freshness_domain) IS DISTINCT FROM
    '[{"domain":"raw","inspectionRole":"ple_base_course_freshness_broker","verifier":"direct_raw_relation_empty"},{"domain":"sealed_rehearsal","inspectionRole":"ple_rehearsal_freshness_witness","verifier":"public.ple_verify_sealed_rehearsal_freshness_empty()"}]'::jsonb THEN
  RAISE EXCEPTION 'Base Course freshness domain registry is inconsistent' USING ERRCODE='55000';
 END IF;
 IF NOT EXISTS(
  SELECT 1 FROM pg_catalog.pg_proc procedure_row
   WHERE procedure_row.oid=to_regprocedure('public.ple_verify_sealed_rehearsal_freshness_empty()')
     AND procedure_row.prorettype='boolean'::regtype AND procedure_row.proargtypes=''::oidvector
     AND procedure_row.prosecdef AND procedure_row.proowner='ple_rehearsal_freshness_witness'::regrole
     AND procedure_row.proconfig=ARRAY['search_path=pg_catalog, public, pg_temp']
 ) OR EXISTS(
  SELECT 1 FROM pg_catalog.pg_proc procedure_row CROSS JOIN LATERAL aclexplode(COALESCE(procedure_row.proacl,acldefault('f',procedure_row.proowner))) privilege
   WHERE procedure_row.oid=to_regprocedure('public.ple_verify_sealed_rehearsal_freshness_empty()')
     AND privilege.grantee<>procedure_row.proowner
     AND (privilege.grantee<>'ple_base_course_freshness_broker'::regrole OR privilege.privilege_type<>'EXECUTE' OR privilege.is_grantable)
 ) OR (SELECT count(*) FROM pg_catalog.pg_proc procedure_row CROSS JOIN LATERAL aclexplode(COALESCE(procedure_row.proacl,acldefault('f',procedure_row.proowner))) privilege
       WHERE procedure_row.oid=to_regprocedure('public.ple_verify_sealed_rehearsal_freshness_empty()')
         AND privilege.grantee='ple_base_course_freshness_broker'::regrole AND privilege.privilege_type='EXECUTE' AND NOT privilege.is_grantable)<>1 THEN
  RAISE EXCEPTION 'Base Course sealed freshness witness authority is unsafe' USING ERRCODE='55000';
 END IF;
 IF EXISTS(
  WITH expected(relation_oid) AS (
   SELECT table_row.oid
     FROM pg_catalog.pg_class table_row
     JOIN pg_catalog.pg_namespace namespace ON namespace.oid=table_row.relnamespace
    WHERE namespace.nspname='public' AND table_row.relkind IN('r','p')
      AND table_row.relname NOT IN('_sqlx_migrations','question_id_namespace','ple_base_course_freshness_relation','ple_base_course_freshness_domain')
  ), actual AS (
   SELECT relation_oid FROM public.ple_base_course_freshness_relation
  ) SELECT 1 FROM ((SELECT * FROM expected EXCEPT SELECT * FROM actual)
                   UNION ALL (SELECT * FROM actual EXCEPT SELECT * FROM expected)) difference
 ) THEN RAISE EXCEPTION 'Base Course freshness registry is inconsistent' USING ERRCODE='55000'; END IF;
 IF EXISTS(
  WITH expected(relation_oid,grantee_name,privilege_type) AS (
   SELECT registry.relation_oid,'ple_base_course_freshness_broker'::name,privilege.privilege_type
     FROM public.ple_base_course_freshness_relation registry
     CROSS JOIN (VALUES('SELECT'),('MAINTAIN')) privilege(privilege_type) WHERE registry.domain='raw'
   UNION ALL
   SELECT registry.relation_oid,'ple_rehearsal_freshness_witness'::name,privilege.privilege_type
     FROM public.ple_base_course_freshness_relation registry
     CROSS JOIN (VALUES('SELECT'),('MAINTAIN')) privilege(privilege_type) WHERE registry.domain='sealed_rehearsal'
  ), actual AS (
   SELECT table_row.oid,role_row.rolname,privilege.privilege_type
     FROM pg_catalog.pg_class table_row CROSS JOIN LATERAL aclexplode(COALESCE(table_row.relacl,acldefault('r',table_row.relowner))) privilege
     JOIN pg_catalog.pg_roles role_row ON role_row.oid=privilege.grantee
    WHERE table_row.oid IN(SELECT relation_oid FROM public.ple_base_course_freshness_relation)
      AND role_row.rolname IN('ple_base_course_freshness_broker','ple_rehearsal_freshness_witness')
      AND privilege.grantee<>table_row.relowner
  ) SELECT 1 FROM ((SELECT * FROM expected EXCEPT SELECT * FROM actual)
                   UNION ALL (SELECT * FROM actual EXCEPT SELECT * FROM expected)) difference
 ) OR EXISTS(
  SELECT 1 FROM pg_catalog.pg_class table_row CROSS JOIN LATERAL aclexplode(COALESCE(table_row.relacl,acldefault('r',table_row.relowner))) privilege
   WHERE table_row.oid IN(SELECT relation_oid FROM public.ple_base_course_freshness_relation)
     AND privilege.grantee IN('ple_base_course_freshness_broker'::regrole,'ple_rehearsal_freshness_witness'::regrole)
     AND privilege.grantee<>table_row.relowner AND privilege.privilege_type NOT IN('SELECT','MAINTAIN')
 ) THEN RAISE EXCEPTION 'Base Course freshness relation privilege matrix is unsafe' USING ERRCODE='55000'; END IF;
 IF EXISTS(
  WITH expected(relation_name,role_name,privilege_type) AS (
   VALUES ('question_id_namespace','ple_base_course_freshness_broker','SELECT'),('question_id_namespace','ple_base_course_freshness_broker','MAINTAIN'),
          ('ple_base_course_freshness_domain','ple_base_course_freshness_broker','SELECT'),('ple_base_course_freshness_domain','ple_base_course_freshness_broker','MAINTAIN'),
          ('ple_base_course_freshness_relation','ple_base_course_freshness_broker','SELECT'),('ple_base_course_freshness_relation','ple_base_course_freshness_broker','MAINTAIN'),
          ('ple_base_course_freshness_domain','ple_rehearsal_freshness_witness','SELECT'),('ple_base_course_freshness_domain','ple_rehearsal_freshness_witness','MAINTAIN'),
          ('ple_base_course_freshness_relation','ple_rehearsal_freshness_witness','SELECT'),('ple_base_course_freshness_relation','ple_rehearsal_freshness_witness','MAINTAIN')
  ), actual AS (
   SELECT table_row.relname,role_row.rolname,privilege.privilege_type FROM pg_catalog.pg_class table_row
    JOIN pg_catalog.pg_namespace namespace ON namespace.oid=table_row.relnamespace
    CROSS JOIN LATERAL aclexplode(COALESCE(table_row.relacl,acldefault('r',table_row.relowner))) privilege
    JOIN pg_catalog.pg_roles role_row ON role_row.oid=privilege.grantee
   WHERE namespace.nspname='public' AND table_row.relname IN('question_id_namespace','ple_base_course_freshness_domain','ple_base_course_freshness_relation')
     AND role_row.rolname IN('ple_base_course_freshness_broker','ple_rehearsal_freshness_witness') AND privilege.grantee<>table_row.relowner
  ) SELECT 1 FROM ((SELECT * FROM expected EXCEPT SELECT * FROM actual) UNION ALL (SELECT * FROM actual EXCEPT SELECT * FROM expected)) difference
 ) THEN RAISE EXCEPTION 'Base Course freshness metadata privilege matrix is unsafe' USING ERRCODE='55000'; END IF;
 IF EXISTS(
  WITH expected(relation_oid,policy_name,role_name) AS (
   SELECT registry.relation_oid,'ple_base_course_freshness_select'::name,'ple_base_course_freshness_broker'::name
     FROM public.ple_base_course_freshness_relation registry JOIN pg_catalog.pg_class table_row ON table_row.oid=registry.relation_oid
    WHERE registry.domain='raw' AND table_row.relrowsecurity
   UNION ALL
   SELECT registry.relation_oid,'ple_rehearsal_freshness_witness_select'::name,'ple_rehearsal_freshness_witness'::name
     FROM public.ple_base_course_freshness_relation registry JOIN pg_catalog.pg_class table_row ON table_row.oid=registry.relation_oid
    WHERE registry.domain='sealed_rehearsal' AND table_row.relrowsecurity
  ), actual AS (
   SELECT policy.polrelid,policy.polname,role_row.rolname
     FROM pg_catalog.pg_policy policy JOIN pg_catalog.pg_roles role_row ON role_row.oid=ANY(policy.polroles)
    WHERE policy.polrelid IN(SELECT relation_oid FROM public.ple_base_course_freshness_relation)
      AND role_row.rolname IN('ple_base_course_freshness_broker','ple_rehearsal_freshness_witness')
      AND policy.polcmd='r' AND policy.polpermissive AND pg_get_expr(policy.polqual,policy.polrelid)='true'
      AND policy.polwithcheck IS NULL
  ) SELECT 1 FROM ((SELECT * FROM expected EXCEPT SELECT * FROM actual)
                   UNION ALL (SELECT * FROM actual EXCEPT SELECT * FROM expected)) difference
 ) OR EXISTS(
  SELECT 1 FROM pg_catalog.pg_policy policy JOIN pg_catalog.pg_roles role_row ON role_row.oid=ANY(policy.polroles)
   WHERE role_row.rolname IN('ple_base_course_freshness_broker','ple_rehearsal_freshness_witness')
     AND (policy.polcmd<>'r' OR NOT policy.polpermissive OR pg_get_expr(policy.polqual,policy.polrelid)<>'true' OR policy.polwithcheck IS NOT NULL)
 ) THEN RAISE EXCEPTION 'Base Course freshness RLS policy matrix is unsafe' USING ERRCODE='55000'; END IF;
 IF EXISTS(SELECT 1 FROM pg_catalog.pg_auth_members WHERE roleid IN('ple_base_course_freshness_broker'::regrole,'ple_rehearsal_freshness_witness'::regrole) OR member IN('ple_base_course_freshness_broker'::regrole,'ple_rehearsal_freshness_witness'::regrole)) THEN
  RAISE EXCEPTION 'Base Course freshness roles must have no memberships' USING ERRCODE='55000';
 END IF;
 IF (SELECT count(*) FROM pg_catalog.pg_roles role_row
      WHERE role_row.rolname IN('ple_base_course_freshness_broker','ple_rehearsal_freshness_witness','ple_base_course_freshness_registry_owner')
        AND NOT(role_row.rolcanlogin OR role_row.rolsuper OR role_row.rolcreatedb OR role_row.rolcreaterole OR role_row.rolinherit OR role_row.rolreplication OR role_row.rolbypassrls))<>3
    OR EXISTS(SELECT 1 FROM pg_catalog.pg_auth_members WHERE roleid IN('ple_base_course_freshness_broker'::regrole,'ple_rehearsal_freshness_witness'::regrole,'ple_base_course_freshness_registry_owner'::regrole) OR member IN('ple_base_course_freshness_broker'::regrole,'ple_rehearsal_freshness_witness'::regrole,'ple_base_course_freshness_registry_owner'::regrole))
    OR (SELECT count(*) FROM pg_catalog.pg_class table_row JOIN pg_catalog.pg_roles role_row ON role_row.oid=table_row.relowner WHERE table_row.oid IN('public.ple_base_course_freshness_domain'::regclass,'public.ple_base_course_freshness_relation'::regclass) AND role_row.rolname='ple_base_course_freshness_registry_owner')<>2
    OR EXISTS(SELECT 1 FROM pg_catalog.pg_class table_row WHERE table_row.relowner IN('ple_base_course_freshness_broker'::regrole,'ple_rehearsal_freshness_witness'::regrole))
    OR EXISTS(SELECT 1 FROM public.ple_base_course_freshness_relation registry JOIN pg_catalog.pg_class table_row ON table_row.oid=registry.relation_oid WHERE table_row.relrowsecurity AND NOT table_row.relforcerowsecurity)
    OR EXISTS(SELECT 1 FROM pg_catalog.pg_class table_row CROSS JOIN LATERAL aclexplode(COALESCE(table_row.relacl,acldefault('r',table_row.relowner))) privilege WHERE table_row.oid IN(SELECT relation_oid FROM public.ple_base_course_freshness_relation) AND privilege.grantee=0)
    OR EXISTS(SELECT 1 FROM pg_catalog.pg_attribute attribute_row CROSS JOIN LATERAL aclexplode(attribute_row.attacl) privilege WHERE attribute_row.attnum>0 AND NOT attribute_row.attisdropped AND privilege.grantee IN('ple_base_course_freshness_broker'::regrole,'ple_rehearsal_freshness_witness'::regrole))
    OR EXISTS(SELECT 1 FROM pg_catalog.pg_class sequence_row WHERE sequence_row.relkind='S' AND (has_sequence_privilege('ple_base_course_freshness_broker',sequence_row.oid,'USAGE') OR has_sequence_privilege('ple_base_course_freshness_broker',sequence_row.oid,'SELECT') OR has_sequence_privilege('ple_base_course_freshness_broker',sequence_row.oid,'UPDATE') OR has_sequence_privilege('ple_rehearsal_freshness_witness',sequence_row.oid,'USAGE') OR has_sequence_privilege('ple_rehearsal_freshness_witness',sequence_row.oid,'SELECT') OR has_sequence_privilege('ple_rehearsal_freshness_witness',sequence_row.oid,'UPDATE')))
    OR EXISTS(SELECT 1 FROM pg_catalog.pg_roles role_row WHERE role_row.rolname IN('ple_base_course_freshness_broker','ple_rehearsal_freshness_witness','ple_base_course_freshness_registry_owner') AND (NOT has_schema_privilege(role_row.rolname,'public','USAGE') OR has_schema_privilege(role_row.rolname,'public','CREATE'))) THEN
  RAISE EXCEPTION 'Base Course freshness role boundary is unsafe' USING ERRCODE='55000';
 END IF;
 FOR relation_row IN
  SELECT namespace.nspname,table_row.relname FROM public.ple_base_course_freshness_relation registry
  JOIN pg_catalog.pg_class table_row ON table_row.oid=registry.relation_oid
  JOIN pg_catalog.pg_namespace namespace ON namespace.oid=table_row.relnamespace
 WHERE registry.domain='raw' ORDER BY namespace.nspname,table_row.relname,table_row.oid
 LOOP EXECUTE format('LOCK TABLE ONLY %I.%I IN SHARE MODE',relation_row.nspname,relation_row.relname); END LOOP;
 LOCK TABLE ONLY public.question_id_namespace IN SHARE MODE;
 SELECT count(*),count(*) FILTER(WHERE singleton AND issued_count=0) INTO namespace_rows,unconsumed_namespace_rows FROM public.question_id_namespace;
 IF (namespace_rows,unconsumed_namespace_rows) IS DISTINCT FROM (1::bigint,1::bigint) THEN failure_kind:='unconsumed_question_namespace';relation_name:=NULL;RETURN NEXT;RETURN;END IF;
 FOR relation_row IN
  SELECT namespace.nspname,table_row.relname FROM public.ple_base_course_freshness_relation registry
  JOIN pg_catalog.pg_class table_row ON table_row.oid=registry.relation_oid
  JOIN pg_catalog.pg_namespace namespace ON namespace.oid=table_row.relnamespace
 WHERE registry.domain='raw' ORDER BY namespace.nspname,table_row.relname,table_row.oid
 LOOP
  EXECUTE format('SELECT EXISTS(SELECT 1 FROM ONLY %I.%I LIMIT 1)',relation_row.nspname,relation_row.relname) INTO relation_has_rows;
  IF relation_has_rows THEN failure_kind:='nonempty_application_relation';relation_name:=relation_row.nspname::text||'.'||relation_row.relname::text;RETURN NEXT;RETURN;END IF;
 END LOOP;
 IF NOT public.ple_verify_sealed_rehearsal_freshness_empty() THEN failure_kind:='nonempty_application_relation';relation_name:='public.rehearsal_operation_protocol';RETURN NEXT;RETURN;END IF;
 failure_kind:=NULL;relation_name:=NULL;RETURN NEXT;
END $$;
ALTER FUNCTION public.ple_verify_sealed_rehearsal_freshness_empty() OWNER TO ple_rehearsal_freshness_witness;
ALTER FUNCTION public.ple_require_fresh_base_course_install_internal() OWNER TO ple_base_course_freshness_broker;
ALTER TABLE public.ple_base_course_freshness_domain OWNER TO ple_base_course_freshness_registry_owner;
ALTER TABLE public.ple_base_course_freshness_relation OWNER TO ple_base_course_freshness_registry_owner;
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM ple_base_course_freshness_broker,ple_rehearsal_freshness_witness;
GRANT SELECT,MAINTAIN ON public.ple_base_course_freshness_domain TO ple_base_course_freshness_broker;
GRANT SELECT ON public.ple_base_course_freshness_relation TO ple_base_course_freshness_broker;
GRANT MAINTAIN ON public.ple_base_course_freshness_relation TO ple_base_course_freshness_broker;
GRANT SELECT,MAINTAIN ON public.ple_base_course_freshness_domain TO ple_rehearsal_freshness_witness;
GRANT SELECT,MAINTAIN ON public.ple_base_course_freshness_relation TO ple_rehearsal_freshness_witness;
GRANT SELECT,MAINTAIN ON public.question_id_namespace TO ple_base_course_freshness_broker;
CREATE POLICY ple_base_course_freshness_namespace_select ON public.question_id_namespace
    FOR SELECT TO ple_base_course_freshness_broker USING(true);
DO $$
DECLARE relation_row record;
BEGIN
 FOR relation_row IN
  SELECT table_row.oid,namespace.nspname,table_row.relname,table_row.relrowsecurity,registry.domain
    FROM public.ple_base_course_freshness_relation registry JOIN pg_catalog.pg_class table_row ON table_row.oid=registry.relation_oid
    JOIN pg_catalog.pg_namespace namespace ON namespace.oid=table_row.relnamespace
   ORDER BY namespace.nspname,table_row.relname,table_row.oid
 LOOP
  IF relation_row.domain='raw' THEN
   EXECUTE format('GRANT SELECT,MAINTAIN ON TABLE %I.%I TO ple_base_course_freshness_broker',relation_row.nspname,relation_row.relname);
   IF relation_row.relrowsecurity THEN
    EXECUTE format('DROP POLICY IF EXISTS ple_base_course_freshness_select ON %I.%I',relation_row.nspname,relation_row.relname);
    EXECUTE format('CREATE POLICY ple_base_course_freshness_select ON %I.%I FOR SELECT TO ple_base_course_freshness_broker USING(true)',relation_row.nspname,relation_row.relname);
   END IF;
  ELSE
   EXECUTE format('GRANT SELECT,MAINTAIN ON TABLE %I.%I TO ple_rehearsal_freshness_witness',relation_row.nspname,relation_row.relname);
   IF relation_row.relrowsecurity THEN
    EXECUTE format('DROP POLICY IF EXISTS ple_base_course_freshness_select ON %I.%I',relation_row.nspname,relation_row.relname);
    EXECUTE format('CREATE POLICY ple_rehearsal_freshness_witness_select ON %I.%I FOR SELECT TO ple_rehearsal_freshness_witness USING(true)',relation_row.nspname,relation_row.relname);
   END IF;
  END IF;
 END LOOP;
END $$;
REVOKE ALL ON FUNCTION public.ple_verify_sealed_rehearsal_freshness_empty() FROM PUBLIC,ple_app,ple_student,ple_grader,ple_grading_reader,ple_base_course_install_broker;
GRANT EXECUTE ON FUNCTION public.ple_verify_sealed_rehearsal_freshness_empty() TO ple_base_course_freshness_broker;
DROP FUNCTION IF EXISTS public.ple_verify_rehearsal_operation_protocol_empty();
COMMIT;
