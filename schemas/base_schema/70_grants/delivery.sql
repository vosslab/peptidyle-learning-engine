-- Privileges from delivery.sql.

SET LOCAL ROLE ple_private_owner;



-- API routines are owned by the API schema owner.  Their narrow private
-- helper remains private-owner code, with an explicit execute boundary.
REVOKE ALL ON FUNCTION ple_private.require_owned_open_question_attempt(text, text, uuid)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.require_owned_open_question_attempt(text, text, uuid)
    TO ple_api_owner;


