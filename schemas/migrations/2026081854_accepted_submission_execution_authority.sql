-- WP-PROF-G1 / G1-W4: accepted-submission execution authority.
--
-- 1851 owns roles and schema. 1852 owns immutable evidence guards. This
-- migration gives the sealed worker exactly the relations and columns needed
-- by the later claim/read/load/lock/commit/fail capabilities. Those callable
-- capabilities are installed by migrations 1855 through 1860.

BEGIN;

-- ASVS 8.2.1-8.2.3 and 8.3.1: reset direct data authority before granting the
-- exact definer capability. Role flags remain owned and attested by 1851.
REVOKE ALL ON SCHEMA public FROM ple_accepted_submission_execution_worker;
GRANT USAGE ON SCHEMA public TO ple_accepted_submission_execution_worker;

REVOKE ALL ON ALL TABLES IN SCHEMA public
    FROM ple_accepted_submission_execution_worker;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public
    FROM ple_accepted_submission_execution_worker;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA public
    FROM ple_accepted_submission_execution_worker;

-- The API fast path is a caller-only capability. It receives function EXECUTE
-- grants in later migrations and has no direct relation authority here.
REVOKE ALL ON SCHEMA public FROM ple_accepted_submission_execution_fast_path;
GRANT USAGE ON SCHEMA public TO ple_accepted_submission_execution_fast_path;

REVOKE ALL ON ALL TABLES IN SCHEMA public
    FROM ple_accepted_submission_execution_fast_path;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public
    FROM ple_accepted_submission_execution_fast_path;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA public
    FROM ple_accepted_submission_execution_fast_path;

-- The worker functions use one private relational witness. Its joins bind the
-- accepted input, issued grader evidence, execution generation, lease, run,
-- enrollment, assignment, retention state, and current scalar summary.
-- ASVS 2.3.1, 8.2.2, 8.4.1, and 15.4.2: every later state transition consumes
-- one coherent server-owned row rather than independently checked fragments.
CREATE VIEW public.ple_accepted_submission_execution_witness_v1
    WITH (security_invoker = true)
AS
SELECT
    execution.tenant_id,
    execution.attempt_id,
    execution.submission_id,
    execution.submission_occurred_at,
    execution.course_id,
    execution.execution_generation,
    execution.current_job_id,
    execution.state AS execution_state,
    execution.active_worker_id,
    job.state AS job_state,
    job.lease_token,
    job.lease_expires_at,
    job.attempt_count,
    job.max_attempts,
    job.available_at,
    evaluation.grading_status,
    evaluation.automated_result_canonical_json,
    evaluation.automated_result_sha256,
    attempt.run_id,
    attempt.assignment_position,
    attempt.occurred_at AS attempt_occurred_at,
    assignment.assignment_id,
    assignment.revision AS assignment_revision,
    enrollment.enrollment_id,
    COALESCE(retention.lifecycle, 'active') AS retention_lifecycle,
    accepted.course_id AS accepted_course_id,
    accepted.accepted_actor_id,
    accepted.idempotency_key AS accepted_idempotency_key,
    accepted.request_sha256,
    floor(extract(epoch FROM accepted.submitted_at) * 1000)::bigint
        AS accepted_millis,
    response.response_canonical_json,
    attempt.payload AS attempt_payload,
    attempt.payload_sha256 AS attempt_payload_sha256,
    attempt.presentation_descriptor_version,
    attempt.presentation_nonce,
    attempt.presentation_digest,
    attempt.presentation_capability,
    attempt.presentation_payload,
    attempt.presentation_payload_sha256,
    attempt.presentation_payload IS NOT NULL AS presentation_required,
    attempt.grading_envelope_payload,
    attempt.grading_envelope_payload_sha256,
    attempt.issued_question_snapshot_payload,
    attempt.issued_question_snapshot_payload_sha256,
    private.flat_required,
    private.flat_payload,
    private.flat_payload_sha256,
    private.webwork_required,
    private.webwork_payload,
    private.webwork_payload_sha256,
    private.webwork_replay_payload,
    private.webwork_replay_payload_sha256,
    private.qti_required,
    private.qti_payload,
    private.qti_payload_sha256,
    run.payload AS run_payload,
    run.payload_sha256 AS run_payload_sha256,
    run.completed_at AS run_completed_at,
    enrollment.first_completed_at,
    enrollment.current_grade_run_id,
    enrollment.best_grade_run_id,
    summary.current_score AS summary_current_score,
    summary.best_score AS summary_best_score,
    summary.latest_score AS summary_latest_score,
    summary.completed_run_count,
    summary.total_question_attempts,
    floor(extract(epoch FROM summary.last_activity_at) * 1000)::bigint
        AS summary_last_activity_at_millis,
    assignment.scoring_generation
