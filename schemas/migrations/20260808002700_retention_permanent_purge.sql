-- MOD-RETENTION R4.4B: relational ownership and permanent purge contracts.

-- The initial queue schema used two anonymous column checks. PostgreSQL named
-- the JSON-object check `worker_job_payload_check` and the closed-kind check
-- `worker_job_payload_check1`. Later forward migrations replaced only the
-- first name, unintentionally leaving the original three-kind constraint in
-- place, which rejects QTI and retention jobs. Give both current checks stable
-- names and restore the independent object-shape guard.
ALTER TABLE worker_job
    DROP CONSTRAINT worker_job_payload_check1;
ALTER TABLE worker_job
    RENAME CONSTRAINT worker_job_payload_check TO worker_job_payload_kind_check;
ALTER TABLE worker_job
    ADD CONSTRAINT worker_job_payload_object_check
        CHECK (jsonb_typeof(payload) = 'object');

-- 1) Add canonical course ownership columns to learner content rows and backfill
--    them from assignment / attempt relations only.
ALTER TABLE question_attempt
    ADD COLUMN course_id uuid;
ALTER TABLE submission
    ADD COLUMN course_id uuid;
ALTER TABLE grade_event
    ADD COLUMN course_id uuid;
ALTER TABLE submission_idempotency
    ADD COLUMN course_id uuid;
ALTER TABLE attempt_feedback
    ADD COLUMN course_id uuid;
ALTER TABLE external_tool_exchange
    ADD COLUMN course_id uuid;
ALTER TABLE external_tool_launch_session
    ADD COLUMN course_id uuid;

DROP TABLE IF EXISTS r44b_attempt_course;
CREATE TEMP TABLE r44b_attempt_course AS
SELECT
    q.tenant_id,
    q.attempt_id,
    (ARRAY_AGG(DISTINCT a.course_id ORDER BY a.course_id))[1] AS course_id,
    COUNT(DISTINCT a.course_id) AS course_count
  FROM public.question_attempt q
  JOIN public.assignment_run r
    ON r.tenant_id = q.tenant_id
  AND r.run_id = q.run_id
  JOIN public.enrollment e
    ON e.tenant_id = r.tenant_id
   AND e.enrollment_id = r.enrollment_id
  JOIN public.assignment a
    ON a.tenant_id = e.tenant_id
   AND a.assignment_id = e.assignment_id
 GROUP BY q.tenant_id, q.attempt_id;

CREATE INDEX ON r44b_attempt_course (tenant_id, attempt_id);

UPDATE public.question_attempt qa
   SET course_id = ac.course_id
  FROM r44b_attempt_course ac
   WHERE qa.tenant_id = ac.tenant_id
   AND qa.attempt_id = ac.attempt_id
   AND ac.course_count = 1;

UPDATE public.submission s
   SET course_id = ac.course_id
  FROM r44b_attempt_course ac
   WHERE s.tenant_id = ac.tenant_id
   AND s.attempt_id = ac.attempt_id
   AND ac.course_count = 1;

UPDATE public.grade_event ge
   SET course_id = ac.course_id
  FROM r44b_attempt_course ac
   WHERE ge.tenant_id = ac.tenant_id
   AND ge.attempt_id = ac.attempt_id
   AND ac.course_count = 1;

UPDATE public.submission_idempotency si
   SET course_id = ac.course_id
  FROM r44b_attempt_course ac
   WHERE si.tenant_id = ac.tenant_id
   AND si.attempt_id = ac.attempt_id
   AND ac.course_count = 1;

UPDATE public.attempt_feedback af
   SET course_id = ac.course_id
  FROM r44b_attempt_course ac
   WHERE af.tenant_id = ac.tenant_id
   AND af.attempt_id = ac.attempt_id
   AND ac.course_count = 1;

UPDATE public.external_tool_exchange et
   SET course_id = ac.course_id
  FROM r44b_attempt_course ac
   WHERE et.tenant_id = ac.tenant_id
   AND et.attempt_id = ac.attempt_id
   AND ac.course_count = 1;

UPDATE public.external_tool_launch_session ls
   SET course_id = ac.course_id
  FROM r44b_attempt_course ac
   WHERE ls.tenant_id = ac.tenant_id
   AND ls.attempt_id = ac.attempt_id
   AND ac.course_count = 1;

DO $$
DECLARE
    unresolved bigint;
BEGIN
    SELECT COUNT(*) INTO unresolved
      FROM public.question_attempt q
      LEFT JOIN r44b_attempt_course ac
        ON ac.tenant_id = q.tenant_id
       AND ac.attempt_id = q.attempt_id
     WHERE q.course_id IS DISTINCT FROM ac.course_id
        OR ac.attempt_id IS NULL
        OR ac.course_count <> 1;
    IF unresolved > 0 THEN
        RAISE EXCEPTION 'question_attempt ownership is unresolved or ambiguous';
    END IF;

    SELECT COUNT(*) INTO unresolved
      FROM public.submission s
      LEFT JOIN r44b_attempt_course ac
        ON ac.tenant_id = s.tenant_id
       AND ac.attempt_id = s.attempt_id
     WHERE s.course_id IS DISTINCT FROM ac.course_id
        OR ac.attempt_id IS NULL
        OR ac.course_count <> 1;
    IF unresolved > 0 THEN
        RAISE EXCEPTION 'submission ownership is unresolved or ambiguous';
    END IF;

    SELECT COUNT(*) INTO unresolved
      FROM public.grade_event ge
      LEFT JOIN r44b_attempt_course ac
        ON ac.tenant_id = ge.tenant_id
       AND ac.attempt_id = ge.attempt_id
     WHERE ge.course_id IS DISTINCT FROM ac.course_id
        OR ac.attempt_id IS NULL
        OR ac.course_count <> 1;
    IF unresolved > 0 THEN
        RAISE EXCEPTION 'grade_event ownership is unresolved or ambiguous';
    END IF;

    SELECT COUNT(*) INTO unresolved
      FROM public.submission_idempotency si
      LEFT JOIN r44b_attempt_course ac
        ON ac.tenant_id = si.tenant_id
       AND ac.attempt_id = si.attempt_id
     WHERE si.course_id IS DISTINCT FROM ac.course_id
        OR ac.attempt_id IS NULL
        OR ac.course_count <> 1;
    IF unresolved > 0 THEN
        RAISE EXCEPTION 'submission_idempotency ownership is unresolved or ambiguous';
    END IF;

    SELECT COUNT(*) INTO unresolved
      FROM public.attempt_feedback af
      LEFT JOIN r44b_attempt_course ac
        ON ac.tenant_id = af.tenant_id
       AND ac.attempt_id = af.attempt_id
     WHERE af.course_id IS DISTINCT FROM ac.course_id
        OR ac.attempt_id IS NULL
        OR ac.course_count <> 1;
    IF unresolved > 0 THEN
        RAISE EXCEPTION 'attempt_feedback ownership is unresolved or ambiguous';
    END IF;

    SELECT COUNT(*) INTO unresolved
      FROM public.external_tool_exchange et
      LEFT JOIN r44b_attempt_course ac
        ON ac.tenant_id = et.tenant_id
       AND ac.attempt_id = et.attempt_id
     WHERE et.course_id IS DISTINCT FROM ac.course_id
        OR ac.attempt_id IS NULL
        OR ac.course_count <> 1;
    IF unresolved > 0 THEN
        RAISE EXCEPTION 'external_tool_exchange ownership is unresolved or ambiguous';
    END IF;

    SELECT COUNT(*) INTO unresolved
      FROM public.external_tool_launch_session ls
      LEFT JOIN r44b_attempt_course ac
        ON ac.tenant_id = ls.tenant_id
       AND ac.attempt_id = ls.attempt_id
     WHERE ls.course_id IS DISTINCT FROM ac.course_id
        OR ac.attempt_id IS NULL
        OR ac.course_count <> 1;
    IF unresolved > 0 THEN
        RAISE EXCEPTION 'external_tool_launch_session ownership is unresolved or ambiguous';
    END IF;
END $$;

DROP TABLE IF EXISTS r44b_attempt_course;

ALTER TABLE question_attempt
    ALTER COLUMN course_id SET NOT NULL;
ALTER TABLE submission
    ALTER COLUMN course_id SET NOT NULL;
ALTER TABLE grade_event
    ALTER COLUMN course_id SET NOT NULL;
ALTER TABLE submission_idempotency
    ALTER COLUMN course_id SET NOT NULL;
ALTER TABLE attempt_feedback
    ALTER COLUMN course_id SET NOT NULL;
ALTER TABLE external_tool_exchange
    ALTER COLUMN course_id SET NOT NULL;
ALTER TABLE external_tool_launch_session
    ALTER COLUMN course_id SET NOT NULL;

ALTER TABLE question_attempt
    ADD CONSTRAINT question_attempt_course_fk
        FOREIGN KEY (tenant_id, course_id)
            REFERENCES course(tenant_id, course_id);
ALTER TABLE submission
    ADD CONSTRAINT submission_course_fk
        FOREIGN KEY (tenant_id, course_id)
            REFERENCES course(tenant_id, course_id);
ALTER TABLE grade_event
    ADD CONSTRAINT grade_event_course_fk
        FOREIGN KEY (tenant_id, course_id)
            REFERENCES course(tenant_id, course_id);
