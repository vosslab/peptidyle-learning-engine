-- MOD-SCHEMA: shared content, tenant records, RLS, grants, and partitions.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_app') THEN
        CREATE ROLE ple_app NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOBYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_student') THEN
        CREATE ROLE ple_student NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOBYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_grader') THEN
        CREATE ROLE ple_grader NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOBYPASSRLS;
    END IF;
END
$$;

CREATE OR REPLACE FUNCTION ple_current_tenant()
RETURNS uuid
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT NULLIF(current_setting('ple.tenant_id', true), '')::uuid
$$;

REVOKE ALL ON FUNCTION ple_current_tenant() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_current_tenant() TO ple_app, ple_student, ple_grader;

CREATE TABLE problem (
    problem_id uuid PRIMARY KEY,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp()
);

CREATE TABLE problem_version (
    problem_id uuid NOT NULL REFERENCES problem(problem_id),
    version_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    title text NOT NULL,
    lifecycle text NOT NULL DEFAULT 'published' CHECK (lifecycle IN ('published', 'withdrawn')),
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (problem_id, version_id)
);

CREATE INDEX problem_version_catalog_idx
    ON problem_version (title, problem_id, version_id);

CREATE TABLE problem_version_payload (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    PRIMARY KEY (problem_id, version_id),
    FOREIGN KEY (problem_id, version_id)
        REFERENCES problem_version(problem_id, version_id)
) PARTITION BY HASH (problem_id);

DO $$
DECLARE
    remainder integer;
BEGIN
    FOR remainder IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE problem_version_payload_p%s PARTITION OF problem_version_payload '
            'FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            remainder,
            remainder
        );
    END LOOP;
END
$$;

CREATE TABLE answer_key (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    key_payload jsonb NOT NULL,
    key_sha256 character(64) NOT NULL,
    PRIMARY KEY (problem_id, version_id),
    FOREIGN KEY (problem_id, version_id)
        REFERENCES problem_version(problem_id, version_id)
);

CREATE TABLE workspace_draft (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, workspace_id)
);

CREATE TABLE assignment (
    tenant_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, assignment_id)
);

CREATE TABLE assignment_problem (
    tenant_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    position integer NOT NULL CHECK (position >= 0),
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    PRIMARY KEY (tenant_id, assignment_id, position),
    FOREIGN KEY (tenant_id, assignment_id)
        REFERENCES assignment(tenant_id, assignment_id) ON DELETE CASCADE,
    FOREIGN KEY (problem_id, version_id)
        REFERENCES problem_version(problem_id, version_id)
);

CREATE TABLE enrollment (
    tenant_id uuid NOT NULL,
    enrollment_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    student_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    PRIMARY KEY (tenant_id, enrollment_id),
    FOREIGN KEY (tenant_id, assignment_id)
        REFERENCES assignment(tenant_id, assignment_id)
);

CREATE TABLE student_assignment_summary (
    tenant_id uuid NOT NULL,
    enrollment_id uuid NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, enrollment_id),
    FOREIGN KEY (tenant_id, enrollment_id)
        REFERENCES enrollment(tenant_id, enrollment_id) ON DELETE CASCADE
);

CREATE TABLE assignment_run (
    tenant_id uuid NOT NULL,
    run_id uuid NOT NULL,
    enrollment_id uuid NOT NULL,
    run_number bigint NOT NULL CHECK (run_number > 0),
    started_at timestamptz NOT NULL,
    completed_at timestamptz,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    PRIMARY KEY (tenant_id, run_id),
    UNIQUE (tenant_id, enrollment_id, run_number),
    FOREIGN KEY (tenant_id, enrollment_id)
        REFERENCES enrollment(tenant_id, enrollment_id)
);

CREATE UNIQUE INDEX assignment_run_one_active_idx
    ON assignment_run (tenant_id, enrollment_id)
    WHERE completed_at IS NULL;

CREATE TABLE question_attempt (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    run_id uuid NOT NULL,
    occurred_at timestamptz NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    PRIMARY KEY (tenant_id, attempt_id, occurred_at),
    FOREIGN KEY (tenant_id, run_id)
        REFERENCES assignment_run(tenant_id, run_id)
) PARTITION BY RANGE (occurred_at);

CREATE TABLE submission (
    tenant_id uuid NOT NULL,
    submission_id uuid NOT NULL,
    occurred_at timestamptz NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    PRIMARY KEY (tenant_id, submission_id, occurred_at)
) PARTITION BY RANGE (occurred_at);

CREATE TABLE grade_event (
    tenant_id uuid NOT NULL,
    grade_event_id uuid NOT NULL,
    occurred_at timestamptz NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    PRIMARY KEY (tenant_id, grade_event_id, occurred_at)
) PARTITION BY RANGE (occurred_at);

