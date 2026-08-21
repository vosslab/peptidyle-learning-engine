-- WP-PROF-LD1: durable lifecycle marker for the installed Base Course.
--
-- This singleton belongs to the deployment, rather than to a course or a
-- person. It is deliberately inaccessible to application roles: only the
-- host-side installer coordinates it while holding its PostgreSQL advisory
-- lock. A generated installation generation binds the database marker to the
-- one reserved object-storage receipt. The first baseline contains no required
-- objects; retaining that explicit manifest prevents a later object addition
-- from becoming an invisible lifecycle change.

BEGIN;

CREATE TABLE public.live_demo_install_state (
    singleton boolean PRIMARY KEY DEFAULT true,
    state text NOT NULL,
    baseline_version text NOT NULL,
    tenant_id uuid,
    installation_generation uuid NOT NULL,
    object_manifest jsonb NOT NULL,
    storage_receipt_sha256 text,
    started_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    completed_at timestamp with time zone,
    CONSTRAINT live_demo_install_state_singleton_check CHECK (singleton),
    CONSTRAINT live_demo_install_state_state_check
        CHECK (state IN ('installing', 'complete')),
    CONSTRAINT live_demo_install_state_baseline_version_check
        CHECK (baseline_version = 'base-course-v1'),
    CONSTRAINT live_demo_install_state_object_manifest_check
        CHECK (object_manifest = '[]'::jsonb),
    CONSTRAINT live_demo_install_state_storage_receipt_sha256_check
        CHECK (
            storage_receipt_sha256 IS NULL
            OR storage_receipt_sha256 ~ '^[0-9a-f]{64}$'
        ),
    CONSTRAINT live_demo_install_state_lifecycle_check CHECK (
        (state = 'installing' AND tenant_id IS NOT NULL
            AND storage_receipt_sha256 IS NULL AND completed_at IS NULL)
        OR (state = 'complete' AND tenant_id IS NOT NULL
            AND storage_receipt_sha256 IS NOT NULL AND completed_at IS NOT NULL)
    )
);

REVOKE ALL ON TABLE public.live_demo_install_state FROM PUBLIC, ple_app;

COMMIT;
