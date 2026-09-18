-- Privileges from assessment_operations.sql.

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_course_assessments(text),
    ple_api.list_assessments_due_soon(),
    ple_api.list_assessment_question_picker(text),
    ple_api.load_assessment_workspace_rows(text, text),
    ple_api.validate_assessment_release(text, text),
    ple_api.create_assessment(text, text, text, text, text),
    ple_api.save_assessment(text, text, bigint, jsonb, jsonb),
    ple_api.save_assessment_inline(text, text, bigint, text, timestamptz),
    ple_api.save_assessment_policies(text, text, bigint, jsonb),
    ple_api.release_assessment(text, text, bigint)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_course_assessments(text),
    ple_api.list_assessments_due_soon(),
    ple_api.list_assessment_question_picker(text),
    ple_api.load_assessment_workspace_rows(text, text),
    ple_api.validate_assessment_release(text, text),
    ple_api.create_assessment(text, text, text, text, text),
    ple_api.save_assessment(text, text, bigint, jsonb, jsonb),
    ple_api.save_assessment_inline(text, text, bigint, text, timestamptz),
    ple_api.save_assessment_policies(text, text, bigint, jsonb),
    ple_api.release_assessment(text, text, bigint)
    TO ple_app;

