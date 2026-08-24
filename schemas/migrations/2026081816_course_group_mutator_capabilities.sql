-- WP-PROF-T4: the course-group aggregate has one session-bound write authority.
-- The accepted member-revocation trigger owns stale group-member cleanup.

BEGIN;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_course_group_mutator_broker') THEN
        CREATE ROLE ple_course_group_mutator_broker
            NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
    END IF;
END $$;
ALTER ROLE ple_course_group_mutator_broker
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_auth_members
               WHERE roleid = 'ple_course_group_mutator_broker'::regrole
                  OR member = 'ple_course_group_mutator_broker'::regrole) THEN
        RAISE EXCEPTION 'ple_course_group_mutator_broker must not have role memberships';
    END IF;
END $$;
REVOKE ALL ON SCHEMA public FROM ple_course_group_mutator_broker;
GRANT USAGE ON SCHEMA public TO ple_course_group_mutator_broker;

-- The broker sees and changes only the aggregate, its active student members,
-- and the assignment rows whose policy refers to that aggregate.
CREATE POLICY course_group_mutator_course_tenant ON public.course
    TO ple_course_group_mutator_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_group_mutator_member_tenant ON public.course_member
    TO ple_course_group_mutator_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_group_mutator_group_tenant ON public.course_group
    TO ple_course_group_mutator_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY course_group_mutator_group_member_tenant ON public.course_group_member
    TO ple_course_group_mutator_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY course_group_mutator_assignment_tenant ON public.assignment
    TO ple_course_group_mutator_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_group_mutator_audience_tenant ON public.assignment_audience_group
    TO ple_course_group_mutator_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_group_mutator_offset_tenant ON public.assignment_group_schedule_offset
    TO ple_course_group_mutator_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_group_mutator_accommodation_tenant ON public.assignment_group_accommodation
    TO ple_course_group_mutator_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_group_mutator_policy_tenant ON public.course_group_membership_policy
    TO ple_course_group_mutator_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());

-- The T2 table predates durable policy-update witnesses.  A server-owned
-- timestamp makes the policy CAS witness complete without creating a second
-- policy lifecycle.  Existing rows acquire one migration-time timestamp.
ALTER TABLE public.course_group_membership_policy
    ADD COLUMN updated_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp();

GRANT SELECT, UPDATE (course_id) ON public.course TO ple_course_group_mutator_broker;
GRANT SELECT, UPDATE (course_membership_id) ON public.course_member TO ple_course_group_mutator_broker;
GRANT SELECT, INSERT, DELETE, UPDATE (purpose, title, revision, updated_at)
    ON public.course_group TO ple_course_group_mutator_broker;
-- The broker never changes a member row.  PostgreSQL needs this narrow lock
-- privilege for the canonical `FOR UPDATE` member ordering.
GRANT SELECT, INSERT, DELETE, UPDATE (course_group_id)
    ON public.course_group_member TO ple_course_group_mutator_broker;
GRANT SELECT, UPDATE (assignment_id) ON public.assignment TO ple_course_group_mutator_broker;
GRANT SELECT ON public.assignment_audience_group, public.assignment_group_schedule_offset,
    public.assignment_group_accommodation TO ple_course_group_mutator_broker;
GRANT SELECT, UPDATE (multiple_membership, revision, updated_at)
    ON public.course_group_membership_policy TO ple_course_group_mutator_broker;
-- FOR UPDATE locks the exact presented session before the aggregate decision.
GRANT SELECT, UPDATE (session_hash) ON public.auth_session TO ple_course_group_mutator_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(), public.ple_course_records_accessible(uuid, uuid)
    TO ple_course_group_mutator_broker;
REVOKE INSERT, UPDATE, DELETE ON public.course_group_member FROM ple_app;

