-- Privileges from delivery_backends.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.imathas_render_cache_entry, ple_private.imathas_question_backend_session FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.enforce_imathas_question_backend_session_transition() FROM PUBLIC;

