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
    TO ple_app, ple_student, ple_queue_broker, ple_retention_broker;

-- Migration 022 deliberately revoked these tables from ordinary principals,
-- including the NOLOGIN queue broker. The pre-existing export commit SDF is
-- owned by that BYPASSRLS broker and still needs only its original closed
-- SELECT/UPDATE/INSERT set. Restore exactly that set here; no login or browser
-- principal receives it, and the SDF continues to require the active tenant,
-- job, lease token, manifest, and exact four-artifact bundle.
GRANT SELECT, UPDATE ON worker_job, student_export_request, student_export_artifact
    TO ple_queue_broker;
GRANT INSERT ON asset_delivery TO ple_queue_broker;

-- Protected student-record deliveries need a relational course owner. This
-- code-first schema has not been deployed; fail closed instead of rewriting
-- checksummed legacy payloads without their original canonical bytes.
ALTER TABLE asset_delivery ADD COLUMN course_id uuid;
DO $$ BEGIN
    IF EXISTS (
        SELECT 1 FROM asset_delivery
         WHERE delivery_kind = 'student_record'
    ) THEN
        RAISE EXCEPTION 'student-record deliveries require an explicit migration';
    END IF;
END $$;
ALTER TABLE asset_delivery
    ADD CONSTRAINT asset_delivery_course_fk
    FOREIGN KEY (tenant_id, course_id) REFERENCES course(tenant_id, course_id),
    ADD CONSTRAINT asset_delivery_course_shape CHECK (
        (delivery_kind = 'catalog' AND course_id IS NULL)
        OR (delivery_kind = 'student_record' AND course_id IS NOT NULL)
    );
CREATE INDEX asset_delivery_course_idx
    ON asset_delivery (tenant_id, course_id, delivery_id)
    WHERE delivery_kind = 'student_record';

-- The earlier closed export broker does not name course_id in its INSERT.
-- Fill that column from the relational export request, while checking that the
-- checksummed typed payload names the identical course. Direct app inserts
-- already provide course_id and still pass through the same payload check.
CREATE FUNCTION ple_bind_student_record_course() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE bound_course uuid;
BEGIN
    IF NEW.delivery_kind <> 'student_record' THEN
        RETURN NEW;
    END IF;
    IF NEW.course_id IS NULL THEN
        SELECT r.course_id INTO bound_course
          FROM public.student_export_artifact a
          JOIN public.student_export_request r ON r.export_id = a.export_id
         WHERE a.object_id = NEW.object_id
           AND r.tenant_id = NEW.tenant_id;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'student-record delivery has no course owner'
                USING ERRCODE = '23503';
        END IF;
        NEW.course_id = bound_course;
    END IF;
    -- The export broker intentionally has BYPASSRLS for cross-tenant queue
    -- claims. Re-check the lifecycle predicate explicitly inside this trigger
    -- so that capability cannot recreate a protected delivery after archive
    -- preparation fenced the course.
    IF NOT public.ple_course_records_accessible(NEW.tenant_id, NEW.course_id) THEN
        RAISE EXCEPTION 'student-record delivery course is unavailable'
            USING ERRCODE = '23503';
    END IF;
    IF NEW.payload #>> '{scope,course}' IS DISTINCT FROM NEW.course_id::text THEN
        RAISE EXCEPTION 'student-record delivery course mismatch'
            USING ERRCODE = '22023';
    END IF;
    RETURN NEW;
END $$;
ALTER FUNCTION ple_bind_student_record_course() OWNER TO ple_queue_broker;
REVOKE ALL ON FUNCTION ple_bind_student_record_course() FROM PUBLIC;
CREATE TRIGGER asset_delivery_bind_student_course
    BEFORE INSERT ON asset_delivery
    FOR EACH ROW EXECUTE FUNCTION ple_bind_student_record_course();

-- Every relational learner alias with a course path uses the same predicate.
-- Existing tenant policies are replaced in-place; no table grants are added.
DROP POLICY IF EXISTS assignment_tenant ON assignment;
CREATE POLICY assignment_app_tenant ON assignment
    FOR ALL TO ple_app
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY assignment_student_records ON assignment
    FOR SELECT TO ple_student
    USING (tenant_id = ple_current_tenant()
           AND public.ple_course_records_accessible(tenant_id, course_id));

DROP POLICY IF EXISTS assignment_problem_tenant ON assignment_problem;
CREATE POLICY assignment_problem_app_tenant ON assignment_problem
    FOR ALL TO ple_app
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY assignment_problem_student_records ON assignment_problem
    FOR SELECT TO ple_student
    USING (tenant_id = ple_current_tenant() AND EXISTS (
        SELECT 1 FROM public.assignment a
         WHERE a.tenant_id = assignment_problem.tenant_id
           AND a.assignment_id = assignment_problem.assignment_id
           AND public.ple_course_records_accessible(a.tenant_id, a.course_id)
    ));

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

