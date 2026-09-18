-- Privileges from assessment_entry_snapshot.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.assessment_entry_snapshot FROM PUBLIC;

GRANT SELECT, INSERT ON TABLE ple_private.assessment_entry_snapshot TO ple_data_owner;

GRANT SELECT ON TABLE ple_private.assessment_entry_snapshot
    TO ple_api_owner, ple_unrelease_executor;

REVOKE ALL ON FUNCTION ple_private.ensure_assessment_entry_snapshot(
    ple_data.entry_kind, ple_data.scoring_rule, numeric, text, integer, text,
    integer, integer, integer
) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.ensure_assessment_entry_snapshot(
    ple_data.entry_kind, ple_data.scoring_rule, numeric, text, integer, text,
    integer, integer, integer
) TO ple_data_owner, ple_api_owner;