FROM public.grading_execution AS execution
JOIN public.worker_job AS job
  ON (job.tenant_id, job.job_id) =
     (execution.tenant_id, execution.current_job_id)
JOIN public.submission_evaluation AS evaluation
  ON (evaluation.tenant_id, evaluation.attempt_id, evaluation.submission_id) =
     (execution.tenant_id, execution.attempt_id, execution.submission_id)
JOIN public.question_attempt AS attempt
  ON (attempt.tenant_id, attempt.attempt_id) =
     (execution.tenant_id, execution.attempt_id)
JOIN public.submission AS accepted_submission
  ON accepted_submission.tenant_id = execution.tenant_id
 AND accepted_submission.attempt_id = execution.attempt_id
 AND accepted_submission.submission_id = execution.submission_id
 AND accepted_submission.occurred_at = execution.submission_occurred_at
JOIN public.submission_idempotency AS accepted
  ON accepted.tenant_id = execution.tenant_id
 AND accepted.attempt_id = execution.attempt_id
 AND accepted.submission_id = execution.submission_id
 AND accepted.submission_occurred_at = execution.submission_occurred_at
JOIN public.accepted_submission_private_response AS response
  ON response.tenant_id = execution.tenant_id
 AND response.course_id = execution.course_id
 AND response.attempt_id = execution.attempt_id
 AND response.submission_id = execution.submission_id
 AND response.submission_occurred_at = execution.submission_occurred_at
JOIN public.issued_attempt_private_execution AS private
  ON private.tenant_id = attempt.tenant_id
 AND private.attempt_id = attempt.attempt_id
 AND private.attempt_occurred_at = attempt.occurred_at
JOIN public.assignment_run AS run
  ON (run.tenant_id, run.run_id) = (attempt.tenant_id, attempt.run_id)
JOIN public.enrollment AS enrollment
  ON (enrollment.tenant_id, enrollment.enrollment_id) =
     (run.tenant_id, run.enrollment_id)
JOIN public.assignment AS assignment
  ON (assignment.tenant_id, assignment.assignment_id) =
     (enrollment.tenant_id, enrollment.assignment_id)
JOIN public.student_assignment_summary AS summary
  ON (summary.tenant_id, summary.enrollment_id) =
     (enrollment.tenant_id, enrollment.enrollment_id)
LEFT JOIN public.course_retention AS retention
  ON (retention.tenant_id, retention.course_id) =
     (assignment.tenant_id, assignment.course_id)
WHERE execution.course_id = assignment.course_id
  AND attempt.course_id = assignment.course_id
  AND accepted.request_contract_version = 2
  AND accepted.accepted_actor_id IS NOT NULL
  AND accepted.course_id = assignment.course_id
  AND accepted_submission.course_id = assignment.course_id
  AND accepted_submission.idempotency_key = accepted.idempotency_key
  AND accepted.request_sha256 = response.response_sha256
  AND response.response_sha256 = encode(
        pg_catalog.sha256(convert_to(response.response_canonical_json, 'UTF8')),
        'hex'
      )
  AND accepted_submission.payload_sha256 = encode(
        pg_catalog.sha256(
            convert_to(
                '{"kind":"acceptedPrivateResponseV1"}'::jsonb::text,
                'UTF8'
            )
        ),
        'hex'
      )
  AND accepted.payload_sha256 = encode(
        pg_catalog.sha256(
            convert_to(
                '{"kind":"acceptedPrivateResponseV1"}'::jsonb::text,
                'UTF8'
            )
        ),
        'hex'
      );

ALTER VIEW public.ple_accepted_submission_execution_witness_v1
    OWNER TO ple_accepted_submission_execution_worker;

REVOKE ALL ON public.ple_accepted_submission_execution_witness_v1 FROM PUBLIC;
REVOKE ALL ON public.ple_accepted_submission_execution_witness_v1 FROM ple_app;
REVOKE ALL ON public.ple_accepted_submission_execution_witness_v1
    FROM ple_accepted_submission_execution;
REVOKE ALL ON public.ple_accepted_submission_execution_witness_v1
    FROM ple_accepted_submission_execution_fast_path;

-- Forced RLS remains the row gate for the definer owner. Most worker policies
-- are deliberately true because the background worker claims across tenants;
-- exact identifiers and tenant witnesses are rechecked inside each later
-- SECURITY DEFINER capability. Receipt SELECT is the exception: the verified
-- app read has a transaction tenant and needs only that tenant's route witness.
CREATE POLICY accepted_execution_worker_job ON public.worker_job
    FOR ALL TO ple_accepted_submission_execution_worker
    USING (true) WITH CHECK (true);
