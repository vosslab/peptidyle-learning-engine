-- Atomic initial teaching content, materialized by the Store from the exact Blueprint.
SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.load_course_instance_blueprint(p_reference bigint, p_revision bigint)
RETURNS TABLE(content jsonb, content_checksum bytea)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT revision.content, revision.content_checksum
      FROM ple_data.blueprint_course AS blueprint
      JOIN ple_data.blueprint_course_revision AS revision
        ON revision.blueprint_course_reference_number = blueprint.reference_number
       AND revision.blueprint_revision_number = p_revision
     WHERE blueprint.reference_number = p_reference
       AND blueprint.availability = 'available'
       AND (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_is_sysadmin())
$$;

REVOKE ALL ON FUNCTION ple_api.load_course_instance_blueprint(bigint, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.load_course_instance_blueprint(bigint, bigint) TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.initialize_course_assignments(
    p_course_id uuid, p_blueprint_reference bigint, p_blueprint_revision bigint, p_assignments jsonb
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    member jsonb;
    candidate ple_data.assignment%ROWTYPE;
    new_assignment_id uuid;
BEGIN
    IF EXISTS (SELECT 1 FROM ple_data.assignment WHERE course_id = p_course_id)
       OR jsonb_typeof(p_assignments) IS DISTINCT FROM 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course initial assignments are invalid';
    END IF;
    FOR member IN SELECT value FROM jsonb_array_elements(p_assignments) LOOP
        IF jsonb_typeof(member -> 'entries') IS DISTINCT FROM 'array'
           OR jsonb_array_length(member -> 'entries') = 0 THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint Assignment requires its Questions';
        END IF;
        -- Only content/policy columns below are admitted. Identity, provenance,
        -- initial Edit Number, and unreleased state are always database-owned.
        SELECT * INTO candidate FROM jsonb_populate_record(NULL::ple_data.assignment,
            member -> 'values');
        new_assignment_id := gen_random_uuid();
        INSERT INTO ple_data.assignment (
            assignment_id, course_id, source_blueprint_course_reference_number,
            source_blueprint_revision_number, source_blueprint_assignment_reference,
            created_at, updated_at,
            assignment_title,
            assignment_instructions,
            available_at,
            due_at,
            closes_at,
            assignment_attempt_time_limit_seconds,
            attempt_limit,
            late_work_rule,
            assignment_completion_rule,
            assignment_completion_score_threshold,
            assignment_attempt_grade_rule,
            assignment_attempt_continuation_rule,
            max_additional_assignment_attempts,
            question_pool_reuse_rule,
            question_variation_rule,
            assignment_attempt_resume_rule,
            assignment_question_display_rule,
            assignment_navigation_rule,
            assignment_question_order_rule,
            feedback_score,
            feedback_per_item_correctness,
            feedback_submitted_response,
            feedback_question_feedback,
            feedback_question_answer,
            feedback_question_answer_explanation,
            feedback_class_statistics
        ) VALUES (
            new_assignment_id, p_course_id, p_blueprint_reference,
            p_blueprint_revision, (member ->> 'source')::uuid,
            transaction_timestamp(), transaction_timestamp(),
            candidate.assignment_title,
            candidate.assignment_instructions,
            NULL,
            NULL,
            NULL,
            candidate.assignment_attempt_time_limit_seconds,
            candidate.attempt_limit,
            candidate.late_work_rule,
            candidate.assignment_completion_rule,
            candidate.assignment_completion_score_threshold,
            candidate.assignment_attempt_grade_rule,
            candidate.assignment_attempt_continuation_rule,
            candidate.max_additional_assignment_attempts,
            candidate.question_pool_reuse_rule,
            candidate.question_variation_rule,
            candidate.assignment_attempt_resume_rule,
            candidate.assignment_question_display_rule,
            candidate.assignment_navigation_rule,
            candidate.assignment_question_order_rule,
            candidate.feedback_score,
            candidate.feedback_per_item_correctness,
            candidate.feedback_submitted_response,
            candidate.feedback_question_feedback,
            candidate.feedback_question_answer,
            candidate.feedback_question_answer_explanation,
            candidate.feedback_class_statistics
        );
        PERFORM ple_data.replace_assignment_entries(new_assignment_id, member -> 'entries');
    END LOOP;
END
$$;

REVOKE ALL ON FUNCTION ple_data.initialize_course_assignments(uuid, bigint, bigint, jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.initialize_course_assignments(uuid, bigint, bigint, jsonb) TO ple_api_owner;
RESET ROLE;
