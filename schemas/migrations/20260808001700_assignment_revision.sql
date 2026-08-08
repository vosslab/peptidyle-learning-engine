-- Optimistic-concurrency token for tenant-owned editable assignments.
-- Existing definitions receive the first valid revision without changing their
-- immutable published-version references or their run policies.
ALTER TABLE assignment
    ADD COLUMN revision bigint NOT NULL DEFAULT 1
        CHECK (revision > 0);
