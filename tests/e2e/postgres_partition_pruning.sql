-- Disposable PostgreSQL acceptance for the two database scale boundaries:
-- monthly pruning of append-only activity and bounded gradebook reads from
-- current summaries. The enclosing runner owns and removes this database.

\set ON_ERROR_STOP on

-- Retention fences require the same server-derived tenant context used by the
-- application Store, even for fixture writes made by the migration principal.
SELECT set_config(
    'ple.tenant_id',
    'f1000000-0000-4000-8000-000000000002',
    false
);

-- One complete activity identity is enough to satisfy the immutable foreign
-- keys while generate_series supplies realistic partition cardinality.
INSERT INTO public.problem (
    problem_id,
    owner_tenant_id,
    owner_user_id,
    visibility,
    license
) VALUES (
    'f1000000-0000-4000-8000-000000000001',
    'f1000000-0000-4000-8000-000000000002',
    'f1000000-0000-4000-8000-000000000003',
    'public',
    'CC-BY-4.0'
);

INSERT INTO public.problem_version (
    problem_id,
    version_id,
    version_number,
    content_sha256,
    workspace_id,
    title,
    authors
) VALUES (
    'f1000000-0000-4000-8000-000000000001',
    'f1000000-0000-4000-8000-000000000004',
    1,
    repeat('1', 64),
    'f1000000-0000-4000-8000-000000000005',
    'Partition pruning fixture',
    '[{"name":"Partition gate"}]'::jsonb
);

INSERT INTO public.course (tenant_id, course_id, title) VALUES (
    'f1000000-0000-4000-8000-000000000002',
    'f1000000-0000-4000-8000-000000000006',
    'Partition pruning course'
);

INSERT INTO public.assignment (tenant_id, assignment_id, course_id, title) VALUES (
    'f1000000-0000-4000-8000-000000000002',
    'f1000000-0000-4000-8000-000000000007',
    'f1000000-0000-4000-8000-000000000006',
    'Partition pruning assignment'
);

INSERT INTO public.enrollment (
    tenant_id,
    enrollment_id,
    assignment_id,
    student_id,
    user_id,
    payload,
    payload_sha256
) VALUES (
    'f1000000-0000-4000-8000-000000000002',
    'f1000000-0000-4000-8000-000000000008',
    'f1000000-0000-4000-8000-000000000007',
    'f1000000-0000-4000-8000-000000000009',
    'f1000000-0000-4000-8000-000000000009',
    '{}'::jsonb,
    repeat('2', 64)
);

INSERT INTO public.assignment_run (
    tenant_id,
    run_id,
    enrollment_id,
    run_number,
    started_at,
    completed_at,
    payload,
    payload_sha256
) VALUES (
    'f1000000-0000-4000-8000-000000000002',
    'f1000000-0000-4000-8000-00000000000a',
    'f1000000-0000-4000-8000-000000000008',
    1,
    '2026-08-01 00:00:00+00',
    '2028-10-01 00:00:00+00',
    '{}'::jsonb,
    repeat('3', 64)
);

-- Ten thousand attempts in each of 26 months gives the optimizer meaningful
-- per-child statistics without turning this gate into a load test.
INSERT INTO public.question_attempt (
    tenant_id,
    attempt_id,
    run_id,
    problem_id,
    version_id,
    occurred_at,
    payload,
    payload_sha256,
    attempt_status,
    submitted_at,
    assignment_position,
    course_id,
    presentation_capability,
    flat_grading_required,
    webwork_grading_required,
    issued_feedback_disclosure
)
SELECT
    'f1000000-0000-4000-8000-000000000002'::uuid,
    md5('partition-attempt-' || generated)::uuid,
    'f1000000-0000-4000-8000-00000000000a'::uuid,
    'f1000000-0000-4000-8000-000000000001'::uuid,
    'f1000000-0000-4000-8000-000000000004'::uuid,
    occurred_at,
    '{}'::jsonb,
    repeat('4', 64),
    'submitted',
    occurred_at + interval '1 minute',
    0,
    'f1000000-0000-4000-8000-000000000006'::uuid,
    'not_applicable',
    false,
    false,
    'deferred'
