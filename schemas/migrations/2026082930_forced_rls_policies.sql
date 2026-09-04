-- Forced RLS policies for Course records, private authoring, and Job leases.

SET LOCAL ROLE ple_api_owner;
GRANT USAGE ON SCHEMA ple_api TO ple_data_owner, ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE POLICY private_lookup_api_owner ON ple_private.account FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY account_state_event_lookup_api_owner ON ple_private.account_state_event
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY account_authenticated_session_private_owner_lookup ON ple_private.account
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY authenticated_session_private_owner_read ON ple_private.authenticated_session
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY authenticated_session_private_owner_create ON ple_private.authenticated_session
    FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY authenticated_session_private_owner_revoke ON ple_private.authenticated_session
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY course_invitation_event_lookup_api_owner ON ple_private.course_invitation_event
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY workspace_lookup_api_owner ON ple_private.authoring_workspace FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_observer_relationship_event_lookup_api_owner ON ple_private.course_observer_relationship_event FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY support_lookup_api_owner ON ple_private.sysadmin_support_capability FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY job_lookup_api_owner ON ple_private.job FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY authoring_workspace_owner_access ON ple_private.authoring_workspace
    FOR ALL TO ple_app
    USING (ple_api.current_session_account_is_authoring_workspace_owner(workspace_id))
    WITH CHECK (
        authoring_workspace_owner_account_id = ple_api.current_session_account_id()
        AND revoked_at IS NULL
    );
CREATE POLICY authoring_workspace_collaborator_event_owner_access
    ON ple_private.authoring_workspace_collaborator_event
    FOR ALL TO ple_app
    USING (ple_api.current_session_account_is_authoring_workspace_owner(workspace_id))
    WITH CHECK (ple_api.current_session_account_is_authoring_workspace_owner(workspace_id));
CREATE POLICY draft_question_workspace_access ON ple_private.draft_question
    FOR ALL TO ple_app
    USING (ple_api.current_session_account_can_access_authoring_workspace(workspace_id))
    WITH CHECK (ple_api.current_session_account_can_access_authoring_workspace(workspace_id));
CREATE POLICY draft_question_source_binding_workspace_access
    ON ple_private.draft_question_source_binding
    FOR ALL TO ple_app
    USING (EXISTS (
        SELECT 1
          FROM ple_private.draft_question AS question
         WHERE question.draft_question_uuid = draft_question_source_binding.draft_question_uuid
           AND ple_api.current_session_account_can_access_authoring_workspace(question.workspace_id)
    ))
    WITH CHECK (EXISTS (
        SELECT 1
          FROM ple_private.draft_question AS question
         WHERE question.draft_question_uuid = draft_question_source_binding.draft_question_uuid
           AND ple_api.current_session_account_can_access_authoring_workspace(question.workspace_id)
    ));
CREATE POLICY workspace_import_access ON ple_private.workspace_import
    FOR ALL TO ple_app
    USING (ple_api.current_session_account_can_access_authoring_workspace(workspace_id))
    WITH CHECK (ple_api.current_session_account_can_access_authoring_workspace(workspace_id));
CREATE POLICY workspace_import_item_result_access ON ple_private.workspace_import_item_result
    FOR ALL TO ple_app
    USING (ple_api.current_session_account_can_access_authoring_workspace(workspace_id))
    WITH CHECK (ple_api.current_session_account_can_access_authoring_workspace(workspace_id));
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
