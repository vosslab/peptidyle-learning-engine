-- assessment_template tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.assessment_template (
    assessment_template_id uuid PRIMARY KEY,
    owner_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    assessment_template_edit_number bigint NOT NULL DEFAULT 1
        CHECK (assessment_template_edit_number > 0),
    template_name text NOT NULL CHECK (
        ple_private.assessment_template_name_is_valid(template_name)
    ),
    assessment_type ple_data.assessment_type NOT NULL,
    assessment_policy_snapshot_id ple_data.sha256_digest NOT NULL
        REFERENCES ple_data.assessment_policy_snapshot(assessment_policy_snapshot_id),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.assessment_template IS 'role: current state, deleted by owner delete of the template. HUMAN_GUIDANCE.md Assessment templates.';