CREATE FUNCTION public.ple_course_group_mutator_require_inputs(
    p_tenant uuid, p_actor uuid, p_course uuid, p_group uuid, p_expected_revision bigint,
    p_purpose text, p_title text, p_member_ids uuid[]
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_tenant IS NULL OR p_actor IS NULL OR p_course IS NULL OR p_group IS NULL
       OR p_expected_revision = 0 OR p_expected_revision < 0
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid course group capability' USING ERRCODE = '22023';
    END IF;
    IF p_purpose NOT IN ('section', 'lab', 'cohort', 'accommodation', 'work')
       OR p_title IS NULL OR p_title <> btrim(p_title)
       OR char_length(p_title) NOT BETWEEN 1 AND 200
       OR p_member_ids IS NULL
       OR EXISTS (SELECT 1 FROM unnest(p_member_ids) member_id WHERE member_id IS NULL) THEN
        RAISE EXCEPTION 'invalid course group definition' USING ERRCODE = '22023';
    END IF;
    IF cardinality(p_member_ids) <> cardinality(ARRAY(SELECT DISTINCT member_id FROM unnest(p_member_ids) member_id)) THEN
        RAISE EXCEPTION 'course group members must be distinct' USING ERRCODE = '22023';
    END IF;
END $$;

CREATE FUNCTION public.ple_put_course_group_v1(
    p_tenant uuid, p_actor uuid, p_course uuid, p_group uuid, p_expected_revision bigint,
    p_purpose text, p_title text, p_member_ids uuid[]
) RETURNS TABLE(revision bigint, affected_assignment_ids uuid[], affected_assignment_revisions bigint[])
LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_existing public.course_group%ROWTYPE; v_current_members uuid[];
DECLARE v_affected_ids uuid[] := ARRAY[]::uuid[]; v_affected_revisions bigint[] := ARRAY[]::bigint[];
DECLARE v_next_revision bigint; v_group_exists boolean; v_member_ids uuid[];
BEGIN
    PERFORM public.ple_course_group_mutator_require_inputs(
        p_tenant,p_actor,p_course,p_group,p_expected_revision,p_purpose,p_title,p_member_ids);
    PERFORM 1 FROM public.course
     WHERE tenant_id=p_tenant AND course_id=p_course FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'course is unavailable' USING ERRCODE='42501'; END IF;
    -- Lock order is course, then the group identity's serialization point,
    -- then group, member, and referenced-assignment rows.
    PERFORM 1 FROM public.course_member
     WHERE tenant_id=p_tenant AND course_id=p_course AND user_id=p_actor
       AND role='instructor' AND status='active'
     ORDER BY course_membership_id FOR UPDATE;
    IF NOT FOUND OR NOT public.ple_course_records_accessible(p_tenant,p_course) THEN
        RAISE EXCEPTION 'active direct instructor authority is required' USING ERRCODE='42501';
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_tenant::text || ':' || p_group::text, 0));
    SELECT * INTO v_existing FROM public.course_group
     WHERE tenant_id=p_tenant AND course_group_id=p_group FOR UPDATE;
    v_group_exists := FOUND;
    IF v_group_exists AND v_existing.course_id <> p_course THEN
        RAISE EXCEPTION 'course group is unavailable' USING ERRCODE='42501';
    END IF;
    SELECT coalesce(array_agg(member_id ORDER BY member_id), ARRAY[]::uuid[])
      INTO v_member_ids FROM unnest(p_member_ids) member_id;
    -- Lock the requested student episodes in a canonical order before loading
    -- assignment references.  Count equality rejects cross-course/revoked IDs.
    PERFORM 1 FROM public.course_member
     WHERE tenant_id=p_tenant AND course_id=p_course AND role='student' AND status='active'
       AND course_membership_id = ANY(v_member_ids)
     ORDER BY course_membership_id FOR UPDATE;
    IF (SELECT count(*) FROM public.course_member
        WHERE tenant_id=p_tenant AND course_id=p_course AND role='student' AND status='active'
          AND course_membership_id = ANY(v_member_ids)) <> cardinality(v_member_ids) THEN
        RAISE EXCEPTION 'course group members must be active course students' USING ERRCODE='42501';
    END IF;
    SELECT coalesce(array_agg(course_membership_id ORDER BY course_membership_id), ARRAY[]::uuid[])
      INTO v_current_members FROM public.course_group_member
     WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id=p_group;
    PERFORM 1 FROM public.course_group_member
     WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id=p_group
     ORDER BY course_membership_id FOR UPDATE;
    SELECT coalesce(array_agg(assignment_id ORDER BY assignment_id), ARRAY[]::uuid[])
      INTO v_affected_ids
      FROM (
        SELECT assignment_id FROM public.assignment_audience_group WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id=p_group
        UNION
        SELECT assignment_id FROM public.assignment_group_schedule_offset WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id=p_group
        UNION
        SELECT assignment_id FROM public.assignment_group_accommodation WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id=p_group
      ) group_references;
    PERFORM 1 FROM public.assignment
     WHERE tenant_id=p_tenant AND course_id=p_course AND assignment_id = ANY(v_affected_ids)
     ORDER BY assignment_id FOR UPDATE;
    IF (SELECT count(*) FROM public.assignment WHERE tenant_id=p_tenant AND course_id=p_course AND assignment_id=ANY(v_affected_ids)) <> cardinality(v_affected_ids) THEN
        RAISE EXCEPTION 'course group assignment reference is unavailable' USING ERRCODE='42501';
    END IF;
    SELECT coalesce(array_agg(assignment_row.revision ORDER BY assignment_row.assignment_id), ARRAY[]::bigint[])
      INTO v_affected_revisions FROM public.assignment assignment_row
     WHERE assignment_row.tenant_id=p_tenant AND assignment_row.course_id=p_course
       AND assignment_row.assignment_id=ANY(v_affected_ids);
    IF v_group_exists THEN
        IF v_existing.purpose = p_purpose AND v_existing.title = p_title AND v_current_members = v_member_ids THEN
            revision := v_existing.revision; affected_assignment_ids := ARRAY[]::uuid[];
            affected_assignment_revisions := ARRAY[]::bigint[]; RETURN NEXT; RETURN;
        END IF;
        IF p_expected_revision IS NULL OR p_expected_revision <> v_existing.revision THEN
            RAISE EXCEPTION 'course group revision conflict' USING ERRCODE = '55000';
        END IF;
        IF (p_purpose NOT IN ('section','lab','cohort') AND EXISTS (
              SELECT 1 FROM public.assignment_audience_group WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id=p_group
              UNION ALL SELECT 1 FROM public.assignment_group_schedule_offset WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id=p_group))
           OR (p_purpose <> 'accommodation' AND EXISTS (
              SELECT 1 FROM public.assignment_group_accommodation WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id=p_group)) THEN
            RAISE EXCEPTION 'course group purpose conflicts with assignment references' USING ERRCODE='23514';
        END IF;
        v_next_revision := v_existing.revision + 1;
        UPDATE public.course_group group_row SET purpose=p_purpose,title=p_title,revision=v_next_revision,
            updated_at=transaction_timestamp()
         WHERE group_row.tenant_id=p_tenant AND group_row.course_id=p_course
           AND group_row.course_group_id=p_group AND group_row.revision=p_expected_revision;
        IF NOT FOUND THEN RAISE EXCEPTION 'course group revision conflict' USING ERRCODE = '55000'; END IF;
    ELSE
        IF p_expected_revision IS NOT NULL THEN RAISE EXCEPTION 'course group revision conflict' USING ERRCODE = '55000'; END IF;
        v_next_revision := 1;
        INSERT INTO public.course_group(tenant_id,course_id,course_group_id,purpose,title,revision)
        VALUES(p_tenant,p_course,p_group,p_purpose,p_title,v_next_revision);
    END IF;
    DELETE FROM public.course_group_member WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id=p_group;
    INSERT INTO public.course_group_member(tenant_id,course_id,course_group_id,course_membership_id)
    SELECT p_tenant,p_course,p_group,member_id FROM unnest(v_member_ids) member_id ORDER BY member_id;
    revision := v_next_revision; affected_assignment_ids := v_affected_ids;
    affected_assignment_revisions := v_affected_revisions; RETURN NEXT;
