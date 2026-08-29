-- G2-W5: align the installed Base Course completion witness with accepted submissions.
BEGIN;

CREATE FUNCTION public.ple_claim_exact_worker_job_v1(
    p_job uuid,
    p_token uuid,
    p_lease_seconds integer,
    p_kind text
) RETURNS TABLE(
    job_id uuid,
    tenant_id uuid,
    payload jsonb,
    lease_token uuid,
    attempt_count integer
)
LANGUAGE plpgsql
VOLATILE
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
BEGIN
    IF p_job IS NULL
       OR p_token IS NULL
       OR p_lease_seconds NOT BETWEEN 1 AND 900
       OR p_kind NOT IN (
            'recalculateAssignment',
            'recalculateCourseItemAnalysis',
            'autoSubmitAttempt',
            'retention',
            'render',
            'export',
            'import',
            'qtiImport',
            'publishPublicAssets'
       )
    THEN
        RAISE EXCEPTION 'invalid exact queue claim arguments' USING ERRCODE = '22023';
    END IF;

    UPDATE public.worker_job AS expired
       SET state = 'dead',
           lease_token = NULL,
           lease_expires_at = NULL,
           last_error = 'timed_out',
           completed_at = transaction_timestamp()
     WHERE expired.job_id = p_job
       AND expired.payload ->> 'kind' = p_kind
       AND expired.state = 'leased'
       AND expired.lease_expires_at <= transaction_timestamp()
       AND expired.attempt_count >= expired.max_attempts;

    RETURN QUERY
    WITH candidate AS (
        SELECT queued.job_id
          FROM public.worker_job AS queued
         WHERE queued.job_id = p_job
           AND queued.payload ->> 'kind' = p_kind
           AND (
                (queued.state = 'ready' AND queued.available_at <= transaction_timestamp())
                OR (
                    queued.state = 'leased'
                    AND queued.lease_expires_at <= transaction_timestamp()
                    AND queued.attempt_count < queued.max_attempts
                )
           )
         FOR UPDATE SKIP LOCKED
    ), claimed AS (
        UPDATE public.worker_job AS queued
           SET state = 'leased',
               lease_token = p_token,
               lease_expires_at = transaction_timestamp()
                    + make_interval(secs => p_lease_seconds),
               attempt_count = queued.attempt_count + 1,
               last_error = NULL,
               completed_at = NULL
          FROM candidate
         WHERE queued.job_id = candidate.job_id
        RETURNING queued.job_id,
                  queued.tenant_id,
                  queued.payload,
                  queued.lease_token,
                  queued.attempt_count
    )
    SELECT * FROM claimed;
END
$$;

ALTER FUNCTION public.ple_claim_exact_worker_job_v1(uuid, uuid, integer, text)
    OWNER TO ple_queue_broker;
