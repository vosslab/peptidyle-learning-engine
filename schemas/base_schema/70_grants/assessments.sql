-- Privileges from assessments.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.assessment, ple_data.assessment_entry,
    ple_data.assessment_entry_question, ple_data.assessment_entry_pool,
    ple_data.assessment_question_pool_fork, ple_data.assessment_policy_snapshot FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_data.enforce_assessment_edit(),
    ple_data.validate_assessment_question_pool_fork(),
    ple_data.replace_assessment_entries(text, jsonb),
    ple_data.save_assessment(text, text, bigint, jsonb, jsonb),
    ple_data.save_assessment_inline(text, text, bigint, text, timestamptz),
    ple_data.save_assessment_policies(text, text, bigint, jsonb),
    ple_data.release_assessment(text, text, bigint)
    FROM PUBLIC;

GRANT SELECT ON ple_data.assessment, ple_data.assessment_entry,
    ple_data.assessment_entry_question, ple_data.assessment_entry_pool,
    ple_data.assessment_question_pool_fork
    TO ple_private_owner;

GRANT UPDATE (assessment_id) ON TABLE ple_data.assessment TO ple_private_owner;

GRANT SELECT ON ple_data.assessment, ple_data.assessment_entry,
    ple_data.assessment_entry_question, ple_data.assessment_entry_pool,
    ple_data.assessment_question_pool_fork
    TO ple_api_owner;

GRANT UPDATE (assessment_id) ON TABLE ple_data.assessment TO ple_api_owner;

GRANT EXECUTE ON FUNCTION ple_data.save_assessment(text, text, bigint, jsonb, jsonb),
    ple_data.save_assessment_inline(text, text, bigint, text, timestamptz),
    ple_data.save_assessment_policies(text, text, bigint, jsonb),
    ple_data.release_assessment(text, text, bigint)
    TO ple_api_owner;