ALTER TABLE submission_idempotency
    ADD CONSTRAINT submission_idempotency_course_fk
        FOREIGN KEY (tenant_id, course_id)
            REFERENCES course(tenant_id, course_id);
ALTER TABLE attempt_feedback
    ADD CONSTRAINT attempt_feedback_course_fk
        FOREIGN KEY (tenant_id, course_id)
            REFERENCES course(tenant_id, course_id);
ALTER TABLE external_tool_exchange
    ADD CONSTRAINT external_tool_exchange_course_fk
        FOREIGN KEY (tenant_id, course_id)
            REFERENCES course(tenant_id, course_id);
ALTER TABLE external_tool_launch_session
    ADD CONSTRAINT external_tool_launch_session_course_fk
        FOREIGN KEY (tenant_id, course_id)
            REFERENCES course(tenant_id, course_id);

CREATE INDEX question_attempt_course_idx
    ON question_attempt (tenant_id, course_id, run_id, attempt_id, occurred_at);
CREATE INDEX submission_course_idx
    ON submission (tenant_id, course_id, attempt_id, occurred_at);
CREATE INDEX grade_event_course_idx
    ON grade_event (tenant_id, course_id, attempt_id, occurred_at);
CREATE INDEX submission_idempotency_course_idx
    ON submission_idempotency (tenant_id, course_id, attempt_id);
CREATE INDEX attempt_feedback_course_idx
    ON attempt_feedback (tenant_id, course_id, attempt_id);
CREATE INDEX external_tool_exchange_course_idx
    ON external_tool_exchange (tenant_id, course_id, attempt_id, transcript_object_id);
CREATE INDEX external_tool_launch_session_course_idx
    ON external_tool_launch_session (tenant_id, course_id, attempt_id);

ALTER TABLE student_export_request
    ADD CONSTRAINT student_export_request_course_fk
        FOREIGN KEY (tenant_id, course_id)
            REFERENCES course(tenant_id, course_id);
CREATE INDEX student_export_request_course_idx
    ON student_export_request (tenant_id, course_id, export_id, job_id);

-- The delete worker freezes relational identities beside the object manifest.
-- These rows are a private, transactionally populated work set: they replace
-- whole-course arrays, support indexed joins, and are erased in the same
-- transaction that writes the coarse deletion tombstone.
CREATE TABLE course_retention_purge_run (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    generation bigint NOT NULL,
    stage text NOT NULL CHECK (stage = 'deleteStudentRecords'),
    run_id uuid NOT NULL,
    PRIMARY KEY (tenant_id, course_id, generation, stage, run_id),
    FOREIGN KEY (tenant_id, course_id, generation, stage)
        REFERENCES course_retention_cleanup_manifest
            (tenant_id, course_id, generation, stage)
        ON DELETE CASCADE
);

CREATE TABLE course_retention_purge_attempt (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    generation bigint NOT NULL,
    stage text NOT NULL CHECK (stage = 'deleteStudentRecords'),
    attempt_id uuid NOT NULL,
    PRIMARY KEY (tenant_id, course_id, generation, stage, attempt_id),
    FOREIGN KEY (tenant_id, course_id, generation, stage)
        REFERENCES course_retention_cleanup_manifest
            (tenant_id, course_id, generation, stage)
        ON DELETE CASCADE
);

CREATE TABLE course_retention_purge_export (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    generation bigint NOT NULL,
    stage text NOT NULL CHECK (stage = 'deleteStudentRecords'),
    export_id uuid NOT NULL,
    job_id uuid NOT NULL,
    PRIMARY KEY (tenant_id, course_id, generation, stage, export_id),
    UNIQUE (tenant_id, course_id, generation, stage, job_id),
    FOREIGN KEY (tenant_id, course_id, generation, stage)
        REFERENCES course_retention_cleanup_manifest
            (tenant_id, course_id, generation, stage)
        ON DELETE CASCADE
);

ALTER TABLE course_retention_purge_run ENABLE ROW LEVEL SECURITY;
ALTER TABLE course_retention_purge_run FORCE ROW LEVEL SECURITY;
ALTER TABLE course_retention_purge_attempt ENABLE ROW LEVEL SECURITY;
ALTER TABLE course_retention_purge_attempt FORCE ROW LEVEL SECURITY;
ALTER TABLE course_retention_purge_export ENABLE ROW LEVEL SECURITY;
ALTER TABLE course_retention_purge_export FORCE ROW LEVEL SECURITY;

CREATE POLICY retention_purge_run_broker ON course_retention_purge_run
    FOR ALL TO ple_retention_broker
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY retention_purge_attempt_broker ON course_retention_purge_attempt
    FOR ALL TO ple_retention_broker
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY retention_purge_export_broker ON course_retention_purge_export
    FOR ALL TO ple_retention_broker
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

GRANT SELECT, INSERT, DELETE ON
    course_retention_purge_run,
    course_retention_purge_attempt,
    course_retention_purge_export
    TO ple_retention_broker;
REVOKE ALL ON
    course_retention_purge_run,
    course_retention_purge_attempt,
    course_retention_purge_export
    FROM PUBLIC, ple_app, ple_student, ple_grader, ple_queue_broker;

CREATE INDEX submission_next_attempt_next_idx
    ON submission_next_attempt (tenant_id, next_attempt_id)
    WHERE next_attempt_id IS NOT NULL;
CREATE INDEX question_statistics_receipt_run_idx
    ON question_statistics_contribution_receipt (tenant_id, first_completed_run_id);
CREATE INDEX question_statistics_receipt_attempt_idx
    ON question_statistics_contribution_receipt (tenant_id, attempt_id);
CREATE INDEX question_prefetch_predecessor_idx
    ON question_prefetch (tenant_id, predecessor_attempt_id);

-- 2) Strictly enforce ownership through fixed-path triggers so callers cannot
--    assert mismatched learner/course ownership on write.
CREATE OR REPLACE FUNCTION ple_bind_question_attempt_course()
RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public AS $$
DECLARE
    candidate_count bigint;
    candidate_courses uuid[];
BEGIN
    IF NEW.tenant_id IS NULL OR NEW.run_id IS NULL THEN
        RAISE EXCEPTION 'invalid learner run insertion' USING ERRCODE = '22023';
    END IF;

    SELECT COUNT(DISTINCT a.course_id), ARRAY_AGG(DISTINCT a.course_id ORDER BY a.course_id)
      INTO candidate_count, candidate_courses
      FROM public.assignment_run r
      JOIN public.enrollment e
        ON e.tenant_id = r.tenant_id
       AND e.enrollment_id = r.enrollment_id
      JOIN public.assignment a
        ON a.tenant_id = e.tenant_id
       AND a.assignment_id = e.assignment_id
     WHERE r.tenant_id = NEW.tenant_id
       AND r.run_id = NEW.run_id;
    IF candidate_count IS NULL OR candidate_count = 0 THEN
        RAISE EXCEPTION 'invalid learner run insertion' USING ERRCODE = '22023';
    END IF;
    IF candidate_count > 1 THEN
        RAISE EXCEPTION 'ambiguous learner run ownership' USING ERRCODE = '22023';
    END IF;
    IF NEW.course_id IS NOT NULL AND NEW.course_id IS DISTINCT FROM candidate_courses[1] THEN
        RAISE EXCEPTION 'attempt payload course mismatch' USING ERRCODE = '22023';
    END IF;
    NEW.course_id := candidate_courses[1];
    RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION ple_bind_course_from_attempt()
RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public AS $$
DECLARE
    candidate_count bigint;
    candidate_courses uuid[];
BEGIN
    IF NEW.tenant_id IS NULL OR NEW.attempt_id IS NULL THEN
        RAISE EXCEPTION 'invalid learner attempt insertion' USING ERRCODE = '22023';
    END IF;

    SELECT COUNT(DISTINCT a.course_id), ARRAY_AGG(DISTINCT a.course_id ORDER BY a.course_id)
      INTO candidate_count, candidate_courses
      FROM public.question_attempt q
      JOIN public.assignment_run r
        ON r.tenant_id = q.tenant_id
       AND r.run_id = q.run_id
      JOIN public.enrollment e
        ON e.tenant_id = r.tenant_id
       AND e.enrollment_id = r.enrollment_id
      JOIN public.assignment a
        ON a.tenant_id = e.tenant_id
       AND a.assignment_id = e.assignment_id
     WHERE q.tenant_id = NEW.tenant_id
       AND q.attempt_id = NEW.attempt_id;
    IF candidate_count IS NULL OR candidate_count = 0 THEN
        RAISE EXCEPTION 'invalid learner attempt insertion' USING ERRCODE = '22023';
    END IF;
    IF candidate_count > 1 THEN
        RAISE EXCEPTION 'ambiguous learner attempt ownership' USING ERRCODE = '22023';
    END IF;
    IF NEW.course_id IS NOT NULL AND NEW.course_id IS DISTINCT FROM candidate_courses[1] THEN
        RAISE EXCEPTION 'payload course mismatch' USING ERRCODE = '22023';
    END IF;
    NEW.course_id := candidate_courses[1];
    RETURN NEW;
END $$;

ALTER FUNCTION ple_bind_question_attempt_course() OWNER TO ple_retention_broker;
ALTER FUNCTION ple_bind_course_from_attempt() OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION ple_bind_question_attempt_course() FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_bind_course_from_attempt() FROM PUBLIC;

