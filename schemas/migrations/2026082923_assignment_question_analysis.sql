-- Assignment Analysis and Assignment Question Analysis.

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.assignment_analysis (
    assignment_analysis_id uuid PRIMARY KEY,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    scoring_generation integer NOT NULL CHECK (scoring_generation > 0),
    completed_at timestamp with time zone NOT NULL,
    completed_assignment_attempt_count integer NOT NULL
        CHECK (completed_assignment_attempt_count >= 0),
    in_progress_assignment_attempt_count integer NOT NULL
        CHECK (in_progress_assignment_attempt_count >= 0),
    minimum_cohort_size integer NOT NULL CHECK (minimum_cohort_size >= 5),
    aggregate jsonb NOT NULL CHECK (jsonb_typeof(aggregate) = 'object'),
    CONSTRAINT assignment_analysis_course_assignment_matches
        FOREIGN KEY (course_id, assignment_id)
        REFERENCES ple_data.assignment (course_id, assignment_id),
    CONSTRAINT assignment_analysis_scoring_generation_is_unique
        UNIQUE (course_id, assignment_id, scoring_generation),
    UNIQUE (assignment_analysis_id, course_id, assignment_id)
);
CREATE TABLE ple_data.assignment_question_analysis (
    assignment_question_analysis_id uuid PRIMARY KEY,
    assignment_analysis_id uuid NOT NULL,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    assignment_entry_id uuid NOT NULL,
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    graded_attempt_count integer NOT NULL CHECK (graded_attempt_count >= 0),
    aggregate jsonb NOT NULL CHECK (jsonb_typeof(aggregate) = 'object'),
    CONSTRAINT assignment_question_analysis_parent_matches
        FOREIGN KEY (assignment_analysis_id, course_id, assignment_id)
        REFERENCES ple_data.assignment_analysis (
            assignment_analysis_id, course_id, assignment_id
        ) ON DELETE CASCADE,
    CONSTRAINT assignment_question_analysis_revision_matches
        FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT assignment_question_analysis_source_is_unique
        UNIQUE (assignment_analysis_id, assignment_entry_id, question_id, revision_number)
);
CREATE INDEX assignment_question_analysis_parent_idx
    ON ple_data.assignment_question_analysis (assignment_analysis_id);
GRANT USAGE ON SCHEMA ple_data TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_data.assignment_analysis TO ple_audit_owner;
ALTER TABLE ple_data.assignment_analysis ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_analysis FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_question_analysis ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_question_analysis FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.assignment_analysis,
    ple_data.assignment_question_analysis FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.assignment_analysis_receipt (
    assignment_analysis_receipt_id uuid PRIMARY KEY,
    assignment_analysis_id uuid NOT NULL REFERENCES ple_data.assignment_analysis (assignment_analysis_id),
    completed_at timestamp with time zone NOT NULL,
    receipt jsonb NOT NULL CHECK (jsonb_typeof(receipt) = 'object'),
    assignment_analysis_checksum bytea NOT NULL
        CHECK (pg_catalog.octet_length(assignment_analysis_checksum) = 32),
    CONSTRAINT assignment_analysis_receipt_is_unique
        UNIQUE (assignment_analysis_id, assignment_analysis_checksum)
);
ALTER TABLE ple_audit.assignment_analysis_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.assignment_analysis_receipt FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.assignment_analysis_receipt FROM PUBLIC;
RESET ROLE;
