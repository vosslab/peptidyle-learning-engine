-- SD1 course and item analysis with thresholded immutable evidence.

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.course_assignment_analysis (
    analysis_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    assignment_id uuid NOT NULL,
    scoring_generation integer NOT NULL CHECK (scoring_generation > 0),
    analyzed_at timestamp with time zone NOT NULL,
    completed_run_count integer NOT NULL CHECK (completed_run_count >= 0),
    in_progress_run_count integer NOT NULL CHECK (in_progress_run_count >= 0),
    minimum_cohort_size integer NOT NULL CHECK (minimum_cohort_size >= 5),
    aggregate jsonb NOT NULL CHECK (jsonb_typeof(aggregate) = 'object'),
    CONSTRAINT course_assignment_analysis_generation_is_unique
        UNIQUE (course_id, assignment_id, scoring_generation)
);
CREATE TABLE ple_data.assignment_item_analysis (
    item_analysis_id uuid PRIMARY KEY,
    analysis_id uuid NOT NULL REFERENCES ple_data.course_assignment_analysis (analysis_id) ON DELETE CASCADE,
    question_id text NOT NULL,
    version_number integer NOT NULL,
    graded_attempt_count integer NOT NULL CHECK (graded_attempt_count >= 0),
    aggregate jsonb NOT NULL CHECK (jsonb_typeof(aggregate) = 'object'),
    CONSTRAINT assignment_item_analysis_version_is_unique UNIQUE (analysis_id, question_id, version_number)
);
GRANT USAGE ON SCHEMA ple_data TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_data.course_assignment_analysis TO ple_audit_owner;
ALTER TABLE ple_data.course_assignment_analysis ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_assignment_analysis FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_item_analysis ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_item_analysis FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.course_assignment_analysis, ple_data.assignment_item_analysis FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.course_analysis_evidence (
    evidence_id uuid PRIMARY KEY,
    analysis_id uuid NOT NULL REFERENCES ple_data.course_assignment_analysis (analysis_id),
    recorded_at timestamp with time zone NOT NULL,
    evidence jsonb NOT NULL CHECK (jsonb_typeof(evidence) = 'object'),
    digest bytea NOT NULL CHECK (pg_catalog.octet_length(digest) = 32),
    CONSTRAINT course_analysis_evidence_is_unique UNIQUE (analysis_id, digest)
);
ALTER TABLE ple_audit.course_analysis_evidence ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.course_analysis_evidence FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.course_analysis_evidence FROM PUBLIC;
RESET ROLE;
