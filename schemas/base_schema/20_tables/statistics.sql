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
    -- Anonymous lifetime lower bounds. They survive Student Work purge and
    -- are deliberately never summed across revisions or purge epochs.
    issued_contributor_floor bigint NOT NULL DEFAULT 0 CHECK (issued_contributor_floor >= 0),
    blank_contributor_floor bigint NOT NULL DEFAULT 0 CHECK (blank_contributor_floor >= 0),
    answered_contributor_floor bigint NOT NULL DEFAULT 0 CHECK (answered_contributor_floor >= 0),
    correct_contributor_floor bigint NOT NULL DEFAULT 0 CHECK (correct_contributor_floor >= 0),
    partial_contributor_floor bigint NOT NULL DEFAULT 0 CHECK (partial_contributor_floor >= 0),
    incorrect_contributor_floor bigint NOT NULL DEFAULT 0 CHECK (incorrect_contributor_floor >= 0),
    credit_sum numeric NOT NULL DEFAULT 0 CHECK (credit_sum >= 0),
    credit_sum_sq numeric NOT NULL DEFAULT 0 CHECK (credit_sum_sq >= 0),
    created_on date NOT NULL DEFAULT CURRENT_DATE,
    updated_on date NOT NULL DEFAULT CURRENT_DATE,
    PRIMARY KEY (published_question_id, revision_number),
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    CHECK (updated_on >= created_on),
    CHECK (answered_count = correct_count + partial_count + incorrect_count),
    CHECK (issued_contributor_floor <= issued_count
        AND blank_contributor_floor <= blank_count
        AND answered_contributor_floor <= answered_count
        AND correct_contributor_floor <= correct_count
        AND partial_contributor_floor <= partial_count
        AND incorrect_contributor_floor <= incorrect_count),
    CHECK (credit_sum >= correct_count
        AND credit_sum <= correct_count + partial_count),
    CHECK (credit_sum_sq >= correct_count
        AND credit_sum_sq <= correct_count + partial_count)
);

CREATE TABLE ple_data.question_pool_statistics (
    question_pool_id ple_data.question_family_id PRIMARY KEY
        REFERENCES ple_data.question_pool(question_pool_id),
    issued_count bigint NOT NULL DEFAULT 0 CHECK (issued_count >= 0),
    answered_count bigint NOT NULL DEFAULT 0 CHECK (answered_count >= 0),
    correct_count bigint NOT NULL DEFAULT 0 CHECK (correct_count >= 0),
    partial_count bigint NOT NULL DEFAULT 0 CHECK (partial_count >= 0),
    incorrect_count bigint NOT NULL DEFAULT 0 CHECK (incorrect_count >= 0),
    answered_contributor_floor bigint NOT NULL DEFAULT 0 CHECK (answered_contributor_floor >= 0),
    correct_contributor_floor bigint NOT NULL DEFAULT 0 CHECK (correct_contributor_floor >= 0),
    partial_contributor_floor bigint NOT NULL DEFAULT 0 CHECK (partial_contributor_floor >= 0),
    incorrect_contributor_floor bigint NOT NULL DEFAULT 0 CHECK (incorrect_contributor_floor >= 0),
    credit_sum numeric NOT NULL DEFAULT 0 CHECK (credit_sum >= 0),
    credit_sum_sq numeric NOT NULL DEFAULT 0 CHECK (credit_sum_sq >= 0),
    issued_contributor_floor bigint NOT NULL DEFAULT 0 CHECK (issued_contributor_floor >= 0),
    created_on date NOT NULL DEFAULT CURRENT_DATE,
    updated_on date NOT NULL DEFAULT CURRENT_DATE,
    CHECK (updated_on >= created_on),
    CHECK (issued_contributor_floor <= issued_count),
    CHECK (answered_contributor_floor <= answered_count
        AND correct_contributor_floor <= correct_count
        AND partial_contributor_floor <= partial_count
        AND incorrect_contributor_floor <= incorrect_count),
    CHECK (answered_count = correct_count + partial_count + incorrect_count),
    CHECK (credit_sum >= correct_count
        AND credit_sum <= correct_count + partial_count),
    CHECK (credit_sum_sq >= correct_count
        AND credit_sum_sq <= correct_count + partial_count)
);

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.question_statistics_observation_receipt (
    course_instance_id ple_data.course_instance_id NOT NULL,
    issued_question_id uuid NOT NULL,
    assessment_submission_id uuid,
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    question_pool_id ple_data.question_family_id,
    normalized_credit numeric CHECK (
        normalized_credit IS NULL OR normalized_credit BETWEEN 0 AND 1
    ),
    observed_at timestamptz NOT NULL,
    PRIMARY KEY (course_instance_id, issued_question_id),
    FOREIGN KEY (course_instance_id, issued_question_id)
        REFERENCES ple_private.issued_question(course_instance_id, issued_question_id)
        ON DELETE CASCADE,
    FOREIGN KEY (course_instance_id, assessment_submission_id)
        REFERENCES ple_private.assessment_submission(course_instance_id, assessment_submission_id)
        ON DELETE CASCADE,
    FOREIGN KEY (question_pool_id)
        REFERENCES ple_data.question_pool(question_pool_id),
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number)
);
COMMENT ON COLUMN ple_private.question_statistics_observation_receipt.assessment_submission_id IS 'NULL means the committed delivery has not yet been submitted.';
COMMENT ON COLUMN ple_private.question_statistics_observation_receipt.question_pool_id IS 'NULL means this Question was delivered directly rather than from a Pool.';


SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.question_revision_statistics IS 'role: aggregate, authored identity-free issued, blank, answered, outcome, and credit-sum counts for one immutable Question Revision, retained after Course Student-record deletion.';
COMMENT ON TABLE ple_data.question_pool_statistics IS 'role: aggregate, authored identity-free issued_count for one Question Pool, retained after Course Student-record deletion.';

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.question_statistics_observation_receipt IS 'role: event, Private exact-once Issued Question observation gate, purged with underlying Student Work while anonymous aggregate counts remain.';
