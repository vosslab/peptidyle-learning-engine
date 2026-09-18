-- Privileges from assessment_release_validation.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.assessment_delivered_question_count(uuid),
    ple_data.assessment_effective_base_duration_seconds(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.assessment_delivered_question_count(uuid),
    ple_data.assessment_effective_base_duration_seconds(uuid)
    TO ple_api_owner, ple_private_owner;

REVOKE ALL ON FUNCTION
    ple_data.assessment_release_issues(uuid, timestamptz, boolean),
    ple_data.validate_assessment_release(uuid, timestamptz, boolean)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION
    ple_data.assessment_release_issues(uuid, timestamptz, boolean),
    ple_data.validate_assessment_release(uuid, timestamptz, boolean)
    TO ple_api_owner;

