-- SD1 exact CourseInstance/Student delivery indexes and answer-free read projection.

SET LOCAL ROLE ple_data_owner;
CREATE INDEX course_membership_event_current_lookup_idx
    ON ple_data.course_membership_event (membership_id, occurred_at DESC, course_membership_event_id DESC);
CREATE INDEX course_membership_account_course_idx ON ple_data.course_membership (account_id, course_id);
CREATE INDEX student_record_account_course_idx ON ple_data.student_record (student_account_id, course_id);
CREATE INDEX assignment_revision_availability_idx
    ON ple_data.assignment_revision (course_id, available_at);
CREATE INDEX assignment_released_revision_lookup_idx
    ON ple_data.assignment (released_assignment_revision_id)
    WHERE assignment_status = 'released';
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;
GRANT SELECT ON TABLE ple_data.published_question, ple_data.question_revision TO ple_api_owner;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
CREATE VIEW ple_api.published_question_summary AS
SELECT versions.question_id, versions.revision_number, versions.backend, versions.published_at, versions.public_metadata
  FROM ple_data.published_question AS questions
  JOIN ple_data.question_revision AS versions ON versions.question_id = questions.question_id;
REVOKE ALL PRIVILEGES ON TABLE ple_api.published_question_summary FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app;
GRANT SELECT ON TABLE ple_api.published_question_summary TO ple_app;
RESET ROLE;
