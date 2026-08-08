-- MOD-UI-EDITOR: explicit per-workspace authoring authority and revision.
--
-- Existing rows deliberately receive a revision but no access binding. They
-- remain inaccessible until an authorized migration process resolves their
-- historical owner; silently assigning them to the next caller would permit
-- workspace takeover.

ALTER TABLE workspace_draft
    ADD COLUMN revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0);

CREATE TABLE workspace_draft_access (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    user_id uuid NOT NULL,
    role text NOT NULL CHECK (role IN ('owner', 'collaborator')),
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, workspace_id, user_id),
    FOREIGN KEY (tenant_id, workspace_id)
        REFERENCES workspace_draft(tenant_id, workspace_id) ON DELETE CASCADE
);

CREATE INDEX workspace_draft_access_user_idx
    ON workspace_draft_access (tenant_id, user_id, workspace_id);

ALTER TABLE workspace_draft_access ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_draft_access FORCE ROW LEVEL SECURITY;
CREATE POLICY workspace_draft_access_tenant ON workspace_draft_access
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

REVOKE ALL ON workspace_draft_access
    FROM PUBLIC, ple_student, ple_grader, ple_qti_grader, ple_queue_broker, ple_auth;
GRANT SELECT, INSERT, UPDATE, DELETE ON workspace_draft_access TO ple_app;
