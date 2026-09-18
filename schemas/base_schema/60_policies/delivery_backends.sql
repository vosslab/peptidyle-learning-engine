-- Row security policies from delivery_backends.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.imathas_render_cache_entry ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.imathas_render_cache_entry FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.imathas_question_backend_session ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.imathas_question_backend_session FORCE ROW LEVEL SECURITY;

CREATE POLICY imathas_render_cache_private_owner_access ON ple_private.imathas_render_cache_entry FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY imathas_session_private_owner_access ON ple_private.imathas_question_backend_session FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