DROP TRIGGER IF EXISTS question_attempt_bind_course ON question_attempt;
CREATE TRIGGER question_attempt_bind_course
    BEFORE INSERT OR UPDATE ON question_attempt
    FOR EACH ROW EXECUTE FUNCTION ple_bind_question_attempt_course();

DROP TRIGGER IF EXISTS submission_bind_course ON submission;
CREATE TRIGGER submission_bind_course
    BEFORE INSERT OR UPDATE ON submission
    FOR EACH ROW EXECUTE FUNCTION ple_bind_course_from_attempt();

DROP TRIGGER IF EXISTS grade_event_bind_course ON grade_event;
CREATE TRIGGER grade_event_bind_course
    BEFORE INSERT OR UPDATE ON grade_event
    FOR EACH ROW EXECUTE FUNCTION ple_bind_course_from_attempt();

DROP TRIGGER IF EXISTS submission_idempotency_bind_course ON submission_idempotency;
CREATE TRIGGER submission_idempotency_bind_course
    BEFORE INSERT OR UPDATE ON submission_idempotency
    FOR EACH ROW EXECUTE FUNCTION ple_bind_course_from_attempt();

DROP TRIGGER IF EXISTS attempt_feedback_bind_course ON attempt_feedback;
CREATE TRIGGER attempt_feedback_bind_course
    BEFORE INSERT OR UPDATE ON attempt_feedback
    FOR EACH ROW EXECUTE FUNCTION ple_bind_course_from_attempt();

DROP TRIGGER IF EXISTS external_tool_exchange_bind_course ON external_tool_exchange;
CREATE TRIGGER external_tool_exchange_bind_course
    BEFORE INSERT OR UPDATE ON external_tool_exchange
    FOR EACH ROW EXECUTE FUNCTION ple_bind_course_from_attempt();

DROP TRIGGER IF EXISTS external_tool_launch_session_bind_course
    ON external_tool_launch_session;
CREATE TRIGGER external_tool_launch_session_bind_course
    BEFORE INSERT OR UPDATE ON external_tool_launch_session
    FOR EACH ROW EXECUTE FUNCTION ple_bind_course_from_attempt();

-- 3) Keep learner rows inaccessible once delete lifecycle begins.
CREATE OR REPLACE FUNCTION ple_course_records_accessible(p_tenant uuid, p_course uuid)
RETURNS boolean
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public AS $$
DECLARE
    current_generation bigint;
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

    IF EXISTS (
        SELECT 1 FROM public.course_retention
         WHERE tenant_id = p_tenant AND course_id = p_course
           AND lifecycle IN ('archived', 'deleted')
    ) THEN
        RETURN false;
    END IF;

    IF EXISTS (
        SELECT 1 FROM public.course_retention_stage
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = current_generation
           AND stage IN ('archiveStudentRecords', 'deleteStudentRecords')
           AND state = 'started'
    ) THEN
        RETURN false;
    END IF;

    RETURN NOT EXISTS (
        SELECT 1 FROM public.course_retention_stage
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = current_generation
           AND stage = 'archiveStudentRecords'
           AND state = 'started'
    );
END $$;

