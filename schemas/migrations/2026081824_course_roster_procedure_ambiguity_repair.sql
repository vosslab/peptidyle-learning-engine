BEGIN;

-- These table-returning procedures expose output variables named tenant_id,
-- course_id, roster_import_id, and roster_revision. Keep PL/pgSQL's strict
-- ambiguity policy and bind every stored column through an explicit alias.
CREATE POLICY course_roster_policy_broker_course_lock
    ON public.course FOR UPDATE TO ple_course_roster_policy_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
GRANT UPDATE (course_id) ON public.course TO ple_course_roster_policy_broker;
CREATE POLICY course_invitation_broker_course_lock
    ON public.course FOR UPDATE TO ple_course_invitation_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
GRANT UPDATE (course_id) ON public.course TO ple_course_invitation_broker;

CREATE OR REPLACE FUNCTION public.ple_replace_course_enrollment_policy_v1(
    p_tenant uuid,
    p_session character(64),
    p_course uuid,
    p_expected bigint,
    p_posture text,
    p_domains jsonb
) RETURNS TABLE (
    tenant_id uuid,
    actor_id uuid,
    course_id uuid,
    roster_revision bigint
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
#variable_conflict error
DECLARE
    v_actor uuid;
    v_roster_revision bigint;
    v_domain_count integer;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL OR p_course IS NULL
       OR p_expected < 1 OR p_posture NOT IN ('invitation_only', 'permitted_domains')
       OR jsonb_typeof(p_domains) <> 'array'
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'enrollment policy arguments are invalid' USING ERRCODE = '22023';
    END IF;

    SELECT count(*) INTO v_domain_count
      FROM jsonb_to_recordset(p_domains) AS domain_row(
          domain text,
          include_subdomains boolean
      );
    IF v_domain_count > 32
       OR (p_posture = 'permitted_domains' AND v_domain_count = 0)
       OR EXISTS (
           SELECT 1
             FROM jsonb_to_recordset(p_domains) AS domain_row(
                 domain text,
                 include_subdomains boolean
             )
            WHERE domain_row.domain IS NULL
               OR domain_row.domain <> lower(btrim(domain_row.domain))
               OR domain_row.domain = ''
               OR char_length(domain_row.domain) > 253
               OR domain_row.domain !~ '^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)+$'
               OR domain_row.include_subdomains IS NULL
       )
       OR (
           SELECT count(DISTINCT domain_row.domain)
             FROM jsonb_to_recordset(p_domains) AS domain_row(
                 domain text,
                 include_subdomains boolean
             )
       ) <> v_domain_count THEN
        RAISE EXCEPTION 'enrollment policy shape is invalid' USING ERRCODE = '22023';
    END IF;

    PERFORM 1
      FROM public.course AS course_row
     WHERE course_row.tenant_id = p_tenant
       AND course_row.course_id = p_course
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;

    SELECT roster_state.revision INTO v_roster_revision
      FROM public.course_roster_state AS roster_state
     WHERE roster_state.tenant_id = p_tenant
       AND roster_state.course_id = p_course
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course roster aggregate is invalid' USING ERRCODE = '55000';
    END IF;

    v_actor := public.ple_course_roster_support_actor(
        p_session,
        p_course,
        'replaceEnrollmentPolicy'
    );
    IF v_actor IS NULL THEN RETURN; END IF;
    IF v_roster_revision <> p_expected THEN
        RAISE EXCEPTION 'course roster revision conflicts' USING ERRCODE = '55000';
    END IF;

    IF (
        SELECT roster_state.signup_posture
          FROM public.course_roster_state AS roster_state
         WHERE roster_state.tenant_id = p_tenant
           AND roster_state.course_id = p_course
    ) = p_posture
       AND COALESCE(
           (
               SELECT jsonb_agg(
                   jsonb_build_object(
                       'domain', domain_row.normalized_domain,
                       'include_subdomains', domain_row.include_subdomains
                   )
                   ORDER BY domain_row.normalized_domain
               )
                 FROM public.course_allowed_email_domain AS domain_row
                WHERE domain_row.tenant_id = p_tenant
                  AND domain_row.course_id = p_course
           ),
           '[]'::jsonb
       ) = p_domains THEN
        tenant_id := p_tenant;
        actor_id := v_actor;
        course_id := p_course;
        roster_revision := v_roster_revision;
        RETURN NEXT;
        RETURN;
    END IF;

    DELETE FROM public.course_allowed_email_domain AS domain_row
     WHERE domain_row.tenant_id = p_tenant
       AND domain_row.course_id = p_course;
    INSERT INTO public.course_allowed_email_domain (
        tenant_id,
        course_id,
        normalized_domain,
        include_subdomains
    )
    SELECT p_tenant, p_course, domain_row.domain, domain_row.include_subdomains
      FROM jsonb_to_recordset(p_domains) AS domain_row(
          domain text,
          include_subdomains boolean
      );

    UPDATE public.course_roster_state AS roster_state
       SET signup_posture = p_posture,
           revision = roster_state.revision + 1,
           updated_at = transaction_timestamp()
     WHERE roster_state.tenant_id = p_tenant
       AND roster_state.course_id = p_course
       AND roster_state.revision = v_roster_revision
    RETURNING roster_state.revision INTO v_roster_revision;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE = '55000';
    END IF;

    tenant_id := p_tenant;
    actor_id := v_actor;
    course_id := p_course;
    roster_revision := v_roster_revision;
    RETURN NEXT;
END
$$;

CREATE OR REPLACE FUNCTION public.ple_commit_course_roster_import_v1(
    p_tenant uuid,
    p_session character(64),
    p_course uuid,
    p_import uuid,
    p_expected bigint,
    p_key text,
    p_bindings jsonb
) RETURNS TABLE (
    tenant_id uuid,
    actor_id uuid,
    course_id uuid,
    roster_import_id uuid,
    import_revision bigint,
    roster_revision bigint
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
#variable_conflict error
DECLARE
    v_actor uuid;
    v_roster_revision bigint;
    v_import_revision bigint;
    v_import record;
    v_binding_count integer;
    v_ready_count integer;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL OR p_course IS NULL OR p_import IS NULL
       OR p_expected < 1 OR p_key IS NULL OR jsonb_typeof(p_bindings) <> 'array'
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'roster import commit arguments are invalid' USING ERRCODE = '22023';
    END IF;

    PERFORM 1
      FROM public.course AS course_row
     WHERE course_row.tenant_id = p_tenant
       AND course_row.course_id = p_course
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;

    SELECT roster_state.revision INTO v_roster_revision
      FROM public.course_roster_state AS roster_state
     WHERE roster_state.tenant_id = p_tenant
       AND roster_state.course_id = p_course
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course roster aggregate is invalid' USING ERRCODE = '55000';
    END IF;

    v_actor := public.ple_course_roster_support_actor(p_session, p_course, 'commitImport');
    IF v_actor IS NULL THEN RETURN; END IF;

    SELECT import_row.* INTO v_import
      FROM public.course_roster_import AS import_row
     WHERE import_row.tenant_id = p_tenant
       AND import_row.course_id = p_course
       AND import_row.roster_import_id = p_import
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;

    IF v_import.status = 'committed' THEN
        IF v_import.commit_idempotency_key IS DISTINCT FROM p_key THEN
            RAISE EXCEPTION 'roster import commit idempotency conflicts' USING ERRCODE = '55000';
        END IF;
        tenant_id := p_tenant;
        actor_id := v_actor;
        course_id := p_course;
        roster_import_id := p_import;
        import_revision := v_import.revision;
        roster_revision := v_import.committed_roster_revision;
        RETURN NEXT;
        RETURN;
    END IF;

    IF v_import.status <> 'preview' OR v_import.expires_at <= transaction_timestamp()
       OR v_import.revision <> p_expected OR v_import.roster_revision <> v_roster_revision THEN
        RAISE EXCEPTION 'roster import commit conflicts' USING ERRCODE = '55000';
    END IF;

    SELECT count(*) INTO v_ready_count
      FROM public.course_roster_import_row AS import_row
     WHERE import_row.tenant_id = p_tenant
       AND import_row.course_id = p_course
       AND import_row.roster_import_id = p_import
       AND import_row.row_status = 'ready_to_invite';
    SELECT count(*) INTO v_binding_count
      FROM jsonb_to_recordset(p_bindings) AS binding(
          row_number integer,
          token_hex text,
          idempotency_key text,
          lifetime bigint
      );

    IF v_binding_count <> v_ready_count
       OR v_binding_count <> (
           SELECT count(DISTINCT binding.row_number)
             FROM jsonb_to_recordset(p_bindings) AS binding(
                 row_number integer,
                 token_hex text,
                 idempotency_key text,
                 lifetime bigint
             )
       )
       OR EXISTS (
           SELECT 1
             FROM jsonb_to_recordset(p_bindings) AS binding(
                 row_number integer,
                 token_hex text,
                 idempotency_key text,
                 lifetime bigint
             )
            WHERE binding.row_number IS NULL
               OR binding.token_hex !~ '^[0-9a-f]{64}$'
               OR binding.idempotency_key IS NULL
               OR binding.lifetime < 1
               OR binding.lifetime > 2592000
       )
       OR EXISTS (
           (
               SELECT import_row.row_number
                 FROM public.course_roster_import_row AS import_row
                WHERE import_row.tenant_id = p_tenant
                  AND import_row.course_id = p_course
                  AND import_row.roster_import_id = p_import
                  AND import_row.row_status = 'ready_to_invite'
           )
           EXCEPT
           (
               SELECT binding.row_number
                 FROM jsonb_to_recordset(p_bindings) AS binding(
                     row_number integer,
                     token_hex text,
                     idempotency_key text,
                     lifetime bigint
                 )
           )
       ) THEN
        RAISE EXCEPTION 'roster import invitation bindings are invalid' USING ERRCODE = '22023';
    END IF;

    INSERT INTO public.course_invitation (
        tenant_id,
        course_id,
        invitation_id,
        token_hash,
        normalized_email,
        delivery_email,
        roster_id,
        invited_by,
        idempotency_key,
        expires_at,
        roster_import_id,
        roster_import_row_number
    )
    SELECT p_tenant,
           p_course,
           gen_random_uuid(),
           decode(binding.token_hex, 'hex'),
           import_row.normalized_email,
           import_row.delivery_email,
           import_row.roster_id,
           v_actor,
           binding.idempotency_key,
           transaction_timestamp() + binding.lifetime * interval '1 second',
           p_import,
           import_row.row_number
      FROM public.course_roster_import_row AS import_row
      JOIN jsonb_to_recordset(p_bindings) AS binding(
          row_number integer,
          token_hex text,
          idempotency_key text,
          lifetime bigint
      ) USING (row_number)
     WHERE import_row.tenant_id = p_tenant
       AND import_row.course_id = p_course
       AND import_row.roster_import_id = p_import
       AND import_row.row_status = 'ready_to_invite';

    INSERT INTO public.course_invitation_delivery (
        tenant_id,
        course_id,
        invitation_id,
        delivery_id
    )
    SELECT p_tenant, p_course, invitation.invitation_id, gen_random_uuid()
      FROM public.course_invitation AS invitation
     WHERE invitation.tenant_id = p_tenant
       AND invitation.course_id = p_course
       AND invitation.roster_import_id = p_import;

    UPDATE public.course_roster_state AS roster_state
       SET revision = roster_state.revision + 1,
           updated_at = transaction_timestamp()
     WHERE roster_state.tenant_id = p_tenant
       AND roster_state.course_id = p_course
       AND roster_state.revision = v_roster_revision
    RETURNING roster_state.revision INTO v_roster_revision;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE = '55000';
    END IF;

    UPDATE public.course_roster_import AS import_row
       SET status = 'committed',
           revision = import_row.revision + 1,
           commit_idempotency_key = p_key,
           committed_roster_revision = v_roster_revision,
           committed_at = transaction_timestamp()
     WHERE import_row.tenant_id = p_tenant
       AND import_row.course_id = p_course
       AND import_row.roster_import_id = p_import
       AND import_row.status = 'preview'
    RETURNING import_row.revision INTO v_import_revision;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'roster import transition is unavailable' USING ERRCODE = '55000';
    END IF;

    tenant_id := p_tenant;
    actor_id := v_actor;
    course_id := p_course;
    roster_import_id := p_import;
    import_revision := v_import_revision;
    roster_revision := v_roster_revision;
    RETURN NEXT;
END
$$;

CREATE OR REPLACE FUNCTION public.ple_revoke_course_invitation_v1(
    p_tenant uuid,
    p_session character(64),
    p_course uuid,
    p_invitation uuid,
    p_expected bigint
) RETURNS TABLE (
    tenant_id uuid,
    actor_id uuid,
    course_id uuid,
    invitation_id uuid,
    was_revoked boolean,
    roster_revision bigint
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
#variable_conflict error
DECLARE
    v_actor uuid;
    v_roster_revision bigint;
    v_invitation record;
BEGIN
    IF p_tenant IS NULL OR p_course IS NULL OR p_invitation IS NULL
       OR p_expected < 1 OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'invitation revocation arguments are invalid' USING ERRCODE = '22023';
    END IF;

    PERFORM 1
      FROM public.course AS course_row
     WHERE course_row.tenant_id = p_tenant
       AND course_row.course_id = p_course
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;

    SELECT roster_state.revision INTO v_roster_revision
      FROM public.course_roster_state AS roster_state
     WHERE roster_state.tenant_id = p_tenant
       AND roster_state.course_id = p_course
     FOR UPDATE;
    IF NOT FOUND OR v_roster_revision <> p_expected THEN
        RAISE EXCEPTION 'course roster revision conflicts' USING ERRCODE = '55000';
    END IF;

    v_actor := public.ple_course_roster_support_actor(
        p_session,
        p_course,
        'revokeInvitation'
    );
    IF v_actor IS NULL THEN RETURN; END IF;

    SELECT invitation.* INTO v_invitation
      FROM public.course_invitation AS invitation
     WHERE invitation.tenant_id = p_tenant
       AND invitation.course_id = p_course
       AND invitation.invitation_id = p_invitation
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;

    IF v_invitation.status = 'pending'
       AND v_invitation.expires_at <= transaction_timestamp() THEN
        UPDATE public.course_invitation AS invitation
           SET status = 'expired'
         WHERE invitation.tenant_id = p_tenant
           AND invitation.course_id = p_course
           AND invitation.invitation_id = p_invitation;
        v_invitation.status := 'expired';
    END IF;
    IF v_invitation.status = 'revoked' THEN
        tenant_id := p_tenant;
        actor_id := v_actor;
        course_id := p_course;
        invitation_id := p_invitation;
        was_revoked := true;
        roster_revision := v_roster_revision;
        RETURN NEXT;
        RETURN;
    END IF;
    IF v_invitation.status <> 'pending' THEN
        RAISE EXCEPTION 'invitation terminal conflict' USING ERRCODE = '55000';
    END IF;

    UPDATE public.course_invitation AS invitation
       SET status = 'revoked'
     WHERE invitation.tenant_id = p_tenant
       AND invitation.course_id = p_course
       AND invitation.invitation_id = p_invitation
       AND invitation.status = 'pending';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'invitation transition is unavailable' USING ERRCODE = '55000';
    END IF;

    UPDATE public.course_roster_state AS roster_state
       SET revision = roster_state.revision + 1,
           updated_at = transaction_timestamp()
     WHERE roster_state.tenant_id = p_tenant
       AND roster_state.course_id = p_course
       AND roster_state.revision = v_roster_revision
    RETURNING roster_state.revision INTO v_roster_revision;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE = '55000';
    END IF;

    tenant_id := p_tenant;
    actor_id := v_actor;
    course_id := p_course;
    invitation_id := p_invitation;
    was_revoked := false;
    roster_revision := v_roster_revision;
    RETURN NEXT;
END
$$;

CREATE OR REPLACE FUNCTION public.ple_revoke_course_student_as_roster_actor_v1(
    p_tenant uuid,
    p_session character(64),
    p_course uuid,
    p_member uuid,
    p_expected_revision bigint
) RETURNS TABLE (
    tenant_id uuid,
    actor_id uuid,
    course_id uuid,
    course_membership_id uuid,
    was_revoked boolean,
    roster_revision bigint
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
#variable_conflict error
DECLARE
    v_course uuid;
    v_roster_revision bigint;
    v_member uuid;
    v_role text;
    v_status text;
    v_actor uuid;
BEGIN
    IF p_tenant IS NULL OR p_session IS NULL OR p_course IS NULL OR p_member IS NULL
       OR p_expected_revision IS NULL OR p_expected_revision < 1
       OR p_tenant = '00000000-0000-0000-0000-000000000000'::uuid
       OR p_course = '00000000-0000-0000-0000-000000000000'::uuid
       OR p_member = '00000000-0000-0000-0000-000000000000'::uuid
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RAISE EXCEPTION 'course roster revocation arguments are invalid' USING ERRCODE = '22023';
    END IF;

    SELECT course_row.course_id INTO v_course
      FROM public.course AS course_row
     WHERE course_row.tenant_id = p_tenant
       AND course_row.course_id = p_course
       AND public.ple_course_records_accessible(
           course_row.tenant_id,
           course_row.course_id
       )
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course roster revocation is unavailable' USING ERRCODE = '42501';
    END IF;

    SELECT roster_state.revision INTO v_roster_revision
      FROM public.course_roster_state AS roster_state
     WHERE roster_state.tenant_id = p_tenant
       AND roster_state.course_id = p_course
     FOR UPDATE;
    IF NOT FOUND OR v_roster_revision <> p_expected_revision THEN
        RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE = '55000';
    END IF;

    SELECT member.course_membership_id, member.role, member.status
      INTO v_member, v_role, v_status
      FROM public.course_member AS member
     WHERE member.tenant_id = p_tenant
       AND member.course_id = p_course
       AND member.course_membership_id = p_member
     FOR UPDATE;
    IF NOT FOUND THEN RETURN; END IF;
    IF v_role <> 'student' THEN
        RAISE EXCEPTION 'course roster membership is invalid' USING ERRCODE = '55000';
    END IF;

    v_actor := public.ple_course_roster_support_actor(
        p_session,
        p_course,
        'revokeMember'
    );
    IF v_actor IS NULL THEN
        RAISE EXCEPTION 'course roster revocation actor is unavailable' USING ERRCODE = '42501';
    END IF;

    tenant_id := p_tenant;
    actor_id := v_actor;
    course_id := p_course;
    course_membership_id := v_member;
    IF v_status = 'revoked' THEN
        was_revoked := true;
        roster_revision := v_roster_revision;
        RETURN NEXT;
        RETURN;
    END IF;

    UPDATE public.course_member AS member
       SET status = 'revoked',
           revoked_at = transaction_timestamp()
     WHERE member.tenant_id = p_tenant
       AND member.course_id = p_course
       AND member.course_membership_id = v_member
       AND member.role = 'student'
       AND member.status = 'active';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course roster membership is invalid' USING ERRCODE = '55000';
    END IF;

    UPDATE public.course_roster_state AS roster_state
       SET revision = roster_state.revision + 1,
           updated_at = transaction_timestamp()
     WHERE roster_state.tenant_id = p_tenant
       AND roster_state.course_id = p_course
       AND roster_state.revision = v_roster_revision
    RETURNING roster_state.revision INTO v_roster_revision;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE = '55000';
    END IF;

    was_revoked := false;
    roster_revision := v_roster_revision;
    RETURN NEXT;
END
$$;

ALTER FUNCTION public.ple_replace_course_enrollment_policy_v1(
    uuid, character, uuid, bigint, text, jsonb
) OWNER TO ple_course_roster_policy_broker;
ALTER FUNCTION public.ple_commit_course_roster_import_v1(
    uuid, character, uuid, uuid, bigint, text, jsonb
) OWNER TO ple_course_roster_import_broker;
ALTER FUNCTION public.ple_revoke_course_invitation_v1(
    uuid, character, uuid, uuid, bigint
) OWNER TO ple_course_invitation_broker;
ALTER FUNCTION public.ple_revoke_course_student_as_roster_actor_v1(
    uuid, character, uuid, uuid, bigint
) OWNER TO ple_course_roster_mutator_broker;

REVOKE ALL ON FUNCTION public.ple_replace_course_enrollment_policy_v1(
    uuid, character, uuid, bigint, text, jsonb
) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_commit_course_roster_import_v1(
    uuid, character, uuid, uuid, bigint, text, jsonb
) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_revoke_course_invitation_v1(
    uuid, character, uuid, uuid, bigint
) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_revoke_course_student_as_roster_actor_v1(
    uuid, character, uuid, uuid, bigint
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_replace_course_enrollment_policy_v1(
    uuid, character, uuid, bigint, text, jsonb
) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_commit_course_roster_import_v1(
    uuid, character, uuid, uuid, bigint, text, jsonb
) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_revoke_course_invitation_v1(
    uuid, character, uuid, uuid, bigint
) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_revoke_course_student_as_roster_actor_v1(
    uuid, character, uuid, uuid, bigint
) TO ple_app;

DO $$
BEGIN
    IF has_function_privilege(
           'public',
           'public.ple_replace_course_enrollment_policy_v1(uuid,character,uuid,bigint,text,jsonb)'::regprocedure,
           'EXECUTE'
       )
       OR has_function_privilege(
           'public',
           'public.ple_commit_course_roster_import_v1(uuid,character,uuid,uuid,bigint,text,jsonb)'::regprocedure,
           'EXECUTE'
       )
       OR has_function_privilege(
           'public',
           'public.ple_revoke_course_invitation_v1(uuid,character,uuid,uuid,bigint)'::regprocedure,
           'EXECUTE'
       )
       OR has_function_privilege(
           'public',
           'public.ple_revoke_course_student_as_roster_actor_v1(uuid,character,uuid,uuid,bigint)'::regprocedure,
           'EXECUTE'
       ) THEN
        RAISE EXCEPTION 'course roster procedure repair widened public authority';
    END IF;
END
$$;

COMMIT;
