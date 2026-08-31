-- SD1 private calculated-grade snapshots and immutable control evidence.

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.assignment_grade_calculation (
    assignment_grade_calculation_id uuid PRIMARY KEY,
    student_record_id uuid NOT NULL REFERENCES ple_data.student_record (student_record_id),
    assignment_id uuid NOT NULL REFERENCES ple_data.assignment (assignment_id),
    calculated_at timestamp with time zone NOT NULL,
    grade jsonb NOT NULL CHECK (jsonb_typeof(grade) = 'object'),
    generation integer NOT NULL CHECK (generation > 0),
    CONSTRAINT assignment_grade_calculation_generation_is_unique UNIQUE (student_record_id, assignment_id, generation),
    CONSTRAINT assignment_grade_calculation_parent_is_unique
        UNIQUE (student_record_id, assignment_id, assignment_grade_calculation_id)
);
CREATE FUNCTION ple_private.reject_assignment_grade_calculation_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Assignment Grade Calculation is immutable';
END
$$;
CREATE TRIGGER assignment_grade_calculation_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.assignment_grade_calculation
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_assignment_grade_calculation_change();
CREATE TABLE ple_private.assignment_grade (
    assignment_grade_id uuid PRIMARY KEY,
    student_record_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    assignment_grade_calculation_id uuid NOT NULL,
    selected_at timestamp with time zone NOT NULL,
    CONSTRAINT assignment_grade_selected_calculation_matches
        FOREIGN KEY (student_record_id, assignment_id, assignment_grade_calculation_id)
        REFERENCES ple_private.assignment_grade_calculation
            (student_record_id, assignment_id, assignment_grade_calculation_id),
    CONSTRAINT assignment_grade_student_assignment_is_unique UNIQUE (student_record_id, assignment_id)
);
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.assignment_grade_calculation TO ple_audit_owner;
ALTER TABLE ple_private.assignment_grade_calculation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_grade_calculation FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_grade ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.assignment_grade FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.assignment_grade_calculation, ple_private.assignment_grade FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_assignment_grade_calculation_change() FROM PUBLIC;
RESET ROLE;
SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.assignment_grade_event (
    event_id uuid PRIMARY KEY,
    assignment_grade_calculation_id uuid NOT NULL REFERENCES ple_private.assignment_grade_calculation (assignment_grade_calculation_id),
    event_kind text NOT NULL CHECK (event_kind IN ('calculated', 'recalculated', 'released')),
    occurred_at timestamp with time zone NOT NULL,
    evidence jsonb NOT NULL CHECK (jsonb_typeof(evidence) = 'object')
);
CREATE FUNCTION ple_audit.reject_assignment_grade_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Assignment Grade Event is immutable';
END
$$;
CREATE TRIGGER assignment_grade_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.assignment_grade_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_assignment_grade_event_change();
ALTER TABLE ple_audit.assignment_grade_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.assignment_grade_event FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.assignment_grade_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_audit.reject_assignment_grade_event_change() FROM PUBLIC;
RESET ROLE;
