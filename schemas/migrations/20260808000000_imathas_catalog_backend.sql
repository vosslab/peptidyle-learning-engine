-- MOD-ADP-IMATHAS: permit immutable, server-prepared iMathAS snapshots.
--
-- This is deliberately a forward migration: 20260807000200_catalog.sql is an
-- already-published historical schema. Dropping and recreating the named check
-- preserves every catalog row, all RLS policies, and the existing constraints.
-- `IF EXISTS` also keeps this statement safe to reapply during recovery.
ALTER TABLE problem_version
    DROP CONSTRAINT IF EXISTS problem_version_backend_check;

ALTER TABLE problem_version
    ADD CONSTRAINT problem_version_backend_check
        CHECK (backend IN ('native', 'webwork', 'qti', 'h5p', 'imathas'));
