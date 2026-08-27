-- WP-PROF-G1 / G1-W4: exact accepted-submission completion lock.
-- Lock one exact accepted-submission completion source for the pure Rust planner.
--
-- This capability returns private completion material only while every member of
-- the accepted worker claim is current. The caller keeps the surrounding
-- transaction open through the completion commit.

BEGIN;

CREATE FUNCTION public.ple_lock_accepted_submission_completion_v1(
    p_tenant_id uuid,
    p_worker_job_id uuid,
    p_lease_token uuid,
    p_submission_id uuid,
    p_execution_generation bigint,
    p_worker_id uuid
)
RETURNS TABLE (
    tenant_id uuid,
    worker_job_id uuid,
    worker_lease_token uuid,
    submission_id uuid,
    execution_generation bigint,
    worker_id uuid,
    attempt_id uuid,
    assignment_id uuid,
    assignment_header jsonb,
    assignment_audience_groups jsonb,
    assignment_items jsonb,
    assignment_selection_groups jsonb,
    assignment_selection_candidates jsonb,
    enrollment_id uuid,
    enrollment_user_id uuid,
    enrollment_student_id uuid,
    run_id uuid,
    assignment_scoring_generation bigint,
    accepted_at_millis bigint,
    attempt_payload jsonb,
    attempt_payload_sha256 character(64),
    presentation_payload jsonb,
    presentation_payload_sha256 character(64),
    presentation_required boolean,
    run_payload jsonb,
    run_payload_sha256 character(64),
    run_completed_at_millis bigint,
    enrollment_first_completed_at_millis bigint,
    enrollment_current_grade_run_id uuid,
    enrollment_best_grade_run_id uuid,
    summary_tenant_id uuid,
    summary_enrollment_id uuid,
    summary_current_score double precision,
    summary_best_score double precision,
    summary_latest_score double precision,
    summary_completed_run_count bigint,
    summary_total_question_attempts bigint,
    summary_last_activity_at_millis bigint,
    same_run_attempts jsonb,
    run_items jsonb
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
DECLARE
    v_attempt_id uuid;
    v_enrollment_id uuid;
    v_run_id uuid;
BEGIN
    -- ASVS 2.2.1, 8.2.2, and 8.4.1: validate the complete tenant-bound claim.
    IF p_tenant_id IS NULL
       OR p_worker_job_id IS NULL
       OR p_lease_token IS NULL
       OR p_submission_id IS NULL
       OR p_execution_generation IS NULL
       OR p_execution_generation <= 0
       OR p_worker_id IS NULL
       OR p_tenant_id IS DISTINCT FROM public.ple_current_tenant() THEN
        RETURN;
    END IF;

    -- ASVS 2.3.3 and 15.4.2: authorize and lock the exact live tuple in one
    -- statement before exposing any completion material.
    SELECT
        execution.attempt_id,
        run.enrollment_id,
        attempt.run_id
      INTO
        v_attempt_id,
        v_enrollment_id,
        v_run_id
      FROM public.grading_execution AS execution
      JOIN public.worker_job AS job
        ON (job.tenant_id, job.job_id) =
           (execution.tenant_id, execution.current_job_id)
      JOIN public.submission_evaluation AS evaluation
        ON (
            evaluation.tenant_id,
            evaluation.attempt_id,
            evaluation.submission_id
        ) = (
            execution.tenant_id,
            execution.attempt_id,
            execution.submission_id
        )
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
      JOIN public.issued_attempt_private_execution AS private_execution
        ON private_execution.tenant_id = attempt.tenant_id
       AND private_execution.attempt_id = attempt.attempt_id
       AND private_execution.attempt_occurred_at = attempt.occurred_at
      JOIN public.assignment_run AS run
        ON (run.tenant_id, run.run_id) =
           (attempt.tenant_id, attempt.run_id)
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
           (execution.tenant_id, execution.course_id)
     WHERE execution.tenant_id = p_tenant_id
       AND execution.current_job_id = p_worker_job_id
       AND execution.submission_id = p_submission_id
       AND execution.execution_generation = p_execution_generation
       AND execution.state = 'running'
       AND execution.active_worker_id = p_worker_id
       AND job.state = 'leased'
       AND job.lease_token = p_lease_token
       AND job.lease_expires_at > transaction_timestamp()
       AND job.payload = jsonb_build_object(
            'kind',
            'gradeAcceptedSubmission',
            'attempt',
            execution.attempt_id::text,
            'submission',
            execution.submission_id::text,
            'execution_generation',
            execution.execution_generation
       )
       AND evaluation.grading_status = 'automated_pending'
       AND evaluation.automated_result_canonical_json IS NULL
       AND evaluation.automated_result_sha256 IS NULL
       AND COALESCE(retention.lifecycle, 'active') = 'active'
       AND execution.course_id = assignment.course_id
       AND attempt.course_id = assignment.course_id
       AND accepted.request_contract_version = 2
       AND accepted.accepted_actor_id IS NOT NULL
       AND accepted.course_id = assignment.course_id
       AND accepted_submission.course_id = assignment.course_id
       AND accepted_submission.idempotency_key = accepted.idempotency_key
       AND accepted.request_sha256 = response.response_sha256
       AND response.response_sha256 = encode(
            pg_catalog.sha256(
                convert_to(response.response_canonical_json, 'UTF8')
            ),
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
       )
     FOR UPDATE OF execution, job, evaluation, attempt, run, enrollment, summary;

    IF NOT FOUND THEN
        RETURN;
    END IF;

    -- ASVS 15.3.1: return only the fields consumed by the pure completion
    -- planner and the exact claim fields the Rust adapter re-verifies.
    RETURN QUERY
    SELECT
        execution.tenant_id,
        job.job_id,
        job.lease_token,
        execution.submission_id,
        execution.execution_generation,
        p_worker_id,
        attempt.attempt_id,
        assignment.assignment_id,
        jsonb_build_object(
            'assignmentId',
            assignment.assignment_id,
            'courseId',
            assignment.course_id,
            'title',
            assignment.title,
            'lifecycle',
            assignment.lifecycle,
            'instructions',
            assignment.instructions,
            'completionPolicy',
            assignment.completion_policy,
            'completionThreshold',
            assignment.completion_threshold::text,
            'attemptSelectionPolicy',
            assignment.attempt_selection_policy,
            'continuedPracticePolicy',
            assignment.continued_practice_policy,
            'practiceMaxAdditionalRuns',
            assignment.practice_max_additional_runs,
            'variationPolicy',
            assignment.variation_policy,
            'audienceKind',
            assignment.audience_kind,
            'scoreDisclosure',
            assignment.score_disclosure,
            'perItemCorrectnessDisclosure',
            assignment.per_item_correctness_disclosure,
            'feedbackTextDisclosure',
            assignment.feedback_text_disclosure,
            'solutionDisclosure',
            assignment.solution_disclosure,
            'classStatisticsDisclosure',
            assignment.class_statistics_disclosure
        ),
        (
            SELECT COALESCE(
                jsonb_agg(
                    audience.course_group_id
                    ORDER BY audience.course_group_id
                ),
                '[]'::jsonb
            )
              FROM public.assignment_audience_group AS audience
             WHERE audience.tenant_id = execution.tenant_id
               AND audience.assignment_id = assignment.assignment_id
        ),
        (
            SELECT COALESCE(
                jsonb_agg(
                    jsonb_build_object(
                        'assignmentItemId',
                        item.assignment_item_id,
                        'position',
                        item.position,
                        'problemId',
                        item.problem_id,
                        'versionId',
                        item.version_id,
                        'pointsPossible',
                        item.points_possible::text,
                        'deliveryState',
                        item.delivery_state,
                        'scoringMode',
                        item.scoring_mode
                    )
                    ORDER BY item.position
                ),
                '[]'::jsonb
            )
              FROM public.assignment_item AS item
             WHERE item.tenant_id = execution.tenant_id
               AND item.assignment_id = assignment.assignment_id
        ),
        (
            SELECT COALESCE(
                jsonb_agg(
                    jsonb_build_object(
                        'selectionGroupId',
                        selection_group.selection_group_id,
                        'position',
                        selection_group.position,
                        'drawCount',
                        selection_group.draw_count,
                        'pointsPerItem',
                        selection_group.points_per_item::text,
                        'orderingPolicy',
                        selection_group.ordering_policy,
                        'algorithmVersion',
                        selection_group.algorithm_version
                    )
                    ORDER BY selection_group.position
                ),
                '[]'::jsonb
            )
              FROM public.assignment_selection_group AS selection_group
             WHERE selection_group.tenant_id = execution.tenant_id
               AND selection_group.assignment_id = assignment.assignment_id
        ),
        (
            SELECT COALESCE(
                jsonb_agg(
                    jsonb_build_object(
                        'selectionGroupId',
                        candidate.selection_group_id,
                        'candidateId',
                        candidate.candidate_id,
                        'position',
                        candidate.position,
                        'problemId',
                        candidate.problem_id,
                        'versionId',
                        candidate.version_id,
                        'deliveryState',
                        candidate.delivery_state
                    )
                    ORDER BY candidate.selection_group_id, candidate.position
                ),
                '[]'::jsonb
            )
              FROM public.assignment_selection_candidate AS candidate
             WHERE candidate.tenant_id = execution.tenant_id
               AND candidate.assignment_id = assignment.assignment_id
        ),
        enrollment.enrollment_id,
        enrollment.user_id,
        enrollment.student_id,
        run.run_id,
        assignment.scoring_generation,
        floor(extract(epoch FROM accepted.submitted_at) * 1000)::bigint,
        attempt.payload,
        attempt.payload_sha256,
        attempt.presentation_payload,
        attempt.presentation_payload_sha256,
        attempt.presentation_payload IS NOT NULL,
        run.payload,
        run.payload_sha256,
        floor(extract(epoch FROM run.completed_at) * 1000)::bigint,
        floor(extract(epoch FROM enrollment.first_completed_at) * 1000)::bigint,
        enrollment.current_grade_run_id,
        enrollment.best_grade_run_id,
        summary.tenant_id,
        summary.enrollment_id,
        summary.current_score,
        summary.best_score,
        summary.latest_score,
        summary.completed_run_count,
        summary.total_question_attempts,
        floor(extract(epoch FROM summary.last_activity_at) * 1000)::bigint,
        (
            SELECT COALESCE(
                jsonb_agg(
                    jsonb_build_object(
                        'attemptId',
                        peer.attempt_id,
                        'payload',
                        peer.payload,
                        'payloadSha256',
                        peer.payload_sha256,
                        'status',
                        peer.attempt_status,
                        'submittedAtMillis',
                        floor(
                            extract(epoch FROM peer.submitted_at) * 1000
                        )::bigint,
                        'evaluation',
                        peer_evaluation.payload,
                        'evaluationSha256',
                        peer_evaluation.payload_sha256,
                        'evaluationStatus',
                        peer_evaluation.grading_status
                    )
                    ORDER BY
                        peer.assignment_position,
                        peer.occurred_at,
                        peer.attempt_id
                ),
                '[]'::jsonb
            )
              FROM public.question_attempt AS peer
              LEFT JOIN public.submission_evaluation AS peer_evaluation
                ON (peer_evaluation.tenant_id, peer_evaluation.attempt_id) =
                   (peer.tenant_id, peer.attempt_id)
             WHERE peer.tenant_id = execution.tenant_id
               AND peer.run_id = run.run_id
        ),
        (
            SELECT COALESCE(
                jsonb_agg(
                    jsonb_build_object(
                        'run',
                        item.run_id,
                        'assignmentItem',
                        item.assignment_item_id,
                        'sourcePosition',
                        item.source_position,
                        'issuedPosition',
                        item.issued_position,
                        'reference',
                        jsonb_build_object(
                            'problem',
                            item.problem_id,
                            'version',
                            item.version_id
                        ),
                        'statisticsEligible',
                        item.statistics_eligible,
                        'selectionGroup',
                        item.selection_group_id,
                        'selectionSeed',
                        item.selection_seed
                    )
                    ORDER BY item.issued_position
                ),
                '[]'::jsonb
            )
              FROM public.assignment_run_item AS item
             WHERE item.tenant_id = execution.tenant_id
               AND item.run_id = run.run_id
        )
      FROM public.grading_execution AS execution
      JOIN public.worker_job AS job
        ON (job.tenant_id, job.job_id) =
           (execution.tenant_id, execution.current_job_id)
      JOIN public.question_attempt AS attempt
        ON (attempt.tenant_id, attempt.attempt_id) =
           (execution.tenant_id, execution.attempt_id)
      JOIN public.assignment_run AS run
        ON (run.tenant_id, run.run_id) =
           (attempt.tenant_id, attempt.run_id)
      JOIN public.enrollment AS enrollment
        ON (enrollment.tenant_id, enrollment.enrollment_id) =
           (run.tenant_id, run.enrollment_id)
      JOIN public.assignment AS assignment
        ON (assignment.tenant_id, assignment.assignment_id) =
           (enrollment.tenant_id, enrollment.assignment_id)
      JOIN public.student_assignment_summary AS summary
        ON (summary.tenant_id, summary.enrollment_id) =
           (enrollment.tenant_id, enrollment.enrollment_id)
      JOIN public.submission_idempotency AS accepted
        ON accepted.tenant_id = execution.tenant_id
       AND accepted.attempt_id = execution.attempt_id
       AND accepted.submission_id = execution.submission_id
       AND accepted.submission_occurred_at = execution.submission_occurred_at
     WHERE execution.tenant_id = p_tenant_id
       AND execution.attempt_id = v_attempt_id
       AND enrollment.enrollment_id = v_enrollment_id
       AND run.run_id = v_run_id;
END
$$;

ALTER FUNCTION public.ple_lock_accepted_submission_completion_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid
) OWNER TO ple_accepted_submission_execution_worker;

