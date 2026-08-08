-- MOD-RUN-FEEDBACK: immutable first-submission receipt snapshots.
-- A replay must return the run/summary state observed when its original
-- response committed, rather than current mutable enrollment projections.
-- Existing submission rows deliberately receive no synthetic backfill: no
-- historical bytes are available to reconstruct truthfully.
CREATE TABLE submission_receipt_snapshot (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    run_payload jsonb NOT NULL CHECK (jsonb_typeof(run_payload) = 'object'),
    run_payload_sha256 character(64) NOT NULL,
    summary_payload jsonb NOT NULL CHECK (jsonb_typeof(summary_payload) = 'object'),
    summary_payload_sha256 character(64) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, attempt_id),
    FOREIGN KEY (tenant_id, attempt_id)
        REFERENCES submission_idempotency(tenant_id, attempt_id)
        ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
);

ALTER TABLE submission_receipt_snapshot ENABLE ROW LEVEL SECURITY;
ALTER TABLE submission_receipt_snapshot FORCE ROW LEVEL SECURITY;
CREATE POLICY submission_receipt_snapshot_tenant ON submission_receipt_snapshot
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
GRANT SELECT, INSERT ON submission_receipt_snapshot TO ple_app;
