-- SD1 forced RLS policies for course records, private authoring, and worker leases.

SET LOCAL ROLE ple_api_owner;
GRANT USAGE ON SCHEMA ple_api TO ple_data_owner, ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE POLICY private_lookup_api_owner ON ple_private.account FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY workspace_lookup_api_owner ON ple_private.authoring_workspace FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_observer_lookup_api_owner ON ple_private.course_observer_grant FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY student_observer_lookup_api_owner ON ple_private.student_observer_grant FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY support_lookup_api_owner ON ple_private.sysadmin_support_capability FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY worker_job_lookup_api_owner ON ple_private.worker_job FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY workspace_owner_access ON ple_private.authoring_workspace
    FOR ALL TO ple_app
    USING (ple_api.current_actor_owns_workspace(workspace_id))
    WITH CHECK (
        owner_user_id = ple_api.current_actor_user_id()
        AND revoked_at IS NULL
    );
CREATE POLICY workspace_collaborator_owner_access ON ple_private.authoring_workspace_collaborator
    FOR ALL TO ple_app
    USING (ple_api.current_actor_owns_workspace(workspace_id))
    WITH CHECK (ple_api.current_actor_owns_workspace(workspace_id));
CREATE POLICY workspace_draft_owner_access ON ple_private.workspace_draft_question
    FOR ALL TO ple_app
    USING (ple_api.current_actor_can_access_workspace(workspace_id))
    WITH CHECK (ple_api.current_actor_can_access_workspace(workspace_id));
CREATE POLICY workspace_flat_question_source_access ON ple_private.workspace_flat_question_source
    FOR ALL TO ple_app
    USING (ple_api.current_actor_can_access_workspace(workspace_id))
    WITH CHECK (ple_api.current_actor_can_access_workspace(workspace_id));
CREATE POLICY workspace_flat_question_grading_write_access ON ple_private.workspace_flat_question_grading
    FOR INSERT TO ple_app
    WITH CHECK (ple_api.current_actor_can_access_workspace(workspace_id));
CREATE POLICY workspace_flat_question_grading_update_access ON ple_private.workspace_flat_question_grading
    FOR UPDATE TO ple_app
    USING (ple_api.current_actor_can_access_workspace(workspace_id))
    WITH CHECK (ple_api.current_actor_can_access_workspace(workspace_id));
CREATE POLICY workspace_flat_question_grading_delete_access ON ple_private.workspace_flat_question_grading
    FOR DELETE TO ple_app
    USING (ple_api.current_actor_can_access_workspace(workspace_id));
CREATE POLICY published_flat_question_grading_reader_access
    ON ple_private.published_flat_question_grading
    FOR SELECT TO ple_grader
    USING (true);
CREATE POLICY published_qti_question_grading_reader_access
    ON ple_private.published_qti_question_grading
    FOR SELECT TO ple_grader
    USING (true);
CREATE POLICY workspace_qti_import_access ON ple_private.workspace_qti_import
    FOR ALL TO ple_app
    USING (ple_api.current_actor_can_access_workspace(workspace_id))
    WITH CHECK (ple_api.current_actor_can_access_workspace(workspace_id));
CREATE POLICY workspace_qti_import_grading_write_access ON ple_private.workspace_qti_import_grading
    FOR INSERT TO ple_app
    WITH CHECK (ple_api.current_actor_can_access_workspace(workspace_id));
CREATE POLICY workspace_qti_import_grading_update_access ON ple_private.workspace_qti_import_grading
    FOR UPDATE TO ple_app
    USING (ple_api.current_actor_can_access_workspace(workspace_id))
    WITH CHECK (ple_api.current_actor_can_access_workspace(workspace_id));
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
CREATE POLICY membership_lookup_api_owner ON ple_data.course_membership FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY student_lookup_api_owner ON ple_data.course_student FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY enrollment_lookup_api_owner ON ple_data.assignment_enrollment FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY course_instance_member_read ON ple_data.course_instance
    FOR SELECT TO ple_app
    USING (
        ple_api.current_actor_is_course_instructor(course_id)
        OR EXISTS (
            SELECT 1 FROM ple_data.course_membership AS membership
            WHERE membership.course_id = course_instance.course_id
              AND membership.user_id = ple_api.current_actor_user_id()
              AND membership.revoked_at IS NULL
        )
        OR ple_api.current_actor_has_course_observer_grant(course_id)
    );
