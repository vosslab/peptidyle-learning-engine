-- MOD-RUN-FEEDBACK: immutable instructor-controlled on-release disclosure.
-- The row stores only the release decision; private teaching content remains
-- exclusively in attempt_feedback and original submission receipts stay fixed.
CREATE TABLE feedback_release (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    released_by uuid NOT NULL,
    released_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, attempt_id),
    FOREIGN KEY (tenant_id, attempt_id)
        REFERENCES attempt_feedback(tenant_id, attempt_id)
        ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
);

CREATE INDEX feedback_release_tenant_released_at_idx
    ON feedback_release (tenant_id, released_at, attempt_id);

ALTER TABLE feedback_release ENABLE ROW LEVEL SECURITY;
ALTER TABLE feedback_release FORCE ROW LEVEL SECURITY;
CREATE POLICY feedback_release_tenant ON feedback_release
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
GRANT SELECT, INSERT ON feedback_release TO ple_app;
