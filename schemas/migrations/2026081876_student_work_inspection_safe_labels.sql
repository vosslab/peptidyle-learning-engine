-- WP-INST-G2 / G2-W3B: current safe presentation labels for inspected work.
--
-- The broker remains the sole application-executable private-response reader.
-- Labels are current route presentation metadata, repeated in its transient
-- rowset for Rust agreement checks; neither label enters the audit payload.

BEGIN;

ALTER TABLE public.course_roster_profile FORCE ROW LEVEL SECURITY;
GRANT SELECT ON public.course_roster_profile TO ple_student_work_inspection_broker;
CREATE POLICY student_work_inspection_broker_profile
    ON public.course_roster_profile FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());

DROP FUNCTION public.ple_inspect_student_work_v1(
    uuid, character, integer, integer, integer, integer
);

CREATE FUNCTION public.ple_inspect_student_work_v1(
    p_tenant_id uuid,
    p_session character(64),
    p_course_reference integer,
    p_membership_reference integer,
    p_assignment_reference integer,
    p_run_reference integer
) RETURNS TABLE (
    attempt_id uuid,
    assignment_position integer,
    submitted_at_millis bigint,
    response_canonical_json text,
    response_sha256 character(64),
    canonical_json_version smallint,
    receipt_attempt_canonical_json text,
    receipt_attempt_payload jsonb,
    receipt_attempt_payload_sha256 character(64),
    presentation_canonical_json text,
    presentation_payload jsonb,
    presentation_payload_sha256 character(64),
    presentation_required boolean,
    issued_presentation_digest bytea,
    presentation_capability text,
    scoring_generation bigint,
    scoring_status text,
    score_visible boolean,
    correctness_visible boolean,
    student_display_label text,
    assignment_title text
)
LANGUAGE plpgsql
VOLATILE
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    v_course uuid;
    v_membership uuid;
    v_assignment uuid;
    v_run uuid;
    v_actor uuid;
    v_student_display_label text;
    v_assignment_title text;
    v_payload jsonb;
    v_rows jsonb;
    v_digest character(64);
    v_access_id uuid := pg_catalog.gen_random_uuid();
    v_audit_id uuid := pg_catalog.gen_random_uuid();
    v_occurred_at timestamptz := transaction_timestamp();
    v_max_submissions constant integer := 9216;