ALTER FUNCTION ple_course_records_accessible(uuid, uuid)
    OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION ple_course_records_accessible(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_course_records_accessible(uuid, uuid)
    TO ple_app, ple_student, ple_queue_broker, ple_retention_broker;

-- A visibility predicate alone cannot close the transaction that started just
-- before archival. Every producer therefore takes a shared lock on the exact
-- course-retention row before it writes. Retention prepare/commit already take
-- the conflicting row lock, so unrelated courses remain independent while a
-- same-course writer either finishes before the manifest or observes the
-- archived lifecycle after it waits.
CREATE OR REPLACE FUNCTION ple_lock_course_write(
    p_tenant uuid,
    p_course uuid,
    p_definition boolean
) RETURNS boolean
LANGUAGE plpgsql
VOLATILE
SECURITY DEFINER
SET search_path = pg_catalog, public AS $$
DECLARE
    current_generation bigint;
    current_lifecycle text;
    current_disposition text;
BEGIN
    IF p_tenant IS NULL
       OR p_course IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR NOT EXISTS (
           SELECT 1
             FROM public.course c
            WHERE c.tenant_id = p_tenant
              AND c.course_id = p_course
       )
    THEN
        RETURN false;
    END IF;

    SELECT r.generation, r.lifecycle, r.assignment_disposition
      INTO current_generation, current_lifecycle, current_disposition
      FROM public.course_retention r
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course
     FOR SHARE;
    IF NOT FOUND THEN
        RETURN true;
    END IF;

    IF p_definition THEN
        RETURN current_lifecycle = 'active' OR current_disposition = 'retain';
    END IF;
    IF current_lifecycle <> 'active' THEN
        RETURN false;
    END IF;

    RETURN NOT EXISTS (
        SELECT 1
          FROM public.course_retention_stage s
         WHERE s.tenant_id = p_tenant
           AND s.course_id = p_course
           AND s.generation = current_generation
           AND s.stage IN ('archiveStudentRecords', 'deleteStudentRecords')
           AND s.state = 'started'
    );
END $$;

CREATE OR REPLACE FUNCTION ple_fence_learner_record_write()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public AS $$
DECLARE
    owner_tenant uuid;
    owner_course uuid;
    related_course uuid;
BEGIN
    IF TG_TABLE_NAME IN (
        'question_attempt',
        'submission',
        'grade_event',
        'submission_idempotency',
        'attempt_feedback',
        'external_tool_exchange',
        'external_tool_launch_session',
        'student_export_request'
    ) THEN
        owner_tenant := NEW.tenant_id;
        owner_course := NEW.course_id;
        IF TG_OP = 'UPDATE' THEN
            IF OLD.course_id IS DISTINCT FROM NEW.course_id THEN
                RAISE EXCEPTION 'learner record course ownership is immutable'
                    USING ERRCODE = '22023';
            END IF;
        END IF;
    ELSIF TG_TABLE_NAME = 'asset_delivery' THEN
        IF NEW.delivery_kind <> 'student_record' THEN
            RETURN NEW;
        END IF;
        owner_tenant := NEW.tenant_id;
        owner_course := NEW.course_id;
        IF TG_OP = 'UPDATE' THEN
            IF OLD.course_id IS DISTINCT FROM NEW.course_id THEN
                RAISE EXCEPTION 'student delivery course ownership is immutable'
                    USING ERRCODE = '22023';
            END IF;
        END IF;
    ELSIF TG_TABLE_NAME = 'audit_event' THEN
        IF NEW.delivery_scope <> 'student_record' THEN
            RETURN NEW;
        END IF;
        owner_tenant := NEW.tenant_id;
        owner_course := NEW.course_id;
        IF TG_OP = 'UPDATE' THEN
            IF OLD.course_id IS DISTINCT FROM NEW.course_id THEN
                RAISE EXCEPTION 'student audit course ownership is immutable'
                    USING ERRCODE = '22023';
            END IF;
        END IF;
    ELSIF TG_TABLE_NAME = 'course_member' THEN
        IF TG_OP = 'INSERT' AND NEW.role <> 'student' THEN
            RETURN NEW;
        END IF;
        IF TG_OP = 'UPDATE' THEN
            IF OLD.role <> 'student' AND NEW.role <> 'student' THEN
                RETURN NEW;
            END IF;
        END IF;
        owner_tenant := NEW.tenant_id;
        owner_course := NEW.course_id;
    ELSIF TG_TABLE_NAME IN ('assignment_problem', 'enrollment') THEN
        SELECT a.tenant_id, a.course_id
          INTO owner_tenant, owner_course
          FROM public.assignment a
         WHERE a.tenant_id = NEW.tenant_id
           AND a.assignment_id = NEW.assignment_id;
    ELSIF TG_TABLE_NAME IN ('student_assignment_summary', 'assignment_run') THEN
        SELECT a.tenant_id, a.course_id
          INTO owner_tenant, owner_course
          FROM public.enrollment e
          JOIN public.assignment a
            ON a.tenant_id = e.tenant_id
           AND a.assignment_id = e.assignment_id
         WHERE e.tenant_id = NEW.tenant_id
           AND e.enrollment_id = NEW.enrollment_id;
    ELSIF TG_TABLE_NAME = 'feedback_release' THEN
        SELECT af.tenant_id, af.course_id
          INTO owner_tenant, owner_course
          FROM public.attempt_feedback af
         WHERE af.tenant_id = NEW.tenant_id
           AND af.attempt_id = NEW.attempt_id;
    ELSIF TG_TABLE_NAME = 'submission_receipt_snapshot' THEN
        SELECT si.tenant_id, si.course_id
          INTO owner_tenant, owner_course
          FROM public.submission_idempotency si
         WHERE si.tenant_id = NEW.tenant_id
           AND si.attempt_id = NEW.attempt_id;
    ELSIF TG_TABLE_NAME = 'submission_next_attempt' THEN
        SELECT si.tenant_id, si.course_id
          INTO owner_tenant, owner_course
          FROM public.submission_idempotency si
         WHERE si.tenant_id = NEW.tenant_id
           AND si.attempt_id = NEW.predecessor_attempt_id;
        IF NEW.next_attempt_id IS NOT NULL THEN
            SELECT qa.course_id
              INTO related_course
              FROM public.question_attempt qa
             WHERE qa.tenant_id = NEW.tenant_id
               AND qa.attempt_id = NEW.next_attempt_id;
            IF NOT FOUND OR related_course IS DISTINCT FROM owner_course THEN
                RAISE EXCEPTION 'successor attempt crosses a course boundary'
                    USING ERRCODE = '22023';
            END IF;
        END IF;
    ELSIF TG_TABLE_NAME = 'question_prefetch' THEN
        SELECT a.tenant_id, a.course_id
          INTO owner_tenant, owner_course
          FROM public.assignment_run ar
          JOIN public.enrollment e
            ON e.tenant_id = ar.tenant_id
           AND e.enrollment_id = ar.enrollment_id
          JOIN public.assignment a
            ON a.tenant_id = e.tenant_id
           AND a.assignment_id = e.assignment_id
         WHERE ar.tenant_id = NEW.tenant_id
           AND ar.run_id = NEW.run_id;
        SELECT qa.course_id
          INTO related_course
          FROM public.question_attempt qa
         WHERE qa.tenant_id = NEW.tenant_id
           AND qa.attempt_id = NEW.predecessor_attempt_id;
        IF NOT FOUND OR related_course IS DISTINCT FROM owner_course THEN
            RAISE EXCEPTION 'prefetch predecessor crosses a course boundary'
                USING ERRCODE = '22023';
        END IF;
    ELSIF TG_TABLE_NAME = 'question_statistics_contribution_receipt' THEN
        SELECT si.tenant_id, si.course_id
          INTO owner_tenant, owner_course
          FROM public.submission_idempotency si
         WHERE si.tenant_id = NEW.tenant_id
           AND si.attempt_id = NEW.attempt_id;
        SELECT a.course_id
          INTO related_course
          FROM public.assignment_run ar
          JOIN public.enrollment e
            ON e.tenant_id = ar.tenant_id
           AND e.enrollment_id = ar.enrollment_id
          JOIN public.assignment a
            ON a.tenant_id = e.tenant_id
           AND a.assignment_id = e.assignment_id
         WHERE ar.tenant_id = NEW.tenant_id
           AND ar.run_id = NEW.first_completed_run_id
           AND e.enrollment_id = NEW.enrollment_id;
        IF NOT FOUND OR related_course IS DISTINCT FROM owner_course THEN
            RAISE EXCEPTION 'statistics receipt crosses a course boundary'
                USING ERRCODE = '22023';
        END IF;
    ELSIF TG_TABLE_NAME = 'student_export_artifact' THEN
        SELECT r.tenant_id, r.course_id
          INTO owner_tenant, owner_course
          FROM public.student_export_request r
         WHERE r.export_id = NEW.export_id;
    ELSE
        RAISE EXCEPTION 'unsupported learner record fence table'
            USING ERRCODE = '22023';
    END IF;

    IF owner_tenant IS NULL
       OR owner_course IS NULL
       OR NOT public.ple_lock_course_write(owner_tenant, owner_course, false)
    THEN
        RAISE EXCEPTION 'learner record course is unavailable'
            USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION ple_fence_assignment_definition_write()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public AS $$
DECLARE
    owner_tenant uuid;
    owner_course uuid;
BEGIN
    IF TG_TABLE_NAME = 'assignment' THEN
        owner_tenant := NEW.tenant_id;
        owner_course := NEW.course_id;
        IF TG_OP = 'UPDATE' THEN
            IF OLD.course_id IS DISTINCT FROM NEW.course_id THEN
                RAISE EXCEPTION 'assignment course ownership is immutable'
                    USING ERRCODE = '22023';
            END IF;
        END IF;
    ELSIF TG_TABLE_NAME = 'assignment_problem' THEN
        SELECT a.tenant_id, a.course_id
          INTO owner_tenant, owner_course
          FROM public.assignment a
         WHERE a.tenant_id = NEW.tenant_id
           AND a.assignment_id = NEW.assignment_id;
    ELSE
        RAISE EXCEPTION 'unsupported assignment definition fence table'
            USING ERRCODE = '22023';
    END IF;

    IF owner_tenant IS NULL
       OR owner_course IS NULL
       OR NOT public.ple_lock_course_write(owner_tenant, owner_course, true)
    THEN
        RAISE EXCEPTION 'assignment definition course is unavailable'
            USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END $$;

ALTER FUNCTION ple_lock_course_write(uuid, uuid, boolean)
    OWNER TO ple_retention_broker;
ALTER FUNCTION ple_fence_learner_record_write()
    OWNER TO ple_retention_broker;
ALTER FUNCTION ple_fence_assignment_definition_write()
    OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION ple_lock_course_write(uuid, uuid, boolean),
    ple_fence_learner_record_write(),
    ple_fence_assignment_definition_write()
    FROM PUBLIC;

CREATE TRIGGER assignment_retention_fence
    BEFORE INSERT OR UPDATE ON assignment
    FOR EACH ROW EXECUTE FUNCTION ple_fence_assignment_definition_write();
CREATE TRIGGER assignment_problem_retention_fence
    BEFORE INSERT OR UPDATE ON assignment_problem
    FOR EACH ROW EXECUTE FUNCTION ple_fence_assignment_definition_write();
CREATE TRIGGER enrollment_retention_fence
    BEFORE INSERT OR UPDATE ON enrollment
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER student_assignment_summary_retention_fence
    BEFORE INSERT OR UPDATE ON student_assignment_summary
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER assignment_run_retention_fence
    BEFORE INSERT OR UPDATE ON assignment_run
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER question_attempt_retention_fence
    BEFORE INSERT OR UPDATE ON question_attempt
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER submission_retention_fence
    BEFORE INSERT OR UPDATE ON submission
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER grade_event_retention_fence
    BEFORE INSERT OR UPDATE ON grade_event
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER submission_idempotency_retention_fence
    BEFORE INSERT OR UPDATE ON submission_idempotency
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER attempt_feedback_retention_fence
    BEFORE INSERT OR UPDATE ON attempt_feedback
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER feedback_release_retention_fence
    BEFORE INSERT OR UPDATE ON feedback_release
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER submission_receipt_snapshot_retention_fence
    BEFORE INSERT OR UPDATE ON submission_receipt_snapshot
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER question_prefetch_retention_fence
    BEFORE INSERT OR UPDATE ON question_prefetch
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER submission_next_attempt_retention_fence
    BEFORE INSERT OR UPDATE ON submission_next_attempt
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER statistics_receipt_retention_fence
    BEFORE INSERT OR UPDATE ON question_statistics_contribution_receipt
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER external_tool_exchange_retention_fence
    BEFORE INSERT OR UPDATE ON external_tool_exchange
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER external_tool_launch_session_retention_fence
    BEFORE INSERT OR UPDATE ON external_tool_launch_session
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER student_export_request_retention_fence
    BEFORE INSERT ON student_export_request
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER student_export_artifact_retention_fence
    BEFORE INSERT OR UPDATE ON student_export_artifact
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER asset_delivery_retention_fence
    BEFORE INSERT OR UPDATE ON asset_delivery
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER audit_event_retention_fence
    BEFORE INSERT OR UPDATE ON audit_event
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();
CREATE TRIGGER course_member_retention_fence
    BEFORE INSERT OR UPDATE ON course_member
    FOR EACH ROW EXECUTE FUNCTION ple_fence_learner_record_write();

-- 4) Preserve earlier helper behavior as private stage helpers.
ALTER FUNCTION ple_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    RENAME TO r44a_prepare_retention_work;
ALTER FUNCTION ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    RENAME TO r44a_commit_retention_work;

-- 5) Delete helper: build exact learner manifest and persist before return.
CREATE OR REPLACE FUNCTION r44b_prepare_retention_work(p_tenant uuid, p_job uuid, p_token uuid,
                                           p_course uuid, p_stage text, p_generation bigint)
RETURNS jsonb
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public AS $$
DECLARE
    current_lifecycle text;
    existing_state text;
    existing_job uuid;
    existing_object_count bigint;
    manifest_count bigint;
    manifest jsonb;
    object_count bigint;
