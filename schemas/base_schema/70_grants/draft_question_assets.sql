-- Privileges from draft_question_assets.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON ple_private.draft_question_asset FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.validate_draft_question_asset() FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.register_draft_question_asset(uuid,bigint,uuid,uuid,jsonb,bytea,bigint,text,bigint,integer,integer),
    ple_private.load_draft_question_asset(uuid,uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.register_draft_question_asset(uuid,bigint,uuid,uuid,jsonb,bytea,bigint,text,bigint,integer,integer),
    ple_private.load_draft_question_asset(uuid,uuid) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.register_draft_question_asset(uuid,bigint,uuid,uuid,jsonb,bytea,bigint,text,bigint,integer,integer),
    ple_api.load_draft_question_asset(uuid,uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.register_draft_question_asset(uuid,bigint,uuid,uuid,jsonb,bytea,bigint,text,bigint,integer,integer),
    ple_api.load_draft_question_asset(uuid,uuid) TO ple_app;

