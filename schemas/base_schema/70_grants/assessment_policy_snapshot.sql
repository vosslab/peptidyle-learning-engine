-- Privileges from assessment_policy_snapshot.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.assessment_policy_snapshot FROM PUBLIC;

GRANT SELECT, INSERT ON TABLE ple_data.assessment_policy_snapshot TO ple_private_owner;

GRANT SELECT ON TABLE ple_data.assessment_policy_snapshot
    TO ple_api_owner, ple_unrelease_executor;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.ensure_assessment_policy_snapshot(
    text, text, timestamptz, timestamptz, timestamptz, integer, integer,
    ple_data.late_work_rule, ple_data.question_variation_rule, ple_data.question_order_rule,
    ple_data.feedback_release, ple_data.feedback_release, ple_data.feedback_release,
    ple_data.feedback_release, ple_data.feedback_release, ple_data.feedback_release,
    ple_data.assessment_type
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.ensure_assessment_policy_snapshot(
    text, text, timestamptz, timestamptz, timestamptz, integer, integer,
    ple_data.late_work_rule, ple_data.question_variation_rule, ple_data.question_order_rule,
    ple_data.feedback_release, ple_data.feedback_release, ple_data.feedback_release,
    ple_data.feedback_release, ple_data.feedback_release, ple_data.feedback_release,
    ple_data.assessment_type
) TO ple_data_owner, ple_api_owner;