BEGIN
    IF p_tenant IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_generation <= 0
       OR p_course IS NULL
       OR p_token IS NULL
       OR p_job IS NULL
       OR p_stage <> 'deleteStudentRecords'
    THEN
        RETURN NULL;
    END IF;

    PERFORM 1
      FROM public.worker_job w
      JOIN public.course_retention r
        ON r.tenant_id = w.tenant_id AND r.course_id = p_course
      JOIN public.course_retention_dispatch d
        ON d.tenant_id = w.tenant_id
       AND d.course_id = p_course
       AND d.stage = p_stage
       AND d.generation = p_generation
       AND d.job_id = w.job_id
     WHERE w.job_id = p_job
       AND w.tenant_id = p_tenant
       AND w.state = 'leased'
       AND w.lease_token = p_token
       AND w.lease_expires_at > transaction_timestamp()
       AND w.payload = jsonb_build_object('kind', 'retention', 'course', p_course::text,
                                          'stage', p_stage, 'generation', p_generation)
     FOR UPDATE OF w;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    PERFORM 1
      FROM public.course_retention_stage s
     WHERE s.tenant_id = p_tenant
       AND s.course_id = p_course
       AND s.stage = p_stage
       AND s.generation = p_generation
       AND s.due_at <= transaction_timestamp()
       AND (s.state = 'scheduled' OR (s.state = 'started' AND s.job_id = p_job))
     FOR UPDATE;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    SELECT r.lifecycle INTO current_lifecycle
      FROM public.course_retention r
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course
       AND r.generation = p_generation
     FOR UPDATE;
    IF NOT FOUND OR current_lifecycle NOT IN ('active', 'archived') THEN
        RETURN NULL;
    END IF;

    SELECT m.state,
           m.job_id,
           m.object_count,
           COALESCE((
               SELECT COUNT(*)
                 FROM public.course_retention_cleanup_manifest_object o
                WHERE o.tenant_id = m.tenant_id
                  AND o.course_id = m.course_id
                  AND o.generation = m.generation
                  AND o.stage = m.stage
           ), 0)
      INTO existing_state,
           existing_job,
           existing_object_count,
           manifest_count
      FROM public.course_retention_cleanup_manifest m
     WHERE m.tenant_id = p_tenant
       AND m.course_id = p_course
       AND m.stage = p_stage
       AND m.generation = p_generation
     FOR UPDATE;
    IF FOUND THEN
        IF existing_state <> 'prepared'
           OR existing_job IS DISTINCT FROM p_job
           OR existing_object_count IS DISTINCT FROM manifest_count THEN
            RETURN NULL;
        END IF;
        IF EXISTS (
            (SELECT ar.run_id
               FROM public.assignment_run ar
               JOIN public.enrollment e
                 ON e.tenant_id = ar.tenant_id
                AND e.enrollment_id = ar.enrollment_id
               JOIN public.assignment a
                 ON a.tenant_id = e.tenant_id
                AND a.assignment_id = e.assignment_id
              WHERE ar.tenant_id = p_tenant
                AND a.course_id = p_course
             EXCEPT
             SELECT s.run_id
               FROM public.course_retention_purge_run s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage)
            UNION ALL
            (SELECT s.run_id
               FROM public.course_retention_purge_run s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage
             EXCEPT
             SELECT ar.run_id
               FROM public.assignment_run ar
               JOIN public.enrollment e
                 ON e.tenant_id = ar.tenant_id
                AND e.enrollment_id = ar.enrollment_id
               JOIN public.assignment a
                 ON a.tenant_id = e.tenant_id
                AND a.assignment_id = e.assignment_id
              WHERE ar.tenant_id = p_tenant
                AND a.course_id = p_course)
        ) OR EXISTS (
            (SELECT qa.attempt_id
               FROM public.question_attempt qa
              WHERE qa.tenant_id = p_tenant
                AND qa.course_id = p_course
             EXCEPT
             SELECT s.attempt_id
               FROM public.course_retention_purge_attempt s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage)
            UNION ALL
            (SELECT s.attempt_id
               FROM public.course_retention_purge_attempt s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage
             EXCEPT
             SELECT qa.attempt_id
               FROM public.question_attempt qa
              WHERE qa.tenant_id = p_tenant
                AND qa.course_id = p_course)
        ) OR EXISTS (
            (SELECT r.export_id, r.job_id
               FROM public.student_export_request r
              WHERE r.tenant_id = p_tenant
                AND r.course_id = p_course
             EXCEPT
             SELECT s.export_id, s.job_id
               FROM public.course_retention_purge_export s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage)
            UNION ALL
            (SELECT s.export_id, s.job_id
               FROM public.course_retention_purge_export s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage
             EXCEPT
             SELECT r.export_id, r.job_id
               FROM public.student_export_request r
              WHERE r.tenant_id = p_tenant
                AND r.course_id = p_course)
        ) THEN
            RETURN NULL;
        END IF;

        UPDATE public.course_retention_stage
           SET state = 'started',
               job_id = p_job,
               lease_token = p_token,
               claimed_at = transaction_timestamp()
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND stage = p_stage
           AND generation = p_generation;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'failed to bind delete retention stage';
        END IF;

        SELECT COALESCE(jsonb_agg(object_id::text ORDER BY object_id), '[]'::jsonb)
          INTO manifest
          FROM public.course_retention_cleanup_manifest_object
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = p_generation
           AND stage = p_stage;
        RETURN jsonb_build_object('kind', 'cleanup', 'objects', manifest);
    END IF;

    IF EXISTS (
        SELECT 1
          FROM public.student_export_request r
          JOIN public.student_export_artifact a
            ON a.export_id = r.export_id
         WHERE r.tenant_id = p_tenant
           AND r.course_id = p_course
           AND a.object_payload IS NOT NULL
           AND (
               jsonb_typeof(a.object_payload) <> 'object'
               OR a.object_payload->>'id' <> a.object_id::text
               OR a.object_payload->>'bucket' <> 'student-records'
               OR a.object_payload->>'category' <> 'export'
               OR jsonb_typeof(a.object_payload->'key') <> 'object'
               OR a.object_payload->'key'->>'kind' <> 'studentRecord'
               OR a.object_payload->'key'->>'tenant' <> p_tenant::text
               OR a.object_payload->'key'->>'object' <> a.object_id::text
           )
    ) THEN
        RAISE EXCEPTION 'invalid student-record retention manifest';
    END IF;

    IF EXISTS (
        SELECT 1 FROM public.external_tool_exchange et
         WHERE et.tenant_id = p_tenant
           AND et.course_id = p_course
           AND et.transcript_object_id IS NOT NULL
           AND NOT EXISTS (
               SELECT 1
                 FROM public.question_attempt qa
                 JOIN public.assignment_run ar
                   ON ar.tenant_id = qa.tenant_id
                  AND ar.run_id = qa.run_id
                 JOIN public.enrollment e
                   ON e.tenant_id = ar.tenant_id
                  AND e.enrollment_id = ar.enrollment_id
                 JOIN public.assignment a
                   ON a.tenant_id = e.tenant_id
                  AND a.assignment_id = e.assignment_id
                WHERE qa.tenant_id = et.tenant_id
                  AND qa.attempt_id = et.attempt_id
                  AND a.course_id = p_course
           )
    ) THEN
        RAISE EXCEPTION 'cannot verify external tool transcript ownership';
    END IF;

    WITH manifest_object AS (
        SELECT DISTINCT a.object_id AS object_id
          FROM public.student_export_request r
          JOIN public.student_export_artifact a
            ON a.export_id = r.export_id
         WHERE r.tenant_id = p_tenant
           AND r.course_id = p_course
        UNION
        SELECT DISTINCT d.object_id AS object_id

          FROM public.asset_delivery d
         WHERE d.tenant_id = p_tenant
           AND d.course_id = p_course
           AND d.delivery_kind = 'student_record'
        UNION
        SELECT DISTINCT et.transcript_object_id AS object_id
          FROM public.external_tool_exchange et
         WHERE et.tenant_id = p_tenant
           AND et.course_id = p_course
           AND et.transcript_object_id IS NOT NULL
    )
    SELECT COALESCE(jsonb_agg(object_id::text ORDER BY object_id), '[]'::jsonb),
           COUNT(*)
      INTO manifest, object_count
      FROM manifest_object;

    UPDATE public.course_retention_stage
       SET state = 'started',
           job_id = p_job,
           lease_token = p_token,
           claimed_at = transaction_timestamp()
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND stage = p_stage
       AND generation = p_generation;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to bind delete retention stage';
    END IF;

    UPDATE public.worker_job w
       SET state = 'dead',
           lease_token = NULL,
           lease_expires_at = NULL,
           completed_at = transaction_timestamp(),
           last_error = 'permanent'
      FROM public.student_export_request r
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course
       AND r.job_id = w.job_id
       AND r.state = 'queued'
       AND w.state IN ('ready', 'leased');

    UPDATE public.student_export_request
       SET state = 'failed'
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND state = 'queued';

    INSERT INTO public.course_retention_cleanup_manifest
        (tenant_id, course_id, generation, stage, job_id, state,
         object_count, prepared_at)
    VALUES (p_tenant, p_course, p_generation, p_stage, p_job, 'prepared',
            object_count, transaction_timestamp());

    INSERT INTO public.course_retention_cleanup_manifest_object
        (tenant_id, course_id, generation, stage, object_id)
    SELECT p_tenant, p_course, p_generation, p_stage, manifest_object.object_id
      FROM (
          SELECT a.object_id
            FROM public.student_export_request r
            JOIN public.student_export_artifact a
              ON a.export_id = r.export_id
           WHERE r.tenant_id = p_tenant
             AND r.course_id = p_course
          UNION
          SELECT d.object_id
            FROM public.asset_delivery d
           WHERE d.tenant_id = p_tenant
             AND d.course_id = p_course
             AND d.delivery_kind = 'student_record'
          UNION
          SELECT et.transcript_object_id
            FROM public.external_tool_exchange et
           WHERE et.tenant_id = p_tenant
             AND et.course_id = p_course
             AND et.transcript_object_id IS NOT NULL
      ) AS manifest_object;

    INSERT INTO public.course_retention_purge_run
        (tenant_id, course_id, generation, stage, run_id)
    SELECT p_tenant, p_course, p_generation, p_stage, ar.run_id
      FROM public.assignment_run ar
      JOIN public.enrollment e
        ON e.tenant_id = ar.tenant_id
       AND e.enrollment_id = ar.enrollment_id
      JOIN public.assignment a
        ON a.tenant_id = e.tenant_id
       AND a.assignment_id = e.assignment_id
     WHERE ar.tenant_id = p_tenant
       AND a.course_id = p_course;

    INSERT INTO public.course_retention_purge_attempt
        (tenant_id, course_id, generation, stage, attempt_id)
    SELECT p_tenant, p_course, p_generation, p_stage, qa.attempt_id
      FROM public.question_attempt qa
     WHERE qa.tenant_id = p_tenant
       AND qa.course_id = p_course;

    INSERT INTO public.course_retention_purge_export
        (tenant_id, course_id, generation, stage, export_id, job_id)
    SELECT p_tenant, p_course, p_generation, p_stage, r.export_id, r.job_id
      FROM public.student_export_request r
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course;

    DELETE FROM public.asset_delivery
     WHERE tenant_id = p_tenant
       AND delivery_kind = 'student_record'
       AND course_id = p_course;

    IF current_lifecycle = 'active' THEN
        UPDATE public.course_retention
           SET lifecycle = 'archived'
         WHERE tenant_id = p_tenant
           AND course_id = p_course
           AND generation = p_generation
           AND lifecycle = 'active';
        IF NOT FOUND THEN
            RAISE EXCEPTION 'failed to archive retention lifecycle before delete work';
        END IF;
    END IF;

    RETURN jsonb_build_object('kind', 'cleanup', 'objects', manifest);
