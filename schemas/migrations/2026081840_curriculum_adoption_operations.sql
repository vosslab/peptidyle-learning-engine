-- WP-PROF-B2: closed relational preparation and execute-only operation bridge.
--
-- Rust/qmodel owns semantic normalization, digesting, pin substitution, ordering,
-- target-term schedule resolution, DST decisions, and public preview projection.
-- This migration owns authenticated relational facts, locks, preparation binding,
-- durable receipts/evidence, and the narrow repairable current-import projection.

BEGIN;

-- The NOLOGIN broker reads only the three destination witnesses needed to
-- bind public locators to an authenticated Instructor.  `ple_app` receives no
-- table access; forced RLS keeps the broker in the SQL-derived tenant.
CREATE POLICY curriculum_adoption_course_witness_read ON public.course
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_member_witness_read ON public.course_member
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_assignment_witness_read ON public.assignment
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());
GRANT SELECT ON public.course, public.course_member, public.assignment
    TO ple_curriculum_adoption_broker;
-- Direct-Instructor witnesses are locked during the operation snapshot.
-- This column grant enables that lock while the broker has no UPDATE policy.
GRANT UPDATE(course_membership_id) ON public.course_member
    TO ple_curriculum_adoption_broker;
CREATE POLICY curriculum_adoption_course_lock ON public.course
    FOR UPDATE TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant()) WITH CHECK (false);
CREATE POLICY curriculum_adoption_member_lock ON public.course_member
    FOR UPDATE TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant()) WITH CHECK (false);
CREATE POLICY curriculum_adoption_assignment_lock ON public.assignment
    FOR UPDATE TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant()) WITH CHECK (false);

-- ASVS 1.2.4, 1.5.2, 2.2.1: every bridge input is parameterized, closed,
-- bounded, and versioned before a relation is read or a lock is acquired.
CREATE FUNCTION public.ple_curriculum_adoption_closed_object_v1(
    p_value jsonb, p_keys text[], p_limit integer
) RETURNS void LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
BEGIN
    IF p_value IS NULL OR jsonb_typeof(p_value) <> 'object'
       OR octet_length(p_value::text) > p_limit
       OR EXISTS (SELECT 1 FROM jsonb_object_keys(p_value) AS key
                    WHERE NOT key = ANY (p_keys)) THEN
        RAISE EXCEPTION 'curriculum adoption JSON object is invalid'
            USING ERRCODE = '22023';
    END IF;
END $$;

CREATE FUNCTION public.ple_curriculum_adoption_route_number_v1(
    p_reference jsonb, p_prefix text
) RETURNS integer LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_text text; v_digits text;
BEGIN
    IF jsonb_typeof(p_reference) <> 'string' OR p_prefix NOT IN ('C', 'A', 'BP', 'AC') THEN
        RAISE EXCEPTION 'curriculum adoption route reference is invalid' USING ERRCODE = '22023';
    END IF;
    v_text := p_reference #>> '{}';
    IF v_text !~ ('^' || p_prefix || '-[1-9][0-9]{0,9}$') THEN
        RAISE EXCEPTION 'curriculum adoption route reference is invalid' USING ERRCODE = '22023';
    END IF;
    v_digits := substr(v_text, char_length(p_prefix) + 2);
    IF v_digits::numeric > 2147483647 THEN
        RAISE EXCEPTION 'curriculum adoption route reference is invalid' USING ERRCODE = '22023';
    END IF;
    RETURN v_digits::integer;
EXCEPTION WHEN numeric_value_out_of_range THEN
    RAISE EXCEPTION 'curriculum adoption route reference is invalid' USING ERRCODE = '22023';
END $$;

