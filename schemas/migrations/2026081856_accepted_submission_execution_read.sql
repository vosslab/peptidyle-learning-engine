-- WP-PROF-G1 / G1-W4: verified canonical automated-result read.
-- This capability exposes one safe projection after the database verifies the
-- complete route and the worker-owned canonical evidence. Actor entitlement
-- is established by the caller earlier in the same database transaction.

BEGIN;

CREATE FUNCTION public.ple_read_accepted_submission_evaluation_v1(
    p_tenant_id uuid,
    p_course_id uuid,
    p_assignment_id uuid,
    p_attempt_id uuid
) RETURNS TABLE (
    evaluation_payload jsonb
)
LANGUAGE plpgsql
VOLATILE
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    v_canonical_source text;
    v_canonical_digest character(64);
    v_stored_projection jsonb;
    v_projection_digest character(64);
    v_source_projection jsonb;
BEGIN
    -- ASVS 2.2.1-2.2.3 and 8.4.1: bind every route identifier to the
    -- current tenant before reading worker-owned evidence. This function is
    -- route and integrity verification, while the caller owns actor authority.
    IF p_tenant_id IS NULL
       OR p_course_id IS NULL
       OR p_assignment_id IS NULL
       OR p_attempt_id IS NULL
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant()
    THEN
        RETURN;
    END IF;

    -- ASVS 2.3.1 and 8.2.1-8.2.3: accept only one route-consistent,
    -- completed, graded aggregate with its immutable receipt present.
    BEGIN
        SELECT
            evaluation.automated_result_canonical_json,
            evaluation.automated_result_sha256,
            evaluation.payload,
            evaluation.payload_sha256
          INTO STRICT
            v_canonical_source,
            v_canonical_digest,
            v_stored_projection,
            v_projection_digest
          FROM public.submission_evaluation AS evaluation
          JOIN public.grading_execution AS execution
            ON execution.tenant_id = evaluation.tenant_id
           AND execution.attempt_id = evaluation.attempt_id
           AND execution.submission_id = evaluation.submission_id
          JOIN public.question_attempt AS attempt
            ON attempt.tenant_id = evaluation.tenant_id
           AND attempt.attempt_id = evaluation.attempt_id
          JOIN public.assignment_run AS run
            ON run.tenant_id = attempt.tenant_id
           AND run.run_id = attempt.run_id
          JOIN public.enrollment AS enrollment
            ON enrollment.tenant_id = run.tenant_id
           AND enrollment.enrollment_id = run.enrollment_id
          JOIN public.assignment AS assignment
            ON assignment.tenant_id = enrollment.tenant_id
           AND assignment.assignment_id = enrollment.assignment_id
          JOIN public.submission_receipt_snapshot AS receipt
            ON receipt.tenant_id = evaluation.tenant_id
           AND receipt.attempt_id = evaluation.attempt_id
         WHERE evaluation.tenant_id = p_tenant_id
           AND evaluation.attempt_id = p_attempt_id
           AND evaluation.course_id = p_course_id
           AND execution.course_id = p_course_id
           AND attempt.course_id = p_course_id
           AND assignment.course_id = p_course_id
           AND enrollment.assignment_id = p_assignment_id
           AND assignment.assignment_id = p_assignment_id
           AND execution.state = 'completed'
           AND attempt.attempt_status = 'submitted'
           AND evaluation.grading_status = 'graded'
           AND evaluation.automated_result_canonical_json_version = 1;
    EXCEPTION
        WHEN no_data_found OR too_many_rows THEN
            RETURN;
    END;

    -- ASVS 11.4.1 and 11.4.3: SHA-256 attests to the exact canonical
    -- UTF-8 source. The projection digest names those same source bytes; JSONB
    -- serialization is never substituted as evidence.
    IF v_canonical_source IS NULL
       OR octet_length(v_canonical_source) NOT BETWEEN 1 AND 524288
       OR v_canonical_digest IS NULL
       OR v_canonical_digest IS DISTINCT FROM encode(
            pg_catalog.sha256(convert_to(v_canonical_source, 'UTF8')),
            'hex'
       )
       OR v_projection_digest IS DISTINCT FROM v_canonical_digest
    THEN
        RETURN;
    END IF;

    -- ASVS 15.3.1 and 16.5.3: a malformed canonical source or a projection
    -- disagreement closes the read while the function returns no private
    -- source, digest, worker, execution, or submission fields.
    BEGIN
        v_source_projection := v_canonical_source::jsonb;
    EXCEPTION
        WHEN invalid_text_representation THEN
            RETURN;
    END;

    IF v_source_projection IS DISTINCT FROM v_stored_projection THEN
        RETURN;
    END IF;

    evaluation_payload := v_stored_projection;
    RETURN NEXT;
END;
$$;

ALTER FUNCTION public.ple_read_accepted_submission_evaluation_v1(
    uuid, uuid, uuid, uuid
)
    OWNER TO ple_accepted_submission_execution_worker;

REVOKE ALL ON FUNCTION
    public.ple_read_accepted_submission_evaluation_v1(uuid, uuid, uuid, uuid)
    FROM PUBLIC;
REVOKE ALL ON FUNCTION
    public.ple_read_accepted_submission_evaluation_v1(uuid, uuid, uuid, uuid)
    FROM ple_app;
