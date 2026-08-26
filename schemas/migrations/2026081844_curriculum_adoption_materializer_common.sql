-- WP-PROF-B2: private common validation and immutable-adoption helpers.
--
-- This migration deliberately exposes no public bridge.  Migration 1847 owns
-- the one public dispatcher; 1845 and 1846 consume only these broker-private
-- helpers after their operation-specific relational rechecks.

BEGIN;

-- ASVS 1.5.2 and 2.2.1: callers must name every key they provide.  The
-- existing 1840 helper rejects unknown keys; this companion also rejects a
-- missing required key, so its accepted object shape is exact.
CREATE FUNCTION public.ple_cam_require_exact_object_v1(
    p_value jsonb, p_keys text[], p_limit integer
) RETURNS void LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_value IS NULL OR jsonb_typeof(p_value) <> 'object'
       OR octet_length(p_value::text) > p_limit
       OR EXISTS (
            SELECT 1 FROM jsonb_object_keys(p_value) AS key
             WHERE NOT key = ANY (p_keys)
       ) OR NOT p_value ?& p_keys THEN
        RAISE EXCEPTION 'curriculum adoption materialization object is invalid'
            USING ERRCODE = '22023';
    END IF;
END $$;

CREATE FUNCTION public.ple_cam_uuid_v1(p_value jsonb)
RETURNS uuid LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_text text;
BEGIN
    IF jsonb_typeof(p_value) <> 'string' THEN
        RAISE EXCEPTION 'curriculum adoption materialization identifier is invalid'
            USING ERRCODE = '22023';
    END IF;
    v_text := p_value #>> '{}';
    IF v_text !~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' THEN
        RAISE EXCEPTION 'curriculum adoption materialization identifier is invalid'
            USING ERRCODE = '22023';
    END IF;
    RETURN v_text::uuid;
END $$;

CREATE FUNCTION public.ple_cam_positive_revision_v1(p_value jsonb)
RETURNS bigint LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_text text;
BEGIN
    IF jsonb_typeof(p_value) <> 'string' THEN
        RAISE EXCEPTION 'curriculum adoption materialization revision is invalid'
            USING ERRCODE = '22023';
    END IF;
    v_text := p_value #>> '{}';
    IF v_text !~ '^[1-9][0-9]{0,18}$'
       OR v_text::numeric > 9223372036854775807::numeric THEN
        RAISE EXCEPTION 'curriculum adoption materialization revision is invalid'
            USING ERRCODE = '22023';
    END IF;
    RETURN v_text::bigint;
END $$;