FROM generate_series(1, 260000) AS series(generated)
CROSS JOIN LATERAL (
    SELECT date '2026-08-01'
           + ((generated - 1) / 10000) * interval '1 month'
           + (generated % 27) * interval '1 day'
           + (generated % 86400) * interval '1 second' AS occurred_at
) AS activity;

-- The other three monthly parents need enough rows to prove that their own
-- bounds prune; they do not need attempt-table scale in this acceptance gate.
INSERT INTO public.submission (
    tenant_id,
    submission_id,
    occurred_at,
    payload,
    payload_sha256,
    attempt_id,
    idempotency_key,
    course_id
)
SELECT
    'f1000000-0000-4000-8000-000000000002'::uuid,
    md5('partition-submission-' || generated)::uuid,
    occurred_at,
    '{}'::jsonb,
    repeat('5', 64),
    md5(
        'partition-attempt-'
        || (((generated - 1) / 200) * 10000 + ((generated - 1) % 200) + 1)
    )::uuid,
    'partition-' || generated,
    'f1000000-0000-4000-8000-000000000006'::uuid
FROM generate_series(1, 5200) AS series(generated)
CROSS JOIN LATERAL (
    SELECT date '2026-08-01'
           + ((generated - 1) / 200) * interval '1 month'
           + (generated % 27) * interval '1 day' AS occurred_at
) AS activity;

INSERT INTO public.record_access_log (
    tenant_id,
    access_log_id,
    occurred_at,
    payload,
    payload_sha256,
    delivery_scope,
    delivery_id
)
SELECT
    'f1000000-0000-4000-8000-000000000002'::uuid,
    md5('partition-access-' || generated)::uuid,
    occurred_at,
    '{}'::jsonb,
    repeat('6', 64),
    'catalog',
    md5('partition-delivery-' || generated)::uuid
FROM generate_series(1, 5200) AS series(generated)
CROSS JOIN LATERAL (
    SELECT date '2026-08-01'
           + ((generated - 1) / 200) * interval '1 month'
           + (generated % 27) * interval '1 day' AS occurred_at
) AS activity;

INSERT INTO public.audit_event (
    tenant_id,
    audit_event_id,
    occurred_at,
    actor_id,
    action,
    target_kind,
    target_id,
    payload,
    payload_sha256
)
SELECT
    'f1000000-0000-4000-8000-000000000002'::uuid,
    md5('partition-audit-' || generated)::uuid,
    occurred_at,
    'f1000000-0000-4000-8000-000000000003'::uuid,
    'partitionGate',
    'questionAttempt',
    md5('partition-target-' || generated)::uuid,
    '{}'::jsonb,
    repeat('7', 64)
FROM generate_series(1, 5200) AS series(generated)
CROSS JOIN LATERAL (
    SELECT date '2026-08-01'
           + ((generated - 1) / 200) * interval '1 month'
           + (generated % 27) * interval '1 day' AS occurred_at
) AS activity;

ANALYZE public.question_attempt;
ANALYZE public.submission;
ANALYZE public.record_access_log;
ANALYZE public.audit_event;

-- Inspect the actual JSON plan tree. Each time-bounded query must touch the
-- requested May child and no sibling or default partition.
DO $$
DECLARE
    parent_name text;
    expected_child text;
    expected_rows bigint;
    actual_rows bigint;
    plan_document jsonb;
    scanned_partitions text[];