CREATE FUNCTION public.ple_curriculum_adoption_bridge_operation_v1(p_value jsonb)
RETURNS text LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_kind text; v_request jsonb; v_keys text[]; v_binding jsonb;
BEGIN
    PERFORM public.ple_curriculum_adoption_closed_object_v1(
        p_value, ARRAY['version', 'operation', 'request', 'materializationBinding'], 524288
    );
    IF NOT p_value ?& ARRAY['version', 'operation', 'request']
       OR p_value->>'version' <> '1' OR jsonb_typeof(p_value->'version') <> 'number' THEN
        RAISE EXCEPTION 'curriculum adoption bridge version is invalid' USING ERRCODE = '22023';
    END IF;
    PERFORM public.ple_curriculum_adoption_closed_object_v1(
        p_value->'operation', ARRAY['kind'], 128
    );
    IF NOT p_value->'operation' ? 'kind' OR jsonb_typeof(p_value->'operation'->'kind') <> 'string' THEN
        RAISE EXCEPTION 'curriculum adoption bridge operation is invalid' USING ERRCODE = '22023';
    END IF;
    v_kind := p_value->'operation'->>'kind'; v_request := p_value->'request';
    -- Inspection intentionally receives the scalar CourseReference wire; it
    -- has no command envelope and cannot carry browser authority by itself.
    IF v_kind = 'inspectImports' THEN
        PERFORM public.ple_curriculum_adoption_route_number_v1(v_request, 'C');
        RETURN v_kind;
    END IF;
    v_keys := CASE v_kind
        WHEN 'previewForkAlpha' THEN ARRAY['source','replacements']
        WHEN 'applyForkAlpha' THEN ARRAY['source','replacements','idempotencyKey']
        WHEN 'previewBlueprintInstantiation' THEN ARRAY['source','course','targetTerm','replacements']
        WHEN 'applyBlueprintInstantiation' THEN ARRAY['source','course','targetTerm','previewWitness','replacements','idempotencyKey']
        WHEN 'previewAlphaInstantiation' THEN ARRAY['source','title','targetTerm','replacements']
        WHEN 'applyAlphaInstantiation' THEN ARRAY['source','title','targetTerm','replacements','idempotencyKey']
        WHEN 'previewCourseRollover' THEN ARRAY['witness','title','targetTerm','replacements']
        WHEN 'applyCourseRollover' THEN ARRAY['previewWitness','title','targetTerm','replacements','idempotencyKey']
        WHEN 'previewCourseTermShift' THEN ARRAY['witness','targetTerm']
        WHEN 'applyCourseTermShift' THEN ARRAY['previewWitness','targetTerm','idempotencyKey']
        WHEN 'previewAssignmentFastForward' THEN ARRAY['course','assignment','importRevision','source']
        WHEN 'applyAssignmentFastForward' THEN ARRAY['course','assignment','importRevision','source','previewWitness','idempotencyKey']
        WHEN 'previewSourceDerivedAssignment' THEN ARRAY['course','source','replacements']
        WHEN 'createSourceDerivedAssignment' THEN ARRAY['course','source','previewWitness','replacements','idempotencyKey']
        WHEN 'inspectImports' THEN ARRAY['course']
        WHEN 'reconcile' THEN ARRAY['receipt']
        ELSE NULL
    END;
    IF v_keys IS NULL THEN
        RAISE EXCEPTION 'curriculum adoption bridge operation is invalid' USING ERRCODE = '22023';
    END IF;
    PERFORM public.ple_curriculum_adoption_closed_object_v1(v_request, v_keys, 524288);
    IF NOT v_request ?& v_keys THEN
        RAISE EXCEPTION 'curriculum adoption request fields are invalid' USING ERRCODE = '22023';
    END IF;
    IF v_kind LIKE 'apply%' OR v_kind IN ('createSourceDerivedAssignment', 'reconcile') THEN
        IF jsonb_typeof(v_request->COALESCE(NULLIF('idempotencyKey', ''), 'receipt')) <> 'string'
           AND v_kind <> 'reconcile' THEN
            RAISE EXCEPTION 'curriculum adoption idempotency key is invalid' USING ERRCODE = '22023';
        END IF;
    END IF;
    IF v_kind IN (
        'applyForkAlpha', 'applyBlueprintInstantiation', 'applyAlphaInstantiation',
        'applyCourseRollover', 'applyCourseTermShift', 'applyAssignmentFastForward',
        'createSourceDerivedAssignment'
    ) THEN
        -- The canonical byte value is Rust-owned, but this exact closed wire
        -- is checked before a locked snapshot can persist it (ASVS 2.2.1).
        v_binding := p_value->'materializationBinding';
        PERFORM public.ple_cam_require_exact_object_v1(
            v_binding, ARRAY['version', 'actor', 'requestSha256'], 4096
        );
        IF jsonb_typeof(v_binding->'version') <> 'number'
           OR v_binding->>'version' <> '1' THEN
            RAISE EXCEPTION 'curriculum adoption materialization binding is invalid'
                USING ERRCODE = '22023';
        END IF;
        PERFORM public.ple_cam_uuid_v1(v_binding->'actor');
        PERFORM public.ple_cam_digest_bytes_v1(v_binding->'requestSha256');
    ELSIF p_value ? 'materializationBinding' THEN
        -- Preview, inspection, and receipt-led reconciliation cannot acquire
        -- a synthetic request digest or materialization identity.
        RAISE EXCEPTION 'curriculum adoption materialization binding is invalid'
            USING ERRCODE = '22023';
    END IF;
    RETURN v_kind;
