-- Privileges from content_classification.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.content_discipline, ple_data.content_subject,
    ple_data.content_subject_discipline, ple_data.content_topic, ple_data.content_subtopic
    FROM PUBLIC, ple_app, ple_auth, ple_student, ple_api_owner;

