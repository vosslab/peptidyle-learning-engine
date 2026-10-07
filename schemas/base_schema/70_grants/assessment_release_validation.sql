-- Privileges from assessment_release_validation.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON FUNCTION ple_data.assessment_delivered_question_count(text),
    ple_data.assessment_effective_base_duration_seconds(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.assessment_delivered_question_count(text),
    ple_data.assessment_effective_base_duration_seconds(text)
    TO ple_api_owner, ple_private_owner;

-- ASVS 8.2.1 and 8.3.1: trusted release validation consumes this detailed evidence;
-- direct access remains unavailable to PUBLIC and ple_app.
REVOKE ALL ON FUNCTION ple_data.assessment_pool_release_issues(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.assessment_pool_release_issues(text)
    TO ple_api_owner;

REVOKE ALL ON FUNCTION
    ple_data.assessment_schedule_issues(text, timestamptz, boolean),
    ple_data.validate_assessment_post_release_edit(text, timestamptz, boolean),
    ple_data.assessment_release_issues(text, timestamptz, boolean),
    ple_data.validate_assessment_release(text, timestamptz, boolean)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION
    ple_data.assessment_schedule_issues(text, timestamptz, boolean),
    ple_data.validate_assessment_post_release_edit(text, timestamptz, boolean),
    ple_data.assessment_release_issues(text, timestamptz, boolean),
    ple_data.validate_assessment_release(text, timestamptz, boolean)
    TO ple_api_owner;