END $$;

CREATE FUNCTION public.ple_delete_course_group_v1(
    p_tenant uuid, p_actor uuid, p_course uuid, p_group uuid, p_expected_revision bigint
) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_existing public.course_group%ROWTYPE;
BEGIN
    IF p_tenant IS NULL OR p_actor IS NULL OR p_course IS NULL OR p_group IS NULL
       OR p_expected_revision IS NULL OR p_expected_revision < 1
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invalid course group deletion capability' USING ERRCODE='22023';
    END IF;
    PERFORM 1 FROM public.course WHERE tenant_id=p_tenant AND course_id=p_course FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'course is unavailable' USING ERRCODE='42501'; END IF;
    PERFORM 1 FROM public.course_member WHERE tenant_id=p_tenant AND course_id=p_course AND user_id=p_actor
       AND role='instructor' AND status='active' ORDER BY course_membership_id FOR UPDATE;
    IF NOT FOUND OR NOT public.ple_course_records_accessible(p_tenant,p_course) THEN
        RAISE EXCEPTION 'active direct instructor authority is required' USING ERRCODE='42501';
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_tenant::text || ':' || p_group::text, 0));
    SELECT * INTO v_existing FROM public.course_group WHERE tenant_id=p_tenant AND course_group_id=p_group FOR UPDATE;
    IF NOT FOUND THEN RETURN false; END IF;
    IF v_existing.course_id <> p_course OR v_existing.revision <> p_expected_revision THEN
        RAISE EXCEPTION 'course group revision conflict' USING ERRCODE = '55000';
    END IF;
    IF EXISTS (SELECT 1 FROM public.assignment_audience_group WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id=p_group
               UNION ALL SELECT 1 FROM public.assignment_group_schedule_offset WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id=p_group
               UNION ALL SELECT 1 FROM public.assignment_group_accommodation WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id=p_group) THEN
        RAISE EXCEPTION 'referenced course group cannot be deleted' USING ERRCODE='23514';
    END IF;
    DELETE FROM public.course_group_member WHERE tenant_id=p_tenant AND course_id=p_course AND course_group_id=p_group;
    DELETE FROM public.course_group group_row WHERE group_row.tenant_id=p_tenant
      AND group_row.course_id=p_course AND group_row.course_group_id=p_group
      AND group_row.revision=p_expected_revision;
    RETURN FOUND;