DROP POLICY IF EXISTS asset_delivery_visible_select ON asset_delivery;
CREATE POLICY asset_delivery_visible_select ON asset_delivery
    FOR SELECT TO ple_app
    USING (
        (delivery_kind = 'catalog' AND EXISTS (
            SELECT 1 FROM problem_version AS visible_version
             WHERE visible_version.problem_id = asset_delivery.problem_id
               AND visible_version.version_id = asset_delivery.version_id
        ))
        OR (delivery_kind = 'student_record'
            AND tenant_id = ple_current_tenant()
            AND public.ple_course_records_accessible(tenant_id, course_id))
    );
DROP POLICY IF EXISTS asset_delivery_app_insert ON asset_delivery;
CREATE POLICY asset_delivery_app_insert ON asset_delivery
    FOR INSERT TO ple_app
    WITH CHECK (
        (delivery_kind = 'catalog' AND course_id IS NULL AND EXISTS (
            SELECT 1 FROM problem_version AS visible_version
             WHERE visible_version.problem_id = asset_delivery.problem_id
               AND visible_version.version_id = asset_delivery.version_id
        ))
        OR (delivery_kind = 'student_record'
            AND tenant_id = ple_current_tenant()
            AND public.ple_course_records_accessible(tenant_id, course_id))
    );
CREATE POLICY asset_delivery_export_broker_insert ON asset_delivery
    FOR INSERT TO ple_queue_broker
    WITH CHECK (delivery_kind = 'student_record'
                AND tenant_id = ple_current_tenant()
                AND public.ple_course_records_accessible(tenant_id, course_id));

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

