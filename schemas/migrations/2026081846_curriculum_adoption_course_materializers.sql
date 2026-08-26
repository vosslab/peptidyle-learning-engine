-- WP-PROF-B2: whole-course creation, rollover, and atomic term shifting.
BEGIN;

-- Term-shift materialization locks the exact witnessed course before invoking
-- the dedicated schedule writer.  The broker has no course UPDATE policy;
-- this key-column grant supplies only PostgreSQL's FOR UPDATE prerequisite.
GRANT UPDATE(course_id) ON public.course
    TO ple_curriculum_adoption_broker;

-- Only this broker can coalesce the intermediate schedule-trigger bumps of a
-- whole-course shift.  It is a relational capability, not a caller setting.
CREATE TABLE public.curriculum_adoption_schedule_coalescer (
    tenant_id uuid NOT NULL, course_id uuid NOT NULL,
    PRIMARY KEY (tenant_id, course_id),
    FOREIGN KEY (tenant_id, course_id) REFERENCES public.course (tenant_id, course_id)
        ON DELETE CASCADE
);
ALTER TABLE public.curriculum_adoption_schedule_coalescer ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.curriculum_adoption_schedule_coalescer FORCE ROW LEVEL SECURITY;
CREATE POLICY curriculum_adoption_schedule_coalescer_broker
    ON public.curriculum_adoption_schedule_coalescer FOR ALL TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
GRANT SELECT, INSERT, DELETE ON public.curriculum_adoption_schedule_coalescer
    TO ple_curriculum_adoption_broker;
REVOKE ALL ON public.curriculum_adoption_schedule_coalescer
    FROM PUBLIC, ple_app, ple_auth, ple_student, ple_grader, ple_grading_reader,
         ple_retention_broker, ple_curriculum_schedule_revision_broker;

-- A second NOLOGIN owner has only the columns needed to shift a witnessed
-- course schedule.  The adoption broker may invoke this one capability; it
-- retains no broad course or assignment mutation grant.
CREATE POLICY curriculum_schedule_shift_course ON public.course
    FOR SELECT TO ple_curriculum_schedule_revision_broker USING (tenant_id=public.ple_current_tenant());
CREATE POLICY curriculum_schedule_shift_course_update ON public.course
    FOR UPDATE TO ple_curriculum_schedule_revision_broker USING (tenant_id=public.ple_current_tenant())
    WITH CHECK (tenant_id=public.ple_current_tenant());
CREATE POLICY curriculum_schedule_shift_assignment ON public.assignment
    FOR SELECT TO ple_curriculum_schedule_revision_broker USING (tenant_id=public.ple_current_tenant());
CREATE POLICY curriculum_schedule_shift_assignment_update ON public.assignment
    FOR UPDATE TO ple_curriculum_schedule_revision_broker USING (tenant_id=public.ple_current_tenant())
    WITH CHECK (tenant_id=public.ple_current_tenant());
CREATE POLICY curriculum_schedule_shift_member ON public.course_member
    FOR SELECT TO ple_curriculum_schedule_revision_broker USING (tenant_id=public.ple_current_tenant());
CREATE POLICY curriculum_schedule_shift_member_lock ON public.course_member
    FOR UPDATE TO ple_curriculum_schedule_revision_broker
    USING (tenant_id=public.ple_current_tenant()) WITH CHECK (false);
CREATE POLICY curriculum_schedule_shift_policy ON public.assignment_effective_policy_base
    FOR SELECT TO ple_curriculum_schedule_revision_broker USING (tenant_id=public.ple_current_tenant());
CREATE POLICY curriculum_schedule_shift_policy_update ON public.assignment_effective_policy_base
    FOR UPDATE TO ple_curriculum_schedule_revision_broker USING (tenant_id=public.ple_current_tenant())
    WITH CHECK (tenant_id=public.ple_current_tenant());
GRANT SELECT, UPDATE(term_start_date,term_end_date,time_zone) ON public.course TO ple_curriculum_schedule_revision_broker;
GRANT SELECT ON public.course_member TO ple_curriculum_schedule_revision_broker;
GRANT UPDATE(course_membership_id) ON public.course_member TO ple_curriculum_schedule_revision_broker;
GRANT SELECT, UPDATE(revision,updated_at) ON public.assignment TO ple_curriculum_schedule_revision_broker;
GRANT SELECT, UPDATE(available_at,due_at,closes_at,updated_at) ON public.assignment_effective_policy_base TO ple_curriculum_schedule_revision_broker;

