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
    assessment_type ple_data.assessment_type NOT NULL,
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
    late_work_rule ple_data.late_work_rule NOT NULL,
    question_variation_rule ple_data.question_variation_rule NOT NULL,
    assessment_question_order_rule ple_data.question_order_rule NOT NULL,
    feedback_score ple_data.feedback_release NOT NULL,
    feedback_per_item_correctness ple_data.feedback_release NOT NULL,
    feedback_submitted_response ple_data.feedback_release NOT NULL,
    feedback_question_answer ple_data.feedback_release NOT NULL,
    feedback_question_answer_explanation ple_data.feedback_release NOT NULL,
    feedback_class_statistics ple_data.feedback_release NOT NULL,
    -- ASVS 2.2.1-2.2.3: mandatory variant fields cannot pass CHECK as unknown.
    CHECK (
        assessment_type NOT IN ('quiz', 'exam')
        OR (assessment_attempt_limit IS NOT NULL AND assessment_attempt_limit = 1)
    ),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);



SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.assessment_template IS 'role: current state, deleted by owner delete of the template. HUMAN_GUIDANCE.md Assessment templates.';



COMMENT ON TABLE ple_private.assessment_template IS 'role: current state, deleted by owner delete of the template. HUMAN_GUIDANCE.md Assessment templates.';




COMMENT ON TABLE ple_private.assessment_template IS 'role: current state, deleted by owner delete of the template. HUMAN_GUIDANCE.md Assessment templates.';



COMMENT ON TABLE ple_private.assessment_template IS 'role: current state, deleted by owner delete of the template. HUMAN_GUIDANCE.md Assessment templates.';