CREATE POLICY accepted_execution_worker_execution ON public.grading_execution
    FOR ALL TO ple_accepted_submission_execution_worker
    USING (true) WITH CHECK (true);
CREATE POLICY accepted_execution_worker_evaluation ON public.submission_evaluation
    FOR ALL TO ple_accepted_submission_execution_worker
    USING (true) WITH CHECK (true);
CREATE POLICY accepted_execution_worker_receipt ON public.grading_execution_receipt
    FOR INSERT TO ple_accepted_submission_execution_worker
    WITH CHECK (true);
CREATE POLICY accepted_execution_worker_operation ON public.grading_operation
    FOR INSERT TO ple_accepted_submission_execution_worker
    WITH CHECK (true);
CREATE POLICY accepted_execution_worker_submission ON public.submission
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_idempotency ON public.submission_idempotency
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_attempt ON public.question_attempt
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_attempt_completion ON public.question_attempt
    FOR UPDATE TO ple_accepted_submission_execution_worker
    USING (true) WITH CHECK (true);
CREATE POLICY accepted_execution_worker_run ON public.assignment_run
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_run_completion ON public.assignment_run
    FOR UPDATE TO ple_accepted_submission_execution_worker
    USING (true) WITH CHECK (true);
CREATE POLICY accepted_execution_worker_enrollment ON public.enrollment
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_enrollment_completion ON public.enrollment
    FOR UPDATE TO ple_accepted_submission_execution_worker
    USING (true) WITH CHECK (true);
CREATE POLICY accepted_execution_worker_assignment ON public.assignment
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_audience ON public.assignment_audience_group
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_items ON public.assignment_item
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_run_items ON public.assignment_run_item
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_selection_groups ON public.assignment_selection_group
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_selection_candidates
    ON public.assignment_selection_candidate
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_retention ON public.course_retention
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_private_response
    ON public.accepted_submission_private_response
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_private_execution
    ON public.issued_attempt_private_execution
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);
CREATE POLICY accepted_execution_worker_feedback ON public.attempt_feedback
    FOR INSERT TO ple_accepted_submission_execution_worker
    WITH CHECK (true);
CREATE POLICY accepted_execution_worker_receipt_snapshot
    ON public.submission_receipt_snapshot
    FOR INSERT TO ple_accepted_submission_execution_worker
    WITH CHECK (true);
CREATE POLICY accepted_execution_worker_receipt_snapshot_select
    ON public.submission_receipt_snapshot
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY accepted_execution_worker_summary_completion
    ON public.student_assignment_summary
    FOR UPDATE TO ple_accepted_submission_execution_worker
    USING (true) WITH CHECK (true);
CREATE POLICY accepted_execution_worker_summary_select
    ON public.student_assignment_summary
    FOR SELECT TO ple_accepted_submission_execution_worker
    USING (true);

-- Exact relation authority for the later definer functions. UUIDs are supplied
-- by the server, so this capability requires no sequence privilege.
GRANT SELECT ON
    public.worker_job,
    public.grading_execution,
    public.submission_evaluation,
    public.submission,
    public.submission_idempotency,
    public.question_attempt,
    public.assignment_run,
    public.enrollment,
    public.assignment,
    public.assignment_audience_group,
    public.assignment_item,
    public.assignment_run_item,
    public.assignment_selection_group,
    public.assignment_selection_candidate,
    public.course_retention,
    public.accepted_submission_private_response,
    public.issued_attempt_private_execution,
    public.student_assignment_summary
TO ple_accepted_submission_execution_worker;

GRANT UPDATE (
    state,
    lease_token,
    lease_expires_at,
    attempt_count,
    last_error,
    completed_at,
    available_at
) ON public.worker_job TO ple_accepted_submission_execution_worker;

GRANT UPDATE (
    state,
    active_worker_id,
    retry_count,
    updated_at
) ON public.grading_execution TO ple_accepted_submission_execution_worker;

GRANT UPDATE (
    grading_status,
    credit_fraction,
    correct,
    payload,
    payload_sha256,
    automated_result_canonical_json,
    automated_result_sha256,
    automated_result_canonical_json_version,
    evaluated_at,
    evaluation_revision
) ON public.submission_evaluation TO ple_accepted_submission_execution_worker;

GRANT INSERT ON
    public.grading_execution_receipt,
    public.grading_operation,
    public.attempt_feedback,
    public.submission_receipt_snapshot
TO ple_accepted_submission_execution_worker;

GRANT UPDATE (attempt_status, submitted_at) ON public.question_attempt
    TO ple_accepted_submission_execution_worker;