CREATE FUNCTION public.ple_cmc_apply_term_schedule_v1(
    p_tenant uuid,p_actor uuid,p_course uuid,p_term jsonb,p_rows jsonb
) RETURNS void LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE row jsonb; v_assignment_id uuid; expected bigint;
BEGIN
    IF p_tenant IS NULL OR p_actor IS NULL OR p_course IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR jsonb_typeof(p_rows)<>'array' THEN RAISE EXCEPTION 'term schedule capability is unavailable' USING ERRCODE='42501'; END IF;
    PERFORM 1 FROM public.course_member AS member WHERE member.tenant_id=p_tenant AND member.course_id=p_course
      AND member.user_id=p_actor AND member.role='instructor' AND member.status='active' FOR KEY SHARE;
    IF NOT FOUND THEN RAISE EXCEPTION 'term schedule capability is unavailable' USING ERRCODE='42501'; END IF;
    UPDATE public.course AS course_row SET term_start_date=(p_term->>'startDate')::date,term_end_date=(p_term->>'endDate')::date,time_zone=p_term->>'timeZone'
      WHERE course_row.tenant_id=p_tenant AND course_row.course_id=p_course;
    IF NOT FOUND THEN RAISE EXCEPTION 'term schedule course is unavailable' USING ERRCODE='PBC01'; END IF;
    FOR row IN SELECT row_data.value FROM jsonb_array_elements(p_rows) AS row_data(value)
        JOIN public.assignment AS assignment_row ON assignment_row.tenant_id=p_tenant
          AND assignment_row.course_id=p_course
          AND assignment_row.public_id=public.ple_curriculum_adoption_route_number_v1(row_data.value->'assignment','A')
        ORDER BY assignment_row.public_id LOOP
        expected:=public.ple_cam_positive_revision_v1(row->'expectedRevision');
        SELECT assignment_row.assignment_id INTO v_assignment_id FROM public.assignment AS assignment_row WHERE assignment_row.tenant_id=p_tenant
          AND assignment_row.course_id=p_course AND assignment_row.public_id=public.ple_curriculum_adoption_route_number_v1(row->'assignment','A')
          AND assignment_row.revision=expected FOR UPDATE;
        IF NOT FOUND THEN RAISE EXCEPTION 'term shift is stale' USING ERRCODE='PBC01'; END IF;
        UPDATE public.assignment_effective_policy_base AS policy_row SET available_at=CASE WHEN row#>'{schedule,availableAt}'='null'::jsonb THEN NULL ELSE to_timestamp((row#>>'{schedule,availableAt,timestamp}')::double precision/1000) END,due_at=CASE WHEN row#>'{schedule,dueAt}'='null'::jsonb THEN NULL ELSE to_timestamp((row#>>'{schedule,dueAt,timestamp}')::double precision/1000) END,closes_at=CASE WHEN row#>'{schedule,closesAt}'='null'::jsonb THEN NULL ELSE to_timestamp((row#>>'{schedule,closesAt,timestamp}')::double precision/1000) END,updated_at=transaction_timestamp() WHERE policy_row.tenant_id=p_tenant AND policy_row.assignment_id=v_assignment_id AND policy_row.course_id=p_course;
        IF NOT FOUND THEN RAISE EXCEPTION 'term schedule policy is unavailable' USING ERRCODE='PBC01'; END IF;
        UPDATE public.assignment AS assignment_row SET revision=assignment_row.revision+1,updated_at=transaction_timestamp() WHERE assignment_row.tenant_id=p_tenant AND assignment_row.assignment_id=v_assignment_id AND assignment_row.course_id=p_course;
        IF NOT FOUND THEN RAISE EXCEPTION 'term schedule assignment is unavailable' USING ERRCODE='PBC01'; END IF;
    END LOOP;
END $$;