CREATE POLICY course_membership_instructor_or_self_read ON ple_data.course_membership
    FOR SELECT TO ple_app
    USING (
        ple_api.current_actor_is_course_instructor(course_id)
        OR user_id = ple_api.current_actor_user_id()
    );
CREATE POLICY course_student_instructor_or_self_read ON ple_data.course_student
    FOR SELECT TO ple_app
    USING (
        ple_api.current_actor_is_course_instructor(course_id)
        OR ple_api.current_actor_is_course_student(course_id, student_id)
        OR ple_api.current_actor_has_student_observer_grant(course_id, student_id)
    );
CREATE POLICY assignment_enrollment_authorized_read ON ple_data.assignment_enrollment
    FOR SELECT TO ple_app
    USING (
        ple_api.current_actor_is_course_instructor(course_id)
        OR ple_api.current_actor_is_course_student(course_id, student_id)
        OR ple_api.current_actor_has_student_observer_grant(course_id, student_id)
        OR ple_api.current_actor_has_course_observer_grant(course_id)
    );
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.current_worker_has_job_lease(p_job_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    WITH configured AS (
        SELECT pg_catalog.current_setting('ple.worker_job_id', true) AS raw_job_id,
               pg_catalog.current_setting('ple.worker_lease_token', true) AS raw_lease_token
    )
    SELECT EXISTS (
        SELECT 1
        FROM configured
        JOIN ple_private.worker_job AS job
          ON configured.raw_job_id ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
         AND configured.raw_lease_token ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
         AND job.job_id = configured.raw_job_id::uuid
         AND job.lease_token = configured.raw_lease_token::uuid
        WHERE job.job_id = p_job_id
          AND job.state = 'leased'
          AND job.lease_expires_at > pg_catalog.clock_timestamp()
    )
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_worker_has_job_lease(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.current_worker_has_job_lease(uuid) TO ple_worker;
CREATE FUNCTION ple_api.qti_import_is_committed(p_workspace_id uuid, p_import_id uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM ple_private.workspace_qti_import
        WHERE workspace_id = p_workspace_id
          AND import_id = p_import_id
          AND state = 'committed'
    )
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.qti_import_is_committed(uuid, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.qti_import_is_committed(uuid, uuid) TO ple_grader;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE POLICY worker_job_current_lease_access ON ple_private.worker_job
    FOR SELECT TO ple_worker
    USING (ple_api.current_worker_has_job_lease(job_id));
CREATE POLICY workspace_qti_job_enqueue_access ON ple_private.worker_job
    FOR INSERT TO ple_app
    WITH CHECK (
        target_kind = 'qti_import'
        AND ple_api.current_actor_can_access_workspace(workspace_id)
    );
CREATE POLICY workspace_qti_job_view_access ON ple_private.worker_job
    FOR SELECT TO ple_app
    USING (
        target_kind = 'qti_import'
        AND ple_api.current_actor_can_access_workspace(workspace_id)
    );
CREATE POLICY workspace_qti_job_delete_access ON ple_private.worker_job
    FOR DELETE TO ple_app
    USING (
        target_kind = 'qti_import'
        AND ple_api.current_actor_owns_workspace(workspace_id)
    );
CREATE POLICY workspace_qti_import_worker_commit_access ON ple_private.workspace_qti_import
    FOR UPDATE TO ple_worker
    USING (
        state = 'prepared'
        AND EXISTS (
            SELECT 1 FROM ple_private.worker_job AS job
            WHERE job.job_id = pg_catalog.current_setting('ple.worker_job_id', true)::uuid
              AND job.handler_kind = 'qti_import'
              AND job.target_kind = 'qti_import'
              AND job.workspace_id = workspace_qti_import.workspace_id
              AND job.import_id = workspace_qti_import.import_id
              AND ple_api.current_worker_has_job_lease(job.job_id)
        )
    )
    WITH CHECK (state = 'committed');
CREATE POLICY workspace_qti_import_grading_committed_reader_access
    ON ple_private.workspace_qti_import_grading
    FOR SELECT TO ple_grader
    USING (ple_api.qti_import_is_committed(workspace_id, import_id));
CREATE POLICY worker_job_qti_completion_access ON ple_private.worker_job
    FOR UPDATE TO ple_worker
    USING (
        handler_kind = 'qti_import'
        AND target_kind = 'qti_import'
        AND ple_api.current_worker_has_job_lease(job_id)
    )
    WITH CHECK (state = 'completed' AND lease_token IS NULL AND lease_expires_at IS NULL);
RESET ROLE;
