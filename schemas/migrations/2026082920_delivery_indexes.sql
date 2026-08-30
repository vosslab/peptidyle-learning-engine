-- SD1 exact CourseInstance/Student delivery indexes and answer-free read projection.

SET LOCAL ROLE ple_data_owner;
CREATE INDEX course_membership_current_account_idx ON ple_data.course_membership (account_id, course_id) WHERE revoked_at IS NULL;
CREATE INDEX assignment_enrollment_current_student_idx ON ple_data.assignment_enrollment (student_id, assignment_id) WHERE revoked_at IS NULL;
CREATE INDEX course_delivery_released_idx ON ple_data.course_instance_assignment_delivery (course_id, available_at) WHERE released_at IS NOT NULL;
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;
GRANT SELECT ON TABLE ple_data.published_question, ple_data.published_question_version TO ple_api_owner;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
CREATE VIEW ple_api.published_catalog_summary AS
SELECT questions.question_id, versions.problem_id, versions.version_id, versions.backend, versions.published_at, versions.public_metadata
  FROM ple_data.published_question AS questions
  JOIN ple_data.published_question_version AS versions ON versions.question_id = questions.question_id;
REVOKE ALL PRIVILEGES ON TABLE ple_api.published_catalog_summary FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_api TO ple_app;
GRANT SELECT ON TABLE ple_api.published_catalog_summary TO ple_app;
RESET ROLE;
