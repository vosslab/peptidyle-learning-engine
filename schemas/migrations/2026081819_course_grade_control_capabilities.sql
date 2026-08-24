-- Session-bound live whole-course grade-control capabilities.
-- ASVS 1.2.4, 2.2.1-2.2.3, 2.3.1-2.3.4, 8.2.1-8.2.3, 15.4.2-15.4.3.
BEGIN;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_course_grade_control_broker') THEN
        CREATE ROLE ple_course_grade_control_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
            NOINHERIT NOREPLICATION NOBYPASSRLS;
    END IF;
END
$$;
ALTER ROLE ple_course_grade_control_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOINHERIT NOREPLICATION NOBYPASSRLS;
ALTER ROLE ple_course_grade_control_broker SET search_path TO 'pg_catalog', 'public', 'pg_temp';
REVOKE ALL ON SCHEMA public FROM ple_course_grade_control_broker;
GRANT USAGE ON SCHEMA public TO ple_course_grade_control_broker;

-- The public product has exactly two aggregate operations.  It never mutates
-- individual scheme rows or inserts an audit directly.
REVOKE INSERT, UPDATE, DELETE ON public.course_grade_scheme,
    public.course_grade_category, public.course_grade_category_assignment,
    public.course_grade_letter_band, public.course_total_export_audit FROM ple_app;

CREATE POLICY course_grade_control_course_tenant ON public.course
    TO ple_course_grade_control_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_grade_control_session_tenant ON public.auth_session
    TO ple_course_grade_control_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_grade_control_member_tenant ON public.course_member
    TO ple_course_grade_control_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_grade_control_scheme_tenant ON public.course_grade_scheme
    TO ple_course_grade_control_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY course_grade_control_category_tenant ON public.course_grade_category
    TO ple_course_grade_control_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY course_grade_control_category_assignment_tenant
    ON public.course_grade_category_assignment TO ple_course_grade_control_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY course_grade_control_letter_band_tenant ON public.course_grade_letter_band
    TO ple_course_grade_control_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY course_grade_control_assignment_tenant ON public.assignment
    TO ple_course_grade_control_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_grade_control_export_audit_tenant ON public.course_total_export_audit
    FOR INSERT TO ple_course_grade_control_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());

-- SELECT FOR UPDATE requires an UPDATE privilege even when no course/member
-- value changes.  Narrow column grants preserve that locking-only use.
GRANT SELECT, UPDATE (course_id) ON public.course TO ple_course_grade_control_broker;
GRANT SELECT, UPDATE (session_hash) ON public.auth_session TO ple_course_grade_control_broker;
GRANT SELECT, UPDATE (course_membership_id) ON public.course_member
    TO ple_course_grade_control_broker;
GRANT SELECT, UPDATE ON public.course_grade_scheme TO ple_course_grade_control_broker;
GRANT SELECT, INSERT, DELETE, UPDATE (category_id) ON public.course_grade_category
    TO ple_course_grade_control_broker;
GRANT SELECT, INSERT, DELETE, UPDATE (assignment_id) ON public.course_grade_category_assignment
    TO ple_course_grade_control_broker;
GRANT SELECT, INSERT, DELETE, UPDATE (letter_band_id) ON public.course_grade_letter_band
    TO ple_course_grade_control_broker;
GRANT SELECT, UPDATE (gradebook_included) ON public.assignment
    TO ple_course_grade_control_broker;
GRANT INSERT ON public.course_total_export_audit TO ple_course_grade_control_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(), public.ple_course_records_accessible(uuid, uuid)
    TO ple_course_grade_control_broker;

CREATE FUNCTION public.ple_course_grade_control_invalid() RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
AS $$
BEGIN
    RAISE EXCEPTION 'course grade-control capability arguments are invalid' USING ERRCODE = '22023';
END
$$;