BEGIN
    FOREACH parent_name IN ARRAY ARRAY[
        'question_attempt',
        'submission',
        'record_access_log',
        'audit_event'
    ] LOOP
        expected_child := parent_name || '_2027_05';
        expected_rows := CASE WHEN parent_name = 'question_attempt' THEN 10000 ELSE 200 END;

        EXECUTE format(
            'EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) '
            'SELECT count(*) FROM public.%I '
            'WHERE tenant_id = %L::uuid '
            'AND occurred_at >= timestamptz %L '
            'AND occurred_at < timestamptz %L',
            parent_name,
            'f1000000-0000-4000-8000-000000000002',
            '2027-05-01 00:00:00+00',
            '2027-06-01 00:00:00+00'
        ) INTO plan_document;

        WITH RECURSIVE plan_nodes(node) AS (
            SELECT plan_document -> 0 -> 'Plan'
            UNION ALL
            SELECT child.node
              FROM plan_nodes
              CROSS JOIN LATERAL jsonb_array_elements(
                  COALESCE(plan_nodes.node -> 'Plans', '[]'::jsonb)
              ) AS child(node)
        )
        SELECT array_agg(DISTINCT node ->> 'Relation Name' ORDER BY node ->> 'Relation Name')
          INTO scanned_partitions
          FROM plan_nodes
         WHERE node ->> 'Relation Name' LIKE parent_name || '\_%' ESCAPE '\';

        IF scanned_partitions IS DISTINCT FROM ARRAY[expected_child] THEN
            RAISE EXCEPTION '% did not prune to exactly %; scanned %',
                parent_name, expected_child, scanned_partitions;
        END IF;

        EXECUTE format(
            'SELECT count(*) FROM public.%I '
            'WHERE tenant_id = %L::uuid '
            'AND occurred_at >= timestamptz %L '
            'AND occurred_at < timestamptz %L',
            parent_name,
            'f1000000-0000-4000-8000-000000000002',
            '2027-05-01 00:00:00+00',
            '2027-06-01 00:00:00+00'
        ) INTO actual_rows;
        IF actual_rows <> expected_rows THEN
            RAISE EXCEPTION '% expected % May rows, found %',
                parent_name, expected_rows, actual_rows;
        END IF;

        RAISE NOTICE 'PARTITION_PRUNING_PASS parent=% child=% rows=%',
            parent_name, expected_child, actual_rows;
    END LOOP;
END $$;

-- A separate course supplies 120 assignments and 500 learners each. This is
-- large enough for the normal planner to distinguish a bounded summary page
-- from rebuilding grade state out of append-only activity.
INSERT INTO public.course (tenant_id, course_id, title) VALUES (
    'f1000000-0000-4000-8000-000000000002',
    'f1000000-0000-4000-8000-00000000000b',
    'Gradebook plan course'
);

INSERT INTO public.assignment (tenant_id, assignment_id, course_id, title)
SELECT
    'f1000000-0000-4000-8000-000000000002'::uuid,
    md5('gradebook-assignment-' || assignment_number)::uuid,
    'f1000000-0000-4000-8000-00000000000b'::uuid,
    'Gradebook assignment ' || assignment_number
FROM generate_series(1, 120) AS assignments(assignment_number);

INSERT INTO public.enrollment (
    tenant_id,
    enrollment_id,
    assignment_id,
    student_id,
    user_id,
    payload,
    payload_sha256
)
SELECT
    'f1000000-0000-4000-8000-000000000002'::uuid,
    md5('gradebook-enrollment-' || assignment_number || '-' || student_number)::uuid,
    md5('gradebook-assignment-' || assignment_number)::uuid,
    md5('gradebook-student-' || student_number)::uuid,
    md5('gradebook-student-' || student_number)::uuid,
    '{}'::jsonb,
    repeat('8', 64)
FROM generate_series(1, 120) AS assignments(assignment_number)
CROSS JOIN generate_series(1, 500) AS students(student_number);

INSERT INTO public.student_assignment_summary (
    tenant_id,
    enrollment_id,
    payload,
    payload_sha256
)
SELECT
    tenant_id,
    enrollment_id,
    '{"status":"current"}'::jsonb,
    repeat('9', 64)