END $$;

CREATE OR REPLACE FUNCTION r44b_commit_retention_work(p_tenant uuid, p_job uuid, p_token uuid,
                                          p_course uuid, p_stage text, p_generation bigint)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public AS $$
DECLARE
    prepared_count bigint;
    manifest_count bigint;
    current_lifecycle text;
    frozen_assignment_disposition text;
BEGIN
    IF p_tenant IS NULL
       OR p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_course IS NULL
       OR p_generation <= 0
       OR p_stage <> 'deleteStudentRecords'
    THEN
        RAISE EXCEPTION 'invalid retention worker capability' USING ERRCODE = '22023';
    END IF;

    PERFORM 1
      FROM public.worker_job w
      JOIN public.course_retention_dispatch d
        ON d.tenant_id = w.tenant_id
       AND d.course_id = p_course
       AND d.stage = p_stage
       AND d.generation = p_generation
       AND d.job_id = w.job_id
      JOIN public.course_retention r
        ON r.tenant_id = d.tenant_id
       AND r.course_id = d.course_id
       AND r.generation = d.generation
     WHERE w.job_id = p_job
       AND w.tenant_id = p_tenant
       AND w.state = 'leased'
       AND w.lease_token = p_token
       AND w.lease_expires_at > transaction_timestamp()
       AND w.payload = jsonb_build_object('kind', 'retention', 'course', p_course::text,
                                          'stage', p_stage, 'generation', p_generation)
     FOR UPDATE OF w, r;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    PERFORM 1
      FROM public.course_retention_stage s
     WHERE s.tenant_id = p_tenant
       AND s.course_id = p_course
       AND s.stage = p_stage
       AND s.generation = p_generation
       AND s.state = 'started'
       AND s.job_id = p_job
       AND s.lease_token = p_token
     FOR UPDATE;
    IF NOT FOUND THEN
        RETURN false;
    END IF;

    SELECT r.lifecycle, r.assignment_disposition
      INTO current_lifecycle, frozen_assignment_disposition
      FROM public.course_retention r
     WHERE r.tenant_id = p_tenant
       AND r.course_id = p_course
       AND r.generation = p_generation
     FOR UPDATE;
    IF NOT FOUND OR current_lifecycle <> 'archived' THEN
        RETURN false;
    END IF;

    SELECT m.object_count,
           COALESCE((
               SELECT COUNT(*)
                 FROM public.course_retention_cleanup_manifest_object o
                WHERE o.tenant_id = m.tenant_id
                  AND o.course_id = m.course_id
                  AND o.generation = m.generation
                  AND o.stage = m.stage
           ), 0)
      INTO prepared_count, manifest_count
      FROM public.course_retention_cleanup_manifest m
     WHERE m.tenant_id = p_tenant
       AND m.course_id = p_course
       AND m.stage = p_stage
       AND m.generation = p_generation
       AND m.job_id = p_job
       AND m.state = 'prepared'
     FOR UPDATE;
    IF NOT FOUND OR prepared_count IS NULL THEN
        RETURN false;
    END IF;
    IF prepared_count IS DISTINCT FROM manifest_count THEN
        RAISE EXCEPTION 'prepared manifest rows do not match manifest object count';
    END IF;

    -- The producer triggers share-lock this course's retention row. The
    -- worker's existing row lock therefore freezes only this course while the
    -- indexed work-set tables drive the FK-safe purge below.
    DELETE FROM public.feedback_release fr
     WHERE EXISTS (
         SELECT 1
           FROM public.attempt_feedback af
          WHERE af.tenant_id = fr.tenant_id
            AND af.attempt_id = fr.attempt_id
            AND af.course_id = p_course
            AND af.tenant_id = p_tenant
     );

    DELETE FROM public.submission_receipt_snapshot srs
     WHERE EXISTS (
         SELECT 1
           FROM public.submission_idempotency si
          WHERE si.tenant_id = srs.tenant_id
            AND si.course_id = p_course
            AND si.attempt_id = srs.attempt_id
            AND si.tenant_id = p_tenant
     );

    DELETE FROM public.submission_next_attempt sna
     WHERE sna.tenant_id = p_tenant
       AND (
           EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_attempt s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.attempt_id = sna.predecessor_attempt_id
           )
           OR EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_attempt s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.attempt_id = sna.next_attempt_id
           )
       );

    DELETE FROM public.question_statistics_contribution_receipt qsr
     WHERE qsr.tenant_id = p_tenant
       AND (
           EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_run s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.run_id = qsr.first_completed_run_id
           )
           OR EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_attempt s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.attempt_id = qsr.attempt_id
           )
       );

    DELETE FROM public.question_prefetch qp
     WHERE qp.tenant_id = p_tenant
       AND (
           EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_run s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.run_id = qp.run_id
           )
           OR EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_attempt s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.attempt_id = qp.predecessor_attempt_id
           )
       );

    DELETE FROM public.external_tool_launch_session
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.external_tool_exchange
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.attempt_feedback
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.grade_event
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.submission
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.submission_idempotency
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.question_attempt
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.student_assignment_summary sas
     WHERE EXISTS (
         SELECT 1
           FROM public.enrollment e
           JOIN public.assignment a
             ON a.tenant_id = e.tenant_id
            AND a.assignment_id = e.assignment_id
          WHERE e.tenant_id = sas.tenant_id
            AND e.enrollment_id = sas.enrollment_id
            AND a.course_id = p_course
            AND sas.tenant_id = p_tenant
     );

    DELETE FROM public.assignment_run ar
     WHERE EXISTS (
         SELECT 1
           FROM public.enrollment e
           JOIN public.assignment a
             ON a.tenant_id = e.tenant_id
            AND a.assignment_id = e.assignment_id
          WHERE e.tenant_id = ar.tenant_id
            AND e.enrollment_id = ar.enrollment_id
            AND a.course_id = p_course
            AND ar.tenant_id = p_tenant
     );

    DELETE FROM public.enrollment e
     WHERE EXISTS (
         SELECT 1
           FROM public.assignment a
          WHERE a.tenant_id = e.tenant_id
            AND a.assignment_id = e.assignment_id
            AND a.course_id = p_course
            AND e.tenant_id = p_tenant
     );

    DELETE FROM public.audit_event
     WHERE tenant_id = p_tenant
       AND delivery_scope = 'student_record'
       AND course_id = p_course;

    DELETE FROM public.asset_delivery
     WHERE tenant_id = p_tenant
       AND delivery_kind = 'student_record'
       AND course_id = p_course;

    DELETE FROM public.student_export_artifact a
     WHERE EXISTS (
         SELECT 1
           FROM public.course_retention_purge_export s
          WHERE s.tenant_id = p_tenant
            AND s.course_id = p_course
            AND s.generation = p_generation
            AND s.stage = p_stage
            AND s.export_id = a.export_id
     );

    DELETE FROM public.student_export_request
     WHERE tenant_id = p_tenant
       AND course_id = p_course;

    DELETE FROM public.worker_job w
     WHERE w.tenant_id = p_tenant
       AND EXISTS (
           SELECT 1
             FROM public.course_retention_purge_export s
            WHERE s.tenant_id = p_tenant
              AND s.course_id = p_course
              AND s.generation = p_generation
              AND s.stage = p_stage
              AND s.job_id = w.job_id
       );

    DELETE FROM public.course_member
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND role = 'student';

    IF frozen_assignment_disposition = 'delete' THEN
        DELETE FROM public.assignment_problem ap
         USING public.assignment a
         WHERE ap.tenant_id = a.tenant_id
           AND ap.assignment_id = a.assignment_id
           AND a.tenant_id = p_tenant
           AND a.course_id = p_course;

        DELETE FROM public.assignment a
         WHERE a.tenant_id = p_tenant
           AND a.course_id = p_course;
    END IF;

    IF EXISTS (
        SELECT 1 FROM public.feedback_release fr
         WHERE fr.tenant_id = p_tenant
           AND EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_attempt s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.attempt_id = fr.attempt_id
           )
        UNION ALL
        SELECT 1 FROM public.submission_receipt_snapshot srs
         WHERE srs.tenant_id = p_tenant
           AND EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_attempt s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.attempt_id = srs.attempt_id
           )
        UNION ALL
        SELECT 1 FROM public.submission_next_attempt sna
         WHERE sna.tenant_id = p_tenant
           AND (
               EXISTS (
                   SELECT 1
                     FROM public.course_retention_purge_attempt s
                    WHERE s.tenant_id = p_tenant
                      AND s.course_id = p_course
                      AND s.generation = p_generation
                      AND s.stage = p_stage
                      AND s.attempt_id = sna.predecessor_attempt_id
               )
               OR EXISTS (
                   SELECT 1
                     FROM public.course_retention_purge_attempt s
                    WHERE s.tenant_id = p_tenant
                      AND s.course_id = p_course
                      AND s.generation = p_generation
                      AND s.stage = p_stage
                      AND s.attempt_id = sna.next_attempt_id
               )
           )
        UNION ALL
        SELECT 1 FROM public.question_statistics_contribution_receipt qsr
         WHERE qsr.tenant_id = p_tenant
           AND (
               EXISTS (
                   SELECT 1
                     FROM public.course_retention_purge_run s
                    WHERE s.tenant_id = p_tenant
                      AND s.course_id = p_course
                      AND s.generation = p_generation
                      AND s.stage = p_stage
                      AND s.run_id = qsr.first_completed_run_id
               )
               OR EXISTS (
                   SELECT 1
                     FROM public.course_retention_purge_attempt s
                    WHERE s.tenant_id = p_tenant
                      AND s.course_id = p_course
                      AND s.generation = p_generation
                      AND s.stage = p_stage
                      AND s.attempt_id = qsr.attempt_id
               )
           )
        UNION ALL
        SELECT 1 FROM public.question_prefetch qp
         WHERE qp.tenant_id = p_tenant
           AND (
               EXISTS (
                   SELECT 1
                     FROM public.course_retention_purge_run s
                    WHERE s.tenant_id = p_tenant
                      AND s.course_id = p_course
                      AND s.generation = p_generation
                      AND s.stage = p_stage
                      AND s.run_id = qp.run_id
               )
               OR EXISTS (
                   SELECT 1
                     FROM public.course_retention_purge_attempt s
                    WHERE s.tenant_id = p_tenant
                      AND s.course_id = p_course
                      AND s.generation = p_generation
                      AND s.stage = p_stage
                      AND s.attempt_id = qp.predecessor_attempt_id
               )
           )
        UNION ALL
        SELECT 1 FROM public.external_tool_launch_session e
         WHERE e.tenant_id = p_tenant
           AND e.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.external_tool_exchange e
         WHERE e.tenant_id = p_tenant
           AND e.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.attempt_feedback af
         WHERE af.tenant_id = p_tenant
           AND af.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.submission s
         WHERE s.tenant_id = p_tenant
           AND s.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.grade_event ge
         WHERE ge.tenant_id = p_tenant
           AND ge.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.submission_idempotency si
         WHERE si.tenant_id = p_tenant
           AND si.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.question_attempt qa
         WHERE qa.tenant_id = p_tenant
           AND qa.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.student_assignment_summary sas
         WHERE EXISTS (
            SELECT 1
              FROM public.enrollment e
              JOIN public.assignment a
                ON a.tenant_id = e.tenant_id
               AND a.assignment_id = e.assignment_id
             WHERE e.tenant_id = sas.tenant_id
               AND sas.tenant_id = p_tenant
               AND e.enrollment_id = sas.enrollment_id
               AND a.course_id = p_course
         )
        UNION ALL
        SELECT 1 FROM public.assignment_run ar
         WHERE EXISTS (
           SELECT 1
              FROM public.enrollment e
              JOIN public.assignment a
                ON a.tenant_id = e.tenant_id
               AND a.assignment_id = e.assignment_id
               WHERE e.tenant_id = ar.tenant_id
                 AND ar.tenant_id = p_tenant
                 AND e.enrollment_id = ar.enrollment_id
                 AND a.course_id = p_course
         )
        UNION ALL
        SELECT 1 FROM public.enrollment e
         WHERE EXISTS (
            SELECT 1
              FROM public.assignment a
             WHERE a.tenant_id = e.tenant_id
               AND a.assignment_id = e.assignment_id
               AND a.tenant_id = p_tenant
               AND a.course_id = p_course
         )
        UNION ALL
        SELECT 1 FROM public.audit_event ae
         WHERE ae.tenant_id = p_tenant
           AND ae.delivery_scope = 'student_record'
           AND ae.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.asset_delivery ad
         WHERE ad.tenant_id = p_tenant
           AND ad.delivery_kind = 'student_record'
           AND ad.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.student_export_artifact sae
         WHERE EXISTS (
             SELECT 1
               FROM public.course_retention_purge_export s
              WHERE s.tenant_id = p_tenant
                AND s.course_id = p_course
                AND s.generation = p_generation
                AND s.stage = p_stage
                AND s.export_id = sae.export_id
         )
        UNION ALL
        SELECT 1 FROM public.student_export_request ser
         WHERE ser.tenant_id = p_tenant
           AND ser.course_id = p_course
        UNION ALL
        SELECT 1 FROM public.worker_job w
         WHERE w.tenant_id = p_tenant
           AND EXISTS (
               SELECT 1
                 FROM public.course_retention_purge_export s
                WHERE s.tenant_id = p_tenant
                  AND s.course_id = p_course
                  AND s.generation = p_generation
                  AND s.stage = p_stage
                  AND s.job_id = w.job_id
           )
        UNION ALL
        SELECT 1 FROM public.course_member cm
         WHERE cm.tenant_id = p_tenant
           AND cm.course_id = p_course
           AND cm.role = 'student'
        UNION ALL
        SELECT 1 FROM public.assignment_problem ap
         WHERE frozen_assignment_disposition = 'delete'
           AND EXISTS (
               SELECT 1
                 FROM public.assignment a
                WHERE a.tenant_id = ap.tenant_id
                  AND a.assignment_id = ap.assignment_id
                  AND a.course_id = p_course
           )
        UNION ALL
        SELECT 1 FROM public.assignment a
         WHERE frozen_assignment_disposition = 'delete'
           AND a.tenant_id = p_tenant
           AND a.course_id = p_course
    ) THEN
        RAISE EXCEPTION 'delete retention commit left residual learner rows';
    END IF;

    -- The work sets contain educational-record identities, so successful
    -- deletion erases them before the durable coarse tombstone is written.
    DELETE FROM public.course_retention_purge_export
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND generation = p_generation
       AND stage = p_stage;
    DELETE FROM public.course_retention_purge_attempt
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND generation = p_generation
       AND stage = p_stage;
    DELETE FROM public.course_retention_purge_run
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND generation = p_generation
       AND stage = p_stage;

    UPDATE public.course_retention_cleanup_manifest
       SET state = 'completed',
           completed_at = transaction_timestamp()
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND generation = p_generation
       AND stage = p_stage
       AND job_id = p_job
       AND state = 'prepared';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to finalize delete manifest';
    END IF;

    UPDATE public.course_retention_stage
       SET state = 'completed'
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND stage = p_stage
       AND generation = p_generation
       AND state = 'started'
       AND job_id = p_job
       AND lease_token = p_token;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to complete delete retention stage';
    END IF;

    UPDATE public.course_retention
       SET lifecycle = 'deleted'
     WHERE tenant_id = p_tenant
       AND course_id = p_course
       AND generation = p_generation
       AND lifecycle = 'archived';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to mark retention lifecycle deleted';
    END IF;

    UPDATE public.worker_job
       SET state = 'completed',
           lease_token = NULL,
           lease_expires_at = NULL,
           completed_at = transaction_timestamp()
     WHERE job_id = p_job
       AND tenant_id = p_tenant
       AND state = 'leased'
       AND lease_token = p_token
       AND payload = jsonb_build_object('kind', 'retention', 'course', p_course::text,
                                       'stage', p_stage, 'generation', p_generation);
    IF NOT FOUND THEN
        RAISE EXCEPTION 'failed to mark delete worker job complete';
    END IF;

    RETURN true;
