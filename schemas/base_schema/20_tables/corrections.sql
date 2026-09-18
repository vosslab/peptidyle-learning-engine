-- corrections tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.forced_question_correction (
    forced_question_correction_id uuid PRIMARY KEY,
    flawed_question_id text NOT NULL,
    flawed_revision_number integer NOT NULL CHECK (flawed_revision_number > 0),
    replacement_question_id text NOT NULL,
    replacement_revision_number integer NOT NULL CHECK (replacement_revision_number > 0),
    approved_by_account_id uuid NOT NULL,
    approver_role ple_data.product_role NOT NULL DEFAULT 'sysadmin',
    approved_at timestamptz NOT NULL,
    correction_generation integer NOT NULL CHECK (correction_generation > 0),
    reason ple_data.correction_reason NOT NULL,
    CHECK ((flawed_question_id, flawed_revision_number)
        <> (replacement_question_id, replacement_revision_number)),
    FOREIGN KEY (flawed_question_id, flawed_revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    FOREIGN KEY (replacement_question_id, replacement_revision_number)
        REFERENCES ple_data.question_revision(published_question_id, revision_number),
    FOREIGN KEY (approved_by_account_id, approver_role)
        REFERENCES ple_private.account(account_id, product_role),
    UNIQUE (flawed_question_id, flawed_revision_number, correction_generation),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);





-- This retained event name is intentionally restricted to Forced Question
-- Correction evidence; the proposal event variants and their foreign keys are
-- absent from the baseline.
CREATE TABLE ple_data.question_change_event (
    question_change_event_id uuid PRIMARY KEY,
    forced_question_correction_id uuid NOT NULL UNIQUE
        REFERENCES ple_data.forced_question_correction(forced_question_correction_id),
    recorded_by_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    occurred_at timestamptz NOT NULL,
    evidence jsonb NOT NULL CHECK (jsonb_typeof(evidence) = 'object')
);

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.forced_question_correction_assessment_attempt_target (
    forced_question_correction_id uuid NOT NULL REFERENCES ple_data.forced_question_correction(forced_question_correction_id),
    assessment_attempt_id uuid NOT NULL
        REFERENCES ple_private.assessment_attempt(assessment_attempt_id) ON DELETE CASCADE,
    PRIMARY KEY (forced_question_correction_id, assessment_attempt_id),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


CREATE TABLE ple_audit.forced_question_correction_issued_question_target (
    forced_question_correction_id uuid NOT NULL REFERENCES ple_data.forced_question_correction(forced_question_correction_id),
    issued_question_id uuid NOT NULL
        REFERENCES ple_private.issued_question(issued_question_id) ON DELETE CASCADE,
    PRIMARY KEY (forced_question_correction_id, issued_question_id),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.forced_question_correction IS 'role: current state, Immutable Sysadmin-approved critical correction from one exact Question Revision to another.';

COMMENT ON TABLE ple_data.question_change_event IS 'role: event, Immutable Forced Question Correction event; Question Change Proposal events are not persisted.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.forced_question_correction_assessment_attempt_target IS 'role: event, deleted by none for the correction record; targets follow Unrelease. HUMAN_GUIDANCE.md Forced Question corrections.';

COMMENT ON TABLE ple_audit.forced_question_correction_issued_question_target IS 'role: event, deleted by none for the correction record; targets follow Unrelease. HUMAN_GUIDANCE.md Forced Question corrections.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.forced_question_correction IS 'role: current state, Immutable Sysadmin-approved critical correction from one exact Question Revision to another.';

COMMENT ON TABLE ple_data.question_change_event IS 'role: event, Immutable Forced Question Correction event; Question Change Proposal events are not persisted.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.forced_question_correction_assessment_attempt_target IS 'role: event, deleted by none for the correction record; targets follow Unrelease. HUMAN_GUIDANCE.md Forced Question corrections.';

COMMENT ON TABLE ple_audit.forced_question_correction_issued_question_target IS 'role: event, deleted by none for the correction record; targets follow Unrelease. HUMAN_GUIDANCE.md Forced Question corrections.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.forced_question_correction IS 'role: current state, Immutable Sysadmin-approved critical correction from one exact Question Revision to another.';

COMMENT ON TABLE ple_data.question_change_event IS 'role: event, Immutable Forced Question Correction event; Question Change Proposal events are not persisted.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.forced_question_correction_assessment_attempt_target IS 'role: event, deleted by none for the correction record; targets follow Unrelease. HUMAN_GUIDANCE.md Forced Question corrections.';

COMMENT ON TABLE ple_audit.forced_question_correction_issued_question_target IS 'role: event, deleted by none for the correction record; targets follow Unrelease. HUMAN_GUIDANCE.md Forced Question corrections.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.forced_question_correction IS 'role: current state, Immutable Sysadmin-approved critical correction from one exact Question Revision to another.';

COMMENT ON TABLE ple_data.question_change_event IS 'role: event, Immutable Forced Question Correction event; Question Change Proposal events are not persisted.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.forced_question_correction_assessment_attempt_target IS 'role: event, deleted by none for the correction record; targets follow Unrelease. HUMAN_GUIDANCE.md Forced Question corrections.';

COMMENT ON TABLE ple_audit.forced_question_correction_issued_question_target IS 'role: event, deleted by none for the correction record; targets follow Unrelease. HUMAN_GUIDANCE.md Forced Question corrections.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.forced_question_correction IS 'role: current state, Immutable Sysadmin-approved critical correction from one exact Question Revision to another.';

COMMENT ON TABLE ple_data.question_change_event IS 'role: event, Immutable Forced Question Correction event; Question Change Proposal events are not persisted.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.forced_question_correction_assessment_attempt_target IS 'role: event, deleted by none for the correction record; targets follow Unrelease. HUMAN_GUIDANCE.md Forced Question corrections.';

COMMENT ON TABLE ple_audit.forced_question_correction_issued_question_target IS 'role: event, deleted by none for the correction record; targets follow Unrelease. HUMAN_GUIDANCE.md Forced Question corrections.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.forced_question_correction IS 'role: current state, Immutable Sysadmin-approved critical correction from one exact Question Revision to another.';

COMMENT ON TABLE ple_data.question_change_event IS 'role: event, Immutable Forced Question Correction event; Question Change Proposal events are not persisted.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.forced_question_correction_assessment_attempt_target IS 'role: event, deleted by none for the correction record; targets follow Unrelease. HUMAN_GUIDANCE.md Forced Question corrections.';

COMMENT ON TABLE ple_audit.forced_question_correction_issued_question_target IS 'role: event, deleted by none for the correction record; targets follow Unrelease. HUMAN_GUIDANCE.md Forced Question corrections.';