-- Start the callable boundary from an empty explicit non-owner ACL. This also
-- removes any deployment-local default privilege from the new function.
REVOKE ALL ON FUNCTION public.ple_lock_accepted_submission_completion_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid
) FROM PUBLIC;

DO $$
DECLARE
    v_function regprocedure :=
        (
            'public.ple_lock_accepted_submission_completion_v1('
            || 'uuid,uuid,uuid,uuid,bigint,uuid)'
        )::regprocedure;
    v_grantee oid;
BEGIN
    FOR v_grantee IN
        SELECT DISTINCT privilege.grantee
          FROM pg_catalog.pg_proc AS procedure_row
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(
                  procedure_row.proacl,
                  pg_catalog.acldefault('f', procedure_row.proowner)
              )
          ) AS privilege
         WHERE procedure_row.oid = v_function
           AND privilege.grantee <> 0
           AND privilege.grantee <> procedure_row.proowner
    LOOP
        EXECUTE pg_catalog.format(
            'REVOKE ALL PRIVILEGES ON FUNCTION %s FROM %I',
            v_function,
            pg_catalog.pg_get_userbyid(v_grantee)
        );
    END LOOP;
END
$$;

GRANT EXECUTE ON FUNCTION public.ple_lock_accepted_submission_completion_v1(
    uuid,
    uuid,
    uuid,
    uuid,
    bigint,
    uuid
) TO ple_accepted_submission_execution,
    ple_accepted_submission_execution_fast_path;