REVOKE ALL ON FUNCTION public.ple_claim_exact_worker_job_v1(uuid, uuid, integer, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_claim_exact_worker_job_v1(uuid, uuid, integer, text)
    TO ple_app;

CREATE FUNCTION public.ple_verify_base_course_accepted_private_response_v1(
    p_tenant uuid,
    p_course uuid,
    p_attempt uuid,
    p_submission uuid
) RETURNS boolean
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
RETURN (
    SELECT count(*) = 1
       AND bool_and(
            response_sha256 = encode(
                pg_catalog.sha256(convert_to(response_canonical_json, 'UTF8')),
                'hex'
            )
            AND jsonb_typeof(response_canonical_json::jsonb) = 'object'
            AND response_canonical_json::jsonb ->> 'kind' = 'multipleChoice'
            AND jsonb_typeof(response_canonical_json::jsonb -> 'selected') = 'array'
            AND jsonb_array_length(response_canonical_json::jsonb -> 'selected') = 1
            AND jsonb_typeof(response_canonical_json::jsonb -> 'selected' -> 0) = 'string'
            AND response_canonical_json::jsonb -> 'selected' ->> 0 <> ''
        )
      FROM public.accepted_submission_private_response
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND attempt_id = p_attempt
       AND submission_id = p_submission
);

ALTER FUNCTION public.ple_verify_base_course_accepted_private_response_v1(
    uuid, uuid, uuid, uuid
) OWNER TO ple_base_course_completion_verification_broker;
REVOKE ALL ON FUNCTION public.ple_verify_base_course_accepted_private_response_v1(
    uuid, uuid, uuid, uuid
) FROM PUBLIC;

GRANT SELECT (
    tenant_id,
    course_id,
    attempt_id,
    submission_id,
    response_canonical_json,
    response_sha256
) ON public.accepted_submission_private_response
TO ple_base_course_completion_verification_broker;

CREATE POLICY base_course_completion_accepted_response_select
    ON public.accepted_submission_private_response
    FOR SELECT
    TO ple_base_course_completion_verification_broker
    USING (true);

DO $migration$
DECLARE
    definition text;
    old_clause CONSTANT text :=
        'payload=jsonb_build_object(''kind'',''multipleChoice'',''selected'',jsonb_build_array(''amide''))';
    new_clause CONSTANT text :=
        'payload=jsonb_build_object(''kind'',''acceptedPrivateResponseV1'') '
        'AND public.ple_verify_base_course_accepted_private_response_v1(p_tenant,bc,ma,sub)';
    old_evaluation_revision CONSTANT text :=
        'course_id=bc AND evaluation_revision=1';
    new_evaluation_revision CONSTANT text :=
        'course_id=bc AND evaluation_revision=2';
BEGIN
    SELECT pg_catalog.pg_get_functiondef(
        'public.ple_verify_base_course_completion_internal(uuid,uuid,text,jsonb)'::regprocedure
    ) INTO STRICT definition;
    IF pg_catalog.strpos(definition, old_clause) = 0
       OR pg_catalog.strpos(
            pg_catalog.replace(definition, old_clause, ''),
            old_clause
       ) <> 0
    THEN
        RAISE EXCEPTION 'Base Course completion response clause is unavailable or ambiguous';
    END IF;
    definition := pg_catalog.replace(definition, old_clause, new_clause);
    IF pg_catalog.strpos(definition, old_evaluation_revision) = 0
       OR pg_catalog.strpos(
            pg_catalog.replace(definition, old_evaluation_revision, ''),
            old_evaluation_revision
       ) <> 0
    THEN
        RAISE EXCEPTION 'Base Course completion evaluation revision is unavailable or ambiguous';
    END IF;
    definition := pg_catalog.replace(
        definition,
        old_evaluation_revision,
        new_evaluation_revision
    );
    EXECUTE definition;
END
$migration$;

DO $proof$
DECLARE
    exact_claim regprocedure :=
        'public.ple_claim_exact_worker_job_v1(uuid,uuid,integer,text)'::regprocedure;
    helper regprocedure :=
        'public.ple_verify_base_course_accepted_private_response_v1(uuid,uuid,uuid,uuid)'::regprocedure;
    verifier regprocedure :=
        'public.ple_verify_base_course_completion_internal(uuid,uuid,text,jsonb)'::regprocedure;
    column_name text;
BEGIN
    IF (
        SELECT owner.rolname <> 'ple_queue_broker'
            OR NOT procedure.prosecdef
            OR procedure.provolatile <> 'v'
            OR procedure.proconfig IS DISTINCT FROM
                ARRAY['search_path=pg_catalog, public, pg_temp']
          FROM pg_catalog.pg_proc AS procedure
          JOIN pg_catalog.pg_roles AS owner ON owner.oid = procedure.proowner
         WHERE procedure.oid = exact_claim
    ) OR NOT pg_catalog.has_function_privilege(
        'ple_app',
        exact_claim,
        'EXECUTE'
    ) OR pg_catalog.has_function_privilege(
        'public',
        exact_claim,
        'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'exact worker-job claim authority is unsafe';
    END IF;
    IF (
        SELECT owner.rolname <> 'ple_base_course_completion_verification_broker'
            OR NOT procedure.prosecdef
            OR procedure.provolatile <> 's'
            OR procedure.proconfig IS DISTINCT FROM
                ARRAY['search_path=pg_catalog, public, pg_temp']
          FROM pg_catalog.pg_proc AS procedure
          JOIN pg_catalog.pg_roles AS owner ON owner.oid = procedure.proowner
         WHERE procedure.oid = helper
    ) THEN
        RAISE EXCEPTION 'Base Course accepted-response helper catalog is unsafe';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.aclexplode(
                COALESCE(
                    (SELECT proacl FROM pg_catalog.pg_proc WHERE oid = helper),
                    pg_catalog.acldefault(
                        'f',
                        (SELECT proowner FROM pg_catalog.pg_proc WHERE oid = helper)
                    )
                )
          ) AS privilege
         WHERE privilege.grantee <>
            (SELECT proowner FROM pg_catalog.pg_proc WHERE oid = helper)
    ) THEN
        RAISE EXCEPTION 'Base Course accepted-response helper execution matrix is unsafe';
    END IF;
    FOREACH column_name IN ARRAY ARRAY[
        'tenant_id',
        'course_id',
        'attempt_id',
        'submission_id',
        'response_canonical_json',
        'response_sha256'
    ] LOOP
        IF NOT pg_catalog.has_column_privilege(
            'ple_base_course_completion_verification_broker',
            'public.accepted_submission_private_response',
            column_name,
            'SELECT'
        ) THEN
            RAISE EXCEPTION 'Base Course accepted-response column authority is incomplete';
        END IF;
    END LOOP;
    IF pg_catalog.has_table_privilege(
        'ple_base_course_completion_verification_broker',
        'public.accepted_submission_private_response',
        'SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER'
    ) THEN
        RAISE EXCEPTION 'Base Course accepted-response table authority is too broad';
    END IF;
    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_policy AS policy
          JOIN pg_catalog.pg_class AS relation ON relation.oid = policy.polrelid
          JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = relation.relnamespace
         WHERE namespace.nspname = 'public'
           AND relation.relname = 'accepted_submission_private_response'
           AND policy.polname = 'base_course_completion_accepted_response_select'
           AND policy.polcmd = 'r'
           AND policy.polpermissive
           AND policy.polroles =
                ARRAY['ple_base_course_completion_verification_broker'::regrole::oid]
           AND pg_catalog.pg_get_expr(policy.polqual, policy.polrelid) = 'true'
           AND policy.polwithcheck IS NULL
    ) THEN
        RAISE EXCEPTION 'Base Course accepted-response RLS policy is unsafe';
    END IF;
    IF pg_catalog.strpos(
        pg_catalog.pg_get_functiondef(verifier),
        'ple_verify_base_course_accepted_private_response_v1'
    ) = 0 OR pg_catalog.strpos(
        pg_catalog.pg_get_functiondef(verifier),
        'jsonb_build_array(''amide'')'
    ) <> 0 THEN
        RAISE EXCEPTION 'Base Course completion verifier retained the legacy response contract';
    END IF;
END
$proof$;

COMMIT;
