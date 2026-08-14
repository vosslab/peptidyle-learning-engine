-- Forward repair for catalog rows whose immutable bytes or tenant grants must
-- be authored only by the owning tenant.  The original policies treated a
-- visible public/institution version as sufficient for INSERT, which allowed a
-- tenant that could read a row to append protected catalog state to it.

-- This narrow predicate is an application write capability. Reader roles use
-- the grant visibility policies below and must not invoke its bypass-RLS
-- ownership lookup directly.
REVOKE ALL ON FUNCTION public.ple_problem_owned_by_current_tenant(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_problem_owned_by_current_tenant(uuid) TO ple_app;

-- Keep tenant-scoped reads unchanged, but separate them from the owner-only
-- grant write.  `ple_problem_owned_by_current_tenant` is the dedicated,
-- RLS-obeying ownership predicate; it avoids recursing through problem
-- visibility while disclosing only the boolean ownership result.
DROP POLICY catalog_tenant_grant_tenant ON public.catalog_tenant_grant;

CREATE POLICY catalog_tenant_grant_app_visible_select ON public.catalog_tenant_grant
    FOR SELECT TO ple_app, ple_student, ple_grader
    USING (tenant_id = public.ple_current_tenant());

CREATE POLICY catalog_tenant_grant_app_owner_insert ON public.catalog_tenant_grant
    FOR INSERT TO ple_app
    WITH CHECK (
        tenant_id = public.ple_current_tenant()
        AND public.ple_problem_owned_by_current_tenant(problem_id)
    );

-- These append-only rows retain the existing visible-read policy.  Their
-- inserts are publication-time owner work, never a consequence of catalog
-- visibility in another tenant.
DROP POLICY problem_version_payload_app_insert ON public.problem_version_payload;

CREATE POLICY problem_version_payload_app_owner_insert ON public.problem_version_payload
    FOR INSERT TO ple_app
    WITH CHECK (public.ple_problem_owned_by_current_tenant(problem_id));

DROP POLICY published_source_artifact_app_insert ON public.published_source_artifact;

CREATE POLICY published_source_artifact_app_owner_insert ON public.published_source_artifact
    FOR INSERT TO ple_app
    WITH CHECK (public.ple_problem_owned_by_current_tenant(problem_id));