-- PostgreSQL must preserve Rust's byte-array wire form without hashing a JSON
-- rendering.  The digest-specific wrapper below requires exactly 32 bytes.
CREATE FUNCTION public.ple_cam_bytes_v1(
    p_value jsonb, p_minimum integer, p_maximum integer
)
RETURNS bytea LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_bytes bytea;
BEGIN
    IF p_minimum < 0 OR p_maximum < p_minimum
       OR jsonb_typeof(p_value) <> 'array'
       OR jsonb_array_length(p_value) NOT BETWEEN p_minimum AND p_maximum
       OR EXISTS (
            SELECT 1
              FROM jsonb_array_elements(p_value) AS item(value)
             WHERE jsonb_typeof(item.value) <> 'number'
                OR item.value #>> '{}' !~ '^(0|[1-9][0-9]{0,2})$'
                OR (item.value #>> '{}')::integer > 255
       ) THEN
        RAISE EXCEPTION 'curriculum adoption materialization bytes are invalid'
            USING ERRCODE = '22023';
    END IF;
    SELECT decode(string_agg(lpad(to_hex((item.value #>> '{}')::integer), 2, '0'), ''
                             ORDER BY item.ordinality), 'hex')
      INTO v_bytes
      FROM jsonb_array_elements(p_value) WITH ORDINALITY AS item(value, ordinality);
    RETURN v_bytes;
END $$;

CREATE FUNCTION public.ple_cam_digest_bytes_v1(p_value jsonb)
RETURNS bytea LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    RETURN public.ple_cam_bytes_v1(p_value, 32, 32);
EXCEPTION WHEN SQLSTATE '22023' THEN
    RAISE EXCEPTION 'curriculum adoption materialization digest is invalid'
        USING ERRCODE = '22023';
END $$;

CREATE FUNCTION public.ple_cam_receipt_key_v1(p_value jsonb)
RETURNS text LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_key text;
BEGIN
    PERFORM public.ple_cam_require_exact_object_v1(p_value, ARRAY['idempotencyKey'], 4096);
    IF jsonb_typeof(p_value->'idempotencyKey') <> 'string' THEN
        RAISE EXCEPTION 'curriculum adoption materialization receipt is invalid'
            USING ERRCODE = '22023';
    END IF;
    v_key := p_value->>'idempotencyKey';
    IF v_key !~ '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$' THEN
        RAISE EXCEPTION 'curriculum adoption materialization receipt is invalid'
            USING ERRCODE = '22023';
    END IF;
    RETURN v_key;
END $$;

CREATE FUNCTION public.ple_cam_validate_semantic_v1(p_value jsonb)
RETURNS void LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_bytes bytea; v_digest bytea;
BEGIN
    PERFORM public.ple_cam_require_exact_object_v1(
        p_value,
        ARRAY['semanticInput', 'canonicalVersion', 'canonicalBytes', 'semanticDigest'],
        524288
    );
    IF jsonb_typeof(p_value->'semanticInput') <> 'object'
       OR jsonb_typeof(p_value->'canonicalVersion') <> 'number'
       OR p_value->>'canonicalVersion' <> '1' THEN
        RAISE EXCEPTION 'curriculum adoption semantic evidence is invalid'
            USING ERRCODE = '22023';
    END IF;
    v_bytes := public.ple_cam_bytes_v1(p_value->'canonicalBytes', 1, 524288);
    v_digest := public.ple_cam_digest_bytes_v1(p_value->'semanticDigest');
    IF v_digest IS DISTINCT FROM digest(v_bytes, 'sha256') THEN
        RAISE EXCEPTION 'curriculum adoption semantic evidence is invalid'
            USING ERRCODE = '22023';
    END IF;
END $$;

CREATE FUNCTION public.ple_cam_validate_materialization_envelope_v1(
    p_preparation uuid, p_envelope jsonb
) RETURNS jsonb LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_operation text; v_plan_kind text; v_plan_keys text[];
BEGIN
    -- Migration 1840's abandoned flat plan is intentionally not a fallback
    -- protocol: it has no actor/request digest binding or tagged plan union.
    IF p_envelope ? 'semanticInput' OR p_envelope ? 'semanticDigest'
       OR p_envelope ? 'termShift' THEN
        RAISE EXCEPTION 'legacy curriculum adoption materialization plan is invalid'
            USING ERRCODE = '22023';
    END IF;
    PERFORM public.ple_cam_require_exact_object_v1(
        p_envelope,
        ARRAY['version', 'operation', 'preparationId', 'actor', 'requestSha256', 'plan'],
        524288
    );
    IF p_preparation IS NULL OR jsonb_typeof(p_envelope->'version') <> 'number'
       OR p_envelope->>'version' <> '1'
       OR public.ple_cam_uuid_v1(p_envelope->'preparationId') IS DISTINCT FROM p_preparation THEN
        RAISE EXCEPTION 'curriculum adoption materialization envelope is invalid'
            USING ERRCODE = '22023';
    END IF;
    PERFORM public.ple_cam_uuid_v1(p_envelope->'actor');
    PERFORM public.ple_cam_digest_bytes_v1(p_envelope->'requestSha256');
    PERFORM public.ple_cam_require_exact_object_v1(p_envelope->'operation', ARRAY['kind'], 256);
    IF jsonb_typeof(p_envelope->'operation'->'kind') <> 'string' THEN
        RAISE EXCEPTION 'curriculum adoption materialization operation is invalid'
            USING ERRCODE = '22023';
    END IF;
    v_operation := p_envelope->'operation'->>'kind';
    PERFORM public.ple_cam_require_exact_object_v1(p_envelope->'plan', ARRAY['kind', 'plan'], 524288);
    IF jsonb_typeof(p_envelope->'plan'->'kind') <> 'string'
       OR jsonb_typeof(p_envelope->'plan'->'plan') <> 'object' THEN
        RAISE EXCEPTION 'curriculum adoption materialization plan is invalid'
            USING ERRCODE = '22023';
    END IF;
    v_plan_kind := p_envelope->'plan'->>'kind';
    v_plan_keys := CASE v_operation
        WHEN 'applyForkAlpha' THEN ARRAY['semantic', 'source']
        WHEN 'applyBlueprintInstantiation' THEN ARRAY[
            'semantic', 'source', 'destinationWitness', 'targetTerm', 'preview', 'corrections',
            'materialization'
        ]
        WHEN 'applyAlphaInstantiation' THEN ARRAY[
            'semantic', 'source', 'targetTerm', 'preview', 'corrections', 'assignments'
        ]
        WHEN 'applyCourseRollover' THEN ARRAY[
            'semantic', 'sourceWitness', 'targetTerm', 'preview', 'corrections', 'assignments',
            'rolloverSources'
        ]
        WHEN 'applyCourseTermShift' THEN ARRAY['semantic', 'courseWitness', 'targetTerm', 'rows']
        WHEN 'applyAssignmentFastForward' THEN ARRAY[
            'semantic', 'witness', 'assignment', 'expectedAssignmentRevision',
            'expectedImportRevision', 'targetTerm', 'materialization'
        ]
        WHEN 'createSourceDerivedAssignment' THEN ARRAY[
            'semantic', 'source', 'destinationWitness', 'targetTerm', 'preview', 'corrections',
            'materialization'
        ]
        ELSE NULL
    END;
    IF v_plan_keys IS NULL OR v_plan_kind IS DISTINCT FROM (CASE v_operation
        WHEN 'applyForkAlpha' THEN 'forkAlpha'
        WHEN 'applyBlueprintInstantiation' THEN 'blueprintInstantiation'
        WHEN 'applyAlphaInstantiation' THEN 'alphaInstantiation'
        WHEN 'applyCourseRollover' THEN 'courseRollover'
        WHEN 'applyCourseTermShift' THEN 'courseTermShift'
        WHEN 'applyAssignmentFastForward' THEN 'assignmentFastForward'
        WHEN 'createSourceDerivedAssignment' THEN 'sourceDerivedAssignment'
        ELSE NULL
    END) THEN
        RAISE EXCEPTION 'curriculum adoption materialization operation is invalid'
            USING ERRCODE = '22023';
    END IF;
    PERFORM public.ple_cam_require_exact_object_v1(p_envelope#>'{plan,plan}', v_plan_keys, 524288);
    PERFORM public.ple_cam_validate_semantic_v1(p_envelope#>'{plan,plan,semantic}');
    RETURN jsonb_build_object(
        'operation', v_operation,
        'actor', p_envelope->'actor',
        'requestSha256', p_envelope->'requestSha256',
        'plan', p_envelope->'plan'
    );
END $$;

CREATE FUNCTION public.ple_cam_validate_reconciliation_envelope_v1(
    p_preparation uuid, p_envelope jsonb
) RETURNS jsonb LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_repair jsonb; v_previous_reference integer := NULL; v_reference integer;
DECLARE v_key text;
BEGIN
    PERFORM public.ple_cam_require_exact_object_v1(
        p_envelope, ARRAY['version', 'operation', 'preparationId', 'actor', 'receipt', 'repairs'],
        524288
    );
    IF p_preparation IS NULL OR jsonb_typeof(p_envelope->'version') <> 'number'
       OR p_envelope->>'version' <> '1'
       OR public.ple_cam_uuid_v1(p_envelope->'preparationId') IS DISTINCT FROM p_preparation THEN
        RAISE EXCEPTION 'curriculum adoption reconciliation envelope is invalid'
            USING ERRCODE = '22023';
    END IF;
    PERFORM public.ple_cam_uuid_v1(p_envelope->'actor');
    PERFORM public.ple_cam_require_exact_object_v1(p_envelope->'operation', ARRAY['kind'], 256);
    IF p_envelope#>>'{operation,kind}' IS DISTINCT FROM 'reconcile' THEN
        RAISE EXCEPTION 'curriculum adoption reconciliation operation is invalid'
            USING ERRCODE = '22023';
    END IF;
    v_key := public.ple_cam_receipt_key_v1(p_envelope->'receipt');
    IF jsonb_typeof(p_envelope->'repairs') <> 'array'
       OR jsonb_array_length(p_envelope->'repairs') > 1024 THEN
        RAISE EXCEPTION 'curriculum adoption reconciliation repairs are invalid'
            USING ERRCODE = '22023';
    END IF;
    FOR v_repair IN SELECT item.value
      FROM jsonb_array_elements(p_envelope->'repairs') WITH ORDINALITY AS item(value, ordinality)
     ORDER BY item.ordinality
    LOOP
        PERFORM public.ple_cam_require_exact_object_v1(
            v_repair, ARRAY['assignment', 'expectedAssignmentRevision', 'receipt', 'revision'], 4096
        );
        v_reference := public.ple_curriculum_adoption_route_number_v1(v_repair->'assignment', 'A');
        IF (v_previous_reference IS NOT NULL AND v_reference <= v_previous_reference)
           OR public.ple_cam_positive_revision_v1(v_repair->'expectedAssignmentRevision') IS NULL
           OR public.ple_cam_positive_revision_v1(v_repair->'revision') IS NULL
           OR public.ple_cam_receipt_key_v1(v_repair->'receipt') IS DISTINCT FROM v_key THEN
            RAISE EXCEPTION 'curriculum adoption reconciliation repairs are invalid'
                USING ERRCODE = '22023';
        END IF;
        v_previous_reference := v_reference;
    END LOOP;
    RETURN jsonb_build_object(
        'operation', 'reconcile', 'actor', p_envelope->'actor', 'receiptKey', v_key,
        'repairs', p_envelope->'repairs'
    );
END $$;

-- ASVS 2.3.1 and 15.4.2: a preparation is valid only for the same tenant,
-- session-derived actor, operation, exact Rust-owned request digest, and
-- transaction.  Deleting it makes an apply envelope single-use while
-- PostgreSQL retains the snapshot locks until commit or rollback.
CREATE FUNCTION public.ple_cam_consume_materialization_preparation_v1(
    p_tenant uuid, p_session character(64), p_preparation uuid, p_envelope jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_actor uuid; v_envelope jsonb; v_preparation_row record; v_key text;
DECLARE v_request_sha256 bytea;
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'curriculum adoption materialization is unavailable' USING ERRCODE = '42501';
    END IF;
    v_actor := public.ple_curriculum_adoption_actor_v1(p_tenant, p_session);
    v_envelope := public.ple_cam_validate_materialization_envelope_v1(p_preparation, p_envelope);
    IF public.ple_cam_uuid_v1(v_envelope->'actor') IS DISTINCT FROM v_actor THEN
        RAISE EXCEPTION 'curriculum adoption materialization binding is unavailable'
            USING ERRCODE = 'PBC01';
    END IF;
    v_request_sha256 := public.ple_cam_digest_bytes_v1(v_envelope->'requestSha256');
    BEGIN
        SELECT * INTO v_preparation_row
          FROM pg_temp.ple_curriculum_adoption_materialization_preparation
         WHERE preparation_id = p_preparation
           AND tenant_id = p_tenant
           AND actor_user_id = v_actor
           AND operation = v_envelope->>'operation'
         FOR UPDATE;
    EXCEPTION WHEN undefined_table THEN
        RAISE EXCEPTION 'curriculum adoption preparation is unavailable' USING ERRCODE = 'PBC01';
    END;
    IF NOT FOUND OR jsonb_typeof(v_preparation_row.request) <> 'object'
       OR jsonb_typeof(v_preparation_row.facts) <> 'object'
       OR octet_length(v_preparation_row.request_sha256) <> 32
       OR v_preparation_row.request_sha256 IS DISTINCT FROM v_request_sha256 THEN
        RAISE EXCEPTION 'curriculum adoption preparation is unavailable' USING ERRCODE = 'PBC01';
    END IF;
    v_key := public.ple_cam_receipt_key_v1(
        jsonb_build_object('idempotencyKey', v_preparation_row.request->'idempotencyKey')
    );
    DELETE FROM pg_temp.ple_curriculum_adoption_materialization_preparation
     WHERE preparation_id = p_preparation;
    RETURN v_envelope || jsonb_build_object(
        'idempotencyKey', v_key,
        'request', v_preparation_row.request,
        'facts', v_preparation_row.facts
    );
END $$;

CREATE FUNCTION public.ple_cam_consume_reconciliation_preparation_v1(
    p_tenant uuid, p_session character(64), p_preparation uuid, p_envelope jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_actor uuid; v_envelope jsonb; v_preparation_row record;
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'curriculum adoption reconciliation is unavailable' USING ERRCODE = '42501';
    END IF;
    v_actor := public.ple_curriculum_adoption_actor_v1(p_tenant, p_session);
    v_envelope := public.ple_cam_validate_reconciliation_envelope_v1(p_preparation, p_envelope);
    IF public.ple_cam_uuid_v1(v_envelope->'actor') IS DISTINCT FROM v_actor THEN
        RAISE EXCEPTION 'curriculum adoption reconciliation binding is unavailable'
            USING ERRCODE = 'PBC01';
    END IF;
    BEGIN
        SELECT * INTO v_preparation_row
          FROM pg_temp.ple_curriculum_adoption_reconciliation_preparation
         WHERE preparation_id = p_preparation
           AND tenant_id = p_tenant
           AND actor_user_id = v_actor
         FOR UPDATE;
    EXCEPTION WHEN undefined_table THEN
        RAISE EXCEPTION 'curriculum adoption preparation is unavailable' USING ERRCODE = 'PBC01';
    END;
    IF NOT FOUND OR jsonb_typeof(v_preparation_row.request) <> 'object'
       OR jsonb_typeof(v_preparation_row.facts) <> 'object'
       OR public.ple_cam_receipt_key_v1(v_preparation_row.request->'receipt')
          IS DISTINCT FROM v_envelope->>'receiptKey' THEN
        RAISE EXCEPTION 'curriculum adoption preparation is unavailable' USING ERRCODE = 'PBC01';
    END IF;
    DELETE FROM pg_temp.ple_curriculum_adoption_reconciliation_preparation
     WHERE preparation_id = p_preparation;
    RETURN v_envelope || jsonb_build_object(
        'request', v_preparation_row.request,
        'facts', v_preparation_row.facts
    );
END $$;

CREATE FUNCTION public.ple_cam_course_reference_v1(p_tenant uuid, p_course uuid)
RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_reference integer;
BEGIN
    SELECT course.public_id INTO v_reference
      FROM public.course AS course
     WHERE course.tenant_id = p_tenant AND course.course_id = p_course;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'curriculum adoption result is unavailable' USING ERRCODE = 'PBI01';
    END IF;
    RETURN to_jsonb('C-' || v_reference::text);
END $$;

CREATE FUNCTION public.ple_cam_assignment_reference_v1(p_tenant uuid, p_assignment uuid)
RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_reference integer;
BEGIN
    SELECT assignment_row.public_id INTO v_reference
      FROM public.assignment AS assignment_row
     WHERE assignment_row.tenant_id = p_tenant AND assignment_row.assignment_id = p_assignment;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'curriculum adoption result is unavailable' USING ERRCODE = 'PBI01';
    END IF;
    RETURN to_jsonb('A-' || v_reference::text);
END $$;

CREATE FUNCTION public.ple_cam_alpha_reference_v1(p_alpha uuid)
RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_reference integer;
BEGIN
    SELECT alpha.alpha_course_reference INTO v_reference
      FROM public.alpha_course AS alpha
     WHERE alpha.alpha_course_id = p_alpha;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'curriculum adoption result is unavailable' USING ERRCODE = 'PBI01';
    END IF;
    RETURN to_jsonb('AC-' || v_reference::text);
END $$;

CREATE FUNCTION public.ple_cam_alpha_observation_v1(p_alpha uuid, p_revision bigint)
RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_revision IS NULL OR p_revision <= 0 THEN
        RAISE EXCEPTION 'curriculum adoption result is unavailable' USING ERRCODE = 'PBI01';
    END IF;
    RETURN jsonb_build_object(
        'reference', public.ple_cam_alpha_reference_v1(p_alpha),
        'revision', p_revision::text
    );
END $$;

-- Serialize one receipt identity before any materializer mints an aggregate.
-- The advisory key derives only from the tenant/key identity, never from a
-- semantic or request JSON representation.
CREATE FUNCTION public.ple_cam_select_receipt_v1(
    p_tenant uuid, p_key text, p_operation text, p_actor uuid, p_request_sha256 bytea
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_receipt public.curriculum_adoption_receipt%ROWTYPE;
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_key !~ '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$'
       OR p_operation NOT IN (
            'forkAlpha', 'blueprintInstantiation', 'alphaInstantiation', 'courseRollover',
            'courseTermShift', 'assignmentFastForward', 'sourceDerivedAssignment'
       ) OR p_actor IS NULL OR octet_length(p_request_sha256) <> 32 THEN
        RAISE EXCEPTION 'curriculum adoption materialization is unavailable' USING ERRCODE = '42501';
    END IF;
    PERFORM pg_advisory_xact_lock(
        hashtextextended(p_tenant::text || ':' || p_key, 2026081844::bigint)
    );
    SELECT * INTO v_receipt
      FROM public.curriculum_adoption_receipt AS receipt
     WHERE receipt.tenant_id = p_tenant AND receipt.idempotency_key = p_key
     FOR UPDATE;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;
    IF v_receipt.operation IS DISTINCT FROM p_operation
       OR v_receipt.actor_user_id IS DISTINCT FROM p_actor
       OR v_receipt.request_sha256 IS DISTINCT FROM p_request_sha256 THEN
        RAISE EXCEPTION 'curriculum adoption materialization conflicts' USING ERRCODE = 'PBC01';
    END IF;
    RETURN public.ple_cam_receipt_result_v1(p_tenant, p_key, true);
END $$;

CREATE FUNCTION public.ple_cam_insert_receipt_v1(
    p_tenant uuid, p_key text, p_operation text, p_actor uuid, p_request_sha256 bytea,
    p_destination_course uuid, p_destination_assignment uuid, p_destination_alpha uuid,
    p_source_course uuid, p_source_alpha uuid, p_import_revision bigint, p_target_term jsonb
) RETURNS void LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_target_required boolean;
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_key !~ '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$'
       OR p_actor IS NULL OR octet_length(p_request_sha256) <> 32 THEN
        RAISE EXCEPTION 'curriculum adoption materialization receipt is invalid'
            USING ERRCODE = '22023';
    END IF;
    v_target_required := p_operation IN ('blueprintInstantiation', 'alphaInstantiation',
        'courseRollover', 'courseTermShift');
    IF v_target_required THEN
        PERFORM public.ple_cam_require_exact_object_v1(
            p_target_term, ARRAY['startDate', 'endDate', 'timeZone'], 4096
        );
    ELSIF p_target_term IS NOT NULL THEN
        RAISE EXCEPTION 'curriculum adoption materialization receipt is invalid'
            USING ERRCODE = '22023';
    END IF;
    IF NOT (
        (p_operation = 'forkAlpha' AND p_destination_alpha IS NOT NULL AND p_source_alpha IS NOT NULL
         AND p_destination_course IS NULL AND p_destination_assignment IS NULL
         AND p_source_course IS NULL AND p_import_revision IS NULL)
        OR (p_operation = 'blueprintInstantiation' AND p_destination_course IS NOT NULL
            AND p_destination_assignment IS NOT NULL AND p_destination_alpha IS NULL
            AND p_source_course IS NULL AND p_source_alpha IS NULL AND p_import_revision IS NULL)
        OR (p_operation = 'alphaInstantiation' AND p_destination_course IS NOT NULL
            AND p_destination_assignment IS NULL AND p_destination_alpha IS NULL
            AND p_source_course IS NULL AND p_source_alpha IS NOT NULL AND p_import_revision IS NULL)
        OR (p_operation = 'courseRollover' AND p_destination_course IS NOT NULL
            AND p_destination_assignment IS NULL AND p_destination_alpha IS NULL
            AND p_source_course IS NOT NULL AND p_source_alpha IS NULL AND p_import_revision IS NULL)
        OR (p_operation = 'courseTermShift' AND p_destination_course IS NOT NULL
            AND p_destination_assignment IS NULL AND p_destination_alpha IS NULL
            AND p_source_course IS NULL AND p_source_alpha IS NULL AND p_import_revision IS NULL)
        OR (p_operation = 'assignmentFastForward' AND p_destination_course IS NOT NULL
            AND p_destination_assignment IS NOT NULL AND p_destination_alpha IS NULL
            AND p_source_course IS NULL AND p_source_alpha IS NULL AND p_import_revision > 0)
        OR (p_operation = 'sourceDerivedAssignment' AND p_destination_course IS NOT NULL
            AND p_destination_assignment IS NOT NULL AND p_destination_alpha IS NULL
            AND p_source_course IS NULL AND p_source_alpha IS NULL AND p_import_revision IS NULL)
    ) THEN
        RAISE EXCEPTION 'curriculum adoption materialization receipt is invalid'
            USING ERRCODE = '22023';
    END IF;
    INSERT INTO public.curriculum_adoption_receipt (
        tenant_id, idempotency_key, operation, actor_user_id, request_sha256,
        destination_course_id, destination_assignment_id, destination_alpha_course_id,
        source_course_id, source_alpha_course_id, outcome_import_revision, target_term_json
    ) VALUES (
        p_tenant, p_key, p_operation, p_actor, p_request_sha256,
        p_destination_course, p_destination_assignment, p_destination_alpha,
        p_source_course, p_source_alpha, p_import_revision, p_target_term
    );
END $$;

CREATE FUNCTION public.ple_cam_insert_evidence_v1(
    p_tenant uuid, p_receipt_key text, p_course uuid, p_assignment uuid, p_import_revision bigint,
    p_semantic_payload jsonb, p_canonical_version integer, p_canonical_bytes bytea,
    p_semantic_sha256 bytea, p_provenance jsonb
) RETURNS void LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_kind text; v_operation text; v_blueprint_reference integer; v_blueprint_revision bigint;
DECLARE v_alpha uuid; v_alpha_revision bigint; v_module integer; v_definition integer;
DECLARE v_source_course uuid; v_schedule_revision bigint; v_source_assignment uuid;
DECLARE v_source_assignment_revision bigint;
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_receipt_key !~ '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$'
       OR p_course IS NULL OR p_assignment IS NULL OR p_import_revision IS NULL
       OR p_import_revision <= 0 OR jsonb_typeof(p_semantic_payload) <> 'object'
       OR octet_length(p_semantic_payload::text) NOT BETWEEN 2 AND 524288
       OR p_canonical_version <> 1 OR octet_length(p_canonical_bytes) NOT BETWEEN 1 AND 524288
       OR octet_length(p_semantic_sha256) <> 32
       OR p_semantic_sha256 IS DISTINCT FROM digest(p_canonical_bytes, 'sha256') THEN
        RAISE EXCEPTION 'curriculum adoption semantic evidence is invalid'
            USING ERRCODE = '22023';
    END IF;
    IF jsonb_typeof(p_provenance) <> 'object' OR jsonb_typeof(p_provenance->'kind') <> 'string' THEN
        RAISE EXCEPTION 'curriculum adoption provenance is invalid' USING ERRCODE = '22023';
    END IF;
    v_kind := p_provenance->>'kind';
    SELECT receipt.operation INTO v_operation
      FROM public.curriculum_adoption_receipt AS receipt
     WHERE receipt.tenant_id = p_tenant AND receipt.idempotency_key = p_receipt_key;
    IF NOT FOUND OR (v_operation = 'sourceDerivedAssignment' AND v_kind NOT IN ('blueprint', 'alpha')) THEN
        RAISE EXCEPTION 'curriculum adoption provenance is invalid' USING ERRCODE = '22023';
    END IF;
    IF v_kind = 'blueprint' THEN
        PERFORM public.ple_cam_require_exact_object_v1(
            p_provenance, ARRAY['kind', 'reference', 'revision'], 4096
        );
        v_blueprint_reference := public.ple_curriculum_adoption_route_number_v1(
            p_provenance->'reference', 'BP'
        );
        v_blueprint_revision := public.ple_cam_positive_revision_v1(p_provenance->'revision');
    ELSIF v_kind = 'alpha' THEN
        PERFORM public.ple_cam_require_exact_object_v1(
            p_provenance,
            ARRAY['kind', 'alphaCourseId', 'revision', 'moduleIndex', 'assignmentIndex'], 4096
        );
        v_alpha := public.ple_cam_uuid_v1(p_provenance->'alphaCourseId');
        v_alpha_revision := public.ple_cam_positive_revision_v1(p_provenance->'revision');
        IF jsonb_typeof(p_provenance->'moduleIndex') <> 'number'
           OR jsonb_typeof(p_provenance->'assignmentIndex') <> 'number'
           OR p_provenance->>'moduleIndex' !~ '^(0|[1-9][0-9]{0,3})$'
           OR p_provenance->>'assignmentIndex' !~ '^(0|[1-9][0-9]{0,3})$'
           OR (p_provenance->>'moduleIndex')::integer >= 1024
           OR (p_provenance->>'assignmentIndex')::integer >= 1024 THEN
            RAISE EXCEPTION 'curriculum adoption provenance is invalid' USING ERRCODE = '22023';
        END IF;
        v_module := (p_provenance->>'moduleIndex')::integer;
        v_definition := (p_provenance->>'assignmentIndex')::integer;
    ELSIF v_kind = 'rollover' THEN
        PERFORM public.ple_cam_require_exact_object_v1(
            p_provenance,
            ARRAY['kind', 'sourceCourseId', 'scheduleRevision', 'sourceAssignmentId',
                  'assignmentRevision'],
            4096
        );
        v_source_course := public.ple_cam_uuid_v1(p_provenance->'sourceCourseId');
        v_schedule_revision := public.ple_cam_positive_revision_v1(p_provenance->'scheduleRevision');
        v_source_assignment := public.ple_cam_uuid_v1(p_provenance->'sourceAssignmentId');
        v_source_assignment_revision := public.ple_cam_positive_revision_v1(
            p_provenance->'assignmentRevision'
        );
    ELSE
        RAISE EXCEPTION 'curriculum adoption provenance is invalid' USING ERRCODE = '22023';
    END IF;
    INSERT INTO public.curriculum_assignment_adoption_evidence (
        tenant_id, receipt_key, assignment_id, course_id, import_revision, semantic_payload,
        semantic_canonical_version, semantic_canonical_bytes, semantic_sha256, source_kind,
        source_blueprint_reference, source_blueprint_revision, source_alpha_course_id,
        source_alpha_revision, source_module_position, source_definition_position, source_course_id,
        source_course_schedule_revision, source_assignment_id, source_assignment_revision
    ) VALUES (
        p_tenant, p_receipt_key, p_assignment, p_course, p_import_revision, p_semantic_payload,
        p_canonical_version::smallint, p_canonical_bytes, p_semantic_sha256, v_kind,
        v_blueprint_reference, v_blueprint_revision, v_alpha, v_alpha_revision, v_module,
        v_definition, v_source_course, v_schedule_revision, v_source_assignment,
        v_source_assignment_revision
    );
END $$;

-- The pointer is the only repairable B2 adoption state.  It may reference
-- immutable evidence that already exists; it cannot create or alter receipt,
-- evidence, course, assignment, policy, learner, or schedule state.
CREATE FUNCTION public.ple_cam_upsert_current_v1(
    p_tenant uuid, p_assignment uuid, p_receipt_key text
) RETURNS void LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_assignment IS NULL OR p_receipt_key !~ '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$' THEN
        RAISE EXCEPTION 'curriculum adoption current import is unavailable' USING ERRCODE = '42501';
    END IF;
    PERFORM 1 FROM public.curriculum_assignment_adoption_evidence AS evidence
     WHERE evidence.tenant_id = p_tenant AND evidence.assignment_id = p_assignment
       AND evidence.receipt_key = p_receipt_key
     FOR KEY SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'curriculum adoption current import is unavailable' USING ERRCODE = 'PBC01';
    END IF;
    INSERT INTO public.curriculum_assignment_import_current (
        tenant_id, assignment_id, receipt_key
    ) VALUES (p_tenant, p_assignment, p_receipt_key)
    ON CONFLICT (tenant_id, assignment_id) DO UPDATE
        SET receipt_key = EXCLUDED.receipt_key;
END $$;

CREATE FUNCTION public.ple_cam_reconcile_current_v1(
    p_tenant uuid, p_receipt_key text, p_repairs jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_repair jsonb; v_assignment uuid; v_revision bigint; v_reference integer;
DECLARE v_expected bigint; v_import bigint; v_repaired jsonb := '[]'::jsonb;
DECLARE v_previous_reference integer := NULL;
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_receipt_key !~ '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$'
       OR jsonb_typeof(p_repairs) <> 'array' OR jsonb_array_length(p_repairs) > 1024 THEN
        RAISE EXCEPTION 'curriculum adoption reconciliation is unavailable' USING ERRCODE = '42501';
    END IF;
    FOR v_repair IN SELECT item.value
      FROM jsonb_array_elements(p_repairs) WITH ORDINALITY AS item(value, ordinality)
     ORDER BY item.ordinality
    LOOP
        PERFORM public.ple_cam_require_exact_object_v1(
            v_repair, ARRAY['assignment', 'expectedAssignmentRevision', 'receipt', 'revision'], 4096
        );
        IF public.ple_cam_receipt_key_v1(v_repair->'receipt') IS DISTINCT FROM p_receipt_key THEN
            RAISE EXCEPTION 'curriculum adoption reconciliation is unavailable' USING ERRCODE = 'PBC01';
        END IF;
        v_reference := public.ple_curriculum_adoption_route_number_v1(v_repair->'assignment', 'A');
        IF v_previous_reference IS NOT NULL AND v_reference <= v_previous_reference THEN
            RAISE EXCEPTION 'curriculum adoption reconciliation is unavailable' USING ERRCODE = 'PBC01';
        END IF;
        v_expected := public.ple_cam_positive_revision_v1(v_repair->'expectedAssignmentRevision');
        v_import := public.ple_cam_positive_revision_v1(v_repair->'revision');
        SELECT assignment_row.assignment_id, assignment_row.revision
          INTO v_assignment, v_revision
          FROM public.assignment AS assignment_row
         WHERE assignment_row.tenant_id = p_tenant AND assignment_row.public_id = v_reference
         FOR KEY SHARE;
        IF NOT FOUND OR v_revision IS DISTINCT FROM v_expected THEN
            RAISE EXCEPTION 'curriculum adoption reconciliation is unavailable' USING ERRCODE = 'PBC01';
        END IF;
        PERFORM 1 FROM public.curriculum_assignment_adoption_evidence AS evidence
         WHERE evidence.tenant_id = p_tenant AND evidence.receipt_key = p_receipt_key
           AND evidence.assignment_id = v_assignment AND evidence.import_revision = v_import
         FOR KEY SHARE;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'curriculum adoption reconciliation is unavailable' USING ERRCODE = 'PBC01';
        END IF;
        PERFORM public.ple_cam_upsert_current_v1(p_tenant, v_assignment, p_receipt_key);
        v_repaired := v_repaired || jsonb_build_array(v_repair->'assignment');
        v_previous_reference := v_reference;
    END LOOP;
    RETURN v_repaired;
END $$;

CREATE FUNCTION public.ple_cam_receipt_result_v1(
    p_tenant uuid, p_receipt_key text, p_replayed boolean
) RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_receipt public.curriculum_adoption_receipt%ROWTYPE; v_receipt_json jsonb;
DECLARE v_source_alpha uuid; v_source_alpha_revision bigint;
BEGIN
    SELECT * INTO v_receipt
      FROM public.curriculum_adoption_receipt AS receipt
     WHERE receipt.tenant_id = p_tenant AND receipt.idempotency_key = p_receipt_key;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'curriculum adoption result is unavailable' USING ERRCODE = 'PBI01';
    END IF;
    v_receipt_json := jsonb_build_object('idempotencyKey', p_receipt_key, 'replayed', p_replayed);
    CASE v_receipt.operation
        WHEN 'forkAlpha' THEN
            SELECT lineage.source_alpha_course_id, lineage.source_alpha_revision
              INTO v_source_alpha, v_source_alpha_revision
              FROM public.curriculum_alpha_fork_lineage AS lineage
             WHERE lineage.tenant_id = p_tenant AND lineage.receipt_key = p_receipt_key;
            IF NOT FOUND THEN
                RAISE EXCEPTION 'curriculum adoption result is unavailable' USING ERRCODE = 'PBI01';
            END IF;
            RETURN jsonb_build_object(
                'kind', 'forkAlpha', 'receipt', v_receipt_json,
                'source', public.ple_cam_alpha_observation_v1(v_source_alpha, v_source_alpha_revision),
                'alpha', public.ple_cam_alpha_reference_v1(v_receipt.destination_alpha_course_id)
            );
        WHEN 'blueprintInstantiation' THEN
            RETURN jsonb_build_object(
                'kind', 'blueprintInstantiation', 'receipt', v_receipt_json,
                'course', public.ple_cam_course_reference_v1(p_tenant, v_receipt.destination_course_id),
                'assignment', public.ple_cam_assignment_reference_v1(
                    p_tenant, v_receipt.destination_assignment_id
                )
            );
        WHEN 'alphaInstantiation' THEN
            SELECT adoption.source_alpha_course_id, adoption.source_alpha_revision
              INTO v_source_alpha, v_source_alpha_revision
              FROM public.curriculum_whole_course_adoption AS adoption
             WHERE adoption.tenant_id = p_tenant AND adoption.receipt_key = p_receipt_key;
            IF NOT FOUND THEN
                RAISE EXCEPTION 'curriculum adoption result is unavailable' USING ERRCODE = 'PBI01';
            END IF;
            RETURN jsonb_build_object(
                'kind', 'alphaInstantiation', 'receipt', v_receipt_json,
                'source', public.ple_cam_alpha_observation_v1(v_source_alpha, v_source_alpha_revision),
                'course', public.ple_cam_course_reference_v1(p_tenant, v_receipt.destination_course_id)
            );
        WHEN 'courseRollover' THEN
            RETURN jsonb_build_object(
                'kind', 'courseRollover', 'receipt', v_receipt_json,
                'sourceCourse', public.ple_cam_course_reference_v1(p_tenant, v_receipt.source_course_id),
                'course', public.ple_cam_course_reference_v1(p_tenant, v_receipt.destination_course_id)
            );
        WHEN 'courseTermShift' THEN
            RETURN jsonb_build_object(
                'kind', 'courseTermShift', 'receipt', v_receipt_json,
                'course', public.ple_cam_course_reference_v1(p_tenant, v_receipt.destination_course_id),
                'term', v_receipt.target_term_json
            );
        WHEN 'assignmentFastForward' THEN
            RETURN jsonb_build_object(
                'kind', 'assignmentFastForward', 'receipt', v_receipt_json,
                'course', public.ple_cam_course_reference_v1(p_tenant, v_receipt.destination_course_id),
                'assignment', public.ple_cam_assignment_reference_v1(
                    p_tenant, v_receipt.destination_assignment_id
                ),
                'importRevision', v_receipt.outcome_import_revision::text
            );
        WHEN 'sourceDerivedAssignment' THEN
            RETURN jsonb_build_object(
                'kind', 'sourceDerivedAssignment', 'receipt', v_receipt_json,
                'course', public.ple_cam_course_reference_v1(p_tenant, v_receipt.destination_course_id),
                'assignment', public.ple_cam_assignment_reference_v1(
                    p_tenant, v_receipt.destination_assignment_id
                )
            );
        ELSE
            RAISE EXCEPTION 'curriculum adoption result is unavailable' USING ERRCODE = 'PBI01';
    END CASE;
END $$;

CREATE FUNCTION public.ple_cam_reconciliation_result_v1(
    p_tenant uuid, p_receipt_key text, p_repaired_assignments jsonb
) RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_receipt_key !~ '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$'
       OR jsonb_typeof(p_repaired_assignments) <> 'array'
       OR jsonb_array_length(p_repaired_assignments) > 1024
       OR NOT EXISTS (
            SELECT 1 FROM public.curriculum_adoption_receipt AS receipt
             WHERE receipt.tenant_id = p_tenant AND receipt.idempotency_key = p_receipt_key
       ) THEN
        RAISE EXCEPTION 'curriculum adoption reconciliation is unavailable' USING ERRCODE = 'PBC01';
    END IF;
    RETURN jsonb_build_object(
        'kind', 'reconcile',
        'receipt', jsonb_build_object('idempotencyKey', p_receipt_key, 'replayed', false),
        'repairedAssignments', p_repaired_assignments
    );
END $$;

ALTER FUNCTION public.ple_cam_require_exact_object_v1(jsonb, text[], integer)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_uuid_v1(jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_positive_revision_v1(jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_bytes_v1(jsonb, integer, integer)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_digest_bytes_v1(jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_receipt_key_v1(jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_validate_semantic_v1(jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_validate_materialization_envelope_v1(uuid, jsonb)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_validate_reconciliation_envelope_v1(uuid, jsonb)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_consume_materialization_preparation_v1(uuid, character, uuid, jsonb)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_consume_reconciliation_preparation_v1(uuid, character, uuid, jsonb)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_course_reference_v1(uuid, uuid) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_assignment_reference_v1(uuid, uuid) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_alpha_reference_v1(uuid) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_alpha_observation_v1(uuid, bigint) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_select_receipt_v1(uuid, text, text, uuid, bytea)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_insert_receipt_v1(
    uuid, text, text, uuid, bytea, uuid, uuid, uuid, uuid, uuid, bigint, jsonb
) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_insert_evidence_v1(
    uuid, text, uuid, uuid, bigint, jsonb, integer, bytea, bytea, jsonb
) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_upsert_current_v1(uuid, uuid, text)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_reconcile_current_v1(uuid, text, jsonb)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_receipt_result_v1(uuid, text, boolean)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cam_reconciliation_result_v1(uuid, text, jsonb)
    OWNER TO ple_curriculum_adoption_broker;

REVOKE ALL ON FUNCTION public.ple_cam_require_exact_object_v1(jsonb, text[], integer),
    public.ple_cam_uuid_v1(jsonb), public.ple_cam_positive_revision_v1(jsonb),
    public.ple_cam_bytes_v1(jsonb, integer, integer),
    public.ple_cam_digest_bytes_v1(jsonb), public.ple_cam_receipt_key_v1(jsonb),
    public.ple_cam_validate_semantic_v1(jsonb),
    public.ple_cam_validate_materialization_envelope_v1(uuid, jsonb),
    public.ple_cam_validate_reconciliation_envelope_v1(uuid, jsonb),
    public.ple_cam_consume_materialization_preparation_v1(uuid, character, uuid, jsonb),
    public.ple_cam_consume_reconciliation_preparation_v1(uuid, character, uuid, jsonb),
    public.ple_cam_course_reference_v1(uuid, uuid), public.ple_cam_assignment_reference_v1(uuid, uuid),
    public.ple_cam_alpha_reference_v1(uuid), public.ple_cam_alpha_observation_v1(uuid, bigint),
    public.ple_cam_select_receipt_v1(uuid, text, text, uuid, bytea),
    public.ple_cam_insert_receipt_v1(
        uuid, text, text, uuid, bytea, uuid, uuid, uuid, uuid, uuid, bigint, jsonb
    ), public.ple_cam_insert_evidence_v1(
        uuid, text, uuid, uuid, bigint, jsonb, integer, bytea, bytea, jsonb
    ), public.ple_cam_upsert_current_v1(uuid, uuid, text),
    public.ple_cam_reconcile_current_v1(uuid, text, jsonb),
    public.ple_cam_receipt_result_v1(uuid, text, boolean),
    public.ple_cam_reconciliation_result_v1(uuid, text, jsonb)
    FROM PUBLIC, ple_app, ple_auth, ple_student, ple_grader, ple_grading_reader,
         ple_retention_broker, ple_curriculum_adoption_broker;

-- Security-definer helpers execute as this broker owner.  It needs the
-- narrow internal EXECUTE capability to compose them, while every application
-- role remains excluded by the revocation above.
GRANT EXECUTE ON FUNCTION public.ple_cam_require_exact_object_v1(jsonb, text[], integer),
    public.ple_cam_uuid_v1(jsonb), public.ple_cam_positive_revision_v1(jsonb),
    public.ple_cam_bytes_v1(jsonb, integer, integer),
    public.ple_cam_digest_bytes_v1(jsonb), public.ple_cam_receipt_key_v1(jsonb),
    public.ple_cam_validate_semantic_v1(jsonb),
    public.ple_cam_validate_materialization_envelope_v1(uuid, jsonb),
    public.ple_cam_validate_reconciliation_envelope_v1(uuid, jsonb),
    public.ple_cam_consume_materialization_preparation_v1(uuid, character, uuid, jsonb),
    public.ple_cam_consume_reconciliation_preparation_v1(uuid, character, uuid, jsonb),
    public.ple_cam_course_reference_v1(uuid, uuid), public.ple_cam_assignment_reference_v1(uuid, uuid),
    public.ple_cam_alpha_reference_v1(uuid), public.ple_cam_alpha_observation_v1(uuid, bigint),
    public.ple_cam_select_receipt_v1(uuid, text, text, uuid, bytea),
    public.ple_cam_insert_receipt_v1(
        uuid, text, text, uuid, bytea, uuid, uuid, uuid, uuid, uuid, bigint, jsonb
    ), public.ple_cam_insert_evidence_v1(
        uuid, text, uuid, uuid, bigint, jsonb, integer, bytea, bytea, jsonb
    ), public.ple_cam_upsert_current_v1(uuid, uuid, text),
    public.ple_cam_reconcile_current_v1(uuid, text, jsonb),
    public.ple_cam_receipt_result_v1(uuid, text, boolean),
    public.ple_cam_reconciliation_result_v1(uuid, text, jsonb)
    TO ple_curriculum_adoption_broker;

-- 1844 asserts only its own private helper catalog.  The final public bridge,
-- role grants, and complete material-tree assertions intentionally remain 1847.
DO $$
DECLARE v_function regprocedure;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'public.ple_cam_require_exact_object_v1(jsonb,text[],integer)'::regprocedure,
        'public.ple_cam_uuid_v1(jsonb)'::regprocedure,
        'public.ple_cam_positive_revision_v1(jsonb)'::regprocedure,
        'public.ple_cam_bytes_v1(jsonb,integer,integer)'::regprocedure,
        'public.ple_cam_digest_bytes_v1(jsonb)'::regprocedure,
        'public.ple_cam_receipt_key_v1(jsonb)'::regprocedure,
        'public.ple_cam_validate_semantic_v1(jsonb)'::regprocedure,
        'public.ple_cam_validate_materialization_envelope_v1(uuid,jsonb)'::regprocedure,
        'public.ple_cam_validate_reconciliation_envelope_v1(uuid,jsonb)'::regprocedure,
        'public.ple_cam_consume_materialization_preparation_v1(uuid,character,uuid,jsonb)'::regprocedure,
        'public.ple_cam_consume_reconciliation_preparation_v1(uuid,character,uuid,jsonb)'::regprocedure,
        'public.ple_cam_course_reference_v1(uuid,uuid)'::regprocedure,
        'public.ple_cam_assignment_reference_v1(uuid,uuid)'::regprocedure,
        'public.ple_cam_alpha_reference_v1(uuid)'::regprocedure,
        'public.ple_cam_alpha_observation_v1(uuid,bigint)'::regprocedure,
        'public.ple_cam_select_receipt_v1(uuid,text,text,uuid,bytea)'::regprocedure,
        'public.ple_cam_insert_receipt_v1(uuid,text,text,uuid,bytea,uuid,uuid,uuid,uuid,uuid,bigint,jsonb)'::regprocedure,
        'public.ple_cam_insert_evidence_v1(uuid,text,uuid,uuid,bigint,jsonb,integer,bytea,bytea,jsonb)'::regprocedure,
        'public.ple_cam_upsert_current_v1(uuid,uuid,text)'::regprocedure,
        'public.ple_cam_reconcile_current_v1(uuid,text,jsonb)'::regprocedure,
        'public.ple_cam_receipt_result_v1(uuid,text,boolean)'::regprocedure,
        'public.ple_cam_reconciliation_result_v1(uuid,text,jsonb)'::regprocedure
    ] LOOP
        IF (SELECT pg_get_userbyid(proowner) FROM pg_proc WHERE oid = v_function)
           <> 'ple_curriculum_adoption_broker'
           OR NOT (SELECT prosecdef FROM pg_proc WHERE oid = v_function)
           OR NOT coalesce(
                (SELECT proconfig @> ARRAY['search_path=pg_catalog, public, pg_temp']
                   FROM pg_proc WHERE oid = v_function),
                false
           ) THEN
            RAISE EXCEPTION 'curriculum adoption common helper catalog is unsafe';
        END IF;
        IF has_function_privilege('public', v_function, 'EXECUTE')
           OR has_function_privilege('ple_app', v_function, 'EXECUTE') THEN
            RAISE EXCEPTION 'curriculum adoption common helper leaked';
        END IF;
    END LOOP;
END $$;

COMMIT;