CREATE OR REPLACE FUNCTION public.ple_bump_course_term_schedule_revision_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM public.curriculum_adoption_schedule_coalescer AS marker
        WHERE marker.tenant_id = NEW.tenant_id AND marker.course_id = NEW.course_id) THEN RETURN NEW; END IF;
    IF TG_OP = 'INSERT' THEN
        PERFORM public.ple_advance_course_schedule_revision_v1(NEW.tenant_id, NEW.course_id, true, current_user::name);
    ELSIF ROW(NEW.term_start_date, NEW.term_end_date, NEW.time_zone)
          IS DISTINCT FROM ROW(OLD.term_start_date, OLD.term_end_date, OLD.time_zone) THEN
        PERFORM public.ple_advance_course_schedule_revision_v1(NEW.tenant_id, NEW.course_id, false, current_user::name);
    END IF;
    RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION public.ple_bump_assignment_schedule_revision_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM public.curriculum_adoption_schedule_coalescer AS marker
        WHERE marker.tenant_id = NEW.tenant_id AND marker.course_id = NEW.course_id) THEN RETURN NEW; END IF;
    IF TG_OP = 'INSERT' AND EXISTS (SELECT 1 FROM public.teaching_course_assignment_position AS position_row
        WHERE position_row.tenant_id = NEW.tenant_id AND position_row.course_id = NEW.course_id
          AND position_row.assignment_id = NEW.assignment_id) THEN RETURN NEW; END IF;
    IF TG_OP = 'INSERT' OR ROW(NEW.available_at, NEW.due_at, NEW.closes_at)
       IS DISTINCT FROM ROW(OLD.available_at, OLD.due_at, OLD.closes_at) THEN
        PERFORM public.ple_advance_course_schedule_revision_v1(NEW.tenant_id, NEW.course_id, false, current_user::name);
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION public.ple_cmc_create_course_v1(p_tenant uuid,p_actor uuid,p_session character(64),p_title jsonb,p_term jsonb)
RETURNS uuid LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_course uuid:=gen_random_uuid();
BEGIN
    PERFORM public.ple_cam_require_exact_object_v1(p_term,ARRAY['startDate','endDate','timeZone'],4096);
    IF jsonb_typeof(p_title)<>'string' THEN RAISE EXCEPTION 'curriculum course title is invalid' USING ERRCODE='22023'; END IF;
    PERFORM 1 FROM public.ple_create_course_as_instructor_v1(p_tenant,v_course,p_title#>>'{}',
        (p_term->>'startDate')::date,(p_term->>'endDate')::date,p_term->>'timeZone',p_actor,p_session);
    RETURN v_course;
END $$;

CREATE FUNCTION public.ple_cmc_insert_course_evidence_v1(
    p_tenant uuid,p_key text,p_operation text,p_course uuid,p_origin text,p_course_semantic jsonb,
    p_source_alpha uuid,p_source_alpha_revision bigint,p_source_course uuid,p_source_schedule bigint
) RETURNS void LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    INSERT INTO public.curriculum_whole_course_adoption(tenant_id,course_id,receipt_key,receipt_operation,origin_kind,
        semantic_payload,semantic_canonical_version,semantic_canonical_bytes,semantic_sha256,source_alpha_course_id,
        source_alpha_revision,source_course_id,source_schedule_revision)
    VALUES(p_tenant,p_course,p_key,p_operation,p_origin,p_course_semantic->'semanticInput',
        (p_course_semantic->>'canonicalVersion')::smallint,public.ple_cam_bytes_v1(p_course_semantic->'canonicalBytes',1,524288),
        public.ple_cam_digest_bytes_v1(p_course_semantic->'semanticDigest'),p_source_alpha,p_source_alpha_revision,
        p_source_course,p_source_schedule);
END $$;

