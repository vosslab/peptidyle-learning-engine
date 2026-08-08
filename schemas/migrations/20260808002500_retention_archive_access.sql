-- MOD-RETENTION R4.3: one database-owned ordinary student-record boundary.
-- The predicate is intentionally lifecycle-opaque to callers. It first
-- rejects a tenant mismatch, then checks that the course exists, before it
-- consults any retention row; a foreign tenant therefore cannot probe course
-- or retention existence through this function.

CREATE FUNCTION ple_course_records_accessible(p_tenant uuid, p_course uuid)
RETURNS boolean
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE current_generation bigint;
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN
        RETURN false;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM public.course
        WHERE tenant_id = p_tenant AND course_id = p_course
    ) THEN
        RETURN false;
    END IF;
    SELECT generation INTO current_generation
      FROM public.course_retention
     WHERE tenant_id = p_tenant AND course_id = p_course;
    IF NOT FOUND THEN
        RETURN true;
    END IF;
    IF EXISTS (
        SELECT 1 FROM public.course_retention
         WHERE tenant_id = p_tenant AND course_id = p_course
           AND lifecycle IN ('archived', 'deleted')
    ) THEN
        RETURN false;
    END IF;
    RETURN NOT EXISTS (
        SELECT 1 FROM public.course_retention_stage
         WHERE tenant_id = p_tenant AND course_id = p_course
           AND generation = current_generation
           AND stage = 'archiveStudentRecords'
           AND state = 'started'
    );
END $$;
ALTER FUNCTION ple_course_records_accessible(uuid, uuid) OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION ple_course_records_accessible(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_course_records_accessible(uuid, uuid)
    TO ple_app, ple_student, ple_retention_broker;

-- Every relational learner alias with a course path uses the same predicate.
-- Existing tenant policies are replaced in-place; no table grants are added.
DROP POLICY IF EXISTS assignment_tenant ON assignment;
CREATE POLICY assignment_tenant ON assignment
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS assignment_problem_tenant ON assignment_problem;
CREATE POLICY assignment_problem_tenant ON assignment_problem
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS enrollment_tenant ON enrollment;
CREATE POLICY enrollment_tenant ON enrollment
    USING (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.assignment a
         WHERE a.tenant_id = enrollment.tenant_id
           AND a.assignment_id = enrollment.assignment_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ))
    WITH CHECK (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.assignment a
         WHERE a.tenant_id = enrollment.tenant_id
           AND a.assignment_id = enrollment.assignment_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ));

DROP POLICY IF EXISTS student_assignment_summary_tenant ON student_assignment_summary;
CREATE POLICY student_assignment_summary_tenant ON student_assignment_summary
    USING (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.enrollment e JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE e.tenant_id = student_assignment_summary.tenant_id
           AND e.enrollment_id = student_assignment_summary.enrollment_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ))
    WITH CHECK (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.enrollment e JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE e.tenant_id = student_assignment_summary.tenant_id
           AND e.enrollment_id = student_assignment_summary.enrollment_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ));

DROP POLICY IF EXISTS assignment_run_tenant ON assignment_run;
CREATE POLICY assignment_run_tenant ON assignment_run
    USING (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.enrollment e JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE e.tenant_id = assignment_run.tenant_id
           AND e.enrollment_id = assignment_run.enrollment_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ))
    WITH CHECK (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.enrollment e JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE e.tenant_id = assignment_run.tenant_id
           AND e.enrollment_id = assignment_run.enrollment_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ));

DROP POLICY IF EXISTS question_attempt_tenant ON question_attempt;
CREATE POLICY question_attempt_tenant ON question_attempt
    USING (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.assignment_run r JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE r.tenant_id = question_attempt.tenant_id
           AND r.run_id = question_attempt.run_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ))
    WITH CHECK (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.assignment_run r JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE r.tenant_id = question_attempt.tenant_id
           AND r.run_id = question_attempt.run_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ));

-- Submission and feedback rows carry attempt identity rather than course ID;
-- resolve the ownership chain rather than inventing a denormalized column.
DROP POLICY IF EXISTS submission_tenant ON submission;
CREATE POLICY submission_tenant ON submission
    USING (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.question_attempt q JOIN public.assignment_run r
          ON r.tenant_id = q.tenant_id AND r.run_id = q.run_id
          JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE q.tenant_id = submission.tenant_id
           AND q.attempt_id = submission.attempt_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ))
    WITH CHECK (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.question_attempt q JOIN public.assignment_run r
          ON r.tenant_id = q.tenant_id AND r.run_id = q.run_id
          JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE q.tenant_id = submission.tenant_id
           AND q.attempt_id = submission.attempt_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ));

