-- Row security policies from assessments.sql.

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.assessment ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.assessment FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.assessment_entry ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.assessment_entry FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_data.assessment_question_pool_fork ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_data.assessment_question_pool_fork FORCE ROW LEVEL SECURITY;

CREATE POLICY assessment_data_owner_access ON ple_data.assessment
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY assessment_entry_data_owner_access ON ple_data.assessment_entry
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY assessment_question_pool_fork_data_owner_access ON ple_data.assessment_question_pool_fork
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

CREATE POLICY assessment_private_owner_lookup ON ple_data.assessment
    FOR SELECT TO ple_private_owner USING (true);


-- Student Work starts, saves, and finalizes lock their Assessment root first.
-- PostgreSQL requires UPDATE on one selected column for SELECT FOR UPDATE;
-- this grants no general Assessment mutation capability.
CREATE POLICY assessment_private_owner_student_work_root_lock ON ple_data.assessment
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY assessment_entry_private_owner_lookup ON ple_data.assessment_entry
    FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY assessment_question_pool_fork_private_owner_lookup ON ple_data.assessment_question_pool_fork
    FOR SELECT TO ple_private_owner USING (true);

CREATE POLICY assessment_api_owner_read ON ple_data.assessment
    FOR SELECT TO ple_api_owner
    USING (ple_api.current_session_account_is_course_instructor(course_instance_id));


-- Course-to-Blueprint publication takes the same aggregate-root lock as an
-- Assessment Save. PostgreSQL requires UPDATE on one selected column for
-- SELECT FOR UPDATE; this policy grants no general Assessment mutation path.
CREATE POLICY assessment_api_owner_blueprint_publication_root_lock
    ON ple_data.assessment FOR UPDATE TO ple_api_owner
    USING (ple_api.current_session_account_is_course_instructor(course_instance_id))
    WITH CHECK (ple_api.current_session_account_is_course_instructor(course_instance_id));

CREATE POLICY assessment_entry_api_owner_read ON ple_data.assessment_entry
    FOR SELECT TO ple_api_owner
    USING (EXISTS (
        SELECT 1 FROM ple_data.assessment
         WHERE assessment_id = assessment_entry.assessment_id
           AND ple_api.current_session_account_is_course_instructor(course_instance_id)
    ));

CREATE POLICY assessment_question_pool_fork_api_owner_read ON ple_data.assessment_question_pool_fork
    FOR SELECT TO ple_api_owner
    USING (EXISTS (
        SELECT 1 FROM ple_data.assessment
         WHERE assessment_id = assessment_question_pool_fork.assessment_id
           AND ple_api.current_session_account_is_course_instructor(course_instance_id)
    ));