GRANT UPDATE (completed_at, payload, payload_sha256) ON public.assignment_run
    TO ple_accepted_submission_execution_worker;

GRANT UPDATE (
    first_completed_at,
    current_grade_run_id,
    best_grade_run_id
) ON public.enrollment TO ple_accepted_submission_execution_worker;

GRANT SELECT (
    tenant_id,
    attempt_id,
    canonical_json_version
) ON public.submission_receipt_snapshot
    TO ple_accepted_submission_execution_worker;

GRANT UPDATE (
    current_score,
    best_score,
    latest_score,
    completed_run_count,
    total_question_attempts,
    last_activity_at,
    updated_at
) ON public.student_assignment_summary TO ple_accepted_submission_execution_worker;

-- These existing functions are the only direct callable dependencies needed by
-- the later definer functions. The public W4 wrappers remain separately owned
-- and granted by migrations 1855 through 1860.
GRANT EXECUTE ON FUNCTION public.ple_current_tenant()
    TO ple_accepted_submission_execution_worker;
GRANT EXECUTE ON FUNCTION public.ple_course_records_accessible(uuid, uuid)
    TO ple_accepted_submission_execution_worker;
GRANT EXECUTE ON FUNCTION public.ple_enqueue_assignment_recalculation(
    uuid,
    uuid,
    uuid,
    integer
) TO ple_accepted_submission_execution_worker;
GRANT EXECUTE ON FUNCTION public.ple_record_question_statistics(
    uuid,
    uuid,
    uuid,
    uuid,
    uuid,
    uuid,
    double precision,
    bigint,
    bigint,
    double precision,
    bytea
) TO ple_accepted_submission_execution_worker;

-- Keep ordinary grading compatible while withholding worker-owned canonical
-- evaluation fields from the broad application role. ASVS 8.2.3, 14.2.6,
-- and 15.3.1: application code receives only the established safe columns.
REVOKE ALL ON public.submission_evaluation FROM ple_app;

GRANT SELECT (
    tenant_id,
    attempt_id,
    submission_id,
    grading_status,
    credit_fraction,
    correct,
    payload,
    payload_sha256,
    evaluated_at,
    course_id,
    evaluation_revision
) ON public.submission_evaluation TO ple_app;

GRANT INSERT (
    tenant_id,
    attempt_id,
    submission_id,
    grading_status,
    credit_fraction,
    correct,
    payload,
    payload_sha256,
    evaluated_at,
    course_id,
    evaluation_revision
) ON public.submission_evaluation TO ple_app;

GRANT UPDATE (
    submission_id,
    grading_status,
    credit_fraction,
    correct,
    payload,
    payload_sha256,
    evaluated_at,
    evaluation_revision
) ON public.submission_evaluation TO ple_app;

-- Role and schema attestation.
DO $$
BEGIN
    IF 2 <> (
        SELECT count(*)
        FROM pg_catalog.pg_roles
        WHERE rolname IN (
                  'ple_accepted_submission_execution_worker',
                  'ple_accepted_submission_execution_fast_path'
              )
          AND NOT rolcanlogin
          AND NOT rolinherit
          AND NOT rolsuper
          AND NOT rolcreatedb
          AND NOT rolcreaterole
          AND NOT rolreplication
          AND NOT rolbypassrls
    ) OR EXISTS (
        SELECT 1
        FROM pg_catalog.pg_auth_members AS membership
        WHERE membership.roleid IN (
                  'ple_accepted_submission_execution_worker'::regrole,
                  'ple_accepted_submission_execution_fast_path'::regrole
              )
           OR membership.member IN (
                  'ple_accepted_submission_execution_worker'::regrole,
                  'ple_accepted_submission_execution_fast_path'::regrole
              )
    ) OR pg_catalog.pg_has_role(
        'ple_app',
        'ple_accepted_submission_execution_worker',
        'USAGE'
    ) OR pg_catalog.pg_has_role(
        'ple_app',
        'ple_accepted_submission_execution',
        'USAGE'
    ) OR pg_catalog.pg_has_role(
        'ple_app',
        'ple_accepted_submission_execution_fast_path',
        'USAGE'
    ) OR has_schema_privilege(
        'ple_accepted_submission_execution_worker',
        'public',
        'CREATE'
    ) OR NOT has_schema_privilege(
        'ple_accepted_submission_execution_worker',
        'public',
        'USAGE'
    ) OR has_schema_privilege(
        'ple_accepted_submission_execution_fast_path',
        'public',
        'CREATE'
    ) OR NOT has_schema_privilege(
        'ple_accepted_submission_execution_fast_path',
        'public',
        'USAGE'
    ) THEN
        RAISE EXCEPTION 'accepted-submission worker role authority is unsafe';
    END IF;