END $$;

CREATE OR REPLACE FUNCTION ple_prepare_retention_work(p_tenant uuid, p_job uuid, p_token uuid,
                                           p_course uuid, p_stage text, p_generation bigint)
RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public AS $$
BEGIN
    IF p_stage = 'deleteStudentRecords' THEN
        RETURN r44b_prepare_retention_work(p_tenant, p_job, p_token, p_course, p_stage, p_generation);
    END IF;
    RETURN r44a_prepare_retention_work(p_tenant, p_job, p_token, p_course, p_stage, p_generation);
END $$;

CREATE OR REPLACE FUNCTION ple_commit_retention_work(p_tenant uuid, p_job uuid, p_token uuid,
                                          p_course uuid, p_stage text, p_generation bigint)
RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public AS $$
BEGIN
    IF p_stage = 'deleteStudentRecords' THEN
        RETURN r44b_commit_retention_work(p_tenant, p_job, p_token, p_course, p_stage, p_generation);
    END IF;
    RETURN r44a_commit_retention_work(p_tenant, p_job, p_token, p_course, p_stage, p_generation);
END $$;

ALTER FUNCTION r44a_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;
ALTER FUNCTION r44a_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;
ALTER FUNCTION r44b_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;
ALTER FUNCTION r44b_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;
ALTER FUNCTION ple_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;
ALTER FUNCTION ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;

