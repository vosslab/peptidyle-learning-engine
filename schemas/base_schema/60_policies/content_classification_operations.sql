-- Row security policies from content_classification_operations.sql.

SET LOCAL ROLE ple_data_owner;

CREATE POLICY content_discipline_private_command_access ON ple_data.content_discipline
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY content_subject_private_command_access ON ple_data.content_subject
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY content_subject_discipline_private_command_access ON ple_data.content_subject_discipline
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY content_topic_private_command_access ON ple_data.content_topic
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY content_subtopic_private_command_access ON ple_data.content_subtopic
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