END;
$$;

-- Witness ownership and direct-denial attestation.
DO $$
DECLARE
    v_witness regclass :=
        'public.ple_accepted_submission_execution_witness_v1'::regclass;
BEGIN
    IF (
        SELECT relation.relowner
        FROM pg_catalog.pg_class AS relation
        WHERE relation.oid = v_witness
    ) <> 'ple_accepted_submission_execution_worker'::regrole
       OR NOT EXISTS (
            SELECT 1
            FROM pg_catalog.pg_class AS relation
            WHERE relation.oid = v_witness
              AND relation.relkind = 'v'
              AND relation.reloptions @> ARRAY['security_invoker=true']
       )
       OR has_table_privilege('public', v_witness, 'SELECT')
       OR has_table_privilege('ple_app', v_witness, 'SELECT')
       OR has_table_privilege(
            'ple_accepted_submission_execution',
            v_witness,
            'SELECT'
       )
       OR has_table_privilege(
            'ple_accepted_submission_execution_fast_path',
            v_witness,
            'SELECT'
       )
    THEN
        RAISE EXCEPTION 'accepted-submission execution witness is exposed';
    END IF;
END;
$$;

-- Exact direct table and column ACL attestation. This compares owned grants as
-- a set, so a later accidental privilege addition fails the next fresh apply.
DO $$
BEGIN
    IF EXISTS (
        WITH expected AS (
            SELECT
                'table'::text AS authority_kind,
                relation_name::regclass::oid AS object_id,
                NULL::name AS column_name,
                privilege_type,
                false AS is_grantable
            FROM (
                VALUES
                    ('public.worker_job', 'SELECT'),
                    ('public.grading_execution', 'SELECT'),
                    ('public.submission_evaluation', 'SELECT'),
                    ('public.grading_execution_receipt', 'INSERT'),
                    ('public.grading_operation', 'INSERT'),
                    ('public.submission', 'SELECT'),
                    ('public.submission_idempotency', 'SELECT'),
                    ('public.question_attempt', 'SELECT'),
                    ('public.assignment_run', 'SELECT'),
                    ('public.enrollment', 'SELECT'),
                    ('public.assignment', 'SELECT'),
                    ('public.assignment_audience_group', 'SELECT'),
                    ('public.assignment_item', 'SELECT'),
                    ('public.assignment_run_item', 'SELECT'),
                    ('public.assignment_selection_group', 'SELECT'),
                    ('public.assignment_selection_candidate', 'SELECT'),
                    ('public.course_retention', 'SELECT'),
                    ('public.accepted_submission_private_response', 'SELECT'),
                    ('public.issued_attempt_private_execution', 'SELECT'),
                    ('public.attempt_feedback', 'INSERT'),
                    ('public.submission_receipt_snapshot', 'INSERT'),
                    ('public.student_assignment_summary', 'SELECT')
            ) AS table_grant(relation_name, privilege_type)
            UNION ALL
            SELECT
                'column',
                relation_name::regclass::oid,
                column_name::name,
                privilege_type,
                false
            FROM (
                VALUES
                    ('public.worker_job', 'state', 'UPDATE'),
                    ('public.worker_job', 'lease_token', 'UPDATE'),
                    ('public.worker_job', 'lease_expires_at', 'UPDATE'),
                    ('public.worker_job', 'attempt_count', 'UPDATE'),
                    ('public.worker_job', 'last_error', 'UPDATE'),
                    ('public.worker_job', 'completed_at', 'UPDATE'),
                    ('public.worker_job', 'available_at', 'UPDATE'),
                    ('public.grading_execution', 'state', 'UPDATE'),
                    ('public.grading_execution', 'active_worker_id', 'UPDATE'),
                    ('public.grading_execution', 'retry_count', 'UPDATE'),
                    ('public.grading_execution', 'updated_at', 'UPDATE'),
                    ('public.submission_evaluation', 'grading_status', 'UPDATE'),
                    ('public.submission_evaluation', 'credit_fraction', 'UPDATE'),
                    ('public.submission_evaluation', 'correct', 'UPDATE'),
                    ('public.submission_evaluation', 'payload', 'UPDATE'),
                    ('public.submission_evaluation', 'payload_sha256', 'UPDATE'),
                    (
                        'public.submission_evaluation',
                        'automated_result_canonical_json',
                        'UPDATE'
                    ),
                    (
                        'public.submission_evaluation',
                        'automated_result_sha256',
                        'UPDATE'
                    ),
                    (
                        'public.submission_evaluation',
                        'automated_result_canonical_json_version',
                        'UPDATE'
                    ),
                    ('public.submission_evaluation', 'evaluated_at', 'UPDATE'),
                    ('public.submission_evaluation', 'evaluation_revision', 'UPDATE'),
                    ('public.question_attempt', 'attempt_status', 'UPDATE'),
                    ('public.question_attempt', 'submitted_at', 'UPDATE'),
                    ('public.assignment_run', 'completed_at', 'UPDATE'),
                    ('public.assignment_run', 'payload', 'UPDATE'),
                    ('public.assignment_run', 'payload_sha256', 'UPDATE'),
                    ('public.enrollment', 'first_completed_at', 'UPDATE'),
                    ('public.enrollment', 'current_grade_run_id', 'UPDATE'),
                    ('public.enrollment', 'best_grade_run_id', 'UPDATE'),
                    ('public.student_assignment_summary', 'current_score', 'UPDATE'),
                    ('public.student_assignment_summary', 'best_score', 'UPDATE'),
                    ('public.student_assignment_summary', 'latest_score', 'UPDATE'),
                    (
                        'public.student_assignment_summary',
                        'completed_run_count',
                        'UPDATE'
                    ),
                    (
                        'public.student_assignment_summary',
                        'total_question_attempts',
                        'UPDATE'
                    ),
                    ('public.student_assignment_summary', 'last_activity_at', 'UPDATE'),
                    ('public.student_assignment_summary', 'updated_at', 'UPDATE'),
                    ('public.submission_receipt_snapshot', 'tenant_id', 'SELECT'),
                    ('public.submission_receipt_snapshot', 'attempt_id', 'SELECT'),
                    (
                        'public.submission_receipt_snapshot',
                        'canonical_json_version',
                        'SELECT'
                    )
            ) AS column_grant(relation_name, column_name, privilege_type)
            UNION ALL
            SELECT
                'function',
                function_name::regprocedure::oid,
                NULL::name,
                'EXECUTE',
                false
            FROM unnest(ARRAY[
                'public.ple_current_tenant()',
                'public.ple_course_records_accessible(uuid,uuid)',
                'public.ple_enqueue_assignment_recalculation(uuid,uuid,uuid,integer)',
                'public.ple_record_question_statistics('
                    || 'uuid,uuid,uuid,uuid,uuid,uuid,double precision,'
                    || 'bigint,bigint,double precision,bytea)'
            ]) AS function_grant(function_name)
        ),
        actual AS (
            SELECT
                'table'::text AS authority_kind,
                relation.oid AS object_id,
                NULL::name AS column_name,
                acl.privilege_type,
                acl.is_grantable
            FROM pg_catalog.pg_class AS relation
            JOIN pg_catalog.pg_namespace AS namespace
              ON namespace.oid = relation.relnamespace
            CROSS JOIN LATERAL pg_catalog.aclexplode(relation.relacl) AS acl
            WHERE namespace.nspname = 'public'
              AND relation.relkind IN ('r', 'p', 'v', 'm', 'f')
              AND relation.relowner <>
                    'ple_accepted_submission_execution_worker'::regrole
              AND acl.grantee =
                    'ple_accepted_submission_execution_worker'::regrole
            UNION ALL
            SELECT
                'column',
                attribute.attrelid,
                attribute.attname,
                acl.privilege_type,
                acl.is_grantable
            FROM pg_catalog.pg_attribute AS attribute
            CROSS JOIN LATERAL pg_catalog.aclexplode(attribute.attacl) AS acl
            WHERE attribute.attnum > 0
              AND NOT attribute.attisdropped
              AND acl.grantee =
                    'ple_accepted_submission_execution_worker'::regrole
            UNION ALL
            SELECT
                'function',
                procedure.oid,
                NULL::name,
                acl.privilege_type,
                acl.is_grantable
            FROM pg_catalog.pg_proc AS procedure
            JOIN pg_catalog.pg_namespace AS namespace
              ON namespace.oid = procedure.pronamespace
            CROSS JOIN LATERAL pg_catalog.aclexplode(procedure.proacl) AS acl
            WHERE namespace.nspname = 'public'
              AND procedure.proowner <>
                    'ple_accepted_submission_execution_worker'::regrole
              AND acl.grantee =
                    'ple_accepted_submission_execution_worker'::regrole
        )
        SELECT 1
        FROM (
            (SELECT * FROM expected EXCEPT SELECT * FROM actual)
            UNION ALL
            (SELECT * FROM actual EXCEPT SELECT * FROM expected)
        ) AS privilege_difference
    ) OR EXISTS (
        SELECT 1
        FROM pg_catalog.pg_class AS sequence
        JOIN pg_catalog.pg_namespace AS namespace
          ON namespace.oid = sequence.relnamespace
        WHERE namespace.nspname = 'public'
          AND sequence.relkind = 'S'
          AND (
              sequence.relowner =
                  'ple_accepted_submission_execution_worker'::regrole
              OR EXISTS (
                  SELECT 1
                  FROM pg_catalog.aclexplode(
                      COALESCE(sequence.relacl, '{}'::aclitem[])
                  ) AS privilege
                  WHERE privilege.grantee IN (
                            0::oid,
                            'ple_accepted_submission_execution_worker'::regrole::oid
                        )
                    AND privilege.privilege_type IN ('USAGE', 'SELECT', 'UPDATE')
              )
          )
    ) THEN
        RAISE EXCEPTION 'accepted-submission worker ACL matrix is unsafe';
    END IF;
