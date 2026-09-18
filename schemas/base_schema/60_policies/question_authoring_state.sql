-- Row security policies from question_authoring_state.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.authoring_workspace ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.authoring_workspace FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.authoring_workspace_collaborator_event ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.authoring_workspace_collaborator_event FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.draft_question ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.draft_question FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.draft_question_metadata ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.draft_question_metadata FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.draft_question_source_binding ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.draft_question_source_binding FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_revision_source_binding ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_revision_source_binding FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.draft_question_fork_source ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.draft_question_fork_source FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_folder ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_folder FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_folder_entry ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_folder_entry FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.saved_question_search ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.saved_question_search FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.workspace_import ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.workspace_import FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.workspace_import_item_result ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.workspace_import_item_result FORCE ROW LEVEL SECURITY;

CREATE POLICY authoring_workspace_private_owner_access ON ple_private.authoring_workspace
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY authoring_collaborator_private_owner_access ON ple_private.authoring_workspace_collaborator_event
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY draft_question_private_owner_access ON ple_private.draft_question
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY draft_question_metadata_private_owner_access ON ple_private.draft_question_metadata
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY draft_source_private_owner_access ON ple_private.draft_question_source_binding
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY revision_source_private_owner_access ON ple_private.question_revision_source_binding
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY draft_fork_private_owner_access ON ple_private.draft_question_fork_source
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY question_folder_private_owner_access ON ple_private.question_folder
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY question_folder_entry_private_owner_access ON ple_private.question_folder_entry
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY saved_question_search_private_owner_access ON ple_private.saved_question_search
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY workspace_import_private_owner_access ON ple_private.workspace_import
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY workspace_import_item_result_private_owner_access ON ple_private.workspace_import_item_result
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

