-- Student Record and Student Course Membership roots.

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.student_record (
    student_record_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    student_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT student_record_account_course_is_unique UNIQUE (course_id, student_account_id),
    CONSTRAINT student_record_course_reference_is_unique UNIQUE (student_record_id, course_id)
);
-- ASVS 2.3.1, 8.2.2, and 8.3.1: one protected enrollment transaction binds
-- each Student membership to the stable record for its exact Account and Course.
ALTER TABLE ple_data.course_membership
    ADD COLUMN student_record_id uuid REFERENCES ple_data.student_record (student_record_id),
    ADD CONSTRAINT course_membership_student_record_presence CHECK (
        (role = 'student' AND student_record_id IS NOT NULL)
        OR (role = 'instructor' AND student_record_id IS NULL)
    );
CREATE FUNCTION ple_data.assert_student_membership_record()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NEW.role = 'student' AND NOT EXISTS (
        SELECT 1
          FROM ple_data.student_record AS student_record
         WHERE student_record.student_record_id = NEW.student_record_id
           AND student_record.course_id = NEW.course_id
           AND student_record.student_account_id = NEW.account_id
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = '23514',
            MESSAGE = 'Student Course Membership must bind its Account and Course to the exact Student Record';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER course_membership_binds_exact_student_record
BEFORE INSERT OR UPDATE OF course_id, account_id, role, student_record_id
ON ple_data.course_membership
FOR EACH ROW EXECUTE FUNCTION ple_data.assert_student_membership_record();
ALTER TABLE ple_data.student_record ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.student_record FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.student_record FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.assert_student_membership_record() FROM PUBLIC;
COMMENT ON TABLE ple_data.student_record IS 'Course-scoped educational record stable for one Student Account and Course Instance across membership episodes.';
COMMENT ON COLUMN ple_data.course_membership.student_record_id IS 'Exact stable Student Record bound to this active or historical Student Course Membership; Instructor memberships have no Student Record.';
RESET ROLE;