END;
$$;

-- Exact worker RLS policy and forced-RLS attestation.
DO $$
BEGIN
    IF EXISTS (
        WITH expected(relation_name, policy_name, command, using_code, check_code) AS (
            VALUES
                ('worker_job', 'accepted_execution_worker_job', '*', 'true', 'true'),
                ('grading_execution', 'accepted_execution_worker_execution', '*', 'true', 'true'),
                ('submission_evaluation', 'accepted_execution_worker_evaluation',
                    '*', 'true', 'true'),
                ('grading_execution_receipt', 'accepted_execution_worker_receipt',
                    'a', NULL, 'true'),
                ('grading_operation', 'accepted_execution_worker_operation', 'a', NULL, 'true'),
                ('submission', 'accepted_execution_worker_submission', 'r', 'true', NULL),
                ('submission_idempotency', 'accepted_execution_worker_idempotency',
                    'r', 'true', NULL),
                ('question_attempt', 'accepted_execution_worker_attempt', 'r', 'true', NULL),
                ('question_attempt', 'accepted_execution_worker_attempt_completion',
                    'w', 'true', 'true'),
                ('assignment_run', 'accepted_execution_worker_run', 'r', 'true', NULL),
                ('assignment_run', 'accepted_execution_worker_run_completion', 'w', 'true', 'true'),
                ('enrollment', 'accepted_execution_worker_enrollment', 'r', 'true', NULL),
                ('enrollment', 'accepted_execution_worker_enrollment_completion',
                    'w', 'true', 'true'),
                ('assignment', 'accepted_execution_worker_assignment', 'r', 'true', NULL),
                ('assignment_audience_group', 'accepted_execution_worker_audience',
                    'r', 'true', NULL),
                ('assignment_item', 'accepted_execution_worker_items', 'r', 'true', NULL),
                ('assignment_run_item', 'accepted_execution_worker_run_items', 'r', 'true', NULL),
                ('assignment_selection_group', 'accepted_execution_worker_selection_groups',
                    'r', 'true', NULL),
                ('assignment_selection_candidate',
                    'accepted_execution_worker_selection_candidates', 'r', 'true', NULL),
                ('course_retention', 'accepted_execution_worker_retention', 'r', 'true', NULL),
                ('accepted_submission_private_response',
                    'accepted_execution_worker_private_response', 'r', 'true', NULL),
                ('issued_attempt_private_execution',
                    'accepted_execution_worker_private_execution', 'r', 'true', NULL),
                ('attempt_feedback', 'accepted_execution_worker_feedback', 'a', NULL, 'true'),
                ('submission_receipt_snapshot',
                    'accepted_execution_worker_receipt_snapshot', 'a', NULL, 'true'),
                (
                    'submission_receipt_snapshot',
                    'accepted_execution_worker_receipt_snapshot_select',
                    'r',
                    'tenant',
                    NULL
                ),
                (
                    'student_assignment_summary',
                    'accepted_execution_worker_summary_completion',
                    'w',
                    'true',
                    'true'
                ),
                (
                    'student_assignment_summary',
                    'accepted_execution_worker_summary_select',
                    'r',
                    'true',
                    NULL
                )
        ),
        actual AS (
            SELECT
                relation.relname::text AS relation_name,
                policy.polname::text AS policy_name,
                policy.polcmd::text AS command,
                CASE pg_catalog.pg_get_expr(policy.polqual, policy.polrelid)
                    WHEN '(tenant_id = ple_current_tenant())' THEN 'tenant'
                    ELSE pg_catalog.pg_get_expr(policy.polqual, policy.polrelid)
                END AS using_code,
                pg_catalog.pg_get_expr(policy.polwithcheck, policy.polrelid) AS check_code
            FROM pg_catalog.pg_policy AS policy
            JOIN pg_catalog.pg_class AS relation
              ON relation.oid = policy.polrelid
            JOIN pg_catalog.pg_namespace AS namespace
              ON namespace.oid = relation.relnamespace
            WHERE namespace.nspname = 'public'
              AND cardinality(policy.polroles) = 1
              AND policy.polpermissive
              AND 'ple_accepted_submission_execution_worker'::regrole::oid =
                    ANY (policy.polroles)
        )
        SELECT 1
        FROM (
            (SELECT * FROM expected EXCEPT SELECT * FROM actual)
            UNION ALL
            (SELECT * FROM actual EXCEPT SELECT * FROM expected)
        ) AS policy_difference
        UNION ALL
        SELECT 1
        FROM (SELECT DISTINCT relation_name FROM expected) AS relation_expected
        WHERE NOT EXISTS (
            SELECT 1
            FROM pg_catalog.pg_class AS relation
            JOIN pg_catalog.pg_namespace AS namespace
              ON namespace.oid = relation.relnamespace
            WHERE namespace.nspname = 'public'
              AND relation.relname = relation_expected.relation_name
              AND relation.relrowsecurity
              AND relation.relforcerowsecurity
        )
    ) THEN
        RAISE EXCEPTION 'accepted-submission worker RLS matrix is unsafe';
    END IF;
