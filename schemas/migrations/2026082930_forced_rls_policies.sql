-- SD1 forced RLS policies for course records, private authoring, and worker leases.

SET LOCAL ROLE ple_api_owner;
GRANT USAGE ON SCHEMA ple_api TO ple_data_owner, ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE POLICY private_lookup_api_owner ON ple_private.account FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY instructor_approval_event_lookup_api_owner ON ple_private.instructor_approval_event
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_invitation_event_lookup_api_owner ON ple_private.course_invitation_event
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY workspace_lookup_api_owner ON ple_private.authoring_workspace FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_observer_relationship_event_lookup_api_owner ON ple_private.course_observer_relationship_event FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY support_lookup_api_owner ON ple_private.sysadmin_support_capability FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY worker_job_lookup_api_owner ON ple_private.worker_job FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY workspace_owner_access ON ple_private.authoring_workspace
    FOR ALL TO ple_app
    USING (ple_api.current_session_account_owns_workspace(workspace_id))
    WITH CHECK (
        owner_account_id = ple_api.current_session_account_id()
        AND revoked_at IS NULL
    );
CREATE POLICY workspace_collaborator_owner_access ON ple_private.authoring_workspace_collaborator_event
    FOR ALL TO ple_app
    USING (ple_api.current_session_account_owns_workspace(workspace_id))
    WITH CHECK (ple_api.current_session_account_owns_workspace(workspace_id));
CREATE POLICY draft_question_workspace_access ON ple_private.draft_question
    FOR ALL TO ple_app
    USING (ple_api.current_session_account_can_access_workspace(workspace_id))
    WITH CHECK (ple_api.current_session_account_can_access_workspace(workspace_id));
CREATE POLICY draft_question_revision_workspace_access ON ple_private.draft_question_revision
    FOR ALL TO ple_app
    USING (EXISTS (
        SELECT 1 FROM ple_private.draft_question AS question
        WHERE question.draft_question_id = draft_question_revision.draft_question_id
          AND ple_api.current_session_account_can_access_workspace(question.workspace_id)
    ))
    WITH CHECK (EXISTS (
        SELECT 1 FROM ple_private.draft_question AS question
        WHERE question.draft_question_id = draft_question_revision.draft_question_id
          AND ple_api.current_session_account_can_access_workspace(question.workspace_id)
    ));
CREATE POLICY draft_question_source_access ON ple_private.draft_question_source
    FOR ALL TO ple_app
    USING (ple_api.current_session_account_can_access_workspace(workspace_id))
    WITH CHECK (ple_api.current_session_account_can_access_workspace(workspace_id));
CREATE POLICY draft_question_grading_material_write_access ON ple_private.draft_question_grading_material
    FOR INSERT TO ple_app
    WITH CHECK (ple_api.current_session_account_can_access_workspace(workspace_id));
CREATE POLICY draft_question_grading_material_update_access ON ple_private.draft_question_grading_material
    FOR UPDATE TO ple_app
    USING (ple_api.current_session_account_can_access_workspace(workspace_id))
    WITH CHECK (ple_api.current_session_account_can_access_workspace(workspace_id));
CREATE POLICY draft_question_grading_material_delete_access ON ple_private.draft_question_grading_material
    FOR DELETE TO ple_app
    USING (ple_api.current_session_account_can_access_workspace(workspace_id));
CREATE POLICY workspace_qti_import_access ON ple_private.workspace_qti_import
    FOR ALL TO ple_app
    USING (ple_api.current_session_account_can_access_workspace(workspace_id))
    WITH CHECK (ple_api.current_session_account_can_access_workspace(workspace_id));
CREATE POLICY workspace_qti_import_grading_write_access ON ple_private.workspace_qti_import_grading
    FOR INSERT TO ple_app
    WITH CHECK (ple_api.current_session_account_can_access_workspace(workspace_id));
CREATE POLICY workspace_qti_import_grading_update_access ON ple_private.workspace_qti_import_grading
    FOR UPDATE TO ple_app
    USING (ple_api.current_session_account_can_access_workspace(workspace_id))
    WITH CHECK (ple_api.current_session_account_can_access_workspace(workspace_id));
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
CREATE POLICY membership_lookup_api_owner ON ple_data.course_membership FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY membership_event_lookup_api_owner ON ple_data.course_membership_event
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY student_record_lookup_api_owner ON ple_data.student_record FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_instance_member_read ON ple_data.course_instance
    FOR SELECT TO ple_app
    USING (
        ple_api.current_session_account_is_course_member(course_id)
        OR ple_api.current_session_account_has_course_observer_relationship(course_id)
    );
CREATE POLICY course_membership_instructor_or_self_read ON ple_data.course_membership
    FOR SELECT TO ple_app
    USING (
        ple_api.current_session_account_is_course_instructor(course_id)
        OR ple_api.current_session_account_owns_course_membership(course_id, membership_id)
    );
CREATE POLICY student_record_instructor_or_self_read ON ple_data.student_record
    FOR SELECT TO ple_app
    USING (
        ple_api.current_session_account_is_course_instructor(course_id)
        OR ple_api.current_session_account_owns_student_record(course_id, student_record_id)
    );
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
RESET ROLE;