BEGIN
    IF p_tenant_id IS NULL OR p_session IS NULL
       OR p_course_reference IS NULL OR p_course_reference <= 0
       OR p_membership_reference IS NULL OR p_membership_reference <= 0
       OR p_assignment_reference IS NULL OR p_assignment_reference <= 0
       OR p_run_reference IS NULL OR p_run_reference <= 0
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant()
    THEN
        RETURN;
    END IF;

    SELECT course_id INTO v_course
      FROM public.course
     WHERE tenant_id = p_tenant_id AND public_id = p_course_reference
       AND public.ple_course_records_accessible(tenant_id, course_id);
    IF NOT FOUND THEN RETURN; END IF;
    v_actor := public.ple_course_roster_actor(p_session, v_course, true);
    IF v_actor IS NULL THEN RETURN; END IF;

    SELECT membership.course_membership_id, profile.display_name
      INTO v_membership, v_student_display_label
      FROM public.course_member AS membership
      JOIN public.course_roster_profile AS profile
        ON profile.tenant_id = membership.tenant_id
       AND profile.course_id = membership.course_id
       AND profile.course_membership_id = membership.course_membership_id
     WHERE membership.tenant_id = p_tenant_id
       AND membership.course_id = v_course
       AND membership.public_id = p_membership_reference
       AND membership.role = 'student' AND membership.status = 'active';
    SELECT assignment_id, title INTO v_assignment, v_assignment_title
      FROM public.assignment
     WHERE tenant_id = p_tenant_id AND course_id = v_course
       AND public_id = p_assignment_reference;
    SELECT run.run_id INTO v_run
      FROM public.assignment_run AS run
      JOIN public.enrollment AS enrollment
        ON enrollment.tenant_id = run.tenant_id
       AND enrollment.enrollment_id = run.enrollment_id
      JOIN public.course_member AS member
        ON member.tenant_id = enrollment.tenant_id
       AND member.course_id = v_course AND member.user_id = enrollment.user_id
     WHERE run.tenant_id = p_tenant_id AND run.public_id = p_run_reference
       AND enrollment.assignment_id = v_assignment
       AND member.course_membership_id = v_membership
       AND member.role = 'student' AND member.status = 'active';
    IF v_membership IS NULL OR v_assignment IS NULL OR v_run IS NULL
       OR v_student_display_label IS NULL
       OR char_length(v_student_display_label) NOT BETWEEN 1 AND 200
       OR v_student_display_label <> btrim(v_student_display_label)
       OR v_assignment_title IS NULL
       OR char_length(v_assignment_title) NOT BETWEEN 1 AND 200
       OR v_assignment_title <> btrim(v_assignment_title)
    THEN
        RETURN;
    END IF;

    SELECT jsonb_agg(jsonb_build_object(
        'attempt_id', witness.attempt_id::text,
        'assignment_position', witness.assignment_position,
        'submitted_at_millis', floor(extract(epoch FROM witness.submitted_at) * 1000)::bigint,
        'response_canonical_json', witness.response_canonical_json,
        'response_sha256', witness.response_sha256,
        'canonical_json_version', witness.canonical_json_version,
        'receipt_attempt_canonical_json', witness.receipt_attempt_canonical_json,
        'receipt_attempt_payload', witness.receipt_attempt_payload,
        'receipt_attempt_payload_sha256', witness.receipt_attempt_payload_sha256,
        'presentation_canonical_json', witness.presentation_canonical_json,
        'presentation_payload', witness.presentation_payload,
        'presentation_payload_sha256', witness.presentation_payload_sha256,
        'presentation_required', witness.presentation_required,
        'issued_presentation_digest', encode(witness.issued_presentation_digest, 'hex'),
        'presentation_capability', witness.presentation_capability,
        'scoring_generation', witness.scoring_generation,
        'scoring_status', witness.scoring_status,
        'score_visible', CASE witness.score_disclosure
            WHEN 'during_attempt' THEN true WHEN 'after_submit' THEN true
            WHEN 'after_due' THEN witness.resolved_due_at IS NOT NULL
                AND v_occurred_at >= witness.resolved_due_at
            WHEN 'after_close' THEN witness.resolved_closes_at IS NOT NULL
                AND v_occurred_at >= witness.resolved_closes_at
            ELSE false END,
        'correctness_visible', CASE witness.per_item_correctness_disclosure
            WHEN 'during_attempt' THEN true WHEN 'after_submit' THEN true
            WHEN 'after_due' THEN witness.resolved_due_at IS NOT NULL
                AND v_occurred_at >= witness.resolved_due_at
            WHEN 'after_close' THEN witness.resolved_closes_at IS NOT NULL
                AND v_occurred_at >= witness.resolved_closes_at
            ELSE false END,
        'student_display_label', v_student_display_label,
        'assignment_title', v_assignment_title
    ) ORDER BY witness.submitted_at, witness.assignment_position, witness.attempt_id)
    INTO v_rows
    FROM (
        SELECT source.*
        FROM public.ple_student_work_inspection_witness_v1 AS source
        WHERE source.tenant_id = p_tenant_id AND source.course_id = v_course
          AND source.course_membership_id = v_membership
          AND source.assignment_id = v_assignment AND source.run_id = v_run
          AND source.retention_lifecycle = 'active'
          AND ((source.presentation_required
                AND source.presentation_capability = 'envelope_v1'
                AND source.issued_presentation_digest IS NOT NULL
                AND octet_length(source.issued_presentation_digest) = 32)
            OR (NOT source.presentation_required
                AND source.presentation_capability = 'not_applicable'
                AND source.response_canonical_json::jsonb = '{"kind":"externalTool"}'::jsonb))
        ORDER BY source.submitted_at, source.assignment_position, source.attempt_id
        LIMIT v_max_submissions + 1
    ) AS witness;
    IF v_rows IS NULL OR jsonb_array_length(v_rows) < 1
       OR jsonb_array_length(v_rows) > v_max_submissions THEN
        RETURN;
    END IF;

    SELECT jsonb_build_object(
        'purpose', 'gradebook_inspection',
        'actorId', v_actor::text,
        'membershipId', v_membership::text,
        'assignmentId', v_assignment::text,
        'runId', v_run::text,
        'submissions', (SELECT jsonb_agg(jsonb_build_object(
            'attemptId', entry->>'attempt_id',
            'submittedAtMillis', (entry->>'submitted_at_millis')::bigint,
            'evidence', CASE WHEN (entry->>'presentation_required')::boolean
                THEN 'issued_presentation' ELSE 'presentation_not_applicable' END,
            'presentationDigest', CASE WHEN (entry->>'presentation_required')::boolean
                THEN entry->>'issued_presentation_digest' ELSE NULL END
        ) ORDER BY (entry->>'submitted_at_millis')::bigint,
                   (entry->>'assignment_position')::integer, entry->>'attempt_id')
        FROM jsonb_array_elements(v_rows) AS entries(entry))
    ) INTO v_payload;
    v_digest := encode(pg_catalog.sha256(convert_to(v_payload::text, 'UTF8')), 'hex');

    INSERT INTO public.record_access_log (
        tenant_id, access_log_id, occurred_at, payload, payload_sha256,
        delivery_scope, delivery_id, course_id
    ) VALUES (
        p_tenant_id, v_access_id, v_occurred_at, v_payload, v_digest,
        'student_record', v_run, v_course
    );
    INSERT INTO public.audit_event (
        tenant_id, audit_event_id, occurred_at, actor_id, course_id,
        action, target_kind, target_id, payload, payload_sha256
    ) VALUES (
        p_tenant_id, v_audit_id, v_occurred_at, v_actor, v_course,
        'gradebook_inspection', 'student_work_inspection', v_run, v_payload, v_digest
    );

    RETURN QUERY SELECT decoded.attempt_id, decoded.assignment_position,
        decoded.submitted_at_millis, decoded.response_canonical_json,
        decoded.response_sha256, decoded.canonical_json_version,
        decoded.receipt_attempt_canonical_json, decoded.receipt_attempt_payload,
        decoded.receipt_attempt_payload_sha256, decoded.presentation_canonical_json,
        decoded.presentation_payload, decoded.presentation_payload_sha256,
        decoded.presentation_required, decode(decoded.issued_presentation_digest, 'hex'),
        decoded.presentation_capability, decoded.scoring_generation,
        decoded.scoring_status, decoded.score_visible, decoded.correctness_visible,
        decoded.student_display_label, decoded.assignment_title
    FROM jsonb_to_recordset(v_rows) AS decoded(
        attempt_id uuid, assignment_position integer, submitted_at_millis bigint,
        response_canonical_json text, response_sha256 character(64),
        canonical_json_version smallint, receipt_attempt_canonical_json text,
        receipt_attempt_payload jsonb, receipt_attempt_payload_sha256 character(64),
        presentation_canonical_json text, presentation_payload jsonb,
        presentation_payload_sha256 character(64), presentation_required boolean,
        issued_presentation_digest text, presentation_capability text,
        scoring_generation bigint, scoring_status text, score_visible boolean,
        correctness_visible boolean, student_display_label text, assignment_title text
    ) ORDER BY decoded.submitted_at_millis, decoded.assignment_position, decoded.attempt_id;