END;
$$;

-- Application and caller denial attestation. The app keeps the established
-- ordinary-evaluation fields but cannot access worker canonical columns. The
-- execution callers have function-only authority and cannot read the witness
-- or private accepted response directly. ASVS 8.2.1-8.2.3 and 14.2.6.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM unnest(ARRAY[
            'ple_app',
            'ple_accepted_submission_execution_fast_path'
        ]) AS denied_role(role_name)
        CROSS JOIN unnest(ARRAY[
            'automated_result_canonical_json',
            'automated_result_sha256',
            'automated_result_canonical_json_version'
        ]) AS denied(column_name)
        CROSS JOIN unnest(ARRAY['SELECT', 'INSERT', 'UPDATE']) AS action(privilege_type)
        WHERE has_column_privilege(
            denied_role.role_name,
            'public.submission_evaluation',
            denied.column_name,
            action.privilege_type
        )
    ) OR EXISTS (
        SELECT 1
        FROM unnest(ARRAY[
            'public.accepted_submission_private_response'::regclass,
            'public.ple_accepted_submission_execution_witness_v1'::regclass
        ]) AS denied(relation_id)
        CROSS JOIN unnest(ARRAY[
            'ple_app',
            'ple_accepted_submission_execution',
            'ple_accepted_submission_execution_fast_path'
        ]) AS actor(role_name)
        WHERE has_table_privilege(actor.role_name, denied.relation_id, 'SELECT')
           OR has_any_column_privilege(actor.role_name, denied.relation_id, 'SELECT')
    ) OR EXISTS (
        SELECT 1
        FROM pg_catalog.pg_class AS relation
        JOIN pg_catalog.pg_namespace AS namespace
          ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'public'
          AND relation.relkind IN ('r', 'p', 'v', 'm', 'f')
          AND (
              has_table_privilege(
                'ple_accepted_submission_execution_fast_path',
                relation.oid,
                'SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER'
              )
              OR has_any_column_privilege(
                'ple_accepted_submission_execution_fast_path',
                relation.oid,
                'SELECT,INSERT,UPDATE,REFERENCES'
              )
          )
    ) OR EXISTS (
        SELECT 1
        FROM pg_catalog.pg_class AS sequence
        JOIN pg_catalog.pg_namespace AS namespace
          ON namespace.oid = sequence.relnamespace
        WHERE namespace.nspname = 'public'
          AND sequence.relkind = 'S'
          AND (
              sequence.relowner =
                  'ple_accepted_submission_execution_fast_path'::regrole
              OR EXISTS (
                  SELECT 1
                  FROM pg_catalog.aclexplode(
                      COALESCE(sequence.relacl, '{}'::aclitem[])
                  ) AS privilege
                  WHERE privilege.grantee IN (
                            0::oid,
                            'ple_accepted_submission_execution_fast_path'::regrole::oid
                        )
                    AND privilege.privilege_type IN ('USAGE', 'SELECT', 'UPDATE')
              )
          )
    ) OR NOT has_table_privilege(
        'ple_retention_broker',
        'public.submission_evaluation',
        'DELETE'
    ) OR NOT has_table_privilege(
        'ple_retention_broker',
        'public.submission_receipt_snapshot',
        'DELETE'
    ) THEN
        RAISE EXCEPTION 'retention authority was not preserved';
    END IF;
END;
$$;

COMMIT;
