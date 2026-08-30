-- SD1 Student educational identity and assignment-enrollment roots.

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.course_student (
    student_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    membership_id uuid NOT NULL UNIQUE REFERENCES ple_data.course_membership (membership_id),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT course_student_student_course_is_unique UNIQUE (student_id, course_id)
);
CREATE TABLE ple_data.assignment_enrollment (
    enrollment_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    student_id uuid NOT NULL REFERENCES ple_data.course_student (student_id),
    assignment_id uuid NOT NULL,
    enrolled_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    CONSTRAINT assignment_enrollment_is_unique UNIQUE (student_id, assignment_id),
    CONSTRAINT assignment_enrollment_revocation_is_ordered CHECK (revoked_at IS NULL OR revoked_at >= enrolled_at)
);
ALTER TABLE ple_data.course_student ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_student FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_enrollment ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.assignment_enrollment FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.course_student, ple_data.assignment_enrollment FROM PUBLIC;
COMMENT ON TABLE ple_data.course_student IS 'Course-scoped Student educational identity; distinct from global account and retained across membership history.';
RESET ROLE;
