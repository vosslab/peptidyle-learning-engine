-- Privileges from blueprint_change_proposals.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.blueprint_change_proposal FROM PUBLIC;


-- No mutation privilege or lifecycle state: the original comparison basis survives.
GRANT SELECT, INSERT ON TABLE ple_data.blueprint_change_proposal TO ple_api_owner;

REVOKE ALL ON TABLE ple_data.blueprint_change_proposal_acceptance FROM PUBLIC;

GRANT SELECT, INSERT ON TABLE ple_data.blueprint_change_proposal_acceptance TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_data.reject_blueprint_proposal_evidence_change() FROM PUBLIC;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.create_blueprint_change_proposal(
    text, bigint, uuid, text, bigint, uuid) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_api.read_blueprint_change_proposal(uuid) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_api.list_blueprint_change_proposals(text, boolean, text, uuid, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.create_blueprint_change_proposal(
    text, bigint, uuid, text, bigint, uuid) TO ple_app;

GRANT EXECUTE ON FUNCTION ple_api.read_blueprint_change_proposal(uuid) TO ple_app;

GRANT EXECUTE ON FUNCTION ple_api.list_blueprint_change_proposals(text, boolean, text, uuid, integer) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.lock_blueprint_change_proposal_acceptance(uuid, text, bigint, uuid),
    ple_api.finalize_blueprint_change_proposal_acceptance(uuid, text, bigint, uuid, jsonb, bytea, jsonb, bytea),
    ple_api.read_accepted_blueprint_change_proposal(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.lock_blueprint_change_proposal_acceptance(uuid, text, bigint, uuid),
    ple_api.finalize_blueprint_change_proposal_acceptance(uuid, text, bigint, uuid, jsonb, bytea, jsonb, bytea),
    ple_api.read_accepted_blueprint_change_proposal(uuid) TO ple_app;