CREATE FUNCTION public.ple_cmc_materialize_course_v1(
    p_tenant uuid,p_session character(64),p_preparation uuid,p_envelope jsonb,p_operation text
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE b jsonb; p jsonb; replay jsonb; actor uuid; v_course_id uuid; source_course uuid; source_alpha uuid;
DECLARE source_revision bigint; source_schedule bigint; row jsonb; module jsonb; v_module_id uuid;
DECLARE module_pos integer:=0; v_assignment_id uuid; flat_count integer:=0; assignment_pos integer;
DECLARE operation_name text; source_row jsonb;
BEGIN
    b:=public.ple_cam_consume_materialization_preparation_v1(p_tenant,p_session,p_preparation,p_envelope);
    IF b->>'operation' IS DISTINCT FROM p_operation THEN RAISE EXCEPTION 'course materializer operation is invalid' USING ERRCODE='22023'; END IF;
    p:=b#>'{plan,plan}'; actor:=public.ple_cam_uuid_v1(b->'actor');
    operation_name:=CASE p_operation WHEN 'applyAlphaInstantiation' THEN 'alphaInstantiation' WHEN 'applyCourseRollover' THEN 'courseRollover' ELSE NULL END;
    IF operation_name IS NULL THEN RAISE EXCEPTION 'course materializer operation is invalid' USING ERRCODE='22023'; END IF;
    replay:=public.ple_cam_select_receipt_v1(p_tenant,b->>'idempotencyKey',operation_name,actor,public.ple_cam_digest_bytes_v1(b->'requestSha256'));
    IF replay IS NOT NULL THEN RETURN replay; END IF;
    IF p->'corrections' <> '[]'::jsonb OR jsonb_typeof(p->'assignments')<>'array' OR jsonb_array_length(p->'assignments') NOT BETWEEN 1 AND 1024 THEN
        RAISE EXCEPTION 'course materialization plan is invalid' USING ERRCODE='22023'; END IF;
    IF p_operation='applyAlphaInstantiation' THEN
        SELECT alpha_course_id INTO source_alpha FROM public.alpha_course
         WHERE alpha_course_reference=public.ple_curriculum_adoption_route_number_v1(p#>'{source,reference}','AC');
        source_revision:=public.ple_cam_positive_revision_v1(p#>'{source,revision}');
        IF NOT FOUND THEN RAISE EXCEPTION 'alpha source is unavailable' USING ERRCODE='PBC01'; END IF;
        -- Reauthorize the exact public Alpha revision under the same session;
        -- the plan's source witness is never a substitute for this lock.
        PERFORM public.ple_cac_reusable_document_v1(p_tenant,p_session,p->'source');
    ELSE
        source_course:=public.ple_curriculum_adoption_lock_course_v1(p_tenant,actor,p#>'{sourceWitness,course}');
        PERFORM public.ple_cac_require_witness_v1(p_tenant,source_course,p->'sourceWitness');
        source_schedule:=public.ple_cam_positive_revision_v1(p#>'{sourceWitness,scheduleRevision}');
        IF jsonb_typeof(p->'rolloverSources') <> 'array'
           OR jsonb_array_length(p->'rolloverSources') <> jsonb_array_length(p->'assignments')
           OR EXISTS (
                SELECT 1 FROM jsonb_array_elements(p->'rolloverSources') AS source(value)
                 WHERE jsonb_typeof(source.value->'sourceAssignmentRevision') <> 'number'
           )
           OR EXISTS (
                SELECT 1 FROM jsonb_array_elements(p->'rolloverSources') AS source(value)
                 LEFT JOIN public.assignment AS assignment_row
                   ON assignment_row.tenant_id=p_tenant
                  AND assignment_row.course_id=source_course
                  AND assignment_row.assignment_id=public.ple_cam_uuid_v1(source.value->'sourceAssignmentId')
                  AND assignment_row.revision=public.ple_cam_positive_revision_v1(
                        to_jsonb(source.value->>'sourceAssignmentRevision'))
                 LEFT JOIN public.teaching_course_assignment_position AS topology
                   ON topology.tenant_id=assignment_row.tenant_id AND topology.course_id=assignment_row.course_id
                  AND topology.assignment_id=assignment_row.assignment_id
                 WHERE assignment_row.assignment_id IS NULL
                    OR topology.position <> (source.value->>'assignmentPosition')::integer
                    OR NOT EXISTS (SELECT 1 FROM public.teaching_course_module AS module_row
                        WHERE module_row.tenant_id=topology.tenant_id AND module_row.course_id=topology.course_id
                          AND module_row.course_module_id=topology.course_module_id
                          AND module_row.position=(source.value->>'modulePosition')::integer)
           ) THEN RAISE EXCEPTION 'rollover source witness is stale' USING ERRCODE='PBC01'; END IF;
    END IF;
    v_course_id:=public.ple_cmc_create_course_v1(p_tenant,actor,p_session,p#>'{preview,title}',p->'targetTerm');
    PERFORM public.ple_cam_insert_receipt_v1(p_tenant,b->>'idempotencyKey',operation_name,actor,public.ple_cam_digest_bytes_v1(b->'requestSha256'),
        v_course_id,NULL,NULL,source_course,source_alpha,NULL,p->'targetTerm');
    PERFORM public.ple_cmc_insert_course_evidence_v1(p_tenant,b->>'idempotencyKey',operation_name,v_course_id,
        CASE WHEN p_operation='applyAlphaInstantiation' THEN 'alpha' ELSE 'rollover' END,p->'semantic',source_alpha,source_revision,source_course,source_schedule);
    FOR module IN SELECT value FROM jsonb_array_elements(p#>'{semantic,semanticInput,modules}') LOOP
        IF jsonb_typeof(module)<>'object' OR jsonb_typeof(module->'label')<>'string' THEN RAISE EXCEPTION 'course topology is invalid' USING ERRCODE='22023'; END IF;
        IF module_pos=0 THEN SELECT module_row.course_module_id INTO v_module_id FROM public.teaching_course_module AS module_row
          WHERE module_row.tenant_id=p_tenant AND module_row.course_id=v_course_id AND module_row.is_default FOR UPDATE;
          UPDATE public.teaching_course_module AS module_row SET title=module->>'label' WHERE module_row.tenant_id=p_tenant AND module_row.course_id=v_course_id AND module_row.course_module_id=v_module_id;
        ELSE INSERT INTO public.teaching_course_module(tenant_id,course_id,position,title,is_default)
          VALUES(p_tenant,v_course_id,module_pos,module->>'label',false) RETURNING course_module_id INTO v_module_id; END IF;
        INSERT INTO public.curriculum_whole_course_module(
            tenant_id,course_id,module_position,label
        ) VALUES(p_tenant,v_course_id,module_pos,module->>'label');
        assignment_pos:=0;
        FOR row IN SELECT value FROM jsonb_array_elements(p->'assignments') WHERE (value->>'modulePosition')::integer=module_pos ORDER BY (value->>'assignmentPosition')::integer LOOP
            IF (row->>'assignmentPosition')::integer<>assignment_pos THEN RAISE EXCEPTION 'course assignment topology is invalid' USING ERRCODE='22023'; END IF;
            IF row#>'{semantic,semanticInput,definition}' IS DISTINCT FROM module->'assignments'->assignment_pos THEN
                RAISE EXCEPTION 'course assignment semantic is detached' USING ERRCODE='PBC01'; END IF;
            v_assignment_id:=gen_random_uuid();
            PERFORM 1 FROM public.ple_create_assignment_definition_v1(p_tenant,actor,v_course_id,v_assignment_id,public.ple_caa_definition_payload_v1(row->'materialization'),NULL,NULL);
            UPDATE public.teaching_course_assignment_position AS position_row SET course_module_id=v_module_id,position=assignment_pos
             WHERE position_row.tenant_id=p_tenant AND position_row.course_id=v_course_id AND position_row.assignment_id=v_assignment_id;
            INSERT INTO public.curriculum_adoption_receipt_assignment(tenant_id,receipt_key,operation,course_id,assignment_id,single_destination_assignment_id)
             VALUES(p_tenant,b->>'idempotencyKey',operation_name,v_course_id,v_assignment_id,NULL);
            IF p_operation='applyAlphaInstantiation' THEN source_row:=jsonb_build_object('kind','alpha','alphaCourseId',source_alpha,'revision',source_revision::text,'moduleIndex',module_pos,'assignmentIndex',assignment_pos);
            ELSE source_row:=p->'rolloverSources'->flat_count;
                 IF source_row->>'modulePosition'<>module_pos::text OR source_row->>'assignmentPosition'<>assignment_pos::text THEN RAISE EXCEPTION 'rollover topology is stale' USING ERRCODE='PBC01'; END IF;
                 source_row:=jsonb_build_object('kind','rollover','sourceCourseId',source_course,'scheduleRevision',source_schedule::text,'sourceAssignmentId',source_row->'sourceAssignmentId','assignmentRevision',source_row->'sourceAssignmentRevision'); END IF;
            PERFORM public.ple_cam_insert_evidence_v1(p_tenant,b->>'idempotencyKey',v_course_id,v_assignment_id,1,row->'semantic'->'semanticInput',
                (row#>>'{semantic,canonicalVersion}')::integer,public.ple_cam_bytes_v1(row#>'{semantic,canonicalBytes}',1,524288),public.ple_cam_digest_bytes_v1(row#>'{semantic,semanticDigest}'),source_row);
            PERFORM public.ple_cam_upsert_current_v1(p_tenant,v_assignment_id,b->>'idempotencyKey');
            INSERT INTO public.curriculum_whole_course_assignment(tenant_id,course_id,receipt_key,module_position,assignment_position,destination_assignment_id,source_kind,source_assignment_id,source_assignment_revision)
              VALUES(p_tenant,v_course_id,b->>'idempotencyKey',module_pos,assignment_pos,v_assignment_id,
                CASE WHEN p_operation='applyAlphaInstantiation' THEN 'alpha' ELSE 'rollover' END,
                CASE WHEN p_operation='applyCourseRollover' THEN (p->'rolloverSources'->flat_count->>'sourceAssignmentId')::uuid END,
                CASE WHEN p_operation='applyCourseRollover' THEN public.ple_cam_positive_revision_v1(
                    to_jsonb(p->'rolloverSources'->flat_count->>'sourceAssignmentRevision')) END);
            flat_count:=flat_count+1; assignment_pos:=assignment_pos+1;
        END LOOP;
        module_pos:=module_pos+1;
    END LOOP;
    IF flat_count<>jsonb_array_length(p->'assignments') THEN RAISE EXCEPTION 'course materialization count is invalid' USING ERRCODE='22023'; END IF;
    RETURN public.ple_cam_receipt_result_v1(p_tenant,b->>'idempotencyKey',false);
END $$;

CREATE FUNCTION public.ple_cmc_materialize_term_shift_v1(p_tenant uuid,p_session character(64),p_preparation uuid,p_envelope jsonb)
RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE b jsonb; p jsonb; replay jsonb; actor uuid; v_course_id uuid;
BEGIN
    b:=public.ple_cam_consume_materialization_preparation_v1(p_tenant,p_session,p_preparation,p_envelope);
    IF b->>'operation' IS DISTINCT FROM 'applyCourseTermShift' THEN RAISE EXCEPTION 'term shift operation is invalid' USING ERRCODE='22023'; END IF;
    p:=b#>'{plan,plan}'; actor:=public.ple_cam_uuid_v1(b->'actor');
    replay:=public.ple_cam_select_receipt_v1(p_tenant,b->>'idempotencyKey','courseTermShift',actor,public.ple_cam_digest_bytes_v1(b->'requestSha256'));
    IF replay IS NOT NULL THEN RETURN replay; END IF;
    SELECT course_row.course_id INTO v_course_id FROM public.course AS course_row WHERE course_row.tenant_id=p_tenant AND course_row.public_id=public.ple_curriculum_adoption_route_number_v1(p#>'{courseWitness,course}','C') FOR UPDATE;
    IF NOT FOUND OR public.ple_curriculum_adoption_course_has_issued_work_v1(p_tenant,v_course_id) THEN RAISE EXCEPTION 'term shift is unavailable' USING ERRCODE='PBC01'; END IF;
    PERFORM public.ple_cac_require_witness_v1(p_tenant,v_course_id,p->'courseWitness');
    IF jsonb_typeof(p->'rows') <> 'array'
       OR jsonb_array_length(p->'rows') <> jsonb_array_length(p#>'{courseWitness,assignmentRevisions}')
       OR EXISTS ((SELECT value->'assignment',value->'expectedRevision' FROM jsonb_array_elements(p->'rows'))
                  EXCEPT (SELECT value->'assignment',value->'revision' FROM jsonb_array_elements(p#>'{courseWitness,assignmentRevisions}')))
       OR EXISTS ((SELECT value->'assignment',value->'revision' FROM jsonb_array_elements(p#>'{courseWitness,assignmentRevisions}'))
                  EXCEPT (SELECT value->'assignment',value->'expectedRevision' FROM jsonb_array_elements(p->'rows')))
       OR EXISTS (SELECT 1 FROM jsonb_array_elements(p->'rows') AS duplicate(value)
           GROUP BY duplicate.value->'assignment' HAVING count(*) <> 1) THEN
        RAISE EXCEPTION 'term shift rows must exactly match the witness' USING ERRCODE='PBC01';
    END IF;
    INSERT INTO public.curriculum_adoption_schedule_coalescer VALUES(p_tenant,v_course_id);
    PERFORM public.ple_cmc_apply_term_schedule_v1(p_tenant,actor,v_course_id,p->'targetTerm',p->'rows');
    DELETE FROM public.curriculum_adoption_schedule_coalescer AS marker WHERE marker.tenant_id=p_tenant AND marker.course_id=v_course_id;
    PERFORM public.ple_advance_course_schedule_revision_v1(p_tenant,v_course_id,false,'ple_curriculum_adoption_broker');
    PERFORM public.ple_cam_insert_receipt_v1(p_tenant,b->>'idempotencyKey','courseTermShift',actor,public.ple_cam_digest_bytes_v1(b->'requestSha256'),v_course_id,NULL,NULL,NULL,NULL,NULL,p->'targetTerm');
    RETURN public.ple_cam_receipt_result_v1(p_tenant,b->>'idempotencyKey',false);
END $$;

ALTER FUNCTION public.ple_bump_course_term_schedule_revision_v1() OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_bump_assignment_schedule_revision_v1() OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cmc_apply_term_schedule_v1(uuid,uuid,uuid,jsonb,jsonb) OWNER TO ple_curriculum_schedule_revision_broker;
ALTER FUNCTION public.ple_cmc_create_course_v1(uuid,uuid,character,jsonb,jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cmc_insert_course_evidence_v1(uuid,text,text,uuid,text,jsonb,uuid,bigint,uuid,bigint) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cmc_materialize_course_v1(uuid,character,uuid,jsonb,text) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cmc_materialize_term_shift_v1(uuid,character,uuid,jsonb) OWNER TO ple_curriculum_adoption_broker;
REVOKE ALL ON FUNCTION public.ple_cmc_create_course_v1(uuid,uuid,character,jsonb,jsonb),public.ple_cmc_insert_course_evidence_v1(uuid,text,text,uuid,text,jsonb,uuid,bigint,uuid,bigint),public.ple_cmc_materialize_course_v1(uuid,character,uuid,jsonb,text),public.ple_cmc_materialize_term_shift_v1(uuid,character,uuid,jsonb) FROM PUBLIC,ple_app,ple_auth,ple_student,ple_grader,ple_grading_reader,ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_cmc_apply_term_schedule_v1(uuid,uuid,uuid,jsonb,jsonb) FROM PUBLIC,ple_app,ple_auth,ple_student,ple_grader,ple_grading_reader,ple_retention_broker;
GRANT EXECUTE ON FUNCTION public.ple_cmc_create_course_v1(uuid,uuid,character,jsonb,jsonb),public.ple_cmc_insert_course_evidence_v1(uuid,text,text,uuid,text,jsonb,uuid,bigint,uuid,bigint),public.ple_cmc_materialize_course_v1(uuid,character,uuid,jsonb,text),public.ple_cmc_materialize_term_shift_v1(uuid,character,uuid,jsonb) TO ple_curriculum_adoption_broker;
GRANT EXECUTE ON FUNCTION public.ple_cmc_apply_term_schedule_v1(uuid,uuid,uuid,jsonb,jsonb) TO ple_curriculum_adoption_broker;
-- The schedule writer resolves tenant RLS and the two closed scalar values in
-- its own SECURITY DEFINER context.  Grant precisely those helper edges.
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(),
    public.ple_curriculum_adoption_route_number_v1(jsonb,text),
    public.ple_cam_positive_revision_v1(jsonb)
    TO ple_curriculum_schedule_revision_broker;

-- The schedule writer has precisely the term-shift surface: no relation-wide
-- mutation, no receipt/evidence authority, and no application executor.
DO $$
DECLARE v_function regprocedure;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'public.ple_cmc_create_course_v1(uuid,uuid,character,jsonb,jsonb)'::regprocedure,
        'public.ple_cmc_insert_course_evidence_v1(uuid,text,text,uuid,text,jsonb,uuid,bigint,uuid,bigint)'::regprocedure,
        'public.ple_cmc_materialize_course_v1(uuid,character,uuid,jsonb,text)'::regprocedure,
        'public.ple_cmc_materialize_term_shift_v1(uuid,character,uuid,jsonb)'::regprocedure
    ] LOOP
        IF (SELECT pg_get_userbyid(proowner) FROM pg_proc WHERE oid=v_function)
               <> 'ple_curriculum_adoption_broker'
           OR NOT (SELECT prosecdef FROM pg_proc WHERE oid=v_function)
           OR NOT coalesce((SELECT proconfig @> ARRAY['search_path=pg_catalog, public, pg_temp']
                            FROM pg_proc WHERE oid=v_function),false)
           OR has_function_privilege('ple_app',v_function,'EXECUTE')
           OR NOT has_function_privilege(
                'ple_curriculum_adoption_broker',v_function,'EXECUTE'
              ) THEN
            RAISE EXCEPTION 'curriculum course materializer catalog is unsafe';
        END IF;
    END LOOP;
    IF has_table_privilege('ple_curriculum_schedule_revision_broker',
        'public.curriculum_adoption_schedule_coalescer','SELECT,INSERT,DELETE,UPDATE')
       OR has_table_privilege('ple_curriculum_schedule_revision_broker',
        'public.curriculum_adoption_schedule_coalescer','UPDATE,TRUNCATE,REFERENCES,TRIGGER')
       OR NOT has_function_privilege('ple_curriculum_adoption_broker',
        'public.ple_cmc_apply_term_schedule_v1(uuid,uuid,uuid,jsonb,jsonb)'::regprocedure,'EXECUTE')
       OR has_function_privilege('ple_app',
        'public.ple_cmc_apply_term_schedule_v1(uuid,uuid,uuid,jsonb,jsonb)'::regprocedure,'EXECUTE')
       OR NOT has_function_privilege('ple_curriculum_schedule_revision_broker',
            'public.ple_current_tenant()'::regprocedure,'EXECUTE')
       OR NOT has_function_privilege('ple_curriculum_schedule_revision_broker',
            'public.ple_curriculum_adoption_route_number_v1(jsonb,text)'::regprocedure,'EXECUTE')
       OR NOT has_function_privilege('ple_curriculum_schedule_revision_broker',
            'public.ple_cam_positive_revision_v1(jsonb)'::regprocedure,'EXECUTE')
       OR (SELECT pg_get_userbyid(proowner) FROM pg_proc WHERE oid=
            'public.ple_cmc_apply_term_schedule_v1(uuid,uuid,uuid,jsonb,jsonb)'::regprocedure)
          <> 'ple_curriculum_schedule_revision_broker'
       OR NOT has_table_privilege('ple_curriculum_schedule_revision_broker',
            'public.assignment_effective_policy_base','SELECT,UPDATE')
       OR NOT EXISTS (SELECT 1 FROM pg_policy AS policy_row
            WHERE policy_row.polrelid='public.assignment_effective_policy_base'::regclass
              AND policy_row.polname='curriculum_schedule_shift_policy' AND policy_row.polcmd='r'
              AND 'ple_curriculum_schedule_revision_broker'::regrole::oid=ANY(policy_row.polroles))
       OR NOT EXISTS (SELECT 1 FROM pg_policy AS policy_row
            WHERE policy_row.polrelid='public.assignment_effective_policy_base'::regclass
              AND policy_row.polname='curriculum_schedule_shift_policy_update' AND policy_row.polcmd='w'
              AND 'ple_curriculum_schedule_revision_broker'::regrole::oid=ANY(policy_row.polroles))
    THEN RAISE EXCEPTION 'curriculum term schedule capability catalog is unsafe'; END IF;
END $$;
COMMIT;
