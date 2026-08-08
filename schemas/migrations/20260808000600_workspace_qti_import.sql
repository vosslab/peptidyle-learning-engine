-- MOD-ADP-QTI / MOD-STO: private, immutable QTI workspace staging registry.
-- Archive bytes remain object-store authoritative. This schema persists only
-- verified object metadata and a separate grader-only answer binding.

CREATE TABLE workspace_qti_import (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    source_object_id uuid NOT NULL UNIQUE,
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    payload_sha256 character(64) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, workspace_id, import_id)
);

CREATE TABLE workspace_qti_import_item (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    item_id text NOT NULL CHECK (length(item_id) BETWEEN 1 AND 512),
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    payload_sha256 character(64) NOT NULL,
    PRIMARY KEY (tenant_id, workspace_id, import_id, item_id),
    FOREIGN KEY (tenant_id, workspace_id, import_id)
        REFERENCES workspace_qti_import(tenant_id, workspace_id, import_id)
        ON DELETE RESTRICT
        DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE workspace_qti_import_asset (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    object_id uuid NOT NULL UNIQUE,
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    payload_sha256 character(64) NOT NULL,
    PRIMARY KEY (tenant_id, workspace_id, import_id, asset_id),
    FOREIGN KEY (tenant_id, workspace_id, import_id)
        REFERENCES workspace_qti_import(tenant_id, workspace_id, import_id)
        ON DELETE RESTRICT
        DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE workspace_qti_import_unsupported (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    ordinal integer NOT NULL CHECK (ordinal >= 0),
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    payload_sha256 character(64) NOT NULL,
    PRIMARY KEY (tenant_id, workspace_id, import_id, ordinal),
    FOREIGN KEY (tenant_id, workspace_id, import_id)
        REFERENCES workspace_qti_import(tenant_id, workspace_id, import_id)
        ON DELETE RESTRICT
        DEFERRABLE INITIALLY DEFERRED
);

-- This table is not a JSON payload. Its bytes are opaque, non-browser data.
CREATE TABLE workspace_qti_import_grading (
    tenant_id uuid NOT NULL,
    workspace_id uuid NOT NULL,
    import_id uuid NOT NULL,
    item_id text NOT NULL CHECK (length(item_id) BETWEEN 1 AND 512),
    payload bytea NOT NULL CHECK (octet_length(payload) BETWEEN 1 AND 262144),
    payload_sha256 character(64) NOT NULL,
    PRIMARY KEY (tenant_id, workspace_id, import_id, item_id),
    FOREIGN KEY (tenant_id, workspace_id, import_id, item_id)
        REFERENCES workspace_qti_import_item(tenant_id, workspace_id, import_id, item_id)
        ON DELETE RESTRICT
        DEFERRABLE INITIALLY DEFERRED
);

ALTER TABLE workspace_qti_import ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_qti_import FORCE ROW LEVEL SECURITY;
ALTER TABLE workspace_qti_import_item ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_qti_import_item FORCE ROW LEVEL SECURITY;
ALTER TABLE workspace_qti_import_asset ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_qti_import_asset FORCE ROW LEVEL SECURITY;
ALTER TABLE workspace_qti_import_unsupported ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_qti_import_unsupported FORCE ROW LEVEL SECURITY;
ALTER TABLE workspace_qti_import_grading ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_qti_import_grading FORCE ROW LEVEL SECURITY;

CREATE POLICY workspace_qti_import_tenant ON workspace_qti_import
    TO ple_app USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY workspace_qti_import_item_tenant ON workspace_qti_import_item
    TO ple_app USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY workspace_qti_import_asset_tenant ON workspace_qti_import_asset
    TO ple_app USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY workspace_qti_import_unsupported_tenant ON workspace_qti_import_unsupported
    TO ple_app USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

-- Import workers may insert opaque bindings but cannot select them. The grader
-- role has a separate read-only policy and no access to registry metadata.
CREATE POLICY workspace_qti_import_grading_app_insert ON workspace_qti_import_grading
    FOR INSERT TO ple_app WITH CHECK (tenant_id = ple_current_tenant());
CREATE POLICY workspace_qti_import_grading_grader_select ON workspace_qti_import_grading
    FOR SELECT TO ple_grader USING (tenant_id = ple_current_tenant());

GRANT SELECT, INSERT ON workspace_qti_import, workspace_qti_import_item,
    workspace_qti_import_asset, workspace_qti_import_unsupported TO ple_app;
REVOKE ALL ON workspace_qti_import_grading FROM PUBLIC, ple_app, ple_student;
GRANT INSERT ON workspace_qti_import_grading TO ple_app;
GRANT SELECT ON workspace_qti_import_grading TO ple_grader;
