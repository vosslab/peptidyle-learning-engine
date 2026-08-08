-- MOD-RUN-FEEDBACK: private first-grade teaching material.
-- This is deliberately separate from browser-safe attempt and submission JSON.
CREATE TABLE attempt_feedback (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    hint jsonb,
    correct_response jsonb,
    rationale jsonb,
    content_sha256 character(64) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, attempt_id),
    CHECK (hint IS NULL OR jsonb_typeof(hint) = 'array'),
    CHECK (correct_response IS NULL OR jsonb_typeof(correct_response) = 'array'),
    CHECK (rationale IS NULL OR jsonb_typeof(rationale) = 'array')
);

ALTER TABLE attempt_feedback ENABLE ROW LEVEL SECURITY;
ALTER TABLE attempt_feedback FORCE ROW LEVEL SECURITY;
CREATE POLICY attempt_feedback_tenant ON attempt_feedback
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
GRANT SELECT, INSERT ON attempt_feedback TO ple_app;
