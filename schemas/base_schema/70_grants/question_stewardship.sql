-- Privileges from question_stewardship.sql.

SET LOCAL ROLE ple_private_owner;

-- Immutable revision credit and lineage stewardship.
GRANT SELECT ON ple_private.account, ple_private.account_state_event TO ple_data_owner;

SET LOCAL ROLE ple_data_owner;

GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;

REVOKE ALL ON ALL TABLES IN SCHEMA ple_data FROM PUBLIC;

GRANT INSERT, SELECT ON ple_data.question_revision_acceptance,
    ple_data.question_revision_authorship, ple_data.question_revision_license,
    ple_data.question_ownership_event, ple_data.question_fork_source TO ple_private_owner;

GRANT SELECT ON ple_data.question_current_owner TO ple_private_owner;

GRANT SELECT ON ple_data.question_revision_acceptance TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_data.reject_question_stewardship_change(),
    ple_data.validate_question_revision_acceptance(),
    ple_data.validate_question_ownership_event(), ple_data.validate_question_publication(),
    ple_data.set_current_question_star(text, boolean),
    ple_data.read_current_question_star(text),
    ple_data.set_current_question_watch(text, boolean),
    ple_data.read_current_question_watch(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.set_current_question_star(text, boolean),
    ple_data.read_current_question_star(text),
    ple_data.set_current_question_watch(text, boolean),
    ple_data.read_current_question_watch(text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.set_current_question_star(text, boolean),
    ple_api.read_current_question_star(text),
    ple_api.set_current_question_watch(text, boolean),
    ple_api.read_current_question_watch(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.set_current_question_star(text, boolean),
    ple_api.read_current_question_star(text),
    ple_api.set_current_question_watch(text, boolean),
    ple_api.read_current_question_watch(text) TO ple_app;