-- R4.3 forward correction: migration 02400 locked only active rows before
-- consulting receipts.  Archive completion changes the lifecycle to archived,
-- so an exact post-completion retry could not reach its durable receipt.  Lock
-- the course regardless of lifecycle, then accept only a receipt with the
-- original actor/action/disposition binding.  This does not alter 02400's
-- migration checksum or its request/worker separation.
CREATE OR REPLACE FUNCTION ple_apply_retention_api_action(
    p_session char(64), p_course uuid, p_expected_generation bigint,
    p_action text, p_days integer DEFAULT NULL, p_disposition text DEFAULT NULL
) RETURNS text LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$
DECLARE current record; replay record; next_generation bigint; immediate_stage text; next_disposition text; target_state text; actor uuid;
BEGIN
    IF p_expected_generation <= 0 OR p_action NOT IN ('extend','archive','delete') THEN RETURN NULL; END IF;
    IF p_action='extend' THEN
        IF NOT public.ple_retention_authorize(p_session, p_course, true) THEN
            RAISE EXCEPTION 'retention API operation forbidden' USING ERRCODE = '42501';
        END IF;
        IF p_days NOT BETWEEN 1 AND 36500 OR p_disposition IS NOT NULL THEN RETURN NULL; END IF;
    ELSIF NOT public.ple_retention_authorize(p_session, p_course, false) THEN
        RAISE EXCEPTION 'retention API operation forbidden' USING ERRCODE = '42501';
    ELSIF p_action='archive' AND (p_days IS NOT NULL OR p_disposition NOT IN ('retain','delete')) THEN
        RETURN NULL;
    ELSIF p_action='delete' AND (p_days IS NOT NULL OR p_disposition IS NOT NULL) THEN
        RETURN NULL;
    END IF;
    SELECT user_id INTO actor FROM public.auth_session
     WHERE session_hash=p_session AND tenant_id=public.ple_current_tenant()
       AND revoked_at IS NULL AND expires_at > transaction_timestamp();
    IF actor IS NULL THEN RAISE EXCEPTION 'retention API operation forbidden' USING ERRCODE = '42501'; END IF;
    -- This lock still serializes concurrent requests, but it deliberately
    -- includes archived rows so completed archive receipts remain replayable.
    SELECT * INTO current FROM public.course_retention
     WHERE tenant_id=public.ple_current_tenant() AND course_id=p_course FOR UPDATE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    IF p_action IN ('archive','delete') THEN
        SELECT * INTO replay FROM public.course_retention_api_receipt
         WHERE tenant_id=public.ple_current_tenant() AND course_id=p_course
           AND (expected_generation=p_expected_generation
                OR resulting_generation=p_expected_generation)
         ORDER BY expected_generation DESC
         LIMIT 1;
        IF FOUND THEN
            IF replay.actor_id<>actor OR replay.action<>p_action
               OR replay.assignment_disposition IS DISTINCT FROM p_disposition THEN RETURN NULL; END IF;
            SELECT state INTO target_state FROM public.course_retention_stage
             WHERE tenant_id=replay.tenant_id AND course_id=replay.course_id
               AND generation=replay.resulting_generation AND stage=replay.stage;
            IF target_state='scheduled' THEN RETURN 'scheduled'; END IF;
            IF target_state='started' THEN RETURN 'inProgress'; END IF;
            IF target_state='completed' THEN RETURN 'completed'; END IF;
            RETURN NULL;
        END IF;
    END IF;
    IF current.lifecycle <> 'active' OR current.generation<>p_expected_generation THEN RETURN NULL; END IF;
    immediate_stage := CASE p_action WHEN 'archive' THEN 'archiveStudentRecords'
                                      WHEN 'delete' THEN 'deleteStudentRecords' END;
    IF immediate_stage IS NOT NULL THEN
        SELECT state INTO target_state FROM public.course_retention_stage s
         WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
           AND s.generation=current.generation AND s.stage=immediate_stage;
        IF target_state='started' THEN RETURN 'inProgress'; END IF;
        IF target_state='completed' THEN RETURN 'completed'; END IF;
        IF target_state <> 'scheduled' THEN RETURN NULL; END IF;
        IF EXISTS (
            SELECT 1 FROM public.course_retention_dispatch d
             WHERE d.tenant_id=current.tenant_id AND d.course_id=current.course_id
               AND d.generation=current.generation AND d.stage=immediate_stage
        ) THEN
            IF p_action='archive' AND current.assignment_disposition IS DISTINCT FROM p_disposition THEN
                RETURN NULL;
            END IF;
            RETURN 'scheduled';
        END IF;
    END IF;
    IF EXISTS (
        SELECT 1 FROM public.course_retention_stage s
         WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
           AND s.generation=current.generation AND s.state NOT IN ('scheduled','completed')
    ) THEN RETURN NULL; END IF;
    next_generation := current.generation + 1;
    next_disposition := CASE WHEN p_action='archive' THEN p_disposition ELSE current.assignment_disposition END;
    INSERT INTO public.course_retention_stage (tenant_id, course_id, stage, generation, due_at, state)
    SELECT s.tenant_id, s.course_id, s.stage, next_generation,
           CASE WHEN s.state='completed' THEN s.due_at
                WHEN s.stage=immediate_stage THEN transaction_timestamp()
                WHEN p_action='extend' THEN s.due_at + p_days * interval '1 day'
                ELSE s.due_at END,
           CASE WHEN s.state='completed' THEN 'completed' ELSE 'scheduled' END
     FROM public.course_retention_stage s
     WHERE s.tenant_id=current.tenant_id AND s.course_id=current.course_id
       AND s.generation=current.generation;
    UPDATE public.course_retention_stage SET state='superseded'
     WHERE tenant_id=current.tenant_id AND course_id=current.course_id
       AND generation=current.generation AND state='scheduled';
    UPDATE public.worker_job w SET state='dead', lease_token=NULL, lease_expires_at=NULL,
        completed_at=transaction_timestamp(), last_error='permanent'
      FROM public.course_retention_dispatch d
     WHERE d.tenant_id=current.tenant_id AND d.course_id=current.course_id
       AND d.generation=current.generation AND d.job_id=w.job_id AND w.state IN ('ready','leased');
    UPDATE public.course_retention SET generation=next_generation, assignment_disposition=next_disposition
     WHERE tenant_id=current.tenant_id AND course_id=current.course_id AND generation=current.generation;
    IF immediate_stage IS NOT NULL THEN
        WITH dispatch AS (
            INSERT INTO public.course_retention_dispatch
                (tenant_id, course_id, stage, generation, job_id, dispatched_at)
            VALUES (current.tenant_id, current.course_id, immediate_stage, next_generation,
                    gen_random_uuid(), transaction_timestamp())
            RETURNING job_id
        )
        INSERT INTO public.worker_job (job_id, tenant_id, payload, state, max_attempts)
        SELECT job_id, current.tenant_id,
               jsonb_build_object('kind','retention','course',current.course_id::text,
                                  'stage',immediate_stage,'generation',next_generation),
               'ready', 3 FROM dispatch;
        INSERT INTO public.course_retention_api_receipt
            (tenant_id, course_id, expected_generation, actor_id, action, assignment_disposition,
             resulting_generation, stage)
        VALUES (current.tenant_id, current.course_id, p_expected_generation, actor, p_action,
                p_disposition, next_generation, immediate_stage);
    END IF;
    RETURN CASE WHEN immediate_stage IS NULL THEN 'changed' ELSE 'scheduled' END;
END $$;
ALTER FUNCTION ple_apply_retention_api_action(char, uuid, bigint, text, integer, text)
    OWNER TO ple_retention_broker;
