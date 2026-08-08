-- MOD-API-ASSET: immutable object registry and authorized delivery lookup.

CREATE TABLE asset_delivery (
    delivery_id uuid PRIMARY KEY,
    delivery_kind text NOT NULL
        CHECK (delivery_kind IN ('catalog', 'student_record')),
    tenant_id uuid,
    object_id uuid NOT NULL UNIQUE,
    problem_id uuid,
    version_id uuid,
    asset_id uuid UNIQUE,
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    FOREIGN KEY (problem_id, version_id)
        REFERENCES problem_version(problem_id, version_id),
    CHECK (
        (
            delivery_kind = 'catalog'
            AND tenant_id IS NULL
            AND problem_id IS NOT NULL
            AND version_id IS NOT NULL
            AND asset_id IS NOT NULL
            AND delivery_id = asset_id
        )
        OR
        (
            delivery_kind = 'student_record'
            AND tenant_id IS NOT NULL
            AND problem_id IS NULL
            AND version_id IS NULL
            AND asset_id IS NULL
            AND delivery_id = object_id
        )
    )
);

CREATE INDEX asset_delivery_catalog_version_idx
    ON asset_delivery (problem_id, version_id)
    WHERE delivery_kind = 'catalog';
CREATE INDEX asset_delivery_tenant_idx
    ON asset_delivery (tenant_id, delivery_id)
    WHERE delivery_kind = 'student_record';

ALTER TABLE asset_delivery ENABLE ROW LEVEL SECURITY;
ALTER TABLE asset_delivery FORCE ROW LEVEL SECURITY;
CREATE POLICY asset_delivery_visible_select ON asset_delivery
    FOR SELECT TO ple_app
    USING (
        (
            delivery_kind = 'catalog'
            AND EXISTS (
                SELECT 1 FROM problem_version AS visible_version
                WHERE visible_version.problem_id = asset_delivery.problem_id
                  AND visible_version.version_id = asset_delivery.version_id
            )
        )
        OR (
            delivery_kind = 'student_record'
            AND tenant_id = ple_current_tenant()
        )
    );
CREATE POLICY asset_delivery_app_insert ON asset_delivery
    FOR INSERT TO ple_app
    WITH CHECK (
        (
            delivery_kind = 'catalog'
            AND EXISTS (
                SELECT 1 FROM problem_version AS visible_version
                WHERE visible_version.problem_id = asset_delivery.problem_id
                  AND visible_version.version_id = asset_delivery.version_id
            )
        )
        OR (
            delivery_kind = 'student_record'
            AND tenant_id = ple_current_tenant()
        )
    );

GRANT SELECT, INSERT ON asset_delivery TO ple_app;
