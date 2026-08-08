-- MOD-API-CAT: hot catalog metadata, visibility grants, lineage, and lifecycle.

ALTER TABLE problem_version
    DROP CONSTRAINT problem_version_lifecycle_check;

ALTER TABLE problem_version
    ADD COLUMN backend text NOT NULL DEFAULT 'native'
        CHECK (backend IN ('native', 'webwork', 'qti', 'h5p')),
    ADD COLUMN capabilities jsonb NOT NULL DEFAULT '[]'::jsonb
        CHECK (jsonb_typeof(capabilities) = 'array'),
    ADD COLUMN metadata jsonb NOT NULL DEFAULT '{}'::jsonb
        CHECK (jsonb_typeof(metadata) = 'object'),
    ADD COLUMN publication_scope text NOT NULL DEFAULT 'public'
        CHECK (publication_scope IN ('institution', 'public')),
    ADD COLUMN lifecycle_reason text,
    ADD COLUMN authors jsonb NOT NULL DEFAULT '[]'::jsonb
        CHECK (jsonb_typeof(authors) = 'array'),
    ADD COLUMN previous_version_id uuid,
    ADD COLUMN derived_from_problem_id uuid,
    ADD COLUMN derived_from_version_id uuid,
    ADD CONSTRAINT problem_version_previous_fk
        FOREIGN KEY (problem_id, previous_version_id)
        REFERENCES problem_version(problem_id, version_id)
        DEFERRABLE INITIALLY DEFERRED,
    ADD CONSTRAINT problem_version_derived_from_fk
        FOREIGN KEY (derived_from_problem_id, derived_from_version_id)
        REFERENCES problem_version(problem_id, version_id)
        DEFERRABLE INITIALLY DEFERRED,
    ADD CONSTRAINT problem_version_derived_pair_check
        CHECK (
            (derived_from_problem_id IS NULL) = (derived_from_version_id IS NULL)
        );

-- Existing catalog rows predate author ownership. Enforce this for all new or
-- changed rows without inventing a legacy author; a later data migration can
-- backfill and validate the constraint when the owner mapping is available.
ALTER TABLE problem_version
    ADD CONSTRAINT problem_version_authors_nonempty_check
        CHECK (jsonb_array_length(authors) > 0) NOT VALID;

UPDATE problem_version
SET lifecycle = 'deprecated',
    lifecycle_reason = 'Imported legacy withdrawal'
WHERE lifecycle = 'withdrawn';

ALTER TABLE problem_version
    ADD CONSTRAINT problem_version_lifecycle_check
        CHECK (lifecycle IN ('published', 'deprecated', 'archived')),
    ADD CONSTRAINT problem_version_lifecycle_reason_check
        CHECK (
            (lifecycle = 'published' AND lifecycle_reason IS NULL)
            OR (
                lifecycle IN ('deprecated', 'archived')
                AND char_length(btrim(lifecycle_reason)) BETWEEN 1 AND 1000
            )
        );

CREATE UNIQUE INDEX problem_version_linear_chain_idx
    ON problem_version (problem_id, previous_version_id)
    WHERE previous_version_id IS NOT NULL;

DROP INDEX problem_version_catalog_idx;
CREATE INDEX problem_version_catalog_idx
    ON problem_version (lifecycle, title, problem_id, version_id);
CREATE INDEX problem_version_metadata_idx
    ON problem_version USING gin (metadata jsonb_path_ops);
CREATE INDEX problem_version_capabilities_idx
    ON problem_version USING gin (capabilities jsonb_path_ops);

CREATE TABLE catalog_tenant_grant (
    tenant_id uuid NOT NULL,
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    PRIMARY KEY (tenant_id, problem_id, version_id),
    FOREIGN KEY (problem_id, version_id)
        REFERENCES problem_version(problem_id, version_id)
        ON DELETE CASCADE
        DEFERRABLE INITIALLY DEFERRED
);

ALTER TABLE catalog_tenant_grant ENABLE ROW LEVEL SECURITY;
ALTER TABLE catalog_tenant_grant FORCE ROW LEVEL SECURITY;
CREATE POLICY catalog_tenant_grant_tenant ON catalog_tenant_grant
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

ALTER TABLE problem_version ENABLE ROW LEVEL SECURITY;
ALTER TABLE problem_version FORCE ROW LEVEL SECURITY;
CREATE POLICY problem_version_visible_select ON problem_version
    FOR SELECT TO ple_app, ple_student
    USING (
        publication_scope = 'public'
        OR EXISTS (
            SELECT 1 FROM catalog_tenant_grant AS grant_row
            WHERE grant_row.problem_id = problem_version.problem_id
              AND grant_row.version_id = problem_version.version_id
              AND grant_row.tenant_id = ple_current_tenant()
        )
    );
CREATE POLICY problem_version_app_insert ON problem_version
    FOR INSERT TO ple_app
    WITH CHECK (
        publication_scope = 'public'
        OR EXISTS (
            SELECT 1 FROM catalog_tenant_grant AS grant_row
            WHERE grant_row.problem_id = problem_version.problem_id
              AND grant_row.version_id = problem_version.version_id
              AND grant_row.tenant_id = ple_current_tenant()
        )
    );
CREATE POLICY problem_version_app_update ON problem_version
    FOR UPDATE TO ple_app
    USING (
        publication_scope = 'public'
        OR EXISTS (
            SELECT 1 FROM catalog_tenant_grant AS grant_row
            WHERE grant_row.problem_id = problem_version.problem_id
              AND grant_row.version_id = problem_version.version_id
              AND grant_row.tenant_id = ple_current_tenant()
        )
    )
    WITH CHECK (
        publication_scope = 'public'
        OR EXISTS (
            SELECT 1 FROM catalog_tenant_grant AS grant_row
            WHERE grant_row.problem_id = problem_version.problem_id
              AND grant_row.version_id = problem_version.version_id
              AND grant_row.tenant_id = ple_current_tenant()
        )
    );

ALTER TABLE problem_version_payload ENABLE ROW LEVEL SECURITY;
ALTER TABLE problem_version_payload FORCE ROW LEVEL SECURITY;
CREATE POLICY problem_version_payload_visible_select ON problem_version_payload
    FOR SELECT TO ple_app, ple_student
    USING (
        EXISTS (
            SELECT 1 FROM problem_version AS visible_version
            WHERE visible_version.problem_id = problem_version_payload.problem_id
              AND visible_version.version_id = problem_version_payload.version_id
        )
    );
CREATE POLICY problem_version_payload_app_insert ON problem_version_payload
    FOR INSERT TO ple_app
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM problem_version AS visible_version
            WHERE visible_version.problem_id = problem_version_payload.problem_id
              AND visible_version.version_id = problem_version_payload.version_id
        )
    );

REVOKE SELECT, UPDATE, DELETE ON problem FROM ple_app;
REVOKE UPDATE, DELETE ON problem_version, problem_version_payload FROM ple_app;
GRANT UPDATE (lifecycle, lifecycle_reason) ON problem_version TO ple_app;

GRANT SELECT, INSERT ON catalog_tenant_grant TO ple_app;
GRANT SELECT ON catalog_tenant_grant TO ple_student;
REVOKE SELECT ON problem FROM ple_student;
