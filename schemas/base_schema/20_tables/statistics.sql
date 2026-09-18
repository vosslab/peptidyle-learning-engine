-- statistics tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

-- Privacy-safe global statistics for immutable Question Revisions.  The
-- aggregate key deliberately contains only the immutable Question Revision;
-- it has no Student, Account, Course, Attempt, response, or grade identity.
-- Private exact-once receipts are rooted in Student Work and disappear with it.
-- Course retention and Unrelease leave anonymous totals unchanged; subsequent
-- accepted grades increment those totals without reconstructing private evidence.
CREATE TABLE ple_data.question_revision_statistics (
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    accepted_graded_attempt_count bigint NOT NULL DEFAULT 0
        CHECK (accepted_graded_attempt_count >= 0),
    correct_count bigint NOT NULL DEFAULT 0
        CHECK (correct_count BETWEEN 0 AND accepted_graded_attempt_count),
    created_on date NOT NULL DEFAULT CURRENT_DATE,
    updated_on date NOT NULL DEFAULT CURRENT_DATE,
    CHECK (updated_on >= created_on),
    PRIMARY KEY (published_question_id, revision_number),
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number)
);

CREATE TABLE ple_data.question_revision_choice_statistics (
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    choice_id text NOT NULL CHECK (
        choice_id = btrim(choice_id) AND char_length(choice_id) BETWEEN 1 AND 256
    ),
    selected_count bigint NOT NULL CHECK (selected_count >= 0),
    PRIMARY KEY (published_question_id, revision_number, choice_id),
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision_statistics(published_question_id, revision_number),
    created_on date NOT NULL DEFAULT CURRENT_DATE,
    updated_on date NOT NULL DEFAULT CURRENT_DATE,
    CHECK (updated_on >= created_on)
);


SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.question_statistics_observation_receipt (
    automated_grading_receipt_id uuid PRIMARY KEY
        REFERENCES ple_audit.automated_grading_receipt(automated_grading_receipt_id)
        ON DELETE CASCADE,
    question_attempt_id uuid NOT NULL UNIQUE
        REFERENCES ple_private.question_attempt(question_attempt_id) ON DELETE CASCADE,
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    correct boolean NOT NULL,
    observed_at timestamptz NOT NULL,
    FOREIGN KEY (published_question_id, revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number)
);

CREATE TABLE ple_private.question_statistics_observation_choice (
    question_statistics_observation_receipt_id uuid NOT NULL
        REFERENCES ple_private.question_statistics_observation_receipt(automated_grading_receipt_id)
        ON DELETE CASCADE,
    choice_id text NOT NULL CHECK (
        choice_id = btrim(choice_id) AND char_length(choice_id) BETWEEN 1 AND 256
    ),
    PRIMARY KEY (question_statistics_observation_receipt_id, choice_id),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.question_revision_statistics IS 'role: aggregate, authored identity-free accepted-grade and correct counts for one immutable Question Revision, retained after Course Student-record deletion.';

COMMENT ON TABLE ple_data.question_revision_choice_statistics IS 'role: aggregate, authored identity-free selected eligible-choice counts for one immutable Question Revision, retained after Course Student-record deletion.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.question_statistics_observation_receipt IS 'role: event, Private exact-once accepted-grade observation gate, purged with underlying Student Work while anonymous aggregate counts remain.';

COMMENT ON TABLE ple_private.question_statistics_observation_choice IS 'role: event, Normalized opaque eligible-choice evidence for one statistics observation.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.question_revision_statistics IS 'role: aggregate, authored identity-free accepted-grade and correct counts for one immutable Question Revision, retained after Course Student-record deletion.';

COMMENT ON TABLE ple_data.question_revision_choice_statistics IS 'role: aggregate, authored identity-free selected eligible-choice counts for one immutable Question Revision, retained after Course Student-record deletion.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.question_statistics_observation_receipt IS 'role: event, Private exact-once accepted-grade observation gate, purged with underlying Student Work while anonymous aggregate counts remain.';

COMMENT ON TABLE ple_private.question_statistics_observation_choice IS 'role: event, Normalized opaque eligible-choice evidence for one statistics observation.';