DROP POLICY IF EXISTS submission_idempotency_tenant ON submission_idempotency;
CREATE POLICY submission_idempotency_tenant ON submission_idempotency
    USING (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.question_attempt q JOIN public.assignment_run r
          ON r.tenant_id = q.tenant_id AND r.run_id = q.run_id
          JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE q.tenant_id = submission_idempotency.tenant_id
           AND q.attempt_id = submission_idempotency.attempt_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ))
    WITH CHECK (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.question_attempt q JOIN public.assignment_run r
          ON r.tenant_id = q.tenant_id AND r.run_id = q.run_id
          JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE q.tenant_id = submission_idempotency.tenant_id
           AND q.attempt_id = submission_idempotency.attempt_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ));

DROP POLICY IF EXISTS attempt_feedback_tenant ON attempt_feedback;
CREATE POLICY attempt_feedback_tenant ON attempt_feedback
    USING (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.question_attempt q JOIN public.assignment_run r
          ON r.tenant_id = q.tenant_id AND r.run_id = q.run_id
          JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE q.tenant_id = attempt_feedback.tenant_id
           AND q.attempt_id = attempt_feedback.attempt_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ))
    WITH CHECK (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.question_attempt q JOIN public.assignment_run r
          ON r.tenant_id = q.tenant_id AND r.run_id = q.run_id
          JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE q.tenant_id = attempt_feedback.tenant_id
           AND q.attempt_id = attempt_feedback.attempt_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ));

DROP POLICY IF EXISTS feedback_release_tenant ON feedback_release;
CREATE POLICY feedback_release_tenant ON feedback_release
    USING (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.attempt_feedback f
         WHERE f.tenant_id = feedback_release.tenant_id
           AND f.attempt_id = feedback_release.attempt_id
    ))
    WITH CHECK (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.attempt_feedback f
         WHERE f.tenant_id = feedback_release.tenant_id
           AND f.attempt_id = feedback_release.attempt_id
    ));

DROP POLICY IF EXISTS question_prefetch_tenant ON question_prefetch;
CREATE POLICY question_prefetch_tenant ON question_prefetch
    USING (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.assignment_run r JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE r.tenant_id = question_prefetch.tenant_id
           AND r.run_id = question_prefetch.run_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ))
    WITH CHECK (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.assignment_run r JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE r.tenant_id = question_prefetch.tenant_id
           AND r.run_id = question_prefetch.run_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ));

DROP POLICY IF EXISTS submission_next_attempt_tenant ON submission_next_attempt;
CREATE POLICY submission_next_attempt_tenant ON submission_next_attempt
    USING (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.question_attempt q JOIN public.assignment_run r
          ON r.tenant_id = q.tenant_id AND r.run_id = q.run_id
          JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE q.tenant_id = submission_next_attempt.tenant_id
           AND q.attempt_id = submission_next_attempt.predecessor_attempt_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ))
    WITH CHECK (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.question_attempt q JOIN public.assignment_run r
          ON r.tenant_id = q.tenant_id AND r.run_id = q.run_id
          JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE q.tenant_id = submission_next_attempt.tenant_id
           AND q.attempt_id = submission_next_attempt.predecessor_attempt_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ));

DROP POLICY IF EXISTS external_tool_exchange_tenant ON external_tool_exchange;
CREATE POLICY external_tool_exchange_tenant ON external_tool_exchange
    USING (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.question_attempt q JOIN public.assignment_run r
          ON r.tenant_id = q.tenant_id AND r.run_id = q.run_id
          JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE q.tenant_id = external_tool_exchange.tenant_id
           AND q.attempt_id = external_tool_exchange.attempt_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ))
    WITH CHECK (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.question_attempt q JOIN public.assignment_run r
          ON r.tenant_id = q.tenant_id AND r.run_id = q.run_id
          JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE q.tenant_id = external_tool_exchange.tenant_id
           AND q.attempt_id = external_tool_exchange.attempt_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ));

DROP POLICY IF EXISTS external_tool_launch_session_tenant ON external_tool_launch_session;
CREATE POLICY external_tool_launch_session_tenant ON external_tool_launch_session
    USING (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.question_attempt q JOIN public.assignment_run r
          ON r.tenant_id = q.tenant_id AND r.run_id = q.run_id
          JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE q.tenant_id = external_tool_launch_session.tenant_id
           AND q.attempt_id = external_tool_launch_session.attempt_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ))
    WITH CHECK (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.question_attempt q JOIN public.assignment_run r
          ON r.tenant_id = q.tenant_id AND r.run_id = q.run_id
          JOIN public.enrollment e
          ON e.tenant_id = r.tenant_id AND e.enrollment_id = r.enrollment_id
          JOIN public.assignment a
          ON a.tenant_id = e.tenant_id AND a.assignment_id = e.assignment_id
         WHERE q.tenant_id = external_tool_launch_session.tenant_id
           AND q.attempt_id = external_tool_launch_session.attempt_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ));

