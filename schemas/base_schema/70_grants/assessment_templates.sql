-- Privileges from assessment_templates.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.assessment_template FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.assessment_template_name_is_valid(text),
    ple_private.validate_assessment_template_settings(jsonb) FROM PUBLIC;

GRANT SELECT, INSERT ON TABLE ple_private.assessment_template TO ple_api_owner;

GRANT UPDATE (
    assessment_template_edit_number, template_name, assessment_type,
    assessment_policy_snapshot_id
) ON TABLE ple_private.assessment_template TO ple_api_owner;

GRANT EXECUTE ON FUNCTION ple_private.assessment_template_name_is_valid(text),
    ple_private.validate_assessment_template_settings(jsonb)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.list_assessment_templates(),
    ple_api.read_assessment_template(uuid),
    ple_api.create_assessment_template(uuid, text, text, jsonb),
    ple_api.save_assessment_template(uuid, bigint, text, text, jsonb)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_assessment_templates(),
    ple_api.read_assessment_template(uuid),
    ple_api.create_assessment_template(uuid, text, text, jsonb),
    ple_api.save_assessment_template(uuid, bigint, text, text, jsonb)
    TO ple_app;