END $$;

-- ASVS 2.2.1-2.2.3, 2.3.1-2.3.4, 8.2.1-8.2.3, 8.3.1-8.3.3, 15.4.2-15.4.3:
-- one session-bound, locked policy replacement.  The caller presents a
-- session hash solely to select the live session row; the returned actor is
-- derived from that row and no caller-supplied actor value is trusted.
CREATE FUNCTION public.ple_replace_course_group_purpose_policy_v1(
    p_tenant uuid,
    p_session character(64),
    p_course uuid,
    p_purpose text,
    p_multiple_membership text,
    p_expected_revision bigint
) RETURNS TABLE(
    tenant_id uuid,
    actor_id uuid,
    course_id uuid,
    purpose text,
    multiple_membership text,
    revision bigint
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE
    v_actor uuid;
    v_roles jsonb;
    v_policy_count bigint;
    v_stored_revision bigint;
    v_next_revision bigint;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL OR p_course IS NULL
       OR p_purpose NOT IN ('section', 'lab', 'cohort', 'accommodation', 'work')
       OR p_multiple_membership NOT IN ('allow', 'warn')
       OR p_expected_revision IS NULL OR p_expected_revision < 1
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'course group purpose policy arguments are invalid' USING ERRCODE='22023';
    END IF;

    -- The forced-RLS session policy admits only the presented capability.
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT session_row.user_id, session_row.roles INTO v_actor, v_roles
      FROM public.auth_session AS session_row
     WHERE session_row.session_hash = p_session
       AND session_row.tenant_id = p_tenant
       AND session_row.revoked_at IS NULL
       AND session_row.expires_at > transaction_timestamp()
     FOR UPDATE;
    IF NOT FOUND OR v_actor IS NULL OR NOT v_roles @> '["instructor"]'::jsonb THEN
        RAISE EXCEPTION 'course group purpose policy is unavailable' USING ERRCODE='42501';
    END IF;

    -- Canonical aggregate order: course, direct-Instructor membership, then
    -- all five purpose rows in their closed lexical order.
    PERFORM 1 FROM public.course
     WHERE tenant_id = p_tenant AND course_id = p_course FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course group purpose policy is unavailable' USING ERRCODE='42501';
    END IF;
    PERFORM 1 FROM public.course_member
     WHERE tenant_id = p_tenant AND course_id = p_course AND user_id = v_actor
       AND role = 'instructor' AND status = 'active'
     ORDER BY course_membership_id FOR UPDATE;
    IF NOT FOUND OR NOT public.ple_course_records_accessible(p_tenant, p_course) THEN
        RAISE EXCEPTION 'course group purpose policy is unavailable' USING ERRCODE='42501';
    END IF;
    PERFORM 1 FROM public.course_group_membership_policy
     WHERE tenant_id = p_tenant AND course_id = p_course
     ORDER BY purpose FOR UPDATE;
    SELECT count(*) INTO v_policy_count
      FROM public.course_group_membership_policy
     WHERE tenant_id = p_tenant AND course_id = p_course;
    IF v_policy_count <> 5
       OR (SELECT count(*) FROM public.course_group_membership_policy
           WHERE tenant_id = p_tenant AND course_id = p_course
             AND purpose IN ('section', 'lab', 'cohort', 'accommodation', 'work')) <> 5 THEN
        RAISE EXCEPTION 'course group purpose policy aggregate is invalid' USING ERRCODE='55000';
    END IF;
    SELECT policy_row.revision INTO v_stored_revision
      FROM public.course_group_membership_policy AS policy_row
     WHERE policy_row.tenant_id = p_tenant AND policy_row.course_id = p_course
       AND policy_row.purpose = p_purpose;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course group purpose policy aggregate is invalid' USING ERRCODE='55000';
    END IF;
    IF v_stored_revision <> p_expected_revision THEN
        RAISE EXCEPTION 'course group purpose policy revision conflict' USING ERRCODE = '55000';
    END IF;

    UPDATE public.course_group_membership_policy AS policy_row
       SET multiple_membership = p_multiple_membership,
           revision = policy_row.revision + 1,
           updated_at = transaction_timestamp()
     WHERE policy_row.tenant_id = p_tenant AND policy_row.course_id = p_course
       AND policy_row.purpose = p_purpose AND policy_row.revision = p_expected_revision
     RETURNING policy_row.revision INTO v_next_revision;
    IF NOT FOUND OR v_next_revision < 1 THEN
        RAISE EXCEPTION 'course group purpose policy revision conflict' USING ERRCODE = '55000';
    END IF;

    tenant_id := p_tenant;
    actor_id := v_actor;
    course_id := p_course;
    purpose := p_purpose;
    multiple_membership := p_multiple_membership;
    revision := v_next_revision;
    RETURN NEXT;
END $$;

ALTER FUNCTION public.ple_course_group_mutator_require_inputs(uuid,uuid,uuid,uuid,bigint,text,text,uuid[]) OWNER TO ple_course_group_mutator_broker;
ALTER FUNCTION public.ple_put_course_group_v1(uuid,uuid,uuid,uuid,bigint,text,text,uuid[]) OWNER TO ple_course_group_mutator_broker;
ALTER FUNCTION public.ple_delete_course_group_v1(uuid,uuid,uuid,uuid,bigint) OWNER TO ple_course_group_mutator_broker;
ALTER FUNCTION public.ple_replace_course_group_purpose_policy_v1(uuid,character(64),uuid,text,text,bigint) OWNER TO ple_course_group_mutator_broker;
REVOKE ALL ON FUNCTION public.ple_course_group_mutator_require_inputs(uuid,uuid,uuid,uuid,bigint,text,text,uuid[]) FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION public.ple_put_course_group_v1(uuid,uuid,uuid,uuid,bigint,text,text,uuid[]), public.ple_delete_course_group_v1(uuid,uuid,uuid,uuid,bigint), public.ple_replace_course_group_purpose_policy_v1(uuid,character(64),uuid,text,text,bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_put_course_group_v1(uuid,uuid,uuid,uuid,bigint,text,text,uuid[]), public.ple_delete_course_group_v1(uuid,uuid,uuid,uuid,bigint), public.ple_replace_course_group_purpose_policy_v1(uuid,character(64),uuid,text,text,bigint) TO ple_app;

DO $$
BEGIN
    IF has_table_privilege('ple_app','public.course_group','INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_app','public.course_group_member','INSERT,UPDATE')
       OR has_table_privilege('ple_app','public.course_group_member','DELETE')
       OR NOT has_function_privilege('ple_app','public.ple_put_course_group_v1(uuid,uuid,uuid,uuid,bigint,text,text,uuid[])','EXECUTE')
       OR NOT has_function_privilege('ple_app','public.ple_delete_course_group_v1(uuid,uuid,uuid,uuid,bigint)','EXECUTE')
       OR NOT has_function_privilege('ple_app','public.ple_replace_course_group_purpose_policy_v1(uuid,character(64),uuid,text,text,bigint)','EXECUTE')
       OR has_function_privilege('public','public.ple_put_course_group_v1(uuid,uuid,uuid,uuid,bigint,text,text,uuid[])','EXECUTE')
       OR has_function_privilege('public','public.ple_delete_course_group_v1(uuid,uuid,uuid,uuid,bigint)','EXECUTE')
       OR has_function_privilege('public','public.ple_replace_course_group_purpose_policy_v1(uuid,character(64),uuid,text,text,bigint)','EXECUTE')
       OR has_function_privilege('public','public.ple_course_group_mutator_require_inputs(uuid,uuid,uuid,uuid,bigint,text,text,uuid[])','EXECUTE')
       OR NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname='ple_course_group_mutator_broker'
                      AND NOT rolsuper AND NOT rolcreatedb AND NOT rolcreaterole
                      AND NOT rolreplication AND NOT rolcanlogin AND NOT rolinherit
                      AND NOT rolbypassrls)
       OR EXISTS (SELECT 1 FROM pg_auth_members WHERE roleid='ple_course_group_mutator_broker'::regrole
                  OR member='ple_course_group_mutator_broker'::regrole)
       OR EXISTS (SELECT 1 FROM pg_proc procedure
                  WHERE procedure.oid IN ('public.ple_course_group_mutator_require_inputs(uuid,uuid,uuid,uuid,bigint,text,text,uuid[])'::regprocedure,
                                          'public.ple_put_course_group_v1(uuid,uuid,uuid,uuid,bigint,text,text,uuid[])'::regprocedure,
                                          'public.ple_delete_course_group_v1(uuid,uuid,uuid,uuid,bigint)'::regprocedure,
                                          'public.ple_replace_course_group_purpose_policy_v1(uuid,character(64),uuid,text,text,bigint)'::regprocedure)
                    AND (NOT procedure.prosecdef OR procedure.proowner <> 'ple_course_group_mutator_broker'::regrole
                         OR NOT coalesce(procedure.proconfig,ARRAY[]::text[]) @> ARRAY['search_path=pg_catalog, public, pg_temp']))
       OR NOT has_table_privilege('ple_course_group_mutator_broker','public.course_group','SELECT,INSERT,DELETE')
       OR NOT has_column_privilege('ple_course_group_mutator_broker','public.course_group','purpose','UPDATE')
       OR NOT has_column_privilege('ple_course_group_mutator_broker','public.course_group','title','UPDATE')
       OR NOT has_column_privilege('ple_course_group_mutator_broker','public.course_group','revision','UPDATE')
       OR NOT has_column_privilege('ple_course_group_mutator_broker','public.course_group','updated_at','UPDATE')
       OR has_column_privilege('ple_course_group_mutator_broker','public.course_group','course_group_id','UPDATE')
       OR NOT has_table_privilege('ple_course_group_mutator_broker','public.course_group_member','SELECT,INSERT,DELETE')
       OR NOT has_column_privilege('ple_course_group_mutator_broker','public.course_group_member','course_group_id','UPDATE')
       OR has_column_privilege('ple_course_group_mutator_broker','public.course_group_member','course_membership_id','UPDATE')
       OR NOT has_table_privilege('ple_course_group_mutator_broker','public.course_group_membership_policy','SELECT')
       OR NOT has_column_privilege('ple_course_group_mutator_broker','public.course_group_membership_policy','multiple_membership','UPDATE')
       OR NOT has_column_privilege('ple_course_group_mutator_broker','public.course_group_membership_policy','revision','UPDATE')
       OR NOT has_column_privilege('ple_course_group_mutator_broker','public.course_group_membership_policy','updated_at','UPDATE')
       OR has_column_privilege('ple_course_group_mutator_broker','public.course_group_membership_policy','purpose','UPDATE')
       OR NOT has_table_privilege('ple_course_group_mutator_broker','public.auth_session','SELECT')
       OR NOT has_column_privilege('ple_course_group_mutator_broker','public.auth_session','session_hash','UPDATE')
       OR has_column_privilege('ple_course_group_mutator_broker','public.auth_session','roles','UPDATE')
       OR NOT EXISTS (
            SELECT 1 FROM pg_trigger trigger_row
             JOIN pg_class relation_row ON relation_row.oid=trigger_row.tgrelid
             JOIN pg_namespace namespace_row ON namespace_row.oid=relation_row.relnamespace
             JOIN pg_proc procedure_row ON procedure_row.oid=trigger_row.tgfoid
             JOIN pg_roles owner_row ON owner_row.oid=procedure_row.proowner
             WHERE namespace_row.nspname='public'
               AND relation_row.relname='course_member'
               AND trigger_row.tgname='course_membership_revocation_removes_current_group_memberships'
               AND NOT trigger_row.tgisinternal
               AND procedure_row.proname='ple_remove_revoked_course_group_memberships'
               AND owner_row.rolname='ple_retention_broker'
               AND NOT has_function_privilege('public',procedure_row.oid,'EXECUTE')
       )
       OR (SELECT count(*) FROM pg_policies WHERE schemaname='public' AND policyname IN
             ('course_group_mutator_course_tenant','course_group_mutator_member_tenant','course_group_mutator_group_tenant',
              'course_group_mutator_group_member_tenant','course_group_mutator_assignment_tenant','course_group_mutator_audience_tenant',
              'course_group_mutator_offset_tenant','course_group_mutator_accommodation_tenant','course_group_mutator_policy_tenant')
             AND roles=ARRAY['ple_course_group_mutator_broker']::name[]
             AND cmd='ALL' AND qual LIKE '%tenant_id%' AND with_check LIKE '%tenant_id%') <> 3
       OR (SELECT count(*) FROM pg_policies WHERE schemaname='public' AND policyname IN
             ('course_group_mutator_course_tenant','course_group_mutator_member_tenant','course_group_mutator_assignment_tenant','course_group_mutator_audience_tenant',
              'course_group_mutator_offset_tenant','course_group_mutator_accommodation_tenant')
             AND roles=ARRAY['ple_course_group_mutator_broker']::name[]
             AND cmd='ALL' AND qual LIKE '%tenant_id%' AND with_check IS NULL) <> 6 THEN
        RAISE EXCEPTION 'course group capability grants are unsafe';
    END IF;
END $$;

COMMIT;
