-- Privileges from draft_question_image_publication_operations.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.bind_draft_question_image_publication(uuid, uuid, text, integer, text, text, jsonb, timestamptz) FROM PUBLIC;