END $$;

-- The table is connection-local, not a durable substitute for a receipt.  It
-- binds the Rust-produced plan to the exact session, actor, operation, and
-- locked relational facts in the same transaction (ASVS 2.3.1).  Rust carries
-- any canonical request digest; SQL must never invent one from JSON text.
CREATE FUNCTION public.ple_curriculum_adoption_prepare_temp_v1()
RETURNS void LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    CREATE TEMP TABLE IF NOT EXISTS pg_temp.ple_curriculum_adoption_materialization_preparation (
        preparation_id uuid PRIMARY KEY,
        tenant_id uuid NOT NULL,
        actor_user_id uuid NOT NULL,
        operation text NOT NULL,
        request jsonb NOT NULL,
        facts jsonb NOT NULL,
        request_sha256 bytea NOT NULL CHECK (octet_length(request_sha256) = 32),
        created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
        CHECK (operation IN (
            'applyForkAlpha', 'applyBlueprintInstantiation', 'applyAlphaInstantiation',
            'applyCourseRollover', 'applyCourseTermShift', 'applyAssignmentFastForward',
            'createSourceDerivedAssignment'
        )),
        CHECK (jsonb_typeof(request) = 'object'), CHECK (jsonb_typeof(facts) = 'object')
    ) ON COMMIT DELETE ROWS;
    CREATE TEMP TABLE IF NOT EXISTS pg_temp.ple_curriculum_adoption_reconciliation_preparation (
        preparation_id uuid PRIMARY KEY,
        tenant_id uuid NOT NULL,
        actor_user_id uuid NOT NULL,
        request jsonb NOT NULL,
        facts jsonb NOT NULL,
        created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
        CHECK (jsonb_typeof(request) = 'object'), CHECK (jsonb_typeof(facts) = 'object')
    ) ON COMMIT DELETE ROWS;
    -- A pooled backend retains only empty temp-table definitions.  Clearing
    -- both closed contracts preserves one preparation per transaction without
    -- weakening receipt-free reconciliation with a nullable digest.
    DELETE FROM pg_temp.ple_curriculum_adoption_materialization_preparation;
    DELETE FROM pg_temp.ple_curriculum_adoption_reconciliation_preparation;
END $$;

CREATE FUNCTION public.ple_curriculum_adoption_lock_course_v1(
    p_tenant uuid, p_actor uuid, p_reference jsonb
) RETURNS uuid LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_course uuid;
BEGIN
    SELECT course.course_id INTO v_course FROM public.course AS course
     WHERE course.tenant_id = p_tenant
       AND course.public_id = public.ple_curriculum_adoption_route_number_v1(p_reference, 'C')
     FOR KEY SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'curriculum adoption course is unavailable' USING ERRCODE = 'PBN01';
    END IF;
    -- Canonical destination witness lock: course first, then the one active,
    -- direct Instructor membership.  Assignment writers continue with the
    -- existing course/advisory/assignment order in their own broker.
    PERFORM 1 FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant AND member.course_id = v_course
       AND member.user_id = p_actor AND member.role = 'instructor' AND member.status = 'active'
     ORDER BY member.course_membership_id FOR KEY SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'curriculum adoption requires direct Instructor authority'
            USING ERRCODE = '42501';
    END IF;
    RETURN v_course;
