-- Privileges from delivery.sql.

SET LOCAL ROLE ple_private_owner;



-- API routines are owned by the API schema owner.  Their narrow private
-- helper remains private-owner code, with an explicit execute boundary.
REVOKE ALL ON FUNCTION ple_private.require_owned_open_question_attempt(text, text, uuid)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.require_owned_open_question_attempt(text, text, uuid)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION
    ple_api.create_imathas_question_backend_session(uuid, text, text, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, bytea, bytea, bytea, timestamptz, timestamptz, text, bytea, bytea),
    ple_api.load_imathas_question_backend_session(uuid, text, text, text, uuid, text, text, text, integer, uuid, bytea, text, numeric, text),
    ple_api.lease_imathas_question_backend_session(uuid, text, text, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, bytea, timestamptz)
FROM PUBLIC;

GRANT EXECUTE ON FUNCTION
    ple_api.create_imathas_question_backend_session(uuid, text, text, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, bytea, bytea, bytea, timestamptz, timestamptz, text, bytea, bytea),
    ple_api.load_imathas_question_backend_session(uuid, text, text, text, uuid, text, text, text, integer, uuid, bytea, text, numeric, text),
    ple_api.lease_imathas_question_backend_session(uuid, text, text, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, bytea, timestamptz)
TO ple_app;

