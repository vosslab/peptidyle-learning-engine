-- WP-PROF-T4: live rehearsal start has one explicit, durable restart intent.
-- ASVS 1.2.4, 2.2.1, 2.2.3, and 2.3.1-2.3.4: fixed capability inputs,
-- trusted server validation, locked workflow state, and atomic replacement.
BEGIN;

CREATE FUNCTION public.ple_rehearsal_start(
    p_tenant uuid, p_actor uuid, p_course uuid, p_assignment uuid,
    p_assignment_reference integer, p_revision bigint, p_subject_payload jsonb,
    p_subject_fingerprint bytea, p_genesis_digest bytea, p_run uuid,
    p_start_new_after_completion boolean, p_expected_latest_run uuid
) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    membership uuid;
    reference bigint;
    latest_run public.rehearsal_run%ROWTYPE;
BEGIN
    membership := public.ple_rehearsal_authorize_current(
        p_tenant, p_actor, p_course, p_assignment, p_revision
    );
    IF membership IS NULL OR p_run IS NULL OR p_assignment_reference <= 0
       OR p_start_new_after_completion IS NULL
       OR jsonb_typeof(p_subject_payload) <> 'object'
       OR public.ple_rehearsal_jsonb_bytes(p_subject_payload) > 65536
       OR octet_length(p_subject_fingerprint) <> 32
       OR octet_length(p_genesis_digest) <> 32 THEN
        RETURN NULL;
    END IF;

    -- The source locks from authorization precede this aggregate lock.  This
    -- single latest-row witness covers active and terminal histories alike.
    SELECT * INTO latest_run
      FROM public.rehearsal_run
     WHERE tenant_id = p_tenant AND course_id = p_course
       AND assignment_id = p_assignment
       AND direct_instructor_membership_id = membership
     ORDER BY rehearsal_reference DESC
     LIMIT 1
     FOR UPDATE;
    IF NOT FOUND THEN
        IF p_expected_latest_run IS NOT NULL THEN
            RETURN NULL;
        END IF;
    ELSIF latest_run.rehearsal_run_id IS DISTINCT FROM p_expected_latest_run THEN
        RETURN NULL;
    ELSIF latest_run.assignment_revision <> p_revision THEN
        RETURN NULL;
    ELSIF latest_run.lifecycle = 'active' THEN
        IF latest_run.subject_fingerprint = p_subject_fingerprint
           AND latest_run.actor_id = p_actor THEN
            RETURN latest_run.rehearsal_reference;
        END IF;

        -- Appending revocation events before the terminal transition preserves
        -- the immutable claim history for the live run being replaced.
        INSERT INTO public.rehearsal_submission_claim_event (
            tenant_id, rehearsal_run_id, claim_id, sequence, operation_id, generation, phase
        )
        SELECT root.tenant_id, root.rehearsal_run_id, root.claim_id,
               latest.sequence + 1, latest.operation_id, latest.generation,
               'revokedTerminalLifecycle'
          FROM public.rehearsal_submission_claim_root root
          CROSS JOIN LATERAL (
              SELECT event.sequence, event.operation_id, event.generation, event.phase
                FROM public.rehearsal_submission_claim_event event
               WHERE event.tenant_id = root.tenant_id
                 AND event.rehearsal_run_id = root.rehearsal_run_id
                 AND event.claim_id = root.claim_id
               ORDER BY event.sequence DESC
               LIMIT 1
          ) latest
         WHERE root.tenant_id = p_tenant
           AND root.rehearsal_run_id = latest_run.rehearsal_run_id
           AND latest.phase IN ('prepared', 'gradingDispatched');
        UPDATE public.rehearsal_run
           SET lifecycle = 'discardedByNewSubject',
               terminal_at = public.ple_rehearsal_now(),
               updated_at = public.ple_rehearsal_now()
         WHERE tenant_id = p_tenant
           AND rehearsal_run_id = latest_run.rehearsal_run_id
           AND lifecycle = 'active';
        IF NOT FOUND THEN
            RAISE EXCEPTION 'active rehearsal changed during replacement'
                USING ERRCODE = '55000';
        END IF;
    ELSIF latest_run.lifecycle = 'completed'
       AND p_start_new_after_completion IS NOT TRUE THEN
        RETURN NULL;
    END IF;

    INSERT INTO public.rehearsal_run (
        tenant_id, rehearsal_run_id, course_id, assignment_id, assignment_reference,
        direct_instructor_membership_id, actor_id, assignment_revision, subject_payload,
        subject_fingerprint, evidence_head_digest, evidence_length
    ) VALUES (
        p_tenant, p_run, p_course, p_assignment, p_assignment_reference, membership, p_actor,
        p_revision, p_subject_payload, p_subject_fingerprint, p_genesis_digest, 0
    ) RETURNING rehearsal_reference INTO reference;
    RETURN reference;
END
$$;