DROP POLICY IF EXISTS student_export_request_tenant ON student_export_request;
CREATE POLICY student_export_request_tenant ON student_export_request
    USING (tenant_id = ple_current_tenant()
           AND public.ple_course_records_accessible(tenant_id, course_id))
    WITH CHECK (tenant_id = ple_current_tenant()
                AND public.ple_course_records_accessible(tenant_id, course_id));

DROP POLICY IF EXISTS student_export_artifact_tenant ON student_export_artifact;
CREATE POLICY student_export_artifact_tenant ON student_export_artifact
    USING (EXISTS (
        SELECT 1 FROM public.student_export_request r
         WHERE r.export_id = student_export_artifact.export_id
           AND public.ple_course_records_accessible(r.tenant_id, r.course_id)
    ));

-- asset_delivery has no course column. Its student_record accessor must call
-- ple_course_records_accessible after resolving request/artifact ownership;
-- this migration intentionally does not broaden raw delivery grants.

CREATE OR REPLACE FUNCTION ple_commit_retention_work(p_tenant uuid, p_job uuid, p_token uuid,
                                          p_course uuid, p_stage text, p_generation bigint)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE lifecycle text;
BEGIN
    IF p_tenant IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_generation <= 0
       OR p_stage NOT IN ('notify', 'archiveStudentRecords', 'deleteStudentRecords') THEN
        RETURN false;
    END IF;
    PERFORM 1 FROM public.worker_job w
      JOIN public.course_retention_dispatch d
        ON d.tenant_id=w.tenant_id AND d.course_id=p_course AND d.stage=p_stage
       AND d.generation=p_generation AND d.job_id=w.job_id
      JOIN public.course_retention r
        ON r.tenant_id=d.tenant_id AND r.course_id=d.course_id
       AND r.generation=d.generation
     WHERE w.job_id=p_job AND w.tenant_id=p_tenant AND w.state='leased'
       AND w.lease_token=p_token AND w.lease_expires_at > transaction_timestamp()
       AND w.payload=jsonb_build_object('kind','retention','course',p_course::text,
                                        'stage',p_stage,'generation',p_generation)
     FOR UPDATE OF w, r;
    IF NOT FOUND THEN RETURN false; END IF;
    PERFORM 1 FROM public.course_retention_stage s
     WHERE s.tenant_id=p_tenant AND s.course_id=p_course AND s.stage=p_stage
       AND s.generation=p_generation AND s.state='started'
       AND s.job_id=p_job AND s.lease_token=p_token
     FOR UPDATE;
    IF NOT FOUND THEN RETURN false; END IF;
    IF p_stage='notify' THEN
        INSERT INTO public.course_retention_notification (tenant_id, course_id, generation, intent)
        VALUES (p_tenant,p_course,p_generation,'archive') ON CONFLICT DO NOTHING;
    ELSIF p_stage='archiveStudentRecords' THEN
        SELECT lifecycle INTO lifecycle FROM public.course_retention
         WHERE tenant_id=p_tenant AND course_id=p_course AND generation=p_generation
         FOR UPDATE;
        IF lifecycle <> 'active' THEN RETURN false; END IF;
    ELSIF p_stage <> 'deleteStudentRecords' THEN
        RETURN false;
    END IF;
    UPDATE public.course_retention_stage SET state='completed'
     WHERE tenant_id=p_tenant AND course_id=p_course AND stage=p_stage
       AND generation=p_generation AND state='started' AND job_id=p_job AND lease_token=p_token;
    IF NOT FOUND THEN RETURN false; END IF;
    IF p_stage='archiveStudentRecords' THEN
        UPDATE public.course_retention SET lifecycle='archived'
         WHERE tenant_id=p_tenant AND course_id=p_course AND generation=p_generation
           AND lifecycle='active';
        IF NOT FOUND THEN RETURN false; END IF;
    END IF;
    UPDATE public.worker_job SET state='completed', lease_token=NULL, lease_expires_at=NULL,
        completed_at=transaction_timestamp()
     WHERE job_id=p_job AND tenant_id=p_tenant AND state='leased' AND lease_token=p_token
       AND payload=jsonb_build_object('kind','retention','course',p_course::text,
                                      'stage',p_stage,'generation',p_generation);
    IF NOT FOUND THEN RETURN false; END IF;
    RETURN true;
END $$;
ALTER FUNCTION ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;
