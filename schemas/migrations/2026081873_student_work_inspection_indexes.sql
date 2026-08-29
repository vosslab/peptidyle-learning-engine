-- WP-INST-G2 / G2-W3B: recorded index review for the bounded broker lookup.

BEGIN;

-- Existing course-member, submitted-run, retention, and audit-history indexes
-- already cover the broker's equality joins and audit append history.  The
-- one-time disposable-stack EXPLAIN evidence records that no distinct access
-- pattern justified another write-amplifying index at this migration epoch.

COMMIT;
