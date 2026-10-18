-- Privileges from question_thumbnails.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON ple_private.question_revision_thumbnail FROM PUBLIC;

-- Trusted migrator can SET ROLE to the owner; runtime roles gain no maintenance access.
REVOKE ALL ON FUNCTION ple_private.question_thumbnail_backfill_snapshot(text, integer),
    ple_private.register_existing_question_thumbnail(text, integer, uuid, bytea, jsonb,
        uuid, jsonb, bytea, bigint, bigint) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.reject_question_revision_thumbnail_change(),
    ple_private.validate_question_revision_thumbnail_object_record(),
    ple_private.resolve_question_thumbnail(text, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.resolve_question_thumbnail(text, integer)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.resolve_question_thumbnail(text, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.resolve_question_thumbnail(text, integer) TO ple_app;
