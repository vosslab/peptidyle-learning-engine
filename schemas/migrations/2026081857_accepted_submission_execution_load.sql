-- WP-PROF-G1 / G1-W4: exact private execution-input capability.
--
-- The common accepted-submission handler may read private grading material only
-- after it owns the full current worker lease tuple. This wrapper returns no
-- rows for an inactive, stale, cross-tenant, or otherwise incoherent claim.

BEGIN;

-- ASVS V2.2, V2.3, V8.2, V8.3, V8.4, and V15.3: validate the complete
-- lease-bound identity at the database boundary, then return only the private
-- material needed by the server-owned grading handler.
CREATE FUNCTION public.ple_load_accepted_submission_execution_v2(
    p_tenant_id uuid,
    p_worker_job_id uuid,
    p_lease_token uuid,
    p_submission_id uuid,
    p_execution_generation bigint,
    p_worker_id uuid
) RETURNS TABLE(
    worker_job_id uuid,
    worker_lease_token uuid,
    execution_generation bigint,
    worker_id uuid,
    execution_state text,
    accepted_tenant_id uuid,
    accepted_course_id uuid,
    accepted_assignment_id uuid,
    accepted_attempt_id uuid,
    accepted_submission_id uuid,
    accepted_actor_id uuid,
    accepted_idempotency_key text,
    accepted_request_sha256 character(64),
    accepted_millis bigint,
    response_canonical_json text,
    attempt_payload jsonb,
    attempt_payload_sha256 character(64),
    presentation_descriptor_version smallint,
    presentation_nonce bytea,
    presentation_digest bytea,
    presentation_capability text,
    presentation_payload jsonb,
    presentation_payload_sha256 character(64),
    grading_envelope_payload jsonb,
    grading_envelope_payload_sha256 character(64),
    issued_question_snapshot_payload jsonb,
    issued_question_snapshot_payload_sha256 character(64),
    flat_required boolean,
    flat_payload jsonb,
    flat_payload_sha256 character(64),
    webwork_required boolean,
    webwork_payload jsonb,
    webwork_payload_sha256 character(64),
    webwork_replay_payload jsonb,
    webwork_replay_payload_sha256 character(64),
    qti_required boolean,
    qti_payload bytea,
    qti_payload_sha256 character(64)
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
BEGIN
    IF p_tenant_id IS NULL
       OR p_worker_job_id IS NULL
       OR p_lease_token IS NULL
       OR p_submission_id IS NULL
       OR p_execution_generation IS NULL
       OR p_execution_generation <= 0
       OR p_worker_id IS NULL
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant()
    THEN
        RETURN;
    END IF;

    RETURN QUERY
    SELECT
        witness.current_job_id,
        witness.lease_token,
        witness.execution_generation,
        p_worker_id,
        witness.execution_state,
        witness.tenant_id,
        witness.accepted_course_id,
        witness.assignment_id,
        witness.attempt_id,
        witness.submission_id,
        witness.accepted_actor_id,
        witness.accepted_idempotency_key,
        witness.request_sha256,
        witness.accepted_millis,
        witness.response_canonical_json,
        witness.attempt_payload,
        witness.attempt_payload_sha256,
        witness.presentation_descriptor_version,
        witness.presentation_nonce,
        witness.presentation_digest,
        witness.presentation_capability,
        witness.presentation_payload,
        witness.presentation_payload_sha256,
        witness.grading_envelope_payload,
        witness.grading_envelope_payload_sha256,
        witness.issued_question_snapshot_payload,
        witness.issued_question_snapshot_payload_sha256,
        witness.flat_required,
        witness.flat_payload,
        witness.flat_payload_sha256,
        witness.webwork_required,
        witness.webwork_payload,
        witness.webwork_payload_sha256,
        witness.webwork_replay_payload,
        witness.webwork_replay_payload_sha256,
        witness.qti_required,
        witness.qti_payload,
        witness.qti_payload_sha256
      FROM public.ple_accepted_submission_execution_witness_v1 AS witness
     WHERE witness.tenant_id = p_tenant_id
       AND witness.current_job_id = p_worker_job_id
       AND witness.submission_id = p_submission_id
       AND witness.execution_generation = p_execution_generation
       AND witness.execution_state = 'running'
       AND witness.active_worker_id = p_worker_id
       AND witness.job_state = 'leased'
       AND witness.lease_token = p_lease_token
       AND witness.lease_expires_at > transaction_timestamp()
       AND witness.retention_lifecycle = 'active'
       AND witness.grading_status = 'automated_pending'
       AND witness.automated_result_canonical_json IS NULL
       AND witness.automated_result_sha256 IS NULL;
END;
$$;

ALTER FUNCTION public.ple_load_accepted_submission_execution_v2(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid
) OWNER TO ple_accepted_submission_execution_worker;

REVOKE ALL ON FUNCTION public.ple_load_accepted_submission_execution_v2(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION public.ple_load_accepted_submission_execution_v2(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid
) TO ple_accepted_submission_execution;

GRANT EXECUTE ON FUNCTION public.ple_load_accepted_submission_execution_v2(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid
) TO ple_accepted_submission_execution_fast_path;

-- The focused authority assertion makes this narrow callable boundary durable.
DO $$
DECLARE
    v_function regprocedure :=
        'public.ple_load_accepted_submission_execution_v2(uuid,uuid,uuid,uuid,bigint,uuid)'
        ::regprocedure;
BEGIN
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
         WHERE procedure_row.oid = v_function
           AND (
               procedure_row.proowner
                   <> 'ple_accepted_submission_execution_worker'::regrole
               OR NOT procedure_row.prosecdef
               OR procedure_row.proconfig
                   IS DISTINCT FROM ARRAY['search_path=pg_catalog, public, pg_temp']
               OR procedure_row.proargnames IS DISTINCT FROM ARRAY[
                   'p_tenant_id',
                   'p_worker_job_id',
                   'p_lease_token',
                   'p_submission_id',
                   'p_execution_generation',
                   'p_worker_id',
                   'worker_job_id',
                   'worker_lease_token',
                   'execution_generation',
                   'worker_id',
                   'execution_state',
                   'accepted_tenant_id',
                   'accepted_course_id',
                   'accepted_assignment_id',
                   'accepted_attempt_id',
                   'accepted_submission_id',
                   'accepted_actor_id',
                   'accepted_idempotency_key',
                   'accepted_request_sha256',
                   'accepted_millis',
                   'response_canonical_json',
                   'attempt_payload',
                   'attempt_payload_sha256',
                   'presentation_descriptor_version',
                   'presentation_nonce',
                   'presentation_digest',
                   'presentation_capability',
                   'presentation_payload',
                   'presentation_payload_sha256',
                   'grading_envelope_payload',
                   'grading_envelope_payload_sha256',
                   'issued_question_snapshot_payload',
                   'issued_question_snapshot_payload_sha256',
                   'flat_required',
                   'flat_payload',
                   'flat_payload_sha256',
                   'webwork_required',
                   'webwork_payload',
                   'webwork_payload_sha256',
                   'webwork_replay_payload',
                   'webwork_replay_payload_sha256',
                   'qti_required',
                   'qti_payload',
                   'qti_payload_sha256'
               ]
               OR procedure_row.proargmodes IS DISTINCT FROM ARRAY[
                   'i'::"char", 'i'::"char", 'i'::"char", 'i'::"char",
                   'i'::"char", 'i'::"char", 't'::"char", 't'::"char",
                   't'::"char", 't'::"char", 't'::"char", 't'::"char",
                   't'::"char", 't'::"char", 't'::"char", 't'::"char",
                   't'::"char", 't'::"char", 't'::"char", 't'::"char",
                   't'::"char", 't'::"char", 't'::"char", 't'::"char",
                   't'::"char", 't'::"char", 't'::"char", 't'::"char",
                   't'::"char", 't'::"char", 't'::"char", 't'::"char",
                   't'::"char", 't'::"char", 't'::"char", 't'::"char",
                   't'::"char", 't'::"char", 't'::"char", 't'::"char",
                   't'::"char", 't'::"char", 't'::"char", 't'::"char"
               ]
               OR procedure_row.proallargtypes IS DISTINCT FROM ARRAY[
                   'uuid'::regtype,
                   'uuid'::regtype,
                   'uuid'::regtype,
                   'uuid'::regtype,
                   'bigint'::regtype,
                   'uuid'::regtype,
                   'uuid'::regtype,
                   'uuid'::regtype,
                   'bigint'::regtype,
                   'uuid'::regtype,
                   'text'::regtype,
                   'uuid'::regtype,
                   'uuid'::regtype,
                   'uuid'::regtype,
                   'uuid'::regtype,
                   'uuid'::regtype,
                   'uuid'::regtype,
                   'text'::regtype,
                   'character'::regtype,
                   'bigint'::regtype,
                   'text'::regtype,
                   'jsonb'::regtype,
                   'character'::regtype,
                   'smallint'::regtype,
                   'bytea'::regtype,
                   'bytea'::regtype,
                   'text'::regtype,
                   'jsonb'::regtype,
                   'character'::regtype,
                   'jsonb'::regtype,
                   'character'::regtype,
                   'jsonb'::regtype,
                   'character'::regtype,
                   'boolean'::regtype,
                   'jsonb'::regtype,
                   'character'::regtype,
                   'boolean'::regtype,
                   'jsonb'::regtype,
                   'character'::regtype,
                   'jsonb'::regtype,
                   'character'::regtype,
                   'boolean'::regtype,
                   'bytea'::regtype,
                   'character'::regtype
               ]::oid[]
           )
    )
       OR NOT has_function_privilege(
           'ple_accepted_submission_execution',
           v_function,
           'EXECUTE'
       )
       OR EXISTS (
           WITH expected_acl AS (
               SELECT role_name::regrole::oid AS grantee,
                      'EXECUTE'::text AS privilege_type,
                      false AS is_grantable
                 FROM unnest(ARRAY[
                     'ple_accepted_submission_execution',
                     'ple_accepted_submission_execution_fast_path'
                 ]) AS expected_role(role_name)
           ),
           actual_acl AS (
               SELECT acl.grantee,
                      acl.privilege_type,
                      acl.is_grantable
                 FROM pg_catalog.pg_proc AS procedure_row
                CROSS JOIN LATERAL pg_catalog.aclexplode(
                    COALESCE(
                        procedure_row.proacl,
                        pg_catalog.acldefault('f', procedure_row.proowner)
                    )
                ) AS acl
                WHERE procedure_row.oid = v_function
                  AND acl.grantee <> procedure_row.proowner
           )
           SELECT 1
             FROM (
                 (SELECT * FROM actual_acl EXCEPT SELECT * FROM expected_acl)
                 UNION ALL
                 (SELECT * FROM expected_acl EXCEPT SELECT * FROM actual_acl)
             ) AS privilege_difference
       )
    THEN
        RAISE EXCEPTION 'accepted-submission execution load capability is unsafe';
    END IF;
END;
$$;

COMMIT;