CREATE FUNCTION public.ple_course_grade_control_require_instructor(
    p_tenant uuid, p_session character(64), p_course uuid
) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
AS $$
DECLARE v_actor uuid;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL OR p_course IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        PERFORM public.ple_course_grade_control_invalid();
    END IF;
    PERFORM 1 FROM public.course AS course_row
     WHERE course_row.tenant_id = p_tenant AND course_row.course_id = p_course
       AND public.ple_course_records_accessible(course_row.tenant_id, course_row.course_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course grade-control target is unavailable' USING ERRCODE = '42501';
    END IF;
    SELECT session_row.user_id INTO v_actor
      FROM public.auth_session AS session_row
     WHERE session_row.tenant_id = p_tenant AND session_row.session_hash = p_session
       AND session_row.revoked_at IS NULL AND session_row.expires_at > transaction_timestamp()
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course grade-control actor is unavailable' USING ERRCODE = '42501';
    END IF;
    PERFORM 1 FROM public.course_member AS member_row
     WHERE member_row.tenant_id = p_tenant AND member_row.course_id = p_course
       AND member_row.user_id = v_actor AND member_row.role = 'instructor'
       AND member_row.status = 'active'
     ORDER BY member_row.course_membership_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'active direct Instructor authority is required' USING ERRCODE = '42501';
    END IF;
    RETURN v_actor;
END
$$;

CREATE FUNCTION public.ple_replace_course_grade_scheme_v1(
    p_tenant uuid, p_session character(64), p_course uuid, p_expected_revision bigint,
    p_replacement jsonb
) RETURNS TABLE(tenant_id uuid, actor_id uuid, course_id uuid, scheme_revision bigint,
                mode text, rounding text)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
AS $$
#variable_conflict use_column
DECLARE v_actor uuid; v_current bigint; v_mode text; v_rounding text;
DECLARE v_category jsonb; v_assignment jsonb; v_band jsonb;
DECLARE v_category_id uuid; v_assignment_id uuid; v_position integer; v_seen integer := 0;
BEGIN
    IF p_expected_revision IS NULL OR p_expected_revision < 1 OR p_replacement IS NULL
       OR jsonb_typeof(p_replacement) <> 'object' OR octet_length(p_replacement::text) > 262144
       OR EXISTS (SELECT 1 FROM jsonb_object_keys(p_replacement) AS key
                  WHERE key NOT IN ('mode', 'rounding', 'categories', 'assignments', 'letterBands'))
       OR NOT (p_replacement ?& ARRAY['mode', 'rounding', 'categories', 'assignments', 'letterBands'])
       OR jsonb_typeof(p_replacement->'mode') <> 'string'
       OR jsonb_typeof(p_replacement->'rounding') <> 'string'
       OR jsonb_typeof(p_replacement->'categories') <> 'array'
       OR jsonb_typeof(p_replacement->'assignments') <> 'array'
       OR jsonb_typeof(p_replacement->'letterBands') <> 'array' THEN
        PERFORM public.ple_course_grade_control_invalid();
    END IF;
    v_mode := p_replacement->>'mode';
    v_rounding := p_replacement->>'rounding';
    IF v_mode NOT IN ('total_points', 'weighted_categories')
       OR v_rounding <> 'four_decimal_places_half_away_from_zero' THEN
        PERFORM public.ple_course_grade_control_invalid();
    END IF;
    v_actor := public.ple_course_grade_control_require_instructor(p_tenant, p_session, p_course);
    SELECT scheme.revision INTO v_current FROM public.course_grade_scheme AS scheme
     WHERE scheme.tenant_id = p_tenant AND scheme.course_id = p_course FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'course grade scheme is unavailable' USING ERRCODE = '42501'; END IF;
    -- A stale caller precondition is a deterministic product conflict, not a
    -- PostgreSQL serialization failure eligible for transparent retry.
    IF v_current <> p_expected_revision THEN RAISE EXCEPTION 'stale course grade scheme revision' USING ERRCODE = '55000'; END IF;
    -- Lock all current aggregate dependents before comparing the closed replacement.
    PERFORM assignment_row.assignment_id FROM public.assignment AS assignment_row
     WHERE assignment_row.tenant_id = p_tenant AND assignment_row.course_id = p_course
     ORDER BY assignment_row.assignment_id FOR UPDATE;
    PERFORM category_row.category_id FROM public.course_grade_category AS category_row
     WHERE category_row.tenant_id = p_tenant AND category_row.course_id = p_course
     ORDER BY category_row.category_id FOR UPDATE;
    PERFORM membership_row.assignment_id FROM public.course_grade_category_assignment AS membership_row
     WHERE membership_row.tenant_id = p_tenant AND membership_row.course_id = p_course
     ORDER BY membership_row.category_id, membership_row.assignment_id FOR UPDATE;
    PERFORM band_row.letter_band_id FROM public.course_grade_letter_band AS band_row
     WHERE band_row.tenant_id = p_tenant AND band_row.course_id = p_course
     ORDER BY band_row.letter_band_id FOR UPDATE;
    IF jsonb_array_length(p_replacement->'assignments') <>
       (SELECT count(*) FROM public.assignment WHERE tenant_id=p_tenant AND course_id=p_course) THEN
        PERFORM public.ple_course_grade_control_invalid();
    END IF;
    -- Validate every supplied assignment before deleting dependent rows.  UUID casts and
    -- relational constraints are deliberately inside this one transaction.
    FOR v_assignment IN SELECT value FROM jsonb_array_elements(p_replacement->'assignments') LOOP
        IF jsonb_typeof(v_assignment) <> 'object'
           OR EXISTS (SELECT 1 FROM jsonb_object_keys(v_assignment) AS key
                      WHERE key NOT IN ('assignmentId', 'included', 'categoryId', 'position'))
           OR NOT (v_assignment ?& ARRAY['assignmentId', 'included', 'categoryId', 'position'])
           OR jsonb_typeof(v_assignment->'assignmentId') <> 'string'
           OR jsonb_typeof(v_assignment->'included') <> 'boolean' THEN
            PERFORM public.ple_course_grade_control_invalid();
        END IF;
        v_assignment_id := (v_assignment->>'assignmentId')::uuid;
        SELECT count(*) INTO v_seen FROM public.assignment
         WHERE tenant_id=p_tenant AND course_id=p_course AND assignment_id=v_assignment_id;
        IF v_seen <> 1 OR (SELECT count(*) FROM jsonb_array_elements(p_replacement->'assignments') AS entry
                            WHERE entry->>'assignmentId'=v_assignment->>'assignmentId') <> 1 THEN
            PERFORM public.ple_course_grade_control_invalid();
        END IF;
    END LOOP;
    IF v_mode = 'total_points' AND (
        jsonb_array_length(p_replacement->'categories') <> 0
        OR EXISTS (SELECT 1 FROM jsonb_array_elements(p_replacement->'assignments') AS entry
                   WHERE entry->'categoryId' <> 'null'::jsonb OR entry->'position' <> 'null'::jsonb)
    ) THEN PERFORM public.ple_course_grade_control_invalid(); END IF;
    DELETE FROM public.course_grade_category_assignment WHERE tenant_id=p_tenant AND course_id=p_course;
    DELETE FROM public.course_grade_letter_band WHERE tenant_id=p_tenant AND course_id=p_course;
    DELETE FROM public.course_grade_category WHERE tenant_id=p_tenant AND course_id=p_course;
    UPDATE public.course_grade_scheme SET mode=v_mode, rounding=v_rounding,
        revision=revision+1, updated_at=transaction_timestamp()
     WHERE tenant_id=p_tenant AND course_id=p_course AND revision=p_expected_revision
     RETURNING revision INTO scheme_revision;
    IF NOT FOUND THEN RAISE EXCEPTION 'stale course grade scheme revision' USING ERRCODE = '55000'; END IF;
    FOR v_category IN SELECT value FROM jsonb_array_elements(p_replacement->'categories') LOOP
        IF jsonb_typeof(v_category) <> 'object'
           OR EXISTS (SELECT 1 FROM jsonb_object_keys(v_category) AS key
                      WHERE key NOT IN ('id','position','title','weightBasisPoints','dropLowest'))
           OR NOT (v_category ?& ARRAY['id','position','title','weightBasisPoints','dropLowest'])
           OR jsonb_typeof(v_category->'id') <> 'string'
           OR jsonb_typeof(v_category->'position') <> 'number'
           OR jsonb_typeof(v_category->'title') <> 'string'
           OR jsonb_typeof(v_category->'weightBasisPoints') <> 'number'
           OR jsonb_typeof(v_category->'dropLowest') <> 'number' THEN PERFORM public.ple_course_grade_control_invalid(); END IF;
        v_category_id := (v_category->>'id')::uuid;
        INSERT INTO public.course_grade_category(tenant_id,course_id,category_id,position,title,weight_basis_points,drop_lowest)
        VALUES(p_tenant,p_course,v_category_id,(v_category->>'position')::integer,v_category->>'title',
               (v_category->>'weightBasisPoints')::integer,(v_category->>'dropLowest')::integer);
    END LOOP;
    FOR v_assignment IN SELECT value FROM jsonb_array_elements(p_replacement->'assignments') LOOP
        v_assignment_id := (v_assignment->>'assignmentId')::uuid;
        UPDATE public.assignment SET gradebook_included=(v_assignment->>'included')::boolean
         WHERE tenant_id=p_tenant AND course_id=p_course AND assignment_id=v_assignment_id;
        IF v_mode='weighted_categories' AND v_assignment->'categoryId' <> 'null'::jsonb THEN
            IF jsonb_typeof(v_assignment->'categoryId') <> 'string' OR jsonb_typeof(v_assignment->'position') <> 'number' THEN PERFORM public.ple_course_grade_control_invalid(); END IF;
            INSERT INTO public.course_grade_category_assignment(tenant_id,course_id,category_id,assignment_id,position)
            VALUES(p_tenant,p_course,(v_assignment->>'categoryId')::uuid,v_assignment_id,(v_assignment->>'position')::integer);
        ELSIF v_mode='weighted_categories' AND ((v_assignment->>'included')::boolean) THEN
            PERFORM public.ple_course_grade_control_invalid();
        ELSIF v_mode='weighted_categories' AND v_assignment->'position' <> 'null'::jsonb THEN
            PERFORM public.ple_course_grade_control_invalid();
        END IF;
    END LOOP;
    FOR v_band IN SELECT value FROM jsonb_array_elements(p_replacement->'letterBands') LOOP
        IF jsonb_typeof(v_band) <> 'object'
           OR EXISTS (SELECT 1 FROM jsonb_object_keys(v_band) AS key WHERE key NOT IN ('label','minimumBasisPoints'))
           OR NOT (v_band ?& ARRAY['label','minimumBasisPoints'])
           OR jsonb_typeof(v_band->'label') <> 'string' OR jsonb_typeof(v_band->'minimumBasisPoints') <> 'number' THEN PERFORM public.ple_course_grade_control_invalid(); END IF;
        INSERT INTO public.course_grade_letter_band(tenant_id,course_id,letter_band_id,label,minimum_basis_points)
        VALUES(p_tenant,p_course,gen_random_uuid(),v_band->>'label',(v_band->>'minimumBasisPoints')::integer);
    END LOOP;
    -- The normalized rows must reconstruct the same closed domain contract
    -- that the Rust boundary validated.  This check keeps the capability safe
    -- for every future caller, not just the present Store adapter.
    IF (v_mode='weighted_categories' AND (
        (SELECT count(*) FROM public.course_grade_category WHERE tenant_id=p_tenant AND course_id=p_course)=0
        OR (SELECT coalesce(sum(weight_basis_points),0) FROM public.course_grade_category WHERE tenant_id=p_tenant AND course_id=p_course)<>10000
        OR EXISTS (SELECT 1 FROM public.course_grade_category WHERE tenant_id=p_tenant AND course_id=p_course
                   AND position <> (SELECT count(*)-1 FROM public.course_grade_category AS earlier WHERE earlier.tenant_id=p_tenant AND earlier.course_id=p_course AND earlier.position<=course_grade_category.position))
        OR EXISTS (SELECT 1 FROM public.course_grade_category AS category_row WHERE category_row.tenant_id=p_tenant AND category_row.course_id=p_course
                   AND category_row.drop_lowest >= (SELECT count(*) FROM public.course_grade_category_assignment AS membership_row JOIN public.assignment AS assignment_row ON assignment_row.tenant_id=membership_row.tenant_id AND assignment_row.assignment_id=membership_row.assignment_id WHERE membership_row.tenant_id=p_tenant AND membership_row.course_id=p_course AND membership_row.category_id=category_row.category_id AND assignment_row.gradebook_included))
        OR EXISTS (SELECT 1 FROM public.course_grade_category_assignment AS membership_row WHERE membership_row.tenant_id=p_tenant AND membership_row.course_id=p_course
                   AND membership_row.position <> (SELECT count(*)-1 FROM public.course_grade_category_assignment AS earlier WHERE earlier.tenant_id=p_tenant AND earlier.course_id=p_course AND earlier.category_id=membership_row.category_id AND earlier.position<=membership_row.position))
    )) THEN
        PERFORM public.ple_course_grade_control_invalid();
    END IF;
    tenant_id:=p_tenant; actor_id:=v_actor; course_id:=p_course; mode:=v_mode; rounding:=v_rounding;
    RETURN NEXT;
END
$$;

CREATE FUNCTION public.ple_record_course_grade_export_audit_v1(
    p_tenant uuid, p_session character(64), p_course uuid, p_export uuid, p_row_count integer,
    p_scheme_revision bigint, p_mode text, p_rounding text
) RETURNS TABLE(tenant_id uuid, actor_id uuid, course_id uuid, export_id uuid,
                row_count integer, scheme_revision bigint, mode text, rounding text)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
AS $$
#variable_conflict use_column
DECLARE v_actor uuid;
BEGIN
    IF p_export IS NULL OR p_row_count NOT BETWEEN 0 AND 500 OR p_scheme_revision IS NULL
       OR p_scheme_revision < 1 OR p_mode NOT IN ('total_points','weighted_categories')
       OR p_rounding <> 'four_decimal_places_half_away_from_zero' THEN
        PERFORM public.ple_course_grade_control_invalid();
    END IF;
    v_actor := public.ple_course_grade_control_require_instructor(p_tenant,p_session,p_course);
    PERFORM 1 FROM public.course_grade_scheme AS scheme
     WHERE scheme.tenant_id=p_tenant AND scheme.course_id=p_course AND scheme.revision=p_scheme_revision
       AND scheme.mode=p_mode AND scheme.rounding=p_rounding FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'course grade export snapshot is stale' USING ERRCODE = '55000'; END IF;
    INSERT INTO public.course_total_export_audit(tenant_id,course_id,export_id,requested_by,row_count,scheme_revision,mode,rounding)
    VALUES(p_tenant,p_course,p_export,v_actor,p_row_count,p_scheme_revision,p_mode,p_rounding);
    tenant_id:=p_tenant; actor_id:=v_actor; course_id:=p_course; export_id:=p_export;
    row_count:=p_row_count; scheme_revision:=p_scheme_revision; mode:=p_mode; rounding:=p_rounding;
    RETURN NEXT;
END
$$;

ALTER FUNCTION public.ple_course_grade_control_invalid() OWNER TO ple_course_grade_control_broker;
ALTER FUNCTION public.ple_course_grade_control_require_instructor(uuid, character, uuid)
    OWNER TO ple_course_grade_control_broker;
ALTER FUNCTION public.ple_replace_course_grade_scheme_v1(uuid, character, uuid, bigint, jsonb)
    OWNER TO ple_course_grade_control_broker;
ALTER FUNCTION public.ple_record_course_grade_export_audit_v1(uuid, character, uuid, uuid, integer, bigint, text, text)
    OWNER TO ple_course_grade_control_broker;
REVOKE ALL ON FUNCTION public.ple_course_grade_control_invalid(),
    public.ple_course_grade_control_require_instructor(uuid, character, uuid),
    public.ple_replace_course_grade_scheme_v1(uuid, character, uuid, bigint, jsonb),
    public.ple_record_course_grade_export_audit_v1(uuid, character, uuid, uuid, integer, bigint, text, text)
    FROM PUBLIC, ple_app, ple_course_grade_control_broker;
GRANT EXECUTE ON FUNCTION public.ple_course_grade_control_invalid(),
    public.ple_course_grade_control_require_instructor(uuid, character, uuid)
    TO ple_course_grade_control_broker;
GRANT EXECUTE ON FUNCTION public.ple_replace_course_grade_scheme_v1(uuid, character, uuid, bigint, jsonb),
    public.ple_record_course_grade_export_audit_v1(uuid, character, uuid, uuid, integer, bigint, text, text)
    TO ple_app;

-- Fail closed during fresh migration if role drift or broad app writes reappear.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname='ple_course_grade_control_broker'
        AND (rolcanlogin OR rolsuper OR rolcreatedb OR rolcreaterole OR rolinherit OR rolreplication OR rolbypassrls))
       OR EXISTS (SELECT 1 FROM pg_auth_members WHERE member='ple_course_grade_control_broker'::regrole)
       OR has_table_privilege('ple_app','public.course_grade_scheme','INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_app','public.course_grade_category','INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_app','public.course_grade_category_assignment','INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_app','public.course_grade_letter_band','INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_app','public.course_total_export_audit','INSERT,UPDATE,DELETE')
       OR EXISTS (SELECT 1 FROM pg_proc AS procedure_row CROSS JOIN LATERAL aclexplode(COALESCE(procedure_row.proacl,acldefault('f',procedure_row.proowner))) AS privilege_row
                  WHERE procedure_row.oid='public.ple_replace_course_grade_scheme_v1(uuid,character,uuid,bigint,jsonb)'::regprocedure
                    AND privilege_row.grantee=0 AND privilege_row.privilege_type='EXECUTE')
       OR EXISTS (SELECT 1 FROM pg_proc AS procedure_row CROSS JOIN LATERAL aclexplode(COALESCE(procedure_row.proacl,acldefault('f',procedure_row.proowner))) AS privilege_row
                  WHERE procedure_row.oid='public.ple_record_course_grade_export_audit_v1(uuid,character,uuid,uuid,integer,bigint,text,text)'::regprocedure
                    AND privilege_row.grantee=0 AND privilege_row.privilege_type='EXECUTE') THEN
        RAISE EXCEPTION 'course grade-control privilege matrix is unsafe';
    END IF;
END
$$;
COMMIT;