FROM public.enrollment
WHERE tenant_id = 'f1000000-0000-4000-8000-000000000002'::uuid
  AND assignment_id <> 'f1000000-0000-4000-8000-000000000007'::uuid;

ANALYZE public.assignment;
ANALYZE public.enrollment;
ANALYZE public.student_assignment_summary;

BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config(
    'ple.tenant_id',
    'f1000000-0000-4000-8000-000000000002',
    true
);

DO $$
DECLARE
    plan_document jsonb;
    scanned_relations text[];
    page_rows bigint;
    enrollment_uses_index boolean;
    summary_uses_index boolean;
BEGIN
    -- Keep this statement aligned with GRADEBOOK_SUMMARY_PAGE_SQL in the
    -- PostgreSQL Store. It is the complete first-page production read shape.
    EXECUTE $plan$
        EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)
        SELECT
            enrollment.enrollment_id,
            enrollment.student_id,
            assignment.assignment_id,
            assignment.title AS assignment_title,
            summary.payload,
            summary.payload_sha256
          FROM public.assignment AS assignment
          JOIN public.enrollment AS enrollment
            ON enrollment.tenant_id = assignment.tenant_id
           AND enrollment.assignment_id = assignment.assignment_id
          JOIN public.student_assignment_summary AS summary
            ON summary.tenant_id = enrollment.tenant_id
           AND summary.enrollment_id = enrollment.enrollment_id
         WHERE assignment.tenant_id = 'f1000000-0000-4000-8000-000000000002'::uuid
           AND assignment.course_id = 'f1000000-0000-4000-8000-00000000000b'::uuid
           AND public.ple_course_records_accessible(
               assignment.tenant_id,
               assignment.course_id
           )
         ORDER BY assignment.assignment_id, enrollment.enrollment_id
         LIMIT 51
    $plan$ INTO plan_document;

    WITH RECURSIVE plan_nodes(node) AS (
        SELECT plan_document -> 0 -> 'Plan'
        UNION ALL
        SELECT child.node
          FROM plan_nodes
          CROSS JOIN LATERAL jsonb_array_elements(
              COALESCE(plan_nodes.node -> 'Plans', '[]'::jsonb)
          ) AS child(node)
    )
    SELECT
        array_agg(DISTINCT node ->> 'Relation Name' ORDER BY node ->> 'Relation Name')
            FILTER (WHERE node ? 'Relation Name'),
        bool_or(
            node ->> 'Relation Name' = 'enrollment'
            AND node ->> 'Node Type' LIKE 'Index%'
        ),
        bool_or(
            node ->> 'Relation Name' = 'student_assignment_summary'
            AND node ->> 'Node Type' LIKE 'Index%'
        )
      INTO scanned_relations, enrollment_uses_index, summary_uses_index
      FROM plan_nodes;

    page_rows := (plan_document #>> '{0,Plan,Actual Rows}')::numeric::bigint;
    IF page_rows <> 51 THEN
        RAISE EXCEPTION 'gradebook plan returned %, expected one 51-row lookahead page', page_rows;
    END IF;
    IF scanned_relations IS DISTINCT FROM ARRAY[
        'assignment',
        'enrollment',
        'student_assignment_summary'
    ] THEN
        RAISE EXCEPTION 'gradebook plan touched unexpected relations: %', scanned_relations;
    END IF;
    IF NOT COALESCE(enrollment_uses_index, false)
       OR NOT COALESCE(summary_uses_index, false)
    THEN
        RAISE EXCEPTION 'gradebook plan missed a bounded enrollment/summary index: %',
            plan_document;
    END IF;

    RAISE NOTICE 'GRADEBOOK_CURRENT_SUMMARY_PASS rows=% relations=%',
        page_rows, scanned_relations;
END $$;
ROLLBACK;