REVOKE ALL ON FUNCTION
    public.ple_read_accepted_submission_evaluation_v1(uuid, uuid, uuid, uuid)
    FROM ple_auth;
REVOKE ALL ON FUNCTION
    public.ple_read_accepted_submission_evaluation_v1(uuid, uuid, uuid, uuid)
    FROM ple_student;
REVOKE ALL ON FUNCTION
    public.ple_read_accepted_submission_evaluation_v1(uuid, uuid, uuid, uuid)
    FROM ple_grader;
REVOKE ALL ON FUNCTION
    public.ple_read_accepted_submission_evaluation_v1(uuid, uuid, uuid, uuid)
    FROM ple_grading_reader;
REVOKE ALL ON FUNCTION
    public.ple_read_accepted_submission_evaluation_v1(uuid, uuid, uuid, uuid)
    FROM ple_queue_broker;
REVOKE ALL ON FUNCTION
    public.ple_read_accepted_submission_evaluation_v1(uuid, uuid, uuid, uuid)
    FROM ple_automated_grading_broker;
REVOKE ALL ON FUNCTION
    public.ple_read_accepted_submission_evaluation_v1(uuid, uuid, uuid, uuid)
    FROM ple_accepted_submission_execution;
REVOKE ALL ON FUNCTION
    public.ple_read_accepted_submission_evaluation_v1(uuid, uuid, uuid, uuid)
    FROM ple_accepted_submission_execution_reader;
REVOKE ALL ON FUNCTION
    public.ple_read_accepted_submission_evaluation_v1(uuid, uuid, uuid, uuid)
    FROM ple_retention_broker;

GRANT EXECUTE ON FUNCTION
    public.ple_read_accepted_submission_evaluation_v1(uuid, uuid, uuid, uuid)
    TO ple_app;

-- The catalog assertion treats the function signature and its non-owner ACL
-- as complete sets. A default privilege therefore cannot widen this boundary.
DO $$
DECLARE
    v_function regprocedure :=
        'public.ple_read_accepted_submission_evaluation_v1('
        'uuid,uuid,uuid,uuid)'::regprocedure;
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
          JOIN pg_catalog.pg_namespace AS namespace_row
            ON namespace_row.oid = procedure_row.pronamespace
         WHERE procedure_row.oid = v_function
           AND namespace_row.nspname = 'public'
           AND procedure_row.proname =
               'ple_read_accepted_submission_evaluation_v1'
           AND procedure_row.prokind = 'f'
           AND procedure_row.proowner =
               'ple_accepted_submission_execution_worker'::regrole
           AND procedure_row.prosecdef
           AND procedure_row.provolatile = 'v'
           AND procedure_row.proparallel = 'u'
           AND NOT procedure_row.proleakproof
           AND NOT procedure_row.proisstrict
           AND procedure_row.proretset
           AND procedure_row.prorettype = 'jsonb'::regtype
           AND procedure_row.pronargs = 4
           AND procedure_row.pronargdefaults = 0
           AND procedure_row.proargtypes[0] = 'uuid'::regtype
           AND procedure_row.proargtypes[1] = 'uuid'::regtype
           AND procedure_row.proargtypes[2] = 'uuid'::regtype
           AND procedure_row.proargtypes[3] = 'uuid'::regtype
           AND procedure_row.proallargtypes IS NOT DISTINCT FROM ARRAY[
                'uuid'::regtype,
                'uuid'::regtype,
                'uuid'::regtype,
                'uuid'::regtype,
                'jsonb'::regtype
           ]::oid[]
           AND procedure_row.proargmodes IS NOT DISTINCT FROM ARRAY[
                'i'::"char",
                'i'::"char",
                'i'::"char",
                'i'::"char",
                't'::"char"
           ]
           AND procedure_row.proargnames IS NOT DISTINCT FROM ARRAY[
                'p_tenant_id',
                'p_course_id',
                'p_assignment_id',
                'p_attempt_id',
                'evaluation_payload'
           ]::text[]
           AND procedure_row.proconfig IS NOT DISTINCT FROM ARRAY[
                'search_path=pg_catalog, public, pg_temp'
           ]::text[]
    ) THEN
        RAISE EXCEPTION 'verified evaluation reader catalog is unsafe';
    END IF;

    IF EXISTS (
        WITH expected(grantee, privilege_type, is_grantable) AS (
            VALUES ('ple_app'::regrole::oid, 'EXECUTE'::text, false)
        ),
        actual AS (
            SELECT
                privilege.grantee,
                privilege.privilege_type,
                privilege.is_grantable
              FROM pg_catalog.pg_proc AS procedure_row
              CROSS JOIN LATERAL pg_catalog.aclexplode(
                  COALESCE(
                      procedure_row.proacl,
                      pg_catalog.acldefault('f', procedure_row.proowner)
                  )
              ) AS privilege
             WHERE procedure_row.oid = v_function
               AND privilege.grantee <> procedure_row.proowner
        )
        SELECT 1
          FROM (
              (SELECT * FROM expected EXCEPT SELECT * FROM actual)
              UNION ALL
              (SELECT * FROM actual EXCEPT SELECT * FROM expected)
          ) AS privilege_difference
    ) THEN
        RAISE EXCEPTION 'verified evaluation reader execute ACL is unsafe';
    END IF;
END;
$$;

COMMIT;
