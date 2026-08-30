-- SD1 private calculated-grade snapshots and immutable control evidence.

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.gradebook_snapshot (
    snapshot_id uuid PRIMARY KEY,
    enrollment_id uuid NOT NULL REFERENCES ple_data.assignment_enrollment (enrollment_id),
    calculated_at timestamp with time zone NOT NULL,
    grade jsonb NOT NULL CHECK (jsonb_typeof(grade) = 'object'),
    generation integer NOT NULL CHECK (generation > 0),
    CONSTRAINT gradebook_snapshot_generation_is_unique UNIQUE (enrollment_id, generation)
);
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.gradebook_snapshot TO ple_audit_owner;
ALTER TABLE ple_private.gradebook_snapshot ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.gradebook_snapshot FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.gradebook_snapshot FROM PUBLIC;
RESET ROLE;
SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.grade_control_event (
    event_id uuid PRIMARY KEY,
    snapshot_id uuid NOT NULL REFERENCES ple_private.gradebook_snapshot (snapshot_id),
    event_kind text NOT NULL CHECK (event_kind IN ('calculated', 'recalculated', 'released')),
    occurred_at timestamp with time zone NOT NULL,
    evidence jsonb NOT NULL CHECK (jsonb_typeof(evidence) = 'object')
);
ALTER TABLE ple_audit.grade_control_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.grade_control_event FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.grade_control_event FROM PUBLIC;
RESET ROLE;