REVOKE ALL ON FUNCTION r44a_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint),
    r44a_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint),
    r44b_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint),
    r44b_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    FROM PUBLIC, ple_app;
REVOKE ALL ON FUNCTION ple_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint),
    ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    FROM PUBLIC, ple_app;

GRANT EXECUTE ON FUNCTION ple_prepare_retention_work(uuid, uuid, uuid, uuid, text, bigint),
    ple_commit_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    TO ple_app;

-- 6) Broker privileges for the permanent purge path.
GRANT SELECT, DELETE ON
    feedback_release,
    submission_next_attempt,
    question_statistics_contribution_receipt,
    question_prefetch,
    submission_receipt_snapshot,
    attempt_feedback,
    submission,
    grade_event,
    submission_idempotency,
    question_attempt,
    student_assignment_summary,
    assignment_run,
    enrollment,
    audit_event,
    asset_delivery,
    student_export_request,
    student_export_artifact,
    worker_job,
    external_tool_launch_session,
    external_tool_exchange,
    course_member,
    assignment,
    assignment_problem
    TO ple_retention_broker;

DROP POLICY IF EXISTS retention_broker_feedback_release_tenant_select ON feedback_release;
CREATE POLICY retention_broker_feedback_release_tenant_select
    ON feedback_release FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_feedback_release_tenant_delete ON feedback_release;
CREATE POLICY retention_broker_feedback_release_tenant_delete
    ON feedback_release FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_submission_next_attempt_tenant_select ON submission_next_attempt;
CREATE POLICY retention_broker_submission_next_attempt_tenant_select
    ON submission_next_attempt FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_submission_next_attempt_tenant_delete ON submission_next_attempt;
CREATE POLICY retention_broker_submission_next_attempt_tenant_delete
    ON submission_next_attempt FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS ret_broker_stats_receipt_sel ON question_statistics_contribution_receipt;
CREATE POLICY ret_broker_stats_receipt_sel
    ON question_statistics_contribution_receipt FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS ret_broker_stats_receipt_del ON question_statistics_contribution_receipt;
CREATE POLICY ret_broker_stats_receipt_del
    ON question_statistics_contribution_receipt FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_question_prefetch_tenant_select ON question_prefetch;
CREATE POLICY retention_broker_question_prefetch_tenant_select
    ON question_prefetch FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_question_prefetch_tenant_delete ON question_prefetch;
CREATE POLICY retention_broker_question_prefetch_tenant_delete
    ON question_prefetch FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_submission_receipt_snapshot_tenant_select ON submission_receipt_snapshot;
CREATE POLICY retention_broker_submission_receipt_snapshot_tenant_select
    ON submission_receipt_snapshot FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_submission_receipt_snapshot_tenant_delete ON submission_receipt_snapshot;
CREATE POLICY retention_broker_submission_receipt_snapshot_tenant_delete
    ON submission_receipt_snapshot FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_attempt_feedback_tenant_select ON attempt_feedback;
CREATE POLICY retention_broker_attempt_feedback_tenant_select
    ON attempt_feedback FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_attempt_feedback_tenant_delete ON attempt_feedback;
CREATE POLICY retention_broker_attempt_feedback_tenant_delete
    ON attempt_feedback FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_submission_tenant_select ON submission;
CREATE POLICY retention_broker_submission_tenant_select
    ON submission FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_submission_tenant_delete ON submission;
CREATE POLICY retention_broker_submission_tenant_delete
    ON submission FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_grade_event_tenant_select ON grade_event;
CREATE POLICY retention_broker_grade_event_tenant_select
    ON grade_event FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_grade_event_tenant_delete ON grade_event;
CREATE POLICY retention_broker_grade_event_tenant_delete
    ON grade_event FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_submission_idempotency_tenant_select ON submission_idempotency;
CREATE POLICY retention_broker_submission_idempotency_tenant_select
    ON submission_idempotency FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_submission_idempotency_tenant_delete ON submission_idempotency;
CREATE POLICY retention_broker_submission_idempotency_tenant_delete
    ON submission_idempotency FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_question_attempt_tenant_select ON question_attempt;
CREATE POLICY retention_broker_question_attempt_tenant_select
    ON question_attempt FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_question_attempt_tenant_delete ON question_attempt;
CREATE POLICY retention_broker_question_attempt_tenant_delete
    ON question_attempt FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_student_assignment_summary_tenant_select ON student_assignment_summary;
CREATE POLICY retention_broker_student_assignment_summary_tenant_select
    ON student_assignment_summary FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_student_assignment_summary_tenant_delete ON student_assignment_summary;
CREATE POLICY retention_broker_student_assignment_summary_tenant_delete
    ON student_assignment_summary FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_assignment_run_tenant_select ON assignment_run;
CREATE POLICY retention_broker_assignment_run_tenant_select
    ON assignment_run FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_assignment_run_tenant_delete ON assignment_run;
CREATE POLICY retention_broker_assignment_run_tenant_delete
    ON assignment_run FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_enrollment_tenant_select ON enrollment;
CREATE POLICY retention_broker_enrollment_tenant_select
    ON enrollment FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_enrollment_tenant_delete ON enrollment;
CREATE POLICY retention_broker_enrollment_tenant_delete
    ON enrollment FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_audit_event_tenant_select ON audit_event;
CREATE POLICY retention_broker_audit_event_tenant_select
    ON audit_event FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant() AND delivery_scope = 'student_record');
DROP POLICY IF EXISTS retention_broker_audit_event_tenant_delete ON audit_event;
CREATE POLICY retention_broker_audit_event_tenant_delete
    ON audit_event FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant() AND delivery_scope = 'student_record');

DROP POLICY IF EXISTS retention_broker_asset_delivery_tenant_select ON asset_delivery;
CREATE POLICY retention_broker_asset_delivery_tenant_select
    ON asset_delivery FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant() AND delivery_kind = 'student_record');
DROP POLICY IF EXISTS retention_broker_asset_delivery_tenant_delete ON asset_delivery;
CREATE POLICY retention_broker_asset_delivery_tenant_delete
    ON asset_delivery FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant() AND delivery_kind = 'student_record');

DROP POLICY IF EXISTS retention_broker_export_request_tenant_select ON student_export_request;
CREATE POLICY retention_broker_export_request_tenant_select
    ON student_export_request FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_export_request_tenant_delete ON student_export_request;
CREATE POLICY retention_broker_export_request_tenant_delete
    ON student_export_request FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_export_artifact_tenant_select ON student_export_artifact;
CREATE POLICY retention_broker_export_artifact_tenant_select
    ON student_export_artifact FOR SELECT TO ple_retention_broker
    USING (
        EXISTS (
            SELECT 1
              FROM public.student_export_request r
             WHERE r.export_id = student_export_artifact.export_id
               AND r.tenant_id = ple_current_tenant()
        )
    );
DROP POLICY IF EXISTS retention_broker_export_artifact_tenant_delete ON student_export_artifact;
CREATE POLICY retention_broker_export_artifact_tenant_delete
    ON student_export_artifact FOR DELETE TO ple_retention_broker
    USING (
        EXISTS (
            SELECT 1
              FROM public.student_export_request r
             WHERE r.export_id = student_export_artifact.export_id
               AND r.tenant_id = ple_current_tenant()
        )
    );

DROP POLICY IF EXISTS retention_broker_worker_job_tenant_select ON worker_job;
CREATE POLICY retention_broker_worker_job_tenant_select
    ON worker_job FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_worker_job_tenant_delete ON worker_job;
CREATE POLICY retention_broker_worker_job_tenant_delete
    ON worker_job FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_external_tool_exchange_tenant_select ON external_tool_exchange;
CREATE POLICY retention_broker_external_tool_exchange_tenant_select
    ON external_tool_exchange FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_external_tool_exchange_tenant_delete ON external_tool_exchange;
CREATE POLICY retention_broker_external_tool_exchange_tenant_delete
    ON external_tool_exchange FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_external_tool_launch_session_tenant_select ON external_tool_launch_session;
CREATE POLICY retention_broker_external_tool_launch_session_tenant_select
    ON external_tool_launch_session FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_external_tool_launch_session_tenant_delete ON external_tool_launch_session;
CREATE POLICY retention_broker_external_tool_launch_session_tenant_delete
    ON external_tool_launch_session FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_course_member_tenant_select ON course_member;
CREATE POLICY retention_broker_course_member_tenant_select
    ON course_member FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_course_member_tenant_delete ON course_member;
CREATE POLICY retention_broker_course_member_tenant_delete
    ON course_member FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_assignment_tenant_select ON assignment;
CREATE POLICY retention_broker_assignment_tenant_select
    ON assignment FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_assignment_tenant_delete ON assignment;
CREATE POLICY retention_broker_assignment_tenant_delete
    ON assignment FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());

DROP POLICY IF EXISTS retention_broker_assignment_problem_tenant_select ON assignment_problem;
CREATE POLICY retention_broker_assignment_problem_tenant_select
    ON assignment_problem FOR SELECT TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
DROP POLICY IF EXISTS retention_broker_assignment_problem_tenant_delete ON assignment_problem;
CREATE POLICY retention_broker_assignment_problem_tenant_delete
    ON assignment_problem FOR DELETE TO ple_retention_broker
    USING (tenant_id = ple_current_tenant());