END $$;

CREATE FUNCTION public.ple_curriculum_adoption_lock_assignment_v1(
    p_tenant uuid, p_course uuid, p_reference jsonb
) RETURNS uuid LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_assignment uuid;
BEGIN
    SELECT assignment.assignment_id INTO v_assignment FROM public.assignment AS assignment
     WHERE assignment.tenant_id = p_tenant AND assignment.course_id = p_course
       AND assignment.public_id = public.ple_curriculum_adoption_route_number_v1(p_reference, 'A')
     FOR KEY SHARE;
    IF NOT FOUND THEN RAISE EXCEPTION 'curriculum adoption assignment is unavailable' USING ERRCODE = '42501'; END IF;
    RETURN v_assignment;
END $$;

-- Resolve only relational source authority.  It intentionally returns no
-- public preview and no semantic digest: qmodel is the sole authority for both.
CREATE FUNCTION public.ple_curriculum_adoption_source_fact_v1(
    p_tenant uuid, p_session character(64), p_source jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_reference integer; v_document jsonb;
BEGIN
    PERFORM public.ple_curriculum_adoption_closed_object_v1(p_source, ARRAY['reference','revision'], 4096);
    IF NOT p_source ?& ARRAY['reference','revision']
       OR jsonb_typeof(p_source->'reference') <> 'string'
       OR jsonb_typeof(p_source->'revision') <> 'string' THEN
        RAISE EXCEPTION 'curriculum adoption source witness is invalid' USING ERRCODE = '22023';
    END IF;
    IF p_source->>'reference' ~ '^BP-' THEN
        v_reference := public.ple_curriculum_adoption_route_number_v1(p_source->'reference', 'BP');
        v_document := public.ple_get_curriculum_blueprint_v1(p_tenant, p_session, v_reference);
    ELSIF p_source->>'reference' ~ '^AC-' THEN
        v_reference := public.ple_curriculum_adoption_route_number_v1(p_source->'reference', 'AC');
        v_document := public.ple_get_curriculum_alpha_v1(p_tenant, p_session, v_reference);
    ELSE
        RAISE EXCEPTION 'curriculum adoption source witness is invalid' USING ERRCODE = '22023';
    END IF;
    IF v_document IS NULL OR v_document->>'revision' <> p_source->>'revision' THEN
        RAISE EXCEPTION 'curriculum adoption source witness is stale or unavailable'
            USING ERRCODE = 'PBC01';
    END IF;
    -- Do not derive a source snapshot hash from PostgreSQL's JSON rendering.
    -- The trusted Rust codec owns canonical bytes/digests; SQL returns only
    -- this revision-locked relational document for that codec to bind.
    RETURN jsonb_build_object('source', p_source, 'sourceSnapshot', v_document);
END $$;

CREATE FUNCTION public.ple_snapshot_curriculum_adoption_v1(
    p_tenant uuid, p_session character(64), p_operation jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_actor uuid; v_kind text; v_request jsonb; v_preparation uuid;
DECLARE v_course uuid; v_assignment uuid; v_facts jsonb := '{}'::jsonb;
DECLARE v_request_sha256 bytea;
BEGIN
    v_actor := public.ple_curriculum_adoption_actor_v1(p_tenant, p_session);
    v_kind := public.ple_curriculum_adoption_bridge_operation_v1(p_operation);
    v_request := p_operation->'request';
    -- Relational source and destination reads use explicit public locators only.
    IF v_kind = 'inspectImports' THEN
        v_course := public.ple_curriculum_adoption_lock_course_v1(p_tenant, v_actor, v_request);
        v_facts := v_facts || jsonb_build_object('courseId', v_course);
    ELSIF v_request ? 'course' THEN
        v_course := public.ple_curriculum_adoption_lock_course_v1(p_tenant, v_actor, v_request->'course');
        v_facts := v_facts || jsonb_build_object('courseId', v_course);
    ELSIF v_request ? 'witness' THEN
        v_course := public.ple_curriculum_adoption_lock_course_v1(p_tenant, v_actor, v_request#>'{witness,course}');
        v_facts := v_facts || jsonb_build_object('courseId', v_course);
    ELSIF v_request ? 'previewWitness' THEN
        v_course := public.ple_curriculum_adoption_lock_course_v1(p_tenant, v_actor, v_request#>'{previewWitness,course}');
        v_facts := v_facts || jsonb_build_object('courseId', v_course);
    END IF;
    IF v_request ? 'assignment' THEN
        IF v_course IS NULL THEN RAISE EXCEPTION 'curriculum adoption assignment has no course witness' USING ERRCODE = '22023'; END IF;
        v_assignment := public.ple_curriculum_adoption_lock_assignment_v1(p_tenant, v_course, v_request#>'{assignment,assignment}');
        v_facts := v_facts || jsonb_build_object('assignmentId', v_assignment);
    END IF;
    IF v_request ? 'source' AND jsonb_typeof(v_request->'source') = 'object'
       AND v_request->'source' ? 'reference' THEN
        v_facts := v_facts || public.ple_curriculum_adoption_source_fact_v1(
            p_tenant, p_session, v_request->'source'
        );
    END IF;
    IF v_kind IN (
        'applyForkAlpha', 'applyBlueprintInstantiation', 'applyAlphaInstantiation',
        'applyCourseRollover', 'applyCourseTermShift', 'applyAssignmentFastForward',
        'createSourceDerivedAssignment'
    ) THEN
        PERFORM public.ple_curriculum_adoption_prepare_temp_v1();
        v_preparation := gen_random_uuid();
        v_request_sha256 := public.ple_cam_digest_bytes_v1(
            p_operation#>'{materializationBinding,requestSha256}'
        );
        IF public.ple_cam_uuid_v1(p_operation#>'{materializationBinding,actor}')
           IS DISTINCT FROM v_actor THEN
            RAISE EXCEPTION 'curriculum adoption materialization binding is unavailable'
                USING ERRCODE = 'PBC01';
        END IF;
        INSERT INTO pg_temp.ple_curriculum_adoption_materialization_preparation (
            preparation_id, tenant_id, actor_user_id, operation, request, facts, request_sha256
        ) VALUES (
            v_preparation, p_tenant, v_actor, v_kind, v_request, v_facts, v_request_sha256
        );
        -- The adapter converts these relational facts into the exact closed
        -- qmodel preparation.  Do not mint semantic input in SQL.
        RAISE EXCEPTION 'curriculum adoption relational facts require the canonical qmodel snapshot compiler'
            USING ERRCODE = 'PBI01';
    ELSIF v_kind = 'reconcile' THEN
        PERFORM public.ple_curriculum_adoption_prepare_temp_v1();
        v_preparation := gen_random_uuid();
        INSERT INTO pg_temp.ple_curriculum_adoption_reconciliation_preparation (
            preparation_id, tenant_id, actor_user_id, request, facts
        ) VALUES (
            v_preparation, p_tenant, v_actor, v_request, v_facts
        );
        RAISE EXCEPTION 'curriculum adoption relational facts require the canonical qmodel snapshot compiler'
            USING ERRCODE = 'PBI01';
    END IF;
    RAISE EXCEPTION 'curriculum adoption public preview requires the canonical qmodel snapshot compiler'
        USING ERRCODE = 'PBI01';
END $$;

CREATE FUNCTION public.ple_materialize_curriculum_adoption_v1(
    p_tenant uuid, p_session character(64), p_preparation uuid, p_plan jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    -- Migration 1847 replaces this retired facade with the exact tagged
    -- materialization/reconciliation union.  The migration-1840 flat plan is
    -- never a compatibility wire and cannot select either new temp contract.
    RAISE EXCEPTION 'legacy curriculum adoption materialization plan is invalid'
        USING ERRCODE = '22023';
END $$;

ALTER FUNCTION public.ple_curriculum_adoption_closed_object_v1(jsonb, text[], integer)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_curriculum_adoption_route_number_v1(jsonb, text)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_curriculum_adoption_bridge_operation_v1(jsonb)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_curriculum_adoption_prepare_temp_v1()
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_curriculum_adoption_lock_course_v1(uuid, uuid, jsonb)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_curriculum_adoption_lock_assignment_v1(uuid, uuid, jsonb)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_curriculum_adoption_source_fact_v1(uuid, character, jsonb)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_snapshot_curriculum_adoption_v1(uuid, character, jsonb)
    OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_materialize_curriculum_adoption_v1(uuid, character, uuid, jsonb)
    OWNER TO ple_curriculum_adoption_broker;

REVOKE ALL ON FUNCTION public.ple_curriculum_adoption_closed_object_v1(jsonb, text[], integer),
    public.ple_curriculum_adoption_route_number_v1(jsonb, text),
    public.ple_curriculum_adoption_bridge_operation_v1(jsonb),
    public.ple_curriculum_adoption_prepare_temp_v1(),
    public.ple_curriculum_adoption_lock_course_v1(uuid, uuid, jsonb),
    public.ple_curriculum_adoption_lock_assignment_v1(uuid, uuid, jsonb),
    public.ple_curriculum_adoption_source_fact_v1(uuid, character, jsonb)
    FROM PUBLIC, ple_app, ple_curriculum_adoption_broker;
REVOKE ALL ON FUNCTION public.ple_snapshot_curriculum_adoption_v1(uuid, character, jsonb),
    public.ple_materialize_curriculum_adoption_v1(uuid, character, uuid, jsonb)
    FROM PUBLIC;
-- Security-definer snapshot and materializer functions compose only these
-- private helpers as their broker owner.  No application principal receives
-- the same helper access.
GRANT EXECUTE ON FUNCTION public.ple_curriculum_adoption_closed_object_v1(jsonb, text[], integer),
    public.ple_curriculum_adoption_route_number_v1(jsonb, text),
    public.ple_curriculum_adoption_bridge_operation_v1(jsonb),
    public.ple_curriculum_adoption_prepare_temp_v1(),
    public.ple_curriculum_adoption_lock_course_v1(uuid, uuid, jsonb),
    public.ple_curriculum_adoption_lock_assignment_v1(uuid, uuid, jsonb),
    public.ple_curriculum_adoption_source_fact_v1(uuid, character, jsonb)
    TO ple_curriculum_adoption_broker;
GRANT EXECUTE ON FUNCTION public.ple_snapshot_curriculum_adoption_v1(uuid, character, jsonb),
    public.ple_materialize_curriculum_adoption_v1(uuid, character, uuid, jsonb)
    TO ple_app;

-- The bridge role remains execute-only.  These assertions deliberately fail
-- migration installation if a future edit opens a table, sequence, or helper.
DO $$
BEGIN
    IF has_table_privilege('ple_app', 'public.curriculum_adoption_receipt', 'SELECT,INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_curriculum_adoption_broker', 'public.course', 'INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_curriculum_adoption_broker', 'public.assignment', 'INSERT,UPDATE,DELETE')
       OR has_function_privilege('ple_app', 'public.ple_curriculum_adoption_lock_course_v1(uuid,uuid,jsonb)', 'EXECUTE')
       OR NOT has_function_privilege('ple_app', 'public.ple_snapshot_curriculum_adoption_v1(uuid,character,jsonb)', 'EXECUTE')
       OR NOT has_function_privilege('ple_app', 'public.ple_materialize_curriculum_adoption_v1(uuid,character,uuid,jsonb)', 'EXECUTE')
       OR NOT has_function_privilege('ple_curriculum_adoption_broker', 'public.ple_curriculum_adoption_prepare_temp_v1()', 'EXECUTE')
       OR NOT has_function_privilege('ple_curriculum_adoption_broker', 'public.ple_curriculum_adoption_bridge_operation_v1(jsonb)', 'EXECUTE')
       OR NOT has_function_privilege('ple_curriculum_adoption_broker', 'public.ple_curriculum_adoption_source_fact_v1(uuid,character,jsonb)', 'EXECUTE') THEN
        RAISE EXCEPTION 'curriculum adoption execute-only bridge grants are unsafe';
    END IF;
END $$;

COMMIT;