ALTER FUNCTION public.ple_rehearsal_start(
    uuid, uuid, uuid, uuid, integer, bigint, jsonb, bytea, bytea, uuid, boolean, uuid
) OWNER TO ple_rehearsal_broker;
REVOKE ALL ON FUNCTION public.ple_rehearsal_start(
    uuid, uuid, uuid, uuid, integer, bigint, jsonb, bytea, bytea, uuid, boolean, uuid
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_rehearsal_start(
    uuid, uuid, uuid, uuid, integer, bigint, jsonb, bytea, bytea, uuid, boolean, uuid
) TO ple_app;

-- PostgreSQL overloads by signature.  Retire the old capability rather than
-- leaving a callable path that silently lacks the explicit restart witness.
REVOKE ALL ON FUNCTION public.ple_rehearsal_start(
    uuid, uuid, uuid, uuid, integer, bigint, jsonb, bytea, bytea, uuid
) FROM PUBLIC, ple_app;
DROP FUNCTION public.ple_rehearsal_start(
    uuid, uuid, uuid, uuid, integer, bigint, jsonb, bytea, bytea, uuid
);

-- These broker-prelocked witnesses let the Store canonically verify ordinary
-- live data with plain reads.  No client payload or private evidence crosses
-- this authorization and lock boundary.
CREATE FUNCTION public.ple_prepare_rehearsal_start(
    p_tenant uuid, p_actor uuid, p_course uuid, p_assignment_reference integer,
    p_revision bigint, p_derived_membership uuid DEFAULT NULL
) RETURNS TABLE(
    assignment_id uuid,
    direct_instructor_membership_id uuid,
    derived_membership_id uuid,
    latest_rehearsal_run_id uuid,
    latest_rehearsal_reference bigint,
    latest_assignment_revision bigint
)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    assignment_value uuid;
    instructor_value uuid;
    derived_value uuid;
    latest_value public.rehearsal_run%ROWTYPE;
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_actor IS NULL OR p_course IS NULL OR p_assignment_reference <= 0
       OR p_revision IS NULL OR p_revision <= 0 THEN
        RETURN;
    END IF;

    SELECT assignment_record.assignment_id INTO assignment_value
      FROM public.assignment assignment_record
     WHERE assignment_record.tenant_id = p_tenant
       AND assignment_record.course_id = p_course
       AND assignment_record.public_id = p_assignment_reference
       AND assignment_record.revision = p_revision
     FOR UPDATE;
    IF assignment_value IS NULL THEN
        RETURN;
    END IF;

    -- Lock direct Instructor and optional derived learner memberships in one
    -- UUID-ordered query after the assignment lock.
    PERFORM 1
      FROM public.course_member member
     WHERE member.tenant_id = p_tenant AND member.course_id = p_course
       AND ((member.user_id = p_actor AND member.role = 'instructor'
             AND member.status = 'active')
            OR (p_derived_membership IS NOT NULL
                AND member.course_membership_id = p_derived_membership
                AND member.role = 'student' AND member.status = 'active'))
     ORDER BY member.course_membership_id
     FOR UPDATE;
    SELECT member.course_membership_id INTO instructor_value
      FROM public.course_member member
     WHERE member.tenant_id = p_tenant AND member.course_id = p_course
       AND member.user_id = p_actor AND member.role = 'instructor'
       AND member.status = 'active';
    IF instructor_value IS NULL THEN
        RETURN;
    END IF;
    IF p_derived_membership IS NOT NULL THEN
        SELECT member.course_membership_id INTO derived_value
          FROM public.course_member member
         WHERE member.tenant_id = p_tenant AND member.course_id = p_course
           AND member.course_membership_id = p_derived_membership
           AND member.role = 'student' AND member.status = 'active';
        IF derived_value IS NULL THEN
            RETURN;
        END IF;
    END IF;

    SELECT * INTO latest_value
      FROM public.rehearsal_run run
     WHERE run.tenant_id = p_tenant AND run.course_id = p_course
       AND run.assignment_id = assignment_value
       AND run.direct_instructor_membership_id = instructor_value
     ORDER BY run.rehearsal_reference DESC
     LIMIT 1
     FOR UPDATE;
    RETURN QUERY
    SELECT assignment_value, instructor_value, derived_value,
           latest_value.rehearsal_run_id, latest_value.rehearsal_reference,
           latest_value.assignment_revision;
END
$$;

CREATE FUNCTION public.ple_prepare_rehearsal_operation(
    p_tenant uuid, p_actor uuid, p_course uuid, p_assignment_reference integer,
    p_revision bigint, p_rehearsal_reference bigint
) RETURNS TABLE(
    assignment_id uuid,
    direct_instructor_membership_id uuid,
    rehearsal_run_id uuid
)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    assignment_value uuid;
    instructor_value uuid;
    run_value uuid;
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_actor IS NULL OR p_course IS NULL OR p_assignment_reference <= 0
       OR p_revision IS NULL OR p_revision <= 0
       OR p_rehearsal_reference IS NULL OR p_rehearsal_reference <= 0 THEN
        RETURN;
    END IF;

    SELECT assignment_record.assignment_id INTO assignment_value
      FROM public.assignment assignment_record
     WHERE assignment_record.tenant_id = p_tenant
       AND assignment_record.course_id = p_course
       AND assignment_record.public_id = p_assignment_reference
       AND assignment_record.revision = p_revision
     FOR UPDATE;
    IF assignment_value IS NULL THEN
        RETURN;
    END IF;
    PERFORM 1
      FROM public.course_member member
     WHERE member.tenant_id = p_tenant AND member.course_id = p_course
       AND member.user_id = p_actor AND member.role = 'instructor'
       AND member.status = 'active'
     ORDER BY member.course_membership_id
     FOR UPDATE;
    SELECT member.course_membership_id INTO instructor_value
      FROM public.course_member member
     WHERE member.tenant_id = p_tenant AND member.course_id = p_course
       AND member.user_id = p_actor AND member.role = 'instructor'
       AND member.status = 'active';
    IF instructor_value IS NULL THEN
        RETURN;
    END IF;

    SELECT run.rehearsal_run_id INTO run_value
      FROM public.rehearsal_run run
     WHERE run.tenant_id = p_tenant AND run.course_id = p_course
       AND run.assignment_id = assignment_value
       AND run.direct_instructor_membership_id = instructor_value
       AND run.actor_id = p_actor AND run.assignment_revision = p_revision
       AND run.rehearsal_reference = p_rehearsal_reference
     FOR UPDATE;
    IF run_value IS NULL THEN
        RETURN;
    END IF;
    RETURN QUERY SELECT assignment_value, instructor_value, run_value;
END
$$;

ALTER FUNCTION public.ple_prepare_rehearsal_start(
    uuid, uuid, uuid, integer, bigint, uuid
) OWNER TO ple_rehearsal_broker;
ALTER FUNCTION public.ple_prepare_rehearsal_operation(
    uuid, uuid, uuid, integer, bigint, bigint
) OWNER TO ple_rehearsal_broker;
REVOKE ALL ON FUNCTION public.ple_prepare_rehearsal_start(
    uuid, uuid, uuid, integer, bigint, uuid
) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_prepare_rehearsal_operation(
    uuid, uuid, uuid, integer, bigint, bigint
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_prepare_rehearsal_start(
    uuid, uuid, uuid, integer, bigint, uuid
) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_prepare_rehearsal_operation(
    uuid, uuid, uuid, integer, bigint, bigint
) TO ple_app;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM pg_proc procedure
          JOIN pg_namespace namespace ON namespace.oid = procedure.pronamespace
          CROSS JOIN LATERAL aclexplode(
              COALESCE(procedure.proacl, acldefault('f', procedure.proowner))
          ) privilege
         WHERE namespace.nspname = 'public'
           AND procedure.oid = (
               'public.ple_rehearsal_start(uuid,uuid,uuid,uuid,integer,bigint,'
               || 'jsonb,bytea,bytea,uuid,boolean,uuid)'
           )::regprocedure
           AND privilege.grantee = 0
           AND privilege.privilege_type = 'EXECUTE'
    ) OR NOT has_function_privilege(
        'ple_app',
        (
            'public.ple_rehearsal_start(uuid,uuid,uuid,uuid,integer,bigint,'
            || 'jsonb,bytea,bytea,uuid,boolean,uuid)'
        )::regprocedure,
        'EXECUTE'
    ) OR to_regprocedure(
        (
            'public.ple_rehearsal_start(uuid,uuid,uuid,uuid,integer,bigint,'
            || 'jsonb,bytea,bytea,uuid)'
        )
    ) IS NOT NULL THEN
        RAISE EXCEPTION 'rehearsal start capability grant inventory is unsafe'
            USING ERRCODE = '42501';
    END IF;
    IF has_function_privilege(
        'ple_app',
        'public.ple_prepare_rehearsal_start(uuid,uuid,uuid,integer,bigint,uuid)',
        'EXECUTE'
    ) IS FALSE OR has_function_privilege(
        'ple_app',
        'public.ple_prepare_rehearsal_operation(uuid,uuid,uuid,integer,bigint,bigint)',
        'EXECUTE'
    ) IS FALSE OR has_table_privilege('ple_app', 'public.rehearsal_run', 'UPDATE')
      OR has_table_privilege('ple_app', 'public.assignment', 'UPDATE')
      OR has_table_privilege('ple_app', 'public.course_member', 'UPDATE') THEN
        RAISE EXCEPTION 'rehearsal preparation privilege inventory is unsafe'
            USING ERRCODE = '42501';
    END IF;
END
$$;

COMMIT;
