-- statistics tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

-- Privacy-safe global statistics for immutable Question Revisions.  The
-- aggregate key deliberately contains only the immutable Question Revision;
-- it has no Student, Account, Course, Attempt, response, or grade identity.
-- Private exact-once receipts are rooted in Student Work and disappear with it.
-- Course retention and Unrelease leave anonymous totals unchanged; subsequent
-- submissions increment those totals without reconstructing private evidence.
CREATE TABLE ple_data.question_revision_statistics (
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    issued_count bigint NOT NULL DEFAULT 0 CHECK (issued_count >= 0),
    blank_count bigint NOT NULL DEFAULT 0 CHECK (blank_count >= 0),
    answered_count bigint NOT NULL DEFAULT 0 CHECK (answered_count >= 0),
    correct_count bigint NOT NULL DEFAULT 0 CHECK (correct_count >= 0),
    partial_count bigint NOT NULL DEFAULT 0 CHECK (partial_count >= 0),
    incorrect_count bigint NOT NULL DEFAULT 0 CHECK (incorrect_count >= 0),
    credit_sum numeric NOT NULL DEFAULT 0 CHECK (credit_sum >= 0),
    credit_sum_sq numeric NOT NULL DEFAULT 0 CHECK (credit_sum_sq >= 0),
    created_on date NOT NULL DEFAULT CURRENT_DATE,
    updated_on date NOT NULL DEFAULT CURRENT_DATE,
    PRIMARY KEY (published_question_id, revision_number),
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    CHECK (updated_on >= created_on),
    CHECK (issued_count = blank_count + answered_count),
    CHECK (answered_count = correct_count + partial_count + incorrect_count),
    CHECK (credit_sum >= correct_count
        AND credit_sum <= correct_count + partial_count),
    CHECK (credit_sum_sq >= correct_count
        AND credit_sum_sq <= correct_count + partial_count)
);

CREATE TABLE ple_data.question_pool_statistics (
    question_pool_id ple_data.question_family_id PRIMARY KEY
        REFERENCES ple_data.question_pool(question_pool_id),
    issued_count bigint NOT NULL DEFAULT 0 CHECK (issued_count >= 0),
    created_on date NOT NULL DEFAULT CURRENT_DATE,
    updated_on date NOT NULL DEFAULT CURRENT_DATE,
    CHECK (updated_on >= created_on)
);

CREATE TABLE ple_data.question_pool_member_statistics (
    question_pool_id ple_data.question_family_id NOT NULL
        REFERENCES ple_data.question_pool(question_pool_id),
    published_question_id ple_data.question_family_id NOT NULL
        REFERENCES ple_data.published_question(published_question_id),
    selected_count bigint NOT NULL DEFAULT 0 CHECK (selected_count >= 0),
    created_on date NOT NULL DEFAULT CURRENT_DATE,
    updated_on date NOT NULL DEFAULT CURRENT_DATE,
    PRIMARY KEY (question_pool_id, published_question_id),
    CHECK (updated_on >= created_on)
);


SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.question_statistics_observation_receipt (
    course_instance_id ple_data.course_instance_id NOT NULL,
    issued_question_id uuid NOT NULL,
    assessment_submission_id uuid NOT NULL,
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    observed_at timestamptz NOT NULL,
    PRIMARY KEY (course_instance_id, issued_question_id),
    UNIQUE (course_instance_id, issued_question_id, assessment_submission_id),
    FOREIGN KEY (course_instance_id, issued_question_id)
        REFERENCES ple_private.issued_question(course_instance_id, issued_question_id)
        ON DELETE CASCADE,
    FOREIGN KEY (course_instance_id, assessment_submission_id)
        REFERENCES ple_private.assessment_submission(course_instance_id, assessment_submission_id)
        ON DELETE CASCADE,
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number)
);


SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.question_revision_statistics IS 'role: aggregate, authored identity-free issued, blank, answered, outcome, and credit-sum counts for one immutable Question Revision, retained after Course Student-record deletion.';
COMMENT ON TABLE ple_data.question_pool_statistics IS 'role: aggregate, authored identity-free issued_count for one Question Pool, retained after Course Student-record deletion.';
COMMENT ON TABLE ple_data.question_pool_member_statistics IS 'role: aggregate, authored identity-free selected_count for one Pool member Published Question, removed with the Pool.';

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.question_statistics_observation_receipt IS 'role: event, Private exact-once Issued Question observation gate, purged with underlying Student Work while anonymous aggregate counts remain.';
