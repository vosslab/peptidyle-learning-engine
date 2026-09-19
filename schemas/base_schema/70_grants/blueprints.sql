-- Privileges from blueprints.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.course_classification_tags_are_valid(text[]) FROM PUBLIC;


-- Course retention updates are checked against this shared immutable validator.
GRANT EXECUTE ON FUNCTION ple_data.course_classification_tags_are_valid(text[])
    TO ple_api_owner, ple_course_retention_executor;

GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE
    ple_data.blueprint_course,
    ple_data.blueprint_course_revision,
    ple_data.blueprint_revision_question_pin,
    ple_data.blueprint_revision_module,
    ple_data.blueprint_revision_assessment,
    ple_data.blueprint_revision_event,
    ple_data.blueprint_metadata_event,
    ple_data.blueprint_course_create_receipt,
    ple_data.blueprint_course_save_receipt
TO ple_api_owner;



REVOKE ALL PRIVILEGES ON TABLE
    ple_data.blueprint_course,
    ple_data.blueprint_course_revision,
    ple_data.blueprint_revision_question_pin,
    ple_data.blueprint_revision_module,
    ple_data.blueprint_revision_assessment,
    ple_data.blueprint_revision_event,
    ple_data.blueprint_metadata_event,
    ple_data.blueprint_course_create_receipt,
    ple_data.blueprint_course_save_receipt
FROM PUBLIC;

REVOKE ALL PRIVILEGES ON FUNCTION ple_data.blueprint_content_question_pins(jsonb),
    ple_data.blueprint_content_pools(jsonb),
    ple_data.blueprint_content_assessments(jsonb), ple_data.blueprint_content_modules(jsonb),
    ple_data.blueprint_content_has_exact_keys(jsonb, text[]),
    ple_data.blueprint_content_is_closed(jsonb),
    ple_data.validate_blueprint_content(jsonb),
    ple_data.validate_blueprint_question_selection(text, bigint, jsonb) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.blueprint_content_question_pins(jsonb),
    ple_data.blueprint_content_pools(jsonb),
    ple_data.blueprint_content_assessments(jsonb), ple_data.blueprint_content_modules(jsonb),
    ple_data.blueprint_content_has_exact_keys(jsonb, text[]),
    ple_data.blueprint_content_is_closed(jsonb),
    ple_data.validate_blueprint_content(jsonb),
    ple_data.validate_blueprint_question_selection(text, bigint, jsonb)
TO ple_api_owner;

GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE
    ple_data.blueprint_course_fork, ple_data.blueprint_course_fork_receipt
TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_data.reject_blueprint_course_fork_change() FROM PUBLIC;