CREATE TABLE audit_event (
    tenant_id uuid NOT NULL,
    audit_event_id uuid NOT NULL,
    occurred_at timestamptz NOT NULL,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    PRIMARY KEY (tenant_id, audit_event_id, occurred_at)
) PARTITION BY RANGE (occurred_at);

DO $$
DECLARE
    parent_name text;
    month_start date;
    month_end date;
    month_offset integer;
BEGIN
    FOREACH parent_name IN ARRAY ARRAY['question_attempt', 'submission', 'grade_event', 'audit_event']
    LOOP
        FOR month_offset IN -1..24 LOOP
            month_start := (date_trunc('month', current_date) + month_offset * interval '1 month')::date;
            month_end := (month_start + interval '1 month')::date;
            EXECUTE format(
                'CREATE TABLE %I PARTITION OF %I FOR VALUES FROM (%L) TO (%L)',
                parent_name || '_' || to_char(month_start, 'YYYY_MM'),
                parent_name,
                month_start,
                month_end
            );
        END LOOP;
        EXECUTE format(
            'CREATE TABLE %I PARTITION OF %I DEFAULT',
            parent_name || '_default',
            parent_name
        );
    END LOOP;
END
$$;

CREATE INDEX question_attempt_lookup_idx
    ON question_attempt (tenant_id, attempt_id);
CREATE INDEX question_attempt_run_idx
    ON question_attempt (tenant_id, run_id, occurred_at);
CREATE INDEX submission_tenant_time_idx
    ON submission (tenant_id, occurred_at);
CREATE INDEX grade_event_tenant_time_idx
    ON grade_event (tenant_id, occurred_at);
CREATE INDEX audit_event_tenant_time_idx
    ON audit_event (tenant_id, occurred_at);

ALTER TABLE workspace_draft ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_draft FORCE ROW LEVEL SECURITY;
CREATE POLICY workspace_draft_tenant ON workspace_draft
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

ALTER TABLE assignment ENABLE ROW LEVEL SECURITY;
ALTER TABLE assignment FORCE ROW LEVEL SECURITY;
CREATE POLICY assignment_tenant ON assignment
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

ALTER TABLE assignment_problem ENABLE ROW LEVEL SECURITY;
ALTER TABLE assignment_problem FORCE ROW LEVEL SECURITY;
CREATE POLICY assignment_problem_tenant ON assignment_problem
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

ALTER TABLE enrollment ENABLE ROW LEVEL SECURITY;
ALTER TABLE enrollment FORCE ROW LEVEL SECURITY;
CREATE POLICY enrollment_tenant ON enrollment
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

ALTER TABLE student_assignment_summary ENABLE ROW LEVEL SECURITY;
ALTER TABLE student_assignment_summary FORCE ROW LEVEL SECURITY;
CREATE POLICY student_assignment_summary_tenant ON student_assignment_summary
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

ALTER TABLE assignment_run ENABLE ROW LEVEL SECURITY;
ALTER TABLE assignment_run FORCE ROW LEVEL SECURITY;
CREATE POLICY assignment_run_tenant ON assignment_run
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

ALTER TABLE question_attempt ENABLE ROW LEVEL SECURITY;
ALTER TABLE question_attempt FORCE ROW LEVEL SECURITY;
CREATE POLICY question_attempt_tenant ON question_attempt
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

ALTER TABLE submission ENABLE ROW LEVEL SECURITY;
ALTER TABLE submission FORCE ROW LEVEL SECURITY;
CREATE POLICY submission_tenant ON submission
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

ALTER TABLE grade_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE grade_event FORCE ROW LEVEL SECURITY;
CREATE POLICY grade_event_tenant ON grade_event
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

ALTER TABLE audit_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_event FORCE ROW LEVEL SECURITY;
CREATE POLICY audit_event_tenant ON audit_event
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

GRANT USAGE ON SCHEMA public TO ple_app, ple_student, ple_grader;
GRANT SELECT, INSERT, UPDATE, DELETE ON
    problem,
    problem_version,
    problem_version_payload,
    workspace_draft,
    assignment,
    assignment_problem,
    enrollment,
    student_assignment_summary,
    assignment_run,
    question_attempt,
    submission,
    grade_event,
    audit_event
TO ple_app;

GRANT SELECT ON
    problem,
    problem_version,
    problem_version_payload,
    workspace_draft,
    assignment,
    assignment_problem,
    enrollment,
    student_assignment_summary,
    assignment_run,
    question_attempt
TO ple_student;

REVOKE ALL ON answer_key FROM PUBLIC, ple_app, ple_student;
GRANT SELECT, INSERT, UPDATE, DELETE ON answer_key TO ple_grader;
