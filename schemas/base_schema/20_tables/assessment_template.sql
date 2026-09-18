-- assessment_template tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.assessment_template (
    assessment_template_id uuid PRIMARY KEY,
    owner_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    assessment_template_edit_number bigint NOT NULL DEFAULT 1
        CHECK (assessment_template_edit_number > 0),
    template_name text NOT NULL CHECK (
        ple_private.assessment_template_name_is_valid(template_name)
    ),
    assessment_type text NOT NULL CHECK (assessment_type IN (
        'regular_assignment', 'practice_question_assignment', 'bonus_assignment', 'quiz', 'exam'
    )),
    instructions text NOT NULL CHECK (
        instructions !~ E'\\x00' AND char_length(instructions) <= 50000
    ),
    assessment_attempt_time_limit_seconds integer CHECK (
        assessment_attempt_time_limit_seconds IS NULL
        OR assessment_attempt_time_limit_seconds BETWEEN 1 AND 43200
    ),
    assessment_attempt_limit integer CHECK (
        assessment_attempt_limit IS NULL OR assessment_attempt_limit > 0
    ),
    late_work_rule text NOT NULL CHECK (late_work_rule IN ('accept', 'mark_late', 'reject')),
    question_variation_rule text NOT NULL CHECK (
        question_variation_rule IN ('reuse_variation', 'new_variation')
    ),
    assessment_question_order_rule text NOT NULL CHECK (
        assessment_question_order_rule IN ('authored_order', 'shuffled')
    ),
    feedback_score text NOT NULL CHECK (
        feedback_score IN ('during_attempt', 'after_submit', 'after_due', 'after_close', 'never')
    ),
    feedback_per_item_correctness text NOT NULL CHECK (
        feedback_per_item_correctness IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )
    ),
    feedback_submitted_response text NOT NULL CHECK (
        feedback_submitted_response IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )
    ),
    feedback_question_answer text NOT NULL CHECK (
        feedback_question_answer IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )
    ),
    feedback_question_answer_explanation text NOT NULL CHECK (
        feedback_question_answer_explanation IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )
    ),
    feedback_class_statistics text NOT NULL CHECK (
        feedback_class_statistics IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )
    ),
    -- ASVS 2.2.1-2.2.3: mandatory variant fields cannot pass CHECK as unknown.
    CHECK (
        assessment_type NOT IN ('quiz', 'exam')
        OR (assessment_attempt_limit IS NOT NULL AND assessment_attempt_limit = 1)
    )
);

COMMENT ON TABLE ple_private.assessment_template IS 'role: current state, deleted by owner delete of the template. HUMAN_GUIDANCE.md Assessment templates.';