DO $$
DECLARE
    v_function regprocedure :=
        (
            'public.ple_lock_accepted_submission_completion_v1('
            || 'uuid,uuid,uuid,uuid,bigint,uuid)'
        )::regprocedure;
BEGIN
    -- ASVS 8.2.1 and 8.3.1: attest the sealed function boundary in catalog
    -- state so a privilege or definer drift fails the migration.
    IF NOT EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS procedure_row
         WHERE procedure_row.oid = v_function
           AND procedure_row.proowner =
               'ple_accepted_submission_execution_worker'::regrole
           AND procedure_row.prosecdef
           AND procedure_row.proconfig IS NOT DISTINCT FROM
               ARRAY['search_path=pg_catalog, public, pg_temp']
    )
    THEN
        RAISE EXCEPTION
            'accepted-submission completion-lock catalog is unsafe';
    END IF;

    IF EXISTS (
        WITH expected(grantee, privilege_type, is_grantable) AS (
            VALUES
                (
                    'ple_accepted_submission_execution'::regrole::oid,
                    'EXECUTE'::text,
                    false
                ),
                (
                    'ple_accepted_submission_execution_fast_path'::regrole::oid,
                    'EXECUTE'::text,
                    false
                )
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
        RAISE EXCEPTION
            'accepted-submission completion-lock execute ACL is unsafe';
    END IF;
END
$$;

COMMIT;