END;
$$;

ALTER FUNCTION public.ple_inspect_student_work_v1(
    uuid, character, integer, integer, integer, integer
) OWNER TO ple_student_work_inspection_broker;
REVOKE ALL ON FUNCTION public.ple_inspect_student_work_v1(
    uuid, character, integer, integer, integer, integer
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_inspect_student_work_v1(
    uuid, character, integer, integer, integer, integer
) TO ple_app;

DO $$
DECLARE
    v_function regprocedure :=
        'public.ple_inspect_student_work_v1(uuid,character,integer,integer,integer,integer)'::regprocedure;
    v_broker oid := 'ple_student_work_inspection_broker'::regrole::oid;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_proc AS procedure_row
        WHERE procedure_row.oid = v_function
          AND procedure_row.proowner = 'ple_student_work_inspection_broker'::regrole
          AND procedure_row.prosecdef
          AND procedure_row.proconfig IS NOT DISTINCT FROM ARRAY[
              'search_path=pg_catalog, public, pg_temp'
          ]::text[]
    ) THEN
        RAISE EXCEPTION 'student-work inspection broker catalog is unsafe';
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_proc AS procedure_row
        CROSS JOIN LATERAL pg_catalog.aclexplode(COALESCE(
            procedure_row.proacl, pg_catalog.acldefault('f', procedure_row.proowner)
        )) AS privilege
        WHERE procedure_row.oid = v_function
          AND privilege.grantee <> procedure_row.proowner
          AND (privilege.grantee <> 'ple_app'::regrole::oid
               OR privilege.privilege_type <> 'EXECUTE' OR privilege.is_grantable)
    ) THEN
        RAISE EXCEPTION 'student-work inspection broker execute ACL is unsafe';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM pg_catalog.pg_class AS relation
        WHERE relation.oid = 'public.course_roster_profile'::regclass
          AND relation.relrowsecurity AND relation.relforcerowsecurity
    ) THEN
        RAISE EXCEPTION 'student-work inspection profile RLS is unsafe';
    END IF;
    IF (
        SELECT count(*)
        FROM pg_catalog.pg_class AS relation
        CROSS JOIN LATERAL pg_catalog.aclexplode(COALESCE(
            relation.relacl, pg_catalog.acldefault('r', relation.relowner)
        )) AS privilege
        WHERE relation.oid = 'public.course_roster_profile'::regclass
          AND privilege.grantee = v_broker
    ) <> 1 OR EXISTS (
        SELECT 1
        FROM pg_catalog.pg_class AS relation
        CROSS JOIN LATERAL pg_catalog.aclexplode(COALESCE(
            relation.relacl, pg_catalog.acldefault('r', relation.relowner)
        )) AS privilege
        WHERE relation.oid = 'public.course_roster_profile'::regclass
          AND privilege.grantee = v_broker
          AND (privilege.privilege_type <> 'SELECT' OR privilege.is_grantable)
    ) THEN
        RAISE EXCEPTION 'student-work inspection profile ACL is unsafe';
    END IF;
    IF (
        SELECT count(*)
        FROM pg_catalog.pg_policy AS policy
        WHERE policy.polrelid = 'public.course_roster_profile'::regclass
          AND policy.polcmd = 'r'
          AND (v_broker = ANY(policy.polroles) OR 0::oid = ANY(policy.polroles))
    ) <> 1 OR NOT EXISTS (
        SELECT 1
        FROM pg_catalog.pg_policy AS policy
        WHERE policy.polrelid = 'public.course_roster_profile'::regclass
          AND policy.polname = 'student_work_inspection_broker_profile'
          AND policy.polcmd = 'r'
          AND policy.polpermissive
          AND policy.polroles = ARRAY[v_broker]
          AND pg_catalog.pg_get_expr(policy.polqual, policy.polrelid)
              = '(tenant_id = ple_current_tenant())'
          AND policy.polwithcheck IS NULL
    ) THEN
        RAISE EXCEPTION 'student-work inspection profile policy is unsafe';
    END IF;
END;
$$;

COMMIT;
